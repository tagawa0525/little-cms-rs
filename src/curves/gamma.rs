//! Tone curve (gamma) engine.
//!
//! C版対応: `cmsgamma.c`
//!
//! Provides parametric, tabulated, and segmented tone curves with both
//! float (segment-based) and 16-bit (table-based) evaluation paths.

use super::intrp::InterpParams;

/// Sentinel for unbounded lower segment boundary.
#[allow(dead_code)]
const MINUS_INF: f32 = -1e22;
/// Sentinel for unbounded upper segment boundary.
#[allow(dead_code)]
const PLUS_INF: f32 = 1e22;
/// Near-zero threshold for division guards.
#[allow(dead_code)]
const TOLERANCE: f64 = 0.0001;
/// Default table size for Table16.
#[allow(dead_code)]
const DEFAULT_TABLE_SIZE: u32 = 4096;
/// Maximum allowed table entries.
#[allow(dead_code)]
const MAX_TABLE_ENTRIES: u32 = 65530;

/// A segment of a tone curve (parametric or sampled).
#[derive(Clone, Debug)]
pub struct CurveSegment {
    pub x0: f32,
    pub x1: f32,
    /// Positive = parametric type, 0 = sampled, negative = inverse.
    pub curve_type: i32,
    pub params: [f64; 10],
    /// Sampled points (only when curve_type == 0).
    pub sampled_points: Vec<f32>,
}

/// Tone curve: segment representation (float precision) + 16-bit lookup table.
#[derive(Clone)]
#[allow(dead_code)]
pub struct ToneCurve {
    segments: Vec<CurveSegment>,
    seg_interp: Vec<Option<InterpParams>>,
    table16: Vec<u16>,
    interp_params: InterpParams,
    n_entries: u32,
}

impl ToneCurve {
    /// Build a simple gamma curve: Y = X^gamma.
    ///
    /// C版: `cmsBuildGamma`
    pub fn build_gamma(_gamma: f64) -> Option<Self> {
        todo!()
    }

    /// Build a parametric tone curve of the given type.
    ///
    /// C版: `cmsBuildParametricToneCurve`
    pub fn build_parametric(_curve_type: i32, _params: &[f64]) -> Option<Self> {
        todo!()
    }

    /// Evaluate the curve at a f32 input value.
    ///
    /// C版: `cmsEvalToneCurveFloat`
    pub fn eval_f32(&self, _v: f32) -> f32 {
        todo!()
    }

    /// Evaluate the curve at a u16 input value using the 16-bit table.
    ///
    /// C版: `cmsEvalToneCurve16`
    pub fn eval_u16(&self, _v: u16) -> u16 {
        todo!()
    }

    /// Get the parametric type (0 if not a single parametric segment).
    pub fn parametric_type(&self) -> i32 {
        if self.segments.len() != 1 {
            0
        } else {
            self.segments[0].curve_type
        }
    }

    /// Access the 16-bit lookup table.
    pub fn table16(&self) -> &[u16] {
        &self.table16
    }

    /// Number of entries in the 16-bit table.
    pub fn table16_len(&self) -> u32 {
        self.n_entries
    }

    /// Access a specific segment.
    pub fn segment(&self, n: usize) -> Option<&CurveSegment> {
        self.segments.get(n)
    }
}

/// Evaluate a built-in parametric curve type.
///
/// Positive `curve_type` = forward, negative = inverse.
#[allow(dead_code)]
fn eval_parametric(curve_type: i32, params: &[f64; 10], r: f64) -> f64 {
    let _ = (curve_type, params, r);
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Parametric type 1: Y = X^gamma
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn parametric_type1_gamma_2_2() {
        let curve = ToneCurve::build_parametric(1, &[2.2]).unwrap();
        assert_eq!(curve.parametric_type(), 1);

        let test_values = [0.0f32, 0.25, 0.5, 0.75, 1.0];
        for &x in &test_values {
            let result = curve.eval_f32(x);
            let expected = (x as f64).powf(2.2) as f32;
            assert!(
                (result - expected).abs() < 1e-5,
                "x={x}: result={result}, expected={expected}"
            );
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn parametric_type1_gamma_1_0_is_identity() {
        let curve = ToneCurve::build_gamma(1.0).unwrap();
        for &x in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let result = curve.eval_f32(x);
            assert!((result - x).abs() < 1e-5, "x={x}: result={result}");
        }
    }

    // ========================================================================
    // Parametric type 4: sRGB (IEC 61966-2.1)
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn parametric_type4_srgb() {
        // sRGB parameters: gamma=2.4, a=1/1.055, b=0.055/1.055, c=1/12.92, d=0.04045
        let params = [2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045];
        let curve = ToneCurve::build_parametric(4, &params).unwrap();

        // Below threshold (linear region)
        let x = 0.02f32;
        let result = curve.eval_f32(x);
        let expected = (x as f64 / 12.92) as f32;
        assert!(
            (result - expected).abs() < 1e-4,
            "linear region: x={x}: result={result}, expected={expected}"
        );

        // Above threshold (power region)
        let x = 0.5f32;
        let result = curve.eval_f32(x);
        let expected = ((x as f64 / 1.055 + 0.055 / 1.055).powf(2.4)) as f32;
        assert!(
            (result - expected).abs() < 1e-4,
            "power region: x={x}: result={result}, expected={expected}"
        );
    }

    // ========================================================================
    // Forward-reverse round-trip for all parametric types
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn round_trip_type1() {
        let curve_fwd = ToneCurve::build_parametric(1, &[2.2]).unwrap();
        let curve_rev = ToneCurve::build_parametric(-1, &[2.2]).unwrap();
        for &x in &[0.1f32, 0.25, 0.5, 0.75, 0.9] {
            let y = curve_fwd.eval_f32(x);
            let x_back = curve_rev.eval_f32(y);
            assert!(
                (x_back - x).abs() < 1e-4,
                "type 1: x={x}, y={y}, x_back={x_back}"
            );
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn round_trip_type4_srgb() {
        let params = [2.4, 1.0 / 1.055, 0.055 / 1.055, 1.0 / 12.92, 0.04045];
        let curve_fwd = ToneCurve::build_parametric(4, &params).unwrap();
        let curve_rev = ToneCurve::build_parametric(-4, &params).unwrap();
        for &x in &[0.01f32, 0.1, 0.25, 0.5, 0.75, 0.9] {
            let y = curve_fwd.eval_f32(x);
            let x_back = curve_rev.eval_f32(y);
            assert!(
                (x_back - x).abs() < 1e-3,
                "type 4: x={x}, y={y}, x_back={x_back}"
            );
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn round_trip_type6() {
        let params = [2.2, 1.5, 0.5, 0.1];
        let curve_fwd = ToneCurve::build_parametric(6, &params).unwrap();
        let curve_rev = ToneCurve::build_parametric(-6, &params).unwrap();
        for &x in &[0.1f32, 0.5, 0.9] {
            let y = curve_fwd.eval_f32(x);
            let x_back = curve_rev.eval_f32(y);
            assert!(
                (x_back - x).abs() < 1e-3,
                "type 6: x={x}, y={y}, x_back={x_back}"
            );
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn round_trip_type108() {
        let curve_fwd = ToneCurve::build_parametric(108, &[2.2]).unwrap();
        let curve_rev = ToneCurve::build_parametric(-108, &[2.2]).unwrap();
        for &x in &[0.1f32, 0.25, 0.5, 0.75, 0.9] {
            let y = curve_fwd.eval_f32(x);
            let x_back = curve_rev.eval_f32(y);
            assert!(
                (x_back - x).abs() < 1e-3,
                "type 108: x={x}, y={y}, x_back={x_back}"
            );
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn round_trip_type109() {
        let curve_fwd = ToneCurve::build_parametric(109, &[5.0]).unwrap();
        let curve_rev = ToneCurve::build_parametric(-109, &[5.0]).unwrap();
        for &x in &[0.1f32, 0.25, 0.5, 0.75, 0.9] {
            let y = curve_fwd.eval_f32(x);
            let x_back = curve_rev.eval_f32(y);
            assert!(
                (x_back - x).abs() < 1e-3,
                "type 109: x={x}, y={y}, x_back={x_back}"
            );
        }
    }

    // ========================================================================
    // 16-bit evaluation path
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn eval_u16_gamma_2_2() {
        let curve = ToneCurve::build_gamma(2.2).unwrap();
        // Test endpoints
        assert_eq!(curve.eval_u16(0), 0);
        assert_eq!(curve.eval_u16(0xFFFF), 0xFFFF);

        // Test midpoint
        let mid = curve.eval_u16(0x8000);
        let expected = ((0x8000u32 as f64 / 65535.0).powf(2.2) * 65535.0 + 0.5) as u16;
        let diff = (mid as i32 - expected as i32).unsigned_abs();
        assert!(diff <= 2, "mid: result={mid}, expected={expected}");
    }

    // ========================================================================
    // build_gamma convenience
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn build_gamma_delegates_to_type1() {
        let g1 = ToneCurve::build_gamma(2.2).unwrap();
        let g2 = ToneCurve::build_parametric(1, &[2.2]).unwrap();
        // Both should produce same results
        for &x in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let r1 = g1.eval_f32(x);
            let r2 = g2.eval_f32(x);
            assert!((r1 - r2).abs() < 1e-6, "x={x}: g1={r1}, g2={r2}");
        }
    }
}
