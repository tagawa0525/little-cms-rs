// ============================================================================
// Transform engine (C版: cmsxform.c)
// ============================================================================

use std::cell::Cell;

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
/// Default alarm codes (C版: `_cmsAlarmCodesChunk`)
pub const DEFAULT_ALARM_CODES: [u16; MAX_CHANNELS] = [
    0x7F00, 0x7F00, 0x7F00, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

pub struct Transform {
    pipeline: Option<Pipeline>,
    input_format: PixelFormat,
    output_format: PixelFormat,
    from_input: FormatterIn,
    to_output: FormatterOut,
    flags: u32,
    entry_color_space: ColorSpaceSignature,
    exit_color_space: ColorSpaceSignature,
    rendering_intent: u32,
    gamut_check: Option<Pipeline>,
    alarm_codes: [u16; MAX_CHANNELS],
    cache_in: Cell<[u16; MAX_CHANNELS]>,
    cache_out: Cell<[u16; MAX_CHANNELS]>,
}

impl Transform {
    pub fn input_format(&self) -> PixelFormat {
        self.input_format
    }

    pub fn output_format(&self) -> PixelFormat {
        self.output_format
    }

    /// Returns the pipeline, if any. Null transforms have no pipeline.
    pub fn pipeline(&self) -> Option<&Pipeline> {
        self.pipeline.as_ref()
    }

    /// Convert this transform into a device link profile.
    /// C版: `cmsTransform2DeviceLink`
    pub fn to_device_link(&self, version: f64) -> Result<Profile, CmsError> {
        use crate::pipeline::lut::{Stage, StageLoc};
        use crate::profile::tag_types::TagData;

        let mut lut = self
            .pipeline
            .as_ref()
            .ok_or_else(|| CmsError {
                code: ErrorCode::NotSuitable,
                message: "null transform cannot be converted to device link".into(),
            })?
            .clone();

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
        // Device links always use AToB0 (fix_color_spaces never sets Output class)
        let dest_tag = TagSignature::AToB0;

        let is_v4 = version >= 4.0;

        // Phase 1: Try direct match
        let mut found = find_combination(&lut, is_v4, dest_tag);

        // Phase 2: Optimize and retry (mask out NOOPTIMIZE for device link path)
        // Device link optimization doesn't use format-specific fast paths (pass 0)
        if found.is_none() {
            let mut flags = self.flags & !FLAGS_NOOPTIMIZE;
            super::opt::optimize_pipeline(&mut lut, self.rendering_intent, &mut flags, 0, 0);
            found = find_combination(&lut, is_v4, dest_tag);
        }

        // Phase 3: Force CLUT, ensure curve wrappers, retry
        if found.is_none() {
            let mut flags = (self.flags & !FLAGS_NOOPTIMIZE) | FLAGS_FORCE_CLUT;
            super::opt::optimize_pipeline(&mut lut, self.rendering_intent, &mut flags, 0, 0);

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

        // Write tags (propagate errors)
        crate::profile::virt::set_text_tags_fallible(&mut p, "devicelink")?;

        let d50 = crate::curves::wtpnt::d50_xyz();
        p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(d50))?;
        p.write_tag(dest_tag, TagData::Pipeline(lut))?;
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
        super::opt::optimize_pipeline(
            &mut pipeline,
            intent,
            &mut opt_flags,
            input_format.0,
            output_format.0,
        );

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

        let mut xform = Transform {
            pipeline: Some(pipeline),
            input_format,
            output_format,
            from_input,
            to_output,
            flags,
            entry_color_space,
            exit_color_space,
            rendering_intent: intent,
            gamut_check: None,
            alarm_codes: DEFAULT_ALARM_CODES,
            cache_in: Cell::new([0u16; MAX_CHANNELS]),
            cache_out: Cell::new([0u16; MAX_CHANNELS]),
        };

        // Initialize cache if enabled (16-bit non-float only)
        if !input_format.is_float() && (flags & FLAGS_NOCACHE) == 0 {
            xform.init_cache();
        }

        Ok(xform)
    }

    /// Create a proofing transform with gamut check support.
    ///
    /// C版: `cmsCreateProofingTransformTHR`
    #[allow(clippy::too_many_arguments)]
    pub fn new_proofing(
        mut input_profile: Profile,
        input_format: PixelFormat,
        mut output_profile: Profile,
        output_format: PixelFormat,
        mut proofing_profile: Profile,
        intent: u32,
        proofing_intent: u32,
        flags: u32,
    ) -> Result<Self, CmsError> {
        // Reject mixed float/integer formats (same check as new_multiprofile)
        if input_format.is_float() != output_format.is_float() {
            return Err(CmsError {
                code: ErrorCode::NotSuitable,
                message: "mixed float/integer formats are not supported".into(),
            });
        }

        // Without SOFTPROOFING or GAMUTCHECK, fall back to simple transform
        if (flags & (FLAGS_SOFTPROOFING | FLAGS_GAMUTCHECK)) == 0 {
            return Self::new(
                input_profile,
                input_format,
                output_profile,
                output_format,
                intent,
                flags,
            );
        }

        let bpc = (flags & FLAGS_BLACKPOINTCOMPENSATION) != 0;

        // Serialize profiles so we can create copies
        let input_data = input_profile.save_to_mem()?;
        let output_data = output_profile.save_to_mem()?;
        let proofing_data = proofing_profile.save_to_mem()?;

        // Build gamut check pipeline if requested
        let gamut_check = if (flags & FLAGS_GAMUTCHECK) != 0 {
            let p0 = Profile::open_mem(&input_data)?;
            let p1 = Profile::open_mem(&proofing_data)?;
            let mut gamut = Profile::open_mem(&proofing_data)?;
            Some(super::gmt::create_gamut_check_pipeline(
                &mut [p0, p1],
                &[bpc, bpc],
                &[intent, intent],
                &[1.0, 1.0],
                1,
                &mut gamut,
            )?)
        } else {
            None
        };

        // Build 4-profile proofing chain:
        // [Input, Proofing, Proofing, Output]
        // C版: cmsCreateProofingTransformTHR
        let p0 = Profile::open_mem(&input_data)?;
        let p1 = Profile::open_mem(&proofing_data)?;
        let p2 = Profile::open_mem(&proofing_data)?;
        let p3 = Profile::open_mem(&output_data)?;

        let intents = [
            intent,
            intent,
            1, /* RELATIVE_COLORIMETRIC */
            proofing_intent,
        ];
        let mut bpc_arr = [bpc, bpc, false, false];
        let adaptation = [1.0f64; 4];

        let entry_color_space = p0.header.color_space;
        let last = &p3;
        let last_class = last.header.device_class;
        let exit_color_space = if last_class == ProfileClassSignature::Link
            || last_class == ProfileClassSignature::Abstract
            || last_class == ProfileClassSignature::Input
        {
            last.header.pcs
        } else {
            last.header.color_space
        };

        let mut pipeline =
            cnvrt::link_profiles(&mut [p0, p1, p2, p3], &intents, &mut bpc_arr, &adaptation)?;

        // Optimize
        let mut opt_flags = flags;
        super::opt::optimize_pipeline(
            &mut pipeline,
            intent,
            &mut opt_flags,
            input_format.0,
            output_format.0,
        );

        // Validate channels
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

        let mut xform = Transform {
            pipeline: Some(pipeline),
            input_format,
            output_format,
            from_input,
            to_output,
            flags,
            entry_color_space,
            exit_color_space,
            rendering_intent: intent,
            gamut_check,
            alarm_codes: DEFAULT_ALARM_CODES,
            cache_in: Cell::new([0u16; MAX_CHANNELS]),
            cache_out: Cell::new([0u16; MAX_CHANNELS]),
        };

        if !input_format.is_float() && (flags & FLAGS_NOCACHE) == 0 {
            xform.init_cache();
        }

        Ok(xform)
    }

    /// Create a transform from multiple profiles with per-profile intent, BPC,
    /// and adaptation state. Optionally includes gamut check with a separate
    /// gamut profile.
    ///
    /// C版: `cmsCreateExtendedTransform`
    #[allow(clippy::too_many_arguments)]
    pub fn new_extended(
        profiles: &mut [Profile],
        bpc: &[bool],
        intents: &[u32],
        adaptation: &[f64],
        gamut_profile: Option<&mut Profile>,
        gamut_pcs_position: usize,
        input_format: PixelFormat,
        output_format: PixelFormat,
        mut flags: u32,
    ) -> Result<Self, CmsError> {
        let n = profiles.len();
        if n == 0 || n > 255 {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: format!("Wrong number of profiles. 1..255 expected, {n} found."),
            });
        }

        // Reject mixed float/integer
        if input_format.is_float() != output_format.is_float() {
            return Err(CmsError {
                code: ErrorCode::NotSuitable,
                message: "mixed float/integer formats are not supported".into(),
            });
        }

        // Float transforms always disable cache
        if input_format.is_float() || output_format.is_float() {
            flags |= FLAGS_NOCACHE;
        }

        let last_intent = intents[n - 1];

        // Null transform: skip pipeline, just unpack→pack
        if (flags & FLAGS_NULLTRANSFORM) != 0 {
            return Self::build_null_transform(input_format, output_format, flags);
        }

        // Validate gamut check parameters
        if (flags & FLAGS_GAMUTCHECK) != 0 {
            if gamut_profile.is_none() {
                flags &= !FLAGS_GAMUTCHECK;
            } else if gamut_pcs_position == 0 || gamut_pcs_position >= n {
                return Err(CmsError {
                    code: ErrorCode::Range,
                    message: format!("Wrong gamut PCS position '{gamut_pcs_position}'"),
                });
            }
        }

        // Determine entry/exit color spaces
        let entry_color_space = profiles[0].header.color_space;
        let last = &profiles[n - 1];
        let last_class = last.header.device_class;
        let exit_color_space = if last_class == ProfileClassSignature::Link
            || last_class == ProfileClassSignature::Abstract
            || last_class == ProfileClassSignature::Input
        {
            last.header.pcs
        } else {
            last.header.color_space
        };

        // Detect linear RGB 16-bit input → disable optimization (γ < 1.6)
        if entry_color_space == ColorSpaceSignature::RgbData
            && input_format.bytes() == 2
            && (flags & FLAGS_NOOPTIMIZE) == 0
        {
            let gamma = super::gmt::detect_rgb_profile_gamma(&mut profiles[0], 0.1);
            if gamma > 0.0 && gamma < 1.6 {
                flags |= FLAGS_NOOPTIMIZE;
            }
        }

        // Build pipeline with per-profile arrays
        let mut bpc_owned: Vec<bool> = bpc.to_vec();
        let mut pipeline = cnvrt::link_profiles(profiles, intents, &mut bpc_owned, adaptation)?;

        // Optimize
        let mut opt_flags = flags;
        super::opt::optimize_pipeline(
            &mut pipeline,
            last_intent,
            &mut opt_flags,
            input_format.0,
            output_format.0,
        );

        // Validate channels
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

        // Build gamut check pipeline if requested
        let gamut_check = if let Some(gamut) = gamut_profile {
            if (flags & FLAGS_GAMUTCHECK) != 0 {
                Some(super::gmt::create_gamut_check_pipeline(
                    profiles,
                    bpc,
                    intents,
                    adaptation,
                    gamut_pcs_position,
                    gamut,
                )?)
            } else {
                None
            }
        } else {
            None
        };

        let mut xform = Transform {
            pipeline: Some(pipeline),
            input_format,
            output_format,
            from_input,
            to_output,
            flags,
            entry_color_space,
            exit_color_space,
            rendering_intent: last_intent,
            gamut_check,
            alarm_codes: DEFAULT_ALARM_CODES,
            cache_in: Cell::new([0u16; MAX_CHANNELS]),
            cache_out: Cell::new([0u16; MAX_CHANNELS]),
        };

        if !input_format.is_float() && (flags & FLAGS_NOCACHE) == 0 {
            xform.init_cache();
        }

        Ok(xform)
    }

    /// Change the pixel format of an existing transform without rebuilding
    /// the pipeline. Only works on 16-bit (non-float) transforms.
    ///
    /// C版: `cmsChangeBuffersFormat`
    pub fn change_buffers_format(
        &mut self,
        input_format: PixelFormat,
        output_format: PixelFormat,
    ) -> Result<(), CmsError> {
        // Float formats not supported
        if input_format.is_float() || output_format.is_float() {
            return Err(CmsError {
                code: ErrorCode::NotSuitable,
                message: "change_buffers_format only works with 16-bit precision".into(),
            });
        }

        let from_input =
            find_formatter_in(input_format, CMS_PACK_FLAGS_16BITS).ok_or_else(|| CmsError {
                code: ErrorCode::NotSuitable,
                message: format!("no input formatter for format {:#010X}", input_format.0),
            })?;
        let to_output =
            find_formatter_out(output_format, CMS_PACK_FLAGS_16BITS).ok_or_else(|| CmsError {
                code: ErrorCode::NotSuitable,
                message: format!("no output formatter for format {:#010X}", output_format.0),
            })?;

        self.input_format = input_format;
        self.output_format = output_format;
        self.from_input = from_input;
        self.to_output = to_output;
        Ok(())
    }

    /// Build a null transform (unpack→pack only, no pipeline).
    fn build_null_transform(
        input_format: PixelFormat,
        output_format: PixelFormat,
        flags: u32,
    ) -> Result<Self, CmsError> {
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
            pipeline: None,
            input_format,
            output_format,
            from_input,
            to_output,
            flags,
            entry_color_space: ColorSpaceSignature::RgbData,
            exit_color_space: ColorSpaceSignature::RgbData,
            rendering_intent: 0,
            gamut_check: None,
            alarm_codes: DEFAULT_ALARM_CODES,
            cache_in: Cell::new([0u16; MAX_CHANNELS]),
            cache_out: Cell::new([0u16; MAX_CHANNELS]),
        })
    }

    /// Initialize the 1-pixel cache by evaluating input=0.
    fn init_cache(&mut self) {
        let zero_in = [0u16; MAX_CHANNELS];
        let mut zero_out = [0u16; MAX_CHANNELS];
        if let Some(ref gamut) = self.gamut_check {
            let mut w_gamut = [0u16; MAX_CHANNELS];
            gamut.eval_16(&zero_in, &mut w_gamut);
            if w_gamut[0] >= 1 {
                let n_out = self.output_format.channels() as usize;
                zero_out[..n_out].copy_from_slice(&self.alarm_codes[..n_out]);
            } else if let Some(ref pipe) = self.pipeline {
                pipe.eval_16(&zero_in, &mut zero_out);
            }
        } else if let Some(ref pipe) = self.pipeline {
            pipe.eval_16(&zero_in, &mut zero_out);
        }
        self.cache_in.set(zero_in);
        self.cache_out.set(zero_out);
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

        let is_null = self.pipeline.is_none();
        let use_cache = !is_null && (self.flags & FLAGS_NOCACHE) == 0;

        for i in 0..count {
            let in_offset = i * in_stride;
            let out_offset = i * out_stride;
            unroll(self.input_format, &mut w_in, &input[in_offset..], 0);

            if is_null {
                // Null transform: pass through
                w_out = w_in;
            } else if use_cache && w_in == self.cache_in.get() {
                // Cache hit
                w_out = self.cache_out.get();
            } else if let Some(ref gamut) = self.gamut_check {
                let mut w_gamut = [0u16; MAX_CHANNELS];
                gamut.eval_16(&w_in, &mut w_gamut);
                if w_gamut[0] >= 1 {
                    let n_out = self.output_format.channels() as usize;
                    w_out[..n_out].copy_from_slice(&self.alarm_codes[..n_out]);
                } else {
                    self.pipeline.as_ref().unwrap().eval_16(&w_in, &mut w_out);
                }
                if use_cache {
                    self.cache_in.set(w_in);
                    self.cache_out.set(w_out);
                }
            } else {
                self.pipeline.as_ref().unwrap().eval_16(&w_in, &mut w_out);
                if use_cache {
                    self.cache_in.set(w_in);
                    self.cache_out.set(w_out);
                }
            }

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

        let is_null = self.pipeline.is_none();

        for i in 0..count {
            let in_offset = i * in_stride;
            let out_offset = i * out_stride;
            unroll(self.input_format, &mut w_in, &input[in_offset..], 0);

            if is_null {
                // Null transform: pass through
                w_out = w_in;
            } else if let Some(ref gamut) = self.gamut_check {
                let mut out_of_gamut = [0.0f32; MAX_CHANNELS];
                gamut.eval_float(&w_in, &mut out_of_gamut);
                if out_of_gamut[0] > 0.0 {
                    for (w, &alarm) in w_out.iter_mut().zip(self.alarm_codes.iter()) {
                        *w = alarm as f32 / 65535.0;
                    }
                } else {
                    self.pipeline
                        .as_ref()
                        .unwrap()
                        .eval_float(&w_in, &mut w_out);
                }
            } else {
                self.pipeline
                    .as_ref()
                    .unwrap()
                    .eval_float(&w_in, &mut w_out);
            }

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

    // ================================================================
    // Proofing transform
    // ================================================================

    #[test]
    fn test_proofing_transform_basic() {
        // Create a proofing transform: sRGB → sRGB with sRGB proof
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let proof = roundtrip(&mut Profile::new_srgb());

        let xform = Transform::new_proofing(
            src,
            TYPE_RGB_8,
            dst,
            TYPE_RGB_8,
            proof,
            0,                  // perceptual intent
            1,                  // proofing intent: relative colorimetric
            FLAGS_SOFTPROOFING, // soft proofing only
        )
        .unwrap();

        // Mid-gray should come through approximately intact
        let input: [u8; 3] = [128, 128, 128];
        let mut output = [0u8; 3];
        xform.do_transform(&input, &mut output, 1);

        for i in 0..3 {
            assert!(
                (output[i] as i16 - input[i] as i16).unsigned_abs() <= 10,
                "channel {i}: input={}, output={}",
                input[i],
                output[i]
            );
        }
    }

    #[test]
    fn test_proofing_transform_gamut_check_alarm() {
        // With FLAGS_GAMUTCHECK, out-of-gamut colors should produce alarm codes
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());

        // Create a narrow-gamut proofing profile (very small primaries)
        let gamma_curve = ToneCurve::build_gamma(2.2).unwrap();
        let trc = [gamma_curve.clone(), gamma_curve.clone(), gamma_curve];
        let d65 = CieXyY {
            x: 0.3127,
            y: 0.3290,
            big_y: 1.0,
        };
        // Very narrow gamut: primaries close to white point
        let narrow_primaries = CieXyYTriple {
            red: CieXyY {
                x: 0.40,
                y: 0.35,
                big_y: 1.0,
            },
            green: CieXyY {
                x: 0.30,
                y: 0.40,
                big_y: 1.0,
            },
            blue: CieXyY {
                x: 0.25,
                y: 0.25,
                big_y: 1.0,
            },
        };
        let mut narrow = Profile::new_rgb(&d65, &narrow_primaries, &trc);
        let proof = roundtrip(&mut narrow);

        let alarm = [0x7F00u16; 16];
        let xform = Transform::new_proofing(
            src,
            TYPE_RGB_8,
            dst,
            TYPE_RGB_8,
            proof,
            0, // perceptual
            1, // proofing: relative colorimetric
            FLAGS_SOFTPROOFING | FLAGS_GAMUTCHECK,
        )
        .unwrap();

        // Pure red (255,0,0) should be out of the narrow gamut
        let input: [u8; 3] = [255, 0, 0];
        let mut output = [0u8; 3];
        xform.do_transform(&input, &mut output, 1);

        // Alarm code 0x7F00 → 8-bit = 0x7F = 127
        let alarm_byte = (alarm[0] >> 8) as u8;
        assert_eq!(
            output[0], alarm_byte,
            "out-of-gamut R should be alarm code, got {}",
            output[0]
        );
        assert_eq!(
            output[1], alarm_byte,
            "out-of-gamut G should be alarm code, got {}",
            output[1]
        );
        assert_eq!(
            output[2], alarm_byte,
            "out-of-gamut B should be alarm code, got {}",
            output[2]
        );
    }

    #[test]
    fn test_proofing_no_flags_fallback() {
        // Without SOFTPROOFING/GAMUTCHECK, should be a simple 2-profile transform
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let proof = roundtrip(&mut Profile::new_srgb());

        let xform = Transform::new_proofing(
            src, TYPE_RGB_8, dst, TYPE_RGB_8, proof, 0, 1, 0, // no flags
        )
        .unwrap();

        let input: [u8; 3] = [200, 100, 50];
        let mut output = [0u8; 3];
        xform.do_transform(&input, &mut output, 1);

        // Should produce valid output (approximately identity for same profiles)
        for i in 0..3 {
            assert!(
                (output[i] as i16 - input[i] as i16).unsigned_abs() <= 5,
                "channel {i}: input={}, output={}",
                input[i],
                output[i]
            );
        }
    }

    // ================================================================
    // Phase 11: Null transform
    // ================================================================

    #[test]

    fn test_null_transform_16bit() {
        // FLAGS_NULLTRANSFORM: unpack→pack without pipeline evaluation
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let xform =
            Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_16, 0, FLAGS_NULLTRANSFORM).unwrap();

        let input: [u8; 3] = [128, 64, 255];
        let mut output = [0u8; 6]; // RGB_16 = 3 × 2 bytes
        xform.do_transform(&input, &mut output, 1);

        // Null transform: unpack 8-bit → internal 16-bit → pack 16-bit
        // 128 → FROM_8_TO_16 = (128*65535+128)/255 = 0x8080
        let r = u16::from_ne_bytes([output[0], output[1]]);
        let g = u16::from_ne_bytes([output[2], output[3]]);
        let b = u16::from_ne_bytes([output[4], output[5]]);
        assert!(r > 0x7000, "R too low: {r:#06X}");
        assert!(g > 0x3000, "G too low: {g:#06X}");
        assert!(b > 0xF000, "B too low: {b:#06X}");
    }

    #[test]

    fn test_null_transform_float() {
        // Float null transform: unpack→pack without pipeline
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let xform =
            Transform::new(src, TYPE_RGB_FLT, dst, TYPE_RGB_FLT, 0, FLAGS_NULLTRANSFORM).unwrap();

        let input: [u8; 12] = {
            let mut buf = [0u8; 12];
            buf[0..4].copy_from_slice(&0.5f32.to_ne_bytes());
            buf[4..8].copy_from_slice(&0.25f32.to_ne_bytes());
            buf[8..12].copy_from_slice(&1.0f32.to_ne_bytes());
            buf
        };
        let mut output = [0u8; 12];
        xform.do_transform(&input, &mut output, 1);

        let r = f32::from_ne_bytes(output[0..4].try_into().unwrap());
        let g = f32::from_ne_bytes(output[4..8].try_into().unwrap());
        let b = f32::from_ne_bytes(output[8..12].try_into().unwrap());
        assert!((r - 0.5).abs() < 0.01, "R: {r}");
        assert!((g - 0.25).abs() < 0.01, "G: {g}");
        assert!((b - 1.0).abs() < 0.01, "B: {b}");
    }

    // ================================================================
    // Phase 11: 1-pixel cache
    // ================================================================

    #[test]

    fn test_cache_hit_returns_same_output() {
        // Transform with cache (no FLAGS_NOCACHE): same input twice → same output
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

        let input: [u8; 3] = [200, 100, 50];
        let mut output1 = [0u8; 3];
        let mut output2 = [0u8; 3];
        xform.do_transform(&input, &mut output1, 1);
        xform.do_transform(&input, &mut output2, 1);

        assert_eq!(
            output1, output2,
            "cache hit should produce identical output"
        );
    }

    #[test]

    fn test_cache_miss_updates_output() {
        // Transform with cache: different inputs → different outputs (cache miss)
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut build_rgb_profile());
        let xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

        let input1: [u8; 3] = [200, 100, 50];
        let input2: [u8; 3] = [50, 100, 200];
        let mut output1 = [0u8; 3];
        let mut output2 = [0u8; 3];
        xform.do_transform(&input1, &mut output1, 1);
        xform.do_transform(&input2, &mut output2, 1);

        // Different inputs must produce different outputs
        assert_ne!(
            output1, output2,
            "different inputs should produce different outputs"
        );
    }

    // ================================================================
    // Phase 11: new_extended
    // ================================================================

    #[test]

    fn test_new_extended_basic() {
        // Per-profile intent/BPC with 2 profiles
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());

        let xform = Transform::new_extended(
            &mut [src, dst],
            &[false, false],
            &[1, 1], // relative colorimetric for both
            &[1.0, 1.0],
            None, // no gamut profile
            0,    // no gamut PCS position
            TYPE_RGB_8,
            TYPE_RGB_8,
            0,
        )
        .unwrap();

        let input: [u8; 3] = [128, 128, 128];
        let mut output = [0u8; 3];
        xform.do_transform(&input, &mut output, 1);

        // Same profile → approximately identity
        for i in 0..3 {
            assert!(
                (output[i] as i16 - input[i] as i16).unsigned_abs() <= 5,
                "channel {i}: input={}, output={}",
                input[i],
                output[i]
            );
        }
    }

    #[test]

    fn test_new_extended_gamut_check() {
        // Extended transform with gamut check
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());

        // Create narrow gamut proof profile
        let gamma_curve = ToneCurve::build_gamma(2.2).unwrap();
        let trc = [gamma_curve.clone(), gamma_curve.clone(), gamma_curve];
        let d65 = CieXyY {
            x: 0.3127,
            y: 0.3290,
            big_y: 1.0,
        };
        let narrow_primaries = CieXyYTriple {
            red: CieXyY {
                x: 0.40,
                y: 0.35,
                big_y: 1.0,
            },
            green: CieXyY {
                x: 0.30,
                y: 0.40,
                big_y: 1.0,
            },
            blue: CieXyY {
                x: 0.25,
                y: 0.25,
                big_y: 1.0,
            },
        };
        let mut narrow = Profile::new_rgb(&d65, &narrow_primaries, &trc);
        let mut gamut = roundtrip(&mut narrow);

        let xform = Transform::new_extended(
            &mut [src, dst],
            &[false, false],
            &[0, 0], // perceptual
            &[1.0, 1.0],
            Some(&mut gamut),
            1,
            TYPE_RGB_8,
            TYPE_RGB_8,
            FLAGS_SOFTPROOFING | FLAGS_GAMUTCHECK,
        )
        .unwrap();

        // Saturated red → out of narrow gamut → alarm codes
        let input: [u8; 3] = [255, 0, 0];
        let mut output = [0u8; 3];
        xform.do_transform(&input, &mut output, 1);

        let alarm_byte = (DEFAULT_ALARM_CODES[0] >> 8) as u8;
        assert_eq!(
            output[0], alarm_byte,
            "should be alarm code, got {}",
            output[0]
        );
    }

    // ================================================================
    // Phase 11: change_buffers_format
    // ================================================================

    #[test]

    fn test_change_buffers_format_basic() {
        // Create transform with RGB_8, then change to RGB_16
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let mut xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

        // Change to 16-bit output
        assert!(xform.change_buffers_format(TYPE_RGB_8, TYPE_RGB_16).is_ok());

        let input: [u8; 3] = [128, 128, 128];
        let mut output = [0u8; 6]; // now RGB_16
        xform.do_transform(&input, &mut output, 1);

        let r = u16::from_ne_bytes([output[0], output[1]]);
        assert!(r > 0x2000, "R too low after format change: {r:#06X}");
    }

    #[test]

    fn test_change_buffers_format_rejects_float() {
        // Float formats are not supported by change_buffers_format
        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());
        let mut xform = Transform::new(src, TYPE_RGB_8, dst, TYPE_RGB_8, 1, 0).unwrap();

        let result = xform.change_buffers_format(TYPE_RGB_FLT, TYPE_RGB_FLT);
        assert!(result.is_err(), "float format should be rejected");
    }
}
