//! LUT interpolation engine.
//!
//! C版対応: `cmsintrp.c`
//!
//! Supports 1D through 15D interpolation for both u16 and f32 data types.
//! InterpParams holds only metadata (grid dimensions, strides); the LUT table
//! is passed by reference at evaluation time to avoid self-referential structs.

/// Maximum number of input dimensions for LUT interpolation.
pub const MAX_INPUT_DIMENSIONS: usize = 15;

/// Maximum number of output channels (for stack-allocated temporaries in 4D+ interpolation).
pub const MAX_STAGE_CHANNELS: usize = 128;

// Interpolation flags
pub const LERP_FLAGS_16BITS: u32 = 0x0000;
pub const LERP_FLAGS_FLOAT: u32 = 0x0001;
pub const LERP_FLAGS_TRILINEAR: u32 = 0x0100;

/// Interpolation parameters — metadata for a LUT table.
///
/// Does NOT own or reference the table data. The table is passed to
/// `eval_16` / `eval_float` at call time.
#[derive(Debug, Clone)]
pub struct InterpParams {
    pub n_inputs: u32,
    pub n_outputs: u32,
    pub n_samples: [u32; MAX_INPUT_DIMENSIONS],
    pub domain: [u32; MAX_INPUT_DIMENSIONS],
    pub opta: [u32; MAX_INPUT_DIMENSIONS],
    pub flags: u32,
}

impl InterpParams {
    /// Create interpolation parameters where all input dimensions share the
    /// same grid size.
    ///
    /// C版: `_cmsComputeInterpParams`
    pub fn compute_uniform(
        n_samples: u32,
        n_inputs: u32,
        n_outputs: u32,
        flags: u32,
    ) -> Option<Self> {
        let samples = [n_samples; MAX_INPUT_DIMENSIONS];
        Self::compute(&samples, n_inputs, n_outputs, flags)
    }

    /// Create interpolation parameters with per-dimension grid sizes.
    ///
    /// C版: `_cmsComputeInterpParamsEx`
    pub fn compute(n_samples: &[u32], n_inputs: u32, n_outputs: u32, flags: u32) -> Option<Self> {
        if n_inputs == 0 || n_inputs as usize > MAX_INPUT_DIMENSIONS {
            return None;
        }

        let mut params = InterpParams {
            n_inputs,
            n_outputs,
            n_samples: [0; MAX_INPUT_DIMENSIONS],
            domain: [0; MAX_INPUT_DIMENSIONS],
            opta: [0; MAX_INPUT_DIMENSIONS],
            flags,
        };

        for (i, &s) in n_samples.iter().enumerate().take(n_inputs as usize) {
            params.n_samples[i] = s;
            params.domain[i] = s - 1;
        }

        // Compute strides (opta):
        // opta[0] = n_outputs (innermost stride)
        // opta[i] = opta[i-1] * n_samples[n_inputs - i] for i=1..n_inputs-1
        params.opta[0] = n_outputs;
        for i in 1..n_inputs as usize {
            params.opta[i] = params.opta[i - 1] * n_samples[n_inputs as usize - i];
        }

        Some(params)
    }

    /// Evaluate the LUT using 16-bit integer I/O.
    ///
    /// C版: `Interpolation.Lerp16`
    pub fn eval_16(&self, input: &[u16], output: &mut [u16], table: &[u16]) {
        match self.n_inputs {
            1 => {
                if self.n_outputs == 1 {
                    lin_lerp_1d(input, output, self, table);
                } else {
                    eval_1_input(input, output, self, table);
                }
            }
            _ => unimplemented!("dimensions > 1 not yet implemented"),
        }
    }

    /// Evaluate the LUT using f32 floating-point I/O.
    ///
    /// C版: `Interpolation.LerpFloat`
    pub fn eval_float(&self, input: &[f32], output: &mut [f32], table: &[f32]) {
        match self.n_inputs {
            1 => {
                if self.n_outputs == 1 {
                    lin_lerp_1d_float(input, output, self, table);
                } else {
                    eval_1_input_float(input, output, self, table);
                }
            }
            _ => unimplemented!("dimensions > 1 not yet implemented"),
        }
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Convert a value in `0..0xFFFF*domain` range to S15.16 fixed-point.
///
/// C版: `_cmsToFixedDomain`
pub(crate) fn to_fixed_domain(a: i32) -> i32 {
    a + ((a + 0x7fff) / 0xffff)
}

/// Saturate a f64 to u16 range with rounding.
///
/// C版: `_cmsQuickSaturateWord`
#[allow(dead_code)]
pub(crate) fn quick_saturate_word(d: f64) -> u16 {
    let d = d + 0.5;
    if d <= 0.0 {
        return 0;
    }
    if d >= 65535.0 {
        return 0xffff;
    }
    d.floor() as u16
}

/// Fixed-point linear interpolation.
/// `a` is fractional weight (0..0xFFFF), `l` and `h` are the low and high values.
#[inline]
fn linear_interp(a: i32, l: i32, h: i32) -> u16 {
    let dif = (h - l) * a + 0x8000;
    let res = (dif >> 16) + l;
    res as u16
}

/// Clamp f32 to [0.0, 1.0], treating NaN and near-zero as 0.0.
#[inline]
fn fclamp(v: f32) -> f32 {
    if v < 1.0e-9 || v.is_nan() {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

// ============================================================================
// 1D interpolation
// ============================================================================

/// 1D linear interpolation, single input → single output (u16).
///
/// C版: `LinLerp1D`
fn lin_lerp_1d(input: &[u16], output: &mut [u16], p: &InterpParams, table: &[u16]) {
    let value = input[0];

    if value == 0xffff || p.domain[0] == 0 {
        output[0] = table[p.domain[0] as usize];
        return;
    }

    let val3 = p.domain[0] as i32 * value as i32;
    let val3 = to_fixed_domain(val3);
    let cell0 = (val3 >> 16) as usize;
    let rest = val3 & 0xffff;

    let y0 = table[cell0] as i32;
    let y1 = table[cell0 + 1] as i32;
    output[0] = linear_interp(rest, y0, y1);
}

/// 1D linear interpolation, single input → single output (f32).
///
/// C版: `LinLerp1Dfloat`
fn lin_lerp_1d_float(input: &[f32], output: &mut [f32], p: &InterpParams, table: &[f32]) {
    let val2 = fclamp(input[0]);

    if val2 == 1.0 || p.domain[0] == 0 {
        output[0] = table[p.domain[0] as usize];
        return;
    }

    let val2 = val2 * p.domain[0] as f32;
    let cell0 = val2.floor() as usize;
    let cell1 = cell0 + 1; // safe because val2 < domain (we handled 1.0 above)
    let rest = val2 - cell0 as f32;

    let y0 = table[cell0];
    let y1 = table[cell1];
    output[0] = y0 + (y1 - y0) * rest;
}

/// 1D interpolation, single input → N outputs (u16).
///
/// C版: `Eval1Input`
fn eval_1_input(input: &[u16], output: &mut [u16], p: &InterpParams, table: &[u16]) {
    let value = input[0];

    let val3 = p.domain[0] as i32 * value as i32;
    let val3 = to_fixed_domain(val3);
    let cell0 = (val3 >> 16) as usize;
    let rest = val3 & 0xffff;

    let cell1 = if value == 0xffff { cell0 } else { cell0 + 1 };

    let stride = p.opta[0] as usize;
    let k0 = stride * cell0;
    let k1 = stride * cell1;

    for i in 0..p.n_outputs as usize {
        output[i] = linear_interp(rest, table[k0 + i] as i32, table[k1 + i] as i32);
    }
}

/// 1D interpolation, single input → N outputs (f32).
///
/// C版: `Eval1InputFloat`
fn eval_1_input_float(input: &[f32], output: &mut [f32], p: &InterpParams, table: &[f32]) {
    let val2 = fclamp(input[0]);

    let val2f = val2 * p.domain[0] as f32;
    let cell0 = val2f.floor() as usize;
    let cell1 = if val2 >= 1.0 { cell0 } else { cell0 + 1 };
    let rest = val2f - cell0 as f32;

    let stride = p.opta[0] as usize;
    let k0 = stride * cell0;
    let k1 = stride * cell1;

    for i in 0..p.n_outputs as usize {
        output[i] = table[k0 + i] + (table[k1 + i] - table[k0 + i]) * rest;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // 1D interpolation: single input, single output (LinLerp1D / LinLerp1Dfloat)
    // ========================================================================

    #[test]
    fn identity_1d_16bit() {
        // Identity LUT: output == input for all values
        let n = 256u32;
        let table: Vec<u16> = (0..n).map(|i| (i * 0xFFFF / (n - 1)) as u16).collect();
        let params = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_16BITS).unwrap();

        // Test several input values
        for input_val in [0u16, 1, 128, 255, 0x7FFF, 0xFFFE, 0xFFFF] {
            let mut output = [0u16];
            params.eval_16(&[input_val], &mut output, &table);
            let diff = (output[0] as i32 - input_val as i32).unsigned_abs();
            assert!(
                diff <= 1,
                "input={input_val}: output={}, diff={diff}",
                output[0]
            );
        }
    }

    #[test]
    fn identity_1d_float() {
        // Identity LUT: output == input
        let n = 256u32;
        let table: Vec<f32> = (0..n).map(|i| i as f32 / (n - 1) as f32).collect();
        let params = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_FLOAT).unwrap();

        for &input_val in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let mut output = [0.0f32];
            params.eval_float(&[input_val], &mut output, &table);
            assert!(
                (output[0] - input_val).abs() < 1e-5,
                "input={input_val}: output={}",
                output[0]
            );
        }
    }

    #[test]
    fn gamma_3_0_1d_16bit() {
        // Gamma 3.0 curve: y = x^3
        let n = 4096u32;
        let table: Vec<u16> = (0..n)
            .map(|i| {
                let x = i as f64 / (n - 1) as f64;
                (x.powi(3) * 65535.0 + 0.5) as u16
            })
            .collect();
        let params = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_16BITS).unwrap();

        // Check known values
        let test_inputs: Vec<u16> = [0.0f64, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .map(|&x| (x * 65535.0) as u16)
            .collect();
        let expected: Vec<u16> = [0.0f64, 0.25, 0.5, 0.75, 1.0]
            .iter()
            .map(|&x| (x.powi(3) * 65535.0 + 0.5) as u16)
            .collect();

        for (input_val, exp) in test_inputs.iter().zip(expected.iter()) {
            let mut output = [0u16];
            params.eval_16(&[*input_val], &mut output, &table);
            let diff = (output[0] as i32 - *exp as i32).unsigned_abs();
            assert!(
                diff <= 2,
                "input={input_val}: output={}, expected={exp}",
                output[0]
            );
        }
    }

    #[test]
    fn gamma_3_0_1d_float() {
        // Gamma 3.0 curve: y = x^3
        let n = 4096u32;
        let table: Vec<f32> = (0..n)
            .map(|i| {
                let x = i as f64 / (n - 1) as f64;
                x.powi(3) as f32
            })
            .collect();
        let params = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_FLOAT).unwrap();

        for &x in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let mut output = [0.0f32];
            params.eval_float(&[x], &mut output, &table);
            let expected = (x as f64).powi(3) as f32;
            assert!(
                (output[0] - expected).abs() < 1e-4,
                "input={x}: output={}, expected={expected}",
                output[0]
            );
        }
    }

    // ========================================================================
    // 1D multi-output (Eval1Input / Eval1InputFloat)
    // ========================================================================

    #[test]
    fn multi_output_1d_16bit() {
        // 1 input, 3 outputs — identity-like mapping
        let n = 256u32;
        let n_out = 3u32;
        // Table: for each grid point, 3 output channels
        let table: Vec<u16> = (0..n)
            .flat_map(|i| {
                let v = (i * 0xFFFF / (n - 1)) as u16;
                [v, v / 2, 0xFFFF - v]
            })
            .collect();
        let params = InterpParams::compute_uniform(n, 1, n_out, LERP_FLAGS_16BITS).unwrap();

        // Test at midpoint
        let mut output = [0u16; 3];
        params.eval_16(&[0x8000], &mut output, &table);
        // Channel 0: ~0x8000
        assert!((output[0] as i32 - 0x8000).unsigned_abs() <= 2);
        // Channel 1: ~0x4000
        assert!((output[1] as i32 - 0x4000).unsigned_abs() <= 2);
        // Channel 2: ~0x7FFF
        assert!((output[2] as i32 - 0x7FFF).unsigned_abs() <= 2);
    }

    #[test]
    fn multi_output_1d_float() {
        // 1 input, 3 outputs
        let n = 256u32;
        let n_out = 3u32;
        let table: Vec<f32> = (0..n)
            .flat_map(|i| {
                let x = i as f32 / (n - 1) as f32;
                [x, x * 0.5, 1.0 - x]
            })
            .collect();
        let params = InterpParams::compute_uniform(n, 1, n_out, LERP_FLAGS_FLOAT).unwrap();

        let mut output = [0.0f32; 3];
        params.eval_float(&[0.5], &mut output, &table);
        assert!((output[0] - 0.5).abs() < 1e-4);
        assert!((output[1] - 0.25).abs() < 1e-4);
        assert!((output[2] - 0.5).abs() < 1e-4);
    }

    // ========================================================================
    // Boundary conditions
    // ========================================================================

    #[test]
    fn boundary_1d_16bit_zero_and_max() {
        let n = 17u32;
        let table: Vec<u16> = (0..n).map(|i| (i * 0xFFFF / (n - 1)) as u16).collect();
        let params = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_16BITS).unwrap();

        let mut output = [0u16];
        params.eval_16(&[0], &mut output, &table);
        assert_eq!(output[0], 0);

        params.eval_16(&[0xFFFF], &mut output, &table);
        assert_eq!(output[0], 0xFFFF);
    }

    #[test]
    fn boundary_1d_float_clamp() {
        let n = 17u32;
        let table: Vec<f32> = (0..n).map(|i| i as f32 / (n - 1) as f32).collect();
        let params = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_FLOAT).unwrap();

        // Values exactly at 0.0 and 1.0
        let mut output = [0.0f32];
        params.eval_float(&[0.0], &mut output, &table);
        assert!((output[0]).abs() < 1e-6);

        params.eval_float(&[1.0], &mut output, &table);
        assert!((output[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn invalid_dimensions_returns_none() {
        // 0 inputs should fail
        assert!(InterpParams::compute_uniform(17, 0, 1, LERP_FLAGS_16BITS).is_none());
        // 16 inputs (> MAX_INPUT_DIMENSIONS) should fail
        assert!(InterpParams::compute_uniform(17, 16, 1, LERP_FLAGS_16BITS).is_none());
    }

    // ========================================================================
    // Helper functions
    // ========================================================================

    #[test]
    fn to_fixed_domain_known_values() {
        // 0 maps to 0
        assert_eq!(to_fixed_domain(0), 0);
        // 0xFFFF * 1 maps to 1 << 16 = 65536
        assert_eq!(to_fixed_domain(0xFFFF), 1 << 16);
        // 0xFFFF * 2 maps to 2 << 16
        assert_eq!(to_fixed_domain(0xFFFF * 2), 2 << 16);
    }

    #[test]
    fn quick_saturate_word_known_values() {
        assert_eq!(quick_saturate_word(0.0), 0);
        assert_eq!(quick_saturate_word(65535.0), 0xFFFF);
        assert_eq!(quick_saturate_word(-1.0), 0);
        assert_eq!(quick_saturate_word(70000.0), 0xFFFF);
        assert_eq!(quick_saturate_word(32767.5), 32768);
    }

    // ========================================================================
    // 16bit / float consistency
    // ========================================================================

    #[test]
    fn consistency_1d_16bit_vs_float() {
        let n = 256u32;
        let table_16: Vec<u16> = (0..n)
            .map(|i| {
                let x = i as f64 / (n - 1) as f64;
                (x * x * 65535.0 + 0.5) as u16
            })
            .collect();
        let table_f: Vec<f32> = (0..n)
            .map(|i| {
                let x = i as f64 / (n - 1) as f64;
                (x * x) as f32
            })
            .collect();
        let p16 = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_16BITS).unwrap();
        let pf = InterpParams::compute_uniform(n, 1, 1, LERP_FLAGS_FLOAT).unwrap();

        for i in 0..=20 {
            let x = i as f64 / 20.0;
            let input_16 = [(x * 65535.0 + 0.5) as u16];
            let input_f = [x as f32];

            let mut out_16 = [0u16];
            let mut out_f = [0.0f32];
            p16.eval_16(&input_16, &mut out_16, &table_16);
            pf.eval_float(&input_f, &mut out_f, &table_f);

            let v16 = out_16[0] as f64 / 65535.0;
            let vf = out_f[0] as f64;
            assert!((v16 - vf).abs() < 0.001, "x={x}: 16bit={v16}, float={vf}");
        }
    }
}
