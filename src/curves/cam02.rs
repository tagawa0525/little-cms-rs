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
#[derive(Default, Clone)]
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

// CAT02 matrix (XYZ → RGB)
const CAT02: [[f64; 3]; 3] = [
    [0.7328, 0.4296, -0.1624],
    [-0.7036, 1.6975, 0.0061],
    [0.0030, 0.0136, 0.9834],
];

// CAT02 inverse matrix (RGB → XYZ)
const CAT02_INV: [[f64; 3]; 3] = [
    [1.096124, -0.278869, 0.182745],
    [0.454369, 0.473533, 0.072098],
    [-0.009628, -0.005698, 1.015326],
];

// HPE matrix coefficients (computed as M_HPE × M_CAT02_INV)
fn hpe_matrix() -> [[f64; 3]; 3] {
    // M_HPE (Hunt-Pointer-Estévez) base matrix
    let hpe = [
        [0.38971, 0.68898, -0.07868],
        [-0.22981, 1.18340, 0.04641],
        [0.00000, 0.00000, 1.00000],
    ];
    // Compute HPE × CAT02_INV
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                result[i][j] += hpe[i][k] * CAT02_INV[k][j];
            }
        }
    }
    result
}

// HPE inverse matrix coefficients (computed as CAT02 × M_HPE_INV)
fn hpe_inv_matrix() -> [[f64; 3]; 3] {
    // M_HPE inverse
    let hpe_inv = [
        [1.910197, -1.112124, 0.201908],
        [0.370950, 0.629054, 0.000008],
        [0.000000, 0.000000, 1.000000],
    ];
    // Compute CAT02 × HPE_INV
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                result[i][j] += CAT02[i][k] * hpe_inv[k][j];
            }
        }
    }
    result
}

fn mat3_eval(m: &[[f64; 3]; 3], v: &[f64; 3]) -> [f64; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

fn xyz_to_cat02(color: &mut Cam02Color) {
    color.rgb = mat3_eval(&CAT02, &color.xyz);
}

fn chromatic_adaptation(color: &mut Cam02Color, white: &Cam02Color, d: f64) {
    for i in 0..3 {
        color.rgb_c[i] = ((white.xyz[1] * d / white.rgb[i]) + (1.0 - d)) * color.rgb[i];
    }
}

fn cat02_to_hpe(color: &mut Cam02Color) {
    let m = hpe_matrix();
    color.rgb_p = mat3_eval(&m, &color.rgb_c);
}

fn nonlinear_compression(color: &mut Cam02Color, fl: f64, nbb: f64) {
    for i in 0..3 {
        if color.rgb_p[i] < 0.0 {
            let temp = (-fl * color.rgb_p[i] / 100.0).powf(0.42);
            color.rgb_pa[i] = -400.0 * temp / (temp + 27.13) + 0.1;
        } else {
            let temp = (fl * color.rgb_p[i] / 100.0).powf(0.42);
            color.rgb_pa[i] = 400.0 * temp / (temp + 27.13) + 0.1;
        }
    }

    color.big_a =
        ((2.0 * color.rgb_pa[0] + color.rgb_pa[1] + color.rgb_pa[2] / 20.0) - 0.305) * nbb;
}

fn compute_correlates(color: &mut Cam02Color, model: &CieCam02) {
    // a and b opponent dimensions
    color.a = color.rgb_pa[0] - 12.0 * color.rgb_pa[1] / 11.0 + color.rgb_pa[2] / 11.0;
    color.b = (color.rgb_pa[0] + color.rgb_pa[1] - 2.0 * color.rgb_pa[2]) / 9.0;

    // Hue angle
    color.h = color.b.atan2(color.a).to_degrees();
    if color.h < 0.0 {
        color.h += 360.0;
    }

    // Eccentricity
    let e = (12500.0 / 13.0) * model.nc * model.ncb * ((color.h + 2.0).to_radians().cos() + 3.8);

    // Lightness J
    let a_white = model.adopted_white.big_a;
    if a_white.abs() > 1e-20 {
        color.j = 100.0 * (color.big_a / a_white).powf(model.c * model.z);
    } else {
        color.j = 0.0;
    }

    // t factor (saturation temp)
    let denom = color.rgb_pa[0] + color.rgb_pa[1] + 1.05 * color.rgb_pa[2];
    let t = if denom.abs() > 1e-20 {
        e * (color.a * color.a + color.b * color.b).sqrt() / denom
    } else {
        0.0
    };

    // Chroma C
    color.c = t.powf(0.9) * (color.j / 100.0).sqrt() * (1.64 - 0.29f64.powf(model.n)).powf(0.73);
}

fn inverse_correlates(color: &mut Cam02Color, model: &CieCam02) {
    let j = color.j;
    let c = color.c;
    let h = color.h;

    // t from C
    let j100 = (j / 100.0).sqrt();
    let t = if j100.abs() > 1e-20 {
        let base = c / (j100 * (1.64 - 0.29f64.powf(model.n)).powf(0.73));
        if base > 0.0 {
            base.powf(1.0 / 0.9)
        } else {
            0.0
        }
    } else {
        0.0
    };

    // e from h
    let e = (12500.0 / 13.0) * model.nc * model.ncb * ((h + 2.0).to_radians().cos() + 3.8);

    // A from J
    let a_white = model.adopted_white.big_a;
    let big_a = if model.c.abs() > 1e-20 && model.z.abs() > 1e-20 {
        a_white * (j / 100.0).powf(1.0 / (model.c * model.z))
    } else {
        0.0
    };

    let p2 = big_a / model.nbb + 0.305;

    if t.abs() < 1e-20 {
        color.a = 0.0;
        color.b = 0.0;
    } else {
        let hr = h.to_radians();
        let sin_h = hr.sin();
        let cos_h = hr.cos();
        let p1 = e / t;
        let p3 = 21.0 / 20.0;

        if sin_h.abs() >= cos_h.abs() {
            let p4 = p1 / sin_h;
            color.b = p2 * (2.0 + p3) * (460.0 / 1403.0)
                / (p4 + (2.0 + p3) * (220.0 / 1403.0) * (cos_h / sin_h) - 27.0 / 1403.0
                    + p3 * (6300.0 / 1403.0));
            color.a = color.b * (cos_h / sin_h);
        } else {
            let p5 = p1 / cos_h;
            color.a = p2 * (2.0 + p3) * (460.0 / 1403.0)
                / (p5 + (2.0 + p3) * (220.0 / 1403.0)
                    - (27.0 / 1403.0 - p3 * (6300.0 / 1403.0)) * (sin_h / cos_h));
            color.b = color.a * (sin_h / cos_h);
        }
    }

    // RGBpa from a, b, p2
    color.rgb_pa[0] =
        (460.0 / 1403.0) * p2 + (451.0 / 1403.0) * color.a + (288.0 / 1403.0) * color.b;
    color.rgb_pa[1] =
        (460.0 / 1403.0) * p2 - (891.0 / 1403.0) * color.a - (261.0 / 1403.0) * color.b;
    color.rgb_pa[2] =
        (460.0 / 1403.0) * p2 - (220.0 / 1403.0) * color.a - (6300.0 / 1403.0) * color.b;
}

fn inverse_nonlinearity(color: &mut Cam02Color, fl: f64) {
    for i in 0..3 {
        let c1 = if color.rgb_pa[i] - 0.1 >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let abs_val = (color.rgb_pa[i] - 0.1).abs();
        let denom = 400.0 - abs_val;
        if denom.abs() > 1e-20 {
            color.rgb_p[i] = c1 * (100.0 / fl) * (27.13 * abs_val / denom).powf(1.0 / 0.42);
        } else {
            color.rgb_p[i] = 0.0;
        }
    }
}

fn hpe_to_cat02(color: &mut Cam02Color) {
    let m = hpe_inv_matrix();
    color.rgb_c = mat3_eval(&m, &color.rgb_p);
}

fn inverse_chromatic_adaptation(color: &mut Cam02Color, white: &Cam02Color, d: f64) {
    for i in 0..3 {
        let factor = (white.xyz[1] * d / white.rgb[i]) + (1.0 - d);
        if factor.abs() > 1e-20 {
            color.rgb[i] = color.rgb_c[i] / factor;
        } else {
            color.rgb[i] = 0.0;
        }
    }
}

fn cat02_to_xyz(color: &mut Cam02Color) {
    color.xyz = mat3_eval(&CAT02_INV, &color.rgb);
}

impl CieCam02 {
    /// Initialize CIECAM02 model from viewing conditions.
    ///
    /// C版: `cmsCIECAM02Init`
    #[allow(dead_code)]
    pub fn new(vc: &ViewingConditions) -> Self {
        let la = vc.la;
        let yb = vc.yb;

        // Surround-dependent parameters
        let (f, c, nc) = match vc.surround {
            Surround::Cutsheet => (0.8, 0.41, 0.8),
            Surround::Dark => (0.8, 0.525, 0.8),
            Surround::Dim => (0.9, 0.59, 0.95),
            Surround::Average => (1.0, 0.69, 1.0),
        };

        let y_white = vc.white_point.y;

        // n: relative luminance of background
        let n = yb / y_white;
        // z: adaptation factor
        let z = 1.48 + n.sqrt();
        // Nbb: cone response reduction
        let nbb = 0.725 / n.powf(0.2);
        let ncb = nbb;

        // FL: luminance adaptation
        let k = 1.0 / (5.0 * la + 1.0);
        let k4 = k * k * k * k;
        let fl = 0.2 * k4 * (5.0 * la) + 0.1 * (1.0 - k4) * (1.0 - k4) * (5.0 * la).cbrt();

        // D: degree of chromatic adaptation
        let d = if vc.d_value == D_CALCULATE {
            (f * (1.0 - (1.0 / 3.6) * (-(la - 42.0) / 92.0).exp())).clamp(0.0, 1.0)
        } else {
            vc.d_value
        };

        // Transform adopted white through forward pipeline (without correlates)
        let mut white = Cam02Color {
            xyz: [vc.white_point.x, vc.white_point.y, vc.white_point.z],
            ..Default::default()
        };
        xyz_to_cat02(&mut white);
        // Self-adaptation: RGBc[i] = (Y*D/RGB[i] + 1-D) * RGB[i]
        for i in 0..3 {
            white.rgb_c[i] = ((white.xyz[1] * d / white.rgb[i]) + (1.0 - d)) * white.rgb[i];
        }
        cat02_to_hpe(&mut white);
        nonlinear_compression(&mut white, fl, nbb);

        CieCam02 {
            adopted_white: white,
            la,
            yb,
            f,
            c,
            nc,
            n,
            nbb,
            ncb,
            z,
            fl,
            d,
        }
    }

    /// Forward transform: XYZ → JCh.
    ///
    /// C版: `cmsCIECAM02Forward`
    #[allow(dead_code)]
    pub fn forward(&self, xyz: &CieXyz) -> JCh {
        let mut color = Cam02Color {
            xyz: [xyz.x, xyz.y, xyz.z],
            ..Default::default()
        };

        xyz_to_cat02(&mut color);
        chromatic_adaptation(&mut color, &self.adopted_white, self.d);
        cat02_to_hpe(&mut color);
        nonlinear_compression(&mut color, self.fl, self.nbb);
        compute_correlates(&mut color, self);

        JCh {
            j: color.j,
            c: color.c,
            h: color.h,
        }
    }

    /// Reverse transform: JCh → XYZ.
    ///
    /// C版: `cmsCIECAM02Reverse`
    #[allow(dead_code)]
    pub fn reverse(&self, jch: &JCh) -> CieXyz {
        let mut color = Cam02Color {
            j: jch.j,
            c: jch.c,
            h: jch.h,
            ..Default::default()
        };

        inverse_correlates(&mut color, self);
        inverse_nonlinearity(&mut color, self.fl);
        hpe_to_cat02(&mut color);
        inverse_chromatic_adaptation(&mut color, &self.adopted_white, self.d);
        cat02_to_xyz(&mut color);

        CieXyz {
            x: color.xyz[0],
            y: color.xyz[1],
            z: color.xyz[2],
        }
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
