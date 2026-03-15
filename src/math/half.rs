//! IEEE 754-2008 half-precision (16-bit) float ↔ 32-bit float conversion.
//!
//! Direct bit manipulation matching C版 `cmshalf.c` semantics.

/// Convert a 16-bit half-float to a 32-bit float.
pub fn half_to_float(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1F) as u32;
    let mant = (h & 0x3FF) as u32;

    let f_bits = if exp == 0 {
        if mant == 0 {
            // ±Zero
            sign << 31
        } else {
            // Subnormal: normalize by shifting mantissa until leading 1 appears
            let mut e = 0i32;
            let mut m = mant;
            while (m & 0x400) == 0 {
                m <<= 1;
                e += 1;
            }
            m &= 0x3FF; // remove the leading 1 bit
            let exp_f = (127 - 15 + 1 - e) as u32;
            (sign << 31) | (exp_f << 23) | (m << 13)
        }
    } else if exp == 31 {
        // Infinity or NaN
        (sign << 31) | (0xFF << 23) | (mant << 13)
    } else {
        // Normal number
        let exp_f = exp + (127 - 15);
        (sign << 31) | (exp_f << 23) | (mant << 13)
    };

    f32::from_bits(f_bits)
}

/// Convert a 32-bit float to a 16-bit half-float.
pub fn float_to_half(f: f32) -> u16 {
    let bits = f.to_bits();
    let sign = (bits >> 31) & 1;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let mant = bits & 0x7FFFFF;

    if exp == 0 {
        // Float zero or subnormal → too small for half, becomes ±0
        (sign << 15) as u16
    } else if exp == 0xFF {
        // Infinity or NaN
        if mant == 0 {
            ((sign << 15) | 0x7C00) as u16
        } else {
            // NaN: preserve some mantissa bits, ensure at least 1 bit set
            ((sign << 15) | 0x7C00 | (mant >> 13).max(1)) as u16
        }
    } else {
        let new_exp = exp - 127 + 15;
        if new_exp >= 31 {
            // Overflow → ±infinity
            ((sign << 15) | 0x7C00) as u16
        } else if new_exp <= 0 {
            // Underflow → subnormal or zero
            if new_exp < -10 {
                // Too small even for subnormal
                (sign << 15) as u16
            } else {
                // Subnormal half: add implicit 1 and shift right
                let m = mant | 0x800000;
                let shift = 1 - new_exp + 13;
                ((sign << 15) | (m >> shift)) as u16
            }
        } else {
            // Normal half
            ((sign << 15) | ((new_exp as u32) << 10) | (mant >> 13)) as u16
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_zero() {
        // +0.0 → 0x0000, -0.0 → 0x8000
        assert_eq!(float_to_half(0.0f32), 0x0000);
        assert_eq!(float_to_half(-0.0f32), 0x8000);
        assert_eq!(half_to_float(0x0000), 0.0f32);
        assert!(half_to_float(0x8000).is_sign_negative());
    }

    #[test]
    fn half_one() {
        // 1.0 → 0x3C00
        assert_eq!(float_to_half(1.0f32), 0x3C00);
        assert_eq!(half_to_float(0x3C00), 1.0f32);
    }

    #[test]
    fn half_minus_one() {
        // -1.0 → 0xBC00
        assert_eq!(float_to_half(-1.0f32), 0xBC00);
        assert_eq!(half_to_float(0xBC00), -1.0f32);
    }

    #[test]
    fn half_infinity() {
        assert_eq!(float_to_half(f32::INFINITY), 0x7C00);
        assert_eq!(float_to_half(f32::NEG_INFINITY), 0xFC00);
        assert!(half_to_float(0x7C00).is_infinite());
        assert!(half_to_float(0x7C00).is_sign_positive());
        assert!(half_to_float(0xFC00).is_infinite());
        assert!(half_to_float(0xFC00).is_sign_negative());
    }

    #[test]
    fn half_nan() {
        let h = float_to_half(f32::NAN);
        // NaN has exponent 0x7C00 mask and non-zero mantissa
        assert_eq!(h & 0x7C00, 0x7C00);
        assert_ne!(h & 0x03FF, 0);
        assert!(half_to_float(h).is_nan());
    }

    #[test]
    fn half_round_trip_normals() {
        // Test a range of normal half-float values
        let test_values: &[f32] = &[0.5, 0.25, 2.0, 100.0, 0.001, 65504.0];
        for &v in test_values {
            let h = float_to_half(v);
            let back = half_to_float(h);
            let rel_err = ((back - v) / v).abs();
            assert!(
                rel_err < 0.001,
                "round-trip error for {v}: got {back}, rel_err={rel_err}"
            );
        }
    }

    #[test]
    fn half_subnormal() {
        // Smallest positive subnormal: 2^-24 ≈ 5.96e-8
        let h_min = 0x0001u16;
        let f = half_to_float(h_min);
        assert!(f > 0.0);
        assert!(f < 1e-6);
    }

    #[test]
    fn half_max() {
        // Maximum finite half: 65504.0 → 0x7BFF
        assert_eq!(float_to_half(65504.0f32), 0x7BFF);
        assert_eq!(half_to_float(0x7BFF), 65504.0f32);
    }
}
