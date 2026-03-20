//! PCS (Profile Connection Space) color space conversions.
//!
//! XYZ ↔ Lab, encoding/decoding, DeltaE. Matches C版 `cmspcs.c`.

use crate::types::{CieLCh, CieLab, CieXyY, CieXyz, D50_X, D50_Y, D50_Z};

// CIE L*a*b* constants
const CIE_EPSILON: f64 = 216.0 / 24389.0; // 0.008856
const CIE_KAPPA: f64 = 24389.0 / 27.0; // 903.3

/// ICC PCS XYZ encoding: values are encoded as u1Fixed15Number
/// where 1.0 + (32767.0/32768.0) is the maximum.
const MAX_ENCODABLE_XYZ: f64 = 1.0 + 32767.0 / 32768.0;

fn f_lab(t: f64) -> f64 {
    if t > CIE_EPSILON {
        t.cbrt()
    } else {
        (CIE_KAPPA * t + 16.0) / 116.0
    }
}

fn f_lab_inverse(t: f64) -> f64 {
    let t3 = t * t * t;
    if t3 > CIE_EPSILON {
        t3
    } else {
        (116.0 * t - 16.0) / CIE_KAPPA
    }
}

pub fn xyz_to_lab(white_point: &CieXyz, xyz: &CieXyz) -> CieLab {
    let fx = f_lab(xyz.x / white_point.x);
    let fy = f_lab(xyz.y / white_point.y);
    let fz = f_lab(xyz.z / white_point.z);

    CieLab {
        l: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

pub fn lab_to_xyz(white_point: &CieXyz, lab: &CieLab) -> CieXyz {
    let fy = (lab.l + 16.0) / 116.0;
    let fx = lab.a / 500.0 + fy;
    let fz = fy - lab.b / 200.0;

    CieXyz {
        x: f_lab_inverse(fx) * white_point.x,
        y: f_lab_inverse(fy) * white_point.y,
        z: f_lab_inverse(fz) * white_point.z,
    }
}

pub fn lab_to_lch(lab: &CieLab) -> CieLCh {
    let c = (lab.a * lab.a + lab.b * lab.b).sqrt();
    let h = lab.b.atan2(lab.a).to_degrees();
    CieLCh {
        l: lab.l,
        c,
        h: if h < 0.0 { h + 360.0 } else { h },
    }
}

pub fn lch_to_lab(lch: &CieLCh) -> CieLab {
    let h_rad = lch.h.to_radians();
    CieLab {
        l: lch.l,
        a: lch.c * h_rad.cos(),
        b: lch.c * h_rad.sin(),
    }
}

pub fn xyz_to_xyy(xyz: &CieXyz) -> CieXyY {
    let sum = xyz.x + xyz.y + xyz.z;
    if sum.abs() < 1e-20 {
        // Black: use D50 chromaticity
        let d50_sum = D50_X + D50_Y + D50_Z;
        CieXyY {
            x: D50_X / d50_sum,
            y: D50_Y / d50_sum,
            big_y: 0.0,
        }
    } else {
        CieXyY {
            x: xyz.x / sum,
            y: xyz.y / sum,
            big_y: xyz.y,
        }
    }
}

pub fn xyy_to_xyz(xyy: &CieXyY) -> CieXyz {
    if xyy.y.abs() < 1e-20 {
        CieXyz {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    } else {
        CieXyz {
            x: xyy.x * xyy.big_y / xyy.y,
            y: xyy.big_y,
            z: (1.0 - xyy.x - xyy.y) * xyy.big_y / xyy.y,
        }
    }
}

pub fn delta_e(lab1: &CieLab, lab2: &CieLab) -> f64 {
    let dl = lab1.l - lab2.l;
    let da = lab1.a - lab2.a;
    let db = lab1.b - lab2.b;
    (dl * dl + da * da + db * db).sqrt()
}

/// CIE94 DeltaE. C版: `cmsCIE94DeltaE`
pub fn delta_e_cie94(lab1: &CieLab, lab2: &CieLab) -> f64 {
    let lch1 = lab_to_lch(lab1);
    let lch2 = lab_to_lch(lab2);

    let dl = (lab1.l - lab2.l).abs();
    let dc = (lch1.c - lch2.c).abs();
    let de = delta_e(lab1, lab2);

    let dhsq = de * de - dl * dl - dc * dc;
    let dh = if dhsq < 0.0 { 0.0 } else { dhsq.sqrt() };

    let c12 = (lch1.c * lch2.c).sqrt();
    let sc = 1.0 + 0.048 * c12;
    let sh = 1.0 + 0.014 * c12;

    (dl * dl + (dc / sc).powi(2) + (dh / sh).powi(2)).sqrt()
}

/// BFD lightness function. C版: `ComputeLBFD`
fn compute_lbfd(lab: &CieLab) -> f64 {
    let yt = if lab.l > CIE_KAPPA * CIE_EPSILON {
        let t = (lab.l + 16.0) / 116.0;
        t * t * t * 100.0
    } else {
        100.0 * (lab.l / CIE_KAPPA)
    };
    54.6 * (std::f64::consts::LOG10_E * (yt + 1.5).ln()) - 9.6
}

/// BFD(1:1) DeltaE. C版: `cmsBFDdeltaE`
pub fn delta_e_bfd(lab1: &CieLab, lab2: &CieLab) -> f64 {
    let lbfd1 = compute_lbfd(lab1);
    let lbfd2 = compute_lbfd(lab2);
    let delta_l = lbfd2 - lbfd1;

    let lch1 = lab_to_lch(lab1);
    let lch2 = lab_to_lch(lab2);

    let delta_c = lch2.c - lch1.c;
    let ave_c = (lch1.c + lch2.c) / 2.0;

    // Circular mean for hue angles (handle 0°/360° wrap-around)
    let (mut h1, mut h2) = (lch1.h, lch2.h);
    if (h1 - h2).abs() > 180.0 {
        if h1 < h2 {
            h1 += 360.0;
        } else {
            h2 += 360.0;
        }
    }
    let mut ave_h = (h1 + h2) / 2.0;
    if ave_h >= 360.0 {
        ave_h -= 360.0;
    }

    let de = delta_e(lab1, lab2);
    let dhsq = de * de - (lab2.l - lab1.l).powi(2) - delta_c * delta_c;
    let delta_h = if dhsq > 0.0 { dhsq.sqrt() } else { 0.0 };

    let dc = 0.035 * ave_c / (1.0 + 0.00365 * ave_c) + 0.521;
    let g = (ave_c.powi(4) / (ave_c.powi(4) + 14000.0)).sqrt();
    let ave_h_rad = |n: f64| n * std::f64::consts::PI / 180.0;
    let t = 0.627 + 0.055 * (ave_h_rad(ave_h - 254.0)).cos()
        - 0.040 * (ave_h_rad(2.0 * ave_h - 136.0)).cos()
        + 0.070 * (ave_h_rad(3.0 * ave_h - 31.0)).cos()
        + 0.049 * (ave_h_rad(4.0 * ave_h + 114.0)).cos()
        - 0.015 * (ave_h_rad(5.0 * ave_h - 103.0)).cos();

    let dh = dc * (g * t + 1.0 - g);
    let rh = -0.260 * (ave_h_rad(ave_h - 308.0)).cos()
        - 0.379 * (ave_h_rad(2.0 * ave_h - 160.0)).cos()
        - 0.636 * (ave_h_rad(3.0 * ave_h + 254.0)).cos()
        + 0.226 * (ave_h_rad(4.0 * ave_h + 140.0)).cos()
        - 0.194 * (ave_h_rad(5.0 * ave_h + 280.0)).cos();

    let rc = (ave_c.powi(6) / (ave_c.powi(6) + 70_000_000.0)).sqrt();
    let rt = rh * rc;

    (delta_l * delta_l
        + (delta_c / dc).powi(2)
        + (delta_h / dh).powi(2)
        + rt * (delta_c / dc) * (delta_h / dh))
        .sqrt()
}

/// CMC(l:c) DeltaE. C版: `cmsCMCdeltaE`
pub fn delta_e_cmc(lab1: &CieLab, lab2: &CieLab, l: f64, c: f64) -> f64 {
    if lab1.l == 0.0 && lab2.l == 0.0 {
        return 0.0;
    }

    let lch1 = lab_to_lch(lab1);
    let lch2 = lab_to_lch(lab2);

    let dl = lab2.l - lab1.l;
    let dc = lch2.c - lch1.c;

    let de = delta_e(lab1, lab2);
    let dhsq = de * de - dl * dl - dc * dc;
    let dh = if dhsq > 0.0 { dhsq.sqrt() } else { 0.0 };

    let t = if lch1.h > 164.0 && lch1.h < 345.0 {
        0.56 + (0.2 * ((lch1.h + 168.0).to_radians()).cos()).abs()
    } else {
        0.36 + (0.4 * ((lch1.h + 35.0).to_radians()).cos()).abs()
    };

    let sc = 0.0638 * lch1.c / (1.0 + 0.0131 * lch1.c) + 0.638;
    let sl = if lab1.l < 16.0 {
        0.511
    } else {
        0.040975 * lab1.l / (1.0 + 0.01765 * lab1.l)
    };

    let f = (lch1.c.powi(4) / (lch1.c.powi(4) + 1900.0)).sqrt();
    let sh = sc * (t * f + 1.0 - f);

    ((dl / (l * sl)).powi(2) + (dc / (c * sc)).powi(2) + (dh / sh).powi(2)).sqrt()
}

/// atan2 in degrees [0, 360). C版: `atan2deg`
fn atan2deg(b: f64, a: f64) -> f64 {
    let mut h = b.atan2(a).to_degrees();
    while h < 0.0 {
        h += 360.0;
    }
    while h >= 360.0 {
        h -= 360.0;
    }
    h
}

/// CIEDE2000 DeltaE. C版: `cmsCIE2000DeltaE`
pub fn delta_e_ciede2000(lab1: &CieLab, lab2: &CieLab, kl: f64, kc: f64, kh: f64) -> f64 {
    let c1 = (lab1.a * lab1.a + lab1.b * lab1.b).sqrt();
    let c2 = (lab2.a * lab2.a + lab2.b * lab2.b).sqrt();

    let mean_c = (c1 + c2) / 2.0;
    let g = 0.5 * (1.0 - (mean_c.powi(7) / (mean_c.powi(7) + 25.0_f64.powi(7))).sqrt());

    let a_p1 = (1.0 + g) * lab1.a;
    let c_p1 = (a_p1 * a_p1 + lab1.b * lab1.b).sqrt();
    let h_p1 = atan2deg(lab1.b, a_p1);

    let a_p2 = (1.0 + g) * lab2.a;
    let c_p2 = (a_p2 * a_p2 + lab2.b * lab2.b).sqrt();
    let h_p2 = atan2deg(lab2.b, a_p2);

    let mean_c_p = (c_p1 + c_p2) / 2.0;

    let hp_diff = h_p2 - h_p1;
    let hp_sum = h_p2 + h_p1;

    let mean_h_p = if hp_diff.abs() <= 180.000001 {
        hp_sum / 2.0
    } else if hp_sum < 360.0 {
        (hp_sum + 360.0) / 2.0
    } else {
        (hp_sum - 360.0) / 2.0
    };

    let delta_h = if hp_diff <= -180.000001 {
        hp_diff + 360.0
    } else if hp_diff > 180.0 {
        hp_diff - 360.0
    } else {
        hp_diff
    };

    let delta_l = lab2.l - lab1.l;
    let delta_c = c_p2 - c_p1;
    let delta_h_big = 2.0 * (c_p2 * c_p1).sqrt() * (delta_h.to_radians() / 2.0).sin();

    let t = 1.0 - 0.17 * (mean_h_p - 30.0).to_radians().cos()
        + 0.24 * (2.0 * mean_h_p).to_radians().cos()
        + 0.32 * (3.0 * mean_h_p + 6.0).to_radians().cos()
        - 0.20 * (4.0 * mean_h_p - 63.0).to_radians().cos();

    let mean_l = (lab2.l + lab1.l) / 2.0;
    let sl = 1.0 + 0.015 * (mean_l - 50.0).powi(2) / (20.0 + (mean_l - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * mean_c_p;
    let sh = 1.0 + 0.015 * mean_c_p * t;

    let delta_ro = 30.0 * (-((mean_h_p - 275.0) / 25.0).powi(2)).exp();
    let rc = 2.0 * (mean_c_p.powi(7) / (mean_c_p.powi(7) + 25.0_f64.powi(7))).sqrt();
    let rt = -(2.0 * delta_ro.to_radians()).sin() * rc;

    ((delta_l / (sl * kl)).powi(2)
        + (delta_c / (sc * kc)).powi(2)
        + (delta_h_big / (sh * kh)).powi(2)
        + rt * (delta_c / (sc * kc)) * (delta_h_big / (sh * kh)))
        .sqrt()
}

/// Encode XYZ to ICC 16-bit PCS encoding (u1Fixed15Number for XYZ).
pub fn float_to_pcs_encoded_xyz(xyz: &CieXyz) -> [u16; 3] {
    let encode = |v: f64| -> u16 {
        let clamped = v.clamp(0.0, MAX_ENCODABLE_XYZ);
        (clamped * 32768.0 + 0.5) as u16
    };
    [encode(xyz.x), encode(xyz.y), encode(xyz.z)]
}

/// Decode ICC 16-bit PCS encoding back to XYZ.
pub fn pcs_encoded_xyz_to_float(encoded: &[u16; 3]) -> CieXyz {
    CieXyz {
        x: encoded[0] as f64 / 32768.0,
        y: encoded[1] as f64 / 32768.0,
        z: encoded[2] as f64 / 32768.0,
    }
}

/// Encode Lab to ICC 16-bit PCS encoding.
/// L: 0..100 → 0..0xFF00, a: -128..127 → 0..0xFF00, b: -128..127 → 0..0xFF00
pub fn float_to_pcs_encoded_lab(lab: &CieLab) -> [u16; 3] {
    let l = (lab.l * 655.35 + 0.5) as u16;
    let a = ((lab.a + 128.0) * 256.0 + 0.5) as u16;
    let b = ((lab.b + 128.0) * 256.0 + 0.5) as u16;
    [l, a, b]
}

/// Decode ICC 16-bit PCS encoding back to Lab.
pub fn pcs_encoded_lab_to_float(encoded: &[u16; 3]) -> CieLab {
    CieLab {
        l: encoded[0] as f64 / 655.35,
        a: encoded[1] as f64 / 256.0 - 128.0,
        b: encoded[2] as f64 / 256.0 - 128.0,
    }
}

/// Encode Lab to ICC 16-bit V2 encoding.
/// V2 differs from V4: L uses 652.8 (0xFF00/100) instead of 655.35 (0xFFFF/100),
/// and a/b use 256.0 instead of 257.0.
/// C版: `cmsFloat2LabEncodedV2`
pub fn float_to_pcs_encoded_lab_v2(_lab: &CieLab) -> [u16; 3] {
    todo!()
}

/// Decode ICC 16-bit V2 encoding back to Lab.
/// C版: `cmsLabEncoded2FloatV2`
pub fn pcs_encoded_lab_to_float_v2(_encoded: &[u16; 3]) -> CieLab {
    todo!()
}

// ============================================================================
// Color space endpoints
// ============================================================================

use crate::types::ColorSpaceSignature;

/// Return the white and black endpoints for a color space as 16-bit values.
/// Returns `(white, black, n_channels)` or `None` if unsupported.
/// C版: `_cmsEndPointsBySpace`
pub fn endpoints_by_space(space: ColorSpaceSignature) -> Option<([u16; 16], [u16; 16], u32)> {
    let mut white = [0u16; 16];
    let mut black = [0u16; 16];

    match space {
        ColorSpaceSignature::GrayData => {
            white[0] = 0xFFFF;
            Some((white, black, 1))
        }
        ColorSpaceSignature::RgbData => {
            white[0] = 0xFFFF;
            white[1] = 0xFFFF;
            white[2] = 0xFFFF;
            Some((white, black, 3))
        }
        ColorSpaceSignature::CmykData => {
            // CMYK: white = no ink (0), black = max ink (0xFFFF)
            black[0] = 0xFFFF;
            black[1] = 0xFFFF;
            black[2] = 0xFFFF;
            black[3] = 0xFFFF;
            Some((white, black, 4))
        }
        ColorSpaceSignature::CmyData => {
            black[0] = 0xFFFF;
            black[1] = 0xFFFF;
            black[2] = 0xFFFF;
            Some((white, black, 3))
        }
        ColorSpaceSignature::LabData => {
            // V4 Lab encoding: L*=100 → 0xFFFF, a*=0 → 0x8080, b*=0 → 0x8080
            white[0] = 0xFFFF;
            white[1] = 0x8080;
            white[2] = 0x8080;
            black[1] = 0x8080;
            black[2] = 0x8080;
            Some((white, black, 3))
        }
        _ => None,
    }
}

/// Return a reasonable number of grid points for a CLUT, given the number
/// of input channels and transform flags.
///
/// C版: `_cmsReasonableGridpointsByColorspace`
pub fn reasonable_gridpoints(n_channels: u32, flags: u32) -> u32 {
    // Grid points explicitly specified in flags bits 16..23?
    if flags & 0x00FF_0000 != 0 {
        return ((flags >> 16) & 0xFF).max(2);
    }

    // High-resolution precalc
    if flags & 0x0400 != 0 {
        if n_channels > 4 {
            return 7;
        }
        if n_channels == 4 {
            return 23;
        }
        return 49;
    }

    // Low-resolution precalc
    if flags & 0x0800 != 0 {
        if n_channels > 4 {
            return 6;
        }
        if n_channels == 1 {
            return 33;
        }
        return 17;
    }

    // Defaults
    if n_channels > 4 {
        return 7;
    }
    if n_channels == 4 {
        return 17;
    }
    33
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-4;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    fn d50() -> CieXyz {
        CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        }
    }

    #[test]
    fn xyz_lab_round_trip_white() {
        // D50 white point should map to L*=100, a*=0, b*=0
        let white = d50();
        let lab = xyz_to_lab(&white, &white);
        assert!(close(lab.l, 100.0));
        assert!(close(lab.a, 0.0));
        assert!(close(lab.b, 0.0));
    }

    #[test]
    fn xyz_lab_round_trip_black() {
        let white = d50();
        let black = CieXyz {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let lab = xyz_to_lab(&white, &black);
        assert!(close(lab.l, 0.0));
        let back = lab_to_xyz(&white, &lab);
        assert!(close(back.x, 0.0));
        assert!(close(back.y, 0.0));
        assert!(close(back.z, 0.0));
    }

    #[test]
    fn xyz_lab_round_trip_arbitrary() {
        let white = d50();
        let xyz = CieXyz {
            x: 0.4,
            y: 0.3,
            z: 0.2,
        };
        let lab = xyz_to_lab(&white, &xyz);
        let back = lab_to_xyz(&white, &lab);
        assert!(close(back.x, xyz.x));
        assert!(close(back.y, xyz.y));
        assert!(close(back.z, xyz.z));
    }

    #[test]
    fn lab_lch_round_trip() {
        let lab = CieLab {
            l: 50.0,
            a: 30.0,
            b: -20.0,
        };
        let lch = lab_to_lch(&lab);
        assert!(close(lch.l, 50.0));
        assert!(lch.c > 0.0);
        let back = lch_to_lab(&lch);
        assert!(close(back.l, lab.l));
        assert!(close(back.a, lab.a));
        assert!(close(back.b, lab.b));
    }

    #[test]
    fn xyz_xyy_round_trip() {
        let xyz = CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        };
        let xyy = xyz_to_xyy(&xyz);
        assert!(close(xyy.big_y, D50_Y));
        let back = xyy_to_xyz(&xyy);
        assert!(close(back.x, xyz.x));
        assert!(close(back.y, xyz.y));
        assert!(close(back.z, xyz.z));
    }

    #[test]
    fn delta_e_same_color() {
        let lab = CieLab {
            l: 50.0,
            a: 25.0,
            b: -10.0,
        };
        assert!(close(delta_e(&lab, &lab), 0.0));
    }

    #[test]
    fn delta_e_known_value() {
        let lab1 = CieLab {
            l: 50.0,
            a: 25.0,
            b: -10.0,
        };
        let lab2 = CieLab {
            l: 53.0,
            a: 25.0,
            b: -10.0,
        };
        // Only L differs by 3.0, so ΔE = 3.0
        assert!(close(delta_e(&lab1, &lab2), 3.0));
    }

    #[test]
    fn pcs_xyz_encoding_round_trip() {
        let xyz = CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        };
        let encoded = float_to_pcs_encoded_xyz(&xyz);
        let back = pcs_encoded_xyz_to_float(&encoded);
        assert!(close(back.x, xyz.x));
        assert!(close(back.y, xyz.y));
        assert!(close(back.z, xyz.z));
    }

    #[test]
    fn pcs_lab_encoding_round_trip() {
        let lab = CieLab {
            l: 50.0,
            a: 30.0,
            b: -20.0,
        };
        let encoded = float_to_pcs_encoded_lab(&lab);
        let back = pcs_encoded_lab_to_float(&encoded);
        // PCS encoding has limited precision
        assert!((back.l - lab.l).abs() < 0.01);
        assert!((back.a - lab.a).abs() < 0.01);
        assert!((back.b - lab.b).abs() < 0.01);
    }

    // ================================================================
    // Phase 13: V2 Lab encoding
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn pcs_lab_v2_encoding_round_trip() {
        let lab = CieLab {
            l: 50.0,
            a: 30.0,
            b: -20.0,
        };
        let encoded = float_to_pcs_encoded_lab_v2(&lab);
        let back = pcs_encoded_lab_to_float_v2(&encoded);
        assert!((back.l - lab.l).abs() < 0.01);
        assert!((back.a - lab.a).abs() < 0.01);
        assert!((back.b - lab.b).abs() < 0.01);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn pcs_lab_v2_encoding_white() {
        // L*=100 should encode to a value near (but not exactly) 0xFF00
        let lab = CieLab {
            l: 100.0,
            a: 0.0,
            b: 0.0,
        };
        let encoded = float_to_pcs_encoded_lab_v2(&lab);
        // V2: L * 652.8 = 65280 = 0xFF00
        assert_eq!(encoded[0], 0xFF00);
        // a=0, b=0 → (0+128)*256 = 32768 = 0x8000
        assert_eq!(encoded[1], 0x8000);
        assert_eq!(encoded[2], 0x8000);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn pcs_lab_v2_encoding_black() {
        let lab = CieLab {
            l: 0.0,
            a: 0.0,
            b: 0.0,
        };
        let encoded = float_to_pcs_encoded_lab_v2(&lab);
        assert_eq!(encoded[0], 0);
        assert_eq!(encoded[1], 0x8000);
        assert_eq!(encoded[2], 0x8000);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn pcs_lab_v2_differs_from_v4() {
        // V2 and V4 should produce different L* encodings (652.8 vs 655.35)
        let lab = CieLab {
            l: 50.0,
            a: 0.0,
            b: 0.0,
        };
        let v2 = float_to_pcs_encoded_lab_v2(&lab);
        let v4 = float_to_pcs_encoded_lab(&lab);
        assert_ne!(v2[0], v4[0], "V2 and V4 L* encoding should differ");
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn pcs_lab_v2_clamp_l_above_range() {
        // V2 max L = 0xFFFF * 100.0 / 0xFF00 ≈ 100.39
        // Values above this should clamp
        let lab = CieLab {
            l: 200.0,
            a: 0.0,
            b: 0.0,
        };
        let encoded = float_to_pcs_encoded_lab_v2(&lab);
        assert!(encoded[0] <= 0xFFFF);
    }

    // ================================================================
    // Phase 12: DeltaE extensions
    // ================================================================

    #[test]
    fn test_delta_e_cie94() {
        let lab1 = CieLab {
            l: 50.0,
            a: 2.6772,
            b: -79.7751,
        };
        let lab2 = CieLab {
            l: 50.0,
            a: 0.0,
            b: -82.7485,
        };
        let de = delta_e_cie94(&lab1, &lab2);
        // Reference: manually computed ≈ 1.4083
        assert!(
            (de - 1.4083).abs() < 0.01,
            "CIE94 expected ~1.4083, got {de}"
        );
    }

    #[test]
    fn test_delta_e_cie94_identical() {
        let lab = CieLab {
            l: 50.0,
            a: 25.0,
            b: -10.0,
        };
        let de = delta_e_cie94(&lab, &lab);
        assert!(de.abs() < 1e-10, "identical colors: dE={de}");
    }

    #[test]
    fn test_delta_e_bfd() {
        let lab1 = CieLab {
            l: 50.0,
            a: 2.6772,
            b: -79.7751,
        };
        let lab2 = CieLab {
            l: 50.0,
            a: 0.0,
            b: -82.7485,
        };
        let de = delta_e_bfd(&lab1, &lab2);
        // BFD should produce a positive, reasonable value for this pair
        assert!(de > 0.5, "BFD too low: {de}");
        assert!(de < 5.0, "BFD too high: {de}");
    }

    #[test]
    fn test_delta_e_cmc() {
        let lab1 = CieLab {
            l: 50.0,
            a: 25.0,
            b: -10.0,
        };
        let lab2 = CieLab {
            l: 55.0,
            a: 30.0,
            b: -15.0,
        };
        let de = delta_e_cmc(&lab1, &lab2, 1.0, 1.0);
        // Reference: manually computed ≈ 6.0311
        assert!(
            (de - 6.0311).abs() < 0.05,
            "CMC(1:1) expected ~6.0311, got {de}"
        );
    }

    #[test]
    fn test_delta_e_cmc_both_black() {
        let lab1 = CieLab {
            l: 0.0,
            a: 0.0,
            b: 0.0,
        };
        let lab2 = CieLab {
            l: 0.0,
            a: 0.0,
            b: 0.0,
        };
        let de = delta_e_cmc(&lab1, &lab2, 1.0, 1.0);
        assert_eq!(de, 0.0, "both black → dE=0");
    }

    #[test]
    fn test_delta_e_ciede2000() {
        // Reference pair from Sharma et al. (2005) Table 1, pair #1
        let lab1 = CieLab {
            l: 50.0,
            a: 2.6772,
            b: -79.7751,
        };
        let lab2 = CieLab {
            l: 50.0,
            a: 0.0,
            b: -82.7485,
        };
        let de = delta_e_ciede2000(&lab1, &lab2, 1.0, 1.0, 1.0);
        // Expected ≈ 2.0425
        assert!(
            (de - 2.0425).abs() < 0.005,
            "CIEDE2000 pair #1: expected ~2.0425, got {de}"
        );
    }

    #[test]
    fn test_delta_e_ciede2000_identical() {
        let lab = CieLab {
            l: 50.0,
            a: 25.0,
            b: -10.0,
        };
        let de = delta_e_ciede2000(&lab, &lab, 1.0, 1.0, 1.0);
        assert!(de.abs() < 1e-10, "identical colors: dE={de}");
    }

    #[test]
    fn test_delta_e_ciede2000_monotonic() {
        // Larger Lab differences → larger CIEDE2000
        let ref_lab = CieLab {
            l: 50.0,
            a: 0.0,
            b: 0.0,
        };
        let near = CieLab {
            l: 52.0,
            a: 1.0,
            b: -1.0,
        };
        let far = CieLab {
            l: 60.0,
            a: 20.0,
            b: -30.0,
        };
        let de_near = delta_e_ciede2000(&ref_lab, &near, 1.0, 1.0, 1.0);
        let de_far = delta_e_ciede2000(&ref_lab, &far, 1.0, 1.0, 1.0);
        assert!(
            de_far > de_near,
            "far color should have larger dE: near={de_near}, far={de_far}"
        );
    }
}
