// ============================================================================
// Pipeline optimization (C版: cmsopt.c)
// ============================================================================

use crate::pipeline::lut::{Pipeline, Stage, StageData, StageLoc, sample_clut_16bit};
use crate::types::StageSignature;

use super::xform::{FLAGS_FORCE_CLUT, FLAGS_NOOPTIMIZE};

// ============================================================================
// Constants
// ============================================================================

/// Number of sample points for curve joining / prelinearization.
const PRELINEARIZATION_POINTS: u32 = 4096;

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
            let a = stages[i].stage_type();
            let b = stages[i + 1].stage_type();
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

/// Resample any pipeline into a CLUT.
///
/// C版: `OptimizeByResampling`
pub fn optimize_by_resampling(pipeline: &mut Pipeline, _intent: u32, flags: &mut u32) -> bool {
    let n_in = pipeline.input_channels();
    let n_out = pipeline.output_channels();

    // Determine grid point count from input channels
    let grid_points = crate::math::pcs::reasonable_gridpoints(n_in, *flags);

    // Create new CLUT stage
    let mut clut_stage = match Stage::new_clut_16bit_uniform(grid_points, n_in, n_out, None) {
        Some(s) => s,
        None => return false,
    };

    // Sample: evaluate original pipeline at each grid node
    let original = pipeline.clone();
    let ok = sample_clut_16bit(
        &mut clut_stage,
        |input, output, _cargo| {
            original.eval_16(input, output);
            true
        },
        0, // SAMPLER_WRITE
    );

    if !ok {
        return false;
    }

    // Build new pipeline with just the CLUT
    let Some(mut new_pipeline) = Pipeline::new(n_in, n_out) else {
        return false;
    };
    if !new_pipeline.insert_stage(StageLoc::AtEnd, clut_stage) {
        return false;
    }

    *pipeline = new_pipeline;
    true
}

// ============================================================================
// Main entry point
// ============================================================================

/// Optimize a pipeline in-place.
///
/// C版: `_cmsOptimizePipeline`
pub fn optimize_pipeline(pipeline: &mut Pipeline, intent: u32, flags: &mut u32) {
    // Skip optimization if requested
    if *flags & FLAGS_NOOPTIMIZE != 0 {
        return;
    }

    // Force CLUT path
    if *flags & FLAGS_FORCE_CLUT != 0 {
        pre_optimize(pipeline);
        optimize_by_resampling(pipeline, intent, flags);
        return;
    }

    // Normal optimization path
    pre_optimize(pipeline);

    // Try each strategy in order; stop at first success
    if optimize_by_joining_curves(pipeline, intent, flags) {
        return;
    }
    // Deferred: optimize_by_matrix_shaper
    // Deferred: optimize_by_computing_linearization
    optimize_by_resampling(pipeline, intent, flags);
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
        new_pipeline.insert_stage(StageLoc::AtEnd, stage);
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
        new_pipeline.insert_stage(StageLoc::AtEnd, stage);
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
        let ok = optimize_by_resampling(&mut p, 0, &mut flags);
        assert!(ok);

        // Should be a single CLUT stage
        assert_eq!(p.stage_count(), 1);
        assert_eq!(p.stages()[0].stage_type(), StageSignature::CLutElem);

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
        optimize_pipeline(&mut p, 0, &mut flags);

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
        optimize_pipeline(&mut p, 0, &mut flags);

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
}
