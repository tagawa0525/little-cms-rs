// Tag type handlers: read/write functions for each ICC tag type.
// C版: cmstypes.c

#![allow(dead_code)]

use crate::context::CmsError;
use crate::curves::gamma::ToneCurve;
use crate::pipeline::named::{Mlu, NamedColorList, ProfileSequenceDesc};
use crate::profile::io::IoHandler;
use crate::types::*;

// ============================================================================
// TagData enum
// ============================================================================

/// Decoded tag data. Each variant holds the deserialized content of one ICC tag type.
pub enum TagData {
    Xyz(CieXyz),
    Curve(ToneCurve),
    Mlu(Mlu),
    Signature(u32),
    DateTime(DateTimeNumber),
    Measurement(IccMeasurementConditions),
    ViewingConditions(IccViewingConditions),
    S15Fixed16Array(Vec<f64>),
    U16Fixed16Array(Vec<f64>),
    NamedColor(NamedColorList),
    Chromaticity(CieXyYTriple),
    ColorantOrder(Vec<u8>),
    Data(IccData),
    Screening(Screening),
    UInt8Array(Vec<u8>),
    UInt16Array(Vec<u16>),
    UInt32Array(Vec<u32>),
    UInt64Array(Vec<u64>),
    ProfileSequenceDesc(ProfileSequenceDesc),
    Raw(Vec<u8>),
}

// ============================================================================
// Tag type read/write dispatch
// ============================================================================

/// Read a tag's data given its type signature and payload size.
pub fn read_tag_type(
    _io: &mut IoHandler,
    _sig: TagTypeSignature,
    _size: u32,
) -> Result<TagData, CmsError> {
    todo!()
}

/// Write a tag's data, returning the type signature used.
pub fn write_tag_type(
    _io: &mut IoHandler,
    _data: &TagData,
    _icc_version: u32,
) -> Result<TagTypeSignature, CmsError> {
    todo!()
}

// ============================================================================
// Individual type handlers
// ============================================================================

// --- XYZ type ---
// C版: Type_XYZ_Read / Type_XYZ_Write

pub fn read_xyz_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let xyz = io.read_xyz()?;
    Ok(TagData::Xyz(xyz))
}

pub fn write_xyz_type(io: &mut IoHandler, xyz: &CieXyz) -> Result<(), CmsError> {
    io.write_xyz(xyz)
}

// --- Signature type ---
// C版: Type_Signature_Read / Type_Signature_Write

pub fn read_signature_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let sig = io.read_u32()?;
    Ok(TagData::Signature(sig))
}

pub fn write_signature_type(io: &mut IoHandler, sig: u32) -> Result<(), CmsError> {
    io.write_u32(sig)
}

// --- DateTime type ---
// C版: Type_DateTime_Read / Type_DateTime_Write

pub fn read_datetime_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let dt = DateTimeNumber {
        year: io.read_u16()?,
        month: io.read_u16()?,
        day: io.read_u16()?,
        hours: io.read_u16()?,
        minutes: io.read_u16()?,
        seconds: io.read_u16()?,
    };
    Ok(TagData::DateTime(dt))
}

pub fn write_datetime_type(io: &mut IoHandler, dt: &DateTimeNumber) -> Result<(), CmsError> {
    io.write_u16(dt.year)?;
    io.write_u16(dt.month)?;
    io.write_u16(dt.day)?;
    io.write_u16(dt.hours)?;
    io.write_u16(dt.minutes)?;
    io.write_u16(dt.seconds)?;
    Ok(())
}

// --- S15Fixed16Array type ---
// C版: Type_S15Fixed16_Read / Type_S15Fixed16_Write

pub fn read_s15fixed16_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let count = size as usize / 4;
    let mut arr = Vec::with_capacity(count);
    for _ in 0..count {
        arr.push(io.read_s15fixed16()?);
    }
    Ok(TagData::S15Fixed16Array(arr))
}

pub fn write_s15fixed16_type(io: &mut IoHandler, arr: &[f64]) -> Result<(), CmsError> {
    for &v in arr {
        io.write_s15fixed16(v)?;
    }
    Ok(())
}

// --- U16Fixed16Array type ---
// C版: Type_U16Fixed16_Read / Type_U16Fixed16_Write

pub fn read_u16fixed16_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let count = size as usize / 4;
    let mut arr = Vec::with_capacity(count);
    for _ in 0..count {
        let raw = io.read_u32()?;
        arr.push(raw as f64 / 65536.0);
    }
    Ok(TagData::U16Fixed16Array(arr))
}

pub fn write_u16fixed16_type(io: &mut IoHandler, arr: &[f64]) -> Result<(), CmsError> {
    for &v in arr {
        let raw = (v * 65536.0 + 0.5).floor() as u32;
        io.write_u32(raw)?;
    }
    Ok(())
}

// --- UInt8Array type ---
// C版: Type_UInt8_Read / Type_UInt8_Write

pub fn read_uint8_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let count = size as usize;
    let mut arr = vec![0u8; count];
    if count > 0 && !io.read(&mut arr) {
        return Err(IoHandler::read_err());
    }
    Ok(TagData::UInt8Array(arr))
}

pub fn write_uint8_type(io: &mut IoHandler, arr: &[u8]) -> Result<(), CmsError> {
    if !arr.is_empty() && !io.write(arr) {
        return Err(IoHandler::write_err());
    }
    Ok(())
}

// --- UInt16Array type ---
// C版: Type_UInt16_Read / Type_UInt16_Write

pub fn read_uint16_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let count = size as usize / 2;
    let arr = io.read_u16_array(count)?;
    Ok(TagData::UInt16Array(arr))
}

pub fn write_uint16_type(io: &mut IoHandler, arr: &[u16]) -> Result<(), CmsError> {
    io.write_u16_array(arr)
}

// --- UInt32Array type ---
// C版: Type_UInt32_Read / Type_UInt32_Write

pub fn read_uint32_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let count = size as usize / 4;
    let mut arr = Vec::with_capacity(count);
    for _ in 0..count {
        arr.push(io.read_u32()?);
    }
    Ok(TagData::UInt32Array(arr))
}

pub fn write_uint32_type(io: &mut IoHandler, arr: &[u32]) -> Result<(), CmsError> {
    for &v in arr {
        io.write_u32(v)?;
    }
    Ok(())
}

// --- UInt64Array type ---
// C版: Type_UInt64_Read / Type_UInt64_Write

pub fn read_uint64_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let count = size as usize / 8;
    let mut arr = Vec::with_capacity(count);
    for _ in 0..count {
        arr.push(io.read_u64()?);
    }
    Ok(TagData::UInt64Array(arr))
}

pub fn write_uint64_type(io: &mut IoHandler, arr: &[u64]) -> Result<(), CmsError> {
    for &v in arr {
        io.write_u64(v)?;
    }
    Ok(())
}

// --- Text type ---
// C版: Type_Text_Read / Type_Text_Write
// Reads ASCII text and stores as Mlu.

pub fn read_text_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_text_type(_io: &mut IoHandler, _mlu: &Mlu) -> Result<(), CmsError> {
    todo!()
}

// --- TextDescription type (v2) ---
// C版: Type_Text_Description_Read / Type_Text_Description_Write

pub fn read_text_description_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_text_description_type(_io: &mut IoHandler, _mlu: &Mlu) -> Result<(), CmsError> {
    todo!()
}

// --- MLU type (v4) ---
// C版: Type_MLU_Read / Type_MLU_Write

pub fn read_mlu_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_mlu_type(_io: &mut IoHandler, _mlu: &Mlu) -> Result<(), CmsError> {
    todo!()
}

// --- Curve type ---
// C版: Type_Curve_Read / Type_Curve_Write

pub fn read_curve_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_curve_type(_io: &mut IoHandler, _curve: &ToneCurve) -> Result<(), CmsError> {
    todo!()
}

// --- ParametricCurve type ---
// C版: Type_ParametricCurve_Read / Type_ParametricCurve_Write

pub fn read_parametric_curve_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_parametric_curve_type(
    _io: &mut IoHandler,
    _curve: &ToneCurve,
) -> Result<(), CmsError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // XYZ type round-trip
    // ========================================================================

    #[test]

    fn xyz_type_roundtrip() {
        let xyz = CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        };
        let mut io = IoHandler::from_memory_write(256);
        write_xyz_type(&mut io, &xyz).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_xyz_type(&mut io, size).unwrap();
        match result {
            TagData::Xyz(read_xyz) => {
                assert!((read_xyz.x - xyz.x).abs() < 1.0 / 65536.0);
                assert!((read_xyz.y - xyz.y).abs() < 1.0 / 65536.0);
                assert!((read_xyz.z - xyz.z).abs() < 1.0 / 65536.0);
            }
            _ => panic!("Expected TagData::Xyz"),
        }
    }

    // ========================================================================
    // Signature type round-trip
    // ========================================================================

    #[test]

    fn signature_type_roundtrip() {
        let sig = ColorSpaceSignature::RgbData as u32;
        let mut io = IoHandler::from_memory_write(256);
        write_signature_type(&mut io, sig).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_signature_type(&mut io, size).unwrap();
        match result {
            TagData::Signature(read_sig) => assert_eq!(read_sig, sig),
            _ => panic!("Expected TagData::Signature"),
        }
    }

    // ========================================================================
    // DateTime type round-trip
    // ========================================================================

    #[test]

    fn datetime_type_roundtrip() {
        let dt = DateTimeNumber {
            year: 2024,
            month: 6,
            day: 15,
            hours: 12,
            minutes: 30,
            seconds: 45,
        };
        let mut io = IoHandler::from_memory_write(256);
        write_datetime_type(&mut io, &dt).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_datetime_type(&mut io, size).unwrap();
        match result {
            TagData::DateTime(read_dt) => {
                assert_eq!(read_dt.year, 2024);
                assert_eq!(read_dt.month, 6);
                assert_eq!(read_dt.day, 15);
                assert_eq!(read_dt.hours, 12);
                assert_eq!(read_dt.minutes, 30);
                assert_eq!(read_dt.seconds, 45);
            }
            _ => panic!("Expected TagData::DateTime"),
        }
    }

    // ========================================================================
    // S15Fixed16Array type round-trip
    // ========================================================================

    #[test]

    fn s15fixed16_array_roundtrip() {
        let arr = vec![1.0, -0.5, D50_X, D50_Y, D50_Z];
        let mut io = IoHandler::from_memory_write(256);
        write_s15fixed16_type(&mut io, &arr).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_s15fixed16_type(&mut io, size).unwrap();
        match result {
            TagData::S15Fixed16Array(read_arr) => {
                assert_eq!(read_arr.len(), 5);
                for (a, b) in arr.iter().zip(read_arr.iter()) {
                    assert!((a - b).abs() < 1.0 / 65536.0);
                }
            }
            _ => panic!("Expected TagData::S15Fixed16Array"),
        }
    }

    // ========================================================================
    // U16Fixed16Array type round-trip
    // ========================================================================

    #[test]

    fn u16fixed16_array_roundtrip() {
        let arr = vec![1.0, 2.5, 0.0];
        let mut io = IoHandler::from_memory_write(256);
        write_u16fixed16_type(&mut io, &arr).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_u16fixed16_type(&mut io, size).unwrap();
        match result {
            TagData::U16Fixed16Array(read_arr) => {
                assert_eq!(read_arr.len(), 3);
                for (a, b) in arr.iter().zip(read_arr.iter()) {
                    assert!((a - b).abs() < 1.0 / 65536.0);
                }
            }
            _ => panic!("Expected TagData::U16Fixed16Array"),
        }
    }

    // ========================================================================
    // UInt array type round-trips
    // ========================================================================

    #[test]

    fn uint8_array_roundtrip() {
        let arr: Vec<u8> = vec![1, 2, 3, 4, 5];
        let mut io = IoHandler::from_memory_write(256);
        write_uint8_type(&mut io, &arr).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_uint8_type(&mut io, size).unwrap();
        match result {
            TagData::UInt8Array(read_arr) => assert_eq!(read_arr, arr),
            _ => panic!("Expected TagData::UInt8Array"),
        }
    }

    #[test]

    fn uint16_array_roundtrip() {
        let arr: Vec<u16> = vec![100, 200, 300];
        let mut io = IoHandler::from_memory_write(256);
        write_uint16_type(&mut io, &arr).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_uint16_type(&mut io, size).unwrap();
        match result {
            TagData::UInt16Array(read_arr) => assert_eq!(read_arr, arr),
            _ => panic!("Expected TagData::UInt16Array"),
        }
    }

    #[test]

    fn uint32_array_roundtrip() {
        let arr: Vec<u32> = vec![0xDEADBEEF, 0xCAFEBABE];
        let mut io = IoHandler::from_memory_write(256);
        write_uint32_type(&mut io, &arr).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_uint32_type(&mut io, size).unwrap();
        match result {
            TagData::UInt32Array(read_arr) => assert_eq!(read_arr, arr),
            _ => panic!("Expected TagData::UInt32Array"),
        }
    }

    #[test]

    fn uint64_array_roundtrip() {
        let arr: Vec<u64> = vec![0x0102030405060708, 0xFFEEDDCCBBAA9988];
        let mut io = IoHandler::from_memory_write(256);
        write_uint64_type(&mut io, &arr).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_uint64_type(&mut io, size).unwrap();
        match result {
            TagData::UInt64Array(read_arr) => assert_eq!(read_arr, arr),
            _ => panic!("Expected TagData::UInt64Array"),
        }
    }

    // ========================================================================
    // Text type round-trip
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn text_type_roundtrip() {
        let mut mlu = Mlu::new();
        mlu.set_ascii("en", "US", "Hello World");
        let mut io = IoHandler::from_memory_write(256);
        write_text_type(&mut io, &mlu).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_text_type(&mut io, size).unwrap();
        match result {
            TagData::Mlu(read_mlu) => {
                assert_eq!(
                    read_mlu.get_ascii("en", "US"),
                    Some("Hello World".to_string())
                );
            }
            _ => panic!("Expected TagData::Mlu"),
        }
    }

    // ========================================================================
    // TextDescription type round-trip (v2)
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn text_description_type_roundtrip() {
        let mut mlu = Mlu::new();
        mlu.set_ascii("en", "US", "Test Description");
        let mut io = IoHandler::from_memory_write(512);
        write_text_description_type(&mut io, &mlu).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_text_description_type(&mut io, size).unwrap();
        match result {
            TagData::Mlu(read_mlu) => {
                assert_eq!(
                    read_mlu.get_ascii("en", "US"),
                    Some("Test Description".to_string())
                );
            }
            _ => panic!("Expected TagData::Mlu"),
        }
    }

    // ========================================================================
    // MLU type round-trip (v4)
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn mlu_type_roundtrip() {
        let mut mlu = Mlu::new();
        mlu.set_ascii("en", "US", "English");
        mlu.set_ascii("ja", "JP", "Japanese");
        let mut io = IoHandler::from_memory_write(1024);
        write_mlu_type(&mut io, &mlu).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_mlu_type(&mut io, size).unwrap();
        match result {
            TagData::Mlu(read_mlu) => {
                assert_eq!(read_mlu.get_ascii("en", "US"), Some("English".to_string()));
                assert!(read_mlu.translations_count() >= 2);
            }
            _ => panic!("Expected TagData::Mlu"),
        }
    }

    // ========================================================================
    // Curve type round-trips
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn curve_type_linear() {
        // count=0 → linear curve
        let mut io = IoHandler::from_memory_write(256);
        io.write_u32(0).unwrap(); // count = 0
        let size = io.used_space();
        io.seek(0);
        let result = read_curve_type(&mut io, size).unwrap();
        match result {
            TagData::Curve(curve) => {
                // Linear: f(0.5) ≈ 0.5
                let v = curve.eval_f32(0.5);
                assert!((v - 0.5).abs() < 0.01);
            }
            _ => panic!("Expected TagData::Curve"),
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn curve_type_gamma() {
        // count=1 → single gamma (8.8 fixed point)
        let gamma = 2.2;
        let gamma_fixed = ((gamma * 256.0) + 0.5) as u16;
        let mut io = IoHandler::from_memory_write(256);
        io.write_u32(1).unwrap();
        io.write_u16(gamma_fixed).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_curve_type(&mut io, size).unwrap();
        match result {
            TagData::Curve(curve) => {
                let est = curve.estimate_gamma(0.01);
                assert!((est - gamma).abs() < 0.1);
            }
            _ => panic!("Expected TagData::Curve"),
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn curve_type_tabulated_roundtrip() {
        let table: Vec<u16> = (0..256).map(|i| (i * 257) as u16).collect();
        let curve = ToneCurve::build_tabulated_16(&table).unwrap();
        let mut io = IoHandler::from_memory_write(1024);
        write_curve_type(&mut io, &curve).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_curve_type(&mut io, size).unwrap();
        match result {
            TagData::Curve(read_curve) => {
                assert_eq!(read_curve.table16_len(), 256);
            }
            _ => panic!("Expected TagData::Curve"),
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn parametric_curve_type_roundtrip() {
        // Type 1: gamma only (Y = X^gamma)
        let curve = ToneCurve::build_parametric(1, &[2.2]).unwrap();
        let mut io = IoHandler::from_memory_write(256);
        write_parametric_curve_type(&mut io, &curve).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_parametric_curve_type(&mut io, size).unwrap();
        match result {
            TagData::Curve(read_curve) => {
                let est = read_curve.estimate_gamma(0.01);
                assert!((est - 2.2).abs() < 0.1);
            }
            _ => panic!("Expected TagData::Curve"),
        }
    }
}
