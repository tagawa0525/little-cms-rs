// ============================================================================
// Gamut mapping utilities (C版: cmsgmt.c)
// ============================================================================

use crate::context::{CmsError, ErrorCode};
use crate::curves::gamma::ToneCurve;
use crate::math::pcs;
use crate::pipeline::lut::{Pipeline, Stage, StageLoc, sample_clut_16bit, slice_space_16};
use crate::profile::io::Profile;
use crate::types::*;

use super::xform::{FLAGS_HIGHRESPRECALC, FLAGS_NOCACHE, FLAGS_NOOPTIMIZE, Transform};

/// Threshold for out-of-gamut detection on LUT-based profiles.
const ERR_THRESHOLD: f64 = 5.0;

// ============================================================================
// Gamut check pipeline
// ============================================================================

/// Serialize and deserialize a profile to create a copy.
fn clone_profile(profile: &mut Profile) -> Result<Profile, CmsError> {
    let data = profile.save_to_mem()?;
    Profile::open_mem(&data)
}

/// Build a gamut check pipeline that maps input colors to a single-channel
/// bilevel signal: 0 = in-gamut, >0 = out-of-gamut (with dE magnitude).
///
/// C版: `_cmsCreateGamutCheckPipeline`
pub fn create_gamut_check_pipeline(
    profiles: &mut [Profile],
    bpc: &[bool],
    intents: &[u32],
    adaptation: &[f64],
    gamut_pcs_position: usize,
    gamut_profile: &mut Profile,
) -> Result<Pipeline, CmsError> {
    if gamut_pcs_position == 0 || gamut_pcs_position > 255 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Wrong PCS position. 1..255 expected, {gamut_pcs_position} found."),
        });
    }

    // Threshold: matrix-shaper profiles are very accurate, LUT-based less so
    let threshold = if gamut_profile.is_matrix_shaper() {
        1.0
    } else {
        ERR_THRESHOLD
    };

    let color_space = gamut_profile.header.color_space;
    let n_channels = color_space.channels();
    let n_gridpoints = pcs::reasonable_gridpoints(n_channels, FLAGS_HIGHRESPRECALC);

    // Build hInput: profiles[0..gamut_pcs_position] + Lab → converts input to Lab16
    let mut input_chain: Vec<Profile> = Vec::with_capacity(gamut_pcs_position + 1);
    let mut input_bpc: Vec<bool> = Vec::with_capacity(gamut_pcs_position + 1);
    let mut input_intents: Vec<u32> = Vec::with_capacity(gamut_pcs_position + 1);
    let mut input_adaptation: Vec<f64> = Vec::with_capacity(gamut_pcs_position + 1);

    for i in 0..gamut_pcs_position {
        input_chain.push(clone_profile(&mut profiles[i])?);
        input_bpc.push(bpc[i]);
        input_intents.push(intents[i]);
        input_adaptation.push(adaptation[i]);
    }
    // Append Lab profile at the end
    let mut lab = Profile::new_lab4(None);
    input_chain.push(clone_profile(&mut lab)?);
    input_bpc.push(false);
    input_intents.push(1); // INTENT_RELATIVE_COLORIMETRIC
    input_adaptation.push(1.0);

    let input_cs = profiles[0].header.color_space;
    let n_input_channels = input_cs.channels();
    let input_fmt = PixelFormat::build(input_cs.to_pixel_type(), n_input_channels, 2);

    let h_input = Transform::new_multiprofile(
        &mut input_chain,
        input_fmt,
        TYPE_LAB_16,
        intents[0],
        FLAGS_NOCACHE | FLAGS_NOOPTIMIZE,
    )?;

    // Build hForward: Lab → gamut device colorants (relative colorimetric)
    let lab_fwd = clone_profile(&mut lab)?;
    let gamut_fwd = clone_profile(gamut_profile)?;
    let device_fmt = PixelFormat::build(color_space.to_pixel_type(), n_channels, 2);

    let h_forward = Transform::new(
        lab_fwd,
        TYPE_LAB_16,
        gamut_fwd,
        device_fmt,
        1, // INTENT_RELATIVE_COLORIMETRIC
        FLAGS_NOCACHE | FLAGS_NOOPTIMIZE,
    )?;

    // Build hReverse: gamut device colorants → Lab (relative colorimetric)
    let gamut_rev = clone_profile(gamut_profile)?;
    let lab_rev = clone_profile(&mut lab)?;

    let h_reverse = Transform::new(
        gamut_rev,
        device_fmt,
        lab_rev,
        TYPE_LAB_16,
        1, // INTENT_RELATIVE_COLORIMETRIC
        FLAGS_NOCACHE | FLAGS_NOOPTIMIZE,
    )?;

    // Build the gamut check pipeline: nChannels input → 1 output
    let mut gamut = Pipeline::new(n_channels, 1).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to allocate gamut check pipeline".into(),
    })?;

    let grid_points = [n_gridpoints; crate::curves::intrp::MAX_INPUT_DIMENSIONS];
    let mut clut =
        Stage::new_clut_16bit(&grid_points, n_channels, 1, None).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to allocate gamut check CLUT".into(),
        })?;

    // Pre-compute byte strides for the transforms
    let in_stride = crate::pipeline::pack::pixel_size(input_fmt);
    let lab_stride = crate::pipeline::pack::pixel_size(TYPE_LAB_16);
    let dev_stride = crate::pipeline::pack::pixel_size(device_fmt);

    // Sample the CLUT with the gamut sampler
    sample_clut_16bit(
        &mut clut,
        |input: &[u16], output: &mut [u16], _cargo: &()| {
            // Pack input as 16-bit bytes for hInput transform
            let mut in_buf = vec![0u8; in_stride];
            for (i, &v) in input.iter().enumerate().take(n_input_channels as usize) {
                let bytes = v.to_ne_bytes();
                in_buf[i * 2] = bytes[0];
                in_buf[i * 2 + 1] = bytes[1];
            }

            // hInput: input → Lab16
            let mut lab_buf = vec![0u8; lab_stride];
            h_input.do_transform(&in_buf, &mut lab_buf, 1);

            // Decode Lab16 → CIELab
            let lab_in1 = decode_lab16_buf(&lab_buf);

            // Encode Lab → Lab16 bytes for forward transform
            encode_lab16_buf(&lab_in1, &mut lab_buf);

            // hForward: Lab → device colorants
            let mut proof_buf = vec![0u8; dev_stride];
            h_forward.do_transform(&lab_buf, &mut proof_buf, 1);

            // hReverse: device → Lab
            let mut lab_out_buf = vec![0u8; lab_stride];
            h_reverse.do_transform(&proof_buf, &mut lab_out_buf, 1);
            let lab_out1 = decode_lab16_buf(&lab_out_buf);

            // Second round-trip
            encode_lab16_buf(&lab_out1, &mut lab_buf);
            let mut proof2_buf = vec![0u8; dev_stride];
            h_forward.do_transform(&lab_buf, &mut proof2_buf, 1);
            let mut lab_out2_buf = vec![0u8; lab_stride];
            h_reverse.do_transform(&proof2_buf, &mut lab_out2_buf, 1);
            let lab_out2 = decode_lab16_buf(&lab_out2_buf);

            let lab_in2 = lab_out1;

            // Compute dE values
            let d_e1 = pcs::delta_e(&lab_in1, &lab_out1);
            let d_e2 = pcs::delta_e(&lab_in2, &lab_out2);

            // Gamut decision
            if d_e1 < threshold && d_e2 < threshold {
                output[0] = 0; // In gamut
            } else if d_e1 < threshold {
                output[0] = 0; // Undefined, assume in gamut
            } else if d_e2 < threshold {
                // Clearly out of gamut
                output[0] = ((d_e1 - threshold) + 0.5) as u16;
            } else {
                // Both large — perceptual mapping case
                let error_ratio = if d_e2 == 0.0 { d_e1 } else { d_e1 / d_e2 };
                if error_ratio > threshold {
                    output[0] = ((error_ratio - threshold) + 0.5) as u16;
                } else {
                    output[0] = 0;
                }
            }

            // Clamp to max encodeable value
            if output[0] > 0xFFFE {
                output[0] = 0xFFFE;
            }

            true
        },
        0,
    );

    gamut.insert_stage(StageLoc::AtBegin, clut);
    Ok(gamut)
}

/// Decode Lab from 16-bit byte buffer (native endian).
fn decode_lab16_buf(buf: &[u8]) -> CieLab {
    let encoded = [
        u16::from_ne_bytes([buf[0], buf[1]]),
        u16::from_ne_bytes([buf[2], buf[3]]),
        u16::from_ne_bytes([buf[4], buf[5]]),
    ];
    pcs::pcs_encoded_lab_to_float(&encoded)
}

/// Encode CIELab to 16-bit byte buffer (native endian).
fn encode_lab16_buf(lab: &CieLab, buf: &mut [u8]) {
    let encoded = pcs::float_to_pcs_encoded_lab(lab);
    buf[0..2].copy_from_slice(&encoded[0].to_ne_bytes());
    buf[2..4].copy_from_slice(&encoded[1].to_ne_bytes());
    buf[4..6].copy_from_slice(&encoded[2].to_ne_bytes());
}

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
        // Hue is on the b* axis (90° or 270°) — slope b/a is undefined
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
    if n_channels == 0 || n_channels as usize > MAX_CHANNELS {
        return 0.0;
    }

    // Build a 16-bit formatter for the profile's output space
    let cs_bits = cs.to_pixel_type();
    let output_fmt = PixelFormat::build(cs_bits, n_channels, 2);

    // Create Lab → Profile transform (perceptual intent, both 16-bit)
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
    let n_ch = n_channels as usize;

    // Pre-allocate buffers outside the loop
    let in_stride = crate::pipeline::pack::pixel_size(TYPE_LAB_16);
    let out_stride = crate::pipeline::pack::pixel_size(output_fmt);
    let mut in_buf = vec![0u8; in_stride];
    let mut out_buf = vec![0u8; out_stride];

    let _ = slice_space_16(3, &grid_points, |input, _cargo| {
        // Pack Lab16 input (native endian)
        for (i, &v) in input.iter().take(3).enumerate() {
            let bytes = v.to_ne_bytes();
            in_buf[i * 2] = bytes[0];
            in_buf[i * 2 + 1] = bytes[1];
        }

        xform.do_transform(&in_buf, &mut out_buf, 1);

        // Sum output channels (16-bit values normalized to 0..100 range)
        let mut sum: f32 = 0.0;
        for ch in 0..n_ch {
            let offset = ch * 2;
            let val = u16::from_ne_bytes([out_buf[offset], out_buf[offset + 1]]);
            // Normalize: 0xFFFF → 100%
            sum += val as f32 / 655.35;
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
/// `precision` controls the standard deviation threshold for the gamma
/// estimation fit (passed to `ToneCurve::estimate_gamma`).
/// Returns the estimated gamma value, or -1.0 if the profile is not
/// suitable (non-RGB, unsupported class).
///
/// C版: `cmsDetectRGBProfileGamma`
pub fn detect_rgb_profile_gamma(profile: &mut Profile, precision: f64) -> f64 {
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

    y_curve.estimate_gamma(precision)
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

    // ================================================================
    // create_gamut_check_pipeline
    // ================================================================

    #[test]
    fn gamut_check_pipeline_srgb_midgray_in_gamut() {
        // Mid-gray RGB should be in sRGB gamut → output 0
        // The gamut check pipeline input is RGB (gamut profile's color space),
        // not Lab. The sampler converts RGB → Lab internally.
        let srgb1 = roundtrip(&mut Profile::new_srgb());
        let srgb2 = roundtrip(&mut Profile::new_srgb());
        let mut gamut_profile = roundtrip(&mut Profile::new_srgb());

        let pipeline = create_gamut_check_pipeline(
            &mut [srgb1, srgb2],
            &[false, false],
            &[0, 0],
            &[1.0, 1.0],
            1,
            &mut gamut_profile,
        )
        .unwrap();

        // Mid-gray RGB (0x8000, 0x8000, 0x8000) — always in sRGB gamut
        let mid = 0x8000u16;
        let input = [mid, mid, mid, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut output = [0u16; 16];
        pipeline.eval_16(&input, &mut output);

        // sRGB color checked against sRGB gamut → in gamut
        assert_eq!(output[0], 0, "mid-gray should be in sRGB gamut");
    }

    #[test]
    fn gamut_check_pipeline_narrow_gamut_detects_out_of_gamut() {
        // Create a narrow-gamut profile and check sRGB colors against it.
        // Saturated sRGB colors should be out of the narrow gamut.
        let gamma_curve = crate::curves::gamma::ToneCurve::build_gamma(2.2).unwrap();
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
        let narrow = Profile::new_rgb(&d65, &narrow_primaries, &trc);
        let mut narrow = roundtrip(&mut { narrow });

        let srgb = roundtrip(&mut Profile::new_srgb());
        let srgb2 = roundtrip(&mut Profile::new_srgb());

        let pipeline = create_gamut_check_pipeline(
            &mut [srgb, srgb2],
            &[false, false],
            &[0, 0],
            &[1.0, 1.0],
            1,
            &mut narrow,
        )
        .unwrap();

        // Pure red (0xFFFF, 0, 0) in sRGB — should be outside the narrow gamut
        let input = [0xFFFFu16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut output = [0u16; 16];
        pipeline.eval_16(&input, &mut output);

        assert!(
            output[0] > 0,
            "saturated red should be out of narrow gamut, got {}",
            output[0]
        );
    }
}
