// ============================================================================
// Fixed-point types
// ============================================================================

/// 15.16 signed fixed-point number (15 integer bits + 16 fractional bits).
///
/// C版: `cmsS15Fixed16Number` (i32)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct S15Fixed16(pub i32);

/// 16.16 unsigned fixed-point number (16 integer bits + 16 fractional bits).
///
/// C版: `cmsU16Fixed16Number` (u32)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct U16Fixed16(pub u32);

/// 8.8 unsigned fixed-point number (8 integer bits + 8 fractional bits).
///
/// C版: `cmsU8Fixed8Number` (u16)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct U8Fixed8(pub u16);

// C版: _cmsDoubleTo15Fixed16(v) = floor(v * 65536.0 + 0.5) as i32
impl From<f64> for S15Fixed16 {
    fn from(value: f64) -> Self {
        Self((value * 65536.0 + 0.5).floor() as i32)
    }
}

// C版: _cms15Fixed16toDouble(fix32) = fix32 / 65536.0
impl From<S15Fixed16> for f64 {
    fn from(fixed: S15Fixed16) -> Self {
        fixed.0 as f64 / 65536.0
    }
}

impl From<f64> for U16Fixed16 {
    fn from(value: f64) -> Self {
        Self((value * 65536.0 + 0.5).floor() as u32)
    }
}

impl From<U16Fixed16> for f64 {
    fn from(fixed: U16Fixed16) -> Self {
        fixed.0 as f64 / 65536.0
    }
}

// C版: _cmsDoubleTo8Fixed8(val) = (_cmsDoubleTo15Fixed16(val) >> 8) & 0xFFFF
impl From<f64> for U8Fixed8 {
    fn from(value: f64) -> Self {
        let fixed32 = S15Fixed16::from(value);
        Self(((fixed32.0 >> 8) & 0xFFFF) as u16)
    }
}

// C版: _cms8Fixed8toDouble(fixed8) = fixed8 / 256.0
impl From<U8Fixed8> for f64 {
    fn from(fixed: U8Fixed8) -> Self {
        fixed.0 as f64 / 256.0
    }
}

// ============================================================================
// Pixel type constants
// ============================================================================

pub const PT_ANY: u32 = 0;
pub const PT_GRAY: u32 = 3;
pub const PT_RGB: u32 = 4;
pub const PT_CMY: u32 = 5;
pub const PT_CMYK: u32 = 6;
pub const PT_YCBCR: u32 = 7;
pub const PT_YUV: u32 = 8;
pub const PT_XYZ: u32 = 9;
pub const PT_LAB: u32 = 10;
pub const PT_YUVK: u32 = 11;
pub const PT_HSV: u32 = 12;
pub const PT_HLS: u32 = 13;
pub const PT_YXY: u32 = 14;
pub const PT_MCH1: u32 = 15;
pub const PT_MCH2: u32 = 16;
pub const PT_MCH3: u32 = 17;
pub const PT_MCH4: u32 = 18;
pub const PT_MCH5: u32 = 19;
pub const PT_MCH6: u32 = 20;
pub const PT_MCH7: u32 = 21;
pub const PT_MCH8: u32 = 22;
pub const PT_MCH9: u32 = 23;
pub const PT_MCH10: u32 = 24;
pub const PT_MCH11: u32 = 25;
pub const PT_MCH12: u32 = 26;
pub const PT_MCH13: u32 = 27;
pub const PT_MCH14: u32 = 28;
pub const PT_MCH15: u32 = 29;
pub const PT_LAB_V2: u32 = 30;

// ============================================================================
// PixelFormat
// ============================================================================

/// Pixel format descriptor packed into a u32 bitfield.
///
/// C版: `COLORSPACE_SH` / `CHANNELS_SH` / `BYTES_SH` 等のマクロ群に対応。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelFormat(pub u32);

impl PixelFormat {
    // --- Builders (const fn, for constructing TYPE_* constants) ---

    pub const fn build(colorspace: u32, channels: u32, bytes: u32) -> Self {
        Self((colorspace << 16) | (channels << 3) | bytes)
    }
    pub const fn with_extra(self, e: u32) -> Self {
        Self(self.0 | (e << 7))
    }
    pub const fn with_doswap(self) -> Self {
        Self(self.0 | (1 << 10))
    }
    pub const fn with_swapfirst(self) -> Self {
        Self(self.0 | (1 << 14))
    }
    pub const fn with_flavor(self) -> Self {
        Self(self.0 | (1 << 13))
    }
    pub const fn with_planar(self) -> Self {
        Self(self.0 | (1 << 12))
    }
    pub const fn with_endian16(self) -> Self {
        Self(self.0 | (1 << 11))
    }
    pub const fn with_float(self) -> Self {
        Self(self.0 | (1 << 22))
    }
    pub const fn with_premul(self) -> Self {
        Self(self.0 | (1 << 23))
    }

    // --- Getters (matching C版 T_XXX macros) ---

    pub const fn colorspace(&self) -> u32 {
        (self.0 >> 16) & 31
    }
    pub const fn channels(&self) -> u32 {
        (self.0 >> 3) & 15
    }
    pub const fn bytes(&self) -> u32 {
        self.0 & 7
    }
    pub const fn extra(&self) -> u32 {
        (self.0 >> 7) & 7
    }
    pub const fn doswap(&self) -> u32 {
        (self.0 >> 10) & 1
    }
    pub const fn swapfirst(&self) -> u32 {
        (self.0 >> 14) & 1
    }
    pub const fn flavor(&self) -> u32 {
        (self.0 >> 13) & 1
    }
    pub const fn planar(&self) -> u32 {
        (self.0 >> 12) & 1
    }
    pub const fn endian16(&self) -> u32 {
        (self.0 >> 11) & 1
    }
    pub const fn is_float(&self) -> bool {
        ((self.0 >> 22) & 1) != 0
    }
    pub const fn optimized(&self) -> u32 {
        (self.0 >> 21) & 1
    }
    pub const fn premul(&self) -> u32 {
        (self.0 >> 23) & 1
    }
}

// ============================================================================
// Predefined pixel format constants (TYPE_*)
// ============================================================================

// --- Gray ---
pub const TYPE_GRAY_8: PixelFormat = PixelFormat::build(PT_GRAY, 1, 1);
pub const TYPE_GRAY_8_REV: PixelFormat = PixelFormat::build(PT_GRAY, 1, 1).with_flavor();
pub const TYPE_GRAY_16: PixelFormat = PixelFormat::build(PT_GRAY, 1, 2);
pub const TYPE_GRAY_16_REV: PixelFormat = PixelFormat::build(PT_GRAY, 1, 2).with_flavor();
pub const TYPE_GRAY_16_SE: PixelFormat = PixelFormat::build(PT_GRAY, 1, 2).with_endian16();
pub const TYPE_GRAYA_8: PixelFormat = PixelFormat::build(PT_GRAY, 1, 1).with_extra(1);
pub const TYPE_GRAYA_8_PREMUL: PixelFormat = PixelFormat::build(PT_GRAY, 1, 1)
    .with_extra(1)
    .with_premul();
pub const TYPE_GRAYA_16: PixelFormat = PixelFormat::build(PT_GRAY, 1, 2).with_extra(1);
pub const TYPE_GRAYA_16_PREMUL: PixelFormat = PixelFormat::build(PT_GRAY, 1, 2)
    .with_extra(1)
    .with_premul();
pub const TYPE_GRAYA_16_SE: PixelFormat = PixelFormat::build(PT_GRAY, 1, 2)
    .with_extra(1)
    .with_endian16();
pub const TYPE_GRAYA_8_PLANAR: PixelFormat = PixelFormat::build(PT_GRAY, 1, 1)
    .with_extra(1)
    .with_planar();
pub const TYPE_GRAYA_16_PLANAR: PixelFormat = PixelFormat::build(PT_GRAY, 1, 2)
    .with_extra(1)
    .with_planar();

// --- RGB ---
pub const TYPE_RGB_8: PixelFormat = PixelFormat::build(PT_RGB, 3, 1);
pub const TYPE_RGB_8_PLANAR: PixelFormat = PixelFormat::build(PT_RGB, 3, 1).with_planar();
pub const TYPE_BGR_8: PixelFormat = PixelFormat::build(PT_RGB, 3, 1).with_doswap();
pub const TYPE_BGR_8_PLANAR: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 1).with_doswap().with_planar();
pub const TYPE_RGB_16: PixelFormat = PixelFormat::build(PT_RGB, 3, 2);
pub const TYPE_RGB_16_PLANAR: PixelFormat = PixelFormat::build(PT_RGB, 3, 2).with_planar();
pub const TYPE_RGB_16_SE: PixelFormat = PixelFormat::build(PT_RGB, 3, 2).with_endian16();
pub const TYPE_BGR_16: PixelFormat = PixelFormat::build(PT_RGB, 3, 2).with_doswap();
pub const TYPE_BGR_16_PLANAR: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 2).with_doswap().with_planar();
pub const TYPE_BGR_16_SE: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_doswap()
    .with_endian16();

// --- RGBA ---
pub const TYPE_RGBA_8: PixelFormat = PixelFormat::build(PT_RGB, 3, 1).with_extra(1);
pub const TYPE_RGBA_8_PREMUL: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 1).with_extra(1).with_premul();
pub const TYPE_RGBA_8_PLANAR: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 1).with_extra(1).with_planar();
pub const TYPE_RGBA_16: PixelFormat = PixelFormat::build(PT_RGB, 3, 2).with_extra(1);
pub const TYPE_RGBA_16_PREMUL: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 2).with_extra(1).with_premul();
pub const TYPE_RGBA_16_PLANAR: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 2).with_extra(1).with_planar();
pub const TYPE_RGBA_16_SE: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_endian16();

// --- ARGB ---
pub const TYPE_ARGB_8: PixelFormat = PixelFormat::build(PT_RGB, 3, 1)
    .with_extra(1)
    .with_swapfirst();
pub const TYPE_ARGB_8_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 1)
    .with_extra(1)
    .with_swapfirst()
    .with_premul();
pub const TYPE_ARGB_8_PLANAR: PixelFormat = PixelFormat::build(PT_RGB, 3, 1)
    .with_extra(1)
    .with_swapfirst()
    .with_planar();
pub const TYPE_ARGB_16: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_swapfirst();
pub const TYPE_ARGB_16_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_swapfirst()
    .with_premul();

// --- ABGR ---
pub const TYPE_ABGR_8: PixelFormat = PixelFormat::build(PT_RGB, 3, 1).with_extra(1).with_doswap();
pub const TYPE_ABGR_8_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 1)
    .with_extra(1)
    .with_doswap()
    .with_premul();
pub const TYPE_ABGR_8_PLANAR: PixelFormat = PixelFormat::build(PT_RGB, 3, 1)
    .with_extra(1)
    .with_doswap()
    .with_planar();
pub const TYPE_ABGR_16: PixelFormat = PixelFormat::build(PT_RGB, 3, 2).with_extra(1).with_doswap();
pub const TYPE_ABGR_16_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_doswap()
    .with_premul();
pub const TYPE_ABGR_16_PLANAR: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_doswap()
    .with_planar();
pub const TYPE_ABGR_16_SE: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_doswap()
    .with_endian16();

// --- BGRA ---
pub const TYPE_BGRA_8: PixelFormat = PixelFormat::build(PT_RGB, 3, 1)
    .with_extra(1)
    .with_doswap()
    .with_swapfirst();
pub const TYPE_BGRA_8_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 1)
    .with_extra(1)
    .with_doswap()
    .with_swapfirst()
    .with_premul();
pub const TYPE_BGRA_8_PLANAR: PixelFormat = PixelFormat::build(PT_RGB, 3, 1)
    .with_extra(1)
    .with_doswap()
    .with_swapfirst()
    .with_planar();
pub const TYPE_BGRA_16: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_doswap()
    .with_swapfirst();
pub const TYPE_BGRA_16_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_doswap()
    .with_swapfirst()
    .with_premul();
pub const TYPE_BGRA_16_SE: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_extra(1)
    .with_endian16()
    .with_doswap()
    .with_swapfirst();

// --- CMY ---
pub const TYPE_CMY_8: PixelFormat = PixelFormat::build(PT_CMY, 3, 1);
pub const TYPE_CMY_8_PLANAR: PixelFormat = PixelFormat::build(PT_CMY, 3, 1).with_planar();
pub const TYPE_CMY_16: PixelFormat = PixelFormat::build(PT_CMY, 3, 2);
pub const TYPE_CMY_16_PLANAR: PixelFormat = PixelFormat::build(PT_CMY, 3, 2).with_planar();
pub const TYPE_CMY_16_SE: PixelFormat = PixelFormat::build(PT_CMY, 3, 2).with_endian16();

// --- CMYK ---
pub const TYPE_CMYK_8: PixelFormat = PixelFormat::build(PT_CMYK, 4, 1);
pub const TYPE_CMYKA_8: PixelFormat = PixelFormat::build(PT_CMYK, 4, 1).with_extra(1);
pub const TYPE_CMYK_8_REV: PixelFormat = PixelFormat::build(PT_CMYK, 4, 1).with_flavor();
pub const TYPE_YUVK_8: PixelFormat = TYPE_CMYK_8_REV;
pub const TYPE_CMYK_8_PLANAR: PixelFormat = PixelFormat::build(PT_CMYK, 4, 1).with_planar();
pub const TYPE_CMYK_16: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2);
pub const TYPE_CMYK_16_REV: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2).with_flavor();
pub const TYPE_YUVK_16: PixelFormat = TYPE_CMYK_16_REV;
pub const TYPE_CMYK_16_PLANAR: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2).with_planar();
pub const TYPE_CMYK_16_SE: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2).with_endian16();
pub const TYPE_KYMC_8: PixelFormat = PixelFormat::build(PT_CMYK, 4, 1).with_doswap();
pub const TYPE_KYMC_16: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2).with_doswap();
pub const TYPE_KYMC_16_SE: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2)
    .with_doswap()
    .with_endian16();
pub const TYPE_KCMY_8: PixelFormat = PixelFormat::build(PT_CMYK, 4, 1).with_swapfirst();
pub const TYPE_KCMY_8_REV: PixelFormat = PixelFormat::build(PT_CMYK, 4, 1)
    .with_flavor()
    .with_swapfirst();
pub const TYPE_KCMY_16: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2).with_swapfirst();
pub const TYPE_KCMY_16_REV: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2)
    .with_flavor()
    .with_swapfirst();
pub const TYPE_KCMY_16_SE: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2)
    .with_endian16()
    .with_swapfirst();

// --- Multi-channel (5-12) ---
pub const TYPE_CMYK5_8: PixelFormat = PixelFormat::build(PT_MCH5, 5, 1);
pub const TYPE_CMYK5_16: PixelFormat = PixelFormat::build(PT_MCH5, 5, 2);
pub const TYPE_CMYK5_16_SE: PixelFormat = PixelFormat::build(PT_MCH5, 5, 2).with_endian16();
pub const TYPE_KYMC5_8: PixelFormat = PixelFormat::build(PT_MCH5, 5, 1).with_doswap();
pub const TYPE_KYMC5_16: PixelFormat = PixelFormat::build(PT_MCH5, 5, 2).with_doswap();
pub const TYPE_KYMC5_16_SE: PixelFormat = PixelFormat::build(PT_MCH5, 5, 2)
    .with_doswap()
    .with_endian16();

pub const TYPE_CMYK6_8: PixelFormat = PixelFormat::build(PT_MCH6, 6, 1);
pub const TYPE_CMYK6_8_PLANAR: PixelFormat = PixelFormat::build(PT_MCH6, 6, 1).with_planar();
pub const TYPE_CMYK6_16: PixelFormat = PixelFormat::build(PT_MCH6, 6, 2);
pub const TYPE_CMYK6_16_PLANAR: PixelFormat = PixelFormat::build(PT_MCH6, 6, 2).with_planar();
pub const TYPE_CMYK6_16_SE: PixelFormat = PixelFormat::build(PT_MCH6, 6, 2).with_endian16();

pub const TYPE_CMYK7_8: PixelFormat = PixelFormat::build(PT_MCH7, 7, 1);
pub const TYPE_CMYK7_16: PixelFormat = PixelFormat::build(PT_MCH7, 7, 2);
pub const TYPE_CMYK7_16_SE: PixelFormat = PixelFormat::build(PT_MCH7, 7, 2).with_endian16();
pub const TYPE_KYMC7_8: PixelFormat = PixelFormat::build(PT_MCH7, 7, 1).with_doswap();
pub const TYPE_KYMC7_16: PixelFormat = PixelFormat::build(PT_MCH7, 7, 2).with_doswap();
pub const TYPE_KYMC7_16_SE: PixelFormat = PixelFormat::build(PT_MCH7, 7, 2)
    .with_doswap()
    .with_endian16();

pub const TYPE_CMYK8_8: PixelFormat = PixelFormat::build(PT_MCH8, 8, 1);
pub const TYPE_CMYK8_16: PixelFormat = PixelFormat::build(PT_MCH8, 8, 2);
pub const TYPE_CMYK8_16_SE: PixelFormat = PixelFormat::build(PT_MCH8, 8, 2).with_endian16();
pub const TYPE_KYMC8_8: PixelFormat = PixelFormat::build(PT_MCH8, 8, 1).with_doswap();
pub const TYPE_KYMC8_16: PixelFormat = PixelFormat::build(PT_MCH8, 8, 2).with_doswap();
pub const TYPE_KYMC8_16_SE: PixelFormat = PixelFormat::build(PT_MCH8, 8, 2)
    .with_doswap()
    .with_endian16();

pub const TYPE_CMYK9_8: PixelFormat = PixelFormat::build(PT_MCH9, 9, 1);
pub const TYPE_CMYK9_16: PixelFormat = PixelFormat::build(PT_MCH9, 9, 2);
pub const TYPE_CMYK9_16_SE: PixelFormat = PixelFormat::build(PT_MCH9, 9, 2).with_endian16();
pub const TYPE_KYMC9_8: PixelFormat = PixelFormat::build(PT_MCH9, 9, 1).with_doswap();
pub const TYPE_KYMC9_16: PixelFormat = PixelFormat::build(PT_MCH9, 9, 2).with_doswap();
pub const TYPE_KYMC9_16_SE: PixelFormat = PixelFormat::build(PT_MCH9, 9, 2)
    .with_doswap()
    .with_endian16();

pub const TYPE_CMYK10_8: PixelFormat = PixelFormat::build(PT_MCH10, 10, 1);
pub const TYPE_CMYK10_16: PixelFormat = PixelFormat::build(PT_MCH10, 10, 2);
pub const TYPE_CMYK10_16_SE: PixelFormat = PixelFormat::build(PT_MCH10, 10, 2).with_endian16();
pub const TYPE_KYMC10_8: PixelFormat = PixelFormat::build(PT_MCH10, 10, 1).with_doswap();
pub const TYPE_KYMC10_16: PixelFormat = PixelFormat::build(PT_MCH10, 10, 2).with_doswap();
pub const TYPE_KYMC10_16_SE: PixelFormat = PixelFormat::build(PT_MCH10, 10, 2)
    .with_doswap()
    .with_endian16();

pub const TYPE_CMYK11_8: PixelFormat = PixelFormat::build(PT_MCH11, 11, 1);
pub const TYPE_CMYK11_16: PixelFormat = PixelFormat::build(PT_MCH11, 11, 2);
pub const TYPE_CMYK11_16_SE: PixelFormat = PixelFormat::build(PT_MCH11, 11, 2).with_endian16();
pub const TYPE_KYMC11_8: PixelFormat = PixelFormat::build(PT_MCH11, 11, 1).with_doswap();
pub const TYPE_KYMC11_16: PixelFormat = PixelFormat::build(PT_MCH11, 11, 2).with_doswap();
pub const TYPE_KYMC11_16_SE: PixelFormat = PixelFormat::build(PT_MCH11, 11, 2)
    .with_doswap()
    .with_endian16();

pub const TYPE_CMYK12_8: PixelFormat = PixelFormat::build(PT_MCH12, 12, 1);
pub const TYPE_CMYK12_16: PixelFormat = PixelFormat::build(PT_MCH12, 12, 2);
pub const TYPE_CMYK12_16_SE: PixelFormat = PixelFormat::build(PT_MCH12, 12, 2).with_endian16();
pub const TYPE_KYMC12_8: PixelFormat = PixelFormat::build(PT_MCH12, 12, 1).with_doswap();
pub const TYPE_KYMC12_16: PixelFormat = PixelFormat::build(PT_MCH12, 12, 2).with_doswap();
pub const TYPE_KYMC12_16_SE: PixelFormat = PixelFormat::build(PT_MCH12, 12, 2)
    .with_doswap()
    .with_endian16();

// --- XYZ / Lab / Yxy ---
pub const TYPE_XYZ_16: PixelFormat = PixelFormat::build(PT_XYZ, 3, 2);
pub const TYPE_LAB_8: PixelFormat = PixelFormat::build(PT_LAB, 3, 1);
pub const TYPE_LABV2_8: PixelFormat = PixelFormat::build(PT_LAB_V2, 3, 1);
pub const TYPE_ALAB_8: PixelFormat = PixelFormat::build(PT_LAB, 3, 1)
    .with_extra(1)
    .with_swapfirst();
pub const TYPE_ALABV2_8: PixelFormat = PixelFormat::build(PT_LAB_V2, 3, 1)
    .with_extra(1)
    .with_swapfirst();
pub const TYPE_LAB_16: PixelFormat = PixelFormat::build(PT_LAB, 3, 2);
pub const TYPE_LABV2_16: PixelFormat = PixelFormat::build(PT_LAB_V2, 3, 2);
pub const TYPE_YXY_16: PixelFormat = PixelFormat::build(PT_YXY, 3, 2);

// --- YCbCr ---
pub const TYPE_YCBCR_8: PixelFormat = PixelFormat::build(PT_YCBCR, 3, 1);
pub const TYPE_YCBCR_8_PLANAR: PixelFormat = PixelFormat::build(PT_YCBCR, 3, 1).with_planar();
pub const TYPE_YCBCR_16: PixelFormat = PixelFormat::build(PT_YCBCR, 3, 2);
pub const TYPE_YCBCR_16_PLANAR: PixelFormat = PixelFormat::build(PT_YCBCR, 3, 2).with_planar();
pub const TYPE_YCBCR_16_SE: PixelFormat = PixelFormat::build(PT_YCBCR, 3, 2).with_endian16();

// --- YUV ---
pub const TYPE_YUV_8: PixelFormat = PixelFormat::build(PT_YUV, 3, 1);
pub const TYPE_YUV_8_PLANAR: PixelFormat = PixelFormat::build(PT_YUV, 3, 1).with_planar();
pub const TYPE_YUV_16: PixelFormat = PixelFormat::build(PT_YUV, 3, 2);
pub const TYPE_YUV_16_PLANAR: PixelFormat = PixelFormat::build(PT_YUV, 3, 2).with_planar();
pub const TYPE_YUV_16_SE: PixelFormat = PixelFormat::build(PT_YUV, 3, 2).with_endian16();

// --- HLS ---
pub const TYPE_HLS_8: PixelFormat = PixelFormat::build(PT_HLS, 3, 1);
pub const TYPE_HLS_8_PLANAR: PixelFormat = PixelFormat::build(PT_HLS, 3, 1).with_planar();
pub const TYPE_HLS_16: PixelFormat = PixelFormat::build(PT_HLS, 3, 2);
pub const TYPE_HLS_16_PLANAR: PixelFormat = PixelFormat::build(PT_HLS, 3, 2).with_planar();
pub const TYPE_HLS_16_SE: PixelFormat = PixelFormat::build(PT_HLS, 3, 2).with_endian16();

// --- HSV ---
pub const TYPE_HSV_8: PixelFormat = PixelFormat::build(PT_HSV, 3, 1);
pub const TYPE_HSV_8_PLANAR: PixelFormat = PixelFormat::build(PT_HSV, 3, 1).with_planar();
pub const TYPE_HSV_16: PixelFormat = PixelFormat::build(PT_HSV, 3, 2);
pub const TYPE_HSV_16_PLANAR: PixelFormat = PixelFormat::build(PT_HSV, 3, 2).with_planar();
pub const TYPE_HSV_16_SE: PixelFormat = PixelFormat::build(PT_HSV, 3, 2).with_endian16();

// --- Named color ---
pub const TYPE_NAMED_COLOR_INDEX: PixelFormat = PixelFormat::build(0, 1, 2);

// --- Float (32-bit) ---
pub const TYPE_XYZ_FLT: PixelFormat = PixelFormat::build(PT_XYZ, 3, 4).with_float();
pub const TYPE_LAB_FLT: PixelFormat = PixelFormat::build(PT_LAB, 3, 4).with_float();
pub const TYPE_LABA_FLT: PixelFormat = PixelFormat::build(PT_LAB, 3, 4).with_float().with_extra(1);
pub const TYPE_GRAY_FLT: PixelFormat = PixelFormat::build(PT_GRAY, 1, 4).with_float();
pub const TYPE_GRAYA_FLT: PixelFormat =
    PixelFormat::build(PT_GRAY, 1, 4).with_float().with_extra(1);
pub const TYPE_GRAYA_FLT_PREMUL: PixelFormat = PixelFormat::build(PT_GRAY, 1, 4)
    .with_float()
    .with_extra(1)
    .with_premul();
pub const TYPE_RGB_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 4).with_float();
pub const TYPE_RGBA_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 4).with_float().with_extra(1);
pub const TYPE_RGBA_FLT_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 4)
    .with_float()
    .with_extra(1)
    .with_premul();
pub const TYPE_ARGB_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 4)
    .with_float()
    .with_extra(1)
    .with_swapfirst();
pub const TYPE_ARGB_FLT_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 4)
    .with_float()
    .with_extra(1)
    .with_swapfirst()
    .with_premul();
pub const TYPE_BGR_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 4).with_float().with_doswap();
pub const TYPE_BGRA_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 4)
    .with_float()
    .with_extra(1)
    .with_doswap()
    .with_swapfirst();
pub const TYPE_BGRA_FLT_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 4)
    .with_float()
    .with_extra(1)
    .with_doswap()
    .with_swapfirst()
    .with_premul();
pub const TYPE_ABGR_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 4)
    .with_float()
    .with_extra(1)
    .with_doswap();
pub const TYPE_ABGR_FLT_PREMUL: PixelFormat = PixelFormat::build(PT_RGB, 3, 4)
    .with_float()
    .with_extra(1)
    .with_doswap()
    .with_premul();
pub const TYPE_CMYK_FLT: PixelFormat = PixelFormat::build(PT_CMYK, 4, 4).with_float();

// --- Double (64-bit) ---
pub const TYPE_XYZ_DBL: PixelFormat = PixelFormat::build(PT_XYZ, 3, 0).with_float();
pub const TYPE_LAB_DBL: PixelFormat = PixelFormat::build(PT_LAB, 3, 0).with_float();
pub const TYPE_GRAY_DBL: PixelFormat = PixelFormat::build(PT_GRAY, 1, 0).with_float();
pub const TYPE_RGB_DBL: PixelFormat = PixelFormat::build(PT_RGB, 3, 0).with_float();
pub const TYPE_BGR_DBL: PixelFormat = PixelFormat::build(PT_RGB, 3, 0).with_float().with_doswap();
pub const TYPE_CMYK_DBL: PixelFormat = PixelFormat::build(PT_CMYK, 4, 0).with_float();
pub const TYPE_OKLAB_DBL: PixelFormat = PixelFormat::build(PT_MCH3, 3, 0).with_float();

// --- Half-float (16-bit float) ---
pub const TYPE_GRAY_HALF_FLT: PixelFormat = PixelFormat::build(PT_GRAY, 1, 2).with_float();
pub const TYPE_RGB_HALF_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 2).with_float();
pub const TYPE_CMYK_HALF_FLT: PixelFormat = PixelFormat::build(PT_CMYK, 4, 2).with_float();
pub const TYPE_RGBA_HALF_FLT: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 2).with_float().with_extra(1);
pub const TYPE_ARGB_HALF_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_float()
    .with_extra(1)
    .with_swapfirst();
pub const TYPE_BGR_HALF_FLT: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 2).with_float().with_doswap();
pub const TYPE_BGRA_HALF_FLT: PixelFormat = PixelFormat::build(PT_RGB, 3, 2)
    .with_float()
    .with_extra(1)
    .with_doswap()
    .with_swapfirst();
pub const TYPE_ABGR_HALF_FLT: PixelFormat =
    PixelFormat::build(PT_RGB, 3, 2).with_float().with_doswap();

// ============================================================================
// ICC signature enums
// ============================================================================

macro_rules! icc_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident = $value:expr),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u32)]
        $vis enum $name {
            $($variant = $value),*
        }

        impl TryFrom<u32> for $name {
            type Error = u32;
            fn try_from(value: u32) -> Result<Self, Self::Error> {
                match value {
                    $($value => Ok(Self::$variant),)*
                    _ => Err(value),
                }
            }
        }
    };
}

icc_enum! {
    /// ICC tag type signatures.
    pub enum TagTypeSignature {
        Chromaticity = 0x6368726D,
        Cicp = 0x63696370,
        ColorantOrder = 0x636C726F,
        ColorantTable = 0x636C7274,
        CrdInfo = 0x63726469,
        Curve = 0x63757276,
        Data = 0x64617461,
        Dict = 0x64696374,
        DateTime = 0x6474696D,
        DeviceSettings = 0x64657673,
        Lut16 = 0x6D667432,
        Lut8 = 0x6D667431,
        LutAtoB = 0x6D414220,
        LutBtoA = 0x6D424120,
        Measurement = 0x6D656173,
        MultiLocalizedUnicode = 0x6D6C7563,
        MultiProcessElement = 0x6D706574,
        NamedColor = 0x6E636F6C,
        NamedColor2 = 0x6E636C32,
        ParametricCurve = 0x70617261,
        ProfileSequenceDesc = 0x70736571,
        ProfileSequenceId = 0x70736964,
        ResponseCurveSet16 = 0x72637332,
        S15Fixed16Array = 0x73663332,
        Screening = 0x7363726E,
        Signature = 0x73696720,
        Text = 0x74657874,
        TextDescription = 0x64657363,
        U16Fixed16Array = 0x75663332,
        UcrBg = 0x62666420,
        UInt16Array = 0x75693136,
        UInt32Array = 0x75693332,
        UInt64Array = 0x75693634,
        UInt8Array = 0x75693038,
        Vcgt = 0x76636774,
        ViewingConditions = 0x76696577,
        Xyz = 0x58595A20,
        Mhc2 = 0x4D484332,
    }
}

icc_enum! {
    /// ICC tag signatures.
    pub enum TagSignature {
        AToB0 = 0x41324230,
        AToB1 = 0x41324231,
        AToB2 = 0x41324232,
        BlueMatrixColumn = 0x6258595A,
        BlueTRC = 0x62545243,
        BToA0 = 0x42324130,
        BToA1 = 0x42324131,
        BToA2 = 0x42324132,
        CalibrationDateTime = 0x63616C74,
        CharTarget = 0x74617267,
        ChromaticAdaptation = 0x63686164,
        Chromaticity = 0x6368726D,
        ColorantOrder = 0x636C726F,
        ColorantTable = 0x636C7274,
        ColorantTableOut = 0x636C6F74,
        ColorimetricIntentImageState = 0x63696973,
        Copyright = 0x63707274,
        CrdInfo = 0x63726469,
        Data = 0x64617461,
        DateTime = 0x6474696D,
        DeviceMfgDesc = 0x646D6E64,
        DeviceModelDesc = 0x646D6464,
        DeviceSettings = 0x64657673,
        DToB0 = 0x44324230,
        DToB1 = 0x44324231,
        DToB2 = 0x44324232,
        DToB3 = 0x44324233,
        BToD0 = 0x42324430,
        BToD1 = 0x42324431,
        BToD2 = 0x42324432,
        BToD3 = 0x42324433,
        Gamut = 0x67616D74,
        GrayTRC = 0x6B545243,
        GreenMatrixColumn = 0x6758595A,
        GreenTRC = 0x67545243,
        Luminance = 0x6C756D69,
        Measurement = 0x6D656173,
        MediaBlackPoint = 0x626B7074,
        MediaWhitePoint = 0x77747074,
        NamedColor = 0x6E636F6C,
        NamedColor2 = 0x6E636C32,
        OutputResponse = 0x72657370,
        PerceptualRenderingIntentGamut = 0x72696730,
        Preview0 = 0x70726530,
        Preview1 = 0x70726531,
        Preview2 = 0x70726532,
        ProfileDescription = 0x64657363,
        ProfileDescriptionML = 0x6473636D,
        ProfileSequenceDesc = 0x70736571,
        ProfileSequenceId = 0x70736964,
        Ps2CRD0 = 0x70736430,
        Ps2CRD1 = 0x70736431,
        Ps2CRD2 = 0x70736432,
        Ps2CRD3 = 0x70736433,
        Ps2CSA = 0x70733273,
        Ps2RenderingIntent = 0x70733269,
        RedMatrixColumn = 0x7258595A,
        RedTRC = 0x72545243,
        SaturationRenderingIntentGamut = 0x72696732,
        ScreeningDesc = 0x73637264,
        Screening = 0x7363726E,
        Technology = 0x74656368,
        UcrBg = 0x62666420,
        ViewingCondDesc = 0x76756564,
        ViewingConditions = 0x76696577,
        Vcgt = 0x76636774,
        Meta = 0x6D657461,
        Cicp = 0x63696370,
        ArgyllArts = 0x61727473,
        Mhc2 = 0x4D484332,
    }
}

// TagSignature v2 aliases (same value as v4 MatrixColumn names)
impl TagSignature {
    pub const BLUE_COLORANT: Self = Self::BlueMatrixColumn;
    pub const GREEN_COLORANT: Self = Self::GreenMatrixColumn;
    pub const RED_COLORANT: Self = Self::RedMatrixColumn;
}

icc_enum! {
    /// ICC color space signatures.
    pub enum ColorSpaceSignature {
        XyzData = 0x58595A20,
        LabData = 0x4C616220,
        LuvData = 0x4C757620,
        YCbCrData = 0x59436272,
        YxyData = 0x59787920,
        RgbData = 0x52474220,
        GrayData = 0x47524159,
        HsvData = 0x48535620,
        HlsData = 0x484C5320,
        CmykData = 0x434D594B,
        CmyData = 0x434D5920,
        Mch1Data = 0x4D434831,
        Mch2Data = 0x4D434832,
        Mch3Data = 0x4D434833,
        Mch4Data = 0x4D434834,
        Mch5Data = 0x4D434835,
        Mch6Data = 0x4D434836,
        Mch7Data = 0x4D434837,
        Mch8Data = 0x4D434838,
        Mch9Data = 0x4D434839,
        MchAData = 0x4D434841,
        MchBData = 0x4D434842,
        MchCData = 0x4D434843,
        MchDData = 0x4D434844,
        MchEData = 0x4D434845,
        MchFData = 0x4D434846,
        NamedData = 0x6E6D636C,
        Color1 = 0x31434C52,
        Color2 = 0x32434C52,
        Color3 = 0x33434C52,
        Color4 = 0x34434C52,
        Color5 = 0x35434C52,
        Color6 = 0x36434C52,
        Color7 = 0x37434C52,
        Color8 = 0x38434C52,
        Color9 = 0x39434C52,
        Color10 = 0x41434C52,
        Color11 = 0x42434C52,
        Color12 = 0x43434C52,
        Color13 = 0x44434C52,
        Color14 = 0x45434C52,
        Color15 = 0x46434C52,
        LuvKData = 0x4C75764B,
    }
}

impl ColorSpaceSignature {
    /// Convert ICC color space signature to PixelFormat colorspace index (PT_*).
    /// C版: `_cmsLCMScolorSpace`
    pub fn to_pixel_type(&self) -> u32 {
        match self {
            Self::GrayData => PT_GRAY,
            Self::RgbData => PT_RGB,
            Self::CmyData => PT_CMY,
            Self::CmykData => PT_CMYK,
            Self::YCbCrData => PT_YCBCR,
            Self::YxyData => PT_YXY,
            Self::XyzData => PT_XYZ,
            Self::LabData => PT_LAB,
            Self::LuvKData => PT_YUVK,
            Self::HsvData => PT_HSV,
            Self::HlsData => PT_HLS,
            Self::LuvData => PT_YUV,
            Self::Mch1Data | Self::Color1 => PT_MCH1,
            Self::Mch2Data | Self::Color2 => PT_MCH2,
            Self::Mch3Data | Self::Color3 => PT_MCH3,
            Self::Mch4Data | Self::Color4 => PT_MCH4,
            Self::Mch5Data | Self::Color5 => PT_MCH5,
            Self::Mch6Data | Self::Color6 => PT_MCH6,
            Self::Mch7Data | Self::Color7 => PT_MCH7,
            Self::Mch8Data | Self::Color8 => PT_MCH8,
            Self::Mch9Data | Self::Color9 => PT_MCH9,
            Self::MchAData | Self::Color10 => PT_MCH10,
            Self::MchBData | Self::Color11 => PT_MCH11,
            Self::MchCData | Self::Color12 => PT_MCH12,
            Self::MchDData | Self::Color13 => PT_MCH13,
            Self::MchEData | Self::Color14 => PT_MCH14,
            Self::MchFData | Self::Color15 => PT_MCH15,
            Self::NamedData => PT_ANY,
        }
    }

    /// Convert PixelFormat colorspace index (PT_*) to ICC color space signature.
    /// C版: `_cmsICCcolorSpace`
    pub fn from_pixel_type(pt: u32) -> Option<Self> {
        match pt {
            PT_GRAY => Some(Self::GrayData),
            PT_RGB => Some(Self::RgbData),
            PT_CMY => Some(Self::CmyData),
            PT_CMYK => Some(Self::CmykData),
            PT_YCBCR => Some(Self::YCbCrData),
            PT_YUV => Some(Self::LuvData),
            PT_XYZ => Some(Self::XyzData),
            PT_LAB => Some(Self::LabData),
            PT_YUVK => Some(Self::LuvKData),
            PT_HSV => Some(Self::HsvData),
            PT_HLS => Some(Self::HlsData),
            PT_YXY => Some(Self::YxyData),
            PT_MCH1 => Some(Self::Mch1Data),
            PT_MCH2 => Some(Self::Mch2Data),
            PT_MCH3 => Some(Self::Mch3Data),
            PT_MCH4 => Some(Self::Mch4Data),
            PT_MCH5 => Some(Self::Mch5Data),
            PT_MCH6 => Some(Self::Mch6Data),
            PT_MCH7 => Some(Self::Mch7Data),
            PT_MCH8 => Some(Self::Mch8Data),
            PT_MCH9 => Some(Self::Mch9Data),
            PT_MCH10 => Some(Self::MchAData),
            PT_MCH11 => Some(Self::MchBData),
            PT_MCH12 => Some(Self::MchCData),
            PT_MCH13 => Some(Self::MchDData),
            PT_MCH14 => Some(Self::MchEData),
            PT_MCH15 => Some(Self::MchFData),
            _ => None,
        }
    }

    /// Return the number of channels for a given color space.
    /// C版: `cmsChannelsOfColorSpace`
    pub fn channels(&self) -> u32 {
        match self {
            Self::GrayData => 1,
            Self::XyzData
            | Self::LabData
            | Self::LuvData
            | Self::YCbCrData
            | Self::YxyData
            | Self::RgbData
            | Self::HsvData
            | Self::HlsData
            | Self::CmyData
            | Self::Mch3Data
            | Self::Color3 => 3,
            Self::CmykData | Self::Mch4Data | Self::Color4 => 4,
            Self::Mch5Data | Self::Color5 => 5,
            Self::Mch6Data | Self::Color6 => 6,
            Self::Mch7Data | Self::Color7 => 7,
            Self::Mch8Data | Self::Color8 => 8,
            Self::Mch9Data | Self::Color9 => 9,
            Self::MchAData | Self::Color10 => 10,
            Self::MchBData | Self::Color11 => 11,
            Self::MchCData | Self::Color12 => 12,
            Self::MchDData | Self::Color13 => 13,
            Self::MchEData | Self::Color14 => 14,
            Self::MchFData | Self::Color15 => 15,
            Self::Mch1Data | Self::Color1 | Self::NamedData => 1,
            Self::Mch2Data | Self::Color2 => 2,
            Self::LuvKData => 4,
        }
    }
}

icc_enum! {
    /// ICC profile class signatures.
    pub enum ProfileClassSignature {
        Input = 0x73636E72,
        Display = 0x6D6E7472,
        Output = 0x70727472,
        Link = 0x6C696E6B,
        Abstract = 0x61627374,
        ColorSpace = 0x73706163,
        NamedColor = 0x6E6D636C,
        ColorEncodingSpace = 0x63656E63,
        MultiplexIdentification = 0x6D696420,
        MultiplexLink = 0x6D6C6E6B,
        MultiplexVisualization = 0x6D766973,
    }
}

icc_enum! {
    /// ICC technology signatures.
    pub enum TechnologySignature {
        DigitalCamera = 0x6463616D,
        FilmScanner = 0x6673636E,
        ReflectiveScanner = 0x7273636E,
        InkJetPrinter = 0x696A6574,
        ThermalWaxPrinter = 0x74776178,
        ElectrophotographicPrinter = 0x6570686F,
        ElectrostaticPrinter = 0x65737461,
        DyeSublimationPrinter = 0x64737562,
        PhotographicPaperPrinter = 0x7270686F,
        FilmWriter = 0x6670726E,
        VideoMonitor = 0x7669646D,
        VideoCamera = 0x76696463,
        ProjectionTelevision = 0x706A7476,
        CrtDisplay = 0x43525420,
        PmDisplay = 0x504D4420,
        AmDisplay = 0x414D4420,
        PhotoCd = 0x4B504344,
        PhotoImageSetter = 0x696D6773,
        Gravure = 0x67726176,
        OffsetLithography = 0x6F666673,
        Silkscreen = 0x73696C6B,
        Flexography = 0x666C6578,
        MotionPictureFilmScanner = 0x6D706673,
        MotionPictureFilmRecorder = 0x6D706672,
        DigitalMotionPictureCamera = 0x646D7063,
        DigitalCinemaProjector = 0x64636A70,
    }
}

icc_enum! {
    /// ICC platform signatures.
    pub enum PlatformSignature {
        Macintosh = 0x4150504C,
        Microsoft = 0x4D534654,
        Solaris = 0x53554E57,
        Sgi = 0x53474920,
        Taligent = 0x54474E54,
        Unices = 0x2A6E6978,
    }
}

icc_enum! {
    /// Pipeline stage element type signatures.
    pub enum StageSignature {
        CurveSetElem = 0x63767374,
        MatrixElem = 0x6D617466,
        CLutElem = 0x636C7574,
        BAcsElem = 0x62414353,
        EAcsElem = 0x65414353,
        Xyz2LabElem = 0x6C327820,
        Lab2XyzElem = 0x78326C20,
        NamedColorElem = 0x6E636C20,
        LabV2toV4 = 0x32203420,
        LabV4toV2 = 0x34203220,
        IdentityElem = 0x69646E20,
        Lab2FloatPCS = 0x64326C20,
        FloatPCS2Lab = 0x6C326420,
        Xyz2FloatPCS = 0x64327820,
        FloatPCS2Xyz = 0x78326420,
        ClipNegativesElem = 0x636C7020,
    }
}

icc_enum! {
    /// Curve segment type signatures.
    pub enum CurveSegSignature {
        FormulaCurveSeg = 0x70617266,
        SampledCurveSeg = 0x73616D66,
        SegmentedCurve = 0x63757266,
    }
}

// ============================================================================
// Color space structures
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CieXyz {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CieXyY {
    pub x: f64,
    pub y: f64,
    pub big_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CieLab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CieLCh {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct JCh {
    pub j: f64,
    pub c: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CieXyzTriple {
    pub red: CieXyz,
    pub green: CieXyz,
    pub blue: CieXyz,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CieXyYTriple {
    pub red: CieXyY,
    pub green: CieXyY,
    pub blue: CieXyY,
}

// ============================================================================
// Constants
// ============================================================================

pub const D50_X: f64 = 0.9642;
pub const D50_Y: f64 = 1.0;
pub const D50_Z: f64 = 0.8249;

pub const PERCEPTUAL_BLACK_X: f64 = 0.00336;
pub const PERCEPTUAL_BLACK_Y: f64 = 0.0034731;
pub const PERCEPTUAL_BLACK_Z: f64 = 0.00287;

pub const MAX_CHANNELS: usize = 16;
pub const VERSION: u32 = 2190;
pub const ICC_MAGIC_NUMBER: u32 = 0x61637370; // 'acsp'
pub const LCMS_SIGNATURE: u32 = 0x6C636D73; // 'lcms'

// ============================================================================
// Rendering intent
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Intent {
    Perceptual = 0,
    RelativeColorimetric = 1,
    Saturation = 2,
    AbsoluteColorimetric = 3,
    PreserveKOnlyPerceptual = 10,
    PreserveKOnlyRelativeColorimetric = 11,
    PreserveKOnlySaturation = 12,
    PreserveKPlanePerceptual = 13,
    PreserveKPlaneRelativeColorimetric = 14,
    PreserveKPlaneSaturation = 15,
}

impl TryFrom<u32> for Intent {
    type Error = u32;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Perceptual),
            1 => Ok(Self::RelativeColorimetric),
            2 => Ok(Self::Saturation),
            3 => Ok(Self::AbsoluteColorimetric),
            10 => Ok(Self::PreserveKOnlyPerceptual),
            11 => Ok(Self::PreserveKOnlyRelativeColorimetric),
            12 => Ok(Self::PreserveKOnlySaturation),
            13 => Ok(Self::PreserveKPlanePerceptual),
            14 => Ok(Self::PreserveKPlaneRelativeColorimetric),
            15 => Ok(Self::PreserveKPlaneSaturation),
            _ => Err(value),
        }
    }
}

/// Used direction constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UsedDirection {
    AsInput = 0,
    AsOutput = 1,
    AsProof = 2,
}

// ============================================================================
// ICC structures
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct DateTimeNumber {
    pub year: u16,
    pub month: u16,
    pub day: u16,
    pub hours: u16,
    pub minutes: u16,
    pub seconds: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct EncodedXyzNumber {
    pub x: S15Fixed16,
    pub y: S15Fixed16,
    pub z: S15Fixed16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ProfileId(pub [u8; 16]);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IccHeader {
    pub size: u32,
    pub cmm_id: u32,
    pub version: u32,
    pub device_class: ProfileClassSignature,
    pub color_space: ColorSpaceSignature,
    pub pcs: ColorSpaceSignature,
    pub date: DateTimeNumber,
    pub magic: u32,
    pub platform: PlatformSignature,
    pub flags: u32,
    pub manufacturer: u32,
    pub model: u32,
    pub attributes: u64,
    pub rendering_intent: u32,
    pub illuminant: EncodedXyzNumber,
    pub creator: u32,
    pub profile_id: ProfileId,
    pub reserved: [u8; 28],
}

// Compile-time assertion: ICC header must be exactly 128 bytes per ICC spec.
const _: () = assert!(size_of::<IccHeader>() == 128);

// ============================================================================
// ICC tag data structures
// ============================================================================

/// ICC measurement conditions.
/// C版: `cmsICCMeasurementConditions`
#[derive(Debug, Clone, Default)]
pub struct IccMeasurementConditions {
    pub observer: u32,
    pub backing: CieXyz,
    pub geometry: u32,
    pub flare: f64,
    pub illuminant_type: u32,
}

/// ICC viewing conditions.
/// C版: `cmsICCViewingConditions`
#[derive(Debug, Clone, Default)]
pub struct IccViewingConditions {
    pub illuminant: CieXyz,
    pub surround: CieXyz,
    pub illuminant_type: u32,
}

/// ICC data (binary or ASCII blob).
/// C版: `cmsICCData`
#[derive(Debug, Clone)]
pub struct IccData {
    pub flags: u32,
    pub data: Vec<u8>,
}

/// Screening channel definition.
/// C版: `cmsScreeningChannel`
#[derive(Debug, Clone, Copy, Default)]
pub struct ScreeningChannel {
    pub frequency: f64,
    pub screen_angle: f64,
    pub spot_shape: u32,
}

/// Screening information.
/// C版: `cmsScreening`
#[derive(Debug, Clone)]
pub struct Screening {
    pub flags: u32,
    pub channels: Vec<ScreeningChannel>,
}

/// Video signal type (CICP / ITU-R BT.2100).
/// C版: `cmsVideoSignalType`
#[derive(Debug, Clone, Copy, Default)]
pub struct VideoSignalType {
    pub colour_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
    pub video_full_range_flag: u8,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
#[allow(clippy::excessive_precision)]
mod tests {
    use super::*;

    const FIXED_PRECISION_15_16: f64 = 1.0 / 65535.0;
    const FIXED_PRECISION_8_8: f64 = 1.0 / 255.0;

    // --- S15Fixed16 round-trip tests (C版テストスイートから移植) ---

    fn check_s15fixed16_round_trip(value: f64) {
        let fixed = S15Fixed16::from(value);
        let round_trip: f64 = fixed.into();
        let error = (value - round_trip).abs();
        assert!(
            error <= FIXED_PRECISION_15_16,
            "S15Fixed16 round-trip failed for {value}: error {error} > {FIXED_PRECISION_15_16}"
        );
    }

    #[test]
    fn s15fixed16_round_trip() {
        check_s15fixed16_round_trip(1.0);
        check_s15fixed16_round_trip(2.0);
        check_s15fixed16_round_trip(1.23456);
        check_s15fixed16_round_trip(0.99999);
        check_s15fixed16_round_trip(0.1234567890123456789099999);
        check_s15fixed16_round_trip(-1.0);
        check_s15fixed16_round_trip(-2.0);
        check_s15fixed16_round_trip(-1.23456);
        check_s15fixed16_round_trip(-1.1234567890123456789099999);
        check_s15fixed16_round_trip(32767.1234567890123456789099999);
        check_s15fixed16_round_trip(-32767.1234567890123456789099999);
    }

    // --- U8Fixed8 round-trip tests (C版テストスイートから移植) ---

    fn check_u8fixed8_round_trip(value: f64) {
        let fixed = U8Fixed8::from(value);
        let round_trip: f64 = fixed.into();
        let error = (value - round_trip).abs();
        assert!(
            error <= FIXED_PRECISION_8_8,
            "U8Fixed8 round-trip failed for {value}: error {error} > {FIXED_PRECISION_8_8}"
        );
    }

    #[test]
    fn u8fixed8_round_trip() {
        check_u8fixed8_round_trip(1.0);
        check_u8fixed8_round_trip(2.0);
        check_u8fixed8_round_trip(1.23456);
        check_u8fixed8_round_trip(0.99999);
        check_u8fixed8_round_trip(0.1234567890123456789099999);
        check_u8fixed8_round_trip(255.1234567890123456789099999);
    }

    // --- U16Fixed16 round-trip tests (独自テスト) ---

    fn check_u16fixed16_round_trip(value: f64) {
        let fixed = U16Fixed16::from(value);
        let round_trip: f64 = fixed.into();
        let error = (value - round_trip).abs();
        assert!(
            error <= FIXED_PRECISION_15_16,
            "U16Fixed16 round-trip failed for {value}: error {error} > {FIXED_PRECISION_15_16}"
        );
    }

    #[test]
    fn u16fixed16_round_trip() {
        check_u16fixed16_round_trip(0.0);
        check_u16fixed16_round_trip(1.0);
        check_u16fixed16_round_trip(2.0);
        check_u16fixed16_round_trip(1.23456);
        check_u16fixed16_round_trip(0.99999);
        check_u16fixed16_round_trip(65535.99999);
    }

    // --- D50定数 round-trip tests (C版 CheckD50Roundtrip から移植) ---

    #[test]
    fn d50_round_trip() {
        const D50_X: f64 = 0.9642;
        const D50_Y: f64 = 1.0;
        const D50_Z: f64 = 0.8249;

        let xe = S15Fixed16::from(D50_X);
        let ye = S15Fixed16::from(D50_Y);
        let ze = S15Fixed16::from(D50_Z);

        let x: f64 = xe.into();
        let y: f64 = ye.into();
        let z: f64 = ze.into();

        let dx = D50_X - x;
        let dy = D50_Y - y;
        let dz = D50_Z - z;
        let euc = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!(euc <= 1e-5, "D50 round-trip |err| = {euc}");

        const D50_X2: f64 = 0.96420288;
        const D50_Y2: f64 = 1.0;
        const D50_Z2: f64 = 0.82490540;

        let xe = S15Fixed16::from(D50_X2);
        let ye = S15Fixed16::from(D50_Y2);
        let ze = S15Fixed16::from(D50_Z2);

        let x: f64 = xe.into();
        let y: f64 = ye.into();
        let z: f64 = ze.into();

        let dx = D50_X2 - x;
        let dy = D50_Y2 - y;
        let dz = D50_Z2 - z;
        let euc = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!(euc <= 1e-5, "D50 (high-precision) round-trip |err| = {euc}");
    }

    // --- 型サイズ確認 ---

    #[test]
    fn type_sizes() {
        assert_eq!(size_of::<S15Fixed16>(), 4);
        assert_eq!(size_of::<U16Fixed16>(), 4);
        assert_eq!(size_of::<U8Fixed8>(), 2);
        assert_eq!(size_of::<PixelFormat>(), 4);
    }

    // --- PixelFormat field extraction tests ---

    #[test]
    fn pixel_format_field_extraction() {
        // TYPE_RGB_8
        assert_eq!(TYPE_RGB_8.colorspace(), PT_RGB);
        assert_eq!(TYPE_RGB_8.channels(), 3);
        assert_eq!(TYPE_RGB_8.bytes(), 1);
        assert!(!TYPE_RGB_8.is_float());
        assert_eq!(TYPE_RGB_8.planar(), 0);
        assert_eq!(TYPE_RGB_8.extra(), 0);
        assert_eq!(TYPE_RGB_8.doswap(), 0);
        assert_eq!(TYPE_RGB_8.swapfirst(), 0);

        // TYPE_CMYK_16
        assert_eq!(TYPE_CMYK_16.colorspace(), PT_CMYK);
        assert_eq!(TYPE_CMYK_16.channels(), 4);
        assert_eq!(TYPE_CMYK_16.bytes(), 2);

        // TYPE_RGBA_8: extra=1
        assert_eq!(TYPE_RGBA_8.extra(), 1);

        // TYPE_LAB_FLT: float=true
        assert!(TYPE_LAB_FLT.is_float());
        assert_eq!(TYPE_LAB_FLT.colorspace(), PT_LAB);

        // TYPE_BGR_8: doswap=1
        assert_eq!(TYPE_BGR_8.doswap(), 1);

        // TYPE_ARGB_8: swapfirst=1
        assert_eq!(TYPE_ARGB_8.swapfirst(), 1);

        // TYPE_GRAY_8_REV: flavor=1
        assert_eq!(TYPE_GRAY_8_REV.flavor(), 1);

        // TYPE_RGB_8_PLANAR: planar=1
        assert_eq!(TYPE_RGB_8_PLANAR.planar(), 1);

        // TYPE_GRAY_16_SE: endian16=1
        assert_eq!(TYPE_GRAY_16_SE.endian16(), 1);

        // TYPE_RGBA_8_PREMUL: premul=1
        assert_eq!(TYPE_RGBA_8_PREMUL.premul(), 1);

        // Double format: bytes=0, float=true
        assert!(TYPE_XYZ_DBL.is_float());
        assert_eq!(TYPE_XYZ_DBL.bytes(), 0);
    }

    // --- ICC signature TryFrom<u32> round-trip tests ---

    #[test]
    fn color_space_signature_round_trip() {
        let variants = [
            ColorSpaceSignature::XyzData,
            ColorSpaceSignature::LabData,
            ColorSpaceSignature::RgbData,
            ColorSpaceSignature::GrayData,
            ColorSpaceSignature::CmykData,
            ColorSpaceSignature::CmyData,
            ColorSpaceSignature::HsvData,
            ColorSpaceSignature::HlsData,
            ColorSpaceSignature::YCbCrData,
            ColorSpaceSignature::LuvData,
            ColorSpaceSignature::YxyData,
            ColorSpaceSignature::NamedData,
            ColorSpaceSignature::Color1,
            ColorSpaceSignature::Color15,
            ColorSpaceSignature::LuvKData,
        ];
        for sig in variants {
            let val = sig as u32;
            let round_trip = ColorSpaceSignature::try_from(val).unwrap();
            assert_eq!(sig, round_trip);
        }
    }

    #[test]
    fn color_space_signature_invalid() {
        assert!(ColorSpaceSignature::try_from(0xDEADBEEF).is_err());
    }

    #[test]
    fn profile_class_signature_round_trip() {
        let variants = [
            ProfileClassSignature::Input,
            ProfileClassSignature::Display,
            ProfileClassSignature::Output,
            ProfileClassSignature::Link,
            ProfileClassSignature::Abstract,
            ProfileClassSignature::ColorSpace,
            ProfileClassSignature::NamedColor,
        ];
        for sig in variants {
            let val = sig as u32;
            let round_trip = ProfileClassSignature::try_from(val).unwrap();
            assert_eq!(sig, round_trip);
        }
    }

    #[test]
    fn tag_type_signature_round_trip() {
        let variants = [
            TagTypeSignature::Curve,
            TagTypeSignature::ParametricCurve,
            TagTypeSignature::Xyz,
            TagTypeSignature::Text,
            TagTypeSignature::MultiLocalizedUnicode,
            TagTypeSignature::LutAtoB,
            TagTypeSignature::LutBtoA,
        ];
        for sig in variants {
            let val = sig as u32;
            let round_trip = TagTypeSignature::try_from(val).unwrap();
            assert_eq!(sig, round_trip);
        }
    }

    #[test]
    fn tag_signature_aliases() {
        assert_eq!(TagSignature::BLUE_COLORANT, TagSignature::BlueMatrixColumn);
        assert_eq!(
            TagSignature::GREEN_COLORANT,
            TagSignature::GreenMatrixColumn
        );
        assert_eq!(TagSignature::RED_COLORANT, TagSignature::RedMatrixColumn);
    }

    // --- ICC tag data structure tests ---

    #[test]

    fn measurement_conditions_default() {
        let m = IccMeasurementConditions::default();
        assert_eq!(m.observer, 0);
        assert_eq!(m.geometry, 0);
        assert_eq!(m.flare, 0.0);
        assert_eq!(m.illuminant_type, 0);
    }

    #[test]

    fn viewing_conditions_default() {
        let v = IccViewingConditions::default();
        assert_eq!(v.illuminant_type, 0);
        assert_eq!(v.illuminant.x, 0.0);
    }

    #[test]

    fn icc_data_clone() {
        let d = IccData {
            flags: 1,
            data: vec![0xCA, 0xFE],
        };
        let d2 = d.clone();
        assert_eq!(d2.flags, 1);
        assert_eq!(d2.data, vec![0xCA, 0xFE]);
    }

    #[test]

    fn screening_channel_default() {
        let c = ScreeningChannel::default();
        assert_eq!(c.frequency, 0.0);
        assert_eq!(c.screen_angle, 0.0);
        assert_eq!(c.spot_shape, 0);
    }

    #[test]

    fn screening_clone() {
        let s = Screening {
            flags: 0x01,
            channels: vec![ScreeningChannel {
                frequency: 133.0,
                screen_angle: 45.0,
                spot_shape: 1,
            }],
        };
        let s2 = s.clone();
        assert_eq!(s2.flags, 0x01);
        assert_eq!(s2.channels.len(), 1);
        assert_eq!(s2.channels[0].frequency, 133.0);
    }
}
