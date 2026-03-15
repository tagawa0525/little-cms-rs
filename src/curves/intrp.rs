//! LUT interpolation engine.
//!
//! C版対応: `cmsintrp.c`
//!
//! Supports 1D through 15D interpolation for both u16 and f32 data types.
//! InterpParams holds only metadata (grid dimensions, strides); the LUT table
//! is passed by reference at evaluation time to avoid self-referential structs.

/// Maximum number of input dimensions for LUT interpolation.
pub const MAX_INPUT_DIMENSIONS: usize = 15;
const MAX_INPUT_DIMENSIONS_U32: u32 = MAX_INPUT_DIMENSIONS as u32;

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
        if n_samples.len() < n_inputs as usize {
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
            params.domain[i] = s.checked_sub(1)?;
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
            2 => bilinear_interp_16(input, output, self, table),
            3 => {
                if self.flags & LERP_FLAGS_TRILINEAR != 0 {
                    trilinear_interp_16(input, output, self, table);
                } else {
                    tetrahedral_interp_16(input, output, self, table);
                }
            }
            4..=MAX_INPUT_DIMENSIONS_U32 => {
                debug_assert!(
                    (self.n_outputs as usize) < MAX_STAGE_CHANNELS,
                    "n_outputs ({}) must be less than MAX_STAGE_CHANNELS ({})",
                    self.n_outputs,
                    MAX_STAGE_CHANNELS
                );
                if self.n_outputs as usize >= MAX_STAGE_CHANNELS {
                    return;
                }
                eval_n_inputs_16(input, output, self, table);
            }
            _ => {}
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
            2 => bilinear_interp_float(input, output, self, table),
            3 => {
                if self.flags & LERP_FLAGS_TRILINEAR != 0 {
                    trilinear_interp_float(input, output, self, table);
                } else {
                    tetrahedral_interp_float(input, output, self, table);
                }
            }
            4..=MAX_INPUT_DIMENSIONS_U32 => {
                debug_assert!(
                    (self.n_outputs as usize) < MAX_STAGE_CHANNELS,
                    "n_outputs ({}) must be less than MAX_STAGE_CHANNELS ({})",
                    self.n_outputs,
                    MAX_STAGE_CHANNELS
                );
                if self.n_outputs as usize >= MAX_STAGE_CHANNELS {
                    return;
                }
                eval_n_inputs_float(input, output, self, table);
            }
            _ => {}
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

// ============================================================================
// 2D interpolation
// ============================================================================

/// 2D bilinear interpolation (u16).
///
/// C版: `BilinearInterp16`
fn bilinear_interp_16(input: &[u16], output: &mut [u16], p: &InterpParams, table: &[u16]) {
    let fx = to_fixed_domain(input[0] as i32 * p.domain[0] as i32);
    let x0 = (fx >> 16) as usize;
    let rx = fx & 0xffff;

    let fy = to_fixed_domain(input[1] as i32 * p.domain[1] as i32);
    let y0 = (fy >> 16) as usize;
    let ry = fy & 0xffff;

    let x0_stride = p.opta[1] as usize * x0;
    let x1_stride = if input[0] == 0xffff {
        x0_stride
    } else {
        x0_stride + p.opta[1] as usize
    };
    let y0_stride = p.opta[0] as usize * y0;
    let y1_stride = if input[1] == 0xffff {
        y0_stride
    } else {
        y0_stride + p.opta[0] as usize
    };

    for i in 0..p.n_outputs as usize {
        let d00 = table[x0_stride + y0_stride + i] as i32;
        let d10 = table[x1_stride + y0_stride + i] as i32;
        let d01 = table[x0_stride + y1_stride + i] as i32;
        let d11 = table[x1_stride + y1_stride + i] as i32;

        let dx0 = d00 + (((d10 - d00) * rx + 0x8000) >> 16);
        let dx1 = d01 + (((d11 - d01) * rx + 0x8000) >> 16);
        output[i] = (dx0 + (((dx1 - dx0) * ry + 0x8000) >> 16)) as u16;
    }
}

/// 2D bilinear interpolation (f32).
///
/// C版: `BilinearInterpFloat`
fn bilinear_interp_float(input: &[f32], output: &mut [f32], p: &InterpParams, table: &[f32]) {
    let px = fclamp(input[0]) * p.domain[0] as f32;
    let x0 = px.floor() as usize;
    let rx = px - x0 as f32;

    let py = fclamp(input[1]) * p.domain[1] as f32;
    let y0 = py.floor() as usize;
    let ry = py - y0 as f32;

    let x0_stride = p.opta[1] as usize * x0;
    let x1_stride = if fclamp(input[0]) >= 1.0 {
        x0_stride
    } else {
        x0_stride + p.opta[1] as usize
    };
    let y0_stride = p.opta[0] as usize * y0;
    let y1_stride = if fclamp(input[1]) >= 1.0 {
        y0_stride
    } else {
        y0_stride + p.opta[0] as usize
    };

    for i in 0..p.n_outputs as usize {
        let d00 = table[x0_stride + y0_stride + i];
        let d10 = table[x1_stride + y0_stride + i];
        let d01 = table[x0_stride + y1_stride + i];
        let d11 = table[x1_stride + y1_stride + i];

        let dx0 = d00 + (d10 - d00) * rx;
        let dx1 = d01 + (d11 - d01) * rx;
        output[i] = dx0 + (dx1 - dx0) * ry;
    }
}

// ============================================================================
// 3D interpolation
// ============================================================================

/// 3D trilinear interpolation (u16).
///
/// C版: `TrilinearInterp16`
fn trilinear_interp_16(input: &[u16], output: &mut [u16], p: &InterpParams, table: &[u16]) {
    let fx = to_fixed_domain(input[0] as i32 * p.domain[0] as i32);
    let x0 = (fx >> 16) as usize;
    let rx = fx & 0xffff;

    let fy = to_fixed_domain(input[1] as i32 * p.domain[1] as i32);
    let y0 = (fy >> 16) as usize;
    let ry = fy & 0xffff;

    let fz = to_fixed_domain(input[2] as i32 * p.domain[2] as i32);
    let z0 = (fz >> 16) as usize;
    let rz = fz & 0xffff;

    let x0s = p.opta[2] as usize * x0;
    let x1s = if input[0] == 0xffff {
        x0s
    } else {
        x0s + p.opta[2] as usize
    };
    let y0s = p.opta[1] as usize * y0;
    let y1s = if input[1] == 0xffff {
        y0s
    } else {
        y0s + p.opta[1] as usize
    };
    let z0s = p.opta[0] as usize * z0;
    let z1s = if input[2] == 0xffff {
        z0s
    } else {
        z0s + p.opta[0] as usize
    };

    for i in 0..p.n_outputs as usize {
        let d000 = table[x0s + y0s + z0s + i] as i32;
        let d100 = table[x1s + y0s + z0s + i] as i32;
        let d010 = table[x0s + y1s + z0s + i] as i32;
        let d110 = table[x1s + y1s + z0s + i] as i32;
        let d001 = table[x0s + y0s + z1s + i] as i32;
        let d101 = table[x1s + y0s + z1s + i] as i32;
        let d011 = table[x0s + y1s + z1s + i] as i32;
        let d111 = table[x1s + y1s + z1s + i] as i32;

        let dx00 = d000 + (((d100 - d000) * rx + 0x8000) >> 16);
        let dx01 = d001 + (((d101 - d001) * rx + 0x8000) >> 16);
        let dx10 = d010 + (((d110 - d010) * rx + 0x8000) >> 16);
        let dx11 = d011 + (((d111 - d011) * rx + 0x8000) >> 16);
        let dxy0 = dx00 + (((dx10 - dx00) * ry + 0x8000) >> 16);
        let dxy1 = dx01 + (((dx11 - dx01) * ry + 0x8000) >> 16);
        output[i] = (dxy0 + (((dxy1 - dxy0) * rz + 0x8000) >> 16)) as u16;
    }
}

/// 3D trilinear interpolation (f32).
///
/// C版: `TrilinearInterpFloat`
fn trilinear_interp_float(input: &[f32], output: &mut [f32], p: &InterpParams, table: &[f32]) {
    let px = fclamp(input[0]) * p.domain[0] as f32;
    let x0 = px.floor() as usize;
    let rx = px - x0 as f32;

    let py = fclamp(input[1]) * p.domain[1] as f32;
    let y0 = py.floor() as usize;
    let ry = py - y0 as f32;

    let pz = fclamp(input[2]) * p.domain[2] as f32;
    let z0 = pz.floor() as usize;
    let rz = pz - z0 as f32;

    let x0s = p.opta[2] as usize * x0;
    let x1s = if fclamp(input[0]) >= 1.0 {
        x0s
    } else {
        x0s + p.opta[2] as usize
    };
    let y0s = p.opta[1] as usize * y0;
    let y1s = if fclamp(input[1]) >= 1.0 {
        y0s
    } else {
        y0s + p.opta[1] as usize
    };
    let z0s = p.opta[0] as usize * z0;
    let z1s = if fclamp(input[2]) >= 1.0 {
        z0s
    } else {
        z0s + p.opta[0] as usize
    };

    for i in 0..p.n_outputs as usize {
        let d000 = table[x0s + y0s + z0s + i];
        let d100 = table[x1s + y0s + z0s + i];
        let d010 = table[x0s + y1s + z0s + i];
        let d110 = table[x1s + y1s + z0s + i];
        let d001 = table[x0s + y0s + z1s + i];
        let d101 = table[x1s + y0s + z1s + i];
        let d011 = table[x0s + y1s + z1s + i];
        let d111 = table[x1s + y1s + z1s + i];

        let dx00 = d000 + (d100 - d000) * rx;
        let dx01 = d001 + (d101 - d001) * rx;
        let dx10 = d010 + (d110 - d010) * rx;
        let dx11 = d011 + (d111 - d011) * rx;
        let dxy0 = dx00 + (dx10 - dx00) * ry;
        let dxy1 = dx01 + (dx11 - dx01) * ry;
        output[i] = dxy0 + (dxy1 - dxy0) * rz;
    }
}

/// 3D tetrahedral interpolation (u16) — Sakamoto algorithm.
///
/// C版: `TetrahedralInterp16`
///
/// Decomposes the unit cube into 6 tetrahedra based on the sort order of
/// (rx, ry, rz) fractional parts. Uses 4 lookups + 3 multiplies per output
/// channel instead of trilinear's 8 lookups + 7 lerps.
fn tetrahedral_interp_16(input: &[u16], output: &mut [u16], p: &InterpParams, table: &[u16]) {
    let fx = to_fixed_domain(input[0] as i32 * p.domain[0] as i32);
    let x0 = (fx >> 16) as usize;
    let rx = fx & 0xffff;

    let fy = to_fixed_domain(input[1] as i32 * p.domain[1] as i32);
    let y0 = (fy >> 16) as usize;
    let ry = fy & 0xffff;

    let fz = to_fixed_domain(input[2] as i32 * p.domain[2] as i32);
    let z0 = (fz >> 16) as usize;
    let rz = fz & 0xffff;

    let x0s = p.opta[2] as usize * x0;
    let x1s = if input[0] == 0xffff {
        x0s
    } else {
        x0s + p.opta[2] as usize
    };
    let y0s = p.opta[1] as usize * y0;
    let y1s = if input[1] == 0xffff {
        y0s
    } else {
        y0s + p.opta[1] as usize
    };
    let z0s = p.opta[0] as usize * z0;
    let z1s = if input[2] == 0xffff {
        z0s
    } else {
        z0s + p.opta[0] as usize
    };

    let base = x0s + y0s + z0s;

    for i in 0..p.n_outputs as usize {
        let c0 = table[base + i] as i32;

        let (c1, c2, c3) = if rx >= ry {
            if ry >= rz {
                // rx >= ry >= rz
                (
                    table[x1s + y0s + z0s + i] as i32 - c0,
                    table[x1s + y1s + z0s + i] as i32 - table[x1s + y0s + z0s + i] as i32,
                    table[x1s + y1s + z1s + i] as i32 - table[x1s + y1s + z0s + i] as i32,
                )
            } else if rx >= rz {
                // rx >= rz >= ry
                (
                    table[x1s + y0s + z0s + i] as i32 - c0,
                    table[x1s + y1s + z1s + i] as i32 - table[x1s + y0s + z1s + i] as i32,
                    table[x1s + y0s + z1s + i] as i32 - table[x1s + y0s + z0s + i] as i32,
                )
            } else {
                // rz > rx >= ry
                (
                    table[x1s + y0s + z1s + i] as i32 - table[x0s + y0s + z1s + i] as i32,
                    table[x1s + y1s + z1s + i] as i32 - table[x1s + y0s + z1s + i] as i32,
                    table[x0s + y0s + z1s + i] as i32 - c0,
                )
            }
        } else if rx >= rz {
            // ry > rx >= rz
            (
                table[x1s + y1s + z0s + i] as i32 - table[x0s + y1s + z0s + i] as i32,
                table[x0s + y1s + z0s + i] as i32 - c0,
                table[x1s + y1s + z1s + i] as i32 - table[x1s + y1s + z0s + i] as i32,
            )
        } else if ry >= rz {
            // ry >= rz > rx
            (
                table[x1s + y1s + z1s + i] as i32 - table[x0s + y1s + z1s + i] as i32,
                table[x0s + y1s + z0s + i] as i32 - c0,
                table[x0s + y1s + z1s + i] as i32 - table[x0s + y1s + z0s + i] as i32,
            )
        } else {
            // rz > ry > rx
            (
                table[x1s + y1s + z1s + i] as i32 - table[x0s + y1s + z1s + i] as i32,
                table[x0s + y1s + z1s + i] as i32 - table[x0s + y0s + z1s + i] as i32,
                table[x0s + y0s + z1s + i] as i32 - c0,
            )
        };

        let rest = c1 * rx + c2 * ry + c3 * rz + 0x8001;
        output[i] = (c0 + ((rest + (rest >> 16)) >> 16)) as u16;
    }
}

/// 3D tetrahedral interpolation (f32) — Sakamoto algorithm.
///
/// C版: `TetrahedralInterpFloat`
fn tetrahedral_interp_float(input: &[f32], output: &mut [f32], p: &InterpParams, table: &[f32]) {
    let px = fclamp(input[0]) * p.domain[0] as f32;
    let x0 = px.floor() as usize;
    let rx = px - x0 as f32;

    let py = fclamp(input[1]) * p.domain[1] as f32;
    let y0 = py.floor() as usize;
    let ry = py - y0 as f32;

    let pz = fclamp(input[2]) * p.domain[2] as f32;
    let z0 = pz.floor() as usize;
    let rz = pz - z0 as f32;

    let x0s = p.opta[2] as usize * x0;
    let x1s = if fclamp(input[0]) >= 1.0 {
        x0s
    } else {
        x0s + p.opta[2] as usize
    };
    let y0s = p.opta[1] as usize * y0;
    let y1s = if fclamp(input[1]) >= 1.0 {
        y0s
    } else {
        y0s + p.opta[1] as usize
    };
    let z0s = p.opta[0] as usize * z0;
    let z1s = if fclamp(input[2]) >= 1.0 {
        z0s
    } else {
        z0s + p.opta[0] as usize
    };

    let base = x0s + y0s + z0s;

    for i in 0..p.n_outputs as usize {
        let c0 = table[base + i];

        let (c1, c2, c3) = if rx >= ry {
            if ry >= rz {
                (
                    table[x1s + y0s + z0s + i] - c0,
                    table[x1s + y1s + z0s + i] - table[x1s + y0s + z0s + i],
                    table[x1s + y1s + z1s + i] - table[x1s + y1s + z0s + i],
                )
            } else if rx >= rz {
                (
                    table[x1s + y0s + z0s + i] - c0,
                    table[x1s + y1s + z1s + i] - table[x1s + y0s + z1s + i],
                    table[x1s + y0s + z1s + i] - table[x1s + y0s + z0s + i],
                )
            } else {
                (
                    table[x1s + y0s + z1s + i] - table[x0s + y0s + z1s + i],
                    table[x1s + y1s + z1s + i] - table[x1s + y0s + z1s + i],
                    table[x0s + y0s + z1s + i] - c0,
                )
            }
        } else if rx >= rz {
            (
                table[x1s + y1s + z0s + i] - table[x0s + y1s + z0s + i],
                table[x0s + y1s + z0s + i] - c0,
                table[x1s + y1s + z1s + i] - table[x1s + y1s + z0s + i],
            )
        } else if ry >= rz {
            (
                table[x1s + y1s + z1s + i] - table[x0s + y1s + z1s + i],
                table[x0s + y1s + z0s + i] - c0,
                table[x0s + y1s + z1s + i] - table[x0s + y1s + z0s + i],
            )
        } else {
            (
                table[x1s + y1s + z1s + i] - table[x0s + y1s + z1s + i],
                table[x0s + y1s + z1s + i] - table[x0s + y0s + z1s + i],
                table[x0s + y0s + z1s + i] - c0,
            )
        };

        output[i] = c0 + c1 * rx + c2 * ry + c3 * rz;
    }
}

// ============================================================================
// N-dimensional interpolation (4D-15D, recursive dimensional reduction)
// ============================================================================

/// N-dimensional interpolation (u16), recursive.
///
/// Peels off the outermost input dimension, evaluates the inner (N-1)-dimensional
/// interpolation at two adjacent grid planes, then linearly interpolates between them.
///
/// C版: `Eval4Inputs` .. `Eval15Inputs` (generated by EVAL_FNS macro)
fn eval_n_inputs_16(input: &[u16], output: &mut [u16], p: &InterpParams, table: &[u16]) {
    let n_in = p.n_inputs as usize;

    // Compute position for the outermost dimension
    let fk = to_fixed_domain(input[0] as i32 * p.domain[0] as i32);
    let k0 = (fk >> 16) as usize;
    let rk = fk & 0xffff;
    let k1 = if input[0] == 0xffff { k0 } else { k0 + 1 };

    let stride = p.opta[n_in - 1] as usize;
    let offset0 = stride * k0;
    let offset1 = stride * k1;

    // Build inner params: shift domain to remove outermost dimension
    let mut p1 = p.clone();
    p1.n_inputs -= 1;
    for i in 0..p1.n_inputs as usize {
        p1.domain[i] = p.domain[i + 1];
        p1.n_samples[i] = p.n_samples[i + 1];
    }
    // opta is NOT shifted — inner dimensions use opta[0..n_in-2]

    let n_out = p.n_outputs as usize;
    let mut tmp1 = [0u16; MAX_STAGE_CHANNELS];
    let mut tmp2 = [0u16; MAX_STAGE_CHANNELS];

    // Evaluate inner interpolation at the two adjacent grid planes
    p1.eval_16(&input[1..], &mut tmp1[..n_out], &table[offset0..]);
    p1.eval_16(&input[1..], &mut tmp2[..n_out], &table[offset1..]);

    // Linearly interpolate between the two results
    for i in 0..n_out {
        output[i] = linear_interp(rk, tmp1[i] as i32, tmp2[i] as i32);
    }
}

/// N-dimensional interpolation (f32), recursive.
///
/// C版: `Eval4InputsFloat` .. `Eval15InputsFloat`
fn eval_n_inputs_float(input: &[f32], output: &mut [f32], p: &InterpParams, table: &[f32]) {
    let n_in = p.n_inputs as usize;

    let val = fclamp(input[0]) * p.domain[0] as f32;
    let cell0 = val.floor() as usize;
    let cell1 = if fclamp(input[0]) >= 1.0 {
        cell0
    } else {
        cell0 + 1
    };
    let rest = val - cell0 as f32;

    let stride = p.opta[n_in - 1] as usize;
    let offset0 = stride * cell0;
    let offset1 = stride * cell1;

    let mut p1 = p.clone();
    p1.n_inputs -= 1;
    for i in 0..p1.n_inputs as usize {
        p1.domain[i] = p.domain[i + 1];
        p1.n_samples[i] = p.n_samples[i + 1];
    }

    let n_out = p.n_outputs as usize;
    let mut tmp1 = [0.0f32; MAX_STAGE_CHANNELS];
    let mut tmp2 = [0.0f32; MAX_STAGE_CHANNELS];

    p1.eval_float(&input[1..], &mut tmp1[..n_out], &table[offset0..]);
    p1.eval_float(&input[1..], &mut tmp2[..n_out], &table[offset1..]);

    for i in 0..n_out {
        output[i] = tmp1[i] + (tmp2[i] - tmp1[i]) * rest;
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

    // ========================================================================
    // 2D interpolation (BilinearInterp16 / BilinearInterpFloat)
    // ========================================================================

    /// Build a 2D identity LUT: output[ch] = input[ch] for 3-channel output.
    ///
    /// Table convention: i0 (outermost, Input[0]) varies slowest,
    /// i1 (innermost, Input[1]) varies fastest.
    fn build_2d_identity_table_16(n: u32, n_out: u32) -> Vec<u16> {
        let mut table = vec![0u16; (n * n * n_out) as usize];
        for i0 in 0..n {
            for i1 in 0..n {
                let idx = ((i0 * n + i1) * n_out) as usize;
                table[idx] = (i0 * 0xFFFF / (n - 1)) as u16; // Output[0] = Input[0]
                table[idx + 1] = (i1 * 0xFFFF / (n - 1)) as u16; // Output[1] = Input[1]
                if n_out > 2 {
                    table[idx + 2] = ((i0 * 0xFFFF / (n - 1) + i1 * 0xFFFF / (n - 1)) / 2) as u16;
                }
            }
        }
        table
    }

    fn build_2d_identity_table_float(n: u32, n_out: u32) -> Vec<f32> {
        let mut table = vec![0.0f32; (n * n * n_out) as usize];
        for i0 in 0..n {
            for i1 in 0..n {
                let idx = ((i0 * n + i1) * n_out) as usize;
                let v0 = i0 as f32 / (n - 1) as f32;
                let v1 = i1 as f32 / (n - 1) as f32;
                table[idx] = v0;
                table[idx + 1] = v1;
                if n_out > 2 {
                    table[idx + 2] = (v0 + v1) / 2.0;
                }
            }
        }
        table
    }

    #[test]

    fn identity_2d_16bit() {
        let n = 17u32;
        let n_out = 3u32;
        let table = build_2d_identity_table_16(n, n_out);
        let params = InterpParams::compute_uniform(n, 2, n_out, LERP_FLAGS_16BITS).unwrap();

        // Test diagonal: input (x, x) should give output (x, x, x)
        for &v in &[0u16, 0x4000, 0x8000, 0xC000, 0xFFFF] {
            let mut output = [0u16; 3];
            params.eval_16(&[v, v], &mut output, &table);
            assert!(
                (output[0] as i32 - v as i32).unsigned_abs() <= 2,
                "v={v}: ch0={}",
                output[0]
            );
            assert!(
                (output[1] as i32 - v as i32).unsigned_abs() <= 2,
                "v={v}: ch1={}",
                output[1]
            );
        }
    }

    #[test]

    fn identity_2d_float() {
        let n = 17u32;
        let n_out = 3u32;
        let table = build_2d_identity_table_float(n, n_out);
        let params = InterpParams::compute_uniform(n, 2, n_out, LERP_FLAGS_FLOAT).unwrap();

        for &v in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let mut output = [0.0f32; 3];
            params.eval_float(&[v, v], &mut output, &table);
            assert!((output[0] - v).abs() < 1e-3, "v={v}: ch0={}", output[0]);
            assert!((output[1] - v).abs() < 1e-3, "v={v}: ch1={}", output[1]);
        }
    }

    // ========================================================================
    // 3D interpolation (Tetrahedral / Trilinear)
    // ========================================================================

    /// Build a 3D identity CLUT: 3 inputs → 3 outputs, output[ch] == input[ch].
    ///
    /// Table convention: i0 (outermost, Input[0]) varies slowest,
    /// i2 (innermost, Input[2]) varies fastest.
    fn build_3d_identity_clut_16(n: u32) -> Vec<u16> {
        let n_out = 3u32;
        let mut table = vec![0u16; (n * n * n * n_out) as usize];
        for i0 in 0..n {
            for i1 in 0..n {
                for i2 in 0..n {
                    let idx = (((i0 * n + i1) * n + i2) * n_out) as usize;
                    table[idx] = (i0 * 0xFFFF / (n - 1)) as u16;
                    table[idx + 1] = (i1 * 0xFFFF / (n - 1)) as u16;
                    table[idx + 2] = (i2 * 0xFFFF / (n - 1)) as u16;
                }
            }
        }
        table
    }

    fn build_3d_identity_clut_float(n: u32) -> Vec<f32> {
        let n_out = 3u32;
        let mut table = vec![0.0f32; (n * n * n * n_out) as usize];
        for i0 in 0..n {
            for i1 in 0..n {
                for i2 in 0..n {
                    let idx = (((i0 * n + i1) * n + i2) * n_out) as usize;
                    table[idx] = i0 as f32 / (n - 1) as f32;
                    table[idx + 1] = i1 as f32 / (n - 1) as f32;
                    table[idx + 2] = i2 as f32 / (n - 1) as f32;
                }
            }
        }
        table
    }

    #[test]

    fn identity_3d_tetrahedral_16bit() {
        let n = 17u32;
        let table = build_3d_identity_clut_16(n);
        let params = InterpParams::compute_uniform(n, 3, 3, LERP_FLAGS_16BITS).unwrap();

        // Test various RGB values
        let test_values: Vec<[u16; 3]> = vec![
            [0, 0, 0],
            [0xFFFF, 0xFFFF, 0xFFFF],
            [0x8000, 0x8000, 0x8000],
            [0xFFFF, 0, 0],
            [0, 0xFFFF, 0],
            [0, 0, 0xFFFF],
            [0x4000, 0x8000, 0xC000],
        ];

        for input in &test_values {
            let mut output = [0u16; 3];
            params.eval_16(input, &mut output, &table);
            for ch in 0..3 {
                let diff = (output[ch] as i32 - input[ch] as i32).unsigned_abs();
                assert!(
                    diff <= 2,
                    "input={input:?}: ch{ch} output={}, expected={}",
                    output[ch],
                    input[ch]
                );
            }
        }
    }

    #[test]

    fn identity_3d_tetrahedral_float() {
        let n = 33u32;
        let table = build_3d_identity_clut_float(n);
        let params = InterpParams::compute_uniform(n, 3, 3, LERP_FLAGS_FLOAT).unwrap();

        let test_values: Vec<[f32; 3]> = vec![
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.5, 0.5, 0.5],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.25, 0.5, 0.75],
        ];

        for input in &test_values {
            let mut output = [0.0f32; 3];
            params.eval_float(input, &mut output, &table);
            for ch in 0..3 {
                assert!(
                    (output[ch] - input[ch]).abs() < 1e-3,
                    "input={input:?}: ch{ch} output={}, expected={}",
                    output[ch],
                    input[ch]
                );
            }
        }
    }

    #[test]

    fn identity_3d_trilinear_16bit() {
        let n = 17u32;
        let table = build_3d_identity_clut_16(n);
        let params =
            InterpParams::compute_uniform(n, 3, 3, LERP_FLAGS_16BITS | LERP_FLAGS_TRILINEAR)
                .unwrap();

        let mut output = [0u16; 3];
        params.eval_16(&[0x8000, 0x4000, 0xC000], &mut output, &table);
        assert!((output[0] as i32 - 0x8000).unsigned_abs() <= 2);
        assert!((output[1] as i32 - 0x4000).unsigned_abs() <= 2);
        assert!((output[2] as i32 - 0xC000).unsigned_abs() <= 2);
    }

    #[test]

    fn identity_3d_trilinear_float() {
        let n = 17u32;
        let table = build_3d_identity_clut_float(n);
        let params =
            InterpParams::compute_uniform(n, 3, 3, LERP_FLAGS_FLOAT | LERP_FLAGS_TRILINEAR)
                .unwrap();

        let mut output = [0.0f32; 3];
        params.eval_float(&[0.5, 0.25, 0.75], &mut output, &table);
        assert!((output[0] - 0.5).abs() < 1e-3);
        assert!((output[1] - 0.25).abs() < 1e-3);
        assert!((output[2] - 0.75).abs() < 1e-3);
    }

    #[test]

    fn consistency_3d_tetrahedral_vs_trilinear() {
        // Both should give same results on identity CLUT
        let n = 33u32;
        let table = build_3d_identity_clut_16(n);
        let p_tet = InterpParams::compute_uniform(n, 3, 3, LERP_FLAGS_16BITS).unwrap();
        let p_tri =
            InterpParams::compute_uniform(n, 3, 3, LERP_FLAGS_16BITS | LERP_FLAGS_TRILINEAR)
                .unwrap();

        for r in (0..=0xFFFFu32).step_by(0x3333) {
            for g in (0..=0xFFFFu32).step_by(0x5555) {
                let input = [r as u16, g as u16, 0x8000u16];
                let mut out_tet = [0u16; 3];
                let mut out_tri = [0u16; 3];
                p_tet.eval_16(&input, &mut out_tet, &table);
                p_tri.eval_16(&input, &mut out_tri, &table);
                for ch in 0..3 {
                    let diff = (out_tet[ch] as i32 - out_tri[ch] as i32).unsigned_abs();
                    assert!(
                        diff <= 2,
                        "input={input:?}: ch{ch} tet={}, tri={}",
                        out_tet[ch],
                        out_tri[ch]
                    );
                }
            }
        }
    }

    #[test]

    fn consistency_3d_16bit_vs_float() {
        let n = 33u32;
        let table_16 = build_3d_identity_clut_16(n);
        let table_f = build_3d_identity_clut_float(n);
        let p16 = InterpParams::compute_uniform(n, 3, 3, LERP_FLAGS_16BITS).unwrap();
        let pf = InterpParams::compute_uniform(n, 3, 3, LERP_FLAGS_FLOAT).unwrap();

        let test_values: Vec<[f64; 3]> = vec![[0.1, 0.2, 0.3], [0.5, 0.5, 0.5], [0.9, 0.1, 0.5]];

        for input in &test_values {
            let input_16: Vec<u16> = input.iter().map(|&x| (x * 65535.0 + 0.5) as u16).collect();
            let input_f: Vec<f32> = input.iter().map(|&x| x as f32).collect();

            let mut out_16 = [0u16; 3];
            let mut out_f = [0.0f32; 3];
            p16.eval_16(&input_16, &mut out_16, &table_16);
            pf.eval_float(&input_f, &mut out_f, &table_f);

            for ch in 0..3 {
                let v16 = out_16[ch] as f64 / 65535.0;
                let vf = out_f[ch] as f64;
                assert!(
                    (v16 - vf).abs() < 0.002,
                    "input={input:?}: ch{ch} 16bit={v16}, float={vf}"
                );
            }
        }
    }

    // ========================================================================
    // 4D+ interpolation (recursive dimensional reduction)
    // ========================================================================

    /// Build a 4D identity CLUT: 4 inputs → 4 outputs (e.g. CMYK identity)
    fn build_4d_identity_clut_16(n: u32) -> Vec<u16> {
        let n_out = 4u32;
        let total = n.pow(4) * n_out;
        let mut table = vec![0u16; total as usize];
        for i0 in 0..n {
            for i1 in 0..n {
                for i2 in 0..n {
                    for i3 in 0..n {
                        let idx = ((((i0 * n + i1) * n + i2) * n + i3) * n_out) as usize;
                        table[idx] = (i0 * 0xFFFF / (n - 1)) as u16;
                        table[idx + 1] = (i1 * 0xFFFF / (n - 1)) as u16;
                        table[idx + 2] = (i2 * 0xFFFF / (n - 1)) as u16;
                        table[idx + 3] = (i3 * 0xFFFF / (n - 1)) as u16;
                    }
                }
            }
        }
        table
    }

    fn build_4d_identity_clut_float(n: u32) -> Vec<f32> {
        let n_out = 4u32;
        let total = n.pow(4) * n_out;
        let mut table = vec![0.0f32; total as usize];
        for i0 in 0..n {
            for i1 in 0..n {
                for i2 in 0..n {
                    for i3 in 0..n {
                        let idx = ((((i0 * n + i1) * n + i2) * n + i3) * n_out) as usize;
                        let d = (n - 1) as f32;
                        table[idx] = i0 as f32 / d;
                        table[idx + 1] = i1 as f32 / d;
                        table[idx + 2] = i2 as f32 / d;
                        table[idx + 3] = i3 as f32 / d;
                    }
                }
            }
        }
        table
    }

    #[test]

    fn identity_4d_16bit() {
        let n = 9u32;
        let table = build_4d_identity_clut_16(n);
        let params = InterpParams::compute_uniform(n, 4, 4, LERP_FLAGS_16BITS).unwrap();

        let test_values: Vec<[u16; 4]> = vec![
            [0, 0, 0, 0],
            [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF],
            [0x8000, 0x8000, 0x8000, 0x8000],
            [0x4000, 0x8000, 0xC000, 0x2000],
        ];

        for input in &test_values {
            let mut output = [0u16; 4];
            params.eval_16(input, &mut output, &table);
            for ch in 0..4 {
                let diff = (output[ch] as i32 - input[ch] as i32).unsigned_abs();
                assert!(
                    diff <= 2,
                    "input={input:?}: ch{ch} output={}, expected={}",
                    output[ch],
                    input[ch]
                );
            }
        }
    }

    #[test]

    fn identity_4d_float() {
        let n = 9u32;
        let table = build_4d_identity_clut_float(n);
        let params = InterpParams::compute_uniform(n, 4, 4, LERP_FLAGS_FLOAT).unwrap();

        let test_values: Vec<[f32; 4]> = vec![
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0, 1.0],
            [0.5, 0.5, 0.5, 0.5],
            [0.25, 0.5, 0.75, 0.125],
        ];

        for input in &test_values {
            let mut output = [0.0f32; 4];
            params.eval_float(input, &mut output, &table);
            for ch in 0..4 {
                assert!(
                    (output[ch] - input[ch]).abs() < 1e-2,
                    "input={input:?}: ch{ch} output={}, expected={}",
                    output[ch],
                    input[ch]
                );
            }
        }
    }

    #[test]

    fn identity_5d_16bit() {
        // 5D with small grid to keep memory manageable
        let n = 5u32;
        let n_out = 3u32;
        let total = n.pow(5) * n_out;
        let mut table = vec![0u16; total as usize];
        for i0 in 0..n {
            for i1 in 0..n {
                for i2 in 0..n {
                    for i3 in 0..n {
                        for i4 in 0..n {
                            let idx =
                                (((((i0 * n + i1) * n + i2) * n + i3) * n + i4) * n_out) as usize;
                            table[idx] = (i0 * 0xFFFF / (n - 1)) as u16;
                            table[idx + 1] = (i2 * 0xFFFF / (n - 1)) as u16;
                            table[idx + 2] = (i4 * 0xFFFF / (n - 1)) as u16;
                        }
                    }
                }
            }
        }

        let params = InterpParams::compute_uniform(n, 5, n_out, LERP_FLAGS_16BITS).unwrap();
        let mut output = [0u16; 3];
        // All zeros
        params.eval_16(&[0, 0, 0, 0, 0], &mut output, &table);
        assert_eq!(output[0], 0);
        // All max
        params.eval_16(
            &[0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF],
            &mut output,
            &table,
        );
        assert_eq!(output[0], 0xFFFF);
        // Mid
        params.eval_16(
            &[0x8000, 0x8000, 0x8000, 0x8000, 0x8000],
            &mut output,
            &table,
        );
        assert!((output[0] as i32 - 0x8000).unsigned_abs() <= 4);
    }
}
