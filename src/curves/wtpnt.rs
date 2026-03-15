//! White point, color temperature, and chromatic adaptation.
//!
//! C版対応: `cmswtpnt.c`

use crate::math::mtrx::{Mat3, Vec3};
use crate::math::pcs;
use crate::types::{CieXyY, CieXyYTriple, CieXyz, D50_X, D50_Y, D50_Z};

/// Bradford cone response matrix (LamRigg).
pub const BRADFORD: Mat3 = Mat3([
    Vec3([0.8951, 0.2664, -0.1614]),
    Vec3([-0.7502, 1.7135, 0.0367]),
    Vec3([0.0389, -0.0685, 1.0296]),
]);

/// Near-zero threshold for cone response components.
const MATRIX_DET_TOLERANCE: f64 = 0.0001;

/// Robertson isotemperature data entry.
struct IsoTemp {
    mirek: f64, // microreciprocal kelvin
    ut: f64,    // CIE 1960 u coordinate
    vt: f64,    // CIE 1960 v coordinate
    tt: f64,    // slope of isotemperature line
}

/// Robertson isotemperature data table (31 entries).
const ISOTEMPERATURE: [IsoTemp; 31] = [
    IsoTemp {
        mirek: 0.0,
        ut: 0.18006,
        vt: 0.26352,
        tt: -0.24341,
    },
    IsoTemp {
        mirek: 10.0,
        ut: 0.18066,
        vt: 0.26589,
        tt: -0.25479,
    },
    IsoTemp {
        mirek: 20.0,
        ut: 0.18133,
        vt: 0.26846,
        tt: -0.26876,
    },
    IsoTemp {
        mirek: 30.0,
        ut: 0.18208,
        vt: 0.27119,
        tt: -0.28539,
    },
    IsoTemp {
        mirek: 40.0,
        ut: 0.18293,
        vt: 0.27407,
        tt: -0.30470,
    },
    IsoTemp {
        mirek: 50.0,
        ut: 0.18388,
        vt: 0.27709,
        tt: -0.32675,
    },
    IsoTemp {
        mirek: 60.0,
        ut: 0.18494,
        vt: 0.28021,
        tt: -0.35156,
    },
    IsoTemp {
        mirek: 70.0,
        ut: 0.18611,
        vt: 0.28342,
        tt: -0.37915,
    },
    IsoTemp {
        mirek: 80.0,
        ut: 0.18740,
        vt: 0.28668,
        tt: -0.40955,
    },
    IsoTemp {
        mirek: 90.0,
        ut: 0.18880,
        vt: 0.28997,
        tt: -0.44278,
    },
    IsoTemp {
        mirek: 100.0,
        ut: 0.19032,
        vt: 0.29326,
        tt: -0.47888,
    },
    IsoTemp {
        mirek: 125.0,
        ut: 0.19462,
        vt: 0.30141,
        tt: -0.58204,
    },
    IsoTemp {
        mirek: 150.0,
        ut: 0.19962,
        vt: 0.30921,
        tt: -0.70471,
    },
    IsoTemp {
        mirek: 175.0,
        ut: 0.20525,
        vt: 0.31647,
        tt: -0.84901,
    },
    IsoTemp {
        mirek: 200.0,
        ut: 0.21142,
        vt: 0.32312,
        tt: -1.0182,
    },
    IsoTemp {
        mirek: 225.0,
        ut: 0.21807,
        vt: 0.32909,
        tt: -1.2168,
    },
    IsoTemp {
        mirek: 250.0,
        ut: 0.22511,
        vt: 0.33439,
        tt: -1.4512,
    },
    IsoTemp {
        mirek: 275.0,
        ut: 0.23247,
        vt: 0.33904,
        tt: -1.7298,
    },
    IsoTemp {
        mirek: 300.0,
        ut: 0.24010,
        vt: 0.34308,
        tt: -2.0637,
    },
    IsoTemp {
        mirek: 325.0,
        ut: 0.24702,
        vt: 0.34655,
        tt: -2.4681,
    },
    IsoTemp {
        mirek: 350.0,
        ut: 0.25591,
        vt: 0.34951,
        tt: -2.9641,
    },
    IsoTemp {
        mirek: 375.0,
        ut: 0.26400,
        vt: 0.35200,
        tt: -3.5814,
    },
    IsoTemp {
        mirek: 400.0,
        ut: 0.27218,
        vt: 0.35407,
        tt: -4.3633,
    },
    IsoTemp {
        mirek: 425.0,
        ut: 0.28039,
        vt: 0.35577,
        tt: -5.3762,
    },
    IsoTemp {
        mirek: 450.0,
        ut: 0.28863,
        vt: 0.35714,
        tt: -6.7262,
    },
    IsoTemp {
        mirek: 475.0,
        ut: 0.29685,
        vt: 0.35823,
        tt: -8.5955,
    },
    IsoTemp {
        mirek: 500.0,
        ut: 0.30505,
        vt: 0.35907,
        tt: -11.324,
    },
    IsoTemp {
        mirek: 525.0,
        ut: 0.31320,
        vt: 0.35968,
        tt: -15.628,
    },
    IsoTemp {
        mirek: 550.0,
        ut: 0.32129,
        vt: 0.36011,
        tt: -23.325,
    },
    IsoTemp {
        mirek: 575.0,
        ut: 0.32931,
        vt: 0.36038,
        tt: -40.770,
    },
    IsoTemp {
        mirek: 600.0,
        ut: 0.33724,
        vt: 0.36051,
        tt: -116.45,
    },
];

/// D50 white point in XYZ.
#[allow(dead_code)]
pub fn d50_xyz() -> CieXyz {
    CieXyz {
        x: D50_X,
        y: D50_Y,
        z: D50_Z,
    }
}

/// D50 white point in xyY.
#[allow(dead_code)]
pub fn d50_xyy() -> CieXyY {
    pcs::xyz_to_xyy(&d50_xyz())
}

/// Compute white point chromaticity from correlated color temperature.
///
/// Valid range: 4000K–25000K.
///
/// C版: `cmsWhitePointFromTemp`
#[allow(dead_code)]
pub fn white_point_from_temp(temp_k: f64) -> Option<CieXyY> {
    if !(4000.0..=25000.0).contains(&temp_k) {
        return None;
    }

    let t = temp_k;
    let t2 = t * t;
    let t3 = t2 * t;

    // Calculate x chromaticity
    let x = if t <= 7000.0 {
        -4.6070e9 / t3 + 2.9678e6 / t2 + 0.09911e3 / t + 0.244063
    } else {
        -2.0064e9 / t3 + 1.9018e6 / t2 + 0.24748e3 / t + 0.237040
    };

    // Calculate y from x
    let y = -3.000 * x * x + 2.870 * x - 0.275;

    Some(CieXyY { x, y, big_y: 1.0 })
}

/// Compute correlated color temperature from white point chromaticity.
///
/// Uses Robertson's method with isotemperature data.
///
/// C版: `cmsTempFromWhitePoint`
#[allow(dead_code)]
pub fn temp_from_white_point(wp: &CieXyY) -> Option<f64> {
    // Convert CIE xyY to CIE 1960 u,v
    let denom = -wp.x + 6.0 * wp.y + 1.5;
    if denom.abs() < 1e-20 {
        return None;
    }
    let us = 2.0 * wp.x / denom;
    let vs = 3.0 * wp.y / denom;

    let mut last_d = 0.0f64;

    for j in 1..ISOTEMPERATURE.len() {
        let iso = &ISOTEMPERATURE[j];
        let di = ((vs - iso.vt) - iso.tt * (us - iso.ut)) / (1.0 + iso.tt * iso.tt).sqrt();

        if j == 1 {
            last_d = {
                let iso0 = &ISOTEMPERATURE[0];
                ((vs - iso0.vt) - iso0.tt * (us - iso0.ut)) / (1.0 + iso0.tt * iso0.tt).sqrt()
            };
        }

        if di * last_d < 0.0 {
            // Sign change — interpolate
            let mi = ISOTEMPERATURE[j - 1].mirek;
            let mj = iso.mirek;
            let t = last_d / (last_d - di);
            let mirek = mi + t * (mj - mi);
            if mirek.abs() < 1e-20 {
                return None;
            }
            return Some(1_000_000.0 / mirek);
        }

        last_d = di;
    }

    None
}

/// Compute chromatic adaptation matrix using Bradford (or custom cone) matrix.
///
/// If `cone` is `None`, uses the Bradford matrix.
///
/// C版: `_cmsAdaptationMatrix`
#[allow(dead_code)]
pub fn adaptation_matrix(cone: Option<&Mat3>, from: &CieXyz, to: &CieXyz) -> Option<Mat3> {
    let chad = cone.unwrap_or(&BRADFORD);
    let chad_inv = chad.inverse()?;

    // Transform illuminants to cone response space
    let from_v = Vec3::new(from.x, from.y, from.z);
    let to_v = Vec3::new(to.x, to.y, to.z);

    let cone_src = chad.eval(&from_v);
    let cone_dst = chad.eval(&to_v);

    // Check for near-zero components
    if cone_src.0[0].abs() < MATRIX_DET_TOLERANCE
        || cone_src.0[1].abs() < MATRIX_DET_TOLERANCE
        || cone_src.0[2].abs() < MATRIX_DET_TOLERANCE
    {
        return None;
    }

    // Diagonal scaling matrix
    let scale = Mat3([
        Vec3([cone_dst.0[0] / cone_src.0[0], 0.0, 0.0]),
        Vec3([0.0, cone_dst.0[1] / cone_src.0[1], 0.0]),
        Vec3([0.0, 0.0, cone_dst.0[2] / cone_src.0[2]]),
    ]);

    // Result = chad_inv * scale * chad
    let tmp = scale * *chad;
    Some(chad_inv * tmp)
}

/// Adapt a matrix from a source white point to D50.
fn adapt_matrix_to_d50(m: &Mat3, source_wp: &CieXyY) -> Option<Mat3> {
    let wp_xyz = pcs::xyy_to_xyz(source_wp);
    let d50 = d50_xyz();
    let adapt = adaptation_matrix(None, &wp_xyz, &d50)?;
    Some(adapt * *m)
}

/// Build RGB-to-XYZ transfer matrix from white point and primaries.
///
/// C版: `_cmsBuildRGB2XYZtransferMatrix`
#[allow(dead_code)]
pub fn build_rgb_to_xyz_matrix(wp: &CieXyY, primaries: &CieXyYTriple) -> Option<Mat3> {
    let xr = primaries.red.x;
    let yr = primaries.red.y;
    let xg = primaries.green.x;
    let yg = primaries.green.y;
    let xb = primaries.blue.x;
    let yb = primaries.blue.y;

    // Build primaries matrix
    let primaries_mat = Mat3([
        Vec3([xr, xg, xb]),
        Vec3([yr, yg, yb]),
        Vec3([1.0 - xr - yr, 1.0 - xg - yg, 1.0 - xb - yb]),
    ]);

    let inv = primaries_mat.inverse()?;

    // White point in normalized XYZ
    let xn = wp.x;
    let yn = wp.y;
    let white_xyz = Vec3::new(xn / yn, 1.0, (1.0 - xn - yn) / yn);

    // Scaling coefficients
    let coef = inv.eval(&white_xyz);

    // Scale primaries
    let result = Mat3([
        Vec3([coef.0[0] * xr, coef.0[1] * xg, coef.0[2] * xb]),
        Vec3([coef.0[0] * yr, coef.0[1] * yg, coef.0[2] * yb]),
        Vec3([
            coef.0[0] * (1.0 - xr - yr),
            coef.0[1] * (1.0 - xg - yg),
            coef.0[2] * (1.0 - xb - yb),
        ]),
    ]);

    // Adapt to D50
    adapt_matrix_to_d50(&result, wp)
}

/// Adapt a color from one illuminant to another using Bradford adaptation.
///
/// C版: `cmsAdaptToIlluminant`
#[allow(dead_code)]
pub fn adapt_to_illuminant(src_wp: &CieXyz, illuminant: &CieXyz, value: &CieXyz) -> Option<CieXyz> {
    let m = adaptation_matrix(None, src_wp, illuminant)?;
    let v = Vec3::new(value.x, value.y, value.z);
    let result = m.eval(&v);
    Some(CieXyz {
        x: result.0[0],
        y: result.0[1],
        z: result.0[2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-4;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    // ========================================================================
    // Color temperature round-trip
    // ========================================================================

    #[test]
    fn temp_round_trip() {
        let temps = [
            4000.0, 5000.0, 5500.0, 6000.0, 6500.0, 7000.0, 8000.0, 10000.0, 15000.0, 25000.0,
        ];
        for &t in &temps {
            let wp = white_point_from_temp(t).unwrap();
            let t_back = temp_from_white_point(&wp).unwrap();
            // Robertson method has limited precision; allow ~0.5% relative error
            let tol = t * 0.005;
            assert!((t_back - t).abs() < tol, "T={t}: got back {t_back}");
        }
    }

    #[test]
    fn temp_out_of_range() {
        assert!(white_point_from_temp(3999.0).is_none());
        assert!(white_point_from_temp(25001.0).is_none());
    }

    // ========================================================================
    // Bradford chromatic adaptation
    // ========================================================================

    #[test]
    fn adaptation_d65_to_d50() {
        // D65 white point
        let d65 = CieXyz {
            x: 0.9505,
            y: 1.0,
            z: 1.0890,
        };
        let d50 = d50_xyz();
        let m = adaptation_matrix(None, &d65, &d50).unwrap();

        // Apply to D65 itself — should yield D50
        let v = Vec3::new(d65.x, d65.y, d65.z);
        let result = m.eval(&v);
        assert!(close(result.0[0], d50.x), "X: {} vs {}", result.0[0], d50.x);
        assert!(close(result.0[1], d50.y), "Y: {} vs {}", result.0[1], d50.y);
        assert!(close(result.0[2], d50.z), "Z: {} vs {}", result.0[2], d50.z);
    }

    #[test]
    fn adaptation_same_white_is_identity() {
        let d50 = d50_xyz();
        let m = adaptation_matrix(None, &d50, &d50).unwrap();
        assert!(m.is_identity());
    }

    // ========================================================================
    // RGB to XYZ matrix
    // ========================================================================

    #[test]
    fn rgb_to_xyz_srgb() {
        // sRGB primaries (CIE 1931 2°)
        let wp = CieXyY {
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
        let m = build_rgb_to_xyz_matrix(&wp, &primaries).unwrap();

        // The matrix should transform [1,1,1] (white) close to D50
        let white = Vec3::new(1.0, 1.0, 1.0);
        let xyz = m.eval(&white);
        // Since the function adapts to D50, white should map near D50
        assert!(
            close(xyz.0[1], 1.0),
            "Y for white: {} (expected ~1.0)",
            xyz.0[1]
        );
    }

    // ========================================================================
    // Adapt to illuminant
    // ========================================================================

    #[test]
    fn adapt_to_illuminant_d50_same() {
        let d50 = d50_xyz();
        // Adapting D50 white under D50 to D50 should return D50
        let result = adapt_to_illuminant(&d50, &d50, &d50).unwrap();
        assert!(close(result.x, d50.x));
        assert!(close(result.y, d50.y));
        assert!(close(result.z, d50.z));
    }
}
