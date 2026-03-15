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
        _n_samples: u32,
        _n_inputs: u32,
        _n_outputs: u32,
        _flags: u32,
    ) -> Option<Self> {
        todo!()
    }

    /// Create interpolation parameters with per-dimension grid sizes.
    ///
    /// C版: `_cmsComputeInterpParamsEx`
    pub fn compute(
        _n_samples: &[u32],
        _n_inputs: u32,
        _n_outputs: u32,
        _flags: u32,
    ) -> Option<Self> {
        todo!()
    }

    /// Evaluate the LUT using 16-bit integer I/O.
    ///
    /// C版: `Interpolation.Lerp16`
    pub fn eval_16(&self, _input: &[u16], _output: &mut [u16], _table: &[u16]) {
        todo!()
    }

    /// Evaluate the LUT using f32 floating-point I/O.
    ///
    /// C版: `Interpolation.LerpFloat`
    pub fn eval_float(&self, _input: &[f32], _output: &mut [f32], _table: &[f32]) {
        todo!()
    }
}

/// Convert a value in `0..0xFFFF*domain` range to S15.16 fixed-point.
///
/// C版: `_cmsToFixedDomain`
#[allow(dead_code)]
pub(crate) fn to_fixed_domain(_a: i32) -> i32 {
    todo!()
}

/// Saturate a f64 to u16 range with rounding.
///
/// C版: `_cmsQuickSaturateWord`
#[allow(dead_code)]
pub(crate) fn quick_saturate_word(_d: f64) -> u16 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // 1D interpolation: single input, single output (LinLerp1D / LinLerp1Dfloat)
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn to_fixed_domain_known_values() {
        // 0 maps to 0
        assert_eq!(to_fixed_domain(0), 0);
        // 0xFFFF * 1 maps to 1 << 16 = 65536
        assert_eq!(to_fixed_domain(0xFFFF), 1 << 16);
        // 0xFFFF * 2 maps to 2 << 16
        assert_eq!(to_fixed_domain(0xFFFF * 2), 2 << 16);
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
