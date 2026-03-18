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
    _bpc: bool,
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
    // BPC detection is deferred: cmsDetectBlackPoint not yet implemented.
    // When implemented, BPC would compute black point scaling here.

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

    // Dispatch to intent handler (only DefaultICCintents for now)
    default_icc_intents(profiles, intents, bpc, adaptation_states)
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
}
