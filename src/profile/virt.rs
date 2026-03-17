// ============================================================================
// Virtual (built-in) profile generation (C版: cmsvirt.c)
// ============================================================================

use crate::curves::gamma::ToneCurve;
use crate::profile::io::Profile;
use crate::types::*;

impl Profile {
    /// Create an RGB matrix-shaper profile.
    /// C版: `cmsCreateRGBProfileTHR`
    pub fn new_rgb(
        _white_point: &CieXyY,
        _primaries: &CieXyYTriple,
        _transfer_function: &[ToneCurve; 3],
    ) -> Self {
        todo!("Phase 6a GREEN")
    }

    /// Create a grayscale profile.
    /// C版: `cmsCreateGrayProfileTHR`
    pub fn new_gray(_white_point: &CieXyY, _transfer_function: &ToneCurve) -> Self {
        todo!("Phase 6a GREEN")
    }

    /// Create a Lab v4 identity profile.
    /// C版: `cmsCreateLab4ProfileTHR`
    pub fn new_lab4(_white_point: Option<&CieXyY>) -> Self {
        todo!("Phase 6a GREEN")
    }

    /// Create a Lab v2 identity profile.
    /// C版: `cmsCreateLab2ProfileTHR`
    pub fn new_lab2(_white_point: Option<&CieXyY>) -> Self {
        todo!("Phase 6a GREEN")
    }

    /// Create an XYZ identity profile.
    /// C版: `cmsCreateXYZProfileTHR`
    pub fn new_xyz() -> Self {
        todo!("Phase 6a GREEN")
    }

    /// Create an sRGB profile.
    /// C版: `cmsCreate_sRGBProfileTHR`
    pub fn new_srgb() -> Self {
        todo!("Phase 6a GREEN")
    }

    /// Create a NULL profile (Lab→Gray L* extraction).
    /// C版: `cmsCreateNULLProfileTHR`
    pub fn new_null() -> Self {
        todo!("Phase 6a GREEN")
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::tag_types::TagData;
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
    #[ignore = "not yet implemented"]
    fn lab4_header() {
        let mut p = Profile::new_lab4(None);
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Abstract);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::LabData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::LabData);
        assert!(p2.header.version >= 0x04000000);
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn xyz_header() {
        let mut p = Profile::new_xyz();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Abstract);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::XyzData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::XyzData);
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn srgb_header() {
        let mut p = Profile::new_srgb();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Display);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::RgbData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::XyzData);
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn srgb_is_matrix_shaper() {
        let mut p = Profile::new_srgb();
        let p2 = roundtrip(&mut p);
        assert!(p2.is_matrix_shaper());
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn custom_rgb_profile() {
        use crate::curves::gamma::ToneCurve;

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
    #[ignore = "not yet implemented"]
    fn gray_profile() {
        use crate::curves::gamma::ToneCurve;

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
    #[ignore = "not yet implemented"]
    fn null_profile_header() {
        let mut p = Profile::new_null();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Output);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::GrayData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::LabData);
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
        let input_bytes: &[u8] = bytemuck_cast(&input);
        let mut output_buf = [0u8; 12]; // 3 × f32
        xform.do_transform(input_bytes, &mut output_buf, 1);
        let output: &[f32; 3] = bytemuck_cast_ref(&output_buf);

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

    fn bytemuck_cast(floats: &[f32; 3]) -> &[u8] {
        unsafe { std::slice::from_raw_parts(floats.as_ptr() as *const u8, 12) }
    }

    fn bytemuck_cast_ref(bytes: &[u8; 12]) -> &[f32; 3] {
        unsafe { &*(bytes.as_ptr() as *const [f32; 3]) }
    }
}
