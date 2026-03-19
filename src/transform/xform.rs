// ============================================================================
// Transform engine (C版: cmsxform.c)
// ============================================================================

use crate::context::{CmsError, ErrorCode};
use crate::pipeline::lut::Pipeline;
use crate::pipeline::pack::{
    CMS_PACK_FLAGS_16BITS, CMS_PACK_FLAGS_FLOAT, FormatterIn, FormatterOut, find_formatter_in,
    find_formatter_out, pixel_size,
};
use crate::profile::io::Profile;
use crate::transform::cnvrt;
use crate::types::{ColorSpaceSignature, MAX_CHANNELS, PixelFormat};

use crate::types::{ProfileClassSignature, StageSignature, TagSignature};

// ============================================================================
// Transform flags
// ============================================================================

pub const FLAGS_CLUT_POST_LINEARIZATION: u32 = 0x0001;
pub const FLAGS_FORCE_CLUT: u32 = 0x0002;
pub const FLAGS_NOWHITEONWHITEFIXUP: u32 = 0x0004;
pub const FLAGS_CLUT_PRE_LINEARIZATION: u32 = 0x0010;
pub const FLAGS_NOCACHE: u32 = 0x0040;
pub const FLAGS_NOOPTIMIZE: u32 = 0x0100;
pub const FLAGS_NULLTRANSFORM: u32 = 0x0200;
pub const FLAGS_HIGHRESPRECALC: u32 = 0x0400;
pub const FLAGS_LOWRESPRECALC: u32 = 0x0800;
pub const FLAGS_GAMUTCHECK: u32 = 0x1000;
pub const FLAGS_SOFTPROOFING: u32 = 0x4000;
pub const FLAGS_BLACKPOINTCOMPENSATION: u32 = 0x2000;
pub const FLAGS_COPY_ALPHA: u32 = 0x04000000;

// ============================================================================
// Device link helpers
// ============================================================================

fn is_pcs(cs: ColorSpaceSignature) -> bool {
    cs == ColorSpaceSignature::XyzData || cs == ColorSpaceSignature::LabData
}

/// Set device class and color spaces based on PCS status.
/// C版: `FixColorSpaces`
fn fix_color_spaces(profile: &mut Profile, entry: ColorSpaceSignature, exit: ColorSpaceSignature) {
    // Default: device link
    profile.header.device_class = ProfileClassSignature::Link;
    profile.header.color_space = entry;
    profile.header.pcs = exit;

    // Both PCS → Abstract
    if is_pcs(entry) && is_pcs(exit) {
        profile.header.device_class = ProfileClassSignature::Abstract;
    }
}

/// Allowed LUT stage combination for ICC tag writing.
struct AllowedLut {
    is_v4: bool,
    required_tag: Option<TagSignature>,
    stage_types: &'static [StageSignature],
}

#[rustfmt::skip]
const ALLOWED_LUT_TYPES: &[AllowedLut] = &[
    // V2: Lut16Type
    AllowedLut { is_v4: false, required_tag: None, stage_types: &[StageSignature::MatrixElem, StageSignature::CurveSetElem, StageSignature::CLutElem, StageSignature::CurveSetElem] },
    AllowedLut { is_v4: false, required_tag: None, stage_types: &[StageSignature::CurveSetElem, StageSignature::CLutElem, StageSignature::CurveSetElem] },
    AllowedLut { is_v4: false, required_tag: None, stage_types: &[StageSignature::CurveSetElem, StageSignature::CLutElem] },
    // V4 AToB: LutAtoBType
    AllowedLut { is_v4: true, required_tag: None, stage_types: &[StageSignature::CurveSetElem] },
    AllowedLut { is_v4: true, required_tag: Some(TagSignature::AToB0), stage_types: &[StageSignature::CurveSetElem, StageSignature::MatrixElem, StageSignature::CurveSetElem] },
    AllowedLut { is_v4: true, required_tag: Some(TagSignature::AToB0), stage_types: &[StageSignature::CurveSetElem, StageSignature::CLutElem, StageSignature::CurveSetElem] },
    AllowedLut { is_v4: true, required_tag: Some(TagSignature::AToB0), stage_types: &[StageSignature::CurveSetElem, StageSignature::CLutElem, StageSignature::CurveSetElem, StageSignature::MatrixElem, StageSignature::CurveSetElem] },
    // V4 BToA: LutBtoAType
    AllowedLut { is_v4: true, required_tag: Some(TagSignature::BToA0), stage_types: &[StageSignature::CurveSetElem] },
    AllowedLut { is_v4: true, required_tag: Some(TagSignature::BToA0), stage_types: &[StageSignature::CurveSetElem, StageSignature::MatrixElem, StageSignature::CurveSetElem] },
    AllowedLut { is_v4: true, required_tag: Some(TagSignature::BToA0), stage_types: &[StageSignature::CurveSetElem, StageSignature::CLutElem, StageSignature::CurveSetElem] },
    AllowedLut { is_v4: true, required_tag: Some(TagSignature::BToA0), stage_types: &[StageSignature::MatrixElem, StageSignature::CurveSetElem, StageSignature::CLutElem, StageSignature::CurveSetElem, StageSignature::CurveSetElem] },
];

/// Check if pipeline stages match an allowed combination.
/// C版: `FindCombination`
fn find_combination(lut: &Pipeline, is_v4: bool, dest_tag: TagSignature) -> Option<usize> {
    let stages = lut.stages();
    for (i, entry) in ALLOWED_LUT_TYPES.iter().enumerate() {
        if entry.is_v4 != is_v4 {
            continue;
        }
        if entry.required_tag.is_some_and(|req| req != dest_tag) {
            continue;
        }
        if stages.len() != entry.stage_types.len() {
            continue;
        }
        let matches = stages
            .iter()
            .zip(entry.stage_types.iter())
            .all(|(s, t)| s.stage_type() == *t);
        if matches {
            return Some(i);
        }
    }
    None
}

/// Color transform: converts pixel data between ICC profiles.
pub struct Transform {
    pipeline: Pipeline,
    input_format: PixelFormat,
    output_format: PixelFormat,
    from_input: FormatterIn,
    to_output: FormatterOut,
    flags: u32,
    entry_color_space: ColorSpaceSignature,
    exit_color_space: ColorSpaceSignature,
    rendering_intent: u32,
}

impl Transform {
    pub fn input_format(&self) -> PixelFormat {
        self.input_format
    }

    pub fn output_format(&self) -> PixelFormat {
        self.output_format
    }

    pub fn pipeline(&self) -> &Pipeline {
        &self.pipeline
    }

    /// Convert this transform into a device link profile.
    /// C版: `cmsTransform2DeviceLink`
    /// Convert this transform into a device link profile.
    /// C版: `cmsTransform2DeviceLink`
    pub fn to_device_link(&self, version: f64) -> Result<Profile, CmsError> {
        use crate::pipeline::lut::{Stage, StageLoc};
        use crate::profile::tag_types::TagData;

        let mut lut = self.pipeline.clone();

        // Lab V2/V4 encoding fix
        if self.entry_color_space == ColorSpaceSignature::LabData
            && version < 4.0
            && let Some(stage) = Stage::new_lab_v2_to_v4_curves()
        {
            lut.insert_stage(StageLoc::AtBegin, stage);
        }
        if self.exit_color_space == ColorSpaceSignature::LabData
            && version < 4.0
            && let Some(stage) = Stage::new_lab_v4_to_v2()
        {
            lut.insert_stage(StageLoc::AtEnd, stage);
        }

        // Create profile and set header
        let mut p = Profile::new_placeholder();
        p.set_version_f64(version);
        fix_color_spaces(&mut p, self.entry_color_space, self.exit_color_space);

        // Determine destination tag
        let dest_tag = if p.header.device_class == ProfileClassSignature::Output {
            TagSignature::BToA0
        } else {
            TagSignature::AToB0
        };

        let is_v4 = version >= 4.0;

        // Phase 1: Try direct match
        let mut found = find_combination(&lut, is_v4, dest_tag);

        // Phase 2: Optimize and retry
        if found.is_none() {
            let mut flags = self.flags;
            super::opt::optimize_pipeline(&mut lut, self.rendering_intent, &mut flags);
            found = find_combination(&lut, is_v4, dest_tag);
        }

        // Phase 3: Force CLUT, ensure curve wrappers, retry
        if found.is_none() {
            let mut flags = self.flags | FLAGS_FORCE_CLUT;
            super::opt::optimize_pipeline(&mut lut, self.rendering_intent, &mut flags);

            // Ensure first stage is curves
            if lut
                .first_stage()
                .is_some_and(|s| s.stage_type() != StageSignature::CurveSetElem)
                && let Some(stage) = Stage::new_identity_curves(lut.input_channels())
            {
                lut.insert_stage(StageLoc::AtBegin, stage);
            }

            // Ensure last stage is curves
            if lut
                .last_stage()
                .is_some_and(|s| s.stage_type() != StageSignature::CurveSetElem)
                && let Some(stage) = Stage::new_identity_curves(lut.output_channels())
            {
                lut.insert_stage(StageLoc::AtEnd, stage);
            }

            found = find_combination(&lut, is_v4, dest_tag);
        }

        if found.is_none() {
            return Err(CmsError {
                code: ErrorCode::NotSuitable,
                message: "no compatible LUT format for device link".into(),
            });
        }

        // Write tags
        crate::profile::virt::set_text_tags_public(&mut p, "devicelink");

        let d50 = crate::curves::wtpnt::d50_xyz();
        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(d50));
        let _ = p.write_tag(dest_tag, TagData::Pipeline(lut));
        p.header.rendering_intent = self.rendering_intent;

        Ok(p)
    }

    /// Create a transform from two profiles (consumed).
    pub fn new(
        input_profile: Profile,
        input_format: PixelFormat,
        output_profile: Profile,
        output_format: PixelFormat,
        intent: u32,
        flags: u32,
    ) -> Result<Self, CmsError> {
        Self::new_multiprofile(
            &mut [input_profile, output_profile],
            input_format,
            output_format,
            intent,
            flags,
        )
    }

    /// Create a transform from multiple profiles.
    pub fn new_multiprofile(
        profiles: &mut [Profile],
        input_format: PixelFormat,
        output_format: PixelFormat,
        intent: u32,
        flags: u32,
    ) -> Result<Self, CmsError> {
        if profiles.is_empty() {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: "at least one profile required".into(),
            });
        }

        // Reject mixed float/integer formats
        if input_format.is_float() != output_format.is_float() {
            return Err(CmsError {
                code: ErrorCode::NotSuitable,
                message: "mixed float/integer formats are not supported".into(),
            });
        }

        // Capture color spaces from first/last profiles
        // Entry is always the first profile's device color space.
        // Exit depends on the last profile's class:
        //   Input/Link/Abstract → exit is PCS (header.pcs)
        //   Output/Display → exit is device color space (header.color_space)
        let entry_color_space = profiles[0].header.color_space;
        let last = &profiles[profiles.len() - 1];
        let last_class = last.header.device_class;
        let exit_color_space = if last_class == ProfileClassSignature::Link
            || last_class == ProfileClassSignature::Abstract
            || last_class == ProfileClassSignature::Input
        {
            last.header.pcs
        } else {
            last.header.color_space
        };

        // Build pipeline from profiles
        let n = profiles.len();
        let intents = vec![intent; n];
        let bpc_flag = (flags & FLAGS_BLACKPOINTCOMPENSATION) != 0;
        let mut bpc = vec![bpc_flag; n];
        let adaptation = vec![1.0f64; n];
        let mut pipeline = cnvrt::link_profiles(profiles, &intents, &mut bpc, &adaptation)?;

        // Optimize pipeline (unless FLAGS_NOOPTIMIZE)
        let mut opt_flags = flags;
        super::opt::optimize_pipeline(&mut pipeline, intent, &mut opt_flags);

        // Validate format channels against linked pipeline
        if input_format.channels() != pipeline.input_channels() {
            return Err(CmsError {
                code: ErrorCode::ColorspaceCheck,
                message: format!(
                    "input format channels ({}) != pipeline input channels ({})",
                    input_format.channels(),
                    pipeline.input_channels()
                ),
            });
        }
        if output_format.channels() != pipeline.output_channels() {
            return Err(CmsError {
                code: ErrorCode::ColorspaceCheck,
                message: format!(
                    "output format channels ({}) != pipeline output channels ({})",
                    output_format.channels(),
                    pipeline.output_channels()
                ),
            });
        }

        // Select formatters
        let pack_flags = if input_format.is_float() {
            CMS_PACK_FLAGS_FLOAT
        } else {
            CMS_PACK_FLAGS_16BITS
        };

        let from_input = find_formatter_in(input_format, pack_flags).ok_or_else(|| CmsError {
            code: ErrorCode::NotSuitable,
            message: format!("no input formatter for format {:#010X}", input_format.0),
        })?;

        let to_output = find_formatter_out(output_format, pack_flags).ok_or_else(|| CmsError {
            code: ErrorCode::NotSuitable,
            message: format!("no output formatter for format {:#010X}", output_format.0),
        })?;

        Ok(Transform {
            pipeline,
            input_format,
            output_format,
            from_input,
            to_output,
            flags,
            entry_color_space,
            exit_color_space,
            rendering_intent: intent,
        })
    }

    /// Transform a buffer of pixels.
    pub fn do_transform(&self, input: &[u8], output: &mut [u8], pixel_count: usize) {
        // Copy extra (alpha) channels if requested
        if self.flags & FLAGS_COPY_ALPHA != 0 {
            super::alpha::handle_extra_channels(
                self.input_format,
                self.output_format,
                input,
                output,
                pixel_count,
            );
        }

        match (&self.from_input, &self.to_output) {
            (FormatterIn::U16(unroll), FormatterOut::U16(pack)) => {
                self.do_transform_16(*unroll, *pack, input, output, pixel_count);
            }
            (FormatterIn::Float(unroll), FormatterOut::Float(pack)) => {
                self.do_transform_float(*unroll, *pack, input, output, pixel_count);
            }
            _ => unreachable!("mixed float/u16 formatters rejected at creation"),
        }
    }

    fn do_transform_16(
        &self,
        unroll: crate::pipeline::pack::Formatter16In,
        pack: crate::pipeline::pack::Formatter16Out,
        input: &[u8],
        output: &mut [u8],
        pixel_count: usize,
    ) {
        let in_stride = pixel_size(self.input_format);
        let out_stride = pixel_size(self.output_format);
        // Clamp to buffer capacity
        let max_in = if in_stride > 0 {
            input.len() / in_stride
        } else {
            0
        };
        let max_out = if out_stride > 0 {
            output.len() / out_stride
        } else {
            0
        };
        let count = pixel_count.min(max_in).min(max_out);
        let mut w_in = [0u16; MAX_CHANNELS];
        let mut w_out = [0u16; MAX_CHANNELS];

        for i in 0..count {
            let in_offset = i * in_stride;
            let out_offset = i * out_stride;
            unroll(self.input_format, &mut w_in, &input[in_offset..], 0);
            self.pipeline.eval_16(&w_in, &mut w_out);
            pack(self.output_format, &w_out, &mut output[out_offset..], 0);
        }
    }

    fn do_transform_float(
        &self,
        unroll: crate::pipeline::pack::FormatterFloatIn,
        pack: crate::pipeline::pack::FormatterFloatOut,
        input: &[u8],
        output: &mut [u8],
        pixel_count: usize,
    ) {
        let in_stride = pixel_size(self.input_format);
        let out_stride = pixel_size(self.output_format);
        let max_in = if in_stride > 0 {
            input.len() / in_stride
        } else {
            0
        };
        let max_out = if out_stride > 0 {
            output.len() / out_stride
        } else {
            0
        };
        let count = pixel_count.min(max_in).min(max_out);
        let mut w_in = [0.0f32; MAX_CHANNELS];
        let mut w_out = [0.0f32; MAX_CHANNELS];

        for i in 0..count {
            let in_offset = i * in_stride;
            let out_offset = i * out_stride;
            unroll(self.input_format, &mut w_in, &input[in_offset..], 0);
            self.pipeline.eval_float(&w_in, &mut w_out);
            pack(self.output_format, &w_out, &mut output[out_offset..], 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curves::gamma::ToneCurve;
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
    fn test_to_pixel_type() {
        assert_eq!(ColorSpaceSignature::RgbData.to_pixel_type(), PT_RGB);
        assert_eq!(ColorSpaceSignature::CmykData.to_pixel_type(), PT_CMYK);
        assert_eq!(ColorSpaceSignature::GrayData.to_pixel_type(), PT_GRAY);
        assert_eq!(ColorSpaceSignature::LabData.to_pixel_type(), PT_LAB);
        assert_eq!(ColorSpaceSignature::XyzData.to_pixel_type(), PT_XYZ);
    }

    #[test]
    fn test_from_pixel_type() {
        assert_eq!(
            ColorSpaceSignature::from_pixel_type(PT_RGB),
            Some(ColorSpaceSignature::RgbData)
        );
        assert_eq!(
            ColorSpaceSignature::from_pixel_type(PT_CMYK),
            Some(ColorSpaceSignature::CmykData)
        );
        assert_eq!(
            ColorSpaceSignature::from_pixel_type(PT_LAB_V2),
            Some(ColorSpaceSignature::LabData)
        );
        assert_eq!(ColorSpaceSignature::from_pixel_type(0), None); // PT_ANY
    }

    // ================================================================
    // Transform creation
    // ================================================================

    #[test]
    fn test_create_rgb_to_rgb_transform() {
        let src = roundtrip(&mut build_rgb_profile());
        let dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(
            src, TYPE_RGB_8, dst, TYPE_RGB_8, 0, // perceptual
            0,
        );
        assert!(xform.is_ok());
    }

    #[test]
    fn test_format_query() {
        let src = roundtrip(&mut build_rgb_profile());
        let dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_16, 0, 0).unwrap();
        assert_eq!(xform.input_format(), TYPE_RGB_8);
        assert_eq!(xform.output_format(), TYPE_RGB_16);
    }

    #[test]
    fn test_format_mismatch_error() {
        let src = roundtrip(&mut build_rgb_profile());
        let dst = roundtrip(&mut build_rgb_profile());
        // CMYK format for RGB profile should fail
        let result = Transform::new(src, TYPE_CMYK_8, dst, TYPE_RGB_8, 0, 0);
        assert!(result.is_err());
    }

    // ================================================================
    // do_transform
    // ================================================================

    #[test]
    fn test_do_transform_rgb8_identity() {
        // Same profile for input and output — should be approximately identity
        let src = roundtrip(&mut build_rgb_profile());
        let dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

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
    fn test_do_transform_rgb8_to_rgb16() {
        let src = roundtrip(&mut build_rgb_profile());
        let dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_16, 1, 0).unwrap();

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

    // ================================================================
    // to_device_link
    // ================================================================

    #[test]
    fn test_device_link_rgb_to_rgb() {
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

        let mut dl = xform.to_device_link(4.4).unwrap();
        let dl = roundtrip(&mut dl);
        assert_eq!(dl.header.device_class, ProfileClassSignature::Link);
        assert_eq!(dl.header.color_space, ColorSpaceSignature::RgbData);
    }

    #[test]
    fn test_device_link_has_lut_tag() {
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

        let mut dl = xform.to_device_link(4.4).unwrap();
        let mut dl = roundtrip(&mut dl);
        // Device link should have AToB0 tag
        assert!(dl.read_tag(TagSignature::AToB0).is_ok());
    }

    #[test]
    fn test_device_link_pipeline_preserved() {
        // Verify that device link profile contains a valid pipeline
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

        let mut dl = xform.to_device_link(4.4).unwrap();
        let mut dl = roundtrip(&mut dl);

        // Read the pipeline from AToB0
        if let Ok(TagData::Pipeline(lut)) = dl.read_tag(TagSignature::AToB0) {
            assert_eq!(lut.input_channels(), 3);
            assert_eq!(lut.output_channels(), 3);
        } else {
            panic!("device link should have a valid Pipeline in AToB0");
        }
    }
}
