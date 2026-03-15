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
pub fn white_point_from_temp(_temp_k: f64) -> Option<CieXyY> {
    todo!()
}

/// Compute correlated color temperature from white point chromaticity.
///
/// Uses Robertson's method with isotemperature data.
///
/// C版: `cmsTempFromWhitePoint`
#[allow(dead_code)]
pub fn temp_from_white_point(_wp: &CieXyY) -> Option<f64> {
    todo!()
}

/// Compute chromatic adaptation matrix using Bradford (or custom cone) matrix.
///
/// If `cone` is `None`, uses the Bradford matrix.
///
/// C版: `_cmsAdaptationMatrix`
#[allow(dead_code)]
pub fn adaptation_matrix(_cone: Option<&Mat3>, _from: &CieXyz, _to: &CieXyz) -> Option<Mat3> {
    todo!()
}

/// Build RGB-to-XYZ transfer matrix from white point and primaries.
///
/// C版: `_cmsBuildRGB2XYZtransferMatrix`
#[allow(dead_code)]
pub fn build_rgb_to_xyz_matrix(_wp: &CieXyY, _primaries: &CieXyYTriple) -> Option<Mat3> {
    todo!()
}

/// Adapt a color from one illuminant to another using Bradford adaptation.
///
/// C版: `cmsAdaptToIlluminant`
#[allow(dead_code)]
pub fn adapt_to_illuminant(
    _src_wp: &CieXyz,
    _illuminant: &CieXyz,
    _value: &CieXyz,
) -> Option<CieXyz> {
    todo!()
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
    #[ignore = "not yet implemented"]
    fn temp_round_trip() {
        let temps = [
            4000.0, 5000.0, 5500.0, 6000.0, 6500.0, 7000.0, 8000.0, 10000.0, 15000.0, 25000.0,
        ];
        for &t in &temps {
            let wp = white_point_from_temp(t).unwrap();
            let t_back = temp_from_white_point(&wp).unwrap();
            assert!((t_back - t).abs() < 0.5, "T={t}: got back {t_back}");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn temp_out_of_range() {
        assert!(white_point_from_temp(3999.0).is_none());
        assert!(white_point_from_temp(25001.0).is_none());
    }

    // ========================================================================
    // Bradford chromatic adaptation
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn adaptation_same_white_is_identity() {
        let d50 = d50_xyz();
        let m = adaptation_matrix(None, &d50, &d50).unwrap();
        assert!(m.is_identity());
    }

    // ========================================================================
    // RGB to XYZ matrix
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn adapt_to_illuminant_d50_same() {
        let d50 = d50_xyz();
        // Adapting D50 white under D50 to D50 should return D50
        let result = adapt_to_illuminant(&d50, &d50, &d50).unwrap();
        assert!(close(result.x, d50.x));
        assert!(close(result.y, d50.y));
        assert!(close(result.z, d50.z));
    }
}
