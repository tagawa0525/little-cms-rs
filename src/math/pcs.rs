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
}
