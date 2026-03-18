// ============================================================================
// Gamut mapping utilities (C版: cmsgmt.c)
// ============================================================================

use crate::curves::gamma::ToneCurve;
use crate::math::pcs;
use crate::pipeline::lut::slice_space_16;
use crate::profile::io::Profile;
use crate::types::*;

use super::xform::{FLAGS_NOCACHE, FLAGS_NOOPTIMIZE};

// ============================================================================
// Public API
// ============================================================================

/// Clamp a CIELab color to a gamut prism defined by a*/b* limits.
///
/// Returns `false` if L* < 0 (sets Lab to zero). Otherwise clips L* to
/// [0, 100] and clamps a*, b* to the given rectangular bounds using
/// hue-preserving clipping.
///
/// C版: `cmsDesaturateLab`
pub fn desaturate_lab(lab: &mut CieLab, amax: f64, amin: f64, bmax: f64, bmin: f64) -> bool {
    // Whole luma surface to zero
    if lab.l < 0.0 {
        lab.l = 0.0;
        lab.a = 0.0;
        lab.b = 0.0;
        return false;
    }

    // Clamp white (discard highlights)
    if lab.l > 100.0 {
        lab.l = 100.0;
    }

    // Check gamut prism on a, b faces
    if lab.a >= amin && lab.a <= amax && lab.b >= bmin && lab.b <= bmax {
        return true;
    }

    // Falls outside — transport to LCh and clip by hue zone
    if lab.a == 0.0 {
        // Hue is exactly 90° — atan won't work
        lab.b = if lab.b < 0.0 { bmin } else { bmax };
        return true;
    }

    let lch = pcs::lab_to_lch(lab);
    let slope = lab.b / lab.a;
    let h = lch.h;

    if (0.0..45.0).contains(&h) || (315.0..=360.0).contains(&h) {
        // Clip by amax
        lab.a = amax;
        lab.b = amax * slope;
    } else if (45.0..135.0).contains(&h) {
        // Clip by bmax
        lab.b = bmax;
        lab.a = bmax / slope;
    } else if (135.0..225.0).contains(&h) {
        // Clip by amin
        lab.a = amin;
        lab.b = amin * slope;
    } else if (225.0..315.0).contains(&h) {
        // Clip by bmin
        lab.b = bmin;
        lab.a = bmin / slope;
    } else {
        return false;
    }

    true
}

/// Detect Total Area Coverage (TAC) of an output profile.
///
/// Samples Lab space through the profile and returns the maximum
/// sum of ink channels as a percentage. Returns 0.0 for non-output profiles
/// or on error.
///
/// C版: `cmsDetectTAC`
pub fn detect_tac(profile: &mut Profile) -> f64 {
    // TAC only works on output profiles
    if profile.header.device_class != ProfileClassSignature::Output {
        return 0.0;
    }

    let cs = profile.header.color_space;
    let n_channels = cs.channels();
    if n_channels == 0 || n_channels as usize >= MAX_CHANNELS {
        return 0.0;
    }

    // Build a float formatter for the profile's output space (4 bytes = float)
    let cs_bits = cs.to_pixel_type();
    let output_fmt = PixelFormat::build(cs_bits, n_channels, 4).with_float();

    // Create Lab → Profile transform (perceptual intent)
    let lab_profile = {
        let mut p = Profile::new_lab4(None);
        match p.save_to_mem() {
            Ok(data) => match Profile::open_mem(&data) {
                Ok(p) => p,
                Err(_) => return 0.0,
            },
            Err(_) => return 0.0,
        }
    };
    let profile_copy = match profile.save_to_mem() {
        Ok(data) => match Profile::open_mem(&data) {
            Ok(p) => p,
            Err(_) => return 0.0,
        },
        Err(_) => return 0.0,
    };

    let xform = match super::xform::Transform::new(
        lab_profile,
        TYPE_LAB_16,
        profile_copy,
        output_fmt,
        0, // perceptual
        FLAGS_NOOPTIMIZE | FLAGS_NOCACHE,
    ) {
        Ok(x) => x,
        Err(_) => return 0.0,
    };

    // Sample Lab space: 6 L* × 74 a* × 74 b*
    let grid_points = [6u32, 74, 74];
    let mut max_tac: f32 = 0.0;

    let in_stride = crate::pipeline::pack::pixel_size(TYPE_LAB_16);
    let out_stride = crate::pipeline::pack::pixel_size(output_fmt);

    let _ = slice_space_16(3, &grid_points, |input, _cargo| {
        // Pack Lab16 input
        let mut in_buf = vec![0u8; in_stride];
        for (i, &v) in input.iter().take(3).enumerate() {
            let bytes = v.to_ne_bytes();
            in_buf[i * 2] = bytes[0];
            in_buf[i * 2 + 1] = bytes[1];
        }

        let mut out_buf = vec![0u8; out_stride];
        xform.do_transform(&in_buf, &mut out_buf, 1);

        // Sum output channels (float values)
        let mut sum: f32 = 0.0;
        for ch in 0..n_channels as usize {
            let offset = ch * 4;
            if offset + 4 <= out_buf.len() {
                let val = f32::from_ne_bytes([
                    out_buf[offset],
                    out_buf[offset + 1],
                    out_buf[offset + 2],
                    out_buf[offset + 3],
                ]);
                sum += val;
            }
        }

        if sum > max_tac {
            max_tac = sum;
        }

        true
    });

    max_tac as f64
}

/// Detect the gamma of an RGB profile by sampling gray ramps.
///
/// Returns the estimated gamma value, or -1.0 if the profile is not
/// suitable (non-RGB, unsupported class).
///
/// C版: `cmsDetectRGBProfileGamma`
pub fn detect_rgb_profile_gamma(profile: &mut Profile, threshold: f64) -> f64 {
    // Must be RGB
    if profile.header.color_space != ColorSpaceSignature::RgbData {
        return -1.0;
    }

    // Must be a suitable device class
    let class = profile.header.device_class;
    if !matches!(
        class,
        ProfileClassSignature::Input
            | ProfileClassSignature::Display
            | ProfileClassSignature::Output
            | ProfileClassSignature::ColorSpace
    ) {
        return -1.0;
    }

    // Create Profile → XYZ transform
    let profile_copy = match profile.save_to_mem() {
        Ok(data) => match Profile::open_mem(&data) {
            Ok(p) => p,
            Err(_) => return -1.0,
        },
        Err(_) => return -1.0,
    };
    let xyz_profile = {
        let mut p = Profile::new_xyz();
        match p.save_to_mem() {
            Ok(data) => match Profile::open_mem(&data) {
                Ok(p) => p,
                Err(_) => return -1.0,
            },
            Err(_) => return -1.0,
        }
    };

    // Use 16-bit XYZ output (mixed float/int formats not supported)
    let xform = match super::xform::Transform::new(
        profile_copy,
        TYPE_RGB_16,
        xyz_profile,
        TYPE_XYZ_16,
        1, // relative colorimetric
        FLAGS_NOOPTIMIZE,
    ) {
        Ok(x) => x,
        Err(_) => return -1.0,
    };

    // Sample 256 gray levels
    let mut y_values = [0.0f32; 256];
    for i in 0..256u16 {
        let val16 = ((i as u32 * 65535 + 128) / 255) as u16; // FROM_8_TO_16
        let bytes = val16.to_ne_bytes();

        let mut in_buf = [0u8; 6]; // RGB_16 = 3 × 2 bytes
        // R = G = B = val16
        in_buf[0] = bytes[0];
        in_buf[1] = bytes[1];
        in_buf[2] = bytes[0];
        in_buf[3] = bytes[1];
        in_buf[4] = bytes[0];
        in_buf[5] = bytes[1];

        let mut out_buf = [0u8; 6]; // XYZ_16 = 3 × 2 bytes
        xform.do_transform(&in_buf, &mut out_buf, 1);

        // Extract Y from XYZ_16 (second u16) and decode to float
        // ICC PCS XYZ: u1Fixed15Number, Y = encoded / 32768.0
        let y_encoded = u16::from_ne_bytes([out_buf[2], out_buf[3]]);
        y_values[i as usize] = (y_encoded as f64 / 32768.0) as f32;
    }

    // Build tone curve from Y values and estimate gamma
    let y_curve = match ToneCurve::build_tabulated_float(&y_values) {
        Some(tc) => tc,
        None => return -1.0,
    };

    y_curve.estimate_gamma(threshold)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(p: &mut Profile) -> Profile {
        let data = p.save_to_mem().unwrap();
        Profile::open_mem(&data).unwrap()
    }

    // ================================================================
    // desaturate_lab
    // ================================================================

    #[test]

    fn desaturate_in_gamut() {
        let mut lab = CieLab {
            l: 50.0,
            a: 10.0,
            b: -10.0,
        };
        let ok = desaturate_lab(&mut lab, 50.0, -50.0, 50.0, -50.0);
        assert!(ok);
        assert!((lab.a - 10.0).abs() < 1e-10);
        assert!((lab.b - (-10.0)).abs() < 1e-10);
    }

    #[test]

    fn desaturate_negative_l() {
        let mut lab = CieLab {
            l: -5.0,
            a: 10.0,
            b: 20.0,
        };
        let ok = desaturate_lab(&mut lab, 50.0, -50.0, 50.0, -50.0);
        assert!(!ok);
        assert_eq!(lab.l, 0.0);
        assert_eq!(lab.a, 0.0);
        assert_eq!(lab.b, 0.0);
    }

    #[test]

    fn desaturate_clips_l_above_100() {
        let mut lab = CieLab {
            l: 120.0,
            a: 0.0,
            b: 0.0,
        };
        let ok = desaturate_lab(&mut lab, 50.0, -50.0, 50.0, -50.0);
        assert!(ok);
        assert_eq!(lab.l, 100.0);
    }

    #[test]

    fn desaturate_clips_out_of_gamut_a() {
        let mut lab = CieLab {
            l: 50.0,
            a: 80.0,
            b: 20.0,
        };
        let ok = desaturate_lab(&mut lab, 50.0, -50.0, 50.0, -50.0);
        assert!(ok);
        // Should be clipped by amax (hue zone 0..45° or 315..360°)
        assert!(lab.a <= 50.0 + 0.01, "a = {}", lab.a);
    }

    #[test]

    fn desaturate_zero_a_clips_b() {
        let mut lab = CieLab {
            l: 50.0,
            a: 0.0,
            b: 80.0,
        };
        let ok = desaturate_lab(&mut lab, 50.0, -50.0, 50.0, -50.0);
        assert!(ok);
        assert_eq!(lab.b, 50.0); // clipped to bmax
    }

    // ================================================================
    // detect_tac
    // ================================================================

    #[test]

    fn detect_tac_non_output_returns_zero() {
        // sRGB is Display class, not Output
        let mut p = roundtrip(&mut Profile::new_srgb());
        let tac = detect_tac(&mut p);
        assert_eq!(tac, 0.0);
    }

    // ================================================================
    // detect_rgb_profile_gamma
    // ================================================================

    #[test]

    fn detect_gamma_srgb() {
        let mut p = roundtrip(&mut Profile::new_srgb());
        let gamma = detect_rgb_profile_gamma(&mut p, 0.1);
        // sRGB gamma is approximately 2.2 (actually ~2.19 due to linear segment)
        assert!(gamma > 1.8 && gamma < 2.6, "expected ~2.2, got {}", gamma);
    }

    #[test]

    fn detect_gamma_linear() {
        let gamma_curve = crate::curves::gamma::ToneCurve::build_gamma(1.0).unwrap();
        let trc = [gamma_curve.clone(), gamma_curve.clone(), gamma_curve];
        let d65 = CieXyY {
            x: 0.3127,
            y: 0.3290,
            big_y: 1.0,
        };
        let primaries = CieXyYTriple {
            red: CieXyY {
                x: 0.64,
                y: 0.33,
                big_y: 1.0,
            },
            green: CieXyY {
                x: 0.30,
                y: 0.60,
                big_y: 1.0,
            },
            blue: CieXyY {
                x: 0.15,
                y: 0.06,
                big_y: 1.0,
            },
        };
        let mut p = Profile::new_rgb(&d65, &primaries, &trc);
        let mut p = roundtrip(&mut p);
        let gamma = detect_rgb_profile_gamma(&mut p, 0.1);
        assert!((gamma - 1.0).abs() < 0.15, "expected ~1.0, got {}", gamma);
    }

    #[test]

    fn detect_gamma_non_rgb_returns_minus1() {
        // Lab profile is not RGB
        let mut p = roundtrip(&mut Profile::new_lab4(None));
        let gamma = detect_rgb_profile_gamma(&mut p, 0.1);
        assert_eq!(gamma, -1.0);
    }
}
