//! CIECAM02 color appearance model.
//!
//! C版対応: `cmscam02.c`

use crate::types::{CieXyz, JCh};

/// Surround condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Surround {
    Average = 1,
    Dim = 2,
    Dark = 3,
    Cutsheet = 4,
}

/// Sentinel: auto-calculate D from La.
pub const D_CALCULATE: f64 = -1.0;

/// Viewing conditions for CIECAM02 initialization.
pub struct ViewingConditions {
    pub white_point: CieXyz,
    pub yb: f64,
    pub la: f64,
    pub surround: Surround,
    pub d_value: f64,
}

/// Pre-computed CIECAM02 model.
#[allow(dead_code)]
pub struct CieCam02 {
    adopted_white: Cam02Color,
    la: f64,
    yb: f64,
    f: f64,
    c: f64,
    nc: f64,
    n: f64,
    nbb: f64,
    ncb: f64,
    z: f64,
    fl: f64,
    d: f64,
}

/// Internal color representation through the CIECAM02 pipeline.
#[allow(dead_code)]
#[derive(Default)]
struct Cam02Color {
    xyz: [f64; 3],
    rgb: [f64; 3],
    rgb_c: [f64; 3],
    rgb_p: [f64; 3],
    rgb_pa: [f64; 3],
    a: f64,
    b: f64,
    h: f64,
    big_a: f64,
    j: f64,
    c: f64,
}

impl CieCam02 {
    /// Initialize CIECAM02 model from viewing conditions.
    ///
    /// C版: `cmsCIECAM02Init`
    #[allow(dead_code)]
    pub fn new(_vc: &ViewingConditions) -> Self {
        todo!()
    }

    /// Forward transform: XYZ → JCh.
    ///
    /// C版: `cmsCIECAM02Forward`
    #[allow(dead_code)]
    pub fn forward(&self, _xyz: &CieXyz) -> JCh {
        todo!()
    }

    /// Reverse transform: JCh → XYZ.
    ///
    /// C版: `cmsCIECAM02Reverse`
    #[allow(dead_code)]
    pub fn reverse(&self, _jch: &JCh) -> CieXyz {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{D50_X, D50_Y, D50_Z};

    fn standard_vc() -> ViewingConditions {
        ViewingConditions {
            white_point: CieXyz {
                x: D50_X,
                y: D50_Y,
                z: D50_Z,
            },
            yb: 20.0,
            la: 200.0,
            surround: Surround::Average,
            d_value: D_CALCULATE,
        }
    }

    // ========================================================================
    // Forward-reverse round-trip
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn round_trip_multiple_colors() {
        let model = CieCam02::new(&standard_vc());
        let colors = [
            CieXyz {
                x: 0.4,
                y: 0.3,
                z: 0.2,
            },
            CieXyz {
                x: 0.2,
                y: 0.5,
                z: 0.1,
            },
            CieXyz {
                x: 0.1,
                y: 0.1,
                z: 0.3,
            },
            CieXyz {
                x: 0.8,
                y: 0.9,
                z: 0.7,
            },
        ];
        for xyz in &colors {
            let jch = model.forward(xyz);
            let back = model.reverse(&jch);
            assert!(
                (back.x - xyz.x).abs() < 1e-4
                    && (back.y - xyz.y).abs() < 1e-4
                    && (back.z - xyz.z).abs() < 1e-4,
                "round-trip failed for {:?}: got {:?}",
                xyz,
                back
            );
        }
    }

    // ========================================================================
    // D50 white point: J≈100, C≈0
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn d50_white_forward() {
        let model = CieCam02::new(&standard_vc());
        let wp = CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        };
        let jch = model.forward(&wp);
        assert!(
            (jch.j - 100.0).abs() < 0.5,
            "J for D50 white: {} (expected ~100)",
            jch.j
        );
        assert!(jch.c < 2.0, "C for D50 white: {} (expected ~0)", jch.c);
    }

    // ========================================================================
    // Different surround conditions produce different J
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn surround_affects_lightness() {
        let xyz = CieXyz {
            x: 0.4,
            y: 0.3,
            z: 0.2,
        };

        let surrounds = [
            Surround::Average,
            Surround::Dim,
            Surround::Dark,
            Surround::Cutsheet,
        ];
        let mut j_values = Vec::new();

        for &s in &surrounds {
            let vc = ViewingConditions {
                white_point: CieXyz {
                    x: D50_X,
                    y: D50_Y,
                    z: D50_Z,
                },
                yb: 20.0,
                la: 200.0,
                surround: s,
                d_value: D_CALCULATE,
            };
            let model = CieCam02::new(&vc);
            let jch = model.forward(&xyz);
            j_values.push(jch.j);
        }

        // Different surrounds should produce different J values
        // (at least some pairs should differ)
        let all_same = j_values.windows(2).all(|w| (w[0] - w[1]).abs() < 0.01);
        assert!(
            !all_same,
            "All surround conditions gave same J: {:?}",
            j_values
        );
    }

    // ========================================================================
    // D_CALCULATE: D auto-computed from La
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn d_calculate_auto() {
        // D_CALCULATE should not panic and should produce valid output
        let model = CieCam02::new(&standard_vc());
        let xyz = CieXyz {
            x: 0.4,
            y: 0.3,
            z: 0.2,
        };
        let jch = model.forward(&xyz);
        assert!(jch.j > 0.0 && jch.j < 100.0, "J out of range: {}", jch.j);
    }

    // ========================================================================
    // Boundary: near-zero XYZ should not panic
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn near_zero_xyz_no_panic() {
        let model = CieCam02::new(&standard_vc());
        let dark = CieXyz {
            x: 0.001,
            y: 0.001,
            z: 0.001,
        };
        let jch = model.forward(&dark);
        assert!(jch.j.is_finite(), "J is not finite for near-zero XYZ");
        assert!(jch.c.is_finite(), "C is not finite for near-zero XYZ");
        assert!(jch.h.is_finite(), "h is not finite for near-zero XYZ");
    }
}
