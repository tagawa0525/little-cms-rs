// Tag type handlers: read/write functions for each ICC tag type.
// C版: cmstypes.c

#![allow(dead_code)]

use crate::context::{CmsError, ErrorCode};
use crate::curves::gamma::ToneCurve;
use crate::pipeline::named::{Dict, Mlu, NamedColorList, ProfileSequenceDesc};
use crate::profile::io::IoHandler;
use crate::types::*;

// ============================================================================
// TagData enum
// ============================================================================

/// UCR/BG curves + description.
/// C版: `cmsUcrBg`
pub struct UcrBg {
    pub ucr: ToneCurve,
    pub bg: ToneCurve,
    pub desc: Mlu,
}

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
    UcrBg(Box<UcrBg>),
    VideoSignal(VideoSignalType),
    ProfileSequenceDesc(ProfileSequenceDesc),
    Vcgt(Box<[ToneCurve; 3]>),
    Dict(Dict),
    UInt8Array(Vec<u8>),
    UInt16Array(Vec<u16>),
    UInt32Array(Vec<u32>),
    UInt64Array(Vec<u64>),
    Raw(Vec<u8>),
}

// ============================================================================
// Tag type read/write dispatch
// ============================================================================

/// Read a tag's data given its type signature and payload size.
/// Dispatches to the appropriate type-specific handler.
pub fn read_tag_type(
    io: &mut IoHandler,
    sig: TagTypeSignature,
    size: u32,
) -> Result<TagData, CmsError> {
    match sig {
        TagTypeSignature::Xyz => read_xyz_type(io, size),
        TagTypeSignature::Signature => read_signature_type(io, size),
        TagTypeSignature::DateTime => read_datetime_type(io, size),
        TagTypeSignature::S15Fixed16Array => read_s15fixed16_type(io, size),
        TagTypeSignature::U16Fixed16Array => read_u16fixed16_type(io, size),
        TagTypeSignature::UInt8Array => read_uint8_type(io, size),
        TagTypeSignature::UInt16Array => read_uint16_type(io, size),
        TagTypeSignature::UInt32Array => read_uint32_type(io, size),
        TagTypeSignature::UInt64Array => read_uint64_type(io, size),
        TagTypeSignature::Text => read_text_type(io, size),
        TagTypeSignature::TextDescription => read_text_description_type(io, size),
        TagTypeSignature::MultiLocalizedUnicode => read_mlu_type(io, size),
        TagTypeSignature::Curve => read_curve_type(io, size),
        TagTypeSignature::ParametricCurve => read_parametric_curve_type(io, size),
        TagTypeSignature::Measurement => read_measurement_type(io, size),
        TagTypeSignature::ViewingConditions => read_viewing_conditions_type(io, size),
        TagTypeSignature::Chromaticity => read_chromaticity_type(io, size),
        TagTypeSignature::ColorantOrder => read_colorant_order_type(io, size),
        TagTypeSignature::ColorantTable => read_colorant_table_type(io, size),
        TagTypeSignature::NamedColor2 => read_named_color_type(io, size),
        TagTypeSignature::Data => read_data_type(io, size),
        TagTypeSignature::Screening => read_screening_type(io, size),
        TagTypeSignature::UcrBg => read_ucr_bg_type(io, size),
        TagTypeSignature::CrdInfo => read_crd_info_type(io, size),
        TagTypeSignature::Cicp => read_video_signal_type(io, size),
        TagTypeSignature::ProfileSequenceDesc => read_profile_sequence_desc_type(io, size),
        TagTypeSignature::ProfileSequenceId => read_profile_sequence_id_type(io, size),
        TagTypeSignature::Vcgt => read_vcgt_type(io, size),
        TagTypeSignature::Dict => read_dict_type(io, size),
        _ => {
            // Unknown type: read as raw bytes
            let mut raw = vec![0u8; size as usize];
            if size > 0 && !io.read(&mut raw) {
                return Err(IoHandler::read_err());
            }
            Ok(TagData::Raw(raw))
        }
    }
}

/// Write a tag's data. Returns the TagTypeSignature used.
pub fn write_tag_type(
    io: &mut IoHandler,
    data: &TagData,
    _icc_version: u32,
) -> Result<TagTypeSignature, CmsError> {
    let sig = match data {
        TagData::Xyz(xyz) => {
            write_xyz_type(io, xyz)?;
            TagTypeSignature::Xyz
        }
        TagData::Signature(s) => {
            write_signature_type(io, *s)?;
            TagTypeSignature::Signature
        }
        TagData::DateTime(dt) => {
            write_datetime_type(io, dt)?;
            TagTypeSignature::DateTime
        }
        TagData::S15Fixed16Array(arr) => {
            write_s15fixed16_type(io, arr)?;
            TagTypeSignature::S15Fixed16Array
        }
        TagData::U16Fixed16Array(arr) => {
            write_u16fixed16_type(io, arr)?;
            TagTypeSignature::U16Fixed16Array
        }
        TagData::UInt8Array(arr) => {
            write_uint8_type(io, arr)?;
            TagTypeSignature::UInt8Array
        }
        TagData::UInt16Array(arr) => {
            write_uint16_type(io, arr)?;
            TagTypeSignature::UInt16Array
        }
        TagData::UInt32Array(arr) => {
            write_uint32_type(io, arr)?;
            TagTypeSignature::UInt32Array
        }
        TagData::UInt64Array(arr) => {
            write_uint64_type(io, arr)?;
            TagTypeSignature::UInt64Array
        }
        TagData::Mlu(mlu) => {
            // Use MLU type (v4) by default; v2 text description handled elsewhere
            write_mlu_type(io, mlu)?;
            TagTypeSignature::MultiLocalizedUnicode
        }
        TagData::Curve(curve) => {
            write_curve_type(io, curve)?;
            TagTypeSignature::Curve
        }
        TagData::Measurement(mc) => {
            write_measurement_type(io, mc)?;
            TagTypeSignature::Measurement
        }
        TagData::ViewingConditions(vc) => {
            write_viewing_conditions_type(io, vc)?;
            TagTypeSignature::ViewingConditions
        }
        TagData::Chromaticity(chrm) => {
            write_chromaticity_type(io, chrm)?;
            TagTypeSignature::Chromaticity
        }
        TagData::ColorantOrder(order) => {
            write_colorant_order_type(io, order)?;
            TagTypeSignature::ColorantOrder
        }
        TagData::NamedColor(list) => {
            write_named_color_type(io, list)?;
            TagTypeSignature::NamedColor2
        }
        TagData::Data(d) => {
            write_data_type(io, d)?;
            TagTypeSignature::Data
        }
        TagData::Screening(s) => {
            write_screening_type(io, s)?;
            TagTypeSignature::Screening
        }
        TagData::UcrBg(u) => {
            write_ucr_bg_type(io, u)?;
            TagTypeSignature::UcrBg
        }
        TagData::VideoSignal(vs) => {
            write_video_signal_type(io, vs)?;
            TagTypeSignature::Cicp
        }
        TagData::ProfileSequenceDesc(seq) => {
            write_profile_sequence_desc_type(io, seq, _icc_version)?;
            TagTypeSignature::ProfileSequenceDesc
        }
        TagData::Vcgt(curves) => {
            write_vcgt_type(io, curves)?;
            TagTypeSignature::Vcgt
        }
        TagData::Dict(dict) => {
            write_dict_type(io, dict)?;
            TagTypeSignature::Dict
        }
        TagData::Raw(raw) => {
            if !io.write(raw) {
                return Err(IoHandler::write_err());
            }
            // Raw data doesn't have a type signature; caller must handle
            return Err(CmsError {
                code: ErrorCode::NotSuitable,
                message: "Cannot determine type signature for raw data".to_string(),
            });
        }
    };
    Ok(sig)
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
        if !(0.0..=65535.0).contains(&v) {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: format!("U16Fixed16 value out of range: {}", v),
            });
        }
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
// Reads ASCII text, stores as Mlu with "no language"/"no country".

pub fn read_text_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let mut buf = vec![0u8; size as usize];
    if size > 0 && !io.read(&mut buf) {
        return Err(IoHandler::read_err());
    }
    // Trim trailing NUL
    while buf.last() == Some(&0) {
        buf.pop();
    }
    let text = String::from_utf8_lossy(&buf);
    let mut mlu = Mlu::new();
    mlu.set_ascii("en", "US", &text);
    Ok(TagData::Mlu(mlu))
}

pub fn write_text_type(io: &mut IoHandler, mlu: &Mlu) -> Result<(), CmsError> {
    let text = mlu.get_ascii("en", "US").unwrap_or_default();
    let bytes = text.as_bytes();
    if !io.write(bytes) {
        return Err(IoHandler::write_err());
    }
    // Write NUL terminator
    io.write_u8(0)?;
    Ok(())
}

// --- TextDescription type (v2) ---
// C版: Type_Text_Description_Read / Type_Text_Description_Write
// Structure: u32 ascii_len + ascii_bytes + u32 unicode_lang + u32 unicode_count + utf16 + ...

pub fn read_text_description_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let ascii_len = io.read_u32()? as usize;
    let mut ascii_buf = vec![0u8; ascii_len];
    if ascii_len > 0 && !io.read(&mut ascii_buf) {
        return Err(IoHandler::read_err());
    }
    // Trim trailing NUL
    while ascii_buf.last() == Some(&0) {
        ascii_buf.pop();
    }
    let text = String::from_utf8_lossy(&ascii_buf);
    let mut mlu = Mlu::new();
    mlu.set_ascii("en", "US", &text);
    // Skip unicode and scriptcode sections (tolerate missing data)
    Ok(TagData::Mlu(mlu))
}

pub fn write_text_description_type(io: &mut IoHandler, mlu: &Mlu) -> Result<(), CmsError> {
    let text = mlu.get_ascii("en", "US").unwrap_or_default();
    let bytes = text.as_bytes();
    let len = bytes.len() + 1; // include NUL
    io.write_u32(len as u32)?;
    if !io.write(bytes) {
        return Err(IoHandler::write_err());
    }
    io.write_u8(0)?; // NUL terminator

    // Unicode section: language=0, count=0
    io.write_u32(0)?; // unicode language code
    io.write_u32(0)?; // unicode count

    // ScriptCode section: code=0, count=0, data=67 zero bytes
    io.write_u16(0)?; // script code
    io.write_u8(0)?; // count
    let zeros = [0u8; 67];
    if !io.write(&zeros) {
        return Err(IoHandler::write_err());
    }
    Ok(())
}

// --- MLU type (v4) ---
// C版: Type_MLU_Read / Type_MLU_Write
// Structure: u32 count + u32 rec_len(=12) + records[count] + UTF-16BE pool

pub fn read_mlu_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let count = io.read_u32()? as usize;
    let rec_len = io.read_u32()?;
    if rec_len != 12 {
        return Err(CmsError {
            code: ErrorCode::UnknownExtension,
            message: "MLU record length != 12".to_string(),
        });
    }

    // Header size: type_base(8) already consumed by caller in full pipeline,
    // but here we work with raw payload. The directory is 12*count bytes.
    let dir_size = 12 * count;

    struct MluRecord {
        lang: [u8; 2],
        country: [u8; 2],
        len: u32,
        offset: u32,
    }

    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let lang_raw = io.read_u16()?;
        let country_raw = io.read_u16()?;
        let len = io.read_u32()?;
        let offset = io.read_u32()?;
        records.push(MluRecord {
            lang: lang_raw.to_be_bytes(),
            country: country_raw.to_be_bytes(),
            len,
            offset,
        });
    }

    // Read the UTF-16BE string pool (rest of the tag)
    let pool_start = 8 + dir_size as u32; // 8 = count(4) + rec_len(4)
    let pool_size = size.saturating_sub(pool_start);
    let mut pool = vec![0u8; pool_size as usize];
    if pool_size > 0 && !io.read(&mut pool) {
        return Err(IoHandler::read_err());
    }

    let mut mlu = Mlu::new();
    for rec in &records {
        // Offset in the MLU is relative to start of tag (after type base).
        // String data starts at pool_start, so string offset into pool = rec.offset - pool_start
        let str_offset = rec.offset.saturating_sub(pool_start) as usize;
        let str_len = rec.len as usize;
        if str_offset + str_len > pool.len() {
            continue; // Skip bad records
        }
        let utf16_bytes = &pool[str_offset..str_offset + str_len];
        // Decode UTF-16BE to String
        let text = decode_utf16be(utf16_bytes);
        let lang = std::str::from_utf8(&rec.lang).unwrap_or("en");
        let country = std::str::from_utf8(&rec.country).unwrap_or("US");
        mlu.set_utf8(lang, country, &text);
    }

    Ok(TagData::Mlu(mlu))
}

pub fn write_mlu_type(io: &mut IoHandler, mlu: &Mlu) -> Result<(), CmsError> {
    let count = mlu.translations_count();
    io.write_u32(count as u32)?;
    io.write_u32(12)?; // record length

    // Build UTF-16BE pool and records
    let header_size = 8 + 12 * count; // 8 = count(4) + rec_len(4)
    let mut pool = Vec::new();
    struct WriteRecord {
        lang: u16,
        country: u16,
        offset: u32,
        len: u32,
    }
    let mut records = Vec::with_capacity(count);

    for i in 0..count {
        if let Some((lang_code, country_code)) = mlu.translation_codes(i) {
            let lang_str = std::str::from_utf8(&lang_code.0).unwrap_or("en");
            let country_str = std::str::from_utf8(&country_code.0).unwrap_or("US");
            let text = mlu.get_utf8(lang_str, country_str).unwrap_or_default();
            let utf16_bytes = encode_utf16be(&text);
            let offset = header_size + pool.len();
            let len = utf16_bytes.len();
            records.push(WriteRecord {
                lang: u16::from_be_bytes(lang_code.0),
                country: u16::from_be_bytes(country_code.0),
                offset: offset as u32,
                len: len as u32,
            });
            pool.extend_from_slice(&utf16_bytes);
        }
    }

    // Write directory
    for rec in &records {
        io.write_u16(rec.lang)?;
        io.write_u16(rec.country)?;
        io.write_u32(rec.len)?;
        io.write_u32(rec.offset)?;
    }

    // Write pool
    if !pool.is_empty() && !io.write(&pool) {
        return Err(IoHandler::write_err());
    }

    Ok(())
}

fn decode_utf16be(bytes: &[u8]) -> String {
    let u16s: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&u16s)
}

fn encode_utf16be(text: &str) -> Vec<u8> {
    let mut result = Vec::new();
    for c in text.encode_utf16() {
        result.extend_from_slice(&c.to_be_bytes());
    }
    result
}

// --- Curve type ---
// C版: Type_Curve_Read / Type_Curve_Write

pub fn read_curve_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let count = io.read_u32()?;
    let curve = match count {
        0 => {
            // Linear
            ToneCurve::build_parametric(1, &[1.0]).ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to build linear curve".to_string(),
            })?
        }
        1 => {
            // Single gamma as 8.8 fixed point
            let gamma_fixed = io.read_u16()?;
            let gamma = gamma_fixed as f64 / 256.0;
            ToneCurve::build_parametric(1, &[gamma]).ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to build gamma curve".to_string(),
            })?
        }
        _ => {
            if count > 0x7FFF {
                return Err(CmsError {
                    code: ErrorCode::Range,
                    message: "Curve table too large".to_string(),
                });
            }
            let table = io.read_u16_array(count as usize)?;
            ToneCurve::build_tabulated_16(&table).ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to build tabulated curve".to_string(),
            })?
        }
    };
    Ok(TagData::Curve(curve))
}

pub fn write_curve_type(io: &mut IoHandler, curve: &ToneCurve) -> Result<(), CmsError> {
    // Check if this is a single-segment parametric type 1 (pure gamma)
    if let Some(seg) = curve.segment(0)
        && !curve.is_multisegment()
        && seg.curve_type == 1
    {
        let gamma = seg.params[0];
        let gamma_fixed = ((gamma * 256.0) + 0.5) as u16;
        io.write_u32(1)?;
        io.write_u16(gamma_fixed)?;
        return Ok(());
    }

    // Tabulated curve
    let table = curve.table16();
    io.write_u32(table.len() as u32)?;
    io.write_u16_array(table)?;
    Ok(())
}

// --- ParametricCurve type ---
// C版: Type_ParametricCurve_Read / Type_ParametricCurve_Write

const PARAMS_BY_TYPE: [usize; 5] = [1, 3, 4, 5, 7];

pub fn read_parametric_curve_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let curve_type = io.read_u16()?;
    let _reserved = io.read_u16()?;

    if curve_type > 4 {
        return Err(CmsError {
            code: ErrorCode::UnknownExtension,
            message: format!("Unknown parametric curve type '{}'", curve_type),
        });
    }

    let n_params = PARAMS_BY_TYPE[curve_type as usize];
    let mut params = [0.0f64; 10];
    for p in params.iter_mut().take(n_params) {
        *p = io.read_s15fixed16()?;
    }

    // cmstypes.c uses Type+1 for cmsBuildParametricToneCurve
    let curve = ToneCurve::build_parametric(curve_type as i32 + 1, &params[..n_params])
        .ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to build parametric curve".to_string(),
        })?;
    Ok(TagData::Curve(curve))
}

pub fn write_parametric_curve_type(io: &mut IoHandler, curve: &ToneCurve) -> Result<(), CmsError> {
    let seg = curve.segment(0).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "No segment in parametric curve".to_string(),
    })?;

    let typen = seg.curve_type;
    if !(1..=5).contains(&typen) {
        return Err(CmsError {
            code: ErrorCode::UnknownExtension,
            message: "Unsupported parametric curve type for write".to_string(),
        });
    }

    let n_params = PARAMS_BY_TYPE[(typen - 1) as usize];
    io.write_u16((typen - 1) as u16)?;
    io.write_u16(0)?; // reserved

    for i in 0..n_params {
        io.write_s15fixed16(seg.params[i])?;
    }
    Ok(())
}

// --- Measurement type ---
// C版: Type_Measurement_Read / Type_Measurement_Write

pub fn read_measurement_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let mc = IccMeasurementConditions {
        observer: io.read_u32()?,
        backing: io.read_xyz()?,
        geometry: io.read_u32()?,
        flare: io.read_s15fixed16()?,
        illuminant_type: io.read_u32()?,
    };
    Ok(TagData::Measurement(mc))
}

pub fn write_measurement_type(
    io: &mut IoHandler,
    mc: &IccMeasurementConditions,
) -> Result<(), CmsError> {
    io.write_u32(mc.observer)?;
    io.write_xyz(&mc.backing)?;
    io.write_u32(mc.geometry)?;
    io.write_s15fixed16(mc.flare)?;
    io.write_u32(mc.illuminant_type)?;
    Ok(())
}

// --- ViewingConditions type ---
// C版: Type_ViewingConditions_Read / Type_ViewingConditions_Write

pub fn read_viewing_conditions_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let vc = IccViewingConditions {
        illuminant: io.read_xyz()?,
        surround: io.read_xyz()?,
        illuminant_type: io.read_u32()?,
    };
    Ok(TagData::ViewingConditions(vc))
}

pub fn write_viewing_conditions_type(
    io: &mut IoHandler,
    vc: &IccViewingConditions,
) -> Result<(), CmsError> {
    io.write_xyz(&vc.illuminant)?;
    io.write_xyz(&vc.surround)?;
    io.write_u32(vc.illuminant_type)?;
    Ok(())
}

// --- Chromaticity type ---
// C版: Type_Chromaticity_Read / Type_Chromaticity_Write

pub fn read_chromaticity_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let n_chans = io.read_u16()?;
    if n_chans != 3 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Chromaticity channels must be 3, got {}", n_chans),
        });
    }
    let _table = io.read_u16()?; // phosphor type, ignored
    let chrm = CieXyYTriple {
        red: CieXyY {
            x: io.read_s15fixed16()?,
            y: io.read_s15fixed16()?,
            big_y: 1.0,
        },
        green: CieXyY {
            x: io.read_s15fixed16()?,
            y: io.read_s15fixed16()?,
            big_y: 1.0,
        },
        blue: CieXyY {
            x: io.read_s15fixed16()?,
            y: io.read_s15fixed16()?,
            big_y: 1.0,
        },
    };
    Ok(TagData::Chromaticity(chrm))
}

pub fn write_chromaticity_type(io: &mut IoHandler, chrm: &CieXyYTriple) -> Result<(), CmsError> {
    io.write_u16(3)?; // channels
    io.write_u16(0)?; // phosphor type
    io.write_s15fixed16(chrm.red.x)?;
    io.write_s15fixed16(chrm.red.y)?;
    io.write_s15fixed16(chrm.green.x)?;
    io.write_s15fixed16(chrm.green.y)?;
    io.write_s15fixed16(chrm.blue.x)?;
    io.write_s15fixed16(chrm.blue.y)?;
    Ok(())
}

// --- Data type ---
// C版: Type_Data_Read / Type_Data_Write

pub fn read_data_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    if size < 4 {
        return Err(IoHandler::read_err());
    }
    let flags = io.read_u32()?;
    let data_len = (size - 4) as usize;
    let mut data = vec![0u8; data_len];
    if data_len > 0 && !io.read(&mut data) {
        return Err(IoHandler::read_err());
    }
    Ok(TagData::Data(IccData { flags, data }))
}

pub fn write_data_type(io: &mut IoHandler, d: &IccData) -> Result<(), CmsError> {
    io.write_u32(d.flags)?;
    if !d.data.is_empty() && !io.write(&d.data) {
        return Err(IoHandler::write_err());
    }
    Ok(())
}

// --- Screening type ---
// C版: Type_Screening_Read / Type_Screening_Write

pub fn read_screening_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let flags = io.read_u32()?;
    let n_channels = io.read_u32()?.min(MAX_CHANNELS as u32) as usize;
    let mut channels = Vec::with_capacity(n_channels);
    for _ in 0..n_channels {
        channels.push(ScreeningChannel {
            frequency: io.read_s15fixed16()?,
            screen_angle: io.read_s15fixed16()?,
            spot_shape: io.read_u32()?,
        });
    }
    Ok(TagData::Screening(Screening { flags, channels }))
}

pub fn write_screening_type(io: &mut IoHandler, s: &Screening) -> Result<(), CmsError> {
    if s.channels.len() > MAX_CHANNELS {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Too many screening channels: {}", s.channels.len()),
        });
    }
    io.write_u32(s.flags)?;
    io.write_u32(s.channels.len() as u32)?;
    for ch in &s.channels {
        io.write_s15fixed16(ch.frequency)?;
        io.write_s15fixed16(ch.screen_angle)?;
        io.write_u32(ch.spot_shape)?;
    }
    Ok(())
}

// --- ColorantOrder type ---
// C版: Type_ColorantOrderType_Read / Type_ColorantOrderType_Write

pub fn read_colorant_order_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let count = io.read_u32()?.min(MAX_CHANNELS as u32) as usize;
    let mut order = vec![0u8; count];
    if count > 0 && !io.read(&mut order) {
        return Err(IoHandler::read_err());
    }
    Ok(TagData::ColorantOrder(order))
}

pub fn write_colorant_order_type(io: &mut IoHandler, order: &[u8]) -> Result<(), CmsError> {
    if order.len() > MAX_CHANNELS {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Too many colorant order entries: {}", order.len()),
        });
    }
    io.write_u32(order.len() as u32)?;
    if !order.is_empty() && !io.write(order) {
        return Err(IoHandler::write_err());
    }
    Ok(())
}

// --- ColorantTable type ---
// C版: Type_ColorantTable_Read / Type_ColorantTable_Write
// Uses NamedColorList: each entry has a 32-byte name + PCS[3]

pub fn read_colorant_table_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let count = io.read_u32()? as usize;
    if count > MAX_CHANNELS {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Too many colorants: {}", count),
        });
    }
    let mut list = NamedColorList::new(0, "", "").ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create NamedColorList".to_string(),
    })?;

    for _ in 0..count {
        let mut name_buf = [0u8; 32];
        if !io.read(&mut name_buf) {
            return Err(IoHandler::read_err());
        }
        let name_end = name_buf.iter().position(|&b| b == 0).unwrap_or(32);
        let name = String::from_utf8_lossy(&name_buf[..name_end]);

        let pcs = [io.read_u16()?, io.read_u16()?, io.read_u16()?];
        list.append(&name, &pcs, None);
    }
    Ok(TagData::NamedColor(list))
}

pub fn write_colorant_table_type(
    io: &mut IoHandler,
    list: &NamedColorList,
) -> Result<(), CmsError> {
    let n = list.count();
    io.write_u32(n as u32)?;
    for i in 0..n {
        if let Some(color) = list.info(i) {
            let mut name_buf = [0u8; 32];
            let name_bytes = color.name.as_bytes();
            let copy_len = name_bytes.len().min(31);
            name_buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
            if !io.write(&name_buf) {
                return Err(IoHandler::write_err());
            }
            io.write_u16(color.pcs[0])?;
            io.write_u16(color.pcs[1])?;
            io.write_u16(color.pcs[2])?;
        }
    }
    Ok(())
}

// --- NamedColor2 type ---
// C版: Type_NamedColor_Read / Type_NamedColor_Write

pub fn read_named_color_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let _vendor_flag = io.read_u32()?;
    let count = io.read_u32()? as usize;
    let n_device_coords = io.read_u32()? as usize;

    if n_device_coords > MAX_CHANNELS {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Too many device coordinates: {}", n_device_coords),
        });
    }

    let mut prefix_buf = [0u8; 32];
    let mut suffix_buf = [0u8; 32];
    if !io.read(&mut prefix_buf) || !io.read(&mut suffix_buf) {
        return Err(IoHandler::read_err());
    }
    let prefix_end = prefix_buf.iter().position(|&b| b == 0).unwrap_or(32);
    let suffix_end = suffix_buf.iter().position(|&b| b == 0).unwrap_or(32);
    let prefix = String::from_utf8_lossy(&prefix_buf[..prefix_end]);
    let suffix = String::from_utf8_lossy(&suffix_buf[..suffix_end]);

    let mut list =
        NamedColorList::new(n_device_coords as u32, &prefix, &suffix).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create NamedColorList".to_string(),
        })?;

    for _ in 0..count {
        let mut name_buf = [0u8; 32];
        if !io.read(&mut name_buf) {
            return Err(IoHandler::read_err());
        }
        let name_end = name_buf.iter().position(|&b| b == 0).unwrap_or(32);
        let name = String::from_utf8_lossy(&name_buf[..name_end]);

        let pcs = [io.read_u16()?, io.read_u16()?, io.read_u16()?];

        let colorant: Vec<u16> = (0..n_device_coords)
            .map(|_| io.read_u16())
            .collect::<Result<_, _>>()?;

        list.append(&name, &pcs, Some(&colorant));
    }

    Ok(TagData::NamedColor(list))
}

pub fn write_named_color_type(io: &mut IoHandler, list: &NamedColorList) -> Result<(), CmsError> {
    io.write_u32(0)?; // vendor flag
    io.write_u32(list.count() as u32)?;
    io.write_u32(list.colorant_count())?;

    let mut prefix_buf = [0u8; 32];
    let prefix_bytes = list.prefix().as_bytes();
    let copy_len = prefix_bytes.len().min(31);
    prefix_buf[..copy_len].copy_from_slice(&prefix_bytes[..copy_len]);
    if !io.write(&prefix_buf) {
        return Err(IoHandler::write_err());
    }

    let mut suffix_buf = [0u8; 32];
    let suffix_bytes = list.suffix().as_bytes();
    let copy_len = suffix_bytes.len().min(31);
    suffix_buf[..copy_len].copy_from_slice(&suffix_bytes[..copy_len]);
    if !io.write(&suffix_buf) {
        return Err(IoHandler::write_err());
    }

    let n_device = list.colorant_count() as usize;
    for i in 0..list.count() {
        if let Some(color) = list.info(i) {
            let mut name_buf = [0u8; 32];
            let name_bytes = color.name.as_bytes();
            let copy_len = name_bytes.len().min(31);
            name_buf[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
            if !io.write(&name_buf) {
                return Err(IoHandler::write_err());
            }

            io.write_u16(color.pcs[0])?;
            io.write_u16(color.pcs[1])?;
            io.write_u16(color.pcs[2])?;

            for j in 0..n_device {
                io.write_u16(color.colorant[j])?;
            }
        }
    }
    Ok(())
}

// --- UcrBg type ---
// C版: Type_UcrBg_Read / Type_UcrBg_Write

pub fn read_ucr_bg_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let mut remaining = size as i32;

    // UCR curve
    if remaining < 4 {
        return Err(IoHandler::read_err());
    }
    let ucr_count = io.read_u32()? as usize;
    remaining -= 4;
    let ucr_bytes = ucr_count.checked_mul(2).and_then(|v| i32::try_from(v).ok());
    match ucr_bytes {
        Some(b) if remaining >= b => remaining -= b,
        _ => return Err(IoHandler::read_err()),
    }
    let ucr_table = io.read_u16_array(ucr_count)?;
    let ucr = ToneCurve::build_tabulated_16(&ucr_table).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to build UCR curve".to_string(),
    })?;

    // BG curve
    if remaining < 4 {
        return Err(IoHandler::read_err());
    }
    let bg_count = io.read_u32()? as usize;
    remaining -= 4;
    let bg_bytes = bg_count.checked_mul(2).and_then(|v| i32::try_from(v).ok());
    match bg_bytes {
        Some(b) if remaining >= b => remaining -= b,
        _ => return Err(IoHandler::read_err()),
    }
    let bg_table = io.read_u16_array(bg_count)?;
    let bg = ToneCurve::build_tabulated_16(&bg_table).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to build BG curve".to_string(),
    })?;

    // Description text
    let mut desc = Mlu::new();
    if remaining > 0 && remaining <= 32000 {
        let text_len = remaining as usize;
        let mut text_buf = vec![0u8; text_len];
        if !io.read(&mut text_buf) {
            return Err(IoHandler::read_err());
        }
        while text_buf.last() == Some(&0) {
            text_buf.pop();
        }
        let text = String::from_utf8_lossy(&text_buf);
        desc.set_ascii("en", "US", &text);
    }

    Ok(TagData::UcrBg(Box::new(UcrBg { ucr, bg, desc })))
}

pub fn write_ucr_bg_type(io: &mut IoHandler, u: &UcrBg) -> Result<(), CmsError> {
    // UCR curve
    let ucr_table = u.ucr.table16();
    io.write_u32(ucr_table.len() as u32)?;
    io.write_u16_array(ucr_table)?;

    // BG curve
    let bg_table = u.bg.table16();
    io.write_u32(bg_table.len() as u32)?;
    io.write_u16_array(bg_table)?;

    // Description text
    let text = u.desc.get_ascii("en", "US").unwrap_or_default();
    let bytes = text.as_bytes();
    if !io.write(bytes) {
        return Err(IoHandler::write_err());
    }
    io.write_u8(0)?; // NUL terminator
    Ok(())
}

// --- CrdInfo type ---
// C版: Type_CrdInfo_Read / Type_CrdInfo_Write
// Stores PostScript product name + CRD names as MLU with "PS" language.
// Note: CrdInfo is a v2 legacy format. read_crd_info_type returns TagData::Mlu,
// which will be written as MLU (v4) format through write_tag_type dispatch.
// This is intentional: CrdInfo → MLU upgrade on save.

fn read_count_and_string(
    io: &mut IoHandler,
    mlu: &mut Mlu,
    remaining: &mut u32,
    section: &str,
) -> Result<(), CmsError> {
    if *remaining < 4 {
        return Err(IoHandler::read_err());
    }
    let count = io.read_u32()? as usize;
    *remaining = remaining.saturating_sub(4);
    if (*remaining as usize) < count {
        return Err(IoHandler::read_err());
    }
    let mut buf = vec![0u8; count];
    if count > 0 && !io.read(&mut buf) {
        return Err(IoHandler::read_err());
    }
    *remaining = remaining.saturating_sub(count as u32);
    while buf.last() == Some(&0) {
        buf.pop();
    }
    let text = String::from_utf8_lossy(&buf);
    mlu.set_ascii("PS", section, &text);
    Ok(())
}

fn write_count_and_string(io: &mut IoHandler, mlu: &Mlu, section: &str) -> Result<(), CmsError> {
    let text = mlu.get_ascii("PS", section).unwrap_or_default();
    let bytes = text.as_bytes();
    let len = bytes.len() + 1; // include NUL
    io.write_u32(len as u32)?;
    if !io.write(bytes) {
        return Err(IoHandler::write_err());
    }
    io.write_u8(0)?;
    Ok(())
}

pub fn read_crd_info_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let mut mlu = Mlu::new();
    let mut remaining = size;
    read_count_and_string(io, &mut mlu, &mut remaining, "nm")?;
    read_count_and_string(io, &mut mlu, &mut remaining, "#0")?;
    read_count_and_string(io, &mut mlu, &mut remaining, "#1")?;
    read_count_and_string(io, &mut mlu, &mut remaining, "#2")?;
    read_count_and_string(io, &mut mlu, &mut remaining, "#3")?;
    Ok(TagData::Mlu(mlu))
}

pub fn write_crd_info_type(io: &mut IoHandler, mlu: &Mlu) -> Result<(), CmsError> {
    write_count_and_string(io, mlu, "nm")?;
    write_count_and_string(io, mlu, "#0")?;
    write_count_and_string(io, mlu, "#1")?;
    write_count_and_string(io, mlu, "#2")?;
    write_count_and_string(io, mlu, "#3")?;
    Ok(())
}

// --- VideoSignal (cicp) type ---
// C版: Type_VideoSignal_Read / Type_VideoSignal_Write

pub fn read_video_signal_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    if size != 4 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("VideoSignal tag must be 4 bytes, got {}", size),
        });
    }
    let vs = VideoSignalType {
        colour_primaries: io.read_u8()?,
        transfer_characteristics: io.read_u8()?,
        matrix_coefficients: io.read_u8()?,
        video_full_range_flag: io.read_u8()?,
    };
    Ok(TagData::VideoSignal(vs))
}

pub fn write_video_signal_type(io: &mut IoHandler, vs: &VideoSignalType) -> Result<(), CmsError> {
    io.write_u8(vs.colour_primaries)?;
    io.write_u8(vs.transfer_characteristics)?;
    io.write_u8(vs.matrix_coefficients)?;
    io.write_u8(vs.video_full_range_flag)?;
    Ok(())
}

// --- ProfileSequenceDesc type ---
// C版: Type_ProfileSequenceDesc_Read / Type_ProfileSequenceDesc_Write

pub fn read_profile_sequence_desc_type(
    _io: &mut IoHandler,
    _size: u32,
) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_profile_sequence_desc_type(
    _io: &mut IoHandler,
    _seq: &ProfileSequenceDesc,
    _icc_version: u32,
) -> Result<(), CmsError> {
    todo!()
}

// --- ProfileSequenceId type ---
// C版: Type_ProfileSequenceId_Read / Type_ProfileSequenceId_Write

pub fn read_profile_sequence_id_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_profile_sequence_id_type(
    _io: &mut IoHandler,
    _seq: &ProfileSequenceDesc,
) -> Result<(), CmsError> {
    todo!()
}

// --- vcgt type ---
// C版: Type_vcgt_Read / Type_vcgt_Write

pub fn read_vcgt_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_vcgt_type(_io: &mut IoHandler, _curves: &[ToneCurve; 3]) -> Result<(), CmsError> {
    todo!()
}

// --- Dict type ---
// C版: Type_Dictionary_Read / Type_Dictionary_Write

pub fn read_dict_type(_io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    todo!()
}

pub fn write_dict_type(_io: &mut IoHandler, _dict: &Dict) -> Result<(), CmsError> {
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

    // ========================================================================
    // Measurement type round-trip
    // ========================================================================

    #[test]
    fn measurement_type_roundtrip() {
        let mc = IccMeasurementConditions {
            observer: 1,
            backing: CieXyz {
                x: 0.5,
                y: 0.5,
                z: 0.5,
            },
            geometry: 2,
            flare: 0.1,
            illuminant_type: 3,
        };
        let mut io = IoHandler::from_memory_write(256);
        write_measurement_type(&mut io, &mc).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_measurement_type(&mut io, size).unwrap();
        match result {
            TagData::Measurement(read_mc) => {
                assert_eq!(read_mc.observer, 1);
                assert_eq!(read_mc.geometry, 2);
                assert!((read_mc.flare - 0.1).abs() < 1.0 / 65536.0);
                assert_eq!(read_mc.illuminant_type, 3);
            }
            _ => panic!("Expected TagData::Measurement"),
        }
    }

    // ========================================================================
    // ViewingConditions type round-trip
    // ========================================================================

    #[test]
    fn viewing_conditions_type_roundtrip() {
        let vc = IccViewingConditions {
            illuminant: CieXyz {
                x: D50_X,
                y: D50_Y,
                z: D50_Z,
            },
            surround: CieXyz {
                x: 0.2,
                y: 0.2,
                z: 0.2,
            },
            illuminant_type: 1,
        };
        let mut io = IoHandler::from_memory_write(256);
        write_viewing_conditions_type(&mut io, &vc).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_viewing_conditions_type(&mut io, size).unwrap();
        match result {
            TagData::ViewingConditions(read_vc) => {
                assert!((read_vc.illuminant.x - D50_X).abs() < 1.0 / 65536.0);
                assert_eq!(read_vc.illuminant_type, 1);
            }
            _ => panic!("Expected TagData::ViewingConditions"),
        }
    }

    // ========================================================================
    // Chromaticity type round-trip
    // ========================================================================

    #[test]
    fn chromaticity_type_roundtrip() {
        let chrm = CieXyYTriple {
            red: CieXyY {
                x: 0.64,
                y: 0.33,
                big_y: 1.0,
            },
            green: CieXyY {
                x: 0.30,
                y: 0.60,
                big_y: 1.0,
            },
            blue: CieXyY {
                x: 0.15,
                y: 0.06,
                big_y: 1.0,
            },
        };
        let mut io = IoHandler::from_memory_write(256);
        write_chromaticity_type(&mut io, &chrm).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_chromaticity_type(&mut io, size).unwrap();
        match result {
            TagData::Chromaticity(read_chrm) => {
                assert!((read_chrm.red.x - 0.64).abs() < 1.0 / 65536.0);
                assert!((read_chrm.green.y - 0.60).abs() < 1.0 / 65536.0);
                assert!((read_chrm.blue.x - 0.15).abs() < 1.0 / 65536.0);
            }
            _ => panic!("Expected TagData::Chromaticity"),
        }
    }

    // ========================================================================
    // Data type round-trip
    // ========================================================================

    #[test]
    fn data_type_roundtrip() {
        let d = IccData {
            flags: 0,
            data: vec![0xCA, 0xFE, 0xBA, 0xBE],
        };
        let mut io = IoHandler::from_memory_write(256);
        write_data_type(&mut io, &d).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_data_type(&mut io, size).unwrap();
        match result {
            TagData::Data(read_d) => {
                assert_eq!(read_d.flags, 0);
                assert_eq!(read_d.data, vec![0xCA, 0xFE, 0xBA, 0xBE]);
            }
            _ => panic!("Expected TagData::Data"),
        }
    }

    // ========================================================================
    // Screening type round-trip
    // ========================================================================

    #[test]
    fn screening_type_roundtrip() {
        let s = Screening {
            flags: 1,
            channels: vec![
                ScreeningChannel {
                    frequency: 133.0,
                    screen_angle: 45.0,
                    spot_shape: 1,
                },
                ScreeningChannel {
                    frequency: 100.0,
                    screen_angle: 75.0,
                    spot_shape: 2,
                },
            ],
        };
        let mut io = IoHandler::from_memory_write(256);
        write_screening_type(&mut io, &s).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_screening_type(&mut io, size).unwrap();
        match result {
            TagData::Screening(read_s) => {
                assert_eq!(read_s.flags, 1);
                assert_eq!(read_s.channels.len(), 2);
                assert!((read_s.channels[0].frequency - 133.0).abs() < 1.0 / 65536.0);
                assert_eq!(read_s.channels[1].spot_shape, 2);
            }
            _ => panic!("Expected TagData::Screening"),
        }
    }

    // ========================================================================
    // ColorantOrder type round-trip
    // ========================================================================

    #[test]
    fn colorant_order_type_roundtrip() {
        let order = vec![2, 0, 1, 3];
        let mut io = IoHandler::from_memory_write(256);
        write_colorant_order_type(&mut io, &order).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_colorant_order_type(&mut io, size).unwrap();
        match result {
            TagData::ColorantOrder(read_order) => assert_eq!(read_order, order),
            _ => panic!("Expected TagData::ColorantOrder"),
        }
    }

    // ========================================================================
    // ColorantTable type round-trip
    // ========================================================================

    #[test]
    fn colorant_table_type_roundtrip() {
        let mut list = NamedColorList::new(0, "", "").unwrap();
        list.append("Red", &[100, 200, 300], None);
        list.append("Green", &[400, 500, 600], None);

        let mut io = IoHandler::from_memory_write(512);
        write_colorant_table_type(&mut io, &list).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_colorant_table_type(&mut io, size).unwrap();
        match result {
            TagData::NamedColor(read_list) => {
                assert_eq!(read_list.count(), 2);
                assert_eq!(read_list.info(0).unwrap().name, "Red");
                assert_eq!(read_list.info(0).unwrap().pcs, [100, 200, 300]);
                assert_eq!(read_list.info(1).unwrap().name, "Green");
            }
            _ => panic!("Expected TagData::NamedColor"),
        }
    }

    // ========================================================================
    // NamedColor2 type round-trip
    // ========================================================================

    #[test]
    fn named_color2_type_roundtrip() {
        let mut list = NamedColorList::new(3, "pre_", "_suf").unwrap();
        list.append("Crimson", &[1000, 2000, 3000], Some(&[4000, 5000, 6000]));

        let mut io = IoHandler::from_memory_write(512);
        write_named_color_type(&mut io, &list).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_named_color_type(&mut io, size).unwrap();
        match result {
            TagData::NamedColor(read_list) => {
                assert_eq!(read_list.count(), 1);
                assert_eq!(read_list.prefix(), "pre_");
                assert_eq!(read_list.suffix(), "_suf");
                let color = read_list.info(0).unwrap();
                assert_eq!(color.name, "Crimson");
                assert_eq!(color.pcs, [1000, 2000, 3000]);
                assert_eq!(color.colorant[0], 4000);
                assert_eq!(color.colorant[1], 5000);
                assert_eq!(color.colorant[2], 6000);
            }
            _ => panic!("Expected TagData::NamedColor"),
        }
    }

    // ========================================================================
    // UcrBg type round-trip
    // ========================================================================

    #[test]
    fn ucr_bg_type_roundtrip() {
        let ucr = ToneCurve::build_tabulated_16(&[0, 32768, 65535]).unwrap();
        let bg = ToneCurve::build_tabulated_16(&[0, 16384, 65535]).unwrap();
        let mut desc = Mlu::new();
        desc.set_ascii("en", "US", "UCR/BG test");
        let data = UcrBg { ucr, bg, desc };

        let mut io = IoHandler::from_memory_write(512);
        write_ucr_bg_type(&mut io, &data).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_ucr_bg_type(&mut io, size).unwrap();
        match result {
            TagData::UcrBg(read_data) => {
                assert_eq!(read_data.ucr.table16_len(), 3);
                assert_eq!(read_data.bg.table16_len(), 3);
                assert_eq!(
                    read_data.desc.get_ascii("en", "US"),
                    Some("UCR/BG test".to_string())
                );
            }
            _ => panic!("Expected TagData::UcrBg"),
        }
    }

    // ========================================================================
    // CrdInfo type round-trip
    // ========================================================================

    #[test]
    fn crd_info_type_roundtrip() {
        let mut mlu = Mlu::new();
        mlu.set_ascii("PS", "nm", "Product Name");
        mlu.set_ascii("PS", "#0", "CRD Perceptual");
        mlu.set_ascii("PS", "#1", "CRD Colorimetric");
        mlu.set_ascii("PS", "#2", "CRD Saturation");
        mlu.set_ascii("PS", "#3", "CRD Absolute");

        let mut io = IoHandler::from_memory_write(512);
        write_crd_info_type(&mut io, &mlu).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_crd_info_type(&mut io, size).unwrap();
        match result {
            TagData::Mlu(read_mlu) => {
                assert_eq!(
                    read_mlu.get_ascii("PS", "nm"),
                    Some("Product Name".to_string())
                );
                assert_eq!(
                    read_mlu.get_ascii("PS", "#0"),
                    Some("CRD Perceptual".to_string())
                );
                assert_eq!(
                    read_mlu.get_ascii("PS", "#3"),
                    Some("CRD Absolute".to_string())
                );
            }
            _ => panic!("Expected TagData::Mlu"),
        }
    }

    // ========================================================================
    // VideoSignal (cicp) type round-trip
    // ========================================================================

    #[test]
    fn video_signal_type_roundtrip() {
        let vs = VideoSignalType {
            colour_primaries: 9,          // BT.2020
            transfer_characteristics: 16, // PQ
            matrix_coefficients: 0,
            video_full_range_flag: 1,
        };
        let mut io = IoHandler::from_memory_write(64);
        write_video_signal_type(&mut io, &vs).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_video_signal_type(&mut io, size).unwrap();
        match result {
            TagData::VideoSignal(read_vs) => {
                assert_eq!(read_vs.colour_primaries, 9);
                assert_eq!(read_vs.transfer_characteristics, 16);
                assert_eq!(read_vs.matrix_coefficients, 0);
                assert_eq!(read_vs.video_full_range_flag, 1);
            }
            _ => panic!("Expected TagData::VideoSignal"),
        }
    }

    // ========================================================================
    // ProfileSequenceDesc type round-trip
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn profile_sequence_desc_type_roundtrip() {
        let mut seq = ProfileSequenceDesc::new(2);

        let entry0 = seq.get_mut(0).unwrap();
        entry0.device_mfg = 0x4150504C; // 'APPL'
        entry0.device_model = 0x4D4F4E49;
        entry0.attributes = 0x0000_0001_0000_0000;
        entry0.technology = Some(TechnologySignature::FilmScanner);
        entry0.manufacturer.set_ascii("en", "US", "Apple Inc.");
        entry0.model.set_ascii("en", "US", "Cinema Display");

        let entry1 = seq.get_mut(1).unwrap();
        entry1.device_mfg = 0x41444245; // 'ADBE'
        entry1.manufacturer.set_ascii("en", "US", "Adobe");

        let mut io = IoHandler::from_memory_write(2048);
        write_profile_sequence_desc_type(&mut io, &seq, 0x0400_0000).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_profile_sequence_desc_type(&mut io, size).unwrap();
        match result {
            TagData::ProfileSequenceDesc(read_seq) => {
                assert_eq!(read_seq.len(), 2);
                let e0 = read_seq.get(0).unwrap();
                assert_eq!(e0.device_mfg, 0x4150504C);
                assert_eq!(e0.device_model, 0x4D4F4E49);
                assert_eq!(e0.attributes, 0x0000_0001_0000_0000);
                assert_eq!(e0.technology, Some(TechnologySignature::FilmScanner));
                assert_eq!(
                    e0.manufacturer.get_ascii("en", "US"),
                    Some("Apple Inc.".to_string())
                );
                assert_eq!(
                    e0.model.get_ascii("en", "US"),
                    Some("Cinema Display".to_string())
                );
                let e1 = read_seq.get(1).unwrap();
                assert_eq!(e1.device_mfg, 0x41444245);
                assert_eq!(
                    e1.manufacturer.get_ascii("en", "US"),
                    Some("Adobe".to_string())
                );
            }
            _ => panic!("Expected TagData::ProfileSequenceDesc"),
        }
    }

    // ========================================================================
    // ProfileSequenceId type round-trip
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn profile_sequence_id_type_roundtrip() {
        let mut seq = ProfileSequenceDesc::new(2);

        let entry0 = seq.get_mut(0).unwrap();
        entry0.profile_id = ProfileId([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        entry0.description.set_utf8("en", "US", "sRGB profile");

        let entry1 = seq.get_mut(1).unwrap();
        entry1.profile_id = ProfileId([0xA0; 16]);
        entry1.description.set_utf8("ja", "JP", "日本語テスト");

        let mut io = IoHandler::from_memory_write(4096);
        write_profile_sequence_id_type(&mut io, &seq).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_profile_sequence_id_type(&mut io, size).unwrap();
        match result {
            TagData::ProfileSequenceDesc(read_seq) => {
                assert_eq!(read_seq.len(), 2);
                let e0 = read_seq.get(0).unwrap();
                assert_eq!(e0.profile_id.0[0], 1);
                assert_eq!(e0.profile_id.0[15], 16);
                assert_eq!(
                    e0.description.get_utf8("en", "US"),
                    Some("sRGB profile".to_string())
                );
                let e1 = read_seq.get(1).unwrap();
                assert_eq!(e1.profile_id, ProfileId([0xA0; 16]));
                assert_eq!(
                    e1.description.get_utf8("ja", "JP"),
                    Some("日本語テスト".to_string())
                );
            }
            _ => panic!("Expected TagData::ProfileSequenceDesc"),
        }
    }

    // ========================================================================
    // vcgt type round-trip (table)
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn vcgt_table_type_roundtrip() {
        let r = ToneCurve::build_tabulated_16(&[0, 32768, 65535]).unwrap();
        let g = ToneCurve::build_tabulated_16(&[0, 16384, 65535]).unwrap();
        let b = ToneCurve::build_tabulated_16(&[0, 49152, 65535]).unwrap();

        let mut io = IoHandler::from_memory_write(2048);
        write_vcgt_type(&mut io, &[r.clone(), g.clone(), b.clone()]).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_vcgt_type(&mut io, size).unwrap();
        match result {
            TagData::Vcgt(curves) => {
                // Table type outputs 256 entries per channel, so verify endpoints
                assert_eq!(curves[0].eval_u16(0), 0);
                assert_eq!(curves[0].eval_u16(65535), 65535);
                assert_eq!(curves[1].eval_u16(0), 0);
                assert_eq!(curves[2].eval_u16(0), 0);
                assert_eq!(curves[2].eval_u16(65535), 65535);
            }
            _ => panic!("Expected TagData::Vcgt"),
        }
    }

    // ========================================================================
    // vcgt type round-trip (parametric/formula)
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn vcgt_formula_type_roundtrip() {
        // Build parametric type 5 curves that vcgt formula can encode
        // Y = (aX + b)^Gamma + e  where a=(Max-Min)^(1/Gamma), b=0, e=Min
        let gamma: f64 = 2.2;
        let min: f64 = 0.01;
        let max: f64 = 0.99;
        let a = (max - min).powf(1.0 / gamma);
        let params = [gamma, a, 0.0, 0.0, 0.0, min, 0.0];
        let curve = ToneCurve::build_parametric(5, &params).unwrap();

        let mut io = IoHandler::from_memory_write(2048);
        write_vcgt_type(&mut io, &[curve.clone(), curve.clone(), curve.clone()]).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_vcgt_type(&mut io, size).unwrap();
        match result {
            TagData::Vcgt(curves) => {
                // Check mid-point evaluation is reasonable
                let mid = curves[0].eval_f32(0.5);
                assert!(mid > 0.0 && mid < 1.0);
            }
            _ => panic!("Expected TagData::Vcgt"),
        }
    }

    // ========================================================================
    // Dict type round-trip
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn dict_type_roundtrip_basic() {
        let mut dict = Dict::new();
        dict.add("key1", Some("value1"), None, None);
        dict.add("key2", Some("value2"), None, None);

        let mut io = IoHandler::from_memory_write(4096);
        write_dict_type(&mut io, &dict).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_dict_type(&mut io, size).unwrap();
        match result {
            TagData::Dict(read_dict) => {
                assert_eq!(read_dict.len(), 2);
                let entries: Vec<_> = read_dict.iter().collect();
                assert_eq!(entries[0].name, "key1");
                assert_eq!(entries[0].value.as_deref(), Some("value1"));
                assert_eq!(entries[1].name, "key2");
                assert_eq!(entries[1].value.as_deref(), Some("value2"));
            }
            _ => panic!("Expected TagData::Dict"),
        }
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn dict_type_roundtrip_with_display() {
        let mut dict = Dict::new();
        let mut dn = Mlu::new();
        dn.set_utf8("en", "US", "Key Display");
        let mut dv = Mlu::new();
        dv.set_utf8("ja", "JP", "値の表示");
        dict.add("mykey", Some("myval"), Some(&dn), Some(&dv));

        let mut io = IoHandler::from_memory_write(4096);
        write_dict_type(&mut io, &dict).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_dict_type(&mut io, size).unwrap();
        match result {
            TagData::Dict(read_dict) => {
                assert_eq!(read_dict.len(), 1);
                let entry = read_dict.iter().next().unwrap();
                assert_eq!(entry.name, "mykey");
                assert_eq!(entry.value.as_deref(), Some("myval"));
                assert_eq!(
                    entry.display_name.as_ref().unwrap().get_utf8("en", "US"),
                    Some("Key Display".to_string())
                );
                assert_eq!(
                    entry.display_value.as_ref().unwrap().get_utf8("ja", "JP"),
                    Some("値の表示".to_string())
                );
            }
            _ => panic!("Expected TagData::Dict"),
        }
    }
}
