// Multi-profile pipeline linking.
// C版: cmscnvrt.c

#![allow(dead_code, unused_variables, unused_imports)]

use crate::context::{CmsError, ErrorCode};
use crate::math::mtrx::{Mat3, Vec3};
use crate::pipeline::lut::{Pipeline, Stage, StageLoc};
use crate::profile::io::Profile;
use crate::types::ColorSpaceSignature;

/// Check if two color spaces are compatible for PCS connection.
/// XYZ and Lab are interchangeable; CMYK and 4-color are interchangeable.
/// C版: `ColorSpaceIsCompatible`
pub fn color_space_is_compatible(a: ColorSpaceSignature, b: ColorSpaceSignature) -> bool {
    todo!("Phase 5a: color_space_is_compatible")
}

/// Check if a matrix+offset layer is effectively identity (no-op).
/// C版: `IsEmptyLayer`
pub fn is_empty_layer(m: &Mat3, off: &Vec3) -> bool {
    todo!("Phase 5a: is_empty_layer")
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
    todo!("Phase 5a: add_conversion")
}

/// Build a multi-profile pipeline for ICC standard intents.
/// C版: `DefaultICCintents`
pub fn default_icc_intents(
    profiles: &mut [Profile],
    intents: &[u32],
    bpc: &[bool],
    adaptation_states: &[f64],
) -> Result<Pipeline, CmsError> {
    todo!("Phase 5a: default_icc_intents")
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
    todo!("Phase 5a: link_profiles")
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
    #[ignore = "not yet implemented"]
    fn compatible_same_space() {
        assert!(color_space_is_compatible(
            ColorSpaceSignature::RgbData,
            ColorSpaceSignature::RgbData
        ));
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn compatible_cmyk_4color() {
        assert!(color_space_is_compatible(
            ColorSpaceSignature::CmykData,
            ColorSpaceSignature::Mch4Data
        ));
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn empty_layer_identity() {
        let m = Mat3::identity();
        let off = Vec3::new(0.0, 0.0, 0.0);
        assert!(is_empty_layer(&m, &off));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn empty_layer_non_identity() {
        let m = Mat3::identity();
        let off = Vec3::new(0.1, 0.0, 0.0);
        assert!(!is_empty_layer(&m, &off));
    }

    // ========================================================================
    // add_conversion
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn color_space_channels() {
        assert_eq!(ColorSpaceSignature::RgbData.channels(), 3);
        assert_eq!(ColorSpaceSignature::CmykData.channels(), 4);
        assert_eq!(ColorSpaceSignature::GrayData.channels(), 1);
        assert_eq!(ColorSpaceSignature::XyzData.channels(), 3);
        assert_eq!(ColorSpaceSignature::LabData.channels(), 3);
    }
}
