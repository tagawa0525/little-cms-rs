// Tag type handlers: read/write functions for each ICC tag type.
// C版: cmstypes.c

#![allow(dead_code)]

use crate::context::{CmsError, ErrorCode};
use crate::curves::gamma::ToneCurve;
use crate::pipeline::lut::{CLutTable, Pipeline, Stage, StageData, StageLoc};
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
    Pipeline(Pipeline),
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
        TagTypeSignature::Lut8 => read_lut8_type(io, size),
        TagTypeSignature::Lut16 => read_lut16_type(io, size),
        TagTypeSignature::LutAtoB => read_lut_atob_type(io, size),
        TagTypeSignature::LutBtoA => read_lut_btoa_type(io, size),
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
    icc_version: u32,
    tag_sig: TagSignature,
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
            write_profile_sequence_desc_type(io, seq, icc_version)?;
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
        TagData::Pipeline(pipe) => {
            // V4 AToB/BToA tags use LutAtoB/LutBtoA format.
            // V2 and other tags use Lut8/Lut16.
            let is_v4 = icc_version >= 0x04000000;
            let is_atob = matches!(
                tag_sig,
                TagSignature::AToB0 | TagSignature::AToB1 | TagSignature::AToB2
            );
            let is_btoa = matches!(
                tag_sig,
                TagSignature::BToA0 | TagSignature::BToA1 | TagSignature::BToA2
            );

            if is_v4 && is_atob {
                write_lut_atob_type(io, pipe)?;
                TagTypeSignature::LutAtoB
            } else if is_v4 && is_btoa {
                write_lut_btoa_type(io, pipe)?;
                TagTypeSignature::LutBtoA
            } else if pipe.save_as_8bits() {
                write_lut8_type(io, pipe)?;
                TagTypeSignature::Lut8
            } else {
                write_lut16_type(io, pipe)?;
                TagTypeSignature::Lut16
            }
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

// --- Embedded text helper ---
// C版: ReadEmbeddedText — reads Text, TextDescription, or MLU based on embedded type marker.

fn read_embedded_text(io: &mut IoHandler, size: u32) -> Result<Mlu, CmsError> {
    let start = io.tell();
    let end = start.saturating_add(size);

    if size < 8 {
        // Always advance past the full embedded payload
        io.seek(end);
        return Ok(Mlu::new());
    }
    let type_sig = io.read_u32()?;
    let _reserved = io.read_u32()?;
    let payload_size = size.saturating_sub(8);

    let result = match TagTypeSignature::try_from(type_sig) {
        Ok(TagTypeSignature::Text) => match read_text_type(io, payload_size)? {
            TagData::Mlu(mlu) => mlu,
            _ => Mlu::new(),
        },
        Ok(TagTypeSignature::TextDescription) => {
            match read_text_description_type(io, payload_size)? {
                TagData::Mlu(mlu) => mlu,
                _ => Mlu::new(),
            }
        }
        Ok(TagTypeSignature::MultiLocalizedUnicode) => match read_mlu_type(io, payload_size)? {
            TagData::Mlu(mlu) => mlu,
            _ => Mlu::new(),
        },
        _ => Mlu::new(),
    };

    // Always advance to the end of the embedded payload to keep cursor in sync
    io.seek(end);
    Ok(result)
}

/// Write embedded text as MLU (v4) with type base header.
fn write_embedded_mlu(io: &mut IoHandler, mlu: &Mlu) -> Result<(), CmsError> {
    io.write_u32(TagTypeSignature::MultiLocalizedUnicode as u32)?;
    io.write_u32(0)?; // reserved
    write_mlu_type(io, mlu)
}

// --- ProfileSequenceDesc type ---
// C版: Type_ProfileSequenceDesc_Read / Type_ProfileSequenceDesc_Write

pub fn read_profile_sequence_desc_type(
    io: &mut IoHandler,
    _size: u32,
) -> Result<TagData, CmsError> {
    let count = io.read_u32()? as usize;
    if count > 256 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "ProfileSequenceDesc count too large".to_string(),
        });
    }
    let mut seq = ProfileSequenceDesc::new(count);

    for i in 0..count {
        let entry = seq.get_mut(i).unwrap();
        entry.device_mfg = io.read_u32()?;
        entry.device_model = io.read_u32()?;
        entry.attributes = io.read_u64()?;
        let tech_raw = io.read_u32()?;
        entry.technology = TechnologySignature::try_from(tech_raw).ok();

        // Read manufacturer description (embedded text)
        let mfg_size = io.read_u32()?;
        if mfg_size > 0 {
            entry.manufacturer = read_embedded_text(io, mfg_size)?;
        }

        // Read model description (embedded text)
        let model_size = io.read_u32()?;
        if model_size > 0 {
            entry.model = read_embedded_text(io, model_size)?;
        }
    }

    Ok(TagData::ProfileSequenceDesc(seq))
}

pub fn write_profile_sequence_desc_type(
    io: &mut IoHandler,
    seq: &ProfileSequenceDesc,
    _icc_version: u32,
) -> Result<(), CmsError> {
    io.write_u32(seq.len() as u32)?;

    for i in 0..seq.len() {
        let entry = seq.get(i).unwrap();
        io.write_u32(entry.device_mfg)?;
        io.write_u32(entry.device_model)?;
        io.write_u64(entry.attributes)?;
        io.write_u32(entry.technology.map_or(0, |t| t as u32))?;

        // Write manufacturer: first calculate size, then write
        let mfg_size = {
            let mut null_io = IoHandler::new_null();
            write_embedded_mlu(&mut null_io, &entry.manufacturer)?;
            null_io.used_space()
        };
        io.write_u32(mfg_size)?;
        write_embedded_mlu(io, &entry.manufacturer)?;

        // Write model
        let model_size = {
            let mut null_io = IoHandler::new_null();
            write_embedded_mlu(&mut null_io, &entry.model)?;
            null_io.used_space()
        };
        io.write_u32(model_size)?;
        write_embedded_mlu(io, &entry.model)?;
    }

    Ok(())
}

// --- ProfileSequenceId type ---
// C版: Type_ProfileSequenceId_Read / Type_ProfileSequenceId_Write
// Uses position table for element access.

pub fn read_profile_sequence_id_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let base_offset = io.tell().saturating_sub(8); // before type base
    let count = io.read_u32()? as usize;
    if count > 256 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "ProfileSequenceId count too large".to_string(),
        });
    }

    // Read position table (offset/size pairs)
    let mut offsets = Vec::with_capacity(count);
    let mut sizes = Vec::with_capacity(count);
    for _ in 0..count {
        let offset = io.read_u32()?;
        let size = io.read_u32()?;
        offsets.push(if offset > 0 { offset + base_offset } else { 0 });
        sizes.push(size);
    }

    let mut seq = ProfileSequenceDesc::new(count);

    for i in 0..count {
        if offsets[i] == 0 || sizes[i] == 0 {
            continue;
        }
        io.seek(offsets[i]);
        let entry = seq.get_mut(i).unwrap();

        // Read 16-byte ProfileID
        let mut id_bytes = [0u8; 16];
        if !io.read(&mut id_bytes) {
            return Err(IoHandler::read_err());
        }
        entry.profile_id = ProfileId(id_bytes);

        // Read description (embedded text) — remaining bytes
        let desc_size = sizes[i].saturating_sub(16);
        if desc_size > 0 {
            entry.description = read_embedded_text(io, desc_size)?;
        }
    }

    Ok(TagData::ProfileSequenceDesc(seq))
}

pub fn write_profile_sequence_id_type(
    io: &mut IoHandler,
    seq: &ProfileSequenceDesc,
) -> Result<(), CmsError> {
    let base_offset = io.tell().saturating_sub(8);
    let count = seq.len();
    io.write_u32(count as u32)?;

    // Save directory position, write placeholder
    let dir_pos = io.tell();
    for _ in 0..count {
        io.write_u32(0)?; // offset placeholder
        io.write_u32(0)?; // size placeholder
    }

    // Write each element, recording offset/size
    let mut offsets = Vec::with_capacity(count);
    let mut elem_sizes = Vec::with_capacity(count);
    for i in 0..count {
        let entry = seq.get(i).unwrap();
        let before = io.tell();
        offsets.push(before - base_offset);

        // Write 16-byte ProfileID
        if !io.write(&entry.profile_id.0) {
            return Err(IoHandler::write_err());
        }

        // Write description as embedded MLU
        write_embedded_mlu(io, &entry.description)?;

        elem_sizes.push(io.tell() - before);
    }

    // Go back and fill in the directory
    let end_pos = io.tell();
    io.seek(dir_pos);
    for i in 0..count {
        io.write_u32(offsets[i])?;
        io.write_u32(elem_sizes[i])?;
    }
    io.seek(end_pos);

    Ok(())
}

// --- vcgt type ---
// C版: Type_vcgt_Read / Type_vcgt_Write
// Video card gamma: table type (0) or formula type (1).

const VCGT_TABLE_TYPE: u32 = 0;
const VCGT_FORMULA_TYPE: u32 = 1;

pub fn read_vcgt_type(io: &mut IoHandler, size: u32) -> Result<TagData, CmsError> {
    let tag_type = io.read_u32()?;

    match tag_type {
        VCGT_TABLE_TYPE => {
            let n_channels = io.read_u16()? as usize;
            let n_elems = io.read_u16()? as usize;
            let mut n_bytes = io.read_u16()? as usize;

            if n_channels != 3 {
                return Err(CmsError {
                    code: ErrorCode::Range,
                    message: "vcgt table must have 3 channels".to_string(),
                });
            }

            if n_bytes != 1 && n_bytes != 2 {
                return Err(CmsError {
                    code: ErrorCode::Range,
                    message: format!("vcgt: n_bytes must be 1 or 2, got {n_bytes}"),
                });
            }

            // Adobe quirk: sometimes nBytes is wrong
            if n_elems == 256 && n_bytes == 1 && size == 1576 {
                n_bytes = 2;
            }

            // Validate that table data fits in remaining tag size
            // Header: 4 (tag_type) + 2 (n_channels) + 2 (n_elems) + 2 (n_bytes) = 10 bytes
            let data_size = n_channels
                .checked_mul(n_elems)
                .and_then(|v| v.checked_mul(n_bytes))
                .ok_or_else(|| CmsError {
                    code: ErrorCode::Range,
                    message: "vcgt table size overflow".to_string(),
                })?;
            let header_consumed = 10;
            if data_size + header_consumed > size as usize {
                return Err(CmsError {
                    code: ErrorCode::Range,
                    message: "vcgt table data exceeds tag size".to_string(),
                });
            }

            let mut built_curves = Vec::with_capacity(3);
            for _ in 0..3 {
                let mut table = Vec::with_capacity(n_elems);
                for _ in 0..n_elems {
                    let val = match n_bytes {
                        1 => {
                            let v = io.read_u8()? as u16;
                            v * 257 // FROM_8_TO_16
                        }
                        2 => io.read_u16()?,
                        _ => {
                            return Err(CmsError {
                                code: ErrorCode::UnknownExtension,
                                message: format!("vcgt: unsupported entry size {n_bytes}"),
                            });
                        }
                    };
                    table.push(val);
                }
                built_curves.push(ToneCurve::build_tabulated_16(&table).ok_or_else(|| {
                    CmsError {
                        code: ErrorCode::Internal,
                        message: "Failed to build vcgt curve".to_string(),
                    }
                })?);
            }

            let [r, g, b]: [ToneCurve; 3] = built_curves.try_into().ok().expect("exactly 3 curves");
            Ok(TagData::Vcgt(Box::new([r, g, b])))
        }
        VCGT_FORMULA_TYPE => {
            // 3 channels × 3 parameters (Gamma, Min, Max) as S15Fixed16
            let mut gammas = Vec::with_capacity(3);
            for _ in 0..3 {
                let gamma = io.read_s15fixed16()?;
                let min = io.read_s15fixed16()?;
                let max = io.read_s15fixed16()?;
                if gamma <= 0.0 {
                    return Err(CmsError {
                        code: ErrorCode::Range,
                        message: format!("vcgt formula: gamma must be > 0, got {gamma}"),
                    });
                }
                if max < min {
                    return Err(CmsError {
                        code: ErrorCode::Range,
                        message: format!("vcgt formula: max ({max}) < min ({min})"),
                    });
                }
                gammas.push((gamma, min, max));
            }
            let mut built_curves = Vec::with_capacity(3);
            for &(gamma, min, max) in &gammas {
                // Convert to parametric type 5: Y = (aX+b)^G + e
                let a = (max - min).powf(1.0 / gamma);
                let params = [gamma, a, 0.0, 0.0, 0.0, min, 0.0];
                built_curves.push(ToneCurve::build_parametric(5, &params).ok_or_else(|| {
                    CmsError {
                        code: ErrorCode::Internal,
                        message: "Failed to build vcgt parametric curve".to_string(),
                    }
                })?);
            }
            let [r, g, b]: [ToneCurve; 3] = built_curves.try_into().ok().expect("exactly 3 curves");
            Ok(TagData::Vcgt(Box::new([r, g, b])))
        }
        _ => Err(CmsError {
            code: ErrorCode::UnknownExtension,
            message: format!("Unknown vcgt type {tag_type}"),
        }),
    }
}

pub fn write_vcgt_type(io: &mut IoHandler, curves: &[ToneCurve; 3]) -> Result<(), CmsError> {
    // Check if all 3 curves are parametric type 5 (formula-encodable)
    let all_formula = curves.iter().all(|c| c.parametric_type() == 5);

    if all_formula {
        io.write_u32(VCGT_FORMULA_TYPE)?;
        for curve in curves {
            let seg = curve.segment(0).unwrap();
            let gamma = seg.params[0];
            let min = seg.params[5];
            // Max = a^Gamma + Min
            let a = seg.params[1];
            let max = a.powf(gamma) + min;
            io.write_s15fixed16(gamma)?;
            io.write_s15fixed16(min)?;
            io.write_s15fixed16(max)?;
        }
    } else {
        // Table type: always 256 entries, 2 bytes each
        io.write_u32(VCGT_TABLE_TYPE)?;
        io.write_u16(3)?; // channels
        io.write_u16(256)?; // entries
        io.write_u16(2)?; // bytes per entry
        for curve in curves {
            for i in 0..256u16 {
                let input = (i as f32) / 255.0;
                let output = curve.eval_f32(input);
                let val = (output * 65535.0 + 0.5).clamp(0.0, 65535.0) as u16;
                io.write_u16(val)?;
            }
        }
    }

    Ok(())
}

// --- Dict type ---
// C版: Type_Dictionary_Read / Type_Dictionary_Write
// Stores name/value pairs as UTF-16, with optional DisplayName/DisplayValue as MLU.

pub fn read_dict_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let base_offset = io.tell().saturating_sub(8);
    let count = io.read_u32()? as usize;
    let rec_len = io.read_u32()?;

    // Record length determines which fields are present:
    // 16 = Name + Value offsets only
    // 24 = + DisplayName
    // 32 = + DisplayValue
    if rec_len != 16 && rec_len != 24 && rec_len != 32 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Dict: unsupported record length {rec_len} (expected 16, 24, or 32)"),
        });
    }
    let has_display_name = rec_len >= 24;
    let has_display_value = rec_len >= 32;

    if count > 0x10000 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "Dict count too large".to_string(),
        });
    }

    // Read offset/size directory
    struct DictRecord {
        name_offset: u32,
        name_size: u32,
        value_offset: u32,
        value_size: u32,
        display_name_offset: u32,
        display_name_size: u32,
        display_value_offset: u32,
        display_value_size: u32,
    }

    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let name_offset = io.read_u32()?;
        let name_size = io.read_u32()?;
        let value_offset = io.read_u32()?;
        let value_size = io.read_u32()?;
        let (dn_off, dn_sz) = if has_display_name {
            (io.read_u32()?, io.read_u32()?)
        } else {
            (0, 0)
        };
        let (dv_off, dv_sz) = if has_display_value {
            (io.read_u32()?, io.read_u32()?)
        } else {
            (0, 0)
        };
        records.push(DictRecord {
            name_offset: if name_offset > 0 {
                name_offset + base_offset
            } else {
                0
            },
            name_size,
            value_offset: if value_offset > 0 {
                value_offset + base_offset
            } else {
                0
            },
            value_size,
            display_name_offset: if dn_off > 0 { dn_off + base_offset } else { 0 },
            display_name_size: dn_sz,
            display_value_offset: if dv_off > 0 { dv_off + base_offset } else { 0 },
            display_value_size: dv_sz,
        });
    }

    let mut dict = Dict::new();

    for rec in &records {
        // Read name (UTF-16)
        let name = if rec.name_offset > 0 && rec.name_size > 0 {
            io.seek(rec.name_offset);
            read_utf16_string(io, rec.name_size)?
        } else {
            String::new()
        };

        // Read value (UTF-16)
        let value = if rec.value_offset > 0 && rec.value_size > 0 {
            io.seek(rec.value_offset);
            Some(read_utf16_string(io, rec.value_size)?)
        } else {
            None
        };

        // Read DisplayName (MLU)
        let display_name = if rec.display_name_offset > 0 && rec.display_name_size > 0 {
            io.seek(rec.display_name_offset);
            Some(read_embedded_mlu(io, rec.display_name_size)?)
        } else {
            None
        };

        // Read DisplayValue (MLU)
        let display_value = if rec.display_value_offset > 0 && rec.display_value_size > 0 {
            io.seek(rec.display_value_offset);
            Some(read_embedded_mlu(io, rec.display_value_size)?)
        } else {
            None
        };

        dict.add(
            &name,
            value.as_deref(),
            display_name.as_ref(),
            display_value.as_ref(),
        );
    }

    Ok(TagData::Dict(dict))
}

/// Read a UTF-16BE string of the given byte size.
fn read_utf16_string(io: &mut IoHandler, byte_size: u32) -> Result<String, CmsError> {
    if !byte_size.is_multiple_of(2) {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("UTF-16 string byte_size must be even, got {byte_size}"),
        });
    }
    let n_chars = byte_size as usize / 2;
    let mut chars = Vec::with_capacity(n_chars);
    for _ in 0..n_chars {
        chars.push(io.read_u16()?);
    }
    // Remove trailing NUL
    while chars.last() == Some(&0) {
        chars.pop();
    }
    Ok(String::from_utf16_lossy(&chars))
}

/// Write a UTF-16BE string (no type base header).
fn write_utf16_string(io: &mut IoHandler, s: &str) -> Result<(), CmsError> {
    for c in s.encode_utf16() {
        io.write_u16(c)?;
    }
    Ok(())
}

/// Read an embedded MLU (with type base header) for Dict.
fn read_embedded_mlu(io: &mut IoHandler, size: u32) -> Result<Mlu, CmsError> {
    if size < 8 {
        return Ok(Mlu::new());
    }
    let _type_sig = io.read_u32()?;
    let _reserved = io.read_u32()?;
    let payload_size = size.saturating_sub(8);
    match read_mlu_type(io, payload_size) {
        Ok(TagData::Mlu(mlu)) => Ok(mlu),
        _ => Ok(Mlu::new()),
    }
}

/// Write an embedded MLU (with type base header) for Dict.
fn write_embedded_mlu_for_dict(io: &mut IoHandler, mlu: &Mlu) -> Result<(), CmsError> {
    io.write_u32(TagTypeSignature::MultiLocalizedUnicode as u32)?;
    io.write_u32(0)?; // reserved
    write_mlu_type(io, mlu)
}

pub fn write_dict_type(io: &mut IoHandler, dict: &Dict) -> Result<(), CmsError> {
    let base_offset = io.tell().saturating_sub(8);
    let count = dict.len();

    // Determine record length based on whether any display names/values exist
    let has_display_name = dict.iter().any(|e| e.display_name.is_some());
    let has_display_value = dict.iter().any(|e| e.display_value.is_some());
    let rec_len: u32 = if has_display_value {
        32
    } else if has_display_name {
        24
    } else {
        16
    };

    io.write_u32(count as u32)?;
    io.write_u32(rec_len)?;

    // Save directory position, write placeholder
    let dir_pos = io.tell();
    let fields_per_entry = rec_len as usize / 8; // 2, 3, or 4 pairs
    for _ in 0..(count * fields_per_entry) {
        io.write_u32(0)?; // offset placeholder
        io.write_u32(0)?; // size placeholder
    }

    // Write each entry's data, record offsets/sizes
    struct DictOffsets {
        name_offset: u32,
        name_size: u32,
        value_offset: u32,
        value_size: u32,
        display_name_offset: u32,
        display_name_size: u32,
        display_value_offset: u32,
        display_value_size: u32,
    }

    let mut all_offsets = Vec::with_capacity(count);

    for entry in dict.iter() {
        let mut offsets = DictOffsets {
            name_offset: 0,
            name_size: 0,
            value_offset: 0,
            value_size: 0,
            display_name_offset: 0,
            display_name_size: 0,
            display_value_offset: 0,
            display_value_size: 0,
        };

        // Write Name
        let before = io.tell();
        offsets.name_offset = before - base_offset;
        write_utf16_string(io, &entry.name)?;
        offsets.name_size = io.tell() - before;

        // Write Value
        if let Some(ref val) = entry.value {
            let before = io.tell();
            offsets.value_offset = before - base_offset;
            write_utf16_string(io, val)?;
            offsets.value_size = io.tell() - before;
        }

        // Write DisplayName
        if has_display_name && let Some(ref dn) = entry.display_name {
            let before = io.tell();
            offsets.display_name_offset = before - base_offset;
            write_embedded_mlu_for_dict(io, dn)?;
            offsets.display_name_size = io.tell() - before;
        }

        // Write DisplayValue
        if has_display_value && let Some(ref dv) = entry.display_value {
            let before = io.tell();
            offsets.display_value_offset = before - base_offset;
            write_embedded_mlu_for_dict(io, dv)?;
            offsets.display_value_size = io.tell() - before;
        }

        all_offsets.push(offsets);
    }

    // Go back and fill in directory
    let end_pos = io.tell();
    io.seek(dir_pos);
    for offsets in &all_offsets {
        io.write_u32(offsets.name_offset)?;
        io.write_u32(offsets.name_size)?;
        io.write_u32(offsets.value_offset)?;
        io.write_u32(offsets.value_size)?;
        if has_display_name {
            io.write_u32(offsets.display_name_offset)?;
            io.write_u32(offsets.display_name_size)?;
        }
        if has_display_value {
            io.write_u32(offsets.display_value_offset)?;
            io.write_u32(offsets.display_value_size)?;
        }
    }
    io.seek(end_pos);

    Ok(())
}

// --- LUT type helpers ---

/// Convert 8-bit to 16-bit: fills MSB with LSB.
/// C版: `FROM_8_TO_16`
fn from_8_to_16(v: u8) -> u16 {
    ((v as u16) << 8) | (v as u16)
}

/// Convert 16-bit to 8-bit with rounding.
/// C版: `FROM_16_TO_8`
fn from_16_to_8(v: u16) -> u8 {
    (((v as u32) * 65281 + 8388608) >> 24) as u8
}

// --- Lut8/Lut16 helpers ---

/// Read 8-bit curve tables (256 entries per channel) and add as CurveSetElem stage.
fn read_8bit_tables(
    io: &mut IoHandler,
    pipe: &mut Pipeline,
    n_channels: u32,
) -> Result<(), CmsError> {
    let mut curves = Vec::with_capacity(n_channels as usize);
    for _ in 0..n_channels {
        let mut table = vec![0u16; 256];
        for entry in &mut table {
            *entry = from_8_to_16(io.read_u8()?);
        }
        curves.push(
            ToneCurve::build_tabulated_16(&table).ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to build 8-bit curve".to_string(),
            })?,
        );
    }
    let stage = Stage::new_tone_curves(Some(&curves), n_channels).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create curve stage".to_string(),
    })?;
    if !pipe.insert_stage(StageLoc::AtEnd, stage) {
        return Err(CmsError {
            code: ErrorCode::Internal,
            message: "Failed to insert curve stage".to_string(),
        });
    }
    Ok(())
}

/// Write 8-bit curve tables from curves.
fn write_8bit_tables(io: &mut IoHandler, curves: &[ToneCurve]) -> Result<(), CmsError> {
    for curve in curves {
        for i in 0..256u16 {
            let val = curve.eval_u16(from_8_to_16(i as u8));
            io.write_u8(from_16_to_8(val))?;
        }
    }
    Ok(())
}

/// Read 16-bit curve tables and add as CurveSetElem stage.
fn read_16bit_tables(
    io: &mut IoHandler,
    pipe: &mut Pipeline,
    n_channels: u32,
    n_entries: u32,
) -> Result<(), CmsError> {
    let mut curves = Vec::with_capacity(n_channels as usize);
    for _ in 0..n_channels {
        let table = io.read_u16_array(n_entries as usize)?;
        curves.push(
            ToneCurve::build_tabulated_16(&table).ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to build 16-bit curve".to_string(),
            })?,
        );
    }
    let stage = Stage::new_tone_curves(Some(&curves), n_channels).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create curve stage".to_string(),
    })?;
    if !pipe.insert_stage(StageLoc::AtEnd, stage) {
        return Err(CmsError {
            code: ErrorCode::Internal,
            message: "Failed to insert curve stage".to_string(),
        });
    }
    Ok(())
}

/// Write 16-bit curve tables.
fn write_16bit_tables(
    io: &mut IoHandler,
    curves: &[ToneCurve],
    n_entries: u32,
) -> Result<(), CmsError> {
    for curve in curves {
        for i in 0..n_entries {
            let input = if n_entries <= 1 {
                0u16
            } else {
                ((i as u64 * 65535 + (n_entries as u64 - 1) / 2) / (n_entries as u64 - 1)) as u16
            };
            io.write_u16(curve.eval_u16(input))?;
        }
    }
    Ok(())
}

/// Overflow-safe computation: n × base^exp.
/// C版: `uipow`
fn uipow(n: u32, base: u32, exp: u32) -> Option<u32> {
    if base == 0 || n == 0 {
        return Some(0);
    }
    let mut rv: u64 = 1;
    for _ in 0..exp {
        rv = rv.checked_mul(base as u64)?;
        if rv > u32::MAX as u64 {
            return None;
        }
    }
    let result = rv.checked_mul(n as u64)?;
    if result > u32::MAX as u64 {
        return None;
    }
    Some(result as u32)
}

/// Check if a 3×3 matrix (as 9 f64s) is identity.
fn is_matrix_identity(m: &[f64; 9]) -> bool {
    let identity = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
    m.iter()
        .zip(identity.iter())
        .all(|(a, b)| (a - b).abs() < 1.0e-6)
}

// --- Lut8 type ---
// C版: Type_LUT8_Read / Type_LUT8_Write

pub fn read_lut8_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let input_channels = io.read_u8()? as u32;
    let output_channels = io.read_u8()? as u32;
    let clut_points = io.read_u8()? as u32;
    let _padding = io.read_u8()?;

    if input_channels == 0 || output_channels == 0 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "Lut8: zero channels".to_string(),
        });
    }
    if clut_points == 1 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "Lut8: clut_points == 1 is invalid".to_string(),
        });
    }

    let mut pipe = Pipeline::new(input_channels, output_channels).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create pipeline".to_string(),
    })?;

    // Read 3×3 matrix
    let mut matrix = [0.0f64; 9];
    for m in &mut matrix {
        *m = io.read_s15fixed16()?;
    }
    if input_channels == 3 && !is_matrix_identity(&matrix) {
        let stage = Stage::new_matrix(3, 3, &matrix, None).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create matrix stage".to_string(),
        })?;
        if !pipe.insert_stage(StageLoc::AtBegin, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert matrix stage".to_string(),
            });
        }
    }

    // Input curves (256 × u8 per channel)
    read_8bit_tables(io, &mut pipe, input_channels)?;

    // CLUT
    if clut_points > 0 {
        let n_tab_size =
            uipow(output_channels, clut_points, input_channels).ok_or_else(|| CmsError {
                code: ErrorCode::Range,
                message: "Lut8: CLUT size overflow".to_string(),
            })?;
        let mut table = vec![0u16; n_tab_size as usize];
        for entry in &mut table {
            *entry = from_8_to_16(io.read_u8()?);
        }
        let stage = Stage::new_clut_16bit_uniform(
            clut_points,
            input_channels,
            output_channels,
            Some(&table),
        )
        .ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create CLUT stage".to_string(),
        })?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert CLUT stage".to_string(),
            });
        }
    }

    // Output curves (256 × u8 per channel)
    read_8bit_tables(io, &mut pipe, output_channels)?;

    Ok(TagData::Pipeline(pipe))
}

pub fn write_lut8_type(io: &mut IoHandler, pipe: &Pipeline) -> Result<(), CmsError> {
    let in_ch = pipe.input_channels();
    let out_ch = pipe.output_channels();
    let clut_points = pipe
        .stages()
        .iter()
        .find_map(|s| match s.data() {
            StageData::CLut(clut) => Some(clut.params.n_samples[0]),
            _ => None,
        })
        .unwrap_or(0);

    if clut_points > u8::MAX as u32 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Lut8: CLUT grid points {} exceeds u8 range", clut_points),
        });
    }

    io.write_u8(in_ch as u8)?;
    io.write_u8(out_ch as u8)?;
    io.write_u8(clut_points as u8)?;
    io.write_u8(0)?;

    // Write 3×3 matrix (identity if not present)
    let mut matrix = [0.0f64; 9];
    matrix[0] = 1.0;
    matrix[4] = 1.0;
    matrix[8] = 1.0;
    for stage in pipe.stages() {
        if stage.stage_type() == StageSignature::MatrixElem {
            if let StageData::Matrix { coefficients, .. } = stage.data()
                && coefficients.len() >= 9
            {
                matrix[..9].copy_from_slice(&coefficients[..9]);
            }
            break;
        }
    }
    for &m in &matrix {
        io.write_s15fixed16(m)?;
    }

    // Write input curves (first CurveSetElem stage)
    let input_curves: Vec<ToneCurve> = pipe
        .stages()
        .iter()
        .find(|s| s.stage_type() == StageSignature::CurveSetElem)
        .and_then(|s| s.curves().map(|c| c.to_vec()))
        .unwrap_or_else(|| {
            (0..in_ch)
                .map(|_| ToneCurve::build_gamma(1.0).unwrap())
                .collect()
        });
    write_8bit_tables(io, &input_curves)?;

    // Write CLUT
    if clut_points > 0 {
        for stage in pipe.stages() {
            if let StageData::CLut(clut) = stage.data() {
                match &clut.table {
                    CLutTable::U16(data) => {
                        for &v in data {
                            io.write_u8(from_16_to_8(v))?;
                        }
                    }
                    CLutTable::Float(data) => {
                        for &v in data {
                            io.write_u8(from_16_to_8(
                                (v * 65535.0 + 0.5).clamp(0.0, 65535.0) as u16
                            ))?;
                        }
                    }
                }
                break;
            }
        }
    }

    // Write output curves (last CurveSetElem stage)
    let output_curves: Vec<ToneCurve> = pipe
        .stages()
        .iter()
        .rev()
        .find(|s| s.stage_type() == StageSignature::CurveSetElem)
        .and_then(|s| s.curves().map(|c| c.to_vec()))
        .unwrap_or_else(|| {
            (0..out_ch)
                .map(|_| ToneCurve::build_gamma(1.0).unwrap())
                .collect()
        });
    write_8bit_tables(io, &output_curves)?;

    Ok(())
}

// --- Lut16 type ---
// C版: Type_LUT16_Read / Type_LUT16_Write

pub fn read_lut16_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    let input_channels = io.read_u8()? as u32;
    let output_channels = io.read_u8()? as u32;
    let clut_points = io.read_u8()? as u32;
    let _padding = io.read_u8()?;
    let input_entries = io.read_u16()? as u32;
    let output_entries = io.read_u16()? as u32;

    if input_channels == 0 || output_channels == 0 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "Lut16: zero channels".to_string(),
        });
    }
    if clut_points == 1 || input_entries > 0x7FFF || output_entries > 0x7FFF {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: "Lut16: invalid parameters".to_string(),
        });
    }

    let mut pipe = Pipeline::new(input_channels, output_channels).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create pipeline".to_string(),
    })?;

    // Read 3×3 matrix
    let mut matrix = [0.0f64; 9];
    for m in &mut matrix {
        *m = io.read_s15fixed16()?;
    }
    if input_channels == 3 && !is_matrix_identity(&matrix) {
        let stage = Stage::new_matrix(3, 3, &matrix, None).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create matrix stage".to_string(),
        })?;
        if !pipe.insert_stage(StageLoc::AtBegin, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert matrix stage".to_string(),
            });
        }
    }

    // Input curves
    read_16bit_tables(io, &mut pipe, input_channels, input_entries)?;

    // CLUT
    if clut_points > 0 {
        let n_tab_size =
            uipow(output_channels, clut_points, input_channels).ok_or_else(|| CmsError {
                code: ErrorCode::Range,
                message: "Lut16: CLUT size overflow".to_string(),
            })?;
        let table = io.read_u16_array(n_tab_size as usize)?;
        let stage = Stage::new_clut_16bit_uniform(
            clut_points,
            input_channels,
            output_channels,
            Some(&table),
        )
        .ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create CLUT stage".to_string(),
        })?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert CLUT stage".to_string(),
            });
        }
    }

    // Output curves
    read_16bit_tables(io, &mut pipe, output_channels, output_entries)?;

    Ok(TagData::Pipeline(pipe))
}

pub fn write_lut16_type(io: &mut IoHandler, pipe: &Pipeline) -> Result<(), CmsError> {
    let in_ch = pipe.input_channels();
    let out_ch = pipe.output_channels();
    let clut_points = pipe
        .stages()
        .iter()
        .find_map(|s| match s.data() {
            StageData::CLut(clut) => Some(clut.params.n_samples[0]),
            _ => None,
        })
        .unwrap_or(0);

    // Determine curve table sizes
    let input_entries: u32 = pipe
        .stages()
        .iter()
        .find(|s| s.stage_type() == StageSignature::CurveSetElem)
        .and_then(|s| s.curves().map(|c| c[0].table16_len()))
        .unwrap_or(2);
    let output_entries: u32 = pipe
        .stages()
        .iter()
        .rev()
        .find(|s| s.stage_type() == StageSignature::CurveSetElem)
        .and_then(|s| s.curves().map(|c| c[0].table16_len()))
        .unwrap_or(2);

    if clut_points > u8::MAX as u32 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!("Lut16: CLUT grid points {} exceeds u8 range", clut_points),
        });
    }
    if input_entries > u16::MAX as u32 || output_entries > u16::MAX as u32 {
        return Err(CmsError {
            code: ErrorCode::Range,
            message: format!(
                "Lut16: curve entries ({}/{}) exceed u16 range",
                input_entries, output_entries
            ),
        });
    }

    io.write_u8(in_ch as u8)?;
    io.write_u8(out_ch as u8)?;
    io.write_u8(clut_points as u8)?;
    io.write_u8(0)?;
    io.write_u16(input_entries as u16)?;
    io.write_u16(output_entries as u16)?;

    // Matrix
    let mut matrix = [0.0f64; 9];
    matrix[0] = 1.0;
    matrix[4] = 1.0;
    matrix[8] = 1.0;
    for stage in pipe.stages() {
        if stage.stage_type() == StageSignature::MatrixElem {
            if let StageData::Matrix { coefficients, .. } = stage.data()
                && coefficients.len() >= 9
            {
                matrix[..9].copy_from_slice(&coefficients[..9]);
            }
            break;
        }
    }
    for &m in &matrix {
        io.write_s15fixed16(m)?;
    }

    // Input curves
    let input_curves: Vec<ToneCurve> = pipe
        .stages()
        .iter()
        .find(|s| s.stage_type() == StageSignature::CurveSetElem)
        .and_then(|s| s.curves().map(|c| c.to_vec()))
        .unwrap_or_else(|| {
            (0..in_ch)
                .map(|_| ToneCurve::build_gamma(1.0).unwrap())
                .collect()
        });
    write_16bit_tables(io, &input_curves, input_entries)?;

    // CLUT
    if clut_points > 0 {
        for stage in pipe.stages() {
            if let StageData::CLut(clut) = stage.data() {
                match &clut.table {
                    CLutTable::U16(data) => io.write_u16_array(data)?,
                    CLutTable::Float(data) => {
                        for &v in data {
                            io.write_u16((v * 65535.0 + 0.5).clamp(0.0, 65535.0) as u16)?;
                        }
                    }
                }
                break;
            }
        }
    }

    // Output curves
    let output_curves: Vec<ToneCurve> = pipe
        .stages()
        .iter()
        .rev()
        .find(|s| s.stage_type() == StageSignature::CurveSetElem)
        .and_then(|s| s.curves().map(|c| c.to_vec()))
        .unwrap_or_else(|| {
            (0..out_ch)
                .map(|_| ToneCurve::build_gamma(1.0).unwrap())
                .collect()
        });
    write_16bit_tables(io, &output_curves, output_entries)?;

    Ok(())
}

// --- LutAtoB / LutBtoA V4 helpers ---

/// Read embedded V4 curve set (each curve has type base header, 4-byte aligned).
fn read_v4_curve_set(io: &mut IoHandler, offset: u32, n_curves: u32) -> Result<Stage, CmsError> {
    io.seek(offset);
    let mut curves = Vec::with_capacity(n_curves as usize);
    for _ in 0..n_curves {
        let type_sig = io.read_u32()?;
        let _reserved = io.read_u32()?;
        let curve = match TagTypeSignature::try_from(type_sig) {
            Ok(TagTypeSignature::Curve) => match read_curve_type(io, 0)? {
                TagData::Curve(c) => c,
                _ => {
                    return Err(CmsError {
                        code: ErrorCode::UnknownExtension,
                        message: "Expected curve data".to_string(),
                    });
                }
            },
            Ok(TagTypeSignature::ParametricCurve) => match read_parametric_curve_type(io, 0)? {
                TagData::Curve(c) => c,
                _ => {
                    return Err(CmsError {
                        code: ErrorCode::UnknownExtension,
                        message: "Expected parametric curve data".to_string(),
                    });
                }
            },
            _ => {
                return Err(CmsError {
                    code: ErrorCode::UnknownExtension,
                    message: format!("Unknown curve type in V4 LUT: 0x{type_sig:08X}"),
                });
            }
        };
        curves.push(curve);
        // Align to 4-byte boundary
        let pos = io.tell();
        let aligned = (pos + 3) & !3;
        if aligned > pos {
            io.seek(aligned);
        }
    }
    Stage::new_tone_curves(Some(&curves), n_curves).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create V4 curve set".to_string(),
    })
}

/// Write embedded V4 curve set with type base headers.
fn write_v4_curve_set(io: &mut IoHandler, curves: &[ToneCurve]) -> Result<(), CmsError> {
    for curve in curves {
        if curve.parametric_type() != 0 {
            io.write_u32(TagTypeSignature::ParametricCurve as u32)?;
            io.write_u32(0)?;
            write_parametric_curve_type(io, curve)?;
        } else {
            io.write_u32(TagTypeSignature::Curve as u32)?;
            io.write_u32(0)?;
            write_curve_type(io, curve)?;
        }
        // Align to 4-byte boundary
        let pos = io.tell();
        let aligned = (pos + 3) & !3;
        while io.tell() < aligned {
            io.write_u8(0)?;
        }
    }
    Ok(())
}

/// Read V4 CLUT (per-dimension grid + precision byte).
fn read_v4_clut(
    io: &mut IoHandler,
    offset: u32,
    input_channels: u32,
    output_channels: u32,
) -> Result<Stage, CmsError> {
    use crate::curves::intrp::MAX_INPUT_DIMENSIONS;

    io.seek(offset);
    let mut buf = [0u8; MAX_INPUT_DIMENSIONS];
    if !io.read(&mut buf) {
        return Err(IoHandler::read_err());
    }

    let mut grid_points = [0u32; MAX_INPUT_DIMENSIONS];
    for i in 0..input_channels as usize {
        if buf[i] == 1 {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: "V4 CLUT: grid point == 1 is invalid".to_string(),
            });
        }
        grid_points[i] = buf[i] as u32;
    }

    let precision = io.read_u8()?;
    let _pad1 = io.read_u8()?;
    let _pad2 = io.read_u8()?;
    let _pad3 = io.read_u8()?;

    if precision != 1 && precision != 2 {
        return Err(CmsError {
            code: ErrorCode::UnknownExtension,
            message: format!(
                "V4 CLUT: unsupported precision {} (expected 1 or 2)",
                precision
            ),
        });
    }

    // Compute total entries to read
    let temp_stage = Stage::new_clut_16bit(&grid_points, input_channels, output_channels, None)
        .ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create V4 CLUT".to_string(),
        })?;
    let n_entries = match temp_stage.data() {
        StageData::CLut(clut) => clut.n_entries,
        _ => unreachable!(),
    };

    let mut table = Vec::with_capacity(n_entries as usize);
    if precision == 1 {
        for _ in 0..n_entries {
            table.push(from_8_to_16(io.read_u8()?));
        }
    } else {
        for _ in 0..n_entries {
            table.push(io.read_u16()?);
        }
    }

    Stage::new_clut_16bit(&grid_points, input_channels, output_channels, Some(&table)).ok_or_else(
        || CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create V4 CLUT with data".to_string(),
        },
    )
}

/// Write V4 CLUT.
fn write_v4_clut(io: &mut IoHandler, stage: &Stage) -> Result<(), CmsError> {
    use crate::curves::intrp::MAX_INPUT_DIMENSIONS;

    let (params, table) = match stage.data() {
        StageData::CLut(clut) => (&clut.params, &clut.table),
        _ => {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Expected CLUT stage".to_string(),
            });
        }
    };

    for i in 0..MAX_INPUT_DIMENSIONS {
        io.write_u8(params.n_samples[i] as u8)?;
    }
    io.write_u8(2)?; // precision: 16-bit
    io.write_u8(0)?;
    io.write_u8(0)?;
    io.write_u8(0)?;

    match table {
        CLutTable::U16(data) => io.write_u16_array(data)?,
        CLutTable::Float(data) => {
            for &v in data {
                io.write_u16((v * 65535.0 + 0.5).clamp(0.0, 65535.0) as u16)?;
            }
        }
    }
    Ok(())
}

/// Read V4 matrix (3×3 + 3 offsets).
fn read_v4_matrix(io: &mut IoHandler, offset: u32) -> Result<Stage, CmsError> {
    io.seek(offset);
    let mut matrix = [0.0f64; 9];
    for m in &mut matrix {
        *m = io.read_s15fixed16()?;
    }
    let mut off = [0.0f64; 3];
    for o in &mut off {
        *o = io.read_s15fixed16()?;
    }
    Stage::new_matrix(3, 3, &matrix, Some(&off)).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create V4 matrix".to_string(),
    })
}

/// Write V4 matrix (3×3 + 3 offsets).
fn write_v4_matrix(io: &mut IoHandler, stage: &Stage) -> Result<(), CmsError> {
    let (coefficients, offset) = match stage.data() {
        StageData::Matrix {
            coefficients,
            offset,
        } => (coefficients, offset),
        _ => {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Expected matrix stage".to_string(),
            });
        }
    };
    for i in 0..9 {
        io.write_s15fixed16(coefficients.get(i).copied().unwrap_or(0.0))?;
    }
    for i in 0..3 {
        io.write_s15fixed16(
            offset
                .as_ref()
                .and_then(|o| o.get(i).copied())
                .unwrap_or(0.0),
        )?;
    }
    Ok(())
}

// --- LutAtoB type ---
// C版: Type_LUTA2B_Read / Type_LUTA2B_Write

pub fn read_lut_atob_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    // ICC v4 offsets are relative to the tag type start (including 8-byte type base).
    // Our io starts at the payload (after the 8-byte base), so subtract TAG_BASE_SIZE.
    const TAG_BASE_SIZE: u32 = 8;

    let input_channels = io.read_u8()? as u32;
    let output_channels = io.read_u8()? as u32;
    let _padding = io.read_u16()?;

    let offset_b = io.read_u32()?;
    let offset_mat = io.read_u32()?;
    let offset_m = io.read_u32()?;
    let offset_clut = io.read_u32()?;
    let offset_a = io.read_u32()?;

    let mut pipe = Pipeline::new(input_channels, output_channels).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create pipeline".to_string(),
    })?;

    // AtoB order: A → CLUT → M → Matrix → B
    if offset_a != 0 {
        let stage = read_v4_curve_set(io, offset_a - TAG_BASE_SIZE, input_channels)?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert A curves stage".to_string(),
            });
        }
    }
    if offset_clut != 0 {
        let stage = read_v4_clut(
            io,
            offset_clut - TAG_BASE_SIZE,
            input_channels,
            output_channels,
        )?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert CLUT stage".to_string(),
            });
        }
    }
    if offset_m != 0 {
        let stage = read_v4_curve_set(io, offset_m - TAG_BASE_SIZE, output_channels)?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert M curves stage".to_string(),
            });
        }
    }
    if offset_mat != 0 {
        let stage = read_v4_matrix(io, offset_mat - TAG_BASE_SIZE)?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert matrix stage".to_string(),
            });
        }
    }
    if offset_b != 0 {
        let stage = read_v4_curve_set(io, offset_b - TAG_BASE_SIZE, output_channels)?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert B curves stage".to_string(),
            });
        }
    }

    Ok(TagData::Pipeline(pipe))
}

pub fn write_lut_atob_type(io: &mut IoHandler, pipe: &Pipeline) -> Result<(), CmsError> {
    // ICC v4 offsets are relative to the tag type start (including 8-byte type base).
    // Our io starts at the payload (after the 8-byte base), so add TAG_BASE_SIZE.
    const TAG_BASE_SIZE: u32 = 8;

    io.write_u8(pipe.input_channels() as u8)?;
    io.write_u8(pipe.output_channels() as u8)?;
    io.write_u16(0)?;

    let dir_pos = io.tell();
    for _ in 0..5 {
        io.write_u32(0)?; // placeholders
    }

    let mut offset_b = 0u32;
    let mut offset_mat = 0u32;
    let mut offset_m = 0u32;
    let mut offset_clut = 0u32;
    let mut offset_a = 0u32;

    // AtoB = A curves → CLUT → M curves → Matrix → B curves
    // Collect curve positions: mapping depends on total count.
    // 1 curve → B; 2 curves → A + B; 3 curves → A + M + B
    let mut curve_positions = Vec::new();
    for stage in pipe.stages() {
        match stage.stage_type() {
            StageSignature::CurveSetElem => {
                let pos = io.tell() + TAG_BASE_SIZE;
                let curves = stage.curves().unwrap_or(&[]);
                write_v4_curve_set(io, curves)?;
                curve_positions.push(pos);
            }
            StageSignature::CLutElem => {
                offset_clut = io.tell() + TAG_BASE_SIZE;
                write_v4_clut(io, stage)?;
            }
            StageSignature::MatrixElem => {
                offset_mat = io.tell() + TAG_BASE_SIZE;
                write_v4_matrix(io, stage)?;
            }
            _ => {}
        }
    }
    match curve_positions.len() {
        1 => {
            offset_b = curve_positions[0];
        }
        2 => {
            offset_a = curve_positions[0];
            offset_b = curve_positions[1];
        }
        3 => {
            offset_a = curve_positions[0];
            offset_m = curve_positions[1];
            offset_b = curve_positions[2];
        }
        _ => {}
    }

    let end_pos = io.tell();
    io.seek(dir_pos);
    io.write_u32(offset_b)?;
    io.write_u32(offset_mat)?;
    io.write_u32(offset_m)?;
    io.write_u32(offset_clut)?;
    io.write_u32(offset_a)?;
    io.seek(end_pos);

    Ok(())
}

// --- LutBtoA type ---
// C版: Type_LUTB2A_Read / Type_LUTB2A_Write

pub fn read_lut_btoa_type(io: &mut IoHandler, _size: u32) -> Result<TagData, CmsError> {
    const TAG_BASE_SIZE: u32 = 8;

    let input_channels = io.read_u8()? as u32;
    let output_channels = io.read_u8()? as u32;
    let _padding = io.read_u16()?;

    let offset_b = io.read_u32()?;
    let offset_mat = io.read_u32()?;
    let offset_m = io.read_u32()?;
    let offset_clut = io.read_u32()?;
    let offset_a = io.read_u32()?;

    let mut pipe = Pipeline::new(input_channels, output_channels).ok_or_else(|| CmsError {
        code: ErrorCode::Internal,
        message: "Failed to create pipeline".to_string(),
    })?;

    // BtoA order: B → Matrix → M → CLUT → A
    if offset_b != 0 {
        let stage = read_v4_curve_set(io, offset_b - TAG_BASE_SIZE, input_channels)?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert B curves stage".to_string(),
            });
        }
    }
    if offset_mat != 0 {
        let stage = read_v4_matrix(io, offset_mat - TAG_BASE_SIZE)?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert matrix stage".to_string(),
            });
        }
    }
    if offset_m != 0 {
        let stage = read_v4_curve_set(io, offset_m - TAG_BASE_SIZE, input_channels)?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert M curves stage".to_string(),
            });
        }
    }
    if offset_clut != 0 {
        let stage = read_v4_clut(
            io,
            offset_clut - TAG_BASE_SIZE,
            input_channels,
            output_channels,
        )?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert CLUT stage".to_string(),
            });
        }
    }
    if offset_a != 0 {
        let stage = read_v4_curve_set(io, offset_a - TAG_BASE_SIZE, output_channels)?;
        if !pipe.insert_stage(StageLoc::AtEnd, stage) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "Failed to insert A curves stage".to_string(),
            });
        }
    }

    Ok(TagData::Pipeline(pipe))
}

pub fn write_lut_btoa_type(io: &mut IoHandler, pipe: &Pipeline) -> Result<(), CmsError> {
    const TAG_BASE_SIZE: u32 = 8;

    io.write_u8(pipe.input_channels() as u8)?;
    io.write_u8(pipe.output_channels() as u8)?;
    io.write_u16(0)?;

    let dir_pos = io.tell();
    for _ in 0..5 {
        io.write_u32(0)?;
    }

    let mut offset_b = 0u32;
    let mut offset_mat = 0u32;
    let mut offset_m = 0u32;
    let mut offset_clut = 0u32;
    let mut offset_a = 0u32;

    // BtoA = B curves → Matrix → M curves → CLUT → A curves
    // Collect curve positions: mapping depends on total count.
    // 1 curve → A; 2 curves → B + A; 3 curves → B + M + A
    let mut curve_positions = Vec::new();
    for stage in pipe.stages() {
        match stage.stage_type() {
            StageSignature::CurveSetElem => {
                let pos = io.tell() + TAG_BASE_SIZE;
                let curves = stage.curves().unwrap_or(&[]);
                write_v4_curve_set(io, curves)?;
                curve_positions.push(pos);
            }
            StageSignature::CLutElem => {
                offset_clut = io.tell() + TAG_BASE_SIZE;
                write_v4_clut(io, stage)?;
            }
            StageSignature::MatrixElem => {
                offset_mat = io.tell() + TAG_BASE_SIZE;
                write_v4_matrix(io, stage)?;
            }
            _ => {}
        }
    }
    match curve_positions.len() {
        1 => {
            offset_a = curve_positions[0];
        }
        2 => {
            offset_b = curve_positions[0];
            offset_a = curve_positions[1];
        }
        3 => {
            offset_b = curve_positions[0];
            offset_m = curve_positions[1];
            offset_a = curve_positions[2];
        }
        _ => {}
    }

    let end_pos = io.tell();
    io.seek(dir_pos);
    io.write_u32(offset_b)?;
    io.write_u32(offset_mat)?;
    io.write_u32(offset_m)?;
    io.write_u32(offset_clut)?;
    io.write_u32(offset_a)?;
    io.seek(end_pos);

    Ok(())
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

    // ========================================================================
    // Lut8 type round-trip
    // ========================================================================

    #[test]
    fn lut8_type_roundtrip() {
        // Build a 3→3 pipeline: input curves + 3×3 matrix + CLUT + output curves
        let mut pipe = Pipeline::new(3, 3).unwrap();

        // Input curves (identity)
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        // 3×3 identity matrix
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_matrix(3, 3, &matrix, None).unwrap(),
        );

        // Small CLUT (2×2×2 grid, 3 output channels)
        let mut clut_table = vec![0u16; 2 * 2 * 2 * 3];
        // Set one corner to white
        let last = clut_table.len() - 3;
        clut_table[last] = 65535;
        clut_table[last + 1] = 65535;
        clut_table[last + 2] = 65535;
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_clut_16bit_uniform(2, 3, 3, Some(&clut_table)).unwrap(),
        );

        // Output curves (identity)
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        let mut io = IoHandler::from_memory_write(65536);
        write_lut8_type(&mut io, &pipe).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_lut8_type(&mut io, size).unwrap();
        match result {
            TagData::Pipeline(read_pipe) => {
                assert_eq!(read_pipe.input_channels(), 3);
                assert_eq!(read_pipe.output_channels(), 3);
                // Evaluate: black in → black out
                let mut output = [0.0f32; 3];
                read_pipe.eval_float(&[0.0, 0.0, 0.0], &mut output);
                assert!(output[0] < 0.01);
            }
            _ => panic!("Expected TagData::Pipeline"),
        }
    }

    // ========================================================================
    // Lut16 type round-trip
    // ========================================================================

    #[test]
    fn lut16_type_roundtrip() {
        let mut pipe = Pipeline::new(3, 3).unwrap();

        // Input curves
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        // Matrix
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_matrix(3, 3, &matrix, None).unwrap(),
        );

        // CLUT (3→3, 5-point grid)
        let n = 5 * 5 * 5 * 3;
        let mut clut_table = vec![0u16; n];
        let last = clut_table.len() - 3;
        clut_table[last] = 65535;
        clut_table[last + 1] = 65535;
        clut_table[last + 2] = 65535;
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_clut_16bit_uniform(5, 3, 3, Some(&clut_table)).unwrap(),
        );

        // Output curves
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        let mut io = IoHandler::from_memory_write(65536);
        write_lut16_type(&mut io, &pipe).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_lut16_type(&mut io, size).unwrap();
        match result {
            TagData::Pipeline(read_pipe) => {
                assert_eq!(read_pipe.input_channels(), 3);
                assert_eq!(read_pipe.output_channels(), 3);
                let mut output = [0.0f32; 3];
                read_pipe.eval_float(&[1.0, 1.0, 1.0], &mut output);
                assert!(output[0] > 0.99);
            }
            _ => panic!("Expected TagData::Pipeline"),
        }
    }

    // ========================================================================
    // LutAtoB type round-trip
    // ========================================================================

    #[test]
    fn lut_atob_type_roundtrip() {
        // Build: A curves → CLUT → M curves → Matrix → B curves
        let mut pipe = Pipeline::new(3, 3).unwrap();

        // A curves (input)
        let gamma_curve = ToneCurve::build_gamma(2.2).unwrap();
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_tone_curves(
                Some(&[
                    gamma_curve.clone(),
                    gamma_curve.clone(),
                    gamma_curve.clone(),
                ]),
                3,
            )
            .unwrap(),
        );

        // CLUT (3→3, 3-point grid)
        let n = 3 * 3 * 3 * 3;
        let clut_table = vec![32768u16; n]; // mid-gray fill
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_clut_16bit_uniform(3, 3, 3, Some(&clut_table)).unwrap(),
        );

        // M curves
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        // Matrix
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let offset = [0.0, 0.0, 0.0];
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_matrix(3, 3, &matrix, Some(&offset)).unwrap(),
        );

        // B curves (output)
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        let mut io = IoHandler::from_memory_write(65536);
        write_lut_atob_type(&mut io, &pipe).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_lut_atob_type(&mut io, size).unwrap();
        match result {
            TagData::Pipeline(read_pipe) => {
                assert_eq!(read_pipe.input_channels(), 3);
                assert_eq!(read_pipe.output_channels(), 3);
            }
            _ => panic!("Expected TagData::Pipeline"),
        }
    }

    // ========================================================================
    // LutBtoA type round-trip
    // ========================================================================

    #[test]
    fn lut_btoa_type_roundtrip() {
        // Build: B curves → Matrix → M curves → CLUT → A curves
        let mut pipe = Pipeline::new(3, 3).unwrap();

        // B curves
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        // Matrix
        let matrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];
        let offset = [0.0, 0.0, 0.0];
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_matrix(3, 3, &matrix, Some(&offset)).unwrap(),
        );

        // M curves
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        // CLUT (3→3, 3-point grid)
        let n = 3 * 3 * 3 * 3;
        let clut_table = vec![32768u16; n];
        pipe.insert_stage(
            StageLoc::AtEnd,
            Stage::new_clut_16bit_uniform(3, 3, 3, Some(&clut_table)).unwrap(),
        );

        // A curves
        pipe.insert_stage(StageLoc::AtEnd, Stage::new_identity_curves(3).unwrap());

        let mut io = IoHandler::from_memory_write(65536);
        write_lut_btoa_type(&mut io, &pipe).unwrap();
        let size = io.used_space();
        io.seek(0);
        let result = read_lut_btoa_type(&mut io, size).unwrap();
        match result {
            TagData::Pipeline(read_pipe) => {
                assert_eq!(read_pipe.input_channels(), 3);
                assert_eq!(read_pipe.output_channels(), 3);
            }
            _ => panic!("Expected TagData::Pipeline"),
        }
    }
}
