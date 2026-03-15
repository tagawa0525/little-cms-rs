//! Pipeline and Stage (LUT) engine.
//!
//! C版対応: `cmslut.c`
//!
//! Provides the Pipeline/Stage system that is the core of color transformation.
//! A Pipeline is a chain of Stages, each performing one step of color processing
//! (curves, matrices, CLUTs, etc.).

use crate::curves::gamma::ToneCurve;
use crate::curves::intrp::{self, InterpParams};
use crate::types::StageSignature;

// ============================================================================
// Utility functions
// ============================================================================

/// Quantize a value 0 <= i < max_samples to 0..0xFFFF.
///
/// C版: `_cmsQuantizeVal`
#[allow(dead_code)]
pub(crate) fn quantize_val(i: f64, max_samples: u32) -> u16 {
    let x = (i * 65535.0) / (max_samples - 1) as f64;
    intrp::quick_saturate_word(x)
}

/// Convert f32 [0..1] to u16 [0..0xFFFF].
fn float_to_16(input: &[f32], output: &mut [u16]) {
    for (o, &v) in output.iter_mut().zip(input.iter()) {
        *o = intrp::quick_saturate_word(v as f64 * 65535.0);
    }
}

/// Convert u16 [0..0xFFFF] to f32 [0..1].
fn from_16_to_float(input: &[u16], output: &mut [f32]) {
    for (o, &v) in output.iter_mut().zip(input.iter()) {
        *o = v as f32 / 65535.0;
    }
}

/// Flag for sampling: write results back to CLUT table.
pub const SAMPLER_WRITE: u32 = 0;
/// Flag for sampling: inspect only, do not write back.
pub const SAMPLER_INSPECT: u32 = 1;

// ============================================================================
// StageData
// ============================================================================

/// Stage data payload.
///
/// C版では `void* Data` + 関数ポインタで型消去していたが、
/// Rust では閉じた enum で安全にディスパッチする。
#[derive(Clone)]
pub enum StageData {
    Curves(Vec<ToneCurve>),
    Matrix {
        coefficients: Vec<f64>,
        offset: Option<Vec<f64>>,
    },
    CLut(CLutData),
    NamedColor(super::named::NamedColorList),
    None,
}

// ============================================================================
// CLutData / CLutTable
// ============================================================================

/// CLUT table storage — either u16 or f32.
#[derive(Clone)]
pub enum CLutTable {
    U16(Vec<u16>),
    Float(Vec<f32>),
}

/// CLUT data: interpolation parameters + table.
#[derive(Clone)]
pub struct CLutData {
    pub params: InterpParams,
    pub table: CLutTable,
    pub n_entries: u32,
}

/// Compute total number of grid nodes for a hypercube.
///
/// Returns `None` on overflow or if any dimension <= 1.
///
/// C版: `CubeSize`
#[allow(dead_code)]
fn cube_size(dims: &[u32], n: u32) -> Option<u32> {
    let mut rv: u64 = 1;
    for &dim in &dims[..n as usize] {
        if dim <= 1 {
            return None;
        }
        rv = rv.checked_mul(dim as u64)?;
        if rv > u32::MAX as u64 / 15 {
            return None;
        }
    }
    Some(rv as u32)
}

// ============================================================================
// Stage
// ============================================================================

/// A single processing element in a pipeline.
///
/// C版: `cmsStage`
#[derive(Clone)]
pub struct Stage {
    stage_type: StageSignature,
    implements: StageSignature,
    input_channels: u32,
    output_channels: u32,
    data: StageData,
}

impl Stage {
    // --- Accessors ---

    pub fn stage_type(&self) -> StageSignature {
        self.stage_type
    }

    pub fn implements(&self) -> StageSignature {
        self.implements
    }

    pub fn input_channels(&self) -> u32 {
        self.input_channels
    }

    pub fn output_channels(&self) -> u32 {
        self.output_channels
    }

    pub fn data(&self) -> &StageData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut StageData {
        &mut self.data
    }

    /// Get curves from a CurveSetElem stage.
    ///
    /// C版: `_cmsStageGetPtrToCurveSet`
    pub fn curves(&self) -> Option<&[ToneCurve]> {
        match &self.data {
            StageData::Curves(c) => Some(c),
            _ => None,
        }
    }

    pub fn curves_mut(&mut self) -> Option<&mut [ToneCurve]> {
        match &mut self.data {
            StageData::Curves(c) => Some(c),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_implements(&mut self, sig: StageSignature) {
        self.implements = sig;
    }

    // --- Constructors ---

    /// Create an identity stage that copies input to output.
    ///
    /// C版: `cmsStageAllocIdentity`
    pub fn new_identity(n: u32) -> Option<Self> {
        if n == 0 || n as usize > intrp::MAX_STAGE_CHANNELS {
            return None;
        }
        Some(Stage {
            stage_type: StageSignature::IdentityElem,
            implements: StageSignature::IdentityElem,
            input_channels: n,
            output_channels: n,
            data: StageData::None,
        })
    }

    /// Create a tone curves stage. `None` curves creates identity (gamma 1.0).
    ///
    /// C版: `cmsStageAllocToneCurves`
    pub fn new_tone_curves(curves: Option<&[ToneCurve]>, n: u32) -> Option<Self> {
        if n == 0 || n as usize > intrp::MAX_STAGE_CHANNELS {
            return None;
        }
        let curve_vec: Vec<ToneCurve> = match curves {
            Some(c) => {
                if (c.len() as u32) < n {
                    return None;
                }
                c[..n as usize].to_vec()
            }
            None => {
                let identity = ToneCurve::build_gamma(1.0)?;
                vec![identity; n as usize]
            }
        };
        Some(Stage {
            stage_type: StageSignature::CurveSetElem,
            implements: StageSignature::CurveSetElem,
            input_channels: n,
            output_channels: n,
            data: StageData::Curves(curve_vec),
        })
    }

    /// Create identity curves stage.
    ///
    /// C版: `_cmsStageAllocIdentityCurves`
    pub fn new_identity_curves(n: u32) -> Option<Self> {
        let mut stage = Self::new_tone_curves(None, n)?;
        stage.implements = StageSignature::IdentityElem;
        Some(stage)
    }

    /// Create a matrix stage.
    ///
    /// `matrix` is row-major: `matrix[i * cols + j]`.
    /// Input channels = cols, output channels = rows.
    ///
    /// C版: `cmsStageAllocMatrix`
    pub fn new_matrix(
        rows: u32,
        cols: u32,
        matrix: &[f64],
        offset: Option<&[f64]>,
    ) -> Option<Self> {
        if rows == 0
            || cols == 0
            || rows as usize > intrp::MAX_STAGE_CHANNELS
            || cols as usize > intrp::MAX_STAGE_CHANNELS
        {
            return None;
        }
        let n = (rows as u64) * (cols as u64);
        if n > u32::MAX as u64 {
            return None;
        }
        let n = n as usize;
        if matrix.len() < n {
            return None;
        }

        let offset_vec = offset.map(|o| {
            if o.len() >= rows as usize {
                o[..rows as usize].to_vec()
            } else {
                let mut v = o.to_vec();
                v.resize(rows as usize, 0.0);
                v
            }
        });

        Some(Stage {
            stage_type: StageSignature::MatrixElem,
            implements: StageSignature::MatrixElem,
            input_channels: cols,
            output_channels: rows,
            data: StageData::Matrix {
                coefficients: matrix[..n].to_vec(),
                offset: offset_vec,
            },
        })
    }

    /// Create a 16-bit CLUT stage with per-dimension grid sizes.
    ///
    /// C版: `cmsStageAllocCLut16bitGranular`
    pub fn new_clut_16bit(
        grid_points: &[u32],
        input_channels: u32,
        output_channels: u32,
        table: Option<&[u16]>,
    ) -> Option<Self> {
        use crate::curves::intrp::{LERP_FLAGS_16BITS, MAX_INPUT_DIMENSIONS};

        if input_channels as usize > MAX_INPUT_DIMENSIONS {
            return None;
        }
        if input_channels == 0
            || output_channels == 0
            || output_channels as usize > intrp::MAX_STAGE_CHANNELS
        {
            return None;
        }

        let total_nodes = cube_size(grid_points, input_channels)?;
        let n_entries = output_channels.checked_mul(total_nodes)?;

        let data = match table {
            Some(t) => {
                let mut v = vec![0u16; n_entries as usize];
                let copy_len = t.len().min(n_entries as usize);
                v[..copy_len].copy_from_slice(&t[..copy_len]);
                v
            }
            None => vec![0u16; n_entries as usize],
        };

        let params = InterpParams::compute(
            grid_points,
            input_channels,
            output_channels,
            LERP_FLAGS_16BITS,
        )?;

        Some(Stage {
            stage_type: StageSignature::CLutElem,
            implements: StageSignature::CLutElem,
            input_channels,
            output_channels,
            data: StageData::CLut(CLutData {
                params,
                table: CLutTable::U16(data),
                n_entries,
            }),
        })
    }

    /// Create a 16-bit CLUT stage with uniform grid.
    ///
    /// C版: `cmsStageAllocCLut16bit`
    pub fn new_clut_16bit_uniform(
        grid_points: u32,
        input_channels: u32,
        output_channels: u32,
        table: Option<&[u16]>,
    ) -> Option<Self> {
        use crate::curves::intrp::MAX_INPUT_DIMENSIONS;

        let dims = [grid_points; MAX_INPUT_DIMENSIONS];
        Self::new_clut_16bit(&dims, input_channels, output_channels, table)
    }

    /// Create a float CLUT stage with per-dimension grid sizes.
    ///
    /// C版: `cmsStageAllocCLutFloatGranular`
    pub fn new_clut_float(
        grid_points: &[u32],
        input_channels: u32,
        output_channels: u32,
        table: Option<&[f32]>,
    ) -> Option<Self> {
        use crate::curves::intrp::{LERP_FLAGS_FLOAT, MAX_INPUT_DIMENSIONS};

        if input_channels as usize > MAX_INPUT_DIMENSIONS {
            return None;
        }
        if input_channels == 0
            || output_channels == 0
            || output_channels as usize > intrp::MAX_STAGE_CHANNELS
        {
            return None;
        }

        let total_nodes = cube_size(grid_points, input_channels)?;
        let n_entries = output_channels.checked_mul(total_nodes)?;

        let data = match table {
            Some(t) => {
                let mut v = vec![0.0f32; n_entries as usize];
                let copy_len = t.len().min(n_entries as usize);
                v[..copy_len].copy_from_slice(&t[..copy_len]);
                v
            }
            None => vec![0.0f32; n_entries as usize],
        };

        let params = InterpParams::compute(
            grid_points,
            input_channels,
            output_channels,
            LERP_FLAGS_FLOAT,
        )?;

        Some(Stage {
            stage_type: StageSignature::CLutElem,
            implements: StageSignature::CLutElem,
            input_channels,
            output_channels,
            data: StageData::CLut(CLutData {
                params,
                table: CLutTable::Float(data),
                n_entries,
            }),
        })
    }

    /// Create a float CLUT stage with uniform grid.
    ///
    /// C版: `cmsStageAllocCLutFloat`
    pub fn new_clut_float_uniform(
        grid_points: u32,
        input_channels: u32,
        output_channels: u32,
        table: Option<&[f32]>,
    ) -> Option<Self> {
        use crate::curves::intrp::MAX_INPUT_DIMENSIONS;

        let dims = [grid_points; MAX_INPUT_DIMENSIONS];
        Self::new_clut_float(&dims, input_channels, output_channels, table)
    }

    /// Create an identity CLUT (2-point grid, input = output).
    ///
    /// C版: `_cmsStageAllocIdentityCLut`
    pub fn new_identity_clut(n: u32) -> Option<Self> {
        use crate::curves::intrp::MAX_INPUT_DIMENSIONS;

        let dims = [2u32; MAX_INPUT_DIMENSIONS];
        let mut stage = Self::new_clut_16bit(&dims, n, n, None)?;

        // Fill with identity via sampling
        let n_chan = n;
        sample_clut_16bit(
            &mut stage,
            |input, output, _| {
                output[..n_chan as usize].copy_from_slice(&input[..n_chan as usize]);
                true
            },
            SAMPLER_WRITE,
        );

        stage.implements = StageSignature::IdentityElem;
        Some(stage)
    }

    // --- Special stage constructors ---

    /// C版: `_cmsStageAllocLab2XYZ`
    pub fn new_lab_to_xyz() -> Option<Self> {
        Some(Stage {
            stage_type: StageSignature::Lab2XyzElem,
            implements: StageSignature::Lab2XyzElem,
            input_channels: 3,
            output_channels: 3,
            data: StageData::None,
        })
    }

    /// C版: `_cmsStageAllocXYZ2Lab`
    pub fn new_xyz_to_lab() -> Option<Self> {
        Some(Stage {
            stage_type: StageSignature::Xyz2LabElem,
            implements: StageSignature::Xyz2LabElem,
            input_channels: 3,
            output_channels: 3,
            data: StageData::None,
        })
    }

    /// C版: `_cmsStageClipNegatives`
    pub fn new_clip_negatives(n: u32) -> Option<Self> {
        if n == 0 || n as usize > intrp::MAX_STAGE_CHANNELS {
            return None;
        }
        Some(Stage {
            stage_type: StageSignature::ClipNegativesElem,
            implements: StageSignature::ClipNegativesElem,
            input_channels: n,
            output_channels: n,
            data: StageData::None,
        })
    }

    /// Matrix-based Lab V2→V4 conversion (65535/65280 scaling).
    ///
    /// C版: `_cmsStageAllocLabV2ToV4`
    pub fn new_lab_v2_to_v4() -> Option<Self> {
        let scale = 65535.0 / 65280.0;
        let matrix = [scale, 0.0, 0.0, 0.0, scale, 0.0, 0.0, 0.0, scale];
        let mut stage = Self::new_matrix(3, 3, &matrix, None)?;
        stage.implements = StageSignature::LabV2toV4;
        Some(stage)
    }

    /// Matrix-based Lab V4→V2 conversion (65280/65535 scaling).
    ///
    /// C版: `_cmsStageAllocLabV4ToV2`
    pub fn new_lab_v4_to_v2() -> Option<Self> {
        let scale = 65280.0 / 65535.0;
        let matrix = [scale, 0.0, 0.0, 0.0, scale, 0.0, 0.0, 0.0, scale];
        let mut stage = Self::new_matrix(3, 3, &matrix, None)?;
        stage.implements = StageSignature::LabV4toV2;
        Some(stage)
    }

    /// Curve-based Lab V2→V4 conversion (258-entry tables).
    ///
    /// C版: `_cmsStageAllocLabV2ToV4curves`
    pub fn new_lab_v2_to_v4_curves() -> Option<Self> {
        let mut tables = [[0u16; 258]; 3];
        for table in &mut tables {
            for (i, entry) in table[..257].iter_mut().enumerate() {
                *entry = ((i as u32 * 0xffff + 0x80) >> 8) as u16;
            }
            table[257] = 0xffff;
        }
        let c0 = ToneCurve::build_tabulated_16(&tables[0])?;
        let c1 = ToneCurve::build_tabulated_16(&tables[1])?;
        let c2 = ToneCurve::build_tabulated_16(&tables[2])?;
        let mut stage = Self::new_tone_curves(Some(&[c0, c1, c2]), 3)?;
        stage.implements = StageSignature::LabV2toV4;
        Some(stage)
    }

    /// Normalize Lab float values to [0..1] range.
    /// L: /100, a,b: (+128)/255
    ///
    /// C版: `_cmsStageNormalizeFromLabFloat`
    pub fn new_normalize_from_lab_float() -> Option<Self> {
        let matrix = [
            1.0 / 100.0,
            0.0,
            0.0,
            0.0,
            1.0 / 255.0,
            0.0,
            0.0,
            0.0,
            1.0 / 255.0,
        ];
        let offset = [0.0, 128.0 / 255.0, 128.0 / 255.0];
        let mut stage = Self::new_matrix(3, 3, &matrix, Some(&offset))?;
        stage.implements = StageSignature::Lab2FloatPCS;
        Some(stage)
    }

    /// Denormalize [0..1] range back to Lab float values.
    /// L: *100, a,b: *255 - 128
    ///
    /// C版: `_cmsStageNormalizeToLabFloat`
    pub fn new_normalize_to_lab_float() -> Option<Self> {
        let matrix = [100.0, 0.0, 0.0, 0.0, 255.0, 0.0, 0.0, 0.0, 255.0];
        let offset = [0.0, -128.0, -128.0];
        let mut stage = Self::new_matrix(3, 3, &matrix, Some(&offset))?;
        stage.implements = StageSignature::FloatPCS2Lab;
        Some(stage)
    }

    /// Normalize XYZ to float PCS (multiply by 32768/65535).
    ///
    /// C版: `_cmsStageNormalizeFromXyzFloat`
    pub fn new_normalize_from_xyz_float() -> Option<Self> {
        let n = 32768.0 / 65535.0;
        let matrix = [n, 0.0, 0.0, 0.0, n, 0.0, 0.0, 0.0, n];
        let mut stage = Self::new_matrix(3, 3, &matrix, None)?;
        stage.implements = StageSignature::Xyz2FloatPCS;
        Some(stage)
    }

    /// Denormalize float PCS back to XYZ (multiply by 65535/32768).
    ///
    /// C版: `_cmsStageNormalizeToXyzFloat`
    pub fn new_normalize_to_xyz_float() -> Option<Self> {
        let n = 65535.0 / 32768.0;
        let matrix = [n, 0.0, 0.0, 0.0, n, 0.0, 0.0, 0.0, n];
        let mut stage = Self::new_matrix(3, 3, &matrix, None)?;
        stage.implements = StageSignature::FloatPCS2Xyz;
        Some(stage)
    }

    /// Lab prelinearization curves (identity for L, S-shaped for a/b).
    ///
    /// C版: `_cmsStageAllocLabPrelin`
    pub fn new_lab_prelin() -> Option<Self> {
        let c_l = ToneCurve::build_gamma(1.0)?;
        let c_a = ToneCurve::build_parametric(108, &[2.4])?;
        let c_b = ToneCurve::build_parametric(108, &[2.4])?;
        Self::new_tone_curves(Some(&[c_l, c_a, c_b]), 3)
    }

    // --- Evaluation ---

    /// Evaluate this stage: transform input[] → output[].
    ///
    /// Dispatches based on `stage_type`.
    pub fn eval(&self, input: &[f32], output: &mut [f32]) {
        match self.stage_type {
            StageSignature::IdentityElem => {
                let n = self.input_channels as usize;
                output[..n].copy_from_slice(&input[..n]);
            }
            StageSignature::CurveSetElem => {
                self.eval_curves(input, output);
            }
            StageSignature::MatrixElem => {
                self.eval_matrix(input, output);
            }
            StageSignature::CLutElem => {
                self.eval_clut(input, output);
            }
            StageSignature::Lab2XyzElem => {
                self.eval_lab_to_xyz(input, output);
            }
            StageSignature::Xyz2LabElem => {
                self.eval_xyz_to_lab(input, output);
            }
            StageSignature::ClipNegativesElem => {
                let n = self.input_channels as usize;
                for i in 0..n {
                    output[i] = if input[i] < 0.0 { 0.0 } else { input[i] };
                }
            }
            _ => {
                debug_assert!(
                    false,
                    "Stage::eval: unhandled stage type {:?}",
                    self.stage_type
                );
                let n = self.input_channels.min(self.output_channels) as usize;
                output[..n].copy_from_slice(&input[..n]);
            }
        }
    }

    fn eval_curves(&self, input: &[f32], output: &mut [f32]) {
        if let StageData::Curves(curves) = &self.data {
            for (i, curve) in curves.iter().enumerate() {
                output[i] = curve.eval_f32(input[i]);
            }
        }
    }

    fn eval_matrix(&self, input: &[f32], output: &mut [f32]) {
        if let StageData::Matrix {
            coefficients,
            offset,
        } = &self.data
        {
            let rows = self.output_channels as usize;
            let cols = self.input_channels as usize;
            for i in 0..rows {
                let mut tmp: f64 = 0.0;
                for j in 0..cols {
                    tmp += input[j] as f64 * coefficients[i * cols + j];
                }
                if let Some(off) = offset {
                    tmp += off[i];
                }
                output[i] = tmp as f32;
            }
        }
    }

    fn eval_clut(&self, input: &[f32], output: &mut [f32]) {
        if let StageData::CLut(clut) = &self.data {
            match &clut.table {
                CLutTable::Float(table) => {
                    clut.params.eval_float(input, output, table);
                }
                CLutTable::U16(table) => {
                    let n_in = self.input_channels as usize;
                    let n_out = self.output_channels as usize;
                    let mut in16 = [0u16; intrp::MAX_STAGE_CHANNELS];
                    let mut out16 = [0u16; intrp::MAX_STAGE_CHANNELS];
                    float_to_16(&input[..n_in], &mut in16[..n_in]);
                    clut.params
                        .eval_16(&in16[..n_in], &mut out16[..n_out], table);
                    from_16_to_float(&out16[..n_out], &mut output[..n_out]);
                }
            }
        }
    }

    fn eval_lab_to_xyz(&self, input: &[f32], output: &mut [f32]) {
        use crate::math::pcs;
        use crate::types::{CieLab, CieXyz, D50_X, D50_Y, D50_Z};

        const XYZ_ADJ: f64 = 1.0 + 32767.0 / 32768.0;

        let lab = CieLab {
            l: input[0] as f64 * 100.0,
            a: input[1] as f64 * 255.0 - 128.0,
            b: input[2] as f64 * 255.0 - 128.0,
        };
        let white = CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        };
        let xyz = pcs::lab_to_xyz(&white, &lab);

        output[0] = (xyz.x / XYZ_ADJ) as f32;
        output[1] = (xyz.y / XYZ_ADJ) as f32;
        output[2] = (xyz.z / XYZ_ADJ) as f32;
    }

    fn eval_xyz_to_lab(&self, input: &[f32], output: &mut [f32]) {
        use crate::math::pcs;
        use crate::types::{CieXyz, D50_X, D50_Y, D50_Z};

        const XYZ_ADJ: f64 = 1.0 + 32767.0 / 32768.0;

        let xyz = CieXyz {
            x: input[0] as f64 * XYZ_ADJ,
            y: input[1] as f64 * XYZ_ADJ,
            z: input[2] as f64 * XYZ_ADJ,
        };
        let white = CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        };
        let lab = pcs::xyz_to_lab(&white, &xyz);

        output[0] = (lab.l / 100.0) as f32;
        output[1] = ((lab.a + 128.0) / 255.0) as f32;
        output[2] = ((lab.b + 128.0) / 255.0) as f32;
    }
}

// ============================================================================
// Pipeline
// ============================================================================

/// Insertion location for pipeline stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageLoc {
    AtBegin,
    AtEnd,
}

/// A chain of processing stages that transforms color data.
///
/// C版: `cmsPipeline`
#[derive(Clone)]
#[allow(dead_code)]
pub struct Pipeline {
    stages: Vec<Stage>,
    input_channels: u32,
    output_channels: u32,
    save_as_8bits: bool,
}

impl Pipeline {
    /// Create a new empty pipeline.
    ///
    /// C版: `cmsPipelineAlloc`
    pub fn new(input_channels: u32, output_channels: u32) -> Option<Self> {
        if input_channels as usize >= crate::types::MAX_CHANNELS
            || output_channels as usize >= crate::types::MAX_CHANNELS
        {
            return None;
        }
        Some(Pipeline {
            stages: Vec::new(),
            input_channels,
            output_channels,
            save_as_8bits: false,
        })
    }

    pub fn input_channels(&self) -> u32 {
        self.input_channels
    }

    pub fn output_channels(&self) -> u32 {
        self.output_channels
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn first_stage(&self) -> Option<&Stage> {
        self.stages.first()
    }

    pub fn last_stage(&self) -> Option<&Stage> {
        self.stages.last()
    }

    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }

    pub fn stages_mut(&mut self) -> &mut [Stage] {
        &mut self.stages
    }

    /// Insert a stage at the beginning or end of the pipeline.
    ///
    /// Rolls back on channel mismatch.
    ///
    /// C版: `cmsPipelineInsertStage`
    pub fn insert_stage(&mut self, loc: StageLoc, stage: Stage) -> bool {
        match loc {
            StageLoc::AtBegin => self.stages.insert(0, stage),
            StageLoc::AtEnd => self.stages.push(stage),
        }
        if !self.bless() {
            // Rollback
            match loc {
                StageLoc::AtBegin => {
                    self.stages.remove(0);
                }
                StageLoc::AtEnd => {
                    self.stages.pop();
                }
            }
            self.bless();
            return false;
        }
        true
    }

    /// Remove a stage from the beginning or end. Returns the removed stage.
    ///
    /// C版: `cmsPipelineUnlinkStage`
    pub fn remove_stage(&mut self, loc: StageLoc) -> Option<Stage> {
        if self.stages.is_empty() {
            return None;
        }
        let removed = match loc {
            StageLoc::AtBegin => self.stages.remove(0),
            StageLoc::AtEnd => self.stages.pop().unwrap(),
        };
        self.bless();
        Some(removed)
    }

    /// Concatenate another pipeline's stages (cloned) onto this one.
    ///
    /// Rolls back on channel mismatch.
    ///
    /// C版: `cmsPipelineCat`
    pub fn cat(&mut self, other: &Pipeline) -> bool {
        if self.stages.is_empty() && other.stages.is_empty() {
            self.input_channels = other.input_channels;
            self.output_channels = other.output_channels;
        }
        let prev_len = self.stages.len();
        for stage in &other.stages {
            self.stages.push(stage.clone());
        }
        if !self.bless() {
            self.stages.truncate(prev_len);
            self.bless();
            return false;
        }
        true
    }

    /// Evaluate the pipeline on float data.
    ///
    /// Uses a double-buffer (ping-pong) pattern with stack-allocated buffers.
    ///
    /// C版: `cmsPipelineEvalFloat`
    pub fn eval_float(&self, input: &[f32], output: &mut [f32]) {
        debug_assert!(
            input.len() >= self.input_channels as usize,
            "eval_float: input too short ({} < {})",
            input.len(),
            self.input_channels
        );
        debug_assert!(
            output.len() >= self.output_channels as usize,
            "eval_float: output too short ({} < {})",
            output.len(),
            self.output_channels
        );

        if self.stages.is_empty() {
            let n = self.input_channels.min(self.output_channels) as usize;
            output[..n].copy_from_slice(&input[..n]);
            return;
        }

        let mut buf_a = [0.0f32; intrp::MAX_STAGE_CHANNELS];
        let mut buf_b = [0.0f32; intrp::MAX_STAGE_CHANNELS];
        let mut phase = 0usize;

        let n_in = self.input_channels as usize;
        buf_a[..n_in].copy_from_slice(&input[..n_in]);

        for stage in &self.stages {
            if phase == 0 {
                stage.eval(&buf_a, &mut buf_b);
            } else {
                stage.eval(&buf_b, &mut buf_a);
            }
            phase ^= 1;
        }

        let n_out = self.output_channels as usize;
        if phase == 0 {
            output[..n_out].copy_from_slice(&buf_a[..n_out]);
        } else {
            output[..n_out].copy_from_slice(&buf_b[..n_out]);
        }
    }

    /// Evaluate the pipeline on 16-bit data.
    ///
    /// Converts u16→f32, evaluates float pipeline, converts back f32→u16.
    ///
    /// C版: `cmsPipelineEval16`
    pub fn eval_16(&self, input: &[u16], output: &mut [u16]) {
        debug_assert!(
            input.len() >= self.input_channels as usize,
            "eval_16: input too short ({} < {})",
            input.len(),
            self.input_channels
        );
        debug_assert!(
            output.len() >= self.output_channels as usize,
            "eval_16: output too short ({} < {})",
            output.len(),
            self.output_channels
        );

        let n_in = self.input_channels as usize;
        let n_out = self.output_channels as usize;

        let mut float_in = [0.0f32; intrp::MAX_STAGE_CHANNELS];
        let mut float_out = [0.0f32; intrp::MAX_STAGE_CHANNELS];

        from_16_to_float(&input[..n_in], &mut float_in[..n_in]);
        self.eval_float(&float_in[..n_in], &mut float_out[..n_out]);
        float_to_16(&float_out[..n_out], &mut output[..n_out]);
    }

    pub fn set_save_as_8bits(&mut self, on: bool) -> bool {
        let prev = self.save_as_8bits;
        self.save_as_8bits = on;
        prev
    }

    pub fn save_as_8bits(&self) -> bool {
        self.save_as_8bits
    }

    /// Check if stages match a pattern of types and return their indices.
    ///
    /// C版: `cmsPipelineCheckAndRetreiveStages`
    pub fn check_and_retrieve_stages(&self, types: &[StageSignature]) -> Option<Vec<usize>> {
        if self.stages.len() != types.len() {
            return None;
        }
        for (stage, &expected) in self.stages.iter().zip(types.iter()) {
            if stage.stage_type() != expected {
                return None;
            }
        }
        Some((0..types.len()).collect())
    }

    /// Evaluate the pipeline in reverse using Newton's method.
    ///
    /// Only works for 3→3 or 4→3 pipelines.
    /// Returns `true` and stores the best approximation in `result`.
    /// Returns `false` only if the pipeline dimensions are wrong or
    /// the Jacobian is singular. Matching C behavior, `true` is returned
    /// even when the iteration doesn't fully converge — `result` will
    /// contain the best approximation found.
    ///
    /// C版: `cmsPipelineEvalReverseFloat`
    pub fn eval_reverse_float(
        &self,
        target: &[f32],
        result: &mut [f32],
        hint: Option<&[f32]>,
    ) -> bool {
        use crate::math::mtrx::{Mat3, Vec3};

        const JACOBIAN_EPSILON: f32 = 0.001;
        const MAX_ITERATIONS: usize = 30;

        if self.input_channels != 3 && self.input_channels != 4 {
            return false;
        }
        if self.output_channels != 3 {
            return false;
        }

        debug_assert!(
            target.len() >= self.output_channels as usize,
            "eval_reverse_float: target too short"
        );
        debug_assert!(
            result.len() >= self.input_channels as usize,
            "eval_reverse_float: result too short"
        );
        if self.input_channels == 4 {
            debug_assert!(
                target.len() >= 4,
                "eval_reverse_float: target needs 4 elements for 4→3 pipeline"
            );
        }

        let mut x = [0.0f32; 4];
        match hint {
            Some(h) => {
                x[0] = h[0];
                x[1] = h[1];
                x[2] = h[2];
            }
            None => {
                x[0] = 0.3;
                x[1] = 0.3;
                x[2] = 0.3;
            }
        }
        if self.input_channels == 4 {
            x[3] = target[3];
        }

        let mut last_error: f64 = 1e20;

        for _ in 0..MAX_ITERATIONS {
            let mut fx = [0.0f32; 4];
            self.eval_float(&x, &mut fx);

            let error = {
                let mut sum = 0.0f64;
                for i in 0..3 {
                    let d = (fx[i] - target[i]) as f64;
                    sum += d * d;
                }
                sum.sqrt()
            };

            if error >= last_error {
                break;
            }

            last_error = error;
            result[..self.input_channels as usize]
                .copy_from_slice(&x[..self.input_channels as usize]);

            if error <= 0.0 {
                break;
            }

            // Build Jacobian
            let mut jacobian = Mat3::identity();
            for j in 0..3 {
                let mut xd = x;
                if xd[j] < 1.0 - JACOBIAN_EPSILON {
                    xd[j] += JACOBIAN_EPSILON;
                } else {
                    xd[j] -= JACOBIAN_EPSILON;
                }
                let mut fxd = [0.0f32; 4];
                self.eval_float(&xd, &mut fxd);

                jacobian.0[0].0[j] = ((fxd[0] - fx[0]) / JACOBIAN_EPSILON) as f64;
                jacobian.0[1].0[j] = ((fxd[1] - fx[1]) / JACOBIAN_EPSILON) as f64;
                jacobian.0[2].0[j] = ((fxd[2] - fx[2]) / JACOBIAN_EPSILON) as f64;
            }

            let residual = Vec3::new(
                (fx[0] - target[0]) as f64,
                (fx[1] - target[1]) as f64,
                (fx[2] - target[2]) as f64,
            );

            let Some(delta) = jacobian.solve(&residual) else {
                return false;
            };

            x[0] -= delta.0[0] as f32;
            x[1] -= delta.0[1] as f32;
            x[2] -= delta.0[2] as f32;

            for v in &mut x[..3] {
                *v = v.clamp(0.0, 1.0);
            }
        }

        true
    }

    /// Update pipeline input/output channels from first/last stage.
    /// Validate that adjacent stages have compatible channel counts.
    ///
    /// C版: `BlessLUT`
    fn bless(&mut self) -> bool {
        if self.stages.is_empty() {
            return true;
        }

        // Validate adjacency first, before modifying state
        for w in self.stages.windows(2) {
            if w[0].output_channels() != w[1].input_channels() {
                return false;
            }
        }

        self.input_channels = self.stages.first().unwrap().input_channels();
        self.output_channels = self.stages.last().unwrap().output_channels();
        true
    }
}

// ============================================================================
// CLUT Sampling
// ============================================================================

/// Sample (iterate over) all nodes of a 16-bit CLUT, calling `sampler` for each.
///
/// The sampler receives `(input, output, cargo)` where input is the quantized
/// node position and output is the current table values. If `flags` does not
/// include `SAMPLER_INSPECT`, the sampler's output is written back to the table.
///
/// C版: `cmsStageSampleCLut16bit`
#[allow(dead_code)]
pub fn sample_clut_16bit<F>(stage: &mut Stage, mut sampler: F, flags: u32) -> bool
where
    F: FnMut(&[u16], &mut [u16], &()) -> bool,
{
    let StageData::CLut(clut) = &mut stage.data else {
        return false;
    };
    let CLutTable::U16(table) = &mut clut.table else {
        return false;
    };

    let n_inputs = clut.params.n_inputs as usize;
    let n_outputs = clut.params.n_outputs as usize;
    let n_samples = &clut.params.n_samples;

    let total_nodes = match cube_size(n_samples, n_inputs as u32) {
        Some(n) => n as usize,
        None => return false,
    };

    let mut input = [0u16; intrp::MAX_INPUT_DIMENSIONS + 1];
    let mut output = [0u16; intrp::MAX_STAGE_CHANNELS];
    let mut index = 0;

    for i in 0..total_nodes {
        let mut rest = i;
        for t in (0..n_inputs).rev() {
            let colorant = rest % n_samples[t] as usize;
            rest /= n_samples[t] as usize;
            input[t] = quantize_val(colorant as f64, n_samples[t]);
        }

        output[..n_outputs].copy_from_slice(&table[index..index + n_outputs]);

        if !sampler(&input[..n_inputs], &mut output[..n_outputs], &()) {
            return false;
        }

        if flags & SAMPLER_INSPECT == 0 {
            table[index..index + n_outputs].copy_from_slice(&output[..n_outputs]);
        }

        index += n_outputs;
    }

    true
}

/// Sample (iterate over) all nodes of a float CLUT.
///
/// C版: `cmsStageSampleCLutFloat`
#[allow(dead_code)]
pub fn sample_clut_float<F>(stage: &mut Stage, mut sampler: F, flags: u32) -> bool
where
    F: FnMut(&[f32], &mut [f32], &()) -> bool,
{
    let StageData::CLut(clut) = &mut stage.data else {
        return false;
    };
    let CLutTable::Float(table) = &mut clut.table else {
        return false;
    };

    let n_inputs = clut.params.n_inputs as usize;
    let n_outputs = clut.params.n_outputs as usize;
    let n_samples = &clut.params.n_samples;

    let total_nodes = match cube_size(n_samples, n_inputs as u32) {
        Some(n) => n as usize,
        None => return false,
    };

    let mut input = [0.0f32; intrp::MAX_INPUT_DIMENSIONS + 1];
    let mut output = [0.0f32; intrp::MAX_STAGE_CHANNELS];
    let mut index = 0;

    for i in 0..total_nodes {
        let mut rest = i;
        for t in (0..n_inputs).rev() {
            let colorant = rest % n_samples[t] as usize;
            rest /= n_samples[t] as usize;
            input[t] = quantize_val(colorant as f64, n_samples[t]) as f32 / 65535.0;
        }

        output[..n_outputs].copy_from_slice(&table[index..index + n_outputs]);

        if !sampler(&input[..n_inputs], &mut output[..n_outputs], &()) {
            return false;
        }

        if flags & SAMPLER_INSPECT == 0 {
            table[index..index + n_outputs].copy_from_slice(&output[..n_outputs]);
        }

        index += n_outputs;
    }

    true
}

/// Iterate over all nodes of a hypercube (16-bit version).
///
/// C版: `cmsSliceSpace16`
#[allow(dead_code)]
pub fn slice_space_16<F>(n_inputs: u32, clut_points: &[u32], mut sampler: F) -> bool
where
    F: FnMut(&[u16], &()) -> bool,
{
    if n_inputs as usize >= crate::types::MAX_CHANNELS {
        return false;
    }

    let total_nodes = match cube_size(clut_points, n_inputs) {
        Some(n) => n as usize,
        None => return false,
    };

    let mut input = [0u16; crate::types::MAX_CHANNELS];

    for i in 0..total_nodes {
        let mut rest = i;
        for t in (0..n_inputs as usize).rev() {
            let colorant = rest % clut_points[t] as usize;
            rest /= clut_points[t] as usize;
            input[t] = quantize_val(colorant as f64, clut_points[t]);
        }

        if !sampler(&input[..n_inputs as usize], &()) {
            return false;
        }
    }

    true
}

/// Iterate over all nodes of a hypercube (float version).
///
/// C版: `cmsSliceSpaceFloat`
#[allow(dead_code)]
pub fn slice_space_float<F>(n_inputs: u32, clut_points: &[u32], mut sampler: F) -> bool
where
    F: FnMut(&[f32], &()) -> bool,
{
    if n_inputs as usize >= crate::types::MAX_CHANNELS {
        return false;
    }

    let total_nodes = match cube_size(clut_points, n_inputs) {
        Some(n) => n as usize,
        None => return false,
    };

    let mut input = [0.0f32; crate::types::MAX_CHANNELS];

    for i in 0..total_nodes {
        let mut rest = i;
        for t in (0..n_inputs as usize).rev() {
            let colorant = rest % clut_points[t] as usize;
            rest /= clut_points[t] as usize;
            input[t] = quantize_val(colorant as f64, clut_points[t]) as f32 / 65535.0;
        }

        if !sampler(&input[..n_inputs as usize], &()) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Stage: Identity
    // ========================================================================

    #[test]
    fn stage_identity_passthrough() {
        let stage = Stage::new_identity(3).unwrap();
        let input = [0.25f32, 0.5, 0.75];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);
        assert_eq!(output, input);
    }

    #[test]
    fn stage_identity_channel_count() {
        let stage = Stage::new_identity(4).unwrap();
        assert_eq!(stage.input_channels(), 4);
        assert_eq!(stage.output_channels(), 4);
        assert_eq!(stage.stage_type(), StageSignature::IdentityElem);
    }

    // ========================================================================
    // Stage: Curves
    // ========================================================================

    #[test]
    fn stage_curves_gamma() {
        let curve = ToneCurve::build_gamma(2.2).unwrap();
        let stage =
            Stage::new_tone_curves(Some(&[curve.clone(), curve.clone(), curve]), 3).unwrap();
        assert_eq!(stage.stage_type(), StageSignature::CurveSetElem);
        assert_eq!(stage.input_channels(), 3);
        assert_eq!(stage.output_channels(), 3);

        let input = [0.5f32, 0.5, 0.5];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        let expected = 0.5f64.powf(2.2) as f32;
        for &v in &output {
            assert!((v - expected).abs() < 0.01, "got {v}, expected {expected}");
        }
    }

    #[test]
    fn stage_curves_per_channel() {
        let c1 = ToneCurve::build_gamma(1.0).unwrap();
        let c2 = ToneCurve::build_gamma(2.0).unwrap();
        let c3 = ToneCurve::build_gamma(3.0).unwrap();
        let stage = Stage::new_tone_curves(Some(&[c1, c2, c3]), 3).unwrap();

        let input = [0.5f32, 0.5, 0.5];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        assert!((output[0] - 0.5).abs() < 0.01);
        assert!((output[1] - 0.25).abs() < 0.01);
        assert!((output[2] - 0.125).abs() < 0.01);
    }

    #[test]
    fn stage_curves_none_is_identity() {
        let stage = Stage::new_tone_curves(None, 3).unwrap();
        let input = [0.3f32, 0.6, 0.9];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        for i in 0..3 {
            assert!(
                (output[i] - input[i]).abs() < 0.001,
                "ch {i}: got {}, expected {}",
                output[i],
                input[i]
            );
        }
    }

    // ========================================================================
    // Stage: Matrix
    // ========================================================================

    #[test]
    fn stage_matrix_identity() {
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let stage = Stage::new_matrix(3, 3, &matrix, None).unwrap();
        assert_eq!(stage.stage_type(), StageSignature::MatrixElem);
        assert_eq!(stage.input_channels(), 3);
        assert_eq!(stage.output_channels(), 3);

        let input = [0.2f32, 0.4, 0.6];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        for i in 0..3 {
            assert!(
                (output[i] - input[i]).abs() < 1e-6,
                "ch {i}: got {}, expected {}",
                output[i],
                input[i]
            );
        }
    }

    #[test]
    fn stage_matrix_scale() {
        let matrix = [2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0];
        let stage = Stage::new_matrix(3, 3, &matrix, None).unwrap();

        let input = [0.1f32, 0.2, 0.3];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        assert!((output[0] - 0.2).abs() < 1e-6);
        assert!((output[1] - 0.6).abs() < 1e-6);
        assert!((output[2] - 1.2).abs() < 1e-5);
    }

    #[test]
    fn stage_matrix_with_offset() {
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let offset = [0.1, 0.2, 0.3];
        let stage = Stage::new_matrix(3, 3, &matrix, Some(&offset)).unwrap();

        let input = [0.0f32; 3];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);

        assert!((output[0] - 0.1).abs() < 1e-6);
        assert!((output[1] - 0.2).abs() < 1e-6);
        assert!((output[2] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn stage_matrix_invalid_dims() {
        assert!(Stage::new_matrix(0, 3, &[], None).is_none());
        assert!(Stage::new_matrix(3, 0, &[], None).is_none());
    }

    // ========================================================================
    // Stage: Clone
    // ========================================================================

    // ========================================================================
    // Stage: CLUT
    // ========================================================================

    #[test]

    fn clut_16bit_identity() {
        // 2-point uniform grid, 3in→3out identity table
        // Grid points: [0, 65535] per dimension
        // Table: for a 2^3 = 8 node table with 3 outputs = 24 entries
        // Each node maps its quantized input to the same output
        let mut table = vec![0u16; 8 * 3];
        for r in 0..2u16 {
            for g in 0..2u16 {
                for b in 0..2u16 {
                    let idx = (r * 4 + g * 2 + b) as usize * 3;
                    table[idx] = r * 65535;
                    table[idx + 1] = g * 65535;
                    table[idx + 2] = b * 65535;
                }
            }
        }
        let stage = Stage::new_clut_16bit_uniform(2, 3, 3, Some(&table)).unwrap();
        assert_eq!(stage.stage_type(), StageSignature::CLutElem);
        assert_eq!(stage.input_channels(), 3);
        assert_eq!(stage.output_channels(), 3);

        // Test corners
        let mut output = [0.0f32; 3];
        stage.eval(&[0.0, 0.0, 0.0], &mut output);
        assert!(output[0].abs() < 0.01);
        assert!(output[1].abs() < 0.01);
        assert!(output[2].abs() < 0.01);

        stage.eval(&[1.0, 1.0, 1.0], &mut output);
        assert!((output[0] - 1.0).abs() < 0.01);
        assert!((output[1] - 1.0).abs() < 0.01);
        assert!((output[2] - 1.0).abs() < 0.01);
    }

    #[test]

    fn clut_float_identity() {
        let mut table = vec![0.0f32; 8 * 3];
        for r in 0..2u32 {
            for g in 0..2u32 {
                for b in 0..2u32 {
                    let idx = (r * 4 + g * 2 + b) as usize * 3;
                    table[idx] = r as f32;
                    table[idx + 1] = g as f32;
                    table[idx + 2] = b as f32;
                }
            }
        }
        let stage = Stage::new_clut_float_uniform(2, 3, 3, Some(&table)).unwrap();

        let mut output = [0.0f32; 3];
        stage.eval(&[0.5, 0.5, 0.5], &mut output);
        for &v in &output {
            assert!((v - 0.5).abs() < 0.01, "got {v}, expected 0.5");
        }
    }

    #[test]

    fn clut_granular_grid() {
        // Non-uniform grid: 3 points on dim 0, 2 on dim 1
        let grid = [3u32, 2];
        // 3 * 2 = 6 nodes, 1 output each = 6 entries
        let table: Vec<f32> = (0..6).map(|i| i as f32 / 5.0).collect();
        let stage = Stage::new_clut_float(&grid, 2, 1, Some(&table)).unwrap();
        assert_eq!(stage.input_channels(), 2);
        assert_eq!(stage.output_channels(), 1);
    }

    #[test]

    fn clut_table_none_zeros() {
        // None table should zero-initialize
        let stage = Stage::new_clut_16bit_uniform(2, 3, 3, None).unwrap();
        let mut output = [1.0f32; 3];
        stage.eval(&[0.5, 0.5, 0.5], &mut output);
        // All zeros in table → output should be ~0
        for &v in &output {
            assert!(v.abs() < 0.01, "expected ~0, got {v}");
        }
    }

    #[test]

    fn clut_identity_clut() {
        let stage = Stage::new_identity_clut(3).unwrap();
        assert_eq!(stage.stage_type(), StageSignature::CLutElem);
        assert_eq!(stage.implements(), StageSignature::IdentityElem);

        let mut output = [0.0f32; 3];
        stage.eval(&[0.3, 0.6, 0.9], &mut output);
        assert!((output[0] - 0.3).abs() < 0.02);
        assert!((output[1] - 0.6).abs() < 0.02);
        assert!((output[2] - 0.9).abs() < 0.02);
    }

    #[test]

    fn clut_too_many_inputs() {
        assert!(Stage::new_clut_16bit_uniform(2, 16, 3, None).is_none());
    }

    #[test]

    fn clut_clone() {
        let mut table = vec![0.0f32; 8 * 3];
        for r in 0..2u32 {
            for g in 0..2u32 {
                for b in 0..2u32 {
                    let idx = (r * 4 + g * 2 + b) as usize * 3;
                    table[idx] = r as f32;
                    table[idx + 1] = g as f32;
                    table[idx + 2] = b as f32;
                }
            }
        }
        let stage = Stage::new_clut_float_uniform(2, 3, 3, Some(&table)).unwrap();
        let cloned = stage.clone();

        let input = [0.5f32, 0.5, 0.5];
        let mut out1 = [0.0f32; 3];
        let mut out2 = [0.0f32; 3];
        stage.eval(&input, &mut out1);
        cloned.eval(&input, &mut out2);
        assert_eq!(out1, out2);
    }

    // ========================================================================
    // Stage: Clone
    // ========================================================================

    #[test]
    fn stage_clone() {
        let curve = ToneCurve::build_gamma(2.2).unwrap();
        let stage =
            Stage::new_tone_curves(Some(&[curve.clone(), curve.clone(), curve]), 3).unwrap();
        let cloned = stage.clone();

        let input = [0.5f32, 0.5, 0.5];
        let mut out1 = [0.0f32; 3];
        let mut out2 = [0.0f32; 3];
        stage.eval(&input, &mut out1);
        cloned.eval(&input, &mut out2);

        assert_eq!(out1, out2);
        assert_eq!(cloned.stage_type(), stage.stage_type());
        assert_eq!(cloned.input_channels(), stage.input_channels());
    }

    // ========================================================================
    // Pipeline
    // ========================================================================

    #[test]

    fn pipeline_empty() {
        let p = Pipeline::new(3, 3).unwrap();
        assert_eq!(p.input_channels(), 3);
        assert_eq!(p.output_channels(), 3);
        assert_eq!(p.stage_count(), 0);
        assert!(p.first_stage().is_none());
        assert!(p.last_stage().is_none());

        // Empty pipeline: eval should copy input to output
        let input = [0.2f32, 0.4, 0.6];
        let mut output = [0.0f32; 3];
        p.eval_float(&input, &mut output);
        for i in 0..3 {
            assert!(
                (output[i] - input[i]).abs() < 1e-6,
                "ch {i}: got {}, expected {}",
                output[i],
                input[i]
            );
        }
    }

    #[test]

    fn pipeline_insert_at_end() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let s1 = Stage::new_identity(3).unwrap();
        let s2 = Stage::new_tone_curves(None, 3).unwrap();
        assert!(p.insert_stage(StageLoc::AtEnd, s1));
        assert!(p.insert_stage(StageLoc::AtEnd, s2));
        assert_eq!(p.stage_count(), 2);
        assert_eq!(
            p.first_stage().unwrap().stage_type(),
            StageSignature::IdentityElem
        );
        assert_eq!(
            p.last_stage().unwrap().stage_type(),
            StageSignature::CurveSetElem
        );
    }

    #[test]

    fn pipeline_insert_at_begin() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let s1 = Stage::new_identity(3).unwrap();
        let s2 = Stage::new_tone_curves(None, 3).unwrap();
        assert!(p.insert_stage(StageLoc::AtEnd, s1));
        assert!(p.insert_stage(StageLoc::AtBegin, s2));
        assert_eq!(p.stage_count(), 2);
        assert_eq!(
            p.first_stage().unwrap().stage_type(),
            StageSignature::CurveSetElem
        );
        assert_eq!(
            p.last_stage().unwrap().stage_type(),
            StageSignature::IdentityElem
        );
    }

    #[test]

    fn pipeline_remove() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let s1 = Stage::new_identity(3).unwrap();
        let s2 = Stage::new_tone_curves(None, 3).unwrap();
        let matrix = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0];
        let s3 = Stage::new_matrix(3, 3, &matrix, None).unwrap();
        p.insert_stage(StageLoc::AtEnd, s1);
        p.insert_stage(StageLoc::AtEnd, s2);
        p.insert_stage(StageLoc::AtEnd, s3);
        assert_eq!(p.stage_count(), 3);

        // Remove first
        let removed = p.remove_stage(StageLoc::AtBegin);
        assert_eq!(removed.unwrap().stage_type(), StageSignature::IdentityElem);
        assert_eq!(p.stage_count(), 2);

        // Remove last
        let removed = p.remove_stage(StageLoc::AtEnd);
        assert_eq!(removed.unwrap().stage_type(), StageSignature::MatrixElem);
        assert_eq!(p.stage_count(), 1);
    }

    #[test]

    fn pipeline_eval_float_single_stage() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let matrix = [2.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 4.0];
        let stage = Stage::new_matrix(3, 3, &matrix, None).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage);

        let input = [0.1f32, 0.2, 0.25];
        let mut output = [0.0f32; 3];
        p.eval_float(&input, &mut output);

        assert!((output[0] - 0.2).abs() < 1e-5);
        assert!((output[1] - 0.6).abs() < 1e-5);
        assert!((output[2] - 1.0).abs() < 1e-5);
    }

    #[test]

    fn pipeline_eval_float_chain() {
        // curves(gamma 2.0) → matrix(scale 2x) → curves(gamma 0.5 = sqrt)
        let mut p = Pipeline::new(3, 3).unwrap();

        let curve_sq = ToneCurve::build_gamma(2.0).unwrap();
        let s1 = Stage::new_tone_curves(Some(&[curve_sq.clone(), curve_sq.clone(), curve_sq]), 3)
            .unwrap();

        let matrix = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0];
        let s2 = Stage::new_matrix(3, 3, &matrix, None).unwrap();

        let curve_sqrt = ToneCurve::build_gamma(0.5).unwrap();
        let s3 = Stage::new_tone_curves(
            Some(&[curve_sqrt.clone(), curve_sqrt.clone(), curve_sqrt]),
            3,
        )
        .unwrap();

        p.insert_stage(StageLoc::AtEnd, s1);
        p.insert_stage(StageLoc::AtEnd, s2);
        p.insert_stage(StageLoc::AtEnd, s3);

        // input 0.5 → sq(0.5)=0.25 → *2=0.5 → sqrt(0.5)≈0.707
        let input = [0.5f32, 0.5, 0.5];
        let mut output = [0.0f32; 3];
        p.eval_float(&input, &mut output);

        let expected = (0.5f64.powi(2) * 2.0).sqrt() as f32;
        for &v in &output {
            assert!((v - expected).abs() < 0.02, "got {v}, expected {expected}");
        }
    }

    #[test]

    fn pipeline_eval_16() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let s = Stage::new_identity(3).unwrap();
        p.insert_stage(StageLoc::AtEnd, s);

        let input: [u16; 3] = [0, 32768, 65535];
        let mut output = [0u16; 3];
        p.eval_16(&input, &mut output);

        // Identity should preserve values (within rounding)
        assert!((output[0] as i32).abs() <= 1);
        assert!((output[1] as i32 - 32768).abs() <= 1);
        assert!((output[2] as i32 - 65535).abs() <= 1);
    }

    #[test]

    fn pipeline_cat() {
        let mut p1 = Pipeline::new(3, 3).unwrap();
        let matrix1 = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0];
        p1.insert_stage(
            StageLoc::AtEnd,
            Stage::new_matrix(3, 3, &matrix1, None).unwrap(),
        );

        let mut p2 = Pipeline::new(3, 3).unwrap();
        let matrix2 = [3.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 3.0];
        p2.insert_stage(
            StageLoc::AtEnd,
            Stage::new_matrix(3, 3, &matrix2, None).unwrap(),
        );

        assert!(p1.cat(&p2));
        assert_eq!(p1.stage_count(), 2);

        let input = [0.1f32, 0.1, 0.1];
        let mut output = [0.0f32; 3];
        p1.eval_float(&input, &mut output);

        // 0.1 * 2.0 * 3.0 = 0.6
        for &v in &output {
            assert!((v - 0.6).abs() < 1e-5, "got {v}, expected 0.6");
        }
    }

    #[test]

    fn pipeline_clone() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let curve = ToneCurve::build_gamma(2.2).unwrap();
        p.insert_stage(
            StageLoc::AtEnd,
            Stage::new_tone_curves(Some(&[curve.clone(), curve.clone(), curve]), 3).unwrap(),
        );

        let p2 = p.clone();
        assert_eq!(p2.stage_count(), 1);

        let input = [0.5f32, 0.5, 0.5];
        let mut out1 = [0.0f32; 3];
        let mut out2 = [0.0f32; 3];
        p.eval_float(&input, &mut out1);
        p2.eval_float(&input, &mut out2);
        assert_eq!(out1, out2);
    }

    #[test]

    fn pipeline_check_and_retrieve_stages() {
        let mut p = Pipeline::new(3, 3).unwrap();
        p.insert_stage(StageLoc::AtEnd, Stage::new_tone_curves(None, 3).unwrap());
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        p.insert_stage(
            StageLoc::AtEnd,
            Stage::new_matrix(3, 3, &matrix, None).unwrap(),
        );

        // Matching pattern
        let result = p
            .check_and_retrieve_stages(&[StageSignature::CurveSetElem, StageSignature::MatrixElem]);
        assert!(result.is_some());
        let indices = result.unwrap();
        assert_eq!(indices, vec![0, 1]);

        // Non-matching pattern
        let result = p
            .check_and_retrieve_stages(&[StageSignature::MatrixElem, StageSignature::CurveSetElem]);
        assert!(result.is_none());

        // Wrong count
        let result = p.check_and_retrieve_stages(&[StageSignature::CurveSetElem]);
        assert!(result.is_none());
    }

    // ========================================================================
    // Special stages
    // ========================================================================

    #[test]

    fn stage_lab_to_xyz_roundtrip() {
        let lab2xyz = Stage::new_lab_to_xyz().unwrap();
        let xyz2lab = Stage::new_xyz_to_lab().unwrap();

        // D50 white: Lab (100, 0, 0) → normalized (1.0, 0.502, 0.502)
        let lab_norm = [1.0f32, 128.0 / 255.0, 128.0 / 255.0];
        let mut xyz_out = [0.0f32; 3];
        let mut lab_back = [0.0f32; 3];
        lab2xyz.eval(&lab_norm, &mut xyz_out);
        xyz2lab.eval(&xyz_out, &mut lab_back);

        for i in 0..3 {
            assert!(
                (lab_back[i] - lab_norm[i]).abs() < 0.001,
                "ch {i}: got {}, expected {}",
                lab_back[i],
                lab_norm[i]
            );
        }
    }

    #[test]

    fn stage_clip_negatives() {
        let stage = Stage::new_clip_negatives(3).unwrap();
        let input = [-0.5f32, 0.0, 0.5];
        let mut output = [0.0f32; 3];
        stage.eval(&input, &mut output);
        assert_eq!(output[0], 0.0);
        assert_eq!(output[1], 0.0);
        assert_eq!(output[2], 0.5);
    }

    #[test]

    fn stage_lab_v2_v4_roundtrip() {
        let v2_to_v4 = Stage::new_lab_v2_to_v4().unwrap();
        let v4_to_v2 = Stage::new_lab_v4_to_v2().unwrap();

        let input = [0.5f32, 0.5, 0.5];
        let mut mid = [0.0f32; 3];
        let mut output = [0.0f32; 3];
        v2_to_v4.eval(&input, &mut mid);
        v4_to_v2.eval(&mid, &mut output);

        for i in 0..3 {
            assert!(
                (output[i] - input[i]).abs() < 0.001,
                "ch {i}: got {}, expected {}",
                output[i],
                input[i]
            );
        }
    }

    #[test]

    fn stage_normalize_lab_float_roundtrip() {
        let norm_from = Stage::new_normalize_from_lab_float().unwrap();
        let norm_to = Stage::new_normalize_to_lab_float().unwrap();

        // Lab (50, 0, 0) → normalized → Lab back
        let lab_input = [50.0f32, 0.0, 0.0];
        let mut norm = [0.0f32; 3];
        let mut lab_back = [0.0f32; 3];
        norm_from.eval(&lab_input, &mut norm);
        norm_to.eval(&norm, &mut lab_back);

        for i in 0..3 {
            assert!(
                (lab_back[i] - lab_input[i]).abs() < 0.01,
                "ch {i}: got {}, expected {}",
                lab_back[i],
                lab_input[i]
            );
        }
    }

    #[test]

    fn stage_normalize_xyz_float_roundtrip() {
        let norm_from = Stage::new_normalize_from_xyz_float().unwrap();
        let norm_to = Stage::new_normalize_to_xyz_float().unwrap();

        let xyz_input = [0.5f32, 0.5, 0.5];
        let mut norm = [0.0f32; 3];
        let mut xyz_back = [0.0f32; 3];
        norm_from.eval(&xyz_input, &mut norm);
        norm_to.eval(&norm, &mut xyz_back);

        for i in 0..3 {
            assert!(
                (xyz_back[i] - xyz_input[i]).abs() < 0.001,
                "ch {i}: got {}, expected {}",
                xyz_back[i],
                xyz_input[i]
            );
        }
    }

    // ========================================================================
    // Sampling
    // ========================================================================

    #[test]

    fn sample_clut_16bit_identity() {
        let mut stage = Stage::new_clut_16bit_uniform(2, 3, 3, None).unwrap();
        // Fill with identity
        let result = sample_clut_16bit(
            &mut stage,
            |input, output, _| {
                output[..3].copy_from_slice(&input[..3]);
                true
            },
            SAMPLER_WRITE,
        );
        assert!(result);

        // Evaluate: should be near-identity
        let mut output = [0.0f32; 3];
        stage.eval(&[0.0, 0.0, 0.0], &mut output);
        assert!(output[0].abs() < 0.01);
        stage.eval(&[1.0, 1.0, 1.0], &mut output);
        assert!((output[0] - 1.0).abs() < 0.01);
    }

    #[test]

    fn sample_clut_16bit_inspect() {
        let mut stage = Stage::new_clut_16bit_uniform(2, 3, 3, None).unwrap();
        // Fill first
        sample_clut_16bit(
            &mut stage,
            |input, output, _| {
                output[..3].copy_from_slice(&input[..3]);
                true
            },
            SAMPLER_WRITE,
        );

        // Inspect mode: count nodes without modifying
        let mut count = 0u32;
        sample_clut_16bit(
            &mut stage,
            |_, _, _| {
                count += 1;
                true
            },
            SAMPLER_INSPECT,
        );
        assert_eq!(count, 8); // 2^3
    }

    #[test]

    fn slice_space_16_all_nodes() {
        let points = [3u32, 2];
        let mut count = 0u32;
        let result = slice_space_16(2, &points, |_, _| {
            count += 1;
            true
        });
        assert!(result);
        assert_eq!(count, 6); // 3 * 2
    }

    // ========================================================================
    // Reverse evaluation
    // ========================================================================

    #[test]

    fn pipeline_eval_reverse_float_identity() {
        let mut p = Pipeline::new(3, 3).unwrap();
        p.insert_stage(StageLoc::AtEnd, Stage::new_identity(3).unwrap());

        let target = [0.3f32, 0.5, 0.7];
        let mut result = [0.0f32; 3];
        assert!(p.eval_reverse_float(&target, &mut result, None));

        for i in 0..3 {
            assert!(
                (result[i] - target[i]).abs() < 0.01,
                "ch {i}: got {}, expected {}",
                result[i],
                target[i]
            );
        }
    }

    #[test]

    fn pipeline_eval_reverse_3to3() {
        // Scale by 2 pipeline, reverse should give half
        let mut p = Pipeline::new(3, 3).unwrap();
        let matrix = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0];
        p.insert_stage(
            StageLoc::AtEnd,
            Stage::new_matrix(3, 3, &matrix, None).unwrap(),
        );

        let target = [0.6f32, 0.8, 1.0];
        let mut result = [0.0f32; 3];
        assert!(p.eval_reverse_float(&target, &mut result, None));

        assert!((result[0] - 0.3).abs() < 0.01);
        assert!((result[1] - 0.4).abs() < 0.01);
        assert!((result[2] - 0.5).abs() < 0.01);
    }

    #[test]

    fn pipeline_eval_reverse_wrong_dims() {
        // 2→2 pipeline — reverse should fail
        let mut p = Pipeline::new(2, 2).unwrap();
        p.insert_stage(StageLoc::AtEnd, Stage::new_identity(2).unwrap());

        let target = [0.5f32, 0.5];
        let mut result = [0.0f32; 2];
        assert!(!p.eval_reverse_float(&target, &mut result, None));
    }
}
