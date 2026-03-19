// ============================================================================
// Virtual (built-in) profile generation (C版: cmsvirt.c)
// ============================================================================

use crate::context::{CmsError, ErrorCode};
use crate::curves::gamma::ToneCurve;
use crate::curves::intrp::quick_saturate_word;
use crate::curves::wtpnt;
use crate::math::pcs;
use crate::pipeline::lut::{Pipeline, Stage, StageLoc, sample_clut_16bit};
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
    set_text_tags_public(profile, description);
}

/// Public version of set_text_tags for cross-module use.
pub fn set_text_tags_public(profile: &mut Profile, description: &str) {
    let mut desc_mlu = Mlu::new();
    desc_mlu.set_ascii("en", "US", description);
    profile
        .write_tag(TagSignature::ProfileDescription, TagData::Mlu(desc_mlu))
        .expect("ProfileDescription tag write should not fail");

    let mut copy_mlu = Mlu::new();
    copy_mlu.set_ascii("en", "US", "No copyright, use freely");
    profile
        .write_tag(TagSignature::Copyright, TagData::Mlu(copy_mlu))
        .expect("Copyright tag write should not fail");
}

/// Ink-limiting sampler: reduce CMY to stay under total ink limit.
/// C版: `InkLimitingSampler`
fn ink_limiting_sampler(inp: &[u16], out: &mut [u16], ink_limit: f64) {
    let c = inp[0] as f64;
    let m = inp[1] as f64;
    let y = inp[2] as f64;
    let k = inp[3] as f64;

    let sum_cmy = c + m + y;
    let sum_cmyk = sum_cmy + k;

    let ratio = if sum_cmyk > ink_limit && sum_cmy > 0.0 {
        (1.0 - ((sum_cmyk - ink_limit) / sum_cmy)).clamp(0.0, 1.0)
    } else {
        1.0
    };

    out[0] = quick_saturate_word(c * ratio);
    out[1] = quick_saturate_word(m * ratio);
    out[2] = quick_saturate_word(y * ratio);
    out[3] = inp[3]; // K unchanged
}

/// Parameters for BCHSW adjustment.
struct BchswParams {
    bright: f64,
    contrast: f64,
    hue: f64,
    saturation: f64,
    adjust_wp: bool,
    wp_src: CieXyz,
    wp_dest: CieXyz,
}

/// BCHSW sampler: adjust brightness, contrast, hue, saturation, white point.
/// C版: `bchswSampler`
fn bchsw_sampler(inp: &[u16], out: &mut [u16], params: &BchswParams) {
    let encoded = [inp[0], inp[1], inp[2]];
    let lab_in = pcs::pcs_encoded_lab_to_float(&encoded);
    let lch_in = pcs::lab_to_lch(&lab_in);

    let lch_out = CieLCh {
        l: lch_in.l * params.contrast + params.bright,
        c: lch_in.c + params.saturation,
        h: lch_in.h + params.hue,
    };

    let mut lab_out = pcs::lch_to_lab(&lch_out);

    if params.adjust_wp {
        let xyz = pcs::lab_to_xyz(&params.wp_src, &lab_out);
        lab_out = pcs::xyz_to_lab(&params.wp_dest, &xyz);
    }

    let encoded_out = pcs::float_to_pcs_encoded_lab(&lab_out);
    out[0] = encoded_out[0];
    out[1] = encoded_out[1];
    out[2] = encoded_out[2];
}

// ============================================================================
// OkLab pipeline construction
// ============================================================================

// OkLab transformation matrices (from Björn Ottosson).
// Precision matches the C version constants exactly.

// D50→D65 chromatic adaptation
#[rustfmt::skip]
#[allow(clippy::excessive_precision)]
const M_D50_D65: [f64; 9] = [
     0.9554734527042182,  -0.023098536874261423, 0.0632593086610217,
    -0.028369706963208136, 1.0099954580106629,   0.021041398966943008,
     0.012314001688319899, -0.020507696433477912, 1.3303659366080753,
];

// D65→D50 chromatic adaptation
#[rustfmt::skip]
#[allow(clippy::excessive_precision)]
const M_D65_D50: [f64; 9] = [
    1.0479298208405488,   0.022946793341019088, -0.050182534647531644,
    0.029627815688159344, 0.990434484573249,    -0.01707382502938514,
   -0.009243058152591178, 0.015055144896577895,  0.7518742899580008,
];

// XYZ/D65 → LMS
#[rustfmt::skip]
const M_D65_LMS: [f64; 9] = [
    0.8189330101, 0.3618667424, -0.1288597137,
    0.0329845436, 0.9293118715,  0.0361456387,
    0.0482003018, 0.2643662691,  0.6338517070,
];

// LMS → XYZ/D65
#[rustfmt::skip]
#[allow(clippy::excessive_precision)]
const M_LMS_D65: [f64; 9] = [
     1.227013851103521,  -0.557799980651822,   0.281256148966468,
    -0.040580178423281,   1.112256869616830,  -0.071676678665601,
    -0.076381284505707,  -0.421481978418013,   1.586163220440795,
];

// LMS' → OkLab
#[rustfmt::skip]
const M_LMSPRIME_OKLAB: [f64; 9] = [
    0.2104542553,  0.7936177850, -0.0040720468,
    1.9779984951, -2.4285922050,  0.4505937099,
    0.0259040371,  0.7827717662, -0.8086757660,
];

// OkLab → LMS'
#[rustfmt::skip]
#[allow(clippy::excessive_precision)]
const M_OKLAB_LMSPRIME: [f64; 9] = [
    0.999999998450520,  0.396337792173768,  0.215803758060759,
    1.000000008881761, -0.105561342323656, -0.063854174771706,
    1.000000054672411, -0.089484182094966, -1.291485537864092,
];

/// Build BToA0 pipeline: PCS XYZ → OkLab
///
/// Note: OkLab a/b channels are signed (approximately [-0.5, 0.5]).
/// This pipeline is designed for float evaluation; 16-bit mode will
/// clamp negative a/b values. The cube root uses gamma curves that
/// clamp negative inputs, matching the C version's behavior — LMS
/// values are always positive for physically realizable colors.
fn build_oklab_btoa() -> Option<Pipeline> {
    let mut lut = Pipeline::new(3, 3)?;

    // 1. PCS XYZ → real XYZ (scale up by 65535/32768)
    lut.insert_stage(StageLoc::AtEnd, Stage::new_normalize_to_xyz_float()?);

    // 2. D50→D65 chromatic adaptation
    lut.insert_stage(StageLoc::AtEnd, Stage::new_matrix(3, 3, &M_D50_D65, None)?);

    // 3. D65→LMS
    lut.insert_stage(StageLoc::AtEnd, Stage::new_matrix(3, 3, &M_D65_LMS, None)?);

    // 4. Cube root (LMS → LMS')
    let cube_root = ToneCurve::build_gamma(1.0 / 3.0)?;
    let curves = [cube_root.clone(), cube_root.clone(), cube_root];
    lut.insert_stage(StageLoc::AtEnd, Stage::new_tone_curves(Some(&curves), 3)?);

    // 5. LMS' → OkLab
    lut.insert_stage(
        StageLoc::AtEnd,
        Stage::new_matrix(3, 3, &M_LMSPRIME_OKLAB, None)?,
    );

    Some(lut)
}

/// Build AToB0 pipeline: OkLab → PCS XYZ
///
/// See `build_oklab_btoa` for notes on signed channels and float usage.
fn build_oklab_atob() -> Option<Pipeline> {
    let mut lut = Pipeline::new(3, 3)?;

    // 1. OkLab → LMS'
    lut.insert_stage(
        StageLoc::AtEnd,
        Stage::new_matrix(3, 3, &M_OKLAB_LMSPRIME, None)?,
    );

    // 2. Cube (LMS' → LMS)
    let cube = ToneCurve::build_gamma(3.0)?;
    let curves = [cube.clone(), cube.clone(), cube];
    lut.insert_stage(StageLoc::AtEnd, Stage::new_tone_curves(Some(&curves), 3)?);

    // 3. LMS → D65
    lut.insert_stage(StageLoc::AtEnd, Stage::new_matrix(3, 3, &M_LMS_D65, None)?);

    // 4. D65→D50 chromatic adaptation
    lut.insert_stage(StageLoc::AtEnd, Stage::new_matrix(3, 3, &M_D65_D50, None)?);

    // 5. Real XYZ → PCS XYZ (scale down by 32768/65535)
    lut.insert_stage(StageLoc::AtEnd, Stage::new_normalize_from_xyz_float()?);

    Some(lut)
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

    /// Create a linearization device link profile.
    /// C版: `cmsCreateLinearizationDeviceLinkTHR`
    pub fn new_linearization_device_link(
        color_space: ColorSpaceSignature,
        transfer_functions: &[ToneCurve],
    ) -> Result<Self, CmsError> {
        let n_channels = color_space.channels();
        if transfer_functions.len() < n_channels as usize {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: format!(
                    "need {} curves for {:?}, got {}",
                    n_channels,
                    color_space,
                    transfer_functions.len()
                ),
            });
        }

        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::Link;
        p.header.color_space = color_space;
        p.header.pcs = color_space;
        p.header.rendering_intent = 0;

        set_text_tags(&mut p, "Linearization built-in");

        if let Some(mut lut) = Pipeline::new(n_channels, n_channels) {
            if let Some(stage) = Stage::new_tone_curves(Some(transfer_functions), n_channels) {
                lut.insert_stage(StageLoc::AtBegin, stage);
            }
            let _ = p.write_tag(TagSignature::AToB0, TagData::Pipeline(lut));
        }

        Ok(p)
    }

    /// Create an ink-limiting device link profile (CMYK only).
    /// C版: `cmsCreateInkLimitingDeviceLinkTHR`
    pub fn new_ink_limiting_device_link(
        color_space: ColorSpaceSignature,
        limit: f64,
    ) -> Result<Self, CmsError> {
        if color_space != ColorSpaceSignature::CmykData {
            return Err(CmsError {
                code: ErrorCode::ColorspaceCheck,
                message: "ink limiting only supports CMYK".into(),
            });
        }

        let limit = limit.clamp(1.0, 400.0);
        let n_channels = 4u32;

        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::Link;
        p.header.color_space = color_space;
        p.header.pcs = color_space;
        p.header.rendering_intent = 0;

        set_text_tags(&mut p, "ink-limiting built-in");

        if let Some(mut lut) = Pipeline::new(n_channels, n_channels) {
            // Pre-linearization: identity curves
            if let Some(stage) = Stage::new_identity_curves(n_channels) {
                lut.insert_stage(StageLoc::AtEnd, stage);
            }

            // CLUT with ink-limiting sampler
            if let Some(mut clut) = Stage::new_clut_16bit_uniform(17, n_channels, n_channels, None)
            {
                let ink_limit = limit * 655.35;
                sample_clut_16bit(
                    &mut clut,
                    |inp, out, _| {
                        ink_limiting_sampler(inp, out, ink_limit);
                        true
                    },
                    0,
                );
                lut.insert_stage(StageLoc::AtEnd, clut);
            }

            // Post-linearization: identity curves
            if let Some(stage) = Stage::new_identity_curves(n_channels) {
                lut.insert_stage(StageLoc::AtEnd, stage);
            }

            let _ = p.write_tag(TagSignature::AToB0, TagData::Pipeline(lut));
        }

        Ok(p)
    }

    /// Create a BCHSW (Brightness/Contrast/Hue/Saturation/WhitePoint) abstract profile.
    /// C版: `cmsCreateBCHSWabstractProfileTHR`
    pub fn new_bchsw_abstract(
        n_lut_points: u32,
        bright: f64,
        contrast: f64,
        hue: f64,
        saturation: f64,
        temp_src: u32,
        temp_dest: u32,
    ) -> Self {
        let n_lut_points = n_lut_points.clamp(2, 256);

        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::Abstract;
        p.header.color_space = ColorSpaceSignature::LabData;
        p.header.pcs = ColorSpaceSignature::LabData;
        p.header.rendering_intent = 0;

        set_text_tags(&mut p, "BCHSW built-in");

        let d50 = wtpnt::d50_xyz();
        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(d50));

        // Determine white point adjustment
        let adjust_wp = temp_src != temp_dest;
        let wp_src = if adjust_wp {
            let xy = wtpnt::white_point_from_temp(temp_src as f64).unwrap_or(wtpnt::d50_xyy());
            pcs::xyy_to_xyz(&xy)
        } else {
            d50
        };
        let wp_dest = if adjust_wp {
            let xy = wtpnt::white_point_from_temp(temp_dest as f64).unwrap_or(wtpnt::d50_xyy());
            pcs::xyy_to_xyz(&xy)
        } else {
            d50
        };

        let params = BchswParams {
            bright,
            contrast,
            hue,
            saturation,
            adjust_wp,
            wp_src,
            wp_dest,
        };

        if let Some(mut lut) = Pipeline::new(3, 3) {
            let dims = [n_lut_points, n_lut_points, n_lut_points];
            if let Some(mut clut) = Stage::new_clut_16bit(&dims, 3, 3, None) {
                sample_clut_16bit(
                    &mut clut,
                    |inp, out, _| {
                        bchsw_sampler(inp, out, &params);
                        true
                    },
                    0,
                );
                lut.insert_stage(StageLoc::AtEnd, clut);
            }
            let _ = p.write_tag(TagSignature::AToB0, TagData::Pipeline(lut));
        }

        p
    }

    /// Create an OkLab color space profile.
    /// C版: `cmsCreate_OkLabProfile`
    pub fn new_oklab() -> Self {
        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.4);
        p.header.device_class = ProfileClassSignature::ColorSpace;
        p.header.color_space = ColorSpaceSignature::Color3;
        p.header.pcs = ColorSpaceSignature::XyzData;
        p.header.rendering_intent = 1; // Relative colorimetric

        set_text_tags(&mut p, "OkLab built-in");

        let d50 = wtpnt::d50_xyz();
        let _ = p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(d50));

        // Build BToA0: XYZ/D50 → OkLab
        if let Some(lut) = build_oklab_btoa() {
            let _ = p.write_tag(TagSignature::BToA0, TagData::Pipeline(lut));
        }

        // Build AToB0: OkLab → XYZ/D50
        if let Some(lut) = build_oklab_atob() {
            let _ = p.write_tag(TagSignature::AToB0, TagData::Pipeline(lut));
        }

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
    // Profile::new_linearization_device_link
    // ================================================================

    #[test]
    fn linearization_device_link_rgb_header() {
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let curves = [gamma.clone(), gamma.clone(), gamma];
        let mut p =
            Profile::new_linearization_device_link(ColorSpaceSignature::RgbData, &curves).unwrap();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Link);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::RgbData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::RgbData);
    }

    #[test]
    fn linearization_device_link_has_atob0() {
        let gamma = ToneCurve::build_gamma(1.8).unwrap();
        let curves = [gamma.clone(), gamma.clone(), gamma.clone(), gamma];
        let mut p =
            Profile::new_linearization_device_link(ColorSpaceSignature::CmykData, &curves).unwrap();
        let mut p2 = roundtrip(&mut p);
        assert!(p2.read_tag(TagSignature::AToB0).is_ok());
        assert_eq!(p2.header.color_space, ColorSpaceSignature::CmykData);
    }

    // ================================================================
    // Profile::new_ink_limiting_device_link
    // ================================================================

    #[test]
    fn ink_limiting_header() {
        let p = Profile::new_ink_limiting_device_link(ColorSpaceSignature::CmykData, 200.0);
        assert!(p.is_ok());
        let mut p = p.unwrap();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Link);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::CmykData);
    }

    #[test]
    fn ink_limiting_rejects_non_cmyk() {
        let p = Profile::new_ink_limiting_device_link(ColorSpaceSignature::RgbData, 200.0);
        assert!(p.is_err());
    }

    #[test]
    fn ink_limiting_transform_respects_limit() {
        // Create ink-limiting device link with 200% limit
        let mut link =
            Profile::new_ink_limiting_device_link(ColorSpaceSignature::CmykData, 200.0).unwrap();
        let link = roundtrip(&mut link);

        // Create a pass-through CMYK profile pair using the device link
        let src_cmyk = roundtrip(
            &mut Profile::new_ink_limiting_device_link(
                ColorSpaceSignature::CmykData,
                400.0, // no effective limit
            )
            .unwrap(),
        );

        let xform = Transform::new(src_cmyk, TYPE_CMYK_16, link, TYPE_CMYK_16, 0, 0).unwrap();

        // Input: C=100%, M=100%, Y=100%, K=100% = 400% total
        let input: [u8; 8] = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let mut output = [0u8; 8];
        xform.do_transform(&input, &mut output, 1);

        // Output CMY should be reduced, K unchanged
        let c = u16::from_ne_bytes([output[0], output[1]]) as f64 / 655.35;
        let m = u16::from_ne_bytes([output[2], output[3]]) as f64 / 655.35;
        let y = u16::from_ne_bytes([output[4], output[5]]) as f64 / 655.35;
        let k = u16::from_ne_bytes([output[6], output[7]]) as f64 / 655.35;
        let total = c + m + y + k;
        assert!(
            total <= 210.0,
            "total ink should be ≤ ~200%, got {total:.1}%"
        );
    }

    // ================================================================
    // Profile::new_bchsw_abstract
    // ================================================================

    #[test]
    fn bchsw_header() {
        let mut p = Profile::new_bchsw_abstract(17, 10.0, 1.0, 0.0, 0.0, 6500, 6500);
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::Abstract);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::LabData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::LabData);
    }

    #[test]
    fn bchsw_brightness_increases_l() {
        // Brightness +20 should increase L* of a mid-gray
        let mut bchsw = Profile::new_bchsw_abstract(17, 20.0, 1.0, 0.0, 0.0, 6500, 6500);
        let bchsw = roundtrip(&mut bchsw);

        let src = roundtrip(&mut Profile::new_lab4(None));

        // Use Lab identity profile → BCHSW abstract profile
        let xform = Transform::new(src, TYPE_LAB_FLT, bchsw, TYPE_LAB_FLT, 0, 0).unwrap();

        // Input: L*=50, a*=0, b*=0
        let input: [f32; 3] = [50.0, 0.0, 0.0];
        let input_bytes = floats_to_bytes(&input);
        let mut output_buf = [0u8; 12];
        xform.do_transform(&input_bytes, &mut output_buf, 1);
        let output = bytes_to_floats(&output_buf);

        assert!(
            output[0] > 60.0,
            "L* should increase with brightness, got {}",
            output[0]
        );
    }

    #[test]
    fn bchsw_hue_rotation() {
        // Hue +180 should rotate hue
        let mut bchsw = Profile::new_bchsw_abstract(17, 0.0, 1.0, 180.0, 0.0, 6500, 6500);
        let bchsw = roundtrip(&mut bchsw);

        let src = roundtrip(&mut Profile::new_lab4(None));

        let xform = Transform::new(src, TYPE_LAB_FLT, bchsw, TYPE_LAB_FLT, 0, 0).unwrap();

        // Input: a red-ish color: L*=50, a*=60, b*=20
        let input: [f32; 3] = [50.0, 60.0, 20.0];
        let input_bytes = floats_to_bytes(&input);
        let mut output_buf = [0u8; 12];
        xform.do_transform(&input_bytes, &mut output_buf, 1);
        let output = bytes_to_floats(&output_buf);

        // Hue rotated 180° should invert a* and b* signs (approximately)
        assert!(
            output[1] < -30.0,
            "a* should be negative after 180° rotation, got {}",
            output[1]
        );
    }

    // ================================================================
    // Profile::new_oklab
    // ================================================================

    #[test]
    fn oklab_header() {
        let mut p = Profile::new_oklab();
        let p2 = roundtrip(&mut p);
        assert_eq!(p2.header.device_class, ProfileClassSignature::ColorSpace);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::Color3);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::XyzData);
    }

    #[test]
    fn oklab_has_atob0_and_btoa0() {
        let mut p = Profile::new_oklab();
        let mut p2 = roundtrip(&mut p);
        assert!(
            p2.read_tag(TagSignature::AToB0).is_ok(),
            "OkLab should have AToB0"
        );
        assert!(
            p2.read_tag(TagSignature::BToA0).is_ok(),
            "OkLab should have BToA0"
        );
    }

    #[test]
    fn oklab_pipeline_roundtrip() {
        // Test BToA (XYZ→OkLab) followed by AToB (OkLab→XYZ) at pipeline level.
        // Use D50 white point XYZ = (0.9505, 1.0, 1.089) in PCS encoding.
        let btoa = super::build_oklab_btoa().unwrap();
        let atob = super::build_oklab_atob().unwrap();

        // D50 white in PCS float: XYZ × (32768/65535)
        let pcs_scale = 32768.0 / 65535.0;
        let xyz_in: [f32; 3] = [
            (0.9505 * pcs_scale) as f32,
            (1.0 * pcs_scale) as f32,
            (1.089 * pcs_scale) as f32,
        ];

        // XYZ → OkLab
        let mut oklab = [0.0f32; 3];
        btoa.eval_float(&xyz_in, &mut oklab);

        // OkLab L should be ~1.0 for white, a≈0, b≈0
        assert!(
            (oklab[0] - 1.0).abs() < 0.05,
            "OkLab L should be ~1.0 for D50 white, got {}",
            oklab[0]
        );

        // OkLab → XYZ
        let mut xyz_out = [0.0f32; 3];
        atob.eval_float(&oklab, &mut xyz_out);

        // Round-trip should recover input within tolerance
        for i in 0..3 {
            assert!(
                (xyz_out[i] - xyz_in[i]).abs() < 0.01,
                "XYZ round-trip channel {i}: in={}, out={}",
                xyz_in[i],
                xyz_out[i]
            );
        }
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
