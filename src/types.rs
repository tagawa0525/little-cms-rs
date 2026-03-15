/// 15.16 signed fixed-point number (15 integer bits + 16 fractional bits).
///
/// C版: `cmsS15Fixed16Number` (i32)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct S15Fixed16(pub i32);

/// 16.16 unsigned fixed-point number (16 integer bits + 16 fractional bits).
///
/// C版: `cmsU16Fixed16Number` (u32)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U16Fixed16(pub u32);

/// 8.8 unsigned fixed-point number (8 integer bits + 8 fractional bits).
///
/// C版: `cmsU8Fixed8Number` (u16)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct U8Fixed8(pub u16);

impl From<f64> for S15Fixed16 {
    fn from(_value: f64) -> Self {
        todo!()
    }
}

impl From<S15Fixed16> for f64 {
    fn from(_fixed: S15Fixed16) -> Self {
        todo!()
    }
}

impl From<f64> for U16Fixed16 {
    fn from(_value: f64) -> Self {
        todo!()
    }
}

impl From<U16Fixed16> for f64 {
    fn from(_fixed: U16Fixed16) -> Self {
        todo!()
    }
}

impl From<f64> for U8Fixed8 {
    fn from(_value: f64) -> Self {
        todo!()
    }
}

impl From<U8Fixed8> for f64 {
    fn from(_fixed: U8Fixed8) -> Self {
        todo!()
    }
}

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
    #[ignore = "not yet implemented"]
    fn s15fixed16_round_trip() {
        // C版 CheckFixedPoint15_16 の11テスト値
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
    #[ignore = "not yet implemented"]
    fn u8fixed8_round_trip() {
        // C版 CheckFixedPoint8_8 の6テスト値
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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

        // ICC仕様の高精度D50値
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
    #[ignore = "not yet implemented"]
    fn type_sizes() {
        assert_eq!(size_of::<S15Fixed16>(), 4);
        assert_eq!(size_of::<U16Fixed16>(), 4);
        assert_eq!(size_of::<U8Fixed8>(), 2);
    }
}
