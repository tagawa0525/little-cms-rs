// ============================================================================
// Alpha (extra) channel handling (C版: cmsalpha.c)
// ============================================================================

use crate::types::PixelFormat;

/// Copy extra channels from input to output pixels.
///
/// Called from `Transform::do_transform` when `FLAGS_COPY_ALPHA` is set.
/// Only copies if input and output have the same number of extra channels.
/// Handles byte-width conversion between 8-bit and 16-bit channels.
///
/// C版: `_cmsHandleExtraChannels`
pub fn handle_extra_channels(
    input_format: PixelFormat,
    output_format: PixelFormat,
    input: &[u8],
    output: &mut [u8],
    pixel_count: usize,
) {
    let n_extra = input_format.extra() as usize;
    if n_extra == 0 {
        return;
    }
    if input_format.extra() != output_format.extra() {
        return;
    }

    let in_bytes = true_bytes_size(input_format);
    let out_bytes = true_bytes_size(output_format);
    let in_channels = input_format.channels() as usize;
    let out_channels = output_format.channels() as usize;

    // Total bytes per pixel
    let in_pixel_size = (in_channels + n_extra) * in_bytes;
    let out_pixel_size = (out_channels + n_extra) * out_bytes;

    if in_pixel_size == 0 || out_pixel_size == 0 {
        return;
    }

    // Offset to first extra channel within each pixel
    let in_extra_offset = in_channels * in_bytes;
    let out_extra_offset = out_channels * out_bytes;

    let max_pixels = (input.len() / in_pixel_size)
        .min(output.len() / out_pixel_size)
        .min(pixel_count);

    for px in 0..max_pixels {
        let in_base = px * in_pixel_size + in_extra_offset;
        let out_base = px * out_pixel_size + out_extra_offset;

        for ch in 0..n_extra {
            let in_off = in_base + ch * in_bytes;
            let out_off = out_base + ch * out_bytes;

            copy_channel(
                &input[in_off..],
                in_bytes,
                &mut output[out_off..],
                out_bytes,
            );
        }
    }
}

/// Copy a single channel value with byte-width conversion.
fn copy_channel(src: &[u8], src_bytes: usize, dst: &mut [u8], dst_bytes: usize) {
    if src_bytes == dst_bytes {
        // Same width: direct copy
        dst[..src_bytes].copy_from_slice(&src[..src_bytes]);
        return;
    }

    match (src_bytes, dst_bytes) {
        // 8-bit → 16-bit: v * 257 (0xFF → 0xFFFF)
        (1, 2) => {
            let v = src[0] as u16;
            let v16 = v | (v << 8); // equivalent to v * 257
            let bytes = v16.to_ne_bytes();
            dst[0] = bytes[0];
            dst[1] = bytes[1];
        }
        // 16-bit → 8-bit: (v + 128) / 257
        (2, 1) => {
            let v = u16::from_ne_bytes([src[0], src[1]]);
            dst[0] = ((v as u32 + 128) / 257) as u8;
        }
        // Other conversions: read as normalized f64, write back
        _ => {
            let normalized = read_normalized(src, src_bytes);
            write_normalized(dst, dst_bytes, normalized);
        }
    }
}

/// Read a channel value as a normalized f64 in [0, 1].
fn read_normalized(src: &[u8], bytes: usize) -> f64 {
    match bytes {
        1 => src[0] as f64 / 255.0,
        2 => u16::from_ne_bytes([src[0], src[1]]) as f64 / 65535.0,
        4 => f32::from_ne_bytes([src[0], src[1], src[2], src[3]]) as f64,
        8 => f64::from_ne_bytes([
            src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
        ]),
        _ => 0.0,
    }
}

/// Write a normalized f64 [0, 1] as a channel value.
fn write_normalized(dst: &mut [u8], bytes: usize, val: f64) {
    match bytes {
        1 => dst[0] = (val * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
        2 => {
            let v = (val * 65535.0 + 0.5).clamp(0.0, 65535.0) as u16;
            let b = v.to_ne_bytes();
            dst[0] = b[0];
            dst[1] = b[1];
        }
        4 => {
            let b = (val as f32).to_ne_bytes();
            dst[..4].copy_from_slice(&b);
        }
        8 => {
            let b = val.to_ne_bytes();
            dst[..8].copy_from_slice(&b);
        }
        _ => {}
    }
}

/// Return the true byte size per channel (bytes=0 means double = 8).
fn true_bytes_size(format: PixelFormat) -> usize {
    let b = format.bytes();
    if b == 0 { 8 } else { b as usize }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::io::Profile;
    use crate::types::*;

    fn roundtrip(p: &mut Profile) -> Profile {
        let data = p.save_to_mem().unwrap();
        Profile::open_mem(&data).unwrap()
    }

    // ================================================================
    // handle_extra_channels
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn alpha_copy_rgba8_to_rgba8() {
        // RGBA 8-bit: alpha should be copied as-is
        let input: [u8; 8] = [255, 0, 0, 128, 0, 255, 0, 200]; // 2 pixels
        let mut output = [0u8; 8];
        // Pre-fill output color channels (simulating transform already ran)
        output[0] = 100;
        output[1] = 100;
        output[2] = 100;
        output[4] = 50;
        output[5] = 50;
        output[6] = 50;

        handle_extra_channels(TYPE_RGBA_8, TYPE_RGBA_8, &input, &mut output, 2);

        // Alpha bytes should be copied
        assert_eq!(output[3], 128, "pixel 0 alpha");
        assert_eq!(output[7], 200, "pixel 1 alpha");
        // Color channels should be untouched
        assert_eq!(output[0], 100);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn alpha_no_copy_when_extra_mismatch() {
        // RGBA_8 → RGB_8: different extra count, no copy
        let input: [u8; 4] = [255, 0, 0, 128];
        let mut output = [0u8; 3];

        handle_extra_channels(TYPE_RGBA_8, TYPE_RGB_8, &input, &mut output, 1);

        // Output should be untouched
        assert_eq!(output, [0, 0, 0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn alpha_no_copy_when_no_extra() {
        let input: [u8; 3] = [255, 128, 64];
        let mut output = [0u8; 3];

        handle_extra_channels(TYPE_RGB_8, TYPE_RGB_8, &input, &mut output, 1);

        // No extra channels, output untouched
        assert_eq!(output, [0, 0, 0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn alpha_copy_16bit_to_8bit() {
        // RGBA_16 → RGBA_8: 16-bit alpha converted to 8-bit
        let alpha_16: u16 = 0x8000; // ~50%
        let alpha_bytes = alpha_16.to_ne_bytes();
        let input: [u8; 8] = [
            0xFF,
            0xFF, // R
            0x00,
            0x00, // G
            0x00,
            0x00, // B
            alpha_bytes[0],
            alpha_bytes[1], // A = 0x8000
        ];
        let mut output = [0u8; 4]; // RGBA_8

        handle_extra_channels(TYPE_RGBA_16, TYPE_RGBA_8, &input, &mut output, 1);

        // 0x8000 → (0x8000 + 128) / 257 ≈ 127
        let expected = ((0x8000u32 + 128) / 257) as u8;
        assert!(
            (output[3] as i16 - expected as i16).unsigned_abs() <= 1,
            "expected ~{}, got {}",
            expected,
            output[3]
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn alpha_copy_8bit_to_16bit() {
        // RGBA_8 → RGBA_16: 8-bit alpha expanded to 16-bit
        let input: [u8; 4] = [255, 0, 0, 200]; // RGBA_8
        let mut output = [0u8; 8]; // RGBA_16

        handle_extra_channels(TYPE_RGBA_8, TYPE_RGBA_16, &input, &mut output, 1);

        // 200 * 257 = 0xC8C8
        let alpha_16 = u16::from_ne_bytes([output[6], output[7]]);
        assert_eq!(
            alpha_16,
            200 * 257,
            "expected 0xC8C8, got {:#06X}",
            alpha_16
        );
    }

    // ================================================================
    // Integration: Transform with FLAGS_COPY_ALPHA
    // ================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn transform_copies_alpha_with_flag() {
        use super::super::xform::{FLAGS_COPY_ALPHA, Transform};

        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());

        let xform = Transform::new(
            src,
            TYPE_RGBA_8,
            dst,
            TYPE_RGBA_8,
            1,                // relative colorimetric
            FLAGS_COPY_ALPHA, // enable alpha copy
        )
        .unwrap();

        let input: [u8; 4] = [128, 128, 128, 200];
        let mut output = [0u8; 4];
        xform.do_transform(&input, &mut output, 1);

        // Alpha should be preserved
        assert_eq!(output[3], 200, "alpha not copied");
        // Color channels should be transformed (approximately identity for same profile)
        assert!(output[0] > 100, "color channel too low: {}", output[0]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn transform_no_alpha_without_flag() {
        use super::super::xform::Transform;

        let src = roundtrip(&mut Profile::new_srgb());
        let dst = roundtrip(&mut Profile::new_srgb());

        let xform = Transform::new(
            src,
            TYPE_RGBA_8,
            dst,
            TYPE_RGBA_8,
            1, // relative colorimetric
            0, // no FLAGS_COPY_ALPHA
        )
        .unwrap();

        let input: [u8; 4] = [128, 128, 128, 200];
        let mut output = [0u8; 4];
        xform.do_transform(&input, &mut output, 1);

        // Alpha should NOT be 200 (it'll be 0 or whatever the transform produces)
        // The pipeline only handles 3 color channels, alpha byte is untouched (0)
        assert_eq!(output[3], 0, "alpha should not be copied without flag");
    }
}
