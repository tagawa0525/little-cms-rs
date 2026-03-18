// ============================================================================
// Virtual (built-in) profile generation (C版: cmsvirt.c)
// ============================================================================

use crate::curves::gamma::ToneCurve;
use crate::curves::wtpnt;
use crate::math::pcs;
use crate::pipeline::lut::{Pipeline, Stage, StageLoc};
use crate::pipeline::named::Mlu;
use crate::profile::io::Profile;
use crate::profile::tag_types::TagData;
use crate::types::*;

// ============================================================================
// sRGB constants
// ============================================================================

/// D65 white point chromaticity.
pub const D65: CieXyY = CieXyY {
    x: 0.3127,
    y: 0.3290,
    big_y: 1.0,
};

/// ITU-R BT.709 primaries (sRGB).
pub const REC709_PRIMARIES: CieXyYTriple = CieXyYTriple {
    red: CieXyY {
        x: 0.6400,
        y: 0.3300,
        big_y: 1.0,
    },
    green: CieXyY {
        x: 0.3000,
        y: 0.6000,
        big_y: 1.0,
    },
    blue: CieXyY {
        x: 0.1500,
        y: 0.0600,
        big_y: 1.0,
    },
};

// ============================================================================
// Internal helpers
// ============================================================================

/// Write ProfileDescription and Copyright tags.
/// C版: `SetTextTags`
fn set_text_tags(profile: &mut Profile, description: &str) {
    let mut desc_mlu = Mlu::new();
    desc_mlu.set_ascii("en", "US", description);
    let _ = profile.write_tag(TagSignature::ProfileDescription, TagData::Mlu(desc_mlu));

    let mut copy_mlu = Mlu::new();
    copy_mlu.set_ascii("en", "US", "No copyright, use freely");
    let _ = profile.write_tag(TagSignature::Copyright, TagData::Mlu(copy_mlu));
}

/// Build sRGB parametric gamma curve (type 4).
fn build_srgb_gamma() -> ToneCurve {
    let params = [2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045];
    ToneCurve::build_parametric(4, &params).expect("sRGB gamma parameters are valid")
}

// ============================================================================
// Profile constructors
// ============================================================================

impl Profile {
    /// Create an RGB matrix-shaper profile.
    /// C版: `cmsCreateRGBProfileTHR`
    pub fn new_rgb(
        white_point: &CieXyY,
        primaries: &CieXyYTriple,
        transfer_function: &[ToneCurve; 3],
    ) -> Self {
        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::Display;
        p.header.color_space = ColorSpaceSignature::RgbData;
        p.header.pcs = ColorSpaceSignature::XyzData;
        p.header.rendering_intent = 0; // Perceptual

        set_text_tags(&mut p, "RGB built-in");

        // Media white point is always D50 in the profile
        let d50 = wtpnt::d50_xyz();
        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(d50));

        // Chromatic adaptation matrix (white_point → D50)
        let wp_xyz = pcs::xyy_to_xyz(white_point);
        if let Some(chad) = wtpnt::adaptation_matrix(None, &wp_xyz, &d50) {
            // Store as S15Fixed16 array (row-major 3×3 = 9 values)
            let arr: Vec<f64> = (0..3)
                .flat_map(|r| (0..3).map(move |c| chad.0[r].0[c]))
                .collect();
            let _ = p.write_tag(
                TagSignature::ChromaticAdaptation,
                TagData::S15Fixed16Array(arr),
            );
        }

        // Build RGB→XYZ matrix from primaries and white point
        if let Some(m) = wtpnt::build_rgb_to_xyz_matrix(white_point, primaries) {
            // Extract columns as colorant XYZ values
            let red = CieXyz {
                x: m.0[0].0[0],
                y: m.0[1].0[0],
                z: m.0[2].0[0],
            };
            let green = CieXyz {
                x: m.0[0].0[1],
                y: m.0[1].0[1],
                z: m.0[2].0[1],
            };
            let blue = CieXyz {
                x: m.0[0].0[2],
                y: m.0[1].0[2],
                z: m.0[2].0[2],
            };

            let _ = p.write_tag(TagSignature::RedMatrixColumn, TagData::Xyz(red));
            let _ = p.write_tag(TagSignature::GreenMatrixColumn, TagData::Xyz(green));
            let _ = p.write_tag(TagSignature::BlueMatrixColumn, TagData::Xyz(blue));

            let _ = p.write_tag(
                TagSignature::Chromaticity,
                TagData::Chromaticity(*primaries),
            );
        }

        // TRC tags
        let _ = p.write_tag(
            TagSignature::RedTRC,
            TagData::Curve(transfer_function[0].clone()),
        );
        let _ = p.write_tag(
            TagSignature::GreenTRC,
            TagData::Curve(transfer_function[1].clone()),
        );
        let _ = p.write_tag(
            TagSignature::BlueTRC,
            TagData::Curve(transfer_function[2].clone()),
        );

        p
    }

    /// Create a grayscale profile.
    /// C版: `cmsCreateGrayProfileTHR`
    pub fn new_gray(white_point: &CieXyY, transfer_function: &ToneCurve) -> Self {
        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::Display;
        p.header.color_space = ColorSpaceSignature::GrayData;
        p.header.pcs = ColorSpaceSignature::XyzData;
        p.header.rendering_intent = 0;

        set_text_tags(&mut p, "gray built-in");

        // Media white point is always D50 in the profile (same as new_rgb)
        let d50 = wtpnt::d50_xyz();
        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(d50));

        // Chromatic adaptation matrix (white_point → D50)
        let wp_xyz = pcs::xyy_to_xyz(white_point);
        if let Some(chad) = wtpnt::adaptation_matrix(None, &wp_xyz, &d50) {
            let arr: Vec<f64> = (0..3)
                .flat_map(|r| (0..3).map(move |c| chad.0[r].0[c]))
                .collect();
            let _ = p.write_tag(
                TagSignature::ChromaticAdaptation,
                TagData::S15Fixed16Array(arr),
            );
        }

        let _ = p.write_tag(
            TagSignature::GrayTRC,
            TagData::Curve(transfer_function.clone()),
        );

        p
    }

    /// Create a Lab v4 identity profile.
    /// C版: `cmsCreateLab4ProfileTHR`
    pub fn new_lab4(white_point: Option<&CieXyY>) -> Self {
        let wp = white_point.copied().unwrap_or_else(wtpnt::d50_xyy);
        let wp_xyz = pcs::xyy_to_xyz(&wp);

        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::Abstract;
        p.header.color_space = ColorSpaceSignature::LabData;
        p.header.pcs = ColorSpaceSignature::LabData;

        set_text_tags(&mut p, "Lab identity built-in");

        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(wp_xyz));

        // Identity pipeline with identity curves
        if let Some(mut lut) = Pipeline::new(3, 3) {
            if let Some(stage) = Stage::new_identity_curves(3) {
                lut.insert_stage(StageLoc::AtBegin, stage);
            }
            let _ = p.write_tag(TagSignature::AToB0, TagData::Pipeline(lut));
        }

        p
    }

    /// Create a Lab v2 identity profile.
    /// C版: `cmsCreateLab2ProfileTHR`
    ///
    /// Note: text tags are written as mluc (v4 type) for simplicity,
    /// matching the C version's behavior. Strict ICC v2 conformance
    /// would require textDescriptionType, deferred to a future phase.
    pub fn new_lab2(white_point: Option<&CieXyY>) -> Self {
        let wp = white_point.copied().unwrap_or_else(wtpnt::d50_xyy);
        let wp_xyz = pcs::xyy_to_xyz(&wp);

        let mut p = Profile::new_placeholder();
        p.set_version_f64(2.1);
        p.header.device_class = ProfileClassSignature::Abstract;
        p.header.color_space = ColorSpaceSignature::LabData;
        p.header.pcs = ColorSpaceSignature::LabData;

        set_text_tags(&mut p, "Lab identity built-in");

        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(wp_xyz));

        // Identity pipeline with identity CLUT
        if let Some(mut lut) = Pipeline::new(3, 3) {
            if let Some(stage) = Stage::new_identity_clut(3) {
                lut.insert_stage(StageLoc::AtBegin, stage);
            }
            let _ = p.write_tag(TagSignature::AToB0, TagData::Pipeline(lut));
        }

        p
    }

    /// Create an XYZ identity profile.
    /// C版: `cmsCreateXYZProfileTHR`
    pub fn new_xyz() -> Self {
        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::Abstract;
        p.header.color_space = ColorSpaceSignature::XyzData;
        p.header.pcs = ColorSpaceSignature::XyzData;

        set_text_tags(&mut p, "XYZ identity built-in");

        let d50 = wtpnt::d50_xyz();
        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(d50));

        // Identity pipeline with identity curves
        if let Some(mut lut) = Pipeline::new(3, 3) {
            if let Some(stage) = Stage::new_identity_curves(3) {
                lut.insert_stage(StageLoc::AtBegin, stage);
            }
            let _ = p.write_tag(TagSignature::AToB0, TagData::Pipeline(lut));
        }

        p
    }

    /// Create an sRGB profile.
    /// C版: `cmsCreate_sRGBProfileTHR`
    pub fn new_srgb() -> Self {
        let gamma = build_srgb_gamma();
        let trc = [gamma.clone(), gamma.clone(), gamma];
        let mut p = Self::new_rgb(&D65, &REC709_PRIMARIES, &trc);
        set_text_tags(&mut p, "sRGB built-in");
        p
    }

    /// Create a NULL profile (Lab→Gray L* extraction).
    /// C版: `cmsCreateNULLProfileTHR`
    pub fn new_null() -> Self {
        use crate::pipeline::lut::sample_clut_16bit;

        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::Output;
        p.header.color_space = ColorSpaceSignature::GrayData;
        p.header.pcs = ColorSpaceSignature::LabData;

        set_text_tags(&mut p, "NULL profile built-in");

        let d50 = wtpnt::d50_xyz();
        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(d50));

        // Build pipeline: Lab(3) → Gray(1)
        // Uses a 3→1 CLUT that extracts L* (first component).
        // ICC LutBtoA format requires CLUT for channel reduction.
        if let Some(mut lut) = Pipeline::new(3, 1) {
            // B curves (3-channel identity, input side)
            if let Some(stage) = Stage::new_identity_curves(3) {
                lut.insert_stage(StageLoc::AtEnd, stage);
            }

            // 3→1 CLUT: extract L* (first component)
            if let Some(mut clut) = Stage::new_clut_16bit_uniform(2, 3, 1, None) {
                sample_clut_16bit(
                    &mut clut,
                    |inp, out, _| {
                        out[0] = inp[0];
                        true
                    },
                    0,
                );
                lut.insert_stage(StageLoc::AtEnd, clut);
            }

            // A curves (1-channel identity, output side)
            if let Some(stage) = Stage::new_identity_curves(1) {
                lut.insert_stage(StageLoc::AtEnd, stage);
            }

            let _ = p.write_tag(TagSignature::BToA0, TagData::Pipeline(lut));
        }

        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::xform::Transform;

    // ================================================================
    // Helper: roundtrip profile through save/load
    // ================================================================

    fn roundtrip(p: &mut Profile) -> Profile {
        let data = p.save_to_mem().unwrap();
        Profile::open_mem(&data).unwrap()
    }

    // ================================================================
    // Profile::new_lab4
    // ================================================================

    #[test]
    fn lab4_header() {
        let mut p = Profile::new_lab4(None);
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Abstract);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::LabData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::LabData);
        assert!(p2.header.version >= 0x04000000);
    }

    #[test]
    fn lab4_has_atob0() {
        let mut p = Profile::new_lab4(None);
        let mut p2 = roundtrip(&mut p);
        let tag = p2.read_tag(TagSignature::AToB0);
        assert!(tag.is_ok(), "Lab4 profile should have AToB0 tag");
    }

    // ================================================================
    // Profile::new_lab2
    // ================================================================

    #[test]
    fn lab2_header() {
        let mut p = Profile::new_lab2(None);
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Abstract);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::LabData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::LabData);
        // V2 profile
        assert!(p2.header.version < 0x04000000);
    }

    // ================================================================
    // Profile::new_xyz
    // ================================================================

    #[test]
    fn xyz_header() {
        let mut p = Profile::new_xyz();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Abstract);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::XyzData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::XyzData);
    }

    #[test]
    fn xyz_has_atob0() {
        let mut p = Profile::new_xyz();
        let mut p2 = roundtrip(&mut p);
        let tag = p2.read_tag(TagSignature::AToB0);
        assert!(tag.is_ok(), "XYZ profile should have AToB0 tag");
    }

    // ================================================================
    // Profile::new_srgb
    // ================================================================

    #[test]
    fn srgb_header() {
        let mut p = Profile::new_srgb();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Display);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::RgbData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::XyzData);
    }

    #[test]
    fn srgb_has_matrix_shaper_tags() {
        let mut p = Profile::new_srgb();
        let mut p2 = roundtrip(&mut p);
        // Should have colorant tags
        assert!(p2.read_tag(TagSignature::RedMatrixColumn).is_ok());
        assert!(p2.read_tag(TagSignature::GreenMatrixColumn).is_ok());
        assert!(p2.read_tag(TagSignature::BlueMatrixColumn).is_ok());
        // Should have TRC tags
        assert!(p2.read_tag(TagSignature::RedTRC).is_ok());
        assert!(p2.read_tag(TagSignature::GreenTRC).is_ok());
        assert!(p2.read_tag(TagSignature::BlueTRC).is_ok());
    }

    #[test]
    fn srgb_is_matrix_shaper() {
        let mut p = Profile::new_srgb();
        let p2 = roundtrip(&mut p);
        assert!(p2.is_matrix_shaper());
    }

    #[test]
    fn srgb_has_description() {
        let mut p = Profile::new_srgb();
        let mut p2 = roundtrip(&mut p);
        if let Ok(TagData::Mlu(mlu)) = p2.read_tag(TagSignature::ProfileDescription) {
            let desc = mlu.get_ascii("en", "US").unwrap_or_default();
            assert!(
                desc.contains("sRGB"),
                "description should mention sRGB, got: {desc}"
            );
        } else {
            panic!("ProfileDescription tag should be Mlu");
        }
    }

    #[test]
    fn srgb_has_chromatic_adaptation() {
        let mut p = Profile::new_srgb();
        let mut p2 = roundtrip(&mut p);
        let tag = p2.read_tag(TagSignature::ChromaticAdaptation);
        assert!(tag.is_ok(), "sRGB should have ChromaticAdaptation tag (V4)");
    }

    // ================================================================
    // Profile::new_rgb (custom)
    // ================================================================

    #[test]
    fn custom_rgb_profile() {
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let trc = [gamma.clone(), gamma.clone(), gamma];
        let mut p = Profile::new_rgb(&D65, &REC709_PRIMARIES, &trc);
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::RgbData);
        assert!(p2.is_matrix_shaper());
    }

    // ================================================================
    // Profile::new_gray
    // ================================================================

    #[test]
    fn gray_profile() {
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let mut p = Profile::new_gray(&D65, &gamma);
        let mut p2 = roundtrip(&mut p);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::GrayData);
        assert!(p2.read_tag(TagSignature::GrayTRC).is_ok());
    }

    // ================================================================
    // Profile::new_null
    // ================================================================

    #[test]
    fn null_profile_header() {
        let mut p = Profile::new_null();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Output);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::GrayData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::LabData);
    }

    #[test]
    fn null_profile_has_btoa0() {
        let mut p = Profile::new_null();
        let mut p2 = roundtrip(&mut p);
        let tag = p2.read_tag(TagSignature::BToA0);
        assert!(tag.is_ok(), "NULL profile should have BToA0 tag");
    }

    // ================================================================
    // End-to-end transform tests
    // ================================================================

    #[test]
    fn srgb_identity_transform() {
        let src = {
            let mut p = Profile::new_srgb();
            roundtrip(&mut p)
        };
        let dst = {
            let mut p = Profile::new_srgb();
            roundtrip(&mut p)
        };
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 0, 0).unwrap();

        // Mid-gray should round-trip closely
        let input: [u8; 3] = [128, 128, 128];
        let mut output = [0u8; 3];
        xform.do_transform(&input, &mut output, 1);

        for i in 0..3 {
            assert!(
                (output[i] as i16 - input[i] as i16).unsigned_abs() <= 2,
                "sRGB identity byte {i}: input={}, output={}",
                input[i],
                output[i]
            );
        }
    }

    #[test]
    fn srgb_to_lab_transform() {
        let src = {
            let mut p = Profile::new_srgb();
            roundtrip(&mut p)
        };
        let dst = {
            let mut p = Profile::new_lab4(None);
            roundtrip(&mut p)
        };
        // sRGB→Lab requires float path (Lab format is float-encoded)
        let xform = Transform::new(src, TYPE_RGB_FLT, dst, TYPE_LAB_FLT, 0, 0).unwrap();

        // White (1,1,1) should map to L≈100, a≈0, b≈0
        let input: [f32; 3] = [1.0, 1.0, 1.0];
        let input_bytes = floats_to_bytes(&input);
        let mut output_buf = [0u8; 12]; // 3 × f32
        xform.do_transform(&input_bytes, &mut output_buf, 1);
        let output = bytes_to_floats(&output_buf);

        assert!(
            (output[0] - 100.0).abs() < 1.0,
            "L* should be ~100 for white, got {}",
            output[0]
        );
        assert!(
            output[1].abs() < 3.0,
            "a* should be ~0 for white, got {}",
            output[1]
        );
        assert!(
            output[2].abs() < 3.0,
            "b* should be ~0 for white, got {}",
            output[2]
        );
    }

    // ================================================================
    // Helpers for float byte casting
    // ================================================================

    fn floats_to_bytes(floats: &[f32; 3]) -> [u8; 12] {
        let mut buf = [0u8; 12];
        for (i, &f) in floats.iter().enumerate() {
            buf[i * 4..(i + 1) * 4].copy_from_slice(&f.to_ne_bytes());
        }
        buf
    }

    fn bytes_to_floats(bytes: &[u8; 12]) -> [f32; 3] {
        [
            f32::from_ne_bytes(bytes[0..4].try_into().unwrap()),
            f32::from_ne_bytes(bytes[4..8].try_into().unwrap()),
            f32::from_ne_bytes(bytes[8..12].try_into().unwrap()),
        ]
    }
}
