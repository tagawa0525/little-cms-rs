// ============================================================================
// Pixel format pack/unpack (C版: cmspack.c)
// ============================================================================

/// Pack flags for formatter lookup.
pub const CMS_PACK_FLAGS_16BITS: u32 = 0x0000;
pub const CMS_PACK_FLAGS_FLOAT: u32 = 0x0001;

/// Input formatter: reads one pixel from `buf`, fills `values` (u16).
/// Returns the number of bytes consumed.
/// `stride` is the plane spacing in bytes (used only for planar formats).
pub type Formatter16In =
    fn(format: crate::types::PixelFormat, values: &mut [u16], buf: &[u8], stride: usize) -> usize;

/// Output formatter: writes one pixel from `values` (u16) into `buf`.
/// Returns the number of bytes written.
pub type Formatter16Out =
    fn(format: crate::types::PixelFormat, values: &[u16], buf: &mut [u8], stride: usize) -> usize;

/// Float input formatter.
pub type FormatterFloatIn =
    fn(format: crate::types::PixelFormat, values: &mut [f32], buf: &[u8], stride: usize) -> usize;

/// Float output formatter.
pub type FormatterFloatOut =
    fn(format: crate::types::PixelFormat, values: &[f32], buf: &mut [u8], stride: usize) -> usize;

/// Combined input formatter.
pub enum FormatterIn {
    U16(Formatter16In),
    Float(FormatterFloatIn),
}

/// Combined output formatter.
pub enum FormatterOut {
    U16(Formatter16Out),
    Float(FormatterFloatOut),
}

// ============================================================================
// Byte conversion helpers (stubs — to be implemented in GREEN commit)
// ============================================================================

pub fn from_8_to_16(_v: u8) -> u16 {
    todo!()
}

pub fn from_16_to_8(_v: u16) -> u8 {
    todo!()
}

pub fn lab_v2_to_v4(_v: u16) -> u16 {
    todo!()
}

pub fn lab_v4_to_v2(_v: u16) -> u16 {
    todo!()
}

pub fn pixel_size(_format: crate::types::PixelFormat) -> usize {
    todo!()
}

pub fn find_formatter_in(_format: crate::types::PixelFormat, _flags: u32) -> Option<FormatterIn> {
    todo!()
}

pub fn find_formatter_out(_format: crate::types::PixelFormat, _flags: u32) -> Option<FormatterOut> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    // ================================================================
    // Byte conversion helpers
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_from_8_to_16() {
        assert_eq!(from_8_to_16(0x00), 0x0000u16);
        assert_eq!(from_8_to_16(0x80), 0x8080u16);
        assert_eq!(from_8_to_16(0xFF), 0xFFFFu16);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_from_16_to_8() {
        assert_eq!(from_16_to_8(0x0000), 0x00u8);
        assert_eq!(from_16_to_8(0x8080), 0x80u8);
        assert_eq!(from_16_to_8(0xFFFF), 0xFFu8);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_from_8_to_16_round_trip() {
        for v in 0..=255u8 {
            let v16 = from_8_to_16(v);
            let back = from_16_to_8(v16);
            assert_eq!(back, v, "round-trip failed for {v}");
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_lab_v2_to_v4_round_trip() {
        // V2→V4→V2 should be lossless at common values
        for v in [0u16, 0x8080, 0xFF00, 0xFFFF] {
            let v4 = lab_v2_to_v4(v);
            let back = lab_v4_to_v2(v4);
            assert!(
                (back as i32 - v as i32).unsigned_abs() <= 1,
                "v={v:#06X}: v4={v4:#06X}, back={back:#06X}"
            );
        }
    }

    // ================================================================
    // pixel_size
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_pixel_size() {
        assert_eq!(pixel_size(TYPE_RGB_8), 3);
        assert_eq!(pixel_size(TYPE_RGBA_8), 4);
        assert_eq!(pixel_size(TYPE_CMYK_16), 8);
        assert_eq!(pixel_size(TYPE_GRAY_8), 1);
        assert_eq!(pixel_size(TYPE_GRAY_16), 2);
        assert_eq!(pixel_size(TYPE_RGB_FLT), 12);
    }

    // ================================================================
    // Basic 8-bit chunky formatters (RGB_8)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_rgb_8_unroll_pack_round_trip() {
        let format = TYPE_RGB_8;
        let input: [u8; 3] = [0xAA, 0x55, 0xCC];
        let mut values = [0u16; 16];

        let fmt_in = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap();
        let FormatterIn::U16(unroll) = fmt_in else {
            panic!("expected U16")
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 3);
        assert_eq!(values[0], from_8_to_16(0xAA));
        assert_eq!(values[1], from_8_to_16(0x55));
        assert_eq!(values[2], from_8_to_16(0xCC));

        let fmt_out = find_formatter_out(format, CMS_PACK_FLAGS_16BITS).unwrap();
        let FormatterOut::U16(pack) = fmt_out else {
            panic!("expected U16")
        };
        let mut output = [0u8; 3];
        let written = pack(format, &values, &mut output, 0);
        assert_eq!(written, 3);
        assert_eq!(output, input);
    }

    // ================================================================
    // GRAY_8 round-trip
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_gray_8_round_trip() {
        let format = TYPE_GRAY_8;
        let input: [u8; 1] = [0x80];
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 1);
        assert_eq!(values[0], from_8_to_16(0x80));

        let FormatterOut::U16(pack) = find_formatter_out(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let mut output = [0u8; 1];
        let written = pack(format, &values, &mut output, 0);
        assert_eq!(written, 1);
        assert_eq!(output, input);
    }

    // ================================================================
    // CMYK_16 round-trip
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_cmyk_16_round_trip() {
        let format = TYPE_CMYK_16;
        let input: [u8; 8] = [0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80];
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 8);
        // Native endian: bytes [0x10,0x20] form a u16
        let expected_0 = u16::from_ne_bytes([0x10, 0x20]);
        assert_eq!(values[0], expected_0);

        let FormatterOut::U16(pack) = find_formatter_out(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let mut output = [0u8; 8];
        let written = pack(format, &values, &mut output, 0);
        assert_eq!(written, 8);
        assert_eq!(output, input);
    }

    // ================================================================
    // Swap flags (BGR_8)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_bgr_8_swap() {
        let format = TYPE_BGR_8;
        let input: [u8; 3] = [0x10, 0x20, 0x30]; // memory order: B, G, R
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 3);
        // After unrolling with DOSWAP, channel order should be reversed:
        // values[0]=R, values[1]=G, values[2]=B
        assert_eq!(values[0], from_8_to_16(0x30)); // R
        assert_eq!(values[1], from_8_to_16(0x20)); // G
        assert_eq!(values[2], from_8_to_16(0x10)); // B

        // Pack back to BGR
        let FormatterOut::U16(pack) = find_formatter_out(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let mut output = [0u8; 3];
        let written = pack(format, &values, &mut output, 0);
        assert_eq!(written, 3);
        assert_eq!(output, input); // B, G, R in memory
    }

    // ================================================================
    // Reverse/flavor (CMYK_8_REV)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_cmyk_8_rev_flavor() {
        let format = TYPE_CMYK_8_REV;
        let input: [u8; 4] = [0xFF, 0x80, 0x40, 0x00]; // reversed values
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        unroll(format, &mut values, &input, 0);
        // FLAVOR reverses: 0xFF-v, so 0xFF→0x00, 0x80→0x7F, 0x40→0xBF, 0x00→0xFF
        assert_eq!(values[0], from_8_to_16(0x00));
        assert_eq!(values[1], from_8_to_16(0x7F));
        assert_eq!(values[2], from_8_to_16(0xBF));
        assert_eq!(values[3], from_8_to_16(0xFF));
    }

    // ================================================================
    // Extra channels (RGBA_8)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_rgba_8_extra_channel() {
        let format = TYPE_RGBA_8;
        let input: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xFF]; // R, G, B, Alpha
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 4);
        assert_eq!(values[0], from_8_to_16(0xAA)); // R
        assert_eq!(values[1], from_8_to_16(0xBB)); // G
        assert_eq!(values[2], from_8_to_16(0xCC)); // B
        // Alpha is extra, stored after main channels

        // Pack back
        let FormatterOut::U16(pack) = find_formatter_out(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let mut output = [0u8; 4];
        pack(format, &values, &mut output, 0);
        assert_eq!(output[0], 0xAA); // R
        assert_eq!(output[1], 0xBB); // G
        assert_eq!(output[2], 0xCC); // B
    }

    // ================================================================
    // Swapfirst (ARGB_8)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_argb_8_swapfirst() {
        let format = TYPE_ARGB_8;
        // Memory: Alpha, R, G, B (swapfirst = extra first)
        let input: [u8; 4] = [0xFF, 0xAA, 0xBB, 0xCC];
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 4);
        assert_eq!(values[0], from_8_to_16(0xAA)); // R
        assert_eq!(values[1], from_8_to_16(0xBB)); // G
        assert_eq!(values[2], from_8_to_16(0xCC)); // B
    }

    // ================================================================
    // Planar (RGB_8_PLANAR)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_rgb_8_planar() {
        let format = TYPE_RGB_8_PLANAR;
        // Planar layout: stride=4 (4 pixels per plane)
        // Plane R: [R0, R1, R2, R3], Plane G: [G0, G1, G2, G3], Plane B: [B0, B1, B2, B3]
        let stride = 4usize;
        let mut buf = [0u8; 12]; // 3 planes * 4 bytes
        buf[0] = 0xAA; // R0
        buf[stride] = 0xBB; // G0
        buf[stride * 2] = 0xCC; // B0

        let mut values = [0u16; 16];
        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &buf, stride);
        assert_eq!(consumed, 1); // advance 1 byte to next pixel in plane
        assert_eq!(values[0], from_8_to_16(0xAA)); // R
        assert_eq!(values[1], from_8_to_16(0xBB)); // G
        assert_eq!(values[2], from_8_to_16(0xCC)); // B
    }

    // ================================================================
    // Planar 16-bit (RGB_16_PLANAR)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_rgb_16_planar() {
        let format = TYPE_RGB_16_PLANAR;
        let stride = 8usize; // 4 pixels * 2 bytes each
        let mut buf = [0u8; 24]; // 3 planes * 8 bytes
        // R0 at offset 0
        buf[0..2].copy_from_slice(&0x1234u16.to_ne_bytes());
        // G0 at offset stride
        buf[stride..stride + 2].copy_from_slice(&0x5678u16.to_ne_bytes());
        // B0 at offset 2*stride
        buf[stride * 2..stride * 2 + 2].copy_from_slice(&0x9ABCu16.to_ne_bytes());

        let mut values = [0u16; 16];
        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &buf, stride);
        assert_eq!(consumed, 2);
        assert_eq!(values[0], 0x1234);
        assert_eq!(values[1], 0x5678);
        assert_eq!(values[2], 0x9ABC);
    }

    // ================================================================
    // Float (RGB_FLT)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_rgb_flt_round_trip() {
        let format = TYPE_RGB_FLT;
        let r: f32 = 0.5;
        let g: f32 = 0.25;
        let b: f32 = 1.0;
        let mut input = [0u8; 12];
        input[0..4].copy_from_slice(&r.to_ne_bytes());
        input[4..8].copy_from_slice(&g.to_ne_bytes());
        input[8..12].copy_from_slice(&b.to_ne_bytes());

        let mut values = [0.0f32; 16];
        let FormatterIn::Float(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_FLOAT).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 12);
        assert!((values[0] - 0.5).abs() < 1e-6);
        assert!((values[1] - 0.25).abs() < 1e-6);
        assert!((values[2] - 1.0).abs() < 1e-6);

        let FormatterOut::Float(pack) = find_formatter_out(format, CMS_PACK_FLAGS_FLOAT).unwrap()
        else {
            panic!()
        };
        let mut output = [0u8; 12];
        let written = pack(format, &values, &mut output, 0);
        assert_eq!(written, 12);
        assert_eq!(output, input);
    }

    // ================================================================
    // Double (LAB_DBL)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_lab_dbl_unroll_to_float() {
        let format = TYPE_LAB_DBL;
        let l: f64 = 50.0;
        let a: f64 = -20.0;
        let b_val: f64 = 30.0;
        let mut input = [0u8; 24];
        input[0..8].copy_from_slice(&l.to_ne_bytes());
        input[8..16].copy_from_slice(&a.to_ne_bytes());
        input[16..24].copy_from_slice(&b_val.to_ne_bytes());

        let mut values = [0.0f32; 16];
        let FormatterIn::Float(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_FLOAT).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 24);
        // Lab values should be normalized to 0..1 range
        // L: 0..100 → 0..1, a/b: -128..+127 → 0..1
        assert!((values[0] - 50.0 / 100.0).abs() < 1e-5);
        assert!((values[1] - ((-20.0 + 128.0) / 255.0)).abs() < 1e-5);
        assert!((values[2] - ((30.0 + 128.0) / 255.0)).abs() < 1e-5);
    }

    // ================================================================
    // Half-float (RGB_HALF_FLT)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_rgb_half_flt_round_trip() {
        use crate::math::half::float_to_half;

        let format = TYPE_RGB_HALF_FLT;
        let r = float_to_half(0.5);
        let g = float_to_half(0.25);
        let b = float_to_half(1.0);
        let mut input = [0u8; 6];
        input[0..2].copy_from_slice(&r.to_ne_bytes());
        input[2..4].copy_from_slice(&g.to_ne_bytes());
        input[4..6].copy_from_slice(&b.to_ne_bytes());

        let mut values = [0.0f32; 16];
        let FormatterIn::Float(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_FLOAT).unwrap()
        else {
            panic!()
        };
        let consumed = unroll(format, &mut values, &input, 0);
        assert_eq!(consumed, 6);
        assert!((values[0] - 0.5).abs() < 0.01);
        assert!((values[1] - 0.25).abs() < 0.01);
        assert!((values[2] - 1.0).abs() < 0.01);
    }

    // ================================================================
    // Lab V2 (LABV2_8, LABV2_16)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_labv2_8_unroll() {
        let format = TYPE_LABV2_8;
        let input: [u8; 3] = [0x80, 0x80, 0x80];
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        unroll(format, &mut values, &input, 0);
        // V2→V4 conversion should scale values
        let v2_16 = from_8_to_16(0x80); // 0x8080
        let expected = lab_v2_to_v4(v2_16);
        assert_eq!(values[0], expected);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_labv2_16_round_trip() {
        let format_in = TYPE_LABV2_16;
        let format_out = TYPE_LABV2_16;
        let input: [u8; 6] = [0x80, 0x00, 0x80, 0x00, 0x80, 0x00];
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format_in, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        unroll(format_in, &mut values, &input, 0);

        let FormatterOut::U16(pack) =
            find_formatter_out(format_out, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        let mut output = [0u8; 6];
        pack(format_out, &values, &mut output, 0);
        // Should round-trip within 1 unit
        for i in 0..6 {
            assert!(
                (output[i] as i16 - input[i] as i16).unsigned_abs() <= 1,
                "byte {i}: input={:#04X}, output={:#04X}",
                input[i],
                output[i]
            );
        }
    }

    // ================================================================
    // Endian swap (RGB_16_SE)
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_rgb_16_se_endian_swap() {
        let format = TYPE_RGB_16_SE;
        // Big-endian bytes for values 0x1234, 0x5678, 0x9ABC
        let input: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
        let mut values = [0u16; 16];

        let FormatterIn::U16(unroll) = find_formatter_in(format, CMS_PACK_FLAGS_16BITS).unwrap()
        else {
            panic!()
        };
        unroll(format, &mut values, &input, 0);
        // On little-endian: native bytes [0x12,0x34] = 0x3412, then swap_bytes → 0x1234
        assert_eq!(values[0], 0x1234);
        assert_eq!(values[1], 0x5678);
        assert_eq!(values[2], 0x9ABC);
    }

    // ================================================================
    // Lookup
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn test_find_formatter_in_returns_some_for_common_formats() {
        assert!(find_formatter_in(TYPE_RGB_8, CMS_PACK_FLAGS_16BITS).is_some());
        assert!(find_formatter_in(TYPE_CMYK_16, CMS_PACK_FLAGS_16BITS).is_some());
        assert!(find_formatter_in(TYPE_GRAY_8, CMS_PACK_FLAGS_16BITS).is_some());
        assert!(find_formatter_in(TYPE_RGB_FLT, CMS_PACK_FLAGS_FLOAT).is_some());
        assert!(find_formatter_in(TYPE_LAB_DBL, CMS_PACK_FLAGS_FLOAT).is_some());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_find_formatter_out_returns_some_for_common_formats() {
        assert!(find_formatter_out(TYPE_RGB_8, CMS_PACK_FLAGS_16BITS).is_some());
        assert!(find_formatter_out(TYPE_CMYK_16, CMS_PACK_FLAGS_16BITS).is_some());
        assert!(find_formatter_out(TYPE_GRAY_8, CMS_PACK_FLAGS_16BITS).is_some());
        assert!(find_formatter_out(TYPE_RGB_FLT, CMS_PACK_FLAGS_FLOAT).is_some());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_find_formatter_in_returns_none_for_zero_channels() {
        let bad = PixelFormat::build(PT_RGB, 0, 1);
        assert!(find_formatter_in(bad, CMS_PACK_FLAGS_16BITS).is_none());
    }
}
