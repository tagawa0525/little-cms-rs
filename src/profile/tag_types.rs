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
// Individual type handlers (stubs)
// ============================================================================

pub fn read_xyz_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_xyz_type(_io: &mut IoHandler, _xyz: &CieXyz) -> Result<(), CmsError> {
    todo!()
}

pub fn read_signature_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_signature_type(_io: &mut IoHandler, _sig: u32) -> Result<(), CmsError> {
    todo!()
}

pub fn read_datetime_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_datetime_type(_io: &mut IoHandler, _dt: &DateTimeNumber) -> Result<(), CmsError> {
    todo!()
}

pub fn read_s15fixed16_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_s15fixed16_type(_io: &mut IoHandler, _arr: &[f64]) -> Result<(), CmsError> {
    todo!()
}

pub fn read_u16fixed16_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_u16fixed16_type(_io: &mut IoHandler, _arr: &[f64]) -> Result<(), CmsError> {
    todo!()
}

pub fn read_uint8_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_uint8_type(_io: &mut IoHandler, _arr: &[u8]) -> Result<(), CmsError> {
    todo!()
}

pub fn read_uint16_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_uint16_type(_io: &mut IoHandler, _arr: &[u16]) -> Result<(), CmsError> {
    todo!()
}

pub fn read_uint32_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_uint32_type(_io: &mut IoHandler, _arr: &[u32]) -> Result<(), CmsError> {
    todo!()
}

pub fn read_uint64_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_uint64_type(_io: &mut IoHandler, _arr: &[u64]) -> Result<(), CmsError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // XYZ type round-trip
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
}
