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
        if n == 0 {
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
        if n == 0 {
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
        if rows == 0 || cols == 0 {
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
    #[allow(dead_code)]
    pub fn new_clut_16bit(
        _grid_points: &[u32],
        _input_channels: u32,
        _output_channels: u32,
        _table: Option<&[u16]>,
    ) -> Option<Self> {
        todo!()
    }

    /// Create a 16-bit CLUT stage with uniform grid.
    ///
    /// C版: `cmsStageAllocCLut16bit`
    #[allow(dead_code)]
    pub fn new_clut_16bit_uniform(
        _grid_points: u32,
        _input_channels: u32,
        _output_channels: u32,
        _table: Option<&[u16]>,
    ) -> Option<Self> {
        todo!()
    }

    /// Create a float CLUT stage with per-dimension grid sizes.
    ///
    /// C版: `cmsStageAllocCLutFloatGranular`
    #[allow(dead_code)]
    pub fn new_clut_float(
        _grid_points: &[u32],
        _input_channels: u32,
        _output_channels: u32,
        _table: Option<&[f32]>,
    ) -> Option<Self> {
        todo!()
    }

    /// Create a float CLUT stage with uniform grid.
    ///
    /// C版: `cmsStageAllocCLutFloat`
    #[allow(dead_code)]
    pub fn new_clut_float_uniform(
        _grid_points: u32,
        _input_channels: u32,
        _output_channels: u32,
        _table: Option<&[f32]>,
    ) -> Option<Self> {
        todo!()
    }

    /// Create an identity CLUT (2-point grid, input = output).
    ///
    /// C版: `_cmsStageAllocIdentityCLut`
    #[allow(dead_code)]
    pub fn new_identity_clut(_n: u32) -> Option<Self> {
        todo!()
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn clut_too_many_inputs() {
        assert!(Stage::new_clut_16bit_uniform(2, 16, 3, None).is_none());
    }

    #[test]
    #[ignore = "not yet implemented"]
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
}
