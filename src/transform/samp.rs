// ============================================================================
// Black point detection (C版: cmssamp.c)
// ============================================================================

use crate::math::pcs;
use crate::pipeline::pack::pixel_size;
use crate::profile::io::Profile;
use crate::types::*;

use super::xform::{FLAGS_NOCACHE, FLAGS_NOOPTIMIZE};

// ============================================================================
// Constants
// ============================================================================

/// ICC v4 perceptual black point (D50-relative XYZ).
pub const PERCEPTUAL_BLACK_X: f64 = 0.00336;
pub const PERCEPTUAL_BLACK_Y: f64 = 0.0034731;
pub const PERCEPTUAL_BLACK_Z: f64 = 0.00287;

/// Perceptual black as CieXyz.
pub const PERCEPTUAL_BLACK: CieXyz = CieXyz {
    x: PERCEPTUAL_BLACK_X,
    y: PERCEPTUAL_BLACK_Y,
    z: PERCEPTUAL_BLACK_Z,
};

const D50_WHITE: CieXyz = CieXyz {
    x: D50_X,
    y: D50_Y,
    z: D50_Z,
};

// ============================================================================
// Helper: ink colorspace check
// ============================================================================

/// Check if a colorspace is ink-based (CMYK, CMY, MCHn, n-color).
/// C版: `isInkColorspace`
pub fn is_ink_colorspace(cs: ColorSpaceSignature) -> bool {
    matches!(
        cs,
        ColorSpaceSignature::CmykData
            | ColorSpaceSignature::CmyData
            | ColorSpaceSignature::Mch1Data
            | ColorSpaceSignature::Mch2Data
            | ColorSpaceSignature::Mch3Data
            | ColorSpaceSignature::Mch4Data
            | ColorSpaceSignature::Mch5Data
            | ColorSpaceSignature::Mch6Data
            | ColorSpaceSignature::Mch7Data
            | ColorSpaceSignature::Mch8Data
            | ColorSpaceSignature::Mch9Data
            | ColorSpaceSignature::MchAData
            | ColorSpaceSignature::MchBData
            | ColorSpaceSignature::MchCData
            | ColorSpaceSignature::MchDData
            | ColorSpaceSignature::MchEData
            | ColorSpaceSignature::MchFData
            | ColorSpaceSignature::Color1
            | ColorSpaceSignature::Color2
            | ColorSpaceSignature::Color3
            | ColorSpaceSignature::Color4
            | ColorSpaceSignature::Color5
            | ColorSpaceSignature::Color6
            | ColorSpaceSignature::Color7
            | ColorSpaceSignature::Color8
            | ColorSpaceSignature::Color9
            | ColorSpaceSignature::Color10
            | ColorSpaceSignature::Color11
            | ColorSpaceSignature::Color12
            | ColorSpaceSignature::Color13
            | ColorSpaceSignature::Color14
            | ColorSpaceSignature::Color15
    )
}

// ============================================================================
// Helper: build PixelFormat for a profile's colorspace
// ============================================================================

/// Build a PixelFormat for the given profile's device colorspace.
/// C版: `cmsFormatterForColorspaceOfProfile`
fn formatter_for_colorspace(profile: &Profile, n_bytes: u32) -> Option<PixelFormat> {
    let cs = profile.header.color_space;
    let cs_bits = cs.to_pixel_type();
    let n_chan = cs.channels();
    if n_chan == 0 {
        return None;
    }
    Some(PixelFormat::build(cs_bits, n_chan, n_bytes & 7))
}

// ============================================================================
// Internal detection methods
// ============================================================================

/// Detect black point by transforming the darkest colorant to Lab.
/// C版: `BlackPointAsDarkerColorant`
fn black_point_as_darker_colorant(profile: &mut Profile, intent: u32) -> Option<CieXyz> {
    let cs = profile.header.color_space;
    let n_chan = cs.channels();

    let (_, black, ep_n) = pcs::endpoints_by_space(cs)?;
    if ep_n != n_chan {
        return None;
    }

    // Build format for the profile's device space (16-bit)
    let device_fmt = formatter_for_colorspace(profile, 2)?;
    let device_stride = pixel_size(device_fmt);

    // Create Lab v2 output profile for transform
    let mut lab_profile = {
        let mut p = Profile::new_lab2(None);
        let data = p.save_to_mem().ok()?;
        Profile::open_mem(&data).ok()?
    };

    // Create transform: device → Lab16
    let xform = super::xform::Transform::new(
        std::mem::replace(profile, Profile::new_placeholder()),
        device_fmt,
        std::mem::replace(&mut lab_profile, Profile::new_placeholder()),
        TYPE_LAB_16,
        intent,
        FLAGS_NOCACHE | FLAGS_NOOPTIMIZE,
    );

    // Restore profile (swap back)
    // We can't do this cleanly with owned profiles, so rebuild from the transform result
    let xform = match xform {
        Ok(x) => x,
        Err(_) => return None,
    };

    // Prepare input: darkest colorant (big-endian 16-bit)
    let mut input = vec![0u8; device_stride];
    for (i, &val) in black[..n_chan as usize].iter().enumerate() {
        let offset = i * 2;
        input[offset] = (val >> 8) as u8;
        input[offset + 1] = (val & 0xFF) as u8;
    }

    let mut output = [0u8; 6]; // Lab16 = 3 channels × 2 bytes
    xform.do_transform(&input, &mut output, 1);

    // Decode Lab16
    let lab_encoded = [
        u16::from_ne_bytes([output[0], output[1]]),
        u16::from_ne_bytes([output[2], output[3]]),
        u16::from_ne_bytes([output[4], output[5]]),
    ];
    let lab = pcs::pcs_encoded_lab_to_float(&lab_encoded);

    // Clip L* to [0, 50], force a=b=0
    let lab_clipped = CieLab {
        l: lab.l.clamp(0.0, 50.0),
        a: 0.0,
        b: 0.0,
    };

    Some(pcs::lab_to_xyz(&D50_WHITE, &lab_clipped))
}

/// Detect black point by perceptual roundtrip of black.
/// C版: `BlackPointUsingPerceptualBlack`
fn black_point_using_perceptual(profile: &mut Profile) -> Option<CieXyz> {
    // Create roundtrip: Lab → profile → Lab (perceptual intent)
    let mut lab_in = {
        let mut p = Profile::new_lab4(None);
        let data = p.save_to_mem().ok()?;
        Profile::open_mem(&data).ok()?
    };
    let mut lab_out = {
        let mut p = Profile::new_lab4(None);
        let data = p.save_to_mem().ok()?;
        Profile::open_mem(&data).ok()?
    };

    // 4-profile roundtrip: Lab → [rel col] → profile → [perceptual] → profile → [rel col] → Lab
    // Simplified: Lab → profile → Lab with perceptual intent
    let xform = super::xform::Transform::new_multiprofile(
        &mut [
            std::mem::replace(&mut lab_in, Profile::new_placeholder()),
            std::mem::replace(profile, Profile::new_placeholder()),
            std::mem::replace(&mut lab_out, Profile::new_placeholder()),
        ],
        TYPE_LAB_16,
        TYPE_LAB_16,
        0, // perceptual
        FLAGS_NOCACHE | FLAGS_NOOPTIMIZE,
    )
    .ok()?;

    // Transform black Lab (L=0, a=0, b=0)
    let black_lab = pcs::float_to_pcs_encoded_lab(&CieLab {
        l: 0.0,
        a: 0.0,
        b: 0.0,
    });
    let input = [
        (black_lab[0] >> 8) as u8,
        (black_lab[0] & 0xFF) as u8,
        (black_lab[1] >> 8) as u8,
        (black_lab[1] & 0xFF) as u8,
        (black_lab[2] >> 8) as u8,
        (black_lab[2] & 0xFF) as u8,
    ];
    let mut output = [0u8; 6];
    xform.do_transform(&input, &mut output, 1);

    let result_encoded = [
        u16::from_ne_bytes([output[0], output[1]]),
        u16::from_ne_bytes([output[2], output[3]]),
        u16::from_ne_bytes([output[4], output[5]]),
    ];
    let result_lab = pcs::pcs_encoded_lab_to_float(&result_encoded);

    let lab_clipped = CieLab {
        l: result_lab.l.clamp(0.0, 50.0),
        a: 0.0,
        b: 0.0,
    };

    Some(pcs::lab_to_xyz(&D50_WHITE, &lab_clipped))
}

// ============================================================================
// Public API
// ============================================================================

/// Detect black point of an input profile.
/// C版: `cmsDetectBlackPoint`
pub fn detect_black_point(profile: &mut Profile, intent: u32) -> Option<CieXyz> {
    let class = profile.header.device_class;

    // Reject unsupported profile classes
    if matches!(
        class,
        ProfileClassSignature::Link
            | ProfileClassSignature::Abstract
            | ProfileClassSignature::NamedColor
    ) {
        return None;
    }

    // Absolute colorimetric: not supported for BPC
    if intent == 3 {
        return None;
    }

    let is_v4 = profile.header.version >= 0x04000000;

    // V4 perceptual/saturation
    if is_v4 && (intent == 0 || intent == 2) {
        if profile.is_matrix_shaper() {
            return black_point_as_darker_colorant(profile, intent);
        }
        return Some(PERCEPTUAL_BLACK);
    }

    // Output class: use perceptual roundtrip
    if class == ProfileClassSignature::Output {
        return black_point_using_perceptual(profile);
    }

    // Default: darker colorant method
    black_point_as_darker_colorant(profile, intent)
}

/// Detect black point of an output/destination profile (Adobe algorithm).
/// C版: `cmsDetectDestinationBlackPoint`
///
/// For most cases, delegates to `detect_black_point`.
/// The full Adobe algorithm (L* ramp + quadratic fitting) is deferred.
pub fn detect_dest_black_point(profile: &mut Profile, intent: u32) -> Option<CieXyz> {
    // For now, delegate to the simpler detection method.
    // The full Adobe algorithm with L* ramp analysis and quadratic curve fitting
    // will be implemented when needed for high-accuracy BPC on LUT-based profiles.
    detect_black_point(profile, intent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(p: &mut Profile) -> Profile {
        let data = p.save_to_mem().unwrap();
        Profile::open_mem(&data).unwrap()
    }

    // ================================================================
    // is_ink_colorspace
    // ================================================================

    #[test]
    fn ink_cmyk() {
        assert!(is_ink_colorspace(ColorSpaceSignature::CmykData));
    }

    #[test]
    fn ink_cmy() {
        assert!(is_ink_colorspace(ColorSpaceSignature::CmyData));
    }

    #[test]
    fn ink_mch5() {
        assert!(is_ink_colorspace(ColorSpaceSignature::Mch5Data));
    }

    #[test]
    fn not_ink_rgb() {
        assert!(!is_ink_colorspace(ColorSpaceSignature::RgbData));
    }

    #[test]
    fn not_ink_lab() {
        assert!(!is_ink_colorspace(ColorSpaceSignature::LabData));
    }

    #[test]
    fn not_ink_gray() {
        assert!(!is_ink_colorspace(ColorSpaceSignature::GrayData));
    }

    // ================================================================
    // detect_black_point
    // ================================================================

    #[test]
    fn detect_bp_srgb_near_zero() {
        // sRGB black point should be very close to (0, 0, 0)
        let mut p = roundtrip(&mut Profile::new_srgb());
        let bp = detect_black_point(&mut p, 0).expect("should detect");
        assert!(bp.x.abs() < 0.01, "bp.x = {}", bp.x);
        assert!(bp.y.abs() < 0.01, "bp.y = {}", bp.y);
        assert!(bp.z.abs() < 0.01, "bp.z = {}", bp.z);
    }

    #[test]
    fn detect_bp_srgb_relative_colorimetric() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let bp = detect_black_point(&mut p, 1); // relative colorimetric
        assert!(bp.is_some());
    }

    // ================================================================
    // detect_dest_black_point
    // ================================================================

    #[test]
    fn detect_dest_bp_srgb() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let bp = detect_dest_black_point(&mut p, 0);
        assert!(bp.is_some());
        if let Some(bp) = bp {
            // L* should be in [0, 5] range for sRGB black
            let lab = pcs::xyz_to_lab(&D50_WHITE, &bp);
            assert!(
                lab.l >= 0.0 && lab.l <= 5.0,
                "L* = {} (expected 0..5)",
                lab.l
            );
        }
    }

    // ================================================================
    // endpoints_by_space (helper in pcs.rs)
    // ================================================================

    #[test]
    fn endpoints_rgb() {
        let (white, black, n) = pcs::endpoints_by_space(ColorSpaceSignature::RgbData).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&white[..3], &[0xFFFF, 0xFFFF, 0xFFFF]);
        assert_eq!(&black[..3], &[0, 0, 0]);
    }

    #[test]
    fn endpoints_cmyk() {
        let (white, black, n) = pcs::endpoints_by_space(ColorSpaceSignature::CmykData).unwrap();
        assert_eq!(n, 4);
        assert_eq!(&white[..4], &[0, 0, 0, 0]); // no ink = white
        assert_eq!(&black[..4], &[0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]); // 400% ink
    }

    #[test]
    fn endpoints_lab() {
        let (white, black, n) = pcs::endpoints_by_space(ColorSpaceSignature::LabData).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&white[..3], &[0xFFFF, 0x8080, 0x8080]);
        assert_eq!(&black[..3], &[0, 0x8080, 0x8080]);
    }

    #[test]
    fn endpoints_unsupported() {
        assert!(pcs::endpoints_by_space(ColorSpaceSignature::NamedData).is_none());
    }
}
