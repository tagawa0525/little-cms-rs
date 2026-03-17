// ============================================================================
// Transform engine (C版: cmsxform.c)
// ============================================================================

use crate::context::CmsError;
use crate::pipeline::lut::Pipeline;
use crate::pipeline::pack::{FormatterIn, FormatterOut};
use crate::profile::io::Profile;
use crate::types::{ColorSpaceSignature, PixelFormat};

// ============================================================================
// Transform flags
// ============================================================================

pub const FLAGS_NOCACHE: u32 = 0x0040;
pub const FLAGS_NOOPTIMIZE: u32 = 0x0100;
pub const FLAGS_NULLTRANSFORM: u32 = 0x0200;
pub const FLAGS_GAMUTCHECK: u32 = 0x1000;
pub const FLAGS_SOFTPROOFING: u32 = 0x4000;
pub const FLAGS_BLACKPOINTCOMPENSATION: u32 = 0x2000;

/// Color transform: converts pixel data between ICC profiles.
#[allow(dead_code)] // Fields used in GREEN commit
pub struct Transform {
    pipeline: Pipeline,
    input_format: PixelFormat,
    output_format: PixelFormat,
    from_input: FormatterIn,
    to_output: FormatterOut,
    entry_color_space: ColorSpaceSignature,
    exit_color_space: ColorSpaceSignature,
    rendering_intent: u32,
    flags: u32,
}

impl Transform {
    pub fn input_format(&self) -> PixelFormat {
        todo!()
    }

    pub fn output_format(&self) -> PixelFormat {
        todo!()
    }

    /// Create a transform from two profiles.
    pub fn new(
        _input_profile: &mut Profile,
        _input_format: PixelFormat,
        _output_profile: &mut Profile,
        _output_format: PixelFormat,
        _intent: u32,
        _flags: u32,
    ) -> Result<Self, CmsError> {
        todo!()
    }

    /// Create a transform from multiple profiles.
    pub fn new_multiprofile(
        _profiles: &mut [Profile],
        _input_format: PixelFormat,
        _output_format: PixelFormat,
        _intent: u32,
        _flags: u32,
    ) -> Result<Self, CmsError> {
        todo!()
    }

    /// Transform a buffer of pixels.
    pub fn do_transform(&self, _input: &[u8], _output: &mut [u8], _pixel_count: usize) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curves::gamma::ToneCurve;
    #[allow(unused_imports)]
    use crate::pipeline::pack::{from_8_to_16, from_16_to_8};
    use crate::profile::io::Profile;
    use crate::profile::tag_types::TagData;
    use crate::types::*;

    /// Build a minimal sRGB-like matrix-shaper profile for testing.
    fn build_rgb_profile() -> Profile {
        let mut p = Profile::new_placeholder();
        p.header.color_space = ColorSpaceSignature::RgbData;
        p.header.pcs = ColorSpaceSignature::XyzData;
        p.header.device_class = ProfileClassSignature::Display;
        p.header.version = 0x04200000;

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

    fn roundtrip(p: &mut Profile) -> Profile {
        let data = p.save_to_mem().unwrap();
        Profile::open_mem(&data).unwrap()
    }

    // ================================================================
    // ColorSpaceSignature ↔ PT_* conversion
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_to_pixel_type() {
        assert_eq!(ColorSpaceSignature::RgbData.to_pixel_type(), PT_RGB);
        assert_eq!(ColorSpaceSignature::CmykData.to_pixel_type(), PT_CMYK);
        assert_eq!(ColorSpaceSignature::GrayData.to_pixel_type(), PT_GRAY);
        assert_eq!(ColorSpaceSignature::LabData.to_pixel_type(), PT_LAB);
        assert_eq!(ColorSpaceSignature::XyzData.to_pixel_type(), PT_XYZ);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_from_pixel_type() {
        assert_eq!(
            ColorSpaceSignature::from_pixel_type(PT_RGB),
            Some(ColorSpaceSignature::RgbData)
        );
        assert_eq!(
            ColorSpaceSignature::from_pixel_type(PT_CMYK),
            Some(ColorSpaceSignature::CmykData)
        );
        assert_eq!(ColorSpaceSignature::from_pixel_type(0), None); // PT_ANY
    }

    // ================================================================
    // Transform creation
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_create_rgb_to_rgb_transform() {
        let mut src = roundtrip(&mut build_rgb_profile());
        let mut dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(
            &mut src, TYPE_RGB_8, &mut dst, TYPE_RGB_8, 0, // perceptual
            0,
        );
        assert!(xform.is_ok());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_format_query() {
        let mut src = roundtrip(&mut build_rgb_profile());
        let mut dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(&mut src, TYPE_RGB_8, &mut dst, TYPE_RGB_16, 0, 0).unwrap();
        assert_eq!(xform.input_format(), TYPE_RGB_8);
        assert_eq!(xform.output_format(), TYPE_RGB_16);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_format_mismatch_error() {
        let mut src = roundtrip(&mut build_rgb_profile());
        let mut dst = roundtrip(&mut build_rgb_profile());
        // CMYK format for RGB profile should fail
        let result = Transform::new(&mut src, TYPE_CMYK_8, &mut dst, TYPE_RGB_8, 0, 0);
        assert!(result.is_err());
    }

    // ================================================================
    // do_transform
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_do_transform_rgb8_identity() {
        // Same profile for input and output — should be approximately identity
        let mut src = roundtrip(&mut build_rgb_profile());
        let mut dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(&mut src, TYPE_RGB_8, &mut dst, TYPE_RGB_8, 1, 0).unwrap();

        let input: [u8; 6] = [255, 0, 0, 0, 128, 255]; // 2 pixels: red, cyan-ish
        let mut output = [0u8; 6];
        xform.do_transform(&input, &mut output, 2);

        // With same profile, output should be close to input (within gamma round-trip error)
        for i in 0..6 {
            assert!(
                (output[i] as i16 - input[i] as i16).unsigned_abs() <= 3,
                "pixel byte {i}: input={}, output={}",
                input[i],
                output[i]
            );
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_do_transform_rgb8_to_rgb16() {
        let mut src = roundtrip(&mut build_rgb_profile());
        let mut dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(&mut src, TYPE_RGB_8, &mut dst, TYPE_RGB_16, 1, 0).unwrap();

        let input: [u8; 3] = [128, 128, 128]; // mid-gray
        let mut output = [0u8; 6]; // RGB_16 = 3 channels * 2 bytes
        xform.do_transform(&input, &mut output, 1);

        // Output should be non-zero (gray maps to gray)
        let r = u16::from_ne_bytes([output[0], output[1]]);
        let g = u16::from_ne_bytes([output[2], output[3]]);
        let b = u16::from_ne_bytes([output[4], output[5]]);
        // Mid-gray should produce roughly equal R, G, B
        assert!(r > 0x2000, "R too low: {r:#06X}");
        assert!((r as i32 - g as i32).unsigned_abs() < 0x1000);
        assert!((g as i32 - b as i32).unsigned_abs() < 0x1000);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_do_transform_multiprofile() {
        let p1 = roundtrip(&mut build_rgb_profile());
        let p2 = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new_multiprofile(&mut [p1, p2], TYPE_RGB_8, TYPE_RGB_8, 1, 0);
        assert!(xform.is_ok());
        let xform = xform.unwrap();

        let input: [u8; 3] = [200, 100, 50];
        let mut output = [0u8; 3];
        xform.do_transform(&input, &mut output, 1);
        // Should produce non-zero output
        assert!(output[0] > 0 || output[1] > 0 || output[2] > 0);
    }
}
