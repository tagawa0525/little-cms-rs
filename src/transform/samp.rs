// ============================================================================
// Black point detection (C版: cmssamp.c)
// ============================================================================

use crate::types::{CieXyz, ColorSpaceSignature};

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

// ============================================================================
// Public API stubs
// ============================================================================

/// Check if a colorspace is ink-based (CMYK, CMY, MCHn, n-color).
/// C版: `isInkColorspace`
pub fn is_ink_colorspace(_cs: ColorSpaceSignature) -> bool {
    todo!("Phase 5c GREEN")
}

/// Detect black point of an input profile.
/// C版: `cmsDetectBlackPoint`
pub fn detect_black_point(
    _profile: &mut crate::profile::io::Profile,
    _intent: u32,
) -> Option<CieXyz> {
    todo!("Phase 5c GREEN")
}

/// Detect black point of an output/destination profile (Adobe algorithm).
/// C版: `cmsDetectDestinationBlackPoint`
pub fn detect_dest_black_point(
    _profile: &mut crate::profile::io::Profile,
    _intent: u32,
) -> Option<CieXyz> {
    todo!("Phase 5c GREEN")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::pcs;
    use crate::profile::io::Profile;
    use crate::types::*;

    fn roundtrip(p: &mut Profile) -> Profile {
        let data = p.save_to_mem().unwrap();
        Profile::open_mem(&data).unwrap()
    }

    // ================================================================
    // is_ink_colorspace
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn ink_cmyk() {
        assert!(is_ink_colorspace(ColorSpaceSignature::CmykData));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn ink_cmy() {
        assert!(is_ink_colorspace(ColorSpaceSignature::CmyData));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn ink_mch5() {
        assert!(is_ink_colorspace(ColorSpaceSignature::Mch5Data));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn not_ink_rgb() {
        assert!(!is_ink_colorspace(ColorSpaceSignature::RgbData));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn not_ink_lab() {
        assert!(!is_ink_colorspace(ColorSpaceSignature::LabData));
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn not_ink_gray() {
        assert!(!is_ink_colorspace(ColorSpaceSignature::GrayData));
    }

    // ================================================================
    // detect_black_point
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn detect_bp_srgb_near_zero() {
        // sRGB black point should be very close to (0, 0, 0)
        let mut p = roundtrip(&mut Profile::new_srgb());
        let bp = detect_black_point(&mut p, 0).expect("should detect");
        assert!(bp.x.abs() < 0.01, "bp.x = {}", bp.x);
        assert!(bp.y.abs() < 0.01, "bp.y = {}", bp.y);
        assert!(bp.z.abs() < 0.01, "bp.z = {}", bp.z);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn detect_bp_srgb_relative_colorimetric() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let bp = detect_black_point(&mut p, 1); // relative colorimetric
        assert!(bp.is_some());
    }

    // ================================================================
    // detect_dest_black_point
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn detect_dest_bp_srgb() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let bp = detect_dest_black_point(&mut p, 0);
        assert!(bp.is_some());
        if let Some(bp) = bp {
            // L* should be in [0, 5] range for sRGB black
            let white = CieXyz {
                x: D50_X,
                y: D50_Y,
                z: D50_Z,
            };
            let lab = pcs::xyz_to_lab(&white, &bp);
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
    #[ignore = "not yet implemented"]
    fn endpoints_rgb() {
        let (white, black, n) = pcs::endpoints_by_space(ColorSpaceSignature::RgbData).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&white[..3], &[0xFFFF, 0xFFFF, 0xFFFF]);
        assert_eq!(&black[..3], &[0, 0, 0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn endpoints_cmyk() {
        let (white, black, n) = pcs::endpoints_by_space(ColorSpaceSignature::CmykData).unwrap();
        assert_eq!(n, 4);
        assert_eq!(&white[..4], &[0, 0, 0, 0]); // no ink = white
        assert_eq!(&black[..4], &[0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]); // 400% ink
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn endpoints_lab() {
        let (white, black, n) = pcs::endpoints_by_space(ColorSpaceSignature::LabData).unwrap();
        assert_eq!(n, 3);
        assert_eq!(&white[..3], &[0xFFFF, 0x8080, 0x8080]);
        assert_eq!(&black[..3], &[0, 0x8080, 0x8080]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn endpoints_unsupported() {
        assert!(pcs::endpoints_by_space(ColorSpaceSignature::NamedData).is_none());
    }
}
