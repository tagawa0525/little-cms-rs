//! Tone curve (gamma) engine.
//!
//! C版対応: `cmsgamma.c`
//!
//! Provides parametric, tabulated, and segmented tone curves with both
//! float (segment-based) and 16-bit (table-based) evaluation paths.

use super::intrp::{self, InterpParams, LERP_FLAGS_16BITS, LERP_FLAGS_FLOAT};

/// Sentinel for unbounded lower segment boundary.
const MINUS_INF: f32 = -1e22;
/// Sentinel for unbounded upper segment boundary.
const PLUS_INF: f32 = 1e22;
/// Near-zero threshold for division guards.
const TOLERANCE: f64 = 0.0001;
/// Default table size for Table16.
const DEFAULT_TABLE_SIZE: u32 = 4096;
/// Maximum allowed table entries.
#[allow(dead_code)]
const MAX_TABLE_ENTRIES: u32 = 65530;

/// Number of parameter values used by each built-in parametric type.
fn param_count(curve_type: i32) -> Option<usize> {
    match curve_type.unsigned_abs() {
        1 | 108 | 109 => Some(1),
        2 => Some(3),
        3 | 6 => Some(4),
        4 | 7 | 8 => Some(5),
        5 => Some(7),
        _ => None,
    }
}

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
    pub fn build_gamma(gamma: f64) -> Option<Self> {
        Self::build_parametric(1, &[gamma])
    }

    /// Build a parametric tone curve of the given type.
    ///
    /// C版: `cmsBuildParametricToneCurve`
    pub fn build_parametric(curve_type: i32, params: &[f64]) -> Option<Self> {
        let _count = param_count(curve_type)?;

        let mut seg = CurveSegment {
            x0: MINUS_INF,
            x1: PLUS_INF,
            curve_type,
            params: [0.0; 10],
            sampled_points: Vec::new(),
        };
        for (i, &p) in params.iter().enumerate().take(10) {
            seg.params[i] = p;
        }

        Self::build_segmented(&[seg])
    }

    /// Build a tone curve from a 16-bit lookup table.
    ///
    /// C版: `cmsBuildTabulatedToneCurve16`
    #[allow(dead_code)]
    pub fn build_tabulated_16(values: &[u16]) -> Option<Self> {
        if values.is_empty() || values.len() > MAX_TABLE_ENTRIES as usize {
            return None;
        }
        let n = values.len() as u32;
        let interp_params = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_16BITS)?;
        Some(ToneCurve {
            segments: Vec::new(),
            seg_interp: Vec::new(),
            table16: values.to_vec(),
            interp_params,
            n_entries: n,
        })
    }

    /// Build a tone curve from a float lookup table.
    ///
    /// C版: `cmsBuildTabulatedToneCurveFloat`
    #[allow(dead_code)]
    pub fn build_tabulated_float(values: &[f32]) -> Option<Self> {
        if values.is_empty() || values.len() > MAX_TABLE_ENTRIES as usize {
            return None;
        }
        // Wrap into 3 segments: constant below 0, sampled [0,1], constant above 1
        let seg0 = CurveSegment {
            x0: MINUS_INF,
            x1: 0.0,
            curve_type: 6,
            params: [
                1.0,
                0.0,
                0.0,
                values[0] as f64,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
            sampled_points: Vec::new(),
        };
        let seg1 = CurveSegment {
            x0: 0.0,
            x1: 1.0,
            curve_type: 0,
            params: [0.0; 10],
            sampled_points: values.to_vec(),
        };
        let last = *values.last().unwrap() as f64;
        let seg2 = CurveSegment {
            x0: 1.0,
            x1: PLUS_INF,
            curve_type: 6,
            params: [1.0, 0.0, 0.0, last, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            sampled_points: Vec::new(),
        };
        Self::build_segmented(&[seg0, seg1, seg2])
    }

    /// Build a tone curve from explicit segments.
    ///
    /// C版: `cmsBuildSegmentedToneCurve`
    pub fn build_segmented(segments: &[CurveSegment]) -> Option<Self> {
        if segments.is_empty() {
            return None;
        }

        // Determine grid size
        let n_grid = if segments.len() == 1 && segments[0].curve_type == 1 {
            entries_by_gamma(segments[0].params[0])
        } else {
            DEFAULT_TABLE_SIZE
        };

        // Clone segments and build per-segment interpolation params
        let mut seg_vec = Vec::with_capacity(segments.len());
        let mut seg_interp = Vec::with_capacity(segments.len());
        for seg in segments {
            seg_vec.push(seg.clone());
            if seg.curve_type == 0 && !seg.sampled_points.is_empty() {
                let ip = InterpParams::compute_uniform(
                    seg.sampled_points.len() as u32,
                    1,
                    1,
                    LERP_FLAGS_FLOAT,
                );
                seg_interp.push(ip);
            } else {
                seg_interp.push(None);
            }
        }

        // Generate Table16 by evaluating segments
        let mut table16 = vec![0u16; n_grid as usize];
        for i in 0..n_grid {
            let r = i as f64 / (n_grid - 1) as f64;
            let val = eval_segmented(&seg_vec, &seg_interp, r as f32);
            table16[i as usize] = intrp::quick_saturate_word(val as f64 * 65535.0);
        }

        let interp_params = InterpParams::compute_uniform(n_grid, 1, 1, LERP_FLAGS_16BITS)?;

        Some(ToneCurve {
            segments: seg_vec,
            seg_interp,
            table16,
            interp_params,
            n_entries: n_grid,
        })
    }

    /// Evaluate the curve at a f32 input value.
    ///
    /// C版: `cmsEvalToneCurveFloat`
    pub fn eval_f32(&self, v: f32) -> f32 {
        if self.segments.is_empty() {
            // No segments — use 16-bit table
            let input = intrp::quick_saturate_word(v as f64 * 65535.0);
            let output = self.eval_u16(input);
            return output as f32 / 65535.0;
        }
        eval_segmented(&self.segments, &self.seg_interp, v)
    }

    /// Evaluate the curve at a u16 input value using the 16-bit table.
    ///
    /// C版: `cmsEvalToneCurve16`
    pub fn eval_u16(&self, v: u16) -> u16 {
        let mut output = [0u16];
        self.interp_params.eval_16(&[v], &mut output, &self.table16);
        output[0]
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

/// Determine Table16 size based on gamma value.
/// If gamma is ~1.0, only 2 entries needed (identity).
fn entries_by_gamma(gamma: f64) -> u32 {
    if (gamma - 1.0).abs() < 0.001 {
        2
    } else {
        DEFAULT_TABLE_SIZE
    }
}

/// Evaluate a segmented function at a given input value.
fn eval_segmented(segments: &[CurveSegment], seg_interp: &[Option<InterpParams>], r: f32) -> f32 {
    // Iterate segments in reverse (last segment has priority on boundaries)
    for i in (0..segments.len()).rev() {
        let seg = &segments[i];
        if r > seg.x0 && r <= seg.x1 {
            let out = if seg.curve_type == 0 {
                // Sampled segment
                if seg.sampled_points.is_empty() {
                    return 0.0;
                }
                let r1 = if (seg.x1 - seg.x0).abs() < 1e-12 {
                    0.0f32
                } else {
                    (r - seg.x0) / (seg.x1 - seg.x0)
                };
                if let Some(ip) = &seg_interp[i] {
                    let mut out = [0.0f32];
                    ip.eval_float(&[r1], &mut out, &seg.sampled_points);
                    out[0]
                } else {
                    0.0
                }
            } else {
                // Parametric evaluation
                let val = eval_parametric(seg.curve_type, &seg.params, r as f64);
                // Clamp infinities
                if val.is_infinite() {
                    if val > 0.0 { PLUS_INF } else { MINUS_INF }
                } else {
                    val as f32
                }
            };
            return out as f32;
        }
    }
    // Handle the first segment's left boundary (r == x0 for the first segment)
    if !segments.is_empty() {
        let seg = &segments[0];
        if r <= seg.x0 {
            if seg.curve_type == 0 {
                return seg.sampled_points.first().copied().unwrap_or(0.0);
            } else {
                return eval_parametric(seg.curve_type, &seg.params, r as f64) as f32;
            }
        }
    }
    MINUS_INF
}

// ============================================================================
// Parametric curve evaluation
// ============================================================================

/// Evaluate a built-in parametric curve type.
///
/// Positive `curve_type` = forward, negative = inverse.
fn eval_parametric(curve_type: i32, params: &[f64; 10], r: f64) -> f64 {
    match curve_type {
        // Type 1: Y = X^g
        1 => {
            if r < 0.0 {
                if (params[0] - 1.0).abs() < TOLERANCE {
                    r
                } else {
                    0.0
                }
            } else {
                r.powf(params[0])
            }
        }
        // Type -1: Y = X^(1/g)
        -1 => {
            if r < 0.0 {
                if (params[0] - 1.0).abs() < TOLERANCE {
                    r
                } else {
                    0.0
                }
            } else if params[0].abs() < TOLERANCE {
                PLUS_INF as f64
            } else {
                r.powf(1.0 / params[0])
            }
        }

        // Type 2: CIE 122-1966
        // Y = (a*X + b)^g     for X >= -b/a
        2 => {
            if params[1].abs() < TOLERANCE {
                0.0
            } else {
                let disc = -params[2] / params[1];
                if r >= disc {
                    let e = params[1] * r + params[2];
                    if e > 0.0 { e.powf(params[0]) } else { 0.0 }
                } else {
                    0.0
                }
            }
        }
        -2 => {
            if params[0].abs() < TOLERANCE || params[1].abs() < TOLERANCE || r < 0.0 {
                0.0
            } else {
                let val = (r.powf(1.0 / params[0]) - params[2]) / params[1];
                if val < 0.0 { 0.0 } else { val }
            }
        }

        // Type 3: IEC 61966-3
        // Y = (a*X + b)^g + c  for X >= -b/a, else Y = c
        3 => {
            if params[1].abs() < TOLERANCE {
                0.0
            } else {
                let disc = (-params[2] / params[1]).max(0.0);
                if r >= disc {
                    let e = params[1] * r + params[2];
                    if e > 0.0 {
                        e.powf(params[0]) + params[3]
                    } else {
                        params[3]
                    }
                } else {
                    params[3]
                }
            }
        }
        -3 => {
            if params[0].abs() < TOLERANCE || params[1].abs() < TOLERANCE {
                0.0
            } else if r >= params[3] {
                let e = r - params[3];
                if e > 0.0 {
                    (e.powf(1.0 / params[0]) - params[2]) / params[1]
                } else {
                    0.0
                }
            } else {
                -params[2] / params[1]
            }
        }

        // Type 4: IEC 61966-2.1 (sRGB)
        // Y = (a*X + b)^g     for X >= d
        // Y = c*X              for X < d
        4 => {
            if r >= params[4] {
                let e = params[1] * r + params[2];
                if e > 0.0 { e.powf(params[0]) } else { 0.0 }
            } else {
                r * params[3]
            }
        }
        -4 => {
            // Compute breakpoint
            let e_bp = params[1] * params[4] + params[2];
            let disc = if e_bp > 0.0 {
                e_bp.powf(params[0])
            } else {
                0.0
            };
            if r >= disc {
                if params[0].abs() < TOLERANCE || params[1].abs() < TOLERANCE {
                    0.0
                } else {
                    (r.powf(1.0 / params[0]) - params[2]) / params[1]
                }
            } else if params[3].abs() < TOLERANCE {
                0.0
            } else {
                r / params[3]
            }
        }

        // Type 5: Y = (a*X + b)^g + e   for X >= d
        //         Y = c*X + f            for X < d
        5 => {
            if r >= params[4] {
                let e = params[1] * r + params[2];
                if e > 0.0 {
                    e.powf(params[0]) + params[5]
                } else {
                    params[5]
                }
            } else {
                r * params[3] + params[6]
            }
        }
        -5 => {
            let disc = params[3] * params[4] + params[6];
            if r >= disc {
                let e = r - params[5];
                if params[0].abs() < TOLERANCE || params[1].abs() < TOLERANCE {
                    0.0
                } else if e >= 0.0 {
                    (e.powf(1.0 / params[0]) - params[2]) / params[1]
                } else {
                    0.0
                }
            } else if params[3].abs() < TOLERANCE {
                0.0
            } else {
                (r - params[6]) / params[3]
            }
        }

        // Type 6: Y = (a*X + b)^g + c
        6 => {
            let e = params[1] * r + params[2];
            if params[0] == 1.0 {
                e + params[3]
            } else if e >= 0.0 {
                e.powf(params[0]) + params[3]
            } else {
                params[3]
            }
        }
        -6 => {
            if params[0].abs() < TOLERANCE || params[1].abs() < TOLERANCE {
                0.0
            } else {
                let e = r - params[3];
                if e >= 0.0 {
                    (e.powf(1.0 / params[0]) - params[2]) / params[1]
                } else {
                    0.0
                }
            }
        }

        // Type 7: Logarithmic
        // Y = a * log10(b * X^c + d) + e
        7 => {
            let e = params[2] * r.powf(params[0]) + params[3];
            if e <= 0.0 {
                params[4]
            } else {
                params[1] * e.log10() + params[4]
            }
        }
        -7 => {
            if params[0].abs() < TOLERANCE
                || params[1].abs() < TOLERANCE
                || params[2].abs() < TOLERANCE
            {
                0.0
            } else {
                ((10.0f64.powf((r - params[4]) / params[1]) - params[3]) / params[2])
                    .powf(1.0 / params[0])
            }
        }

        // Type 8: Exponential
        // Y = a * b^(c*X + d) + e
        8 => params[0] * params[1].powf(params[2] * r + params[3]) + params[4],
        -8 => {
            let disc = r - params[4];
            if disc < 0.0 || params[0].abs() < TOLERANCE || params[2].abs() < TOLERANCE {
                0.0
            } else {
                ((disc / params[0]).ln() / params[1].ln() - params[3]) / params[2]
            }
        }

        // Type 108: S-shaped
        // Y = (1 - (1-X)^(1/g))^(1/g)
        108 => {
            if params[0].abs() < TOLERANCE {
                0.0
            } else {
                (1.0 - (1.0 - r).powf(1.0 / params[0])).powf(1.0 / params[0])
            }
        }
        -108 => 1.0 - (1.0 - r.powf(params[0])).powf(params[0]),

        // Type 109: Sigmoidal
        109 => sigmoid_factory(params[0], r),
        -109 => inverse_sigmoid_factory(params[0], r),

        _ => 0.0,
    }
}

// Sigmoid helper functions for type 109
fn sigmoid_base(k: f64, t: f64) -> f64 {
    1.0 / (1.0 + (-k * t).exp()) - 0.5
}

fn inverted_sigmoid_base(k: f64, t: f64) -> f64 {
    -(1.0 / (t + 0.5) - 1.0).ln() / k
}

fn sigmoid_factory(k: f64, t: f64) -> f64 {
    let correction = 0.5 / sigmoid_base(k, 1.0);
    correction * sigmoid_base(k, 2.0 * t - 1.0) + 0.5
}

fn inverse_sigmoid_factory(k: f64, t: f64) -> f64 {
    let correction = 0.5 / sigmoid_base(k, 1.0);
    (inverted_sigmoid_base(k, (t - 0.5) / correction) + 1.0) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Parametric type 1: Y = X^gamma
    // ========================================================================

    #[test]
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
