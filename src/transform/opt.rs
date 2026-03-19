// ============================================================================
// Pipeline optimization (C版: cmsopt.c)
// ============================================================================

use crate::pipeline::lut::{Pipeline, Stage, StageData, StageLoc, sample_clut_16bit};
use crate::types::{ColorSpaceSignature, StageSignature};

use super::xform::{FLAGS_FORCE_CLUT, FLAGS_NOOPTIMIZE, FLAGS_NOWHITEONWHITEFIXUP};

// ============================================================================
// Constants
// ============================================================================

/// Number of sample points for curve joining / prelinearization.
const PRELINEARIZATION_POINTS: u32 = 4096;

/// Rendering intent: absolute colorimetric (ICC spec).
const INTENT_ABSOLUTE_COLORIMETRIC: u32 = 3;

// ============================================================================
// Pre-optimization: remove redundant stages
// ============================================================================

/// Inverse stage pairs — if stage[i] and stage[i+1] match any pair, both are removed.
const INVERSE_PAIRS: &[(StageSignature, StageSignature)] = &[
    (StageSignature::Xyz2LabElem, StageSignature::Lab2XyzElem),
    (StageSignature::Lab2XyzElem, StageSignature::Xyz2LabElem),
    (StageSignature::LabV2toV4, StageSignature::LabV4toV2),
    (StageSignature::LabV4toV2, StageSignature::LabV2toV4),
    (StageSignature::Lab2FloatPCS, StageSignature::FloatPCS2Lab),
    (StageSignature::FloatPCS2Lab, StageSignature::Lab2FloatPCS),
    (StageSignature::Xyz2FloatPCS, StageSignature::FloatPCS2Xyz),
    (StageSignature::FloatPCS2Xyz, StageSignature::Xyz2FloatPCS),
];

/// Remove all identity stages from a pipeline.
fn remove_identity_stages(pipeline: &mut Pipeline) {
    let stages = pipeline.stages_mut();
    let mut i = 0;
    while i < stages.len() {
        if stages[i].implements() == StageSignature::IdentityElem {
            // Cannot use remove_stage (only supports begin/end),
            // so we mark and rebuild. But stages_mut gives us direct access.
            // We'll collect non-identity stages and rebuild.
            break;
        }
        i += 1;
    }
    // If no identity found, nothing to do
    if i >= pipeline.stages().len() {
        return;
    }

    // Rebuild: collect indices to keep
    let keep: Vec<usize> = pipeline
        .stages()
        .iter()
        .enumerate()
        .filter(|(_, s)| s.implements() != StageSignature::IdentityElem)
        .map(|(i, _)| i)
        .collect();

    rebuild_pipeline_with_indices(pipeline, &keep);
}

/// Remove adjacent inverse pairs from a pipeline.
fn remove_inverse_pairs(pipeline: &mut Pipeline) {
    loop {
        let stages = pipeline.stages();
        let mut found = None;
        for i in 0..stages.len().saturating_sub(1) {
            let a = stages[i].implements();
            let b = stages[i + 1].implements();
            if INVERSE_PAIRS.iter().any(|&(x, y)| x == a && y == b) {
                found = Some(i);
                break;
            }
        }
        match found {
            Some(idx) => {
                let keep: Vec<usize> = (0..pipeline.stages().len())
                    .filter(|&i| i != idx && i != idx + 1)
                    .collect();
                rebuild_pipeline_with_indices(pipeline, &keep);
            }
            None => break,
        }
    }
}

/// Multiply adjacent matrix stages into a single matrix.
fn multiply_adjacent_matrices(pipeline: &mut Pipeline) {
    loop {
        let stages = pipeline.stages();
        let mut found = None;
        for i in 0..stages.len().saturating_sub(1) {
            if stages[i].stage_type() == StageSignature::MatrixElem
                && stages[i + 1].stage_type() == StageSignature::MatrixElem
            {
                found = Some(i);
                break;
            }
        }

        let Some(idx) = found else { break };

        // Extract matrix data from both stages
        let stages = pipeline.stages();
        let (m1_coeffs, m1_offset, m1_rows, m1_cols) = match stages[idx].data() {
            StageData::Matrix {
                coefficients,
                offset,
            } => (
                coefficients.clone(),
                offset.clone(),
                stages[idx].output_channels(),
                stages[idx].input_channels(),
            ),
            _ => break,
        };
        let (m2_coeffs, m2_offset, m2_rows, _m2_cols) = match stages[idx + 1].data() {
            StageData::Matrix {
                coefficients,
                offset,
            } => (
                coefficients.clone(),
                offset.clone(),
                stages[idx + 1].output_channels(),
                stages[idx + 1].input_channels(),
            ),
            _ => break,
        };

        // Result: M2 * M1 (m2_rows × m1_cols)
        let r = m2_rows as usize;
        let n = m1_rows as usize; // inner dimension
        let c = m1_cols as usize;
        let mut result = vec![0.0f64; r * c];
        for i in 0..r {
            for j in 0..c {
                let mut sum = 0.0;
                for k in 0..n {
                    sum += m2_coeffs[i * n + k] * m1_coeffs[k * c + j];
                }
                result[i * c + j] = sum;
            }
        }

        // Combine offsets: result_offset = M2 * m1_offset + m2_offset
        let mut result_offset = vec![0.0f64; r];
        if let Some(ref off1) = m1_offset {
            for i in 0..r {
                for k in 0..n {
                    result_offset[i] += m2_coeffs[i * n + k] * off1[k];
                }
            }
        }
        if let Some(ref off2) = m2_offset {
            for i in 0..r {
                result_offset[i] += off2[i];
            }
        }

        let has_offset = m1_offset.is_some() || m2_offset.is_some();
        let new_stage = Stage::new_matrix(
            m2_rows,
            m1_cols,
            &result,
            if has_offset {
                Some(&result_offset)
            } else {
                None
            },
        );

        let Some(new_stage) = new_stage else { break };

        // Replace the two stages with the combined one
        let keep: Vec<usize> = (0..pipeline.stages().len())
            .filter(|&i| i != idx && i != idx + 1)
            .collect();
        rebuild_pipeline_replacing(pipeline, &keep, idx, new_stage);
    }
}

/// Run all pre-optimization cleanup passes.
///
/// C版: `PreOptimize`
pub fn pre_optimize(pipeline: &mut Pipeline) {
    remove_identity_stages(pipeline);
    remove_inverse_pairs(pipeline);
    multiply_adjacent_matrices(pipeline);
}

// ============================================================================
// Optimization strategies
// ============================================================================

/// Collapse a pipeline of pure curve stages into a single joined curve set.
///
/// C版: `OptimizeByJoiningCurves`
pub fn optimize_by_joining_curves(pipeline: &mut Pipeline, _intent: u32, _flags: &mut u32) -> bool {
    // Precondition: all stages must be CurveSetElem
    let n_stages = pipeline.stage_count();
    if n_stages == 0 {
        return false;
    }

    for stage in pipeline.stages() {
        if stage.stage_type() != StageSignature::CurveSetElem {
            return false;
        }
    }

    let n_channels = pipeline.input_channels();
    if n_channels != pipeline.output_channels() {
        return false;
    }

    // Evaluate the composed curves at PRELINEARIZATION_POINTS sample points
    use crate::curves::gamma::ToneCurve;
    use crate::curves::intrp;

    let n_pts = PRELINEARIZATION_POINTS as usize;
    let n_ch = n_channels as usize;
    let mut tables = vec![vec![0u16; n_pts]; n_ch];

    for ch in 0..n_ch {
        for (i, entry) in tables[ch].iter_mut().enumerate() {
            let val = (i as f64 * 65535.0) / (n_pts - 1) as f64;
            let mut v = intrp::quick_saturate_word(val);
            for stage in pipeline.stages() {
                if let Some(curves) = stage.curves() {
                    v = curves[ch].eval_u16(v);
                }
            }
            *entry = v;
        }
    }

    // Build joined tone curves
    let mut joined_curves = Vec::with_capacity(n_ch);
    let mut all_linear = true;

    for table in &tables {
        let tc = match ToneCurve::build_tabulated_16(table) {
            Some(tc) => tc,
            None => return false,
        };
        if !tc.is_linear() {
            all_linear = false;
        }
        joined_curves.push(tc);
    }

    // Replace pipeline with the joined result
    let new_stage = if all_linear {
        Stage::new_identity_curves(n_channels)
    } else {
        Stage::new_tone_curves(Some(&joined_curves), n_channels)
    };

    let Some(new_stage) = new_stage else {
        return false;
    };

    // Build new pipeline
    let Some(mut new_pipeline) = Pipeline::new(n_channels, n_channels) else {
        return false;
    };
    new_pipeline.insert_stage(StageLoc::AtEnd, new_stage);
    *pipeline = new_pipeline;

    true
}

/// Check if all curves in a curve-set stage are linear (identity).
fn all_curves_are_linear(stage: &Stage) -> bool {
    match stage.curves() {
        Some(curves) => curves.iter().all(|c| c.is_linear()),
        None => true,
    }
}

/// Resample any pipeline into an optimized Curves→CLUT→Curves structure.
///
/// Keeps pre/post linearization curves from the pipeline (if present and
/// non-linear), samples the CLUT from the remaining stages, and builds
/// a Prelin16Data fast evaluator.
///
/// C版: `OptimizeByResampling`
pub fn optimize_by_resampling(
    pipeline: &mut Pipeline,
    intent: u32,
    flags: &mut u32,
    input_format: u32,
    output_format: u32,
) -> bool {
    use crate::curves::gamma::ToneCurve;
    use crate::curves::intrp::{InterpParams, LERP_FLAGS_16BITS};
    use crate::pipeline::lut::{CLutTable, FastEval16, Prelin16Data};
    use crate::types::PixelFormat;

    let n_in = pipeline.input_channels();
    let n_out = pipeline.output_channels();

    // Determine grid point count from input channels
    let grid_points = crate::math::pcs::reasonable_gridpoints(n_in, *flags);

    // Detect and extract pre/post linearization curves
    let mut src = pipeline.clone();
    let mut pre_curves: Option<Vec<ToneCurve>> = None;
    let mut post_curves: Option<Vec<ToneCurve>> = None;

    // Check first stage for pre-linearization curves
    if !src.stages().is_empty()
        && src.stages()[0].stage_type() == StageSignature::CurveSetElem
        && !all_curves_are_linear(&src.stages()[0])
    {
        pre_curves = src.stages()[0].curves().map(|c| c.to_vec());
        // Remove the first stage from sampling source
        let keep: Vec<usize> = (1..src.stages().len()).collect();
        rebuild_pipeline_with_indices(&mut src, &keep);
    }

    // Check last stage for post-linearization curves
    if !src.stages().is_empty() {
        let last_idx = src.stages().len() - 1;
        if src.stages()[last_idx].stage_type() == StageSignature::CurveSetElem
            && !all_curves_are_linear(&src.stages()[last_idx])
        {
            post_curves = src.stages()[last_idx].curves().map(|c| c.to_vec());
            let keep: Vec<usize> = (0..last_idx).collect();
            rebuild_pipeline_with_indices(&mut src, &keep);
        }
    }

    // Build the destination pipeline: [PreCurves] → CLUT → [PostCurves]
    let Some(mut dest) = Pipeline::new(n_in, n_out) else {
        return false;
    };

    if let Some(ref curves) = pre_curves
        && let Some(stage) = Stage::new_tone_curves(Some(curves), n_in)
    {
        dest.insert_stage(StageLoc::AtEnd, stage);
    }

    let mut clut_stage = match Stage::new_clut_16bit_uniform(grid_points, n_in, n_out, None) {
        Some(s) => s,
        None => return false,
    };

    // Sample the CLUT using the source pipeline (with pre/post curves removed)
    let ok = sample_clut_16bit(
        &mut clut_stage,
        |input, output, _cargo| {
            src.eval_16(input, output);
            true
        },
        0,
    );
    if !ok {
        return false;
    }

    if !dest.insert_stage(StageLoc::AtEnd, clut_stage) {
        return false;
    }

    if let Some(ref curves) = post_curves
        && let Some(stage) = Stage::new_tone_curves(Some(curves), n_out)
    {
        dest.insert_stage(StageLoc::AtEnd, stage);
    }

    // Fix white misalignment before building fast evaluator
    if intent == INTENT_ABSOLUTE_COLORIMETRIC {
        *flags |= FLAGS_NOWHITEONWHITEFIXUP;
    }
    if *flags & FLAGS_NOWHITEONWHITEFIXUP == 0 {
        let infmt = PixelFormat(input_format);
        let outfmt = PixelFormat(output_format);
        if let (Some(in_cs), Some(out_cs)) = (
            ColorSpaceSignature::from_pixel_type(infmt.colorspace()),
            ColorSpaceSignature::from_pixel_type(outfmt.colorspace()),
        ) {
            fix_white_misalignment(&mut dest, in_cs, out_cs);
        }
    }

    // Build Prelin16Data fast evaluator
    // Find the CLUT stage in dest pipeline
    let clut_idx = dest
        .stages()
        .iter()
        .position(|s| s.stage_type() == StageSignature::CLutElem);
    let Some(clut_idx) = clut_idx else {
        return false;
    };

    let (clut_params, clut_table) = match dest.stages()[clut_idx].data() {
        StageData::CLut(c) => {
            let table = match &c.table {
                CLutTable::U16(t) => t.clone(),
                CLutTable::Float(t) => t
                    .iter()
                    .map(|&v| crate::curves::intrp::quick_saturate_word(v as f64 * 65535.0))
                    .collect(),
            };
            (c.params.clone(), table)
        }
        _ => return false,
    };

    // Build curve data for Prelin16
    let curves_in: Vec<Option<(InterpParams, Vec<u16>)>> = if let Some(ref curves) = pre_curves {
        curves
            .iter()
            .map(|c| {
                let table = c.table16().to_vec();
                let n = table.len() as u32;
                InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_16BITS).map(|p| (p, table))
            })
            .collect()
    } else {
        (0..n_in).map(|_| None).collect()
    };

    let curves_out: Vec<Option<(InterpParams, Vec<u16>)>> = if let Some(ref curves) = post_curves {
        curves
            .iter()
            .map(|c| {
                let table = c.table16().to_vec();
                let n = table.len() as u32;
                InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_16BITS).map(|p| (p, table))
            })
            .collect()
    } else {
        (0..n_out).map(|_| None).collect()
    };

    let p16 = Prelin16Data {
        n_inputs: n_in,
        n_outputs: n_out,
        curves_in,
        clut_params,
        clut_table,
        curves_out,
    };

    dest.fast_eval16 = Some(FastEval16::Prelin16(Box::new(p16)));
    *pipeline = dest;
    true
}

// ============================================================================
// FixWhiteMisalignment
// ============================================================================

/// Check if two white point vectors match.
///
/// Returns true if exact match OR if values differ so extremely (> 0xF000)
/// that fixing would cause artifacts. C版: `WhitesAreEqual`
fn whites_are_equal(n: usize, a: &[u16], b: &[u16]) -> bool {
    for i in 0..n {
        let diff = (a[i] as i32 - b[i] as i32).unsigned_abs();
        if diff > 0xF000 {
            return true; // Extremely different — avoid fixing
        }
        if a[i] != b[i] {
            return false;
        }
    }
    true
}

/// Patch a single CLUT grid node.
///
/// C版: `PatchLUT`
fn patch_lut(stage: &mut Stage, at: &[u16], value: &[u16], n_out: usize, n_in: usize) -> bool {
    let clut_data = match stage.data_mut() {
        StageData::CLut(c) => c,
        _ => return false,
    };

    let params = &clut_data.params;

    // Compute grid index from input coordinates
    let mut index = 0usize;
    for (ch, &at_val) in at[..n_in].iter().enumerate() {
        let p = at_val as f64 * params.domain[ch] as f64 / 65535.0;
        let node = p.floor() as usize;
        if (p - node as f64).abs() > 1e-6 {
            return false; // Not on exact grid node
        }
        index += params.opta[n_in - 1 - ch] as usize * node;
    }

    // Write value to the CLUT table
    match &mut clut_data.table {
        crate::pipeline::lut::CLutTable::U16(table) => {
            table[index..index + n_out].copy_from_slice(&value[..n_out]);
        }
        crate::pipeline::lut::CLutTable::Float(table) => {
            for (dst, &src) in table[index..index + n_out].iter_mut().zip(&value[..n_out]) {
                *dst = src as f32 / 65535.0;
            }
        }
    }

    true
}

/// Patch CLUT white point to ensure white→white mapping.
///
/// C版: `FixWhiteMisalignment`
pub fn fix_white_misalignment(
    pipeline: &mut Pipeline,
    entry_cs: ColorSpaceSignature,
    exit_cs: ColorSpaceSignature,
) -> bool {
    let Some((white_in, _, n_ins)) = crate::math::pcs::endpoints_by_space(entry_cs) else {
        return false;
    };
    let Some((white_out, _, n_outs)) = crate::math::pcs::endpoints_by_space(exit_cs) else {
        return false;
    };

    if pipeline.input_channels() != n_ins || pipeline.output_channels() != n_outs {
        return false;
    }

    // Check current white mapping
    let mut obtained = [0u16; crate::types::MAX_CHANNELS];
    pipeline.eval_16(&white_in, &mut obtained);

    if whites_are_equal(n_outs as usize, &white_out, &obtained) {
        return true; // Already correct
    }

    // Find CLUT stage — support patterns: C, C+Curves, Curves+C, Curves+C+Curves
    let patterns: &[&[StageSignature]] = &[
        &[
            StageSignature::CurveSetElem,
            StageSignature::CLutElem,
            StageSignature::CurveSetElem,
        ],
        &[StageSignature::CurveSetElem, StageSignature::CLutElem],
        &[StageSignature::CLutElem, StageSignature::CurveSetElem],
        &[StageSignature::CLutElem],
    ];

    let mut clut_idx = None;
    let mut post_idx = None;

    for pattern in patterns {
        if let Some(indices) = pipeline.check_and_retrieve_stages(pattern) {
            for &i in &indices {
                if pipeline.stages()[i].stage_type() == StageSignature::CLutElem {
                    clut_idx = Some(i);
                }
            }
            // Post-linearization is a CurveSetElem after the CLUT
            if let Some(ci) = clut_idx {
                for &i in &indices {
                    if i > ci && pipeline.stages()[i].stage_type() == StageSignature::CurveSetElem {
                        post_idx = Some(i);
                    }
                }
            }
            break;
        }
    }

    let Some(clut_idx) = clut_idx else {
        return false;
    };

    // If there's post-linearization, find what white looks like before it
    let mut white_target = [0u16; crate::types::MAX_CHANNELS];
    if let Some(pi) = post_idx {
        let post_curves = pipeline.stages()[pi].curves().unwrap();
        for i in 0..n_outs as usize {
            let inv = post_curves[i].reverse();
            white_target[i] = inv.eval_u16(white_out[i]);
        }
    } else {
        white_target[..n_outs as usize].copy_from_slice(&white_out[..n_outs as usize]);
    }

    // Patch the white node in the CLUT (input = device white = 0xFFFF for RGB, 0 for CMYK)
    let stages = pipeline.stages_mut();
    patch_lut(
        &mut stages[clut_idx],
        &white_in,
        &white_target,
        n_outs as usize,
        n_ins as usize,
    );

    true
}

// ============================================================================
// OptimizeMatrixShaper
// ============================================================================

/// Convert f64 to 1.14 fixed-point.
fn double_to_1fixed14(x: f64) -> i32 {
    (x * 16384.0 + 0.5).floor() as i32
}

/// Fill first shaper LUT: 8-bit input → 1.14 fixed-point.
///
/// C版: `FillFirstShaper`
fn fill_first_shaper(table: &mut [i32; 256], curve: &crate::curves::gamma::ToneCurve) {
    for (i, entry) in table.iter_mut().enumerate() {
        let r = i as f32 / 255.0;
        let y = curve.eval_f32(r);
        if (y as f64) < 131072.0 {
            *entry = double_to_1fixed14(y as f64);
        } else {
            *entry = 0x7FFFFFFF;
        }
    }
}

/// Fill second shaper LUT: 1.14 range [0..16384] → u16.
///
/// C版: `FillSecondShaper`
fn fill_second_shaper(
    table: &mut [u16; 16385],
    curve: &crate::curves::gamma::ToneCurve,
    is_8bit: bool,
) {
    for (i, entry) in table.iter_mut().enumerate() {
        let r = i as f32 / 16384.0;
        let val = curve.eval_f32(r).clamp(0.0, 1.0);

        if is_8bit {
            // Quantize to 8-bit, then expand back to 16-bit
            let w = crate::curves::intrp::quick_saturate_word(val as f64 * 65535.0);
            let b = (w >> 8) as u8;
            *entry = (b as u16) << 8 | b as u16;
        } else {
            *entry = crate::curves::intrp::quick_saturate_word(val as f64 * 65535.0);
        }
    }
}

/// Optimize matrix-shaper pipelines to a 1.14 fixed-point fast path.
///
/// Detects Curves → Matrix (→ Matrix) → Curves pattern and replaces
/// the pipeline evaluation with precomputed LUT + fixed-point matrix.
///
/// C版: `OptimizeMatrixShaper`
pub fn optimize_by_matrix_shaper(
    pipeline: &mut Pipeline,
    _intent: u32,
    _flags: &mut u32,
    input_format: u32,
    output_format: u32,
) -> bool {
    use crate::pipeline::lut::{FastEval16, MatShaper8Data};
    use crate::types::PixelFormat;

    let infmt = PixelFormat(input_format);
    let outfmt = PixelFormat(output_format);

    // Only works on 3-channel input and output (RGB)
    if infmt.channels() != 3 || outfmt.channels() != 3 {
        return false;
    }

    // Only works on 8-bit input
    if infmt.bytes() != 1 {
        return false;
    }

    // Only works on 3→3 pipeline
    if pipeline.input_channels() != 3 || pipeline.output_channels() != 3 {
        return false;
    }

    // Detect pattern: CurveSet → Matrix → CurveSet
    // or CurveSet → Matrix → Matrix → CurveSet (multiply matrices first via pre_optimize)
    let stages = pipeline.stages();

    // After pre_optimize, adjacent matrices are already merged.
    // Look for: Curves → Matrix → Curves
    let pattern = &[
        StageSignature::CurveSetElem,
        StageSignature::MatrixElem,
        StageSignature::CurveSetElem,
    ];

    if pipeline.check_and_retrieve_stages(pattern).is_none() {
        return false;
    }

    // Extract curve and matrix data
    let curve1 = match stages[0].curves() {
        Some(c) if c.len() >= 3 => c,
        _ => return false,
    };
    let (coefficients, offset) = match stages[1].data() {
        StageData::Matrix {
            coefficients,
            offset,
        } => (coefficients, offset),
        _ => return false,
    };
    let curve2 = match stages[2].curves() {
        Some(c) if c.len() >= 3 => c,
        _ => return false,
    };

    // Build MatShaper8Data
    let mut data = Box::new(MatShaper8Data {
        shaper1_r: [0i32; 256],
        shaper1_g: [0i32; 256],
        shaper1_b: [0i32; 256],
        mat: [[0i32; 3]; 3],
        off: [0i32; 3],
        shaper2_r: [0u16; 16385],
        shaper2_g: [0u16; 16385],
        shaper2_b: [0u16; 16385],
    });

    // Fill first shapers
    fill_first_shaper(&mut data.shaper1_r, &curve1[0]);
    fill_first_shaper(&mut data.shaper1_g, &curve1[1]);
    fill_first_shaper(&mut data.shaper1_b, &curve1[2]);

    // Fill second shapers (always 16-bit output for pipeline level)
    fill_second_shaper(&mut data.shaper2_r, &curve2[0], false);
    fill_second_shaper(&mut data.shaper2_g, &curve2[1], false);
    fill_second_shaper(&mut data.shaper2_b, &curve2[2], false);

    // Convert matrix to 1.14 fixed-point
    for i in 0..3 {
        for j in 0..3 {
            data.mat[i][j] = double_to_1fixed14(coefficients[i * 3 + j]);
        }
    }
    if let Some(off) = offset {
        for (dst, &src) in data.off.iter_mut().zip(off) {
            *dst = double_to_1fixed14(src);
        }
    }

    // Set fast evaluator on the pipeline
    pipeline.fast_eval16 = Some(FastEval16::MatShaper(data));
    true
}

// ============================================================================
// OptimizeByComputingLinearization
// ============================================================================

/// Apply slope limiting to extremes of a tone curve table.
/// Ensures monotonicity at the curve endpoints (first/last 2%).
///
/// C版: `SlopeLimiting`
fn slope_limiting(curve: &mut crate::curves::gamma::ToneCurve) {
    let n = curve.table16_len() as usize;
    if n < 10 {
        return;
    }

    let at_begin = ((n as f64) * 0.02 + 0.5).floor() as usize;
    let at_end = n - at_begin - 1;

    let (begin_val, end_val): (f64, f64) = if curve.is_descending() {
        (0xFFFF as f64, 0.0)
    } else {
        (0.0, 0xFFFF as f64)
    };

    // Slope at beginning
    let val = curve.table16()[at_begin] as f64;
    let slope = (val - begin_val) / at_begin as f64;
    let beta = val - slope * at_begin as f64;

    let table = curve.table16_mut();
    for (i, entry) in table[..at_begin].iter_mut().enumerate() {
        *entry = crate::curves::intrp::quick_saturate_word(i as f64 * slope + beta);
    }

    // Slope at end
    let val = table[at_end] as f64;
    let slope = (end_val - val) / at_begin as f64;
    let beta = val - slope * at_end as f64;

    for (i, entry) in table[at_end..n].iter_mut().enumerate() {
        *entry = crate::curves::intrp::quick_saturate_word((i + at_end) as f64 * slope + beta);
    }
}

/// Build Prelin8Data from interpolation params and optional pre-linearization curves.
///
/// C版: `PrelinOpt8alloc`
fn prelin8_alloc(
    params: &crate::curves::intrp::InterpParams,
    curves: Option<&[crate::curves::gamma::ToneCurve]>,
    table: &[u16],
) -> crate::pipeline::lut::Prelin8Data {
    use crate::curves::intrp::to_fixed_domain;
    use crate::pipeline::lut::Prelin8Data;

    let mut p8 = Prelin8Data {
        rx: [0u16; 256],
        ry: [0u16; 256],
        rz: [0u16; 256],
        x0: [0u32; 256],
        y0: [0u32; 256],
        z0: [0u32; 256],
        n_outputs: params.n_outputs,
        opta: [params.opta[0], params.opta[1], params.opta[2]],
        table: table.to_vec(),
    };

    for i in 0..256u16 {
        let i16val = (i << 8) | i; // FROM_8_TO_16

        let (input0, input1, input2) = if let Some(c) = curves {
            (
                c[0].eval_u16(i16val),
                c[1].eval_u16(i16val),
                c[2].eval_u16(i16val),
            )
        } else {
            (i16val, i16val, i16val)
        };

        // Convert to fixed domain
        let v1 = to_fixed_domain(input0 as i32 * params.domain[0] as i32);
        let v2 = to_fixed_domain(input1 as i32 * params.domain[1] as i32);
        let v3 = to_fixed_domain(input2 as i32 * params.domain[2] as i32);

        // Integer part → node index (multiply by stride)
        p8.x0[i as usize] = params.opta[2] * ((v1 >> 16) as u32);
        p8.y0[i as usize] = params.opta[1] * ((v2 >> 16) as u32);
        p8.z0[i as usize] = params.opta[0] * ((v3 >> 16) as u32);

        // Fractional part → interpolation weight
        p8.rx[i as usize] = (v1 & 0xFFFF) as u16;
        p8.ry[i as usize] = (v2 & 0xFFFF) as u16;
        p8.rz[i as usize] = (v3 & 0xFFFF) as u16;
    }

    p8
}

/// Extract pre-linearization curves and create 8-bit fast CLUT path.
///
/// Samples the pipeline at gray ramp points to extract hidden input curves,
/// builds reverse curves, and creates an optimized Prelin8 evaluator.
///
/// C版: `OptimizeByComputingLinearization`
pub fn optimize_by_computing_linearization(
    pipeline: &mut Pipeline,
    intent: u32,
    flags: &mut u32,
    input_format: u32,
    output_format: u32,
) -> bool {
    use crate::curves::gamma::ToneCurve;
    use crate::pipeline::lut::{CLutTable, FastEval16};
    use crate::types::PixelFormat;

    let infmt = PixelFormat(input_format);

    // Only works on 3-channel, 8-bit input (RGB chunky)
    if infmt.channels() != 3 || infmt.bytes() != 1 {
        return false;
    }

    // Only works on 3-input pipelines
    if pipeline.input_channels() != 3 {
        return false;
    }

    let n_out = pipeline.output_channels() as usize;

    // Sample the pipeline at PRELINEARIZATION_POINTS gray ramp points
    // to extract the hidden per-channel transfer function.
    // Feed all channels with the same value (true gray ramp), matching C version.
    let n_pts = PRELINEARIZATION_POINTS as usize;
    let mut curve_data = vec![vec![0u16; n_pts]; 3];

    for pt in 0..n_pts {
        let val = (pt as f64 * 65535.0) / (n_pts - 1) as f64;
        let v = crate::curves::intrp::quick_saturate_word(val);

        // Feed input with a gray ramp (all channels same value)
        let mut input = [0u16; crate::types::MAX_CHANNELS];
        input[..3].fill(v);

        let mut output = [0u16; crate::types::MAX_CHANNELS];
        pipeline.eval_16(&input, &mut output);

        // Store each channel's response
        for (ch, data) in curve_data.iter_mut().enumerate() {
            data[pt] = if ch < n_out { output[ch] } else { 0 };
        }
    }

    // Build tone curves from sampled data
    let mut curves: Vec<ToneCurve> = Vec::with_capacity(3);
    let mut all_linear = true;

    for table in &curve_data {
        let Some(tc) = ToneCurve::build_tabulated_16(table) else {
            return false;
        };

        if !tc.is_monotonic() {
            return false; // Non-monotonic curves can't be linearized
        }

        if !tc.is_linear() {
            all_linear = false;
        }
        curves.push(tc);
    }

    if all_linear {
        return false; // Nothing to extract
    }

    // Apply slope limiting for numerical stability
    for curve in &mut curves {
        slope_limiting(curve);
    }

    // Build reverse curves for pre-linearization
    let rev_curves: Vec<ToneCurve> = curves.iter().map(|c| c.reverse()).collect();

    // Create new pipeline: PrelinCurves → CLUT
    let grid_points = crate::math::pcs::reasonable_gridpoints(3, *flags);

    let mut clut_stage = match Stage::new_clut_16bit_uniform(grid_points, 3, n_out as u32, None) {
        Some(s) => s,
        None => return false,
    };

    // Build a temporary pipeline with pre-linearization → original pipeline
    let Some(mut tmp_pipeline) = Pipeline::new(3, n_out as u32) else {
        return false;
    };

    // Insert reverse curves at the front
    if let Some(stage) = Stage::new_tone_curves(Some(&rev_curves), 3) {
        tmp_pipeline.insert_stage(StageLoc::AtEnd, stage);
    }

    // Clone original stages
    for stage in pipeline.stages() {
        tmp_pipeline.insert_stage(StageLoc::AtEnd, stage.clone());
    }

    // Sample the CLUT
    let original = tmp_pipeline;
    let ok = sample_clut_16bit(
        &mut clut_stage,
        |input, output, _cargo| {
            original.eval_16(input, output);
            true
        },
        0,
    );

    if !ok {
        return false;
    }

    // Build optimized pipeline with CLUT only (pre-linearization is baked into Prelin8)
    let Some(mut new_pipeline) = Pipeline::new(3, n_out as u32) else {
        return false;
    };
    if !new_pipeline.insert_stage(StageLoc::AtEnd, clut_stage) {
        return false;
    }

    // Apply white point fix BEFORE building Prelin8Data.
    // Prelin8Data copies the CLUT table, so the fix must be applied first.
    // (C version shares the table via pointer; Rust version copies it.)
    if intent == INTENT_ABSOLUTE_COLORIMETRIC {
        *flags |= FLAGS_NOWHITEONWHITEFIXUP;
    }
    if *flags & FLAGS_NOWHITEONWHITEFIXUP == 0 {
        let infmt = PixelFormat(input_format);
        let outfmt = PixelFormat(output_format);
        if let (Some(in_cs), Some(out_cs)) = (
            ColorSpaceSignature::from_pixel_type(infmt.colorspace()),
            ColorSpaceSignature::from_pixel_type(outfmt.colorspace()),
        ) {
            fix_white_misalignment(&mut new_pipeline, in_cs, out_cs);
        }
    }

    // Extract CLUT params and table for Prelin8Data (after white fix)
    let clut_stage_ref = &new_pipeline.stages()[0];
    let (params, table_data) = match clut_stage_ref.data() {
        StageData::CLut(c) => {
            let table = match &c.table {
                CLutTable::U16(t) => t.clone(),
                CLutTable::Float(t) => t
                    .iter()
                    .map(|&v| crate::curves::intrp::quick_saturate_word(v as f64 * 65535.0))
                    .collect(),
            };
            (c.params.clone(), table)
        }
        _ => return false,
    };

    // Build Prelin8Data with the extracted pre-linearization curves
    let p8 = prelin8_alloc(&params, Some(&curves), &table_data);

    // Replace pipeline with the new one, attach fast evaluator
    new_pipeline.fast_eval16 = Some(FastEval16::Prelin8(Box::new(p8)));
    *pipeline = new_pipeline;

    true
}

// ============================================================================
// Main entry point
// ============================================================================

/// Optimize a pipeline in-place.
///
/// `input_format` and `output_format` inform format-sensitive optimizations
/// (e.g., matrix-shaper requires 8-bit RGB input).
///
/// C版: `_cmsOptimizePipeline`
pub fn optimize_pipeline(
    pipeline: &mut Pipeline,
    intent: u32,
    flags: &mut u32,
    input_format: u32,
    output_format: u32,
) {
    // Skip optimization if requested
    if *flags & FLAGS_NOOPTIMIZE != 0 {
        return;
    }

    // Force CLUT path
    if *flags & FLAGS_FORCE_CLUT != 0 {
        pre_optimize(pipeline);
        optimize_by_resampling(pipeline, intent, flags, input_format, output_format);
        return;
    }

    // Normal optimization path
    pre_optimize(pipeline);

    // Try each strategy in order; stop at first success
    if optimize_by_joining_curves(pipeline, intent, flags) {
        return;
    }
    if optimize_by_matrix_shaper(pipeline, intent, flags, input_format, output_format) {
        return;
    }
    if optimize_by_computing_linearization(pipeline, intent, flags, input_format, output_format) {
        return;
    }
    optimize_by_resampling(pipeline, intent, flags, input_format, output_format);
}

// ============================================================================
// Helper: rebuild pipeline from selected stage indices
// ============================================================================

/// Rebuild a pipeline keeping only the stages at the given indices.
fn rebuild_pipeline_with_indices(pipeline: &mut Pipeline, keep: &[usize]) {
    let in_ch = if keep.is_empty() {
        pipeline.input_channels()
    } else {
        pipeline.stages()[keep[0]].input_channels()
    };
    let out_ch = if keep.is_empty() {
        pipeline.output_channels()
    } else {
        pipeline.stages()[*keep.last().unwrap()].output_channels()
    };

    let stages: Vec<Stage> = keep.iter().map(|&i| pipeline.stages()[i].clone()).collect();
    let Some(mut new_pipeline) = Pipeline::new(in_ch, out_ch) else {
        return;
    };
    for stage in stages {
        if !new_pipeline.insert_stage(StageLoc::AtEnd, stage) {
            return; // channel mismatch — leave original pipeline unchanged
        }
    }
    *pipeline = new_pipeline;
}

/// Rebuild a pipeline, replacing the removed indices with a new stage at `insert_pos`.
fn rebuild_pipeline_replacing(
    pipeline: &mut Pipeline,
    keep: &[usize],
    insert_pos: usize,
    new_stage: Stage,
) {
    let mut stages: Vec<Stage> = keep.iter().map(|&i| pipeline.stages()[i].clone()).collect();
    // Find where to insert: count how many kept indices are < insert_pos
    let insert_idx = keep.iter().filter(|&&i| i < insert_pos).count();
    stages.insert(insert_idx, new_stage);

    let in_ch = stages
        .first()
        .map_or(pipeline.input_channels(), |s| s.input_channels());
    let out_ch = stages
        .last()
        .map_or(pipeline.output_channels(), |s| s.output_channels());

    let Some(mut new_pipeline) = Pipeline::new(in_ch, out_ch) else {
        return;
    };
    for stage in stages {
        if !new_pipeline.insert_stage(StageLoc::AtEnd, stage) {
            return; // channel mismatch — leave original pipeline unchanged
        }
    }
    *pipeline = new_pipeline;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curves::gamma::ToneCurve;
    use crate::pipeline::lut::{Pipeline, Stage, StageLoc};
    use crate::types::StageSignature;

    // ================================================================
    // pre_optimize: identity removal
    // ================================================================

    #[test]

    fn pre_opt_removes_identity_stages() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let curves = Stage::new_tone_curves(None, 3).unwrap(); // gamma 1.0 = linear
        let identity = Stage::new_identity(3).unwrap();
        p.insert_stage(StageLoc::AtEnd, curves);
        p.insert_stage(StageLoc::AtEnd, identity);
        assert_eq!(p.stage_count(), 2);

        pre_optimize(&mut p);

        // Identity stage should be removed; the curve stage remains
        assert_eq!(p.stage_count(), 1);
        assert_eq!(p.stages()[0].stage_type(), StageSignature::CurveSetElem,);
    }

    // ================================================================
    // pre_optimize: inverse pair removal
    // ================================================================

    #[test]

    fn pre_opt_removes_xyz_lab_inverse_pair() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let xyz2lab = Stage::new_xyz_to_lab().unwrap();
        let lab2xyz = Stage::new_lab_to_xyz().unwrap();
        p.insert_stage(StageLoc::AtEnd, xyz2lab);
        p.insert_stage(StageLoc::AtEnd, lab2xyz);
        assert_eq!(p.stage_count(), 2);

        pre_optimize(&mut p);

        assert_eq!(p.stage_count(), 0);
    }

    #[test]

    fn pre_opt_removes_lab_v2_v4_inverse_pair() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let v2_to_v4 = Stage::new_lab_v2_to_v4().unwrap();
        let v4_to_v2 = Stage::new_lab_v4_to_v2().unwrap();
        p.insert_stage(StageLoc::AtEnd, v2_to_v4);
        p.insert_stage(StageLoc::AtEnd, v4_to_v2);
        assert_eq!(p.stage_count(), 2);

        pre_optimize(&mut p);

        // Both LabV2toV4 stages are MatrixElem internally, so after inverse pair
        // removal they should be combined/removed by multiply_adjacent_matrices.
        // The result should be 0 or 1 identity-like stage.
        assert!(p.stage_count() <= 1);
    }

    // ================================================================
    // pre_optimize: matrix multiplication
    // ================================================================

    #[test]

    fn pre_opt_multiplies_adjacent_matrices() {
        let mut p = Pipeline::new(3, 3).unwrap();

        // Matrix A: scale by 2
        let m_a = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0];
        let stage_a = Stage::new_matrix(3, 3, &m_a, None).unwrap();

        // Matrix B: scale by 0.5
        let m_b = [0.5, 0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5];
        let stage_b = Stage::new_matrix(3, 3, &m_b, None).unwrap();

        p.insert_stage(StageLoc::AtEnd, stage_a);
        p.insert_stage(StageLoc::AtEnd, stage_b);
        assert_eq!(p.stage_count(), 2);

        pre_optimize(&mut p);

        // Should be combined into a single matrix (identity)
        assert_eq!(p.stage_count(), 1);
        assert_eq!(p.stages()[0].stage_type(), StageSignature::MatrixElem,);

        // Verify the result is identity
        if let StageData::Matrix { coefficients, .. } = p.stages()[0].data() {
            let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
            for (a, b) in coefficients.iter().zip(identity.iter()) {
                assert!(
                    (a - b).abs() < 1e-10,
                    "expected identity, got {:?}",
                    coefficients
                );
            }
        } else {
            panic!("expected MatrixElem data");
        }
    }

    #[test]

    fn pre_opt_multiplies_matrices_with_offsets() {
        let mut p = Pipeline::new(3, 3).unwrap();

        // M1: identity with offset [1, 2, 3]
        let m1 = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let off1 = [1.0, 2.0, 3.0];
        let stage1 = Stage::new_matrix(3, 3, &m1, Some(&off1)).unwrap();

        // M2: scale by 2 with no offset
        let m2 = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0];
        let stage2 = Stage::new_matrix(3, 3, &m2, None).unwrap();

        p.insert_stage(StageLoc::AtEnd, stage1);
        p.insert_stage(StageLoc::AtEnd, stage2);

        pre_optimize(&mut p);

        assert_eq!(p.stage_count(), 1);
        if let StageData::Matrix {
            coefficients,
            offset,
        } = p.stages()[0].data()
        {
            // Combined matrix: scale by 2
            let expected_m = [2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0];
            for (a, b) in coefficients.iter().zip(expected_m.iter()) {
                assert!((a - b).abs() < 1e-10);
            }
            // Combined offset: M2 * off1 = [2, 4, 6]
            let off = offset.as_ref().expect("should have offset");
            assert!((off[0] - 2.0).abs() < 1e-10);
            assert!((off[1] - 4.0).abs() < 1e-10);
            assert!((off[2] - 6.0).abs() < 1e-10);
        } else {
            panic!("expected MatrixElem data");
        }
    }

    // ================================================================
    // optimize_by_joining_curves
    // ================================================================

    #[test]

    fn join_curves_two_gamma() {
        let mut p = Pipeline::new(3, 3).unwrap();

        // Two gamma 2.0 curves: composed = gamma 4.0
        let gamma2 = ToneCurve::build_gamma(2.0).unwrap();
        let curves = vec![gamma2.clone(), gamma2.clone(), gamma2.clone()];
        let stage1 = Stage::new_tone_curves(Some(&curves), 3).unwrap();
        let stage2 = Stage::new_tone_curves(Some(&curves), 3).unwrap();

        p.insert_stage(StageLoc::AtEnd, stage1);
        p.insert_stage(StageLoc::AtEnd, stage2);

        let mut flags = 0u32;
        let ok = optimize_by_joining_curves(&mut p, 0, &mut flags);
        assert!(ok);

        // Should be a single curve stage
        assert_eq!(p.stage_count(), 1);
        assert_eq!(p.stages()[0].stage_type(), StageSignature::CurveSetElem,);

        // Verify: mid-gray (0.5) through gamma 4.0 ≈ 0.0625
        let curves = p.stages()[0].curves().unwrap();
        let mid = curves[0].eval_f32(0.5);
        assert!((mid - 0.0625).abs() < 0.01, "expected ~0.0625, got {}", mid);
    }

    #[test]

    fn join_curves_rejects_mixed_stages() {
        let mut p = Pipeline::new(3, 3).unwrap();

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = vec![gamma.clone(), gamma.clone(), gamma.clone()];
        let stage_c = Stage::new_tone_curves(Some(&curves), 3).unwrap();

        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let stage_m = Stage::new_matrix(3, 3, &matrix, None).unwrap();

        p.insert_stage(StageLoc::AtEnd, stage_c);
        p.insert_stage(StageLoc::AtEnd, stage_m);

        let mut flags = 0u32;
        let ok = optimize_by_joining_curves(&mut p, 0, &mut flags);
        assert!(!ok, "should reject pipelines with non-curve stages");
        assert_eq!(p.stage_count(), 2, "pipeline should be unchanged");
    }

    // ================================================================
    // optimize_by_resampling
    // ================================================================

    #[test]

    fn resampling_creates_clut() {
        // Build a pipeline: curves → matrix → curves (typical RGB matrix-shaper)
        let mut p = Pipeline::new(3, 3).unwrap();

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = vec![gamma.clone(), gamma.clone(), gamma.clone()];
        let stage_c1 = Stage::new_tone_curves(Some(&curves), 3).unwrap();

        let matrix = [
            0.4361, 0.3851, 0.1431, 0.2225, 0.7169, 0.0606, 0.0139, 0.0971, 0.7141,
        ];
        let stage_m = Stage::new_matrix(3, 3, &matrix, None).unwrap();

        let inv_gamma = ToneCurve::build_gamma(1.0 / 2.2).unwrap();
        let inv_curves = vec![inv_gamma.clone(), inv_gamma.clone(), inv_gamma.clone()];
        let stage_c2 = Stage::new_tone_curves(Some(&inv_curves), 3).unwrap();

        p.insert_stage(StageLoc::AtEnd, stage_c1);
        p.insert_stage(StageLoc::AtEnd, stage_m);
        p.insert_stage(StageLoc::AtEnd, stage_c2);

        // Save original output for comparison
        let mut orig_out = [0u16; 3];
        p.eval_16(&[0x8000, 0x8000, 0x8000], &mut orig_out);

        let mut flags = 0u32;
        let ok = optimize_by_resampling(&mut p, 0, &mut flags, 0, 0);
        assert!(ok);

        // Should be Curves → CLUT → Curves (pre/post kept)
        assert_eq!(p.stage_count(), 3);
        assert_eq!(p.stages()[0].stage_type(), StageSignature::CurveSetElem);
        assert_eq!(p.stages()[1].stage_type(), StageSignature::CLutElem);
        assert_eq!(p.stages()[2].stage_type(), StageSignature::CurveSetElem);

        // Should have fast eval
        assert!(p.has_fast_eval16());

        // Verify similar output for mid-gray
        let mut opt_out = [0u16; 3];
        p.eval_16(&[0x8000, 0x8000, 0x8000], &mut opt_out);

        for ch in 0..3 {
            let diff = (orig_out[ch] as i32 - opt_out[ch] as i32).unsigned_abs();
            assert!(
                diff < 200,
                "channel {}: orig={}, opt={}, diff={}",
                ch,
                orig_out[ch],
                opt_out[ch],
                diff
            );
        }
    }

    // ================================================================
    // optimize_pipeline: FLAGS_NOOPTIMIZE
    // ================================================================

    #[test]

    fn optimize_pipeline_nooptimize_skips() {
        let mut p = Pipeline::new(3, 3).unwrap();
        let identity = Stage::new_identity(3).unwrap();
        p.insert_stage(StageLoc::AtEnd, identity);
        assert_eq!(p.stage_count(), 1);

        let mut flags = FLAGS_NOOPTIMIZE;
        optimize_pipeline(&mut p, 0, &mut flags, 0, 0);

        // Should not remove the identity stage
        assert_eq!(p.stage_count(), 1);
    }

    #[test]

    fn optimize_pipeline_removes_identity() {
        let mut p = Pipeline::new(3, 3).unwrap();

        // Curves + identity — identity should be removed
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = vec![gamma.clone(), gamma.clone(), gamma.clone()];
        let stage_c = Stage::new_tone_curves(Some(&curves), 3).unwrap();
        let identity = Stage::new_identity(3).unwrap();

        p.insert_stage(StageLoc::AtEnd, stage_c);
        p.insert_stage(StageLoc::AtEnd, identity);

        let mut flags = 0u32;
        optimize_pipeline(&mut p, 0, &mut flags, 0, 0);

        // After pre_optimize: identity removed (1 stage).
        // After optimize_by_joining_curves: single curve stage (1 stage).
        assert_eq!(p.stage_count(), 1);
    }

    // ================================================================
    // reasonable_gridpoints
    // ================================================================

    #[test]

    fn gridpoints_rgb_default() {
        use crate::math::pcs::reasonable_gridpoints;
        assert_eq!(reasonable_gridpoints(3, 0), 33);
    }

    #[test]

    fn gridpoints_cmyk_default() {
        use crate::math::pcs::reasonable_gridpoints;
        assert_eq!(reasonable_gridpoints(4, 0), 17);
    }

    #[test]

    fn gridpoints_highres() {
        use super::super::xform::FLAGS_HIGHRESPRECALC;
        use crate::math::pcs::reasonable_gridpoints;
        assert_eq!(reasonable_gridpoints(3, FLAGS_HIGHRESPRECALC), 49);
    }

    #[test]

    fn gridpoints_lowres() {
        use super::super::xform::FLAGS_LOWRESPRECALC;
        use crate::math::pcs::reasonable_gridpoints;
        assert_eq!(reasonable_gridpoints(3, FLAGS_LOWRESPRECALC), 17);
    }

    #[test]

    fn gridpoints_from_flags() {
        use crate::math::pcs::reasonable_gridpoints;
        // Grid points embedded in flags: bits 16..23
        let flags = 25u32 << 16;
        assert_eq!(reasonable_gridpoints(3, flags), 25);
    }

    // ================================================================
    // Integration: Transform with optimization
    // ================================================================

    #[test]

    fn transform_optimized_srgb_roundtrip() {
        use crate::profile::io::Profile;
        use crate::types::TYPE_RGB_8;

        fn roundtrip(p: &mut Profile) -> Profile {
            let data = p.save_to_mem().unwrap();
            Profile::open_mem(&data).unwrap()
        }

        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());

        // Create transform without NOOPTIMIZE — optimization should run
        let xform = super::super::xform::Transform::new(
            src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, // relative colorimetric
            0, // no special flags
        )
        .unwrap();

        // Test mid-gray roundtrip
        let input: [u8; 3] = [128, 128, 128];
        let mut output = [0u8; 3];
        xform.do_transform(&input, &mut output, 1);

        // Should be close to identity (same profile)
        for i in 0..3 {
            assert!(
                (output[i] as i16 - input[i] as i16).unsigned_abs() <= 3,
                "byte {i}: input={}, output={}",
                input[i],
                output[i]
            );
        }
    }

    // ================================================================
    // fix_white_misalignment
    // ================================================================

    #[test]
    fn fix_white_patches_clut_white_node() {
        use crate::pipeline::lut::sample_clut_16bit;
        use crate::types::ColorSpaceSignature;

        // Build a pipeline: curves → CLUT → curves
        // where the CLUT maps white to slightly-off-white
        let mut p = Pipeline::new(3, 3).unwrap();
        let grid = 9;

        let id_curves = Stage::new_identity_curves(3).unwrap();
        p.insert_stage(StageLoc::AtEnd, id_curves);

        let mut clut = Stage::new_clut_16bit_uniform(grid, 3, 3, None).unwrap();
        sample_clut_16bit(
            &mut clut,
            |input, output, _| {
                // Slightly offset: 0xFFFF maps to 0xFFF0 instead of 0xFFFF
                for ch in 0..3 {
                    output[ch] = (input[ch] as u32 * 0xFFF0 / 0xFFFF) as u16;
                }
                true
            },
            0,
        );
        p.insert_stage(StageLoc::AtEnd, clut);

        let post_curves = Stage::new_identity_curves(3).unwrap();
        p.insert_stage(StageLoc::AtEnd, post_curves);

        // Before fix: white should NOT map to 0xFFFF
        let mut out = [0u16; 3];
        p.eval_16(&[0xFFFF, 0xFFFF, 0xFFFF], &mut out);
        assert!(out[0] < 0xFFFF, "white should be off before fix");

        // Apply fix
        fix_white_misalignment(
            &mut p,
            ColorSpaceSignature::RgbData,
            ColorSpaceSignature::RgbData,
        );

        // After fix: white should map to 0xFFFF
        p.eval_16(&[0xFFFF, 0xFFFF, 0xFFFF], &mut out);
        for (ch, &val) in out[..3].iter().enumerate() {
            assert_eq!(val, 0xFFFF, "ch {ch}: white should be fixed to 0xFFFF");
        }
    }

    #[test]
    fn fix_white_noop_when_already_correct() {
        use crate::pipeline::lut::sample_clut_16bit;
        use crate::types::ColorSpaceSignature;

        // Build a pipeline where white already maps correctly
        let mut p = Pipeline::new(3, 3).unwrap();
        let grid = 9;

        let mut clut = Stage::new_clut_16bit_uniform(grid, 3, 3, None).unwrap();
        sample_clut_16bit(
            &mut clut,
            |input, output, _| {
                output[..3].copy_from_slice(&input[..3]); // identity
                true
            },
            0,
        );
        p.insert_stage(StageLoc::AtEnd, clut);

        // Verify white maps correctly
        let mut out = [0u16; 3];
        p.eval_16(&[0xFFFF, 0xFFFF, 0xFFFF], &mut out);
        assert_eq!(out[0], 0xFFFF);

        // Should be a no-op
        let result = fix_white_misalignment(
            &mut p,
            ColorSpaceSignature::RgbData,
            ColorSpaceSignature::RgbData,
        );
        assert!(result, "should return true (whites already match)");
    }

    // ================================================================
    // optimize_by_matrix_shaper
    // ================================================================

    #[test]
    fn mat_shaper_detects_curve_matrix_curve() {
        // Build a typical matrix-shaper pipeline: curves → matrix → curves
        let mut p = Pipeline::new(3, 3).unwrap();

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = vec![gamma.clone(), gamma.clone(), gamma.clone()];
        let stage_c1 = Stage::new_tone_curves(Some(&curves), 3).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_c1);

        let matrix = [
            0.4361, 0.3851, 0.1431, 0.2225, 0.7169, 0.0606, 0.0139, 0.0971, 0.7141,
        ];
        let stage_m = Stage::new_matrix(3, 3, &matrix, None).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_m);

        let inv_gamma = ToneCurve::build_gamma(1.0 / 2.2).unwrap();
        let inv_curves = vec![inv_gamma.clone(), inv_gamma.clone(), inv_gamma.clone()];
        let stage_c2 = Stage::new_tone_curves(Some(&inv_curves), 3).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_c2);

        // Save original output
        let mut orig_out = [0u16; 3];
        p.eval_16(&[0x8080, 0x8080, 0x8080], &mut orig_out);

        let mut flags = 0u32;
        let ok = optimize_by_matrix_shaper(
            &mut p,
            0,
            &mut flags,
            crate::types::TYPE_RGB_8.0,
            crate::types::TYPE_RGB_8.0,
        );
        assert!(ok, "should detect and optimize matrix-shaper");

        // Pipeline should have fast_eval16 set
        assert!(p.has_fast_eval16(), "should have fast eval path");

        // Output should be close to original
        let mut opt_out = [0u16; 3];
        p.eval_16(&[0x8080, 0x8080, 0x8080], &mut opt_out);

        for ch in 0..3 {
            let diff = (orig_out[ch] as i32 - opt_out[ch] as i32).unsigned_abs();
            assert!(
                diff < 300,
                "ch {ch}: orig={}, opt={}, diff={}",
                orig_out[ch],
                opt_out[ch],
                diff
            );
        }
    }

    #[test]
    fn mat_shaper_preserves_white() {
        let mut p = Pipeline::new(3, 3).unwrap();

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = vec![gamma.clone(), gamma.clone(), gamma.clone()];
        let stage_c1 = Stage::new_tone_curves(Some(&curves), 3).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_c1);

        // Identity matrix — white should stay white
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let stage_m = Stage::new_matrix(3, 3, &matrix, None).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_m);

        let inv_gamma = ToneCurve::build_gamma(1.0 / 2.2).unwrap();
        let inv_curves = vec![inv_gamma.clone(), inv_gamma.clone(), inv_gamma.clone()];
        let stage_c2 = Stage::new_tone_curves(Some(&inv_curves), 3).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_c2);

        let mut flags = 0u32;
        optimize_by_matrix_shaper(
            &mut p,
            0,
            &mut flags,
            crate::types::TYPE_RGB_8.0,
            crate::types::TYPE_RGB_8.0,
        );

        // White (0xFF expanded to 0xFF00+0xFF = 0xFFFF by 8→16) should map to white
        let mut out = [0u16; 3];
        p.eval_16(&[0xFFFF, 0xFFFF, 0xFFFF], &mut out);
        for (ch, &val) in out[..3].iter().enumerate() {
            // Allow some tolerance for fixed-point quantization
            assert!(
                val > 0xFF00,
                "ch {ch}: white should be near 0xFFFF, got {val:#06X}",
            );
        }
    }

    #[test]
    fn mat_shaper_rejects_non_rgb() {
        // CMYK pipeline should not be optimized by matrix shaper
        let mut p = Pipeline::new(4, 4).unwrap();
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = vec![gamma.clone(), gamma.clone(), gamma.clone(), gamma.clone()];
        let stage_c = Stage::new_tone_curves(Some(&curves), 4).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_c);

        let mut flags = 0u32;
        let ok = optimize_by_matrix_shaper(
            &mut p,
            0,
            &mut flags,
            crate::types::TYPE_RGB_8.0,
            crate::types::TYPE_RGB_8.0,
        );
        assert!(!ok, "should reject non-RGB pipelines");
    }

    // ================================================================
    // optimize_by_computing_linearization
    // ================================================================

    #[test]
    fn linearization_extracts_precurves() {
        use crate::pipeline::lut::sample_clut_16bit;

        // Build: curves → CLUT → curves
        let mut p = Pipeline::new(3, 3).unwrap();

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = vec![gamma.clone(), gamma.clone(), gamma.clone()];
        let stage_c1 = Stage::new_tone_curves(Some(&curves), 3).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_c1);

        let mut clut = Stage::new_clut_16bit_uniform(17, 3, 3, None).unwrap();
        sample_clut_16bit(
            &mut clut,
            |input, output, _| {
                output[..3].copy_from_slice(&input[..3]); // identity CLUT
                true
            },
            0,
        );
        p.insert_stage(StageLoc::AtEnd, clut);

        let inv_gamma = ToneCurve::build_gamma(1.0 / 2.2).unwrap();
        let inv_curves = vec![inv_gamma.clone(), inv_gamma.clone(), inv_gamma.clone()];
        let stage_c2 = Stage::new_tone_curves(Some(&inv_curves), 3).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_c2);

        // Save original output
        let mut orig_out = [0u16; 3];
        p.eval_16(&[0x8080, 0x8080, 0x8080], &mut orig_out);

        let mut flags = 0u32;
        let ok = optimize_by_computing_linearization(
            &mut p,
            0,
            &mut flags,
            crate::types::TYPE_RGB_8.0,
            crate::types::TYPE_RGB_8.0,
        );
        assert!(ok, "should extract linearization from pipeline");

        // Should now have fast eval
        assert!(p.has_fast_eval16(), "should have fast eval path");

        // Output should be close to original
        let mut opt_out = [0u16; 3];
        p.eval_16(&[0x8080, 0x8080, 0x8080], &mut opt_out);

        for ch in 0..3 {
            let diff = (orig_out[ch] as i32 - opt_out[ch] as i32).unsigned_abs();
            assert!(
                diff < 500,
                "ch {ch}: orig={}, opt={}, diff={}",
                orig_out[ch],
                opt_out[ch],
                diff
            );
        }
    }

    #[test]
    fn linearization_rejects_non_3ch() {
        // 1-channel pipeline should not be linearized
        let mut p = Pipeline::new(1, 1).unwrap();
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let stage = Stage::new_tone_curves(Some(&[gamma]), 1).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage);

        let mut flags = 0u32;
        let ok = optimize_by_computing_linearization(
            &mut p,
            0,
            &mut flags,
            crate::types::TYPE_RGB_8.0,
            crate::types::TYPE_RGB_8.0,
        );
        assert!(!ok, "should reject non-3ch pipelines");
    }

    // ================================================================
    // Integration: full optimization pipeline with new strategies
    // ================================================================

    #[test]
    fn optimize_pipeline_uses_matrix_shaper() {
        use crate::profile::io::Profile;
        use crate::types::TYPE_RGB_8;

        fn roundtrip(p: &mut Profile) -> Profile {
            let data = p.save_to_mem().unwrap();
            Profile::open_mem(&data).unwrap()
        }

        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());

        // sRGB→sRGB should use matrix-shaper optimization
        let xform =
            super::super::xform::Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

        // Verify the pipeline has fast eval set
        assert!(
            xform.pipeline().has_fast_eval16(),
            "sRGB→sRGB should use matrix-shaper fast path"
        );

        // Verify accuracy: multiple test colors
        let test_colors: &[[u8; 3]] = &[
            [0, 0, 0],       // black
            [255, 255, 255], // white
            [128, 128, 128], // mid gray
            [255, 0, 0],     // red
            [0, 255, 0],     // green
            [0, 0, 255],     // blue
        ];

        for color in test_colors {
            let mut output = [0u8; 3];
            xform.do_transform(color, &mut output, 1);
            for ch in 0..3 {
                assert!(
                    (output[ch] as i16 - color[ch] as i16).unsigned_abs() <= 3,
                    "color {:?} ch {ch}: in={}, out={}",
                    color,
                    color[ch],
                    output[ch]
                );
            }
        }
    }

    // ================================================================
    // Prelin16: 16-bit CLUT fast path
    // ================================================================

    #[test]
    fn resampling_sets_prelin16_fast_eval() {
        // Build a pipeline: curves → CLUT → curves
        let mut p = Pipeline::new(3, 3).unwrap();

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = vec![gamma.clone(), gamma.clone(), gamma.clone()];
        let stage_c1 = Stage::new_tone_curves(Some(&curves), 3).unwrap();

        // Small CLUT for testing
        let stage_clut = Stage::new_clut_16bit_uniform(9, 3, 3, None).unwrap();

        let inv_gamma = ToneCurve::build_gamma(1.0 / 2.2).unwrap();
        let inv_curves = vec![inv_gamma.clone(), inv_gamma.clone(), inv_gamma.clone()];
        let stage_c2 = Stage::new_tone_curves(Some(&inv_curves), 3).unwrap();

        p.insert_stage(StageLoc::AtEnd, stage_c1);
        p.insert_stage(StageLoc::AtEnd, stage_clut);
        p.insert_stage(StageLoc::AtEnd, stage_c2);

        // Save original output
        let mut orig_out = [0u16; 3];
        p.eval_16(&[0x8000, 0x8000, 0x8000], &mut orig_out);

        let mut flags = 0u32;
        let ok = optimize_by_resampling(&mut p, 0, &mut flags, 0, 0);
        assert!(ok);

        // Should have fast eval path
        assert!(
            p.has_fast_eval16(),
            "resampling with curves should set Prelin16 fast eval"
        );

        // Output should be close to original
        let mut opt_out = [0u16; 3];
        p.eval_16(&[0x8000, 0x8000, 0x8000], &mut opt_out);

        for ch in 0..3 {
            let diff = (orig_out[ch] as i32 - opt_out[ch] as i32).unsigned_abs();
            assert!(
                diff < 500,
                "ch {ch}: orig={}, opt={}, diff={}",
                orig_out[ch],
                opt_out[ch],
                diff
            );
        }
    }

    #[test]
    fn resampling_no_curves_uses_direct_clut() {
        // Pipeline with only non-curve stages (no pre/post linearization)
        // Should still optimize but without Prelin16 curves wrapping
        let mut p = Pipeline::new(3, 3).unwrap();

        let matrix = [
            0.4361, 0.3851, 0.1431, 0.2225, 0.7169, 0.0606, 0.0139, 0.0971, 0.7141,
        ];
        let stage_m = Stage::new_matrix(3, 3, &matrix, None).unwrap();
        p.insert_stage(StageLoc::AtEnd, stage_m);

        let mut orig_out = [0u16; 3];
        p.eval_16(&[0x8000, 0x8000, 0x8000], &mut orig_out);

        let mut flags = 0u32;
        let ok = optimize_by_resampling(&mut p, 0, &mut flags, 0, 0);
        assert!(ok);

        // Should have fast eval (direct CLUT or Prelin16 with identity curves)
        assert!(
            p.has_fast_eval16(),
            "resampled pipeline should have fast eval"
        );

        let mut opt_out = [0u16; 3];
        p.eval_16(&[0x8000, 0x8000, 0x8000], &mut opt_out);

        for ch in 0..3 {
            let diff = (orig_out[ch] as i32 - opt_out[ch] as i32).unsigned_abs();
            assert!(
                diff < 200,
                "ch {ch}: orig={}, opt={}, diff={}",
                orig_out[ch],
                opt_out[ch],
                diff
            );
        }
    }
}
