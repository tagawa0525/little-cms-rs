// Multi-profile pipeline linking.
// C版: cmscnvrt.c

#![allow(dead_code)]

use crate::context::{CmsError, ErrorCode};
use crate::math::mtrx::{Mat3, Vec3};
use crate::pipeline::lut::{Pipeline, Stage, StageLoc};
use crate::profile::io::Profile;
use crate::types::{ColorSpaceSignature, ProfileClassSignature};

const EMPTY_LAYER_TOLERANCE: f64 = 0.002;

/// Maximum encodeable XYZ value in 1.15 fixed point.
const MAX_ENCODEABLE_XYZ: f64 = 1.0 + 32767.0 / 32768.0;

/// Check if two color spaces are compatible for PCS connection.
/// XYZ and Lab are interchangeable; CMYK and 4-color are interchangeable.
/// C版: `ColorSpaceIsCompatible`
pub fn color_space_is_compatible(a: ColorSpaceSignature, b: ColorSpaceSignature) -> bool {
    if a == b {
        return true;
    }
    // CMYK ↔ 4-color
    if (a == ColorSpaceSignature::Mch4Data && b == ColorSpaceSignature::CmykData)
        || (a == ColorSpaceSignature::CmykData && b == ColorSpaceSignature::Mch4Data)
    {
        return true;
    }
    // XYZ ↔ Lab
    if (a == ColorSpaceSignature::XyzData && b == ColorSpaceSignature::LabData)
        || (a == ColorSpaceSignature::LabData && b == ColorSpaceSignature::XyzData)
    {
        return true;
    }
    false
}

/// Check if a matrix+offset layer is effectively identity (no-op).
/// C版: `IsEmptyLayer`
pub fn is_empty_layer(m: &Mat3, off: &Vec3) -> bool {
    let id = Mat3::identity();
    let mut diff = 0.0;
    for i in 0..3 {
        for j in 0..3 {
            diff += (m.0[i].0[j] - id.0[i].0[j]).abs();
        }
    }
    for i in 0..3 {
        diff += off.0[i].abs();
    }
    diff < EMPTY_LAYER_TOLERANCE
}

/// Insert a matrix+offset stage into the pipeline.
fn insert_matrix_stage(result: &mut Pipeline, m: &Mat3, off: &Vec3) -> Result<(), CmsError> {
    let flat = [
        m.0[0].0[0],
        m.0[0].0[1],
        m.0[0].0[2],
        m.0[1].0[0],
        m.0[1].0[1],
        m.0[1].0[2],
        m.0[2].0[0],
        m.0[2].0[1],
        m.0[2].0[2],
    ];
    let off_arr = [off.0[0], off.0[1], off.0[2]];
    let stage = Stage::new_matrix(3, 3, &flat, Some(&off_arr)).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create conversion matrix stage".to_string(),
    })?;
    if !result.insert_stage(StageLoc::AtEnd, stage) {
        return Err(CmsError {
            code: ErrorCode::Internal,
            message: "Failed to insert conversion matrix stage".to_string(),
        });
    }
    Ok(())
}

/// Helper to insert a stage with error checking.
fn insert_stage_checked(pipe: &mut Pipeline, stage: Stage, desc: &str) -> Result<(), CmsError> {
    if !pipe.insert_stage(StageLoc::AtEnd, stage) {
        return Err(CmsError {
            code: ErrorCode::Internal,
            message: format!("Failed to insert {desc} stage"),
        });
    }
    Ok(())
}

/// Add a PCS conversion stage (Lab↔XYZ + optional matrix) to a pipeline.
/// C版: `AddConversion`
pub fn add_conversion(
    result: &mut Pipeline,
    in_pcs: ColorSpaceSignature,
    out_pcs: ColorSpaceSignature,
    m: &Mat3,
    off: &Vec3,
) -> Result<(), CmsError> {
    let empty = is_empty_layer(m, off);

    match in_pcs {
        ColorSpaceSignature::XyzData => match out_pcs {
            ColorSpaceSignature::XyzData => {
                if !empty {
                    insert_matrix_stage(result, m, off)?;
                }
            }
            ColorSpaceSignature::LabData => {
                if !empty {
                    insert_matrix_stage(result, m, off)?;
                }
                let stage = Stage::new_xyz_to_lab().ok_or_else(|| CmsError {
                    code: ErrorCode::Internal,
                    message: "Failed to create XYZ→Lab stage".to_string(),
                })?;
                insert_stage_checked(result, stage, "XYZ→Lab")?;
            }
            _ => {
                return Err(CmsError {
                    code: ErrorCode::ColorspaceCheck,
                    message: "PCS mismatch in conversion".to_string(),
                });
            }
        },
        ColorSpaceSignature::LabData => match out_pcs {
            ColorSpaceSignature::XyzData => {
                let stage = Stage::new_lab_to_xyz().ok_or_else(|| CmsError {
                    code: ErrorCode::Internal,
                    message: "Failed to create Lab→XYZ stage".to_string(),
                })?;
                insert_stage_checked(result, stage, "Lab→XYZ")?;
                if !empty {
                    insert_matrix_stage(result, m, off)?;
                }
            }
            ColorSpaceSignature::LabData => {
                if !empty {
                    let lab2xyz = Stage::new_lab_to_xyz().ok_or_else(|| CmsError {
                        code: ErrorCode::Internal,
                        message: "Failed to create Lab→XYZ stage".to_string(),
                    })?;
                    insert_stage_checked(result, lab2xyz, "Lab→XYZ")?;
                    insert_matrix_stage(result, m, off)?;
                    let xyz2lab = Stage::new_xyz_to_lab().ok_or_else(|| CmsError {
                        code: ErrorCode::Internal,
                        message: "Failed to create XYZ→Lab stage".to_string(),
                    })?;
                    insert_stage_checked(result, xyz2lab, "XYZ→Lab")?;
                }
            }
            _ => {
                return Err(CmsError {
                    code: ErrorCode::ColorspaceCheck,
                    message: "PCS mismatch in conversion".to_string(),
                });
            }
        },
        _ => {
            if in_pcs != out_pcs {
                return Err(CmsError {
                    code: ErrorCode::ColorspaceCheck,
                    message: "Color space mismatch in non-PCS conversion".to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Compute the absolute colorimetric conversion matrix.
/// C版: `ComputeAbsoluteIntent`
fn compute_absolute_intent(
    _adaptation_state: f64,
    wp_in: &crate::types::CieXyz,
    wp_out: &crate::types::CieXyz,
) -> Mat3 {
    // Fully adapted observer (standard V4 behaviour)
    Mat3([
        Vec3([wp_in.x / wp_out.x, 0.0, 0.0]),
        Vec3([0.0, wp_in.y / wp_out.y, 0.0]),
        Vec3([0.0, 0.0, wp_in.z / wp_out.z]),
    ])
}

/// Compute BPC (Black Point Compensation) matrix and offset.
/// C版: `ComputeBlackPointCompensation`
fn compute_bpc(bp_in: &crate::types::CieXyz, bp_out: &crate::types::CieXyz) -> (Mat3, Vec3) {
    use crate::types::{D50_X, D50_Y, D50_Z};

    const NEAR_ZERO: f64 = 1e-10;

    let tx = bp_in.x - D50_X;
    let ty = bp_in.y - D50_Y;
    let tz = bp_in.z - D50_Z;

    // Guard against zero denominators: treat that axis as identity (scale=1, offset=0)
    let (ax, bx) = if tx.abs() < NEAR_ZERO {
        (1.0, 0.0)
    } else {
        ((bp_out.x - D50_X) / tx, -D50_X * (bp_out.x - bp_in.x) / tx)
    };
    let (ay, by) = if ty.abs() < NEAR_ZERO {
        (1.0, 0.0)
    } else {
        ((bp_out.y - D50_Y) / ty, -D50_Y * (bp_out.y - bp_in.y) / ty)
    };
    let (az, bz) = if tz.abs() < NEAR_ZERO {
        (1.0, 0.0)
    } else {
        ((bp_out.z - D50_Z) / tz, -D50_Z * (bp_out.z - bp_in.z) / tz)
    };

    let m = Mat3([
        Vec3([ax, 0.0, 0.0]),
        Vec3([0.0, ay, 0.0]),
        Vec3([0.0, 0.0, az]),
    ]);
    let off = Vec3([bx, by, bz]);
    (m, off)
}

/// Compute the conversion layer between two profiles.
/// C版: `ComputeConversion`
fn compute_conversion(
    profiles: &mut [Profile],
    i: usize,
    intent: u32,
    bpc: bool,
    adaptation_state: f64,
) -> Result<(Mat3, Vec3), CmsError> {
    let mut m = Mat3::identity();
    let mut off = Vec3::new(0.0, 0.0, 0.0);

    if intent == 3 {
        // Absolute colorimetric
        let wp_in = profiles[i - 1].read_media_white_point()?;
        let wp_out = profiles[i].read_media_white_point()?;

        m = compute_absolute_intent(adaptation_state, &wp_in, &wp_out);
    }

    // Black Point Compensation: only apply when both sides are detected
    if bpc
        && let (Some(bp_in), Some(bp_out)) = (
            super::samp::detect_black_point(&mut profiles[i - 1], intent),
            super::samp::detect_dest_black_point(&mut profiles[i], intent),
        )
    {
        let (bpc_m, bpc_off) = compute_bpc(&bp_in, &bp_out);
        // Combine: M_result = M_bpc * M_existing, off_result = M_bpc * off + off_bpc
        m = bpc_m * m;
        off = Vec3([
            bpc_m.0[0].0[0] * off.0[0] + bpc_off.0[0],
            bpc_m.0[1].0[1] * off.0[1] + bpc_off.0[1],
            bpc_m.0[2].0[2] * off.0[2] + bpc_off.0[2],
        ]);
    }

    // Offset adjustment for 1.15 fixed point encoding
    for k in 0..3 {
        off.0[k] /= MAX_ENCODEABLE_XYZ;
    }

    Ok((m, off))
}

/// Build a multi-profile pipeline for ICC standard intents.
/// C版: `DefaultICCintents`
pub fn default_icc_intents(
    profiles: &mut [Profile],
    intents: &[u32],
    bpc: &[bool],
    adaptation_states: &[f64],
) -> Result<Pipeline, CmsError> {
    let n = profiles.len();
    if n == 0 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "No profiles provided".to_string(),
        });
    }
    if intents.len() != n || bpc.len() != n || adaptation_states.len() != n {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "Slice lengths must match number of profiles".to_string(),
        });
    }

    let mut result = Pipeline::new(0, 0).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create result pipeline".to_string(),
    })?;

    let mut current_color_space = profiles[0].header.color_space;

    for i in 0..n {
        let class_sig = profiles[i].header.device_class;
        let is_device_link = class_sig == ProfileClassSignature::Link
            || class_sig == ProfileClassSignature::Abstract;

        // First profile is input unless devicelink/abstract
        let is_input = if i == 0 && !is_device_link {
            true
        } else {
            current_color_space != ColorSpaceSignature::XyzData
                && current_color_space != ColorSpaceSignature::LabData
        };

        let intent = intents[i];

        let (color_space_in, color_space_out) = if is_input || is_device_link {
            (profiles[i].header.color_space, profiles[i].header.pcs)
        } else {
            (profiles[i].header.pcs, profiles[i].header.color_space)
        };

        if !color_space_is_compatible(color_space_in, current_color_space) {
            return Err(CmsError {
                code: ErrorCode::ColorspaceCheck,
                message: "ColorSpace mismatch between profiles".to_string(),
            });
        }

        let lut = if is_device_link {
            // Abstract profiles after the first need PCS conversion.
            // C版: cmscnvrt.c DefaultICCintents special case for Abstract at i > 0
            if class_sig == ProfileClassSignature::Abstract && i > 0 {
                let (m, off) =
                    compute_conversion(profiles, i, intent, bpc[i], adaptation_states[i])?;
                add_conversion(&mut result, current_color_space, color_space_in, &m, &off)?;
            }
            profiles[i].read_input_lut(intent)?
        } else if is_input {
            profiles[i].read_input_lut(intent)?
        } else {
            // Output direction: compute conversion first
            let (m, off) = compute_conversion(profiles, i, intent, bpc[i], adaptation_states[i])?;
            add_conversion(&mut result, current_color_space, color_space_in, &m, &off)?;

            profiles[i].read_output_lut(intent)?
        };

        if !result.cat(&lut) {
            return Err(CmsError {
                code: ErrorCode::ColorspaceCheck,
                message: "Failed to concatenate profile pipeline".to_string(),
            });
        }
        current_color_space = color_space_out;
    }

    Ok(result)
}

/// Link multiple profiles into a single pipeline.
/// Entry point for transform pipeline construction.
/// C版: `_cmsLinkProfiles`
pub fn link_profiles(
    profiles: &mut [Profile],
    intents: &[u32],
    bpc: &mut [bool],
    adaptation_states: &[f64],
) -> Result<Pipeline, CmsError> {
    let n = profiles.len();
    if n == 0 || n > 255 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Invalid number of profiles: {n}"),
        });
    }
    if intents.len() != n || bpc.len() != n || adaptation_states.len() != n {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "Slice lengths must match number of profiles".to_string(),
        });
    }

    // BPC adjustment per Adobe spec:
    // - Absolute colorimetric: BPC = false
    // - V4 + perceptual/saturation: BPC = true
    for i in 0..n {
        if intents[i] == 3 {
            // Absolute colorimetric
            bpc[i] = false;
        }
        if intents[i] == 0 || intents[i] == 2 {
            // Perceptual or saturation: force BPC for V4
            if profiles[i].header.version >= 0x4000000 {
                bpc[i] = true;
            }
        }
    }

    // Dispatch to intent handler based on first intent
    match intents[0] {
        10..=12 => black_preserving_k_only_intents(profiles, intents, bpc, adaptation_states),
        13..=15 => black_preserving_k_plane_intents(profiles, intents, bpc, adaptation_states),
        _ => default_icc_intents(profiles, intents, bpc, adaptation_states),
    }
}

/// Translate black-preserving intents to ICC ones.
/// C版: `TranslateNonICCIntents`
fn translate_non_icc_intents(intent: u32) -> u32 {
    match intent {
        10 | 13 => 0, // K-only/K-plane Perceptual
        11 | 14 => 1, // K-only/K-plane Relative Colorimetric
        12 | 15 => 2, // K-only/K-plane Saturation
        _ => intent,
    }
}

/// Build a pipeline for black-preserving K-only intents.
/// C版: `BlackPreservingKOnlyIntents`
pub fn black_preserving_k_only_intents(
    profiles: &mut [Profile],
    intents: &[u32],
    bpc: &[bool],
    adaptation_states: &[f64],
) -> Result<Pipeline, CmsError> {
    let n = profiles.len();
    if n == 0 || n > 255 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Invalid number of profiles: {n}"),
        });
    }

    // Translate to ICC intents
    let icc_intents: Vec<u32> = intents
        .iter()
        .map(|&i| translate_non_icc_intents(i))
        .collect();

    // Skip trailing CMYK devicelinks
    let mut last = n - 1;
    while last >= 2
        && profiles[last].header.device_class == ProfileClassSignature::Link
        && profiles[last].header.color_space == ColorSpaceSignature::CmykData
    {
        last -= 1;
    }
    let preservation_count = last + 1;

    // If not CMYK→CMYK, fall back to default ICC intents
    if profiles[0].header.color_space != ColorSpaceSignature::CmykData
        || (profiles[last].header.color_space != ColorSpaceSignature::CmykData
            && profiles[last].header.device_class != ProfileClassSignature::Output)
    {
        return default_icc_intents(profiles, &icc_intents, bpc, adaptation_states);
    }

    // Build standard ICC pipeline
    let cmyk2cmyk = default_icc_intents(
        &mut profiles[..preservation_count],
        &icc_intents[..preservation_count],
        &bpc[..preservation_count],
        &adaptation_states[..preservation_count],
    )?;

    // Build K tone curve
    let k_tone = super::gmt::build_k_tone_curve(
        &mut profiles[..preservation_count],
        &icc_intents[..preservation_count],
        &bpc[..preservation_count],
        &adaptation_states[..preservation_count],
        4096,
        0,
    )?;

    // Create CLUT
    let n_grid = crate::math::pcs::reasonable_gridpoints(4, 0);
    let grid_points = [n_grid; crate::curves::intrp::MAX_INPUT_DIMENSIONS];
    let mut clut = Stage::new_clut_16bit(&grid_points, 4, 4, None).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to allocate CLUT for K-only preservation".into(),
    })?;

    // Sample the CLUT
    crate::pipeline::lut::sample_clut_16bit(
        &mut clut,
        |input: &[u16], output: &mut [u16], _cargo: &()| {
            // If C=M=Y=0, preserve K only
            if input[0] == 0 && input[1] == 0 && input[2] == 0 {
                output[0] = 0;
                output[1] = 0;
                output[2] = 0;
                output[3] = k_tone.eval_u16(input[3]);
                return true;
            }

            // Otherwise use standard ICC pipeline
            cmyk2cmyk.eval_16(input, output);
            true
        },
        0,
    );

    let mut result = Pipeline::new(4, 4).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to allocate result pipeline".into(),
    })?;
    result.insert_stage(StageLoc::AtBegin, clut);

    // Append trailing devicelinks
    for i in (last + 1)..n {
        let devlink = profiles[i].read_input_lut(icc_intents[i])?;
        if !result.cat(&devlink) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to concatenate devicelink".into(),
            });
        }
    }

    Ok(result)
}

/// Build a pipeline for black-preserving K-plane intents.
/// C版: `BlackPreservingKPlaneIntents`
pub fn black_preserving_k_plane_intents(
    profiles: &mut [Profile],
    intents: &[u32],
    bpc: &[bool],
    adaptation_states: &[f64],
) -> Result<Pipeline, CmsError> {
    let n = profiles.len();
    if n == 0 || n > 255 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Invalid number of profiles: {n}"),
        });
    }

    // Translate to ICC intents
    let icc_intents: Vec<u32> = intents
        .iter()
        .map(|&i| translate_non_icc_intents(i))
        .collect();

    // Skip trailing CMYK devicelinks
    let mut last = n - 1;
    while last >= 2
        && profiles[last].header.device_class == ProfileClassSignature::Link
        && profiles[last].header.color_space == ColorSpaceSignature::CmykData
    {
        last -= 1;
    }
    let preservation_count = last + 1;

    // If not CMYK→CMYK, fall back to default ICC intents
    if profiles[0].header.color_space != ColorSpaceSignature::CmykData
        || (profiles[last].header.color_space != ColorSpaceSignature::CmykData
            && profiles[last].header.device_class != ProfileClassSignature::Output)
    {
        return default_icc_intents(profiles, &icc_intents, bpc, adaptation_states);
    }

    // Read the input LUT of the last profile (for reverse interpolation)
    let lab_k2cmyk = profiles[last].read_input_lut(1)?; // Relative colorimetric

    // Get TAC
    let max_tac = super::gmt::detect_tac(&mut profiles[last]) / 100.0;
    if max_tac <= 0.0 {
        // No TAC detected, fall back to default
        return default_icc_intents(profiles, &icc_intents, bpc, adaptation_states);
    }

    // Build standard ICC pipeline
    let cmyk2cmyk = default_icc_intents(
        &mut profiles[..preservation_count],
        &icc_intents[..preservation_count],
        &bpc[..preservation_count],
        &adaptation_states[..preservation_count],
    )?;

    // Build K tone curve
    let k_tone = super::gmt::build_k_tone_curve(
        &mut profiles[..preservation_count],
        &icc_intents[..preservation_count],
        &bpc[..preservation_count],
        &adaptation_states[..preservation_count],
        4096,
        0,
    )?;

    // Build proof transform: last profile CMYK → Lab (16-bit → Lab DBL)
    let last_copy = super::gmt::clone_profile_pub(&mut profiles[last])?;
    let mut lab = Profile::new_lab4(None);
    let lab_copy = super::gmt::clone_profile_pub(&mut lab)?;

    let h_proof_output = super::xform::Transform::new(
        last_copy,
        crate::types::PixelFormat::build(crate::types::PT_CMYK, 4, 2),
        lab_copy,
        crate::types::TYPE_LAB_DBL,
        1, // Relative colorimetric
        super::xform::FLAGS_NOCACHE | super::xform::FLAGS_NOOPTIMIZE,
    )?;

    // Build CMYK→Lab float transform for the last profile
    let last_copy2 = super::gmt::clone_profile_pub(&mut profiles[last])?;
    let lab_copy2 = super::gmt::clone_profile_pub(&mut lab)?;

    let cmyk2lab = super::xform::Transform::new(
        last_copy2,
        crate::types::TYPE_CMYK_FLT,
        lab_copy2,
        crate::types::TYPE_LAB_FLT,
        1, // Relative colorimetric
        super::xform::FLAGS_NOCACHE | super::xform::FLAGS_NOOPTIMIZE,
    )?;

    // Create CLUT
    let n_grid = crate::math::pcs::reasonable_gridpoints(4, 0);
    let grid_points = [n_grid; crate::curves::intrp::MAX_INPUT_DIMENSIONS];
    let mut clut = Stage::new_clut_16bit(&grid_points, 4, 4, None).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to allocate CLUT for K-plane preservation".into(),
    })?;

    // Pre-compute byte strides
    let cmyk16_stride = crate::pipeline::pack::pixel_size(crate::types::PixelFormat::build(
        crate::types::PT_CMYK,
        4,
        2,
    ));
    let lab_dbl_stride = crate::pipeline::pack::pixel_size(crate::types::TYPE_LAB_DBL);

    let mut proof_in_buf = vec![0u8; cmyk16_stride];
    let mut proof_out_buf = vec![0u8; lab_dbl_stride];

    // Sample the CLUT
    crate::pipeline::lut::sample_clut_16bit(
        &mut clut,
        |input: &[u16], output: &mut [u16], _cargo: &()| {
            // Convert to float
            let mut inf = [0.0f32; 4];
            for i in 0..4 {
                inf[i] = input[i] as f32 / 65535.0;
            }

            // Get K across tone curve
            let lab_k3 = k_tone.eval_f32(inf[3]);

            // If C=M=Y=0, black only
            if input[0] == 0 && input[1] == 0 && input[2] == 0 {
                output[0] = 0;
                output[1] = 0;
                output[2] = 0;
                output[3] = crate::curves::intrp::quick_saturate_word(lab_k3 as f64 * 65535.0);
                return true;
            }

            // Try original transform
            let mut outf = [0.0f32; 4];
            cmyk2cmyk.eval_float(&inf, &mut outf);

            // Store initial result
            for i in 0..4 {
                output[i] = crate::curves::intrp::quick_saturate_word(outf[i] as f64 * 65535.0);
            }

            // Check if K already matches
            if (outf[3] - lab_k3).abs() < (3.0 / 65535.0) {
                return true;
            }

            // Get Lab of colorimetric output
            for (i, &v) in output.iter().enumerate().take(4) {
                proof_in_buf[i * 2..i * 2 + 2].copy_from_slice(&v.to_ne_bytes());
            }
            h_proof_output.do_transform(&proof_in_buf, &mut proof_out_buf, 1);

            // Get Lab+K for reverse interpolation
            let mut lab_k = [0.0f32; 4];
            let cmyk_flt_stride = crate::pipeline::pack::pixel_size(crate::types::TYPE_CMYK_FLT);
            let lab_flt_stride = crate::pipeline::pack::pixel_size(crate::types::TYPE_LAB_FLT);
            let mut cmyk_buf = vec![0u8; cmyk_flt_stride];
            let mut lab_buf = vec![0u8; lab_flt_stride];
            for (i, &v) in outf.iter().enumerate().take(4) {
                cmyk_buf[i * 4..i * 4 + 4].copy_from_slice(&v.to_ne_bytes());
            }
            cmyk2lab.do_transform(&cmyk_buf, &mut lab_buf, 1);
            for i in 0..3 {
                lab_k[i] = f32::from_ne_bytes(lab_buf[i * 4..i * 4 + 4].try_into().unwrap());
            }
            lab_k[3] = lab_k3;

            // Reverse interpolation: Lab+K → CMY (keeping K fixed)
            let mut reverse_out = outf;
            if !lab_k2cmyk.eval_reverse_float(&lab_k, &mut reverse_out, Some(&outf)) {
                // Cannot find suitable value, keep colorimetric
                return true;
            }
            outf[0] = reverse_out[0];
            outf[1] = reverse_out[1];
            outf[2] = reverse_out[2];
            outf[3] = lab_k3;

            // Apply TAC if needed
            let sum_cmy = outf[0] as f64 + outf[1] as f64 + outf[2] as f64;
            let sum_cmyk = sum_cmy + outf[3] as f64;

            let ratio = if sum_cmyk > max_tac {
                let r = 1.0 - (sum_cmyk - max_tac) / sum_cmy;
                if r < 0.0 { 0.0 } else { r }
            } else {
                1.0
            };

            output[0] = crate::curves::intrp::quick_saturate_word(outf[0] as f64 * ratio * 65535.0);
            output[1] = crate::curves::intrp::quick_saturate_word(outf[1] as f64 * ratio * 65535.0);
            output[2] = crate::curves::intrp::quick_saturate_word(outf[2] as f64 * ratio * 65535.0);
            output[3] = crate::curves::intrp::quick_saturate_word(outf[3] as f64 * 65535.0);

            true
        },
        0,
    );

    let mut result = Pipeline::new(4, 4).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to allocate result pipeline".into(),
    })?;
    result.insert_stage(StageLoc::AtBegin, clut);

    // Append trailing devicelinks
    for i in (last + 1)..n {
        let devlink = profiles[i].read_input_lut(icc_intents[i])?;
        if !result.cat(&devlink) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to concatenate devicelink".into(),
            });
        }
    }

    Ok(result)
}

/// Return the list of supported rendering intents.
/// C版: `cmsGetSupportedIntents`
pub fn get_supported_intents() -> &'static [(u32, &'static str)] {
    &[
        (0, "Perceptual"),
        (1, "Relative Colorimetric"),
        (2, "Saturation"),
        (3, "Absolute Colorimetric"),
        (10, "Preserve K Only Perceptual"),
        (11, "Preserve K Only Relative Colorimetric"),
        (12, "Preserve K Only Saturation"),
        (13, "Preserve K Plane Perceptual"),
        (14, "Preserve K Plane Relative Colorimetric"),
        (15, "Preserve K Plane Saturation"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curves::gamma::ToneCurve;
    use crate::profile::io::Profile;
    use crate::profile::tag_types::TagData;
    use crate::types::{
        CieXyz, ColorSpaceSignature, D50_X, D50_Y, D50_Z, ProfileClassSignature, TagSignature,
    };

    /// Build a minimal sRGB-like matrix-shaper profile for testing.
    fn build_rgb_profile() -> Profile {
        let mut p = Profile::new_placeholder();
        p.header.color_space = ColorSpaceSignature::RgbData;
        p.header.pcs = ColorSpaceSignature::XyzData;
        p.header.device_class = ProfileClassSignature::Display;
        p.header.version = 0x04200000;

        // sRGB-like colorants (D50 adapted)
        p.write_tag(
            TagSignature::RedMatrixColumn,
            TagData::Xyz(CieXyz {
                x: 0.4361,
                y: 0.2225,
                z: 0.0139,
            }),
        )
        .unwrap();
        p.write_tag(
            TagSignature::GreenMatrixColumn,
            TagData::Xyz(CieXyz {
                x: 0.3851,
                y: 0.7169,
                z: 0.0971,
            }),
        )
        .unwrap();
        p.write_tag(
            TagSignature::BlueMatrixColumn,
            TagData::Xyz(CieXyz {
                x: 0.1431,
                y: 0.0606,
                z: 0.7141,
            }),
        )
        .unwrap();

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        p.write_tag(TagSignature::RedTRC, TagData::Curve(gamma.clone()))
            .unwrap();
        p.write_tag(TagSignature::GreenTRC, TagData::Curve(gamma.clone()))
            .unwrap();
        p.write_tag(TagSignature::BlueTRC, TagData::Curve(gamma))
            .unwrap();

        p.write_tag(
            TagSignature::MediaWhitePoint,
            TagData::Xyz(CieXyz {
                x: D50_X,
                y: D50_Y,
                z: D50_Z,
            }),
        )
        .unwrap();

        p
    }

    /// Build a Lab PCS profile (same as RGB but with Lab PCS).
    fn build_rgb_lab_profile() -> Profile {
        let mut p = build_rgb_profile();
        p.header.pcs = ColorSpaceSignature::LabData;
        p
    }

    /// Round-trip a profile through save/open to get a valid readable profile.
    fn roundtrip(p: &mut Profile) -> Profile {
        let data = p.save_to_mem().unwrap();
        Profile::open_mem(&data).unwrap()
    }

    // ========================================================================
    // color_space_is_compatible
    // ========================================================================

    #[test]
    fn compatible_same_space() {
        assert!(color_space_is_compatible(
            ColorSpaceSignature::RgbData,
            ColorSpaceSignature::RgbData
        ));
    }

    #[test]
    fn compatible_xyz_lab() {
        assert!(color_space_is_compatible(
            ColorSpaceSignature::XyzData,
            ColorSpaceSignature::LabData
        ));
        assert!(color_space_is_compatible(
            ColorSpaceSignature::LabData,
            ColorSpaceSignature::XyzData
        ));
    }

    #[test]
    fn compatible_cmyk_4color() {
        assert!(color_space_is_compatible(
            ColorSpaceSignature::CmykData,
            ColorSpaceSignature::Mch4Data
        ));
    }

    #[test]
    fn incompatible_rgb_cmyk() {
        assert!(!color_space_is_compatible(
            ColorSpaceSignature::RgbData,
            ColorSpaceSignature::CmykData
        ));
    }

    // ========================================================================
    // is_empty_layer
    // ========================================================================

    #[test]
    fn empty_layer_identity() {
        let m = Mat3::identity();
        let off = Vec3::new(0.0, 0.0, 0.0);
        assert!(is_empty_layer(&m, &off));
    }

    #[test]
    fn empty_layer_non_identity() {
        let m = Mat3::identity();
        let off = Vec3::new(0.1, 0.0, 0.0);
        assert!(!is_empty_layer(&m, &off));
    }

    // ========================================================================
    // add_conversion
    // ========================================================================

    #[test]
    fn add_conversion_xyz_to_xyz_identity() {
        let mut pipe = Pipeline::new(3, 3).unwrap();
        let m = Mat3::identity();
        let off = Vec3::new(0.0, 0.0, 0.0);
        add_conversion(
            &mut pipe,
            ColorSpaceSignature::XyzData,
            ColorSpaceSignature::XyzData,
            &m,
            &off,
        )
        .unwrap();
        // Identity layer should be skipped, pipeline has no stages
        assert_eq!(pipe.stage_count(), 0);
    }

    #[test]
    fn add_conversion_xyz_to_lab() {
        let mut pipe = Pipeline::new(3, 3).unwrap();
        let m = Mat3::identity();
        let off = Vec3::new(0.0, 0.0, 0.0);
        add_conversion(
            &mut pipe,
            ColorSpaceSignature::XyzData,
            ColorSpaceSignature::LabData,
            &m,
            &off,
        )
        .unwrap();
        // Should have XYZ→Lab stage (identity matrix skipped)
        assert!(pipe.stage_count() >= 1);
    }

    #[test]
    fn add_conversion_lab_to_xyz() {
        let mut pipe = Pipeline::new(3, 3).unwrap();
        let m = Mat3::identity();
        let off = Vec3::new(0.0, 0.0, 0.0);
        add_conversion(
            &mut pipe,
            ColorSpaceSignature::LabData,
            ColorSpaceSignature::XyzData,
            &m,
            &off,
        )
        .unwrap();
        // Should have Lab→XYZ stage
        assert!(pipe.stage_count() >= 1);
    }

    // ========================================================================
    // link_profiles: 2-profile RGB→RGB (perceptual)
    // ========================================================================

    #[test]
    fn link_two_rgb_profiles_perceptual() {
        let src = roundtrip(&mut build_rgb_profile());
        let dst = roundtrip(&mut build_rgb_profile());

        let pipe = link_profiles(
            &mut [src, dst],
            &[0, 0], // perceptual
            &mut [false, false],
            &[1.0, 1.0],
        )
        .unwrap();

        // Pipeline should exist and have stages
        assert!(pipe.stage_count() > 0);
    }

    #[test]
    fn link_two_profiles_rgb_to_lab() {
        let src = roundtrip(&mut build_rgb_profile());
        let dst = roundtrip(&mut build_rgb_lab_profile());

        let pipe = link_profiles(
            &mut [src, dst],
            &[1, 1], // relative colorimetric
            &mut [false, false],
            &[1.0, 1.0],
        )
        .unwrap();

        assert!(pipe.stage_count() > 0);
    }

    // ========================================================================
    // ColorSpaceSignature::channels
    // ========================================================================

    #[test]
    fn color_space_channels() {
        assert_eq!(ColorSpaceSignature::RgbData.channels(), 3);
        assert_eq!(ColorSpaceSignature::CmykData.channels(), 4);
        assert_eq!(ColorSpaceSignature::GrayData.channels(), 1);
        assert_eq!(ColorSpaceSignature::XyzData.channels(), 3);
        assert_eq!(ColorSpaceSignature::LabData.channels(), 3);
    }

    // ========================================================================
    // get_supported_intents (Phase 14a-A)
    // ========================================================================

    #[test]
    fn supported_intents_returns_ten_entries() {
        let intents = super::get_supported_intents();
        assert_eq!(intents.len(), 10);
    }

    #[test]
    fn supported_intents_contains_standard_intents() {
        let intents = super::get_supported_intents();
        let ids: Vec<u32> = intents.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&0), "should contain Perceptual");
        assert!(ids.contains(&1), "should contain Relative Colorimetric");
        assert!(ids.contains(&2), "should contain Saturation");
        assert!(ids.contains(&3), "should contain Absolute Colorimetric");
    }

    #[test]
    fn supported_intents_contains_black_preserving() {
        let intents = super::get_supported_intents();
        let ids: Vec<u32> = intents.iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&10), "should contain K-only Perceptual");
        assert!(ids.contains(&13), "should contain K-plane Perceptual");
    }

    #[test]
    fn supported_intents_have_names() {
        let intents = super::get_supported_intents();
        for &(_, name) in intents {
            assert!(!name.is_empty(), "intent name should not be empty");
        }
    }

    // ========================================================================
    // translate_non_icc_intents (Phase 14c)
    // ========================================================================

    #[test]
    fn translate_non_icc_intents_maps_correctly() {
        assert_eq!(super::translate_non_icc_intents(10), 0);
        assert_eq!(super::translate_non_icc_intents(11), 1);
        assert_eq!(super::translate_non_icc_intents(12), 2);
        assert_eq!(super::translate_non_icc_intents(13), 0);
        assert_eq!(super::translate_non_icc_intents(14), 1);
        assert_eq!(super::translate_non_icc_intents(15), 2);
        assert_eq!(super::translate_non_icc_intents(0), 0);
        assert_eq!(super::translate_non_icc_intents(3), 3);
    }
}
