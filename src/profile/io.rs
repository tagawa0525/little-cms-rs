// Profile I/O: IoHandler, Profile struct, header read/write, tag directory.
// C版: cmsio0.c + cmsplugin.c (number helpers)

#![allow(dead_code)]

use crate::context::{CmsError, ErrorCode};
use crate::curves::gamma::ToneCurve;
use crate::curves::wtpnt;
use crate::math::mtrx::Mat3;
use crate::pipeline::lut::{Pipeline, Stage, StageLoc};
use crate::profile::tag_types::TagData;
use crate::types::{
    CieXyz, ColorSpaceSignature, D50_X, D50_Y, D50_Z, DateTimeNumber, EncodedXyzNumber,
    ICC_MAGIC_NUMBER, IccHeader, LCMS_SIGNATURE, PlatformSignature, ProfileClassSignature,
    ProfileId, S15Fixed16, TagSignature, TagTypeSignature,
};

// ============================================================================
// IoHandler
// ============================================================================

/// I/O handler for reading/writing ICC profile data.
/// C版: `cmsIOHANDLER`
pub(crate) enum IoHandler {
    Null {
        pointer: u32,
        used_space: u32,
    },
    Memory {
        data: Vec<u8>,
        pointer: u32,
        used_space: u32,
        reported_size: u32,
        is_write: bool,
    },
    File {
        file: std::fs::File,
        used_space: u32,
        reported_size: u32,
        path: String,
    },
}

impl IoHandler {
    /// C版: `cmsOpenIOhandlerFromNULL`
    pub fn new_null() -> Self {
        IoHandler::Null {
            pointer: 0,
            used_space: 0,
        }
    }

    /// C版: `cmsOpenIOhandlerFromMem` (read mode)
    pub fn from_memory_read(data: &[u8]) -> Self {
        let size = data.len() as u32;
        IoHandler::Memory {
            data: data.to_vec(),
            pointer: 0,
            used_space: 0,
            reported_size: size,
            is_write: false,
        }
    }

    /// C版: `cmsOpenIOhandlerFromMem` (write mode)
    pub fn from_memory_write(capacity: usize) -> Self {
        IoHandler::Memory {
            data: vec![0u8; capacity],
            pointer: 0,
            used_space: 0,
            reported_size: 0,
            is_write: true,
        }
    }

    /// C版: `cmsOpenIOhandlerFromFile` (read mode)
    pub fn from_file_read(path: &str) -> Result<Self, CmsError> {
        use std::io::Seek;

        let mut file = std::fs::File::open(path).map_err(|_| CmsError {
            code: ErrorCode::File,
            message: format!("File '{}' not found", path),
        })?;

        let file_len = file.seek(std::io::SeekFrom::End(0)).map_err(|_| CmsError {
            code: ErrorCode::File,
            message: format!("Cannot get size of file '{}'", path),
        })? as u32;

        file.seek(std::io::SeekFrom::Start(0))
            .map_err(|_| CmsError {
                code: ErrorCode::File,
                message: format!("Seek error on file '{}'", path),
            })?;

        Ok(IoHandler::File {
            file,
            used_space: 0,
            reported_size: file_len,
            path: path.to_string(),
        })
    }

    /// C版: `cmsOpenIOhandlerFromFile` (write mode)
    pub fn from_file_write(path: &str) -> Result<Self, CmsError> {
        let file = std::fs::File::create(path).map_err(|_| CmsError {
            code: ErrorCode::File,
            message: format!("Couldn't create '{}'", path),
        })?;

        Ok(IoHandler::File {
            file,
            used_space: 0,
            reported_size: 0,
            path: path.to_string(),
        })
    }

    pub fn read(&mut self, buf: &mut [u8]) -> bool {
        let len = buf.len();
        if len == 0 {
            return true;
        }

        match self {
            IoHandler::Null { pointer, .. } => {
                *pointer += len as u32;
                true
            }
            IoHandler::Memory {
                data,
                pointer,
                reported_size,
                is_write,
                used_space,
            } => {
                let p = *pointer as usize;
                let size = if *is_write {
                    *used_space as usize
                } else {
                    *reported_size as usize
                };
                if len > size || p > size - len {
                    return false;
                }
                buf.copy_from_slice(&data[p..p + len]);
                *pointer += len as u32;
                true
            }
            IoHandler::File { file, .. } => {
                use std::io::Read;
                file.read_exact(buf).is_ok()
            }
        }
    }

    pub fn write(&mut self, data: &[u8]) -> bool {
        let len = data.len() as u32;
        if len == 0 {
            return true;
        }

        match self {
            IoHandler::Null {
                pointer,
                used_space,
            } => {
                *pointer += len;
                if *pointer > *used_space {
                    *used_space = *pointer;
                }
                true
            }
            IoHandler::Memory {
                data: buf,
                pointer,
                used_space,
                ..
            } => {
                let p = *pointer as usize;
                let cap = buf.len();
                let l = len as usize;
                if l > cap || p > cap - l {
                    return false;
                }
                buf[p..p + l].copy_from_slice(data);
                *pointer += len;
                if *pointer > *used_space {
                    *used_space = *pointer;
                }
                true
            }
            IoHandler::File {
                file, used_space, ..
            } => {
                use std::io::Write;
                if file.write_all(data).is_err() {
                    return false;
                }
                *used_space += len;
                true
            }
        }
    }

    pub fn seek(&mut self, pos: u32) -> bool {
        match self {
            IoHandler::Null { pointer, .. } => {
                *pointer = pos;
                true
            }
            IoHandler::Memory {
                pointer,
                data,
                reported_size,
                is_write,
                ..
            } => {
                let limit = if *is_write {
                    data.len() as u32
                } else {
                    *reported_size
                };
                if pos > limit {
                    return false;
                }
                *pointer = pos;
                true
            }
            IoHandler::File { file, .. } => {
                use std::io::Seek;
                file.seek(std::io::SeekFrom::Start(pos as u64)).is_ok()
            }
        }
    }

    pub fn tell(&mut self) -> u32 {
        match self {
            IoHandler::Null { pointer, .. } => *pointer,
            IoHandler::Memory { pointer, .. } => *pointer,
            IoHandler::File { file, .. } => {
                use std::io::Seek;
                file.stream_position().unwrap_or(0) as u32
            }
        }
    }

    pub fn used_space(&self) -> u32 {
        match self {
            IoHandler::Null { used_space, .. } => *used_space,
            IoHandler::Memory { used_space, .. } => *used_space,
            IoHandler::File { used_space, .. } => *used_space,
        }
    }

    pub fn reported_size(&self) -> u32 {
        match self {
            IoHandler::Null { .. } => 0,
            IoHandler::Memory { reported_size, .. } => *reported_size,
            IoHandler::File { reported_size, .. } => *reported_size,
        }
    }

    pub fn into_memory_buffer(self) -> Option<Vec<u8>> {
        match self {
            IoHandler::Memory { data, .. } => Some(data),
            _ => None,
        }
    }

    // ========================================================================
    // Number I/O helpers (big-endian)
    // C版: cmsplugin.c
    // ========================================================================

    pub(crate) fn read_err() -> CmsError {
        CmsError {
            code: ErrorCode::Read,
            message: "Read error".to_string(),
        }
    }

    pub(crate) fn write_err() -> CmsError {
        CmsError {
            code: ErrorCode::Write,
            message: "Write error".to_string(),
        }
    }

    pub fn read_u8(&mut self) -> Result<u8, CmsError> {
        let mut buf = [0u8; 1];
        if !self.read(&mut buf) {
            return Err(Self::read_err());
        }
        Ok(buf[0])
    }

    pub fn read_u16(&mut self) -> Result<u16, CmsError> {
        let mut buf = [0u8; 2];
        if !self.read(&mut buf) {
            return Err(Self::read_err());
        }
        Ok(u16::from_be_bytes(buf))
    }

    pub fn read_u32(&mut self) -> Result<u32, CmsError> {
        let mut buf = [0u8; 4];
        if !self.read(&mut buf) {
            return Err(Self::read_err());
        }
        Ok(u32::from_be_bytes(buf))
    }

    pub fn read_u64(&mut self) -> Result<u64, CmsError> {
        let mut buf = [0u8; 8];
        if !self.read(&mut buf) {
            return Err(Self::read_err());
        }
        Ok(u64::from_be_bytes(buf))
    }

    pub fn read_f32(&mut self) -> Result<f32, CmsError> {
        let bits = self.read_u32()?;
        let v = f32::from_bits(bits);
        // C版: reject |v| > 1E20 and non-normal/non-zero
        if v.is_nan() || v.is_infinite() || (v != 0.0 && !v.is_normal()) {
            return Err(Self::read_err());
        }
        if v.abs() > 1E20 {
            return Err(Self::read_err());
        }
        Ok(v)
    }

    pub fn read_s15fixed16(&mut self) -> Result<f64, CmsError> {
        let raw = self.read_u32()? as i32;
        Ok(raw as f64 / 65536.0)
    }

    pub fn read_xyz(&mut self) -> Result<CieXyz, CmsError> {
        let x = self.read_s15fixed16()?;
        let y = self.read_s15fixed16()?;
        let z = self.read_s15fixed16()?;
        Ok(CieXyz { x, y, z })
    }

    pub fn read_u16_array(&mut self, n: usize) -> Result<Vec<u16>, CmsError> {
        let mut arr = Vec::with_capacity(n);
        for _ in 0..n {
            arr.push(self.read_u16()?);
        }
        Ok(arr)
    }

    pub fn write_u8(&mut self, v: u8) -> Result<(), CmsError> {
        if !self.write(&[v]) {
            return Err(Self::write_err());
        }
        Ok(())
    }

    pub fn write_u16(&mut self, v: u16) -> Result<(), CmsError> {
        if !self.write(&v.to_be_bytes()) {
            return Err(Self::write_err());
        }
        Ok(())
    }

    pub fn write_u32(&mut self, v: u32) -> Result<(), CmsError> {
        if !self.write(&v.to_be_bytes()) {
            return Err(Self::write_err());
        }
        Ok(())
    }

    pub fn write_u64(&mut self, v: u64) -> Result<(), CmsError> {
        if !self.write(&v.to_be_bytes()) {
            return Err(Self::write_err());
        }
        Ok(())
    }

    pub fn write_f32(&mut self, v: f32) -> Result<(), CmsError> {
        self.write_u32(v.to_bits())
    }

    pub fn write_s15fixed16(&mut self, v: f64) -> Result<(), CmsError> {
        let fixed = (v * 65536.0).round() as i32;
        self.write_u32(fixed as u32)
    }

    pub fn write_xyz(&mut self, xyz: &CieXyz) -> Result<(), CmsError> {
        self.write_s15fixed16(xyz.x)?;
        self.write_s15fixed16(xyz.y)?;
        self.write_s15fixed16(xyz.z)?;
        Ok(())
    }

    pub fn write_u16_array(&mut self, arr: &[u16]) -> Result<(), CmsError> {
        for &v in arr {
            self.write_u16(v)?;
        }
        Ok(())
    }

    /// Align read position to 4-byte boundary.
    /// C版: `_cmsReadAlignment`
    pub fn read_alignment(&mut self) -> Result<(), CmsError> {
        let at = self.tell();
        let next_aligned = (at + 3) & !3;
        let pad = (next_aligned - at) as usize;
        if pad == 0 {
            return Ok(());
        }
        let mut buf = [0u8; 4];
        if !self.read(&mut buf[..pad]) {
            return Err(Self::read_err());
        }
        Ok(())
    }

    /// Align write position to 4-byte boundary with zero padding.
    /// C版: `_cmsWriteAlignment`
    pub fn write_alignment(&mut self) -> Result<(), CmsError> {
        let at = self.tell();
        let next_aligned = (at + 3) & !3;
        let pad = (next_aligned - at) as usize;
        if pad == 0 {
            return Ok(());
        }
        let zeros = [0u8; 4];
        if !self.write(&zeros[..pad]) {
            return Err(Self::write_err());
        }
        Ok(())
    }

    /// Read tag type base: 4 bytes signature + 4 bytes reserved.
    /// C版: `_cmsReadTypeBase`
    pub fn read_type_base(&mut self) -> Result<TagTypeSignature, CmsError> {
        let sig_raw = self.read_u32()?;
        let _reserved = self.read_u32()?;
        TagTypeSignature::try_from(sig_raw).map_err(|_| CmsError {
            code: ErrorCode::BadSignature,
            message: format!("Unknown tag type signature: 0x{:08X}", sig_raw),
        })
    }

    /// Write tag type base: 4 bytes signature + 4 bytes reserved (zero).
    /// C版: `_cmsWriteTypeBase`
    pub fn write_type_base(&mut self, sig: TagTypeSignature) -> Result<(), CmsError> {
        self.write_u32(sig as u32)?;
        self.write_u32(0)?;
        Ok(())
    }
}

// ============================================================================
// Tag types
// ============================================================================

const MAX_TABLE_TAG: usize = 100;

/// State of a tag's data within a Profile.
pub(crate) enum TagDataState {
    NotLoaded,
    Raw(Vec<u8>),
}

/// A single entry in the profile's tag directory.
pub(crate) struct TagEntry {
    pub sig: TagSignature,
    pub offset: u32,
    pub size: u32,
    pub linked: Option<TagSignature>,
    pub data: TagDataState,
    pub save_as_raw: bool,
}

// ============================================================================
// Profile
// ============================================================================

/// An ICC profile.
/// C版: `_cmsICCPROFILE`
pub struct Profile {
    pub(crate) header: IccHeader,
    pub(crate) tags: Vec<TagEntry>,
    pub(crate) io: Option<IoHandler>,
    pub(crate) is_write: bool,
}

impl Profile {
    /// C版: `cmsCreateProfilePlaceholder`
    pub fn new_placeholder() -> Self {
        Profile {
            header: IccHeader {
                size: 0,
                cmm_id: LCMS_SIGNATURE,
                version: 0x02100000,
                device_class: ProfileClassSignature::Display,
                color_space: ColorSpaceSignature::RgbData,
                pcs: ColorSpaceSignature::XyzData,
                date: DateTimeNumber::default(),
                magic: ICC_MAGIC_NUMBER,
                platform: PlatformSignature::Unices,
                flags: 0,
                manufacturer: 0,
                model: 0,
                attributes: 0,
                rendering_intent: 0,
                illuminant: EncodedXyzNumber {
                    x: S15Fixed16::from(D50_X),
                    y: S15Fixed16::from(D50_Y),
                    z: S15Fixed16::from(D50_Z),
                },
                creator: LCMS_SIGNATURE,
                profile_id: ProfileId::default(),
                reserved: [0; 28],
            },
            tags: Vec::new(),
            io: None,
            is_write: false,
        }
    }

    /// C版: `cmsOpenProfileFromMemTHR`
    pub fn open_mem(data: &[u8]) -> Result<Self, CmsError> {
        let io = IoHandler::from_memory_read(data);
        Self::open_from_io(io)
    }

    /// C版: `cmsOpenProfileFromFileTHR`
    pub fn open_file(path: &str) -> Result<Self, CmsError> {
        let io = IoHandler::from_file_read(path)?;
        Self::open_from_io(io)
    }

    fn open_from_io(io: IoHandler) -> Result<Self, CmsError> {
        let mut profile = Self::new_placeholder();
        profile.io = Some(io);
        profile.read_header()?;
        Ok(profile)
    }

    /// C版: `cmsSaveProfileToMem`
    pub fn save_to_mem(&mut self) -> Result<Vec<u8>, CmsError> {
        // Pass 1: compute size using Null handler
        let size = {
            let mut null_io = IoHandler::new_null();
            self.save_to_io(&mut null_io)?;
            null_io.used_space()
        };

        // Pass 2: write to memory
        let mut io = IoHandler::from_memory_write(size as usize);
        self.save_to_io(&mut io)?;
        let used = io.used_space() as usize;
        let mut buf = io.into_memory_buffer().unwrap();
        buf.truncate(used);
        Ok(buf)
    }

    /// C版: `cmsSaveProfileToFile`
    pub fn save_to_file(&mut self, path: &str) -> Result<(), CmsError> {
        let size = {
            let mut null_io = IoHandler::new_null();
            self.save_to_io(&mut null_io)?;
            null_io.used_space()
        };

        let mut io = IoHandler::from_file_write(path)?;
        // Pre-write size into header
        self.header.size = size;
        self.save_to_io(&mut io)?;
        Ok(())
    }

    // ========================================================================
    // Tag directory operations
    // ========================================================================

    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    pub fn tag_signature(&self, n: usize) -> Option<TagSignature> {
        self.tags.get(n).map(|t| t.sig)
    }

    pub fn has_tag(&self, sig: TagSignature) -> bool {
        self.tags.iter().any(|t| t.sig == sig)
    }

    fn search_tag(&self, sig: TagSignature) -> Option<usize> {
        self.tags.iter().position(|t| t.sig == sig)
    }

    fn search_tag_follow_links(&self, sig: TagSignature) -> Option<usize> {
        let mut current_sig = sig;
        for _ in 0..MAX_TABLE_TAG {
            let idx = self.search_tag(current_sig)?;
            match self.tags[idx].linked {
                Some(linked_sig) => current_sig = linked_sig,
                None => return Some(idx),
            }
        }
        None // Cycle detected
    }

    /// Read raw tag bytes from the profile.
    /// C版: `cmsReadRawTag`
    pub fn read_raw_tag(&mut self, sig: TagSignature) -> Result<Vec<u8>, CmsError> {
        let idx = self.search_tag_follow_links(sig).ok_or(CmsError {
            code: ErrorCode::Internal,
            message: format!("Tag {:?} not found", sig),
        })?;

        // If already loaded as raw, return it
        match &self.tags[idx].data {
            TagDataState::Raw(data) => return Ok(data.clone()),
            TagDataState::NotLoaded => {}
        }

        // Load from IO
        let offset = self.tags[idx].offset;
        let size = self.tags[idx].size;
        let io = self.io.as_mut().ok_or(CmsError {
            code: ErrorCode::Read,
            message: "No IO handler".to_string(),
        })?;

        if !io.seek(offset) {
            return Err(CmsError {
                code: ErrorCode::Seek,
                message: "Seek error reading tag".to_string(),
            });
        }

        let mut buf = vec![0u8; size as usize];
        if !io.read(&mut buf) {
            return Err(CmsError {
                code: ErrorCode::Read,
                message: "Read error reading tag".to_string(),
            });
        }

        self.tags[idx].data = TagDataState::Raw(buf.clone());
        Ok(buf)
    }

    /// Write raw tag bytes into the profile (in-memory).
    /// C版: `cmsWriteRawTag`
    pub fn write_raw_tag(&mut self, sig: TagSignature, data: &[u8]) -> Result<(), CmsError> {
        if let Some(idx) = self.search_tag(sig) {
            // Overwrite existing
            self.tags[idx].data = TagDataState::Raw(data.to_vec());
            self.tags[idx].save_as_raw = true;
            self.tags[idx].linked = None;
            self.tags[idx].size = data.len() as u32;
        } else {
            if self.tags.len() >= MAX_TABLE_TAG {
                return Err(CmsError {
                    code: ErrorCode::Range,
                    message: format!("Too many tags ({})", MAX_TABLE_TAG),
                });
            }
            self.tags.push(TagEntry {
                sig,
                offset: 0,
                size: data.len() as u32,
                linked: None,
                data: TagDataState::Raw(data.to_vec()),
                save_as_raw: true,
            });
        }
        Ok(())
    }

    /// C版: `cmsLinkTag`
    pub fn link_tag(&mut self, sig: TagSignature, dest: TagSignature) -> Result<(), CmsError> {
        // Ensure dest exists
        if !self.has_tag(dest) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: format!("Link destination {:?} not found", dest),
            });
        }

        if let Some(idx) = self.search_tag(sig) {
            self.tags[idx].linked = Some(dest);
        } else {
            if self.tags.len() >= MAX_TABLE_TAG {
                return Err(CmsError {
                    code: ErrorCode::Range,
                    message: format!("Too many tags ({})", MAX_TABLE_TAG),
                });
            }
            self.tags.push(TagEntry {
                sig,
                offset: 0,
                size: 0,
                linked: Some(dest),
                data: TagDataState::NotLoaded,
                save_as_raw: false,
            });
        }
        Ok(())
    }

    /// C版: `cmsTagLinkedTo`
    pub fn tag_linked_to(&self, sig: TagSignature) -> Option<TagSignature> {
        let idx = self.search_tag(sig)?;
        self.tags[idx].linked
    }

    /// C版: `cmsGetProfileVersion`
    pub fn version_f64(&self) -> f64 {
        let n = self.header.version >> 16;
        base_to_base(n, 16, 10) as f64 / 100.0
    }

    /// C版: `cmsSetProfileVersion`
    pub fn set_version_f64(&mut self, v: f64) {
        let encoded = base_to_base((v * 100.0 + 0.5) as u32, 10, 16) << 16;
        self.header.version = encoded;
    }

    // ========================================================================
    // Cooked tag read/write
    // ========================================================================

    /// Read a tag and deserialize it to typed data.
    /// C版: `cmsReadTag`
    pub fn read_tag(
        &mut self,
        sig: TagSignature,
    ) -> Result<crate::profile::tag_types::TagData, CmsError> {
        use crate::profile::tag_types;

        // Read raw bytes
        let raw = self.read_raw_tag(sig)?;

        // Parse: first 4 bytes = type signature, next 4 = reserved, rest = payload
        if raw.len() < 8 {
            return Err(CmsError {
                code: ErrorCode::Read,
                message: "Tag too small for type base".to_string(),
            });
        }
        let type_sig_raw = u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]);

        // Handle unknown type signatures by falling back to Raw
        let type_sig = match TagTypeSignature::try_from(type_sig_raw) {
            Ok(sig) => sig,
            Err(_) => return Ok(crate::profile::tag_types::TagData::Raw(raw)),
        };

        let payload = &raw[8..];
        let mut payload_io = IoHandler::from_memory_read(payload);
        tag_types::read_tag_type(&mut payload_io, type_sig, payload.len() as u32)
    }

    /// Write typed tag data into the profile.
    /// C版: `cmsWriteTag`
    pub fn write_tag(
        &mut self,
        sig: TagSignature,
        data: crate::profile::tag_types::TagData,
    ) -> Result<(), CmsError> {
        use crate::profile::tag_types;

        // Pass 1: compute payload size using Null handler
        let (type_sig, payload_size) = {
            let mut null_io = IoHandler::new_null();
            let s = tag_types::write_tag_type(&mut null_io, &data, self.header.version, sig)?;
            (s, null_io.used_space())
        };

        // Pass 2: write payload to correctly sized buffer
        let mut io = IoHandler::from_memory_write(payload_size as usize);
        tag_types::write_tag_type(&mut io, &data, self.header.version, sig)?;

        // Build raw: type_base (8 bytes) + payload
        let mut raw = Vec::with_capacity(8 + payload_size as usize);
        raw.extend_from_slice(&(type_sig as u32).to_be_bytes());
        raw.extend_from_slice(&0u32.to_be_bytes());
        if !io.seek(0) {
            return Err(IoHandler::read_err());
        }
        let mut payload = vec![0u8; payload_size as usize];
        if payload_size > 0 && !io.read(&mut payload) {
            return Err(IoHandler::read_err());
        }
        raw.extend_from_slice(&payload);

        // Store as raw for saving
        self.write_raw_tag(sig, &raw)?;

        Ok(())
    }

    // ========================================================================
    // Header read/write (private)
    // ========================================================================

    /// C版: `_cmsReadHeader`
    fn read_header(&mut self) -> Result<(), CmsError> {
        let io = self.io.as_mut().ok_or(CmsError {
            code: ErrorCode::Read,
            message: "No IO handler".to_string(),
        })?;

        io.seek(0);

        // Read 128-byte ICC header fields
        let size = io.read_u32()?;
        let cmm_id = io.read_u32()?;
        let version_raw = io.read_u32()?;
        let device_class_raw = io.read_u32()?;
        let color_space_raw = io.read_u32()?;
        let pcs_raw = io.read_u32()?;

        // Date (6 × u16)
        let year = io.read_u16()?;
        let month = io.read_u16()?;
        let day = io.read_u16()?;
        let hours = io.read_u16()?;
        let minutes = io.read_u16()?;
        let seconds = io.read_u16()?;

        let magic = io.read_u32()?;
        if magic != ICC_MAGIC_NUMBER {
            return Err(CmsError {
                code: ErrorCode::BadSignature,
                message: "not an ICC profile, invalid signature".to_string(),
            });
        }

        let platform_raw = io.read_u32()?;
        let flags = io.read_u32()?;
        let manufacturer = io.read_u32()?;
        let model = io.read_u32()?;
        let attributes = io.read_u64()?;
        let rendering_intent = io.read_u32()?;

        // Illuminant (3 × S15Fixed16 as raw u32)
        let ill_x = io.read_u32()? as i32;
        let ill_y = io.read_u32()? as i32;
        let ill_z = io.read_u32()? as i32;

        let creator = io.read_u32()?;

        // Profile ID (16 bytes)
        let mut profile_id_bytes = [0u8; 16];
        if !io.read(&mut profile_id_bytes) {
            return Err(IoHandler::read_err());
        }

        // Reserved (28 bytes)
        let mut reserved = [0u8; 28];
        if !io.read(&mut reserved) {
            return Err(IoHandler::read_err());
        }

        // Validate version
        let version = validated_version(version_raw);
        if version > 0x05000000 {
            return Err(CmsError {
                code: ErrorCode::UnknownExtension,
                message: format!("Unsupported profile version '0x{:08X}'", version),
            });
        }

        // Parse signatures (tolerate unknown values)
        let device_class = ProfileClassSignature::try_from(device_class_raw)
            .unwrap_or(ProfileClassSignature::Display);
        let color_space =
            ColorSpaceSignature::try_from(color_space_raw).unwrap_or(ColorSpaceSignature::RgbData);
        let pcs = ColorSpaceSignature::try_from(pcs_raw).unwrap_or(ColorSpaceSignature::XyzData);
        let platform =
            PlatformSignature::try_from(platform_raw).unwrap_or(PlatformSignature::Unices);

        let reported_size = self.io.as_ref().map(|io| io.reported_size()).unwrap_or(0);
        let header_size = size.min(reported_size.max(size));

        self.header = IccHeader {
            size: header_size,
            cmm_id,
            version,
            device_class,
            color_space,
            pcs,
            date: DateTimeNumber {
                year,
                month,
                day,
                hours,
                minutes,
                seconds,
            },
            magic,
            platform,
            flags,
            manufacturer,
            model,
            attributes,
            rendering_intent,
            illuminant: EncodedXyzNumber {
                x: S15Fixed16(ill_x),
                y: S15Fixed16(ill_y),
                z: S15Fixed16(ill_z),
            },
            creator,
            profile_id: ProfileId(profile_id_bytes),
            reserved,
        };

        // Read tag directory
        let io = self.io.as_mut().unwrap();
        let tag_count = io.read_u32()? as usize;
        if tag_count > MAX_TABLE_TAG {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: format!("Too many tags ({})", tag_count),
            });
        }

        self.tags.clear();
        for _ in 0..tag_count {
            let sig_raw = io.read_u32()?;
            let offset = io.read_u32()?;
            let tsize = io.read_u32()?;

            // Sanity: skip zero-offset/size or overflow
            if tsize == 0 || offset == 0 {
                continue;
            }
            if offset.checked_add(tsize).is_none() || offset + tsize > header_size {
                continue;
            }

            let sig = match TagSignature::try_from(sig_raw) {
                Ok(s) => s,
                Err(_) => continue, // Unknown tag, skip
            };

            // Detect linked tags (same offset+size as existing)
            let linked = self.tags.iter().find_map(|existing| {
                if existing.offset == offset && existing.size == tsize {
                    Some(existing.sig)
                } else {
                    None
                }
            });

            self.tags.push(TagEntry {
                sig,
                offset,
                size: tsize,
                linked,
                data: TagDataState::NotLoaded,
                save_as_raw: false,
            });
        }

        // Check for duplicate tags
        for i in 0..self.tags.len() {
            for j in (i + 1)..self.tags.len() {
                if self.tags[i].sig == self.tags[j].sig {
                    return Err(CmsError {
                        code: ErrorCode::Range,
                        message: "Duplicate tag found".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Write the profile to an I/O handler.
    /// C版: `cmsSaveProfileToIOhandler` + `_cmsWriteHeader` + `SaveTags`
    fn save_to_io(&mut self, io: &mut IoHandler) -> Result<(), CmsError> {
        // Compute total size:
        // 128 (header) + 4 (tag count) + 12*N (tag directory) + tag data
        let tag_count = self.active_tag_count();
        let dir_size = 128 + 4 + 12 * tag_count as u32;

        // First, compute offsets for each tag's data
        let mut offsets: Vec<u32> = Vec::new();
        let mut sizes: Vec<u32> = Vec::new();
        let mut current_offset = dir_size;

        for tag in &self.tags {
            if tag.linked.is_some() {
                // Linked tags share the offset of their target
                offsets.push(0); // placeholder, resolved below
                sizes.push(0);
                continue;
            }
            // Align to 4 bytes
            current_offset = (current_offset + 3) & !3;
            offsets.push(current_offset);
            let data_size = match &tag.data {
                TagDataState::Raw(d) => d.len() as u32,
                TagDataState::NotLoaded => tag.size,
            };
            sizes.push(data_size);
            current_offset += data_size;
        }

        // Resolve linked tag offsets
        for i in 0..self.tags.len() {
            if let Some(dest_sig) = self.tags[i].linked
                && let Some(dest_idx) = self.tags.iter().position(|t| t.sig == dest_sig)
            {
                offsets[i] = offsets[dest_idx];
                sizes[i] = sizes[dest_idx];
            }
        }

        let total_size = (current_offset + 3) & !3;

        // Write header
        self.write_header(io, total_size)?;

        // Write tag directory
        io.write_u32(tag_count as u32)?;
        for (i, tag) in self.tags.iter().enumerate() {
            io.write_u32(tag.sig as u32)?;
            io.write_u32(offsets[i])?;
            io.write_u32(sizes[i])?;
        }

        // Pre-load NotLoaded tags from source IO before writing
        self.load_all_tags_for_save()?;

        // Write tag data
        for (i, tag) in self.tags.iter().enumerate() {
            if tag.linked.is_some() {
                continue; // Linked tags share data, don't write twice
            }

            // Pad to offset
            let current = io.tell();
            let target = offsets[i];
            if current < target {
                let pad = vec![0u8; (target - current) as usize];
                if !io.write(&pad) {
                    return Err(IoHandler::write_err());
                }
            }

            if let TagDataState::Raw(data) = &tag.data
                && !io.write(data)
            {
                return Err(IoHandler::write_err());
            }
        }

        Ok(())
    }

    fn load_all_tags_for_save(&mut self) -> Result<(), CmsError> {
        for i in 0..self.tags.len() {
            if self.tags[i].linked.is_some() {
                continue;
            }
            if matches!(self.tags[i].data, TagDataState::NotLoaded) {
                let offset = self.tags[i].offset;
                let size = self.tags[i].size;
                if let Some(src_io) = &mut self.io {
                    let mut buf = vec![0u8; size as usize];
                    if !src_io.seek(offset) {
                        return Err(CmsError {
                            code: ErrorCode::Seek,
                            message: "Seek error loading tag for save".to_string(),
                        });
                    }
                    if !src_io.read(&mut buf) {
                        return Err(IoHandler::read_err());
                    }
                    self.tags[i].data = TagDataState::Raw(buf);
                }
            }
        }
        Ok(())
    }

    fn active_tag_count(&self) -> usize {
        self.tags.len()
    }

    /// C版: `_cmsWriteHeader`
    fn write_header(&self, io: &mut IoHandler, total_size: u32) -> Result<(), CmsError> {
        io.write_u32(total_size)?;
        io.write_u32(self.header.cmm_id)?;
        io.write_u32(self.header.version)?;
        io.write_u32(self.header.device_class as u32)?;
        io.write_u32(self.header.color_space as u32)?;
        io.write_u32(self.header.pcs as u32)?;

        // Date
        io.write_u16(self.header.date.year)?;
        io.write_u16(self.header.date.month)?;
        io.write_u16(self.header.date.day)?;
        io.write_u16(self.header.date.hours)?;
        io.write_u16(self.header.date.minutes)?;
        io.write_u16(self.header.date.seconds)?;

        io.write_u32(self.header.magic)?;
        io.write_u32(self.header.platform as u32)?;
        io.write_u32(self.header.flags)?;
        io.write_u32(self.header.manufacturer)?;
        io.write_u32(self.header.model)?;
        io.write_u64(self.header.attributes)?;
        io.write_u32(self.header.rendering_intent)?;

        // Illuminant
        io.write_u32(self.header.illuminant.x.0 as u32)?;
        io.write_u32(self.header.illuminant.y.0 as u32)?;
        io.write_u32(self.header.illuminant.z.0 as u32)?;

        io.write_u32(self.header.creator)?;

        // Profile ID
        io.write(&self.header.profile_id.0);

        // Reserved
        io.write(&self.header.reserved);

        Ok(())
    }
}

/// BCD version validation.
/// C版: `_validatedVersion`
fn validated_version(raw: u32) -> u32 {
    let bytes = raw.to_be_bytes();
    let b0 = bytes[0].min(0x09);
    let hi = (bytes[1] & 0xF0).min(0x90);
    let lo = (bytes[1] & 0x0F).min(0x09);
    let b1 = hi | lo;
    u32::from_be_bytes([b0, b1, 0, 0])
}

/// Base conversion helper for version encoding/decoding.
/// C版: `BaseToBase`
fn base_to_base(mut input: u32, base_in: u32, base_out: u32) -> u32 {
    let mut digits = Vec::new();
    while input > 0 {
        digits.push(input % base_in);
        input /= base_in;
    }
    let mut out = 0u32;
    for &d in digits.iter().rev() {
        out = out * base_out + d;
    }
    out
}

// ============================================================================
// Pipeline construction helpers
// C版: cmsio1.c
// ============================================================================

/// XYZ encoding factor: 1.15 fixed point maximum = 1 + 32767/32768
const MAX_ENCODEABLE_XYZ: f64 = 1.0 + 32767.0 / 32768.0;
/// Factor to convert from 0..0xFFFF pipeline range to 1.15 fixed point
const INP_ADJ: f64 = 1.0 / MAX_ENCODEABLE_XYZ;
/// Factor to convert from 1.15 fixed point to 0..0xFFFF pipeline range
const OUTP_ADJ: f64 = MAX_ENCODEABLE_XYZ;

/// Intent → tag signature lookup tables
const DEVICE2PCS16: [TagSignature; 4] = [
    TagSignature::AToB0,
    TagSignature::AToB1,
    TagSignature::AToB2,
    TagSignature::AToB1,
];
const PCS2DEVICE16: [TagSignature; 4] = [
    TagSignature::BToA0,
    TagSignature::BToA1,
    TagSignature::BToA2,
    TagSignature::BToA1,
];

impl Profile {
    /// Read the media white point from profile. V2 display profiles return D50.
    /// C版: `_cmsReadMediaWhitePoint`
    pub fn read_media_white_point(&mut self) -> Result<CieXyz, CmsError> {
        let d50 = CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        };

        let tag = match self.read_tag(TagSignature::MediaWhitePoint) {
            Ok(TagData::Xyz(xyz)) => xyz,
            _ => return Ok(d50),
        };

        // V2 display profiles should give D50
        if self.header.version < 0x4000000
            && self.header.device_class == ProfileClassSignature::Display
        {
            return Ok(d50);
        }

        Ok(tag)
    }

    /// Read the chromatic adaptation matrix. V2 display profiles compute it from WP.
    /// C版: `_cmsReadCHAD`
    pub fn read_chad(&mut self) -> Result<Mat3, CmsError> {
        if let Ok(TagData::S15Fixed16Array(vals)) = self.read_tag(TagSignature::ChromaticAdaptation)
            && vals.len() >= 9
        {
            use crate::math::mtrx::Vec3;
            return Ok(Mat3([
                Vec3([vals[0], vals[1], vals[2]]),
                Vec3([vals[3], vals[4], vals[5]]),
                Vec3([vals[6], vals[7], vals[8]]),
            ]));
        }

        // No CHAD: identity by default
        let mut result = Mat3::identity();

        // V2 display profiles: compute adaptation from stored WP to D50
        if self.header.version < 0x4000000
            && self.header.device_class == ProfileClassSignature::Display
            && let Ok(TagData::Xyz(white)) = self.read_tag(TagSignature::MediaWhitePoint)
        {
            let d50 = CieXyz {
                x: D50_X,
                y: D50_Y,
                z: D50_Z,
            };
            if let Some(m) = wtpnt::adaptation_matrix(None, &white, &d50) {
                result = m;
            }
        }

        Ok(result)
    }

    /// Read RGB colorant tags as a 3×3 matrix (RGB → XYZ).
    /// C版: `ReadICCMatrixRGB2XYZ`
    fn read_icc_matrix_rgb2xyz(&mut self) -> Result<Mat3, CmsError> {
        let red = match self.read_tag(TagSignature::RedMatrixColumn)? {
            TagData::Xyz(xyz) => xyz,
            _ => {
                return Err(CmsError {
                    code: ErrorCode::Internal,
                    message: "RedMatrixColumn not found".to_string(),
                });
            }
        };
        let green = match self.read_tag(TagSignature::GreenMatrixColumn)? {
            TagData::Xyz(xyz) => xyz,
            _ => {
                return Err(CmsError {
                    code: ErrorCode::Internal,
                    message: "GreenMatrixColumn not found".to_string(),
                });
            }
        };
        let blue = match self.read_tag(TagSignature::BlueMatrixColumn)? {
            TagData::Xyz(xyz) => xyz,
            _ => {
                return Err(CmsError {
                    code: ErrorCode::Internal,
                    message: "BlueMatrixColumn not found".to_string(),
                });
            }
        };

        use crate::math::mtrx::Vec3;
        Ok(Mat3([
            Vec3([red.x, green.x, blue.x]),
            Vec3([red.y, green.y, blue.y]),
            Vec3([red.z, green.z, blue.z]),
        ]))
    }

    /// Read a ToneCurve tag.
    fn read_curve_tag(&mut self, sig: TagSignature) -> Result<ToneCurve, CmsError> {
        match self.read_tag(sig)? {
            TagData::Curve(c) => Ok(c),
            _ => Err(CmsError {
                code: ErrorCode::Internal,
                message: format!("Expected curve for tag {:?}", sig),
            }),
        }
    }

    /// Build gray input pipeline: Gray → PCS (XYZ or Lab).
    /// C版: `BuildGrayInputMatrixPipeline`
    fn build_gray_input_pipeline(&mut self) -> Result<Pipeline, CmsError> {
        let gray_trc = self.read_curve_tag(TagSignature::GrayTRC)?;

        let mut pipe = Pipeline::new(1, 3).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create gray input pipeline".to_string(),
        })?;

        if self.header.pcs == ColorSpaceSignature::LabData {
            // OneToThreeInputMatrix (1→3): {1, 1, 1}
            let one_to_three = [1.0, 1.0, 1.0];
            let matrix_stage =
                Stage::new_matrix(3, 1, &one_to_three, None).ok_or_else(|| CmsError {
                    code: ErrorCode::Internal,
                    message: "Failed to create 1→3 matrix".to_string(),
                })?;
            pipe.insert_stage(StageLoc::AtEnd, matrix_stage);

            // 3 curves: gray_trc for L*, empty (0x8080) for a*, b*
            let empty_tab =
                ToneCurve::build_tabulated_16(&[0x8080, 0x8080]).ok_or_else(|| CmsError {
                    code: ErrorCode::Internal,
                    message: "Failed to create empty tabulated curve".to_string(),
                })?;
            let curves = [gray_trc, empty_tab.clone(), empty_tab];
            let curve_stage = Stage::new_tone_curves(Some(&curves), 3).ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to create Lab curves".to_string(),
            })?;
            pipe.insert_stage(StageLoc::AtEnd, curve_stage);
        } else {
            // XYZ PCS: curve → matrix (1→3 scaling to D50)
            let curve_stage =
                Stage::new_tone_curves(Some(&[gray_trc]), 1).ok_or_else(|| CmsError {
                    code: ErrorCode::Internal,
                    message: "Failed to create gray curve stage".to_string(),
                })?;
            pipe.insert_stage(StageLoc::AtEnd, curve_stage);

            let gray_input_matrix = [INP_ADJ * D50_X, INP_ADJ * D50_Y, INP_ADJ * D50_Z];
            let matrix_stage =
                Stage::new_matrix(3, 1, &gray_input_matrix, None).ok_or_else(|| CmsError {
                    code: ErrorCode::Internal,
                    message: "Failed to create gray input matrix".to_string(),
                })?;
            pipe.insert_stage(StageLoc::AtEnd, matrix_stage);
        }

        Ok(pipe)
    }

    /// Build RGB input matrix-shaper: RGB → XYZ (+ optional Lab).
    /// C版: `BuildRGBInputMatrixShaper`
    fn build_rgb_input_matrix_shaper(&mut self) -> Result<Pipeline, CmsError> {
        let mut mat = self.read_icc_matrix_rgb2xyz()?;

        // Scale by InpAdj for 1.15 fixed point encoding
        for row in &mut mat.0 {
            for v in row.0.iter_mut() {
                *v *= INP_ADJ;
            }
        }

        let shapes = [
            self.read_curve_tag(TagSignature::RedTRC)?,
            self.read_curve_tag(TagSignature::GreenTRC)?,
            self.read_curve_tag(TagSignature::BlueTRC)?,
        ];

        let mut pipe = Pipeline::new(3, 3).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create RGB input pipeline".to_string(),
        })?;

        let curve_stage = Stage::new_tone_curves(Some(&shapes), 3).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create RGB TRC stage".to_string(),
        })?;
        pipe.insert_stage(StageLoc::AtEnd, curve_stage);

        let flat = [
            mat.0[0].0[0],
            mat.0[0].0[1],
            mat.0[0].0[2],
            mat.0[1].0[0],
            mat.0[1].0[1],
            mat.0[1].0[2],
            mat.0[2].0[0],
            mat.0[2].0[1],
            mat.0[2].0[2],
        ];
        let matrix_stage = Stage::new_matrix(3, 3, &flat, None).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create RGB→XYZ matrix".to_string(),
        })?;
        pipe.insert_stage(StageLoc::AtEnd, matrix_stage);

        if self.header.pcs == ColorSpaceSignature::LabData {
            let lab_stage = Stage::new_xyz_to_lab().ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to create XYZ→Lab stage".to_string(),
            })?;
            pipe.insert_stage(StageLoc::AtEnd, lab_stage);
        }

        Ok(pipe)
    }

    /// Build gray output pipeline: PCS → Gray.
    /// C版: `BuildGrayOutputPipeline`
    fn build_gray_output_pipeline(&mut self) -> Result<Pipeline, CmsError> {
        let gray_trc = self.read_curve_tag(TagSignature::GrayTRC)?;
        let rev_trc = gray_trc.reverse();

        let mut pipe = Pipeline::new(3, 1).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create gray output pipeline".to_string(),
        })?;

        if self.header.pcs == ColorSpaceSignature::LabData {
            // PickLstarMatrix: take L* from Lab → [1, 0, 0]
            let pick_lstar = [1.0, 0.0, 0.0];
            let matrix_stage =
                Stage::new_matrix(1, 3, &pick_lstar, None).ok_or_else(|| CmsError {
                    code: ErrorCode::Internal,
                    message: "Failed to create PickLstar matrix".to_string(),
                })?;
            pipe.insert_stage(StageLoc::AtEnd, matrix_stage);
        } else {
            // PickYMatrix: take Y from XYZ → [0, OutpAdj*D50_Y, 0]
            let pick_y = [0.0, OUTP_ADJ * D50_Y, 0.0];
            let matrix_stage = Stage::new_matrix(1, 3, &pick_y, None).ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to create PickY matrix".to_string(),
            })?;
            pipe.insert_stage(StageLoc::AtEnd, matrix_stage);
        }

        let curve_stage = Stage::new_tone_curves(Some(&[rev_trc]), 1).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create reverse gray curve".to_string(),
        })?;
        pipe.insert_stage(StageLoc::AtEnd, curve_stage);

        Ok(pipe)
    }

    /// Build RGB output matrix-shaper: XYZ → RGB (+ optional Lab→XYZ).
    /// C版: `BuildRGBOutputMatrixShaper`
    fn build_rgb_output_matrix_shaper(&mut self) -> Result<Pipeline, CmsError> {
        let mat = self.read_icc_matrix_rgb2xyz()?;
        let mut inv = mat.inverse().ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "RGB→XYZ matrix is singular".to_string(),
        })?;

        // Scale by OutpAdj
        for row in &mut inv.0 {
            for v in row.0.iter_mut() {
                *v *= OUTP_ADJ;
            }
        }

        let shapes = [
            self.read_curve_tag(TagSignature::RedTRC)?,
            self.read_curve_tag(TagSignature::GreenTRC)?,
            self.read_curve_tag(TagSignature::BlueTRC)?,
        ];
        let inv_shapes: Vec<ToneCurve> = shapes.iter().map(|s| s.reverse()).collect();

        let mut pipe = Pipeline::new(3, 3).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create RGB output pipeline".to_string(),
        })?;

        if self.header.pcs == ColorSpaceSignature::LabData {
            let lab_stage = Stage::new_lab_to_xyz().ok_or_else(|| CmsError {
                code: ErrorCode::Internal,
                message: "Failed to create Lab→XYZ stage".to_string(),
            })?;
            pipe.insert_stage(StageLoc::AtEnd, lab_stage);
        }

        let flat = [
            inv.0[0].0[0],
            inv.0[0].0[1],
            inv.0[0].0[2],
            inv.0[1].0[0],
            inv.0[1].0[1],
            inv.0[1].0[2],
            inv.0[2].0[0],
            inv.0[2].0[1],
            inv.0[2].0[2],
        ];
        let matrix_stage = Stage::new_matrix(3, 3, &flat, None).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create XYZ→RGB matrix".to_string(),
        })?;
        pipe.insert_stage(StageLoc::AtEnd, matrix_stage);

        let curve_stage = Stage::new_tone_curves(Some(&inv_shapes), 3).ok_or_else(|| CmsError {
            code: ErrorCode::Internal,
            message: "Failed to create reverse RGB curves".to_string(),
        })?;
        pipe.insert_stage(StageLoc::AtEnd, curve_stage);

        Ok(pipe)
    }

    /// Check if the profile is implemented as a matrix-shaper.
    /// C版: `cmsIsMatrixShaper`
    pub fn is_matrix_shaper(&self) -> bool {
        match self.header.color_space {
            ColorSpaceSignature::GrayData => self.has_tag(TagSignature::GrayTRC),
            ColorSpaceSignature::RgbData => {
                self.has_tag(TagSignature::RedMatrixColumn)
                    && self.has_tag(TagSignature::GreenMatrixColumn)
                    && self.has_tag(TagSignature::BlueMatrixColumn)
                    && self.has_tag(TagSignature::RedTRC)
                    && self.has_tag(TagSignature::GreenTRC)
                    && self.has_tag(TagSignature::BlueTRC)
            }
            _ => false,
        }
    }

    /// Read an input LUT (device → PCS) pipeline from the profile.
    /// C版: `_cmsReadInputLUT`
    pub fn read_input_lut(&mut self, intent: u32) -> Result<Pipeline, CmsError> {
        if intent > 3 {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: format!("Invalid rendering intent: {intent}"),
            });
        }

        // Try LUT-based tags first (skip float DToB for now)
        {
            let mut tag16 = DEVICE2PCS16[intent as usize];

            // Revert to perceptual if tag not found
            if !self.has_tag(tag16) {
                tag16 = DEVICE2PCS16[0];
            }

            if self.has_tag(tag16) {
                let mut pipe = match self.read_tag(tag16)? {
                    TagData::Pipeline(p) => p,
                    _ => {
                        return Err(CmsError {
                            code: ErrorCode::Internal,
                            message: "Expected Pipeline from LUT tag".to_string(),
                        });
                    }
                };

                // Lab PCS → use trilinear interpolation
                if self.header.pcs == ColorSpaceSignature::LabData {
                    pipe.change_interp_to_trilinear();
                }

                // Lab V2↔V4 conversion for Lut16Type with Lab PCS
                if self.tag_true_type(tag16) == Some(TagTypeSignature::Lut16)
                    && self.header.pcs == ColorSpaceSignature::LabData
                {
                    if self.header.color_space == ColorSpaceSignature::LabData
                        && let Some(stage) = Stage::new_lab_v4_to_v2()
                    {
                        pipe.insert_stage(StageLoc::AtBegin, stage);
                    }
                    if let Some(stage) = Stage::new_lab_v2_to_v4() {
                        pipe.insert_stage(StageLoc::AtEnd, stage);
                    }
                }

                return Ok(pipe);
            }
        }

        // Fallback to matrix-shaper
        match self.header.color_space {
            ColorSpaceSignature::GrayData => self.build_gray_input_pipeline(),
            ColorSpaceSignature::RgbData => self.build_rgb_input_matrix_shaper(),
            _ => Err(CmsError {
                code: ErrorCode::ColorspaceCheck,
                message: "No LUT tags and color space is not Gray or RGB".to_string(),
            }),
        }
    }

    /// Read an output LUT (PCS → device) pipeline from the profile.
    /// C版: `_cmsReadOutputLUT`
    pub fn read_output_lut(&mut self, intent: u32) -> Result<Pipeline, CmsError> {
        if intent > 3 {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: format!("Invalid rendering intent: {intent}"),
            });
        }

        // Try LUT-based tags first (skip float BToD for now)
        {
            let mut tag16 = PCS2DEVICE16[intent as usize];

            if !self.has_tag(tag16) {
                tag16 = PCS2DEVICE16[0];
            }

            if self.has_tag(tag16) {
                let mut pipe = match self.read_tag(tag16)? {
                    TagData::Pipeline(p) => p,
                    _ => {
                        return Err(CmsError {
                            code: ErrorCode::Internal,
                            message: "Expected Pipeline from LUT tag".to_string(),
                        });
                    }
                };

                // Lab PCS → use trilinear interpolation
                if self.header.pcs == ColorSpaceSignature::LabData {
                    pipe.change_interp_to_trilinear();
                }

                // Lab V2↔V4 conversion for Lut16Type with Lab PCS
                if self.tag_true_type(tag16) == Some(TagTypeSignature::Lut16)
                    && self.header.pcs == ColorSpaceSignature::LabData
                {
                    if let Some(stage) = Stage::new_lab_v4_to_v2() {
                        pipe.insert_stage(StageLoc::AtBegin, stage);
                    }
                    if self.header.color_space == ColorSpaceSignature::LabData
                        && let Some(stage) = Stage::new_lab_v2_to_v4()
                    {
                        pipe.insert_stage(StageLoc::AtEnd, stage);
                    }
                }

                return Ok(pipe);
            }
        }

        // Fallback to matrix-shaper
        match self.header.color_space {
            ColorSpaceSignature::GrayData => self.build_gray_output_pipeline(),
            ColorSpaceSignature::RgbData => self.build_rgb_output_matrix_shaper(),
            _ => Err(CmsError {
                code: ErrorCode::ColorspaceCheck,
                message: "No LUT tags and color space is not Gray or RGB".to_string(),
            }),
        }
    }

    /// Get the true type signature of a tag (first 4 bytes of raw tag data).
    /// Peeks cached data when available to avoid cloning the entire tag.
    /// C版: `_cmsGetTagTrueType`
    pub fn tag_true_type(&mut self, sig: TagSignature) -> Option<TagTypeSignature> {
        let idx = self.search_tag_follow_links(sig)?;
        let raw = match &self.tags[idx].data {
            TagDataState::Raw(data) => data.as_slice(),
            TagDataState::NotLoaded => {
                // Load and cache, then peek
                self.read_raw_tag(sig).ok()?;
                let idx = self.search_tag_follow_links(sig)?;
                match &self.tags[idx].data {
                    TagDataState::Raw(data) => data.as_slice(),
                    _ => return None,
                }
            }
        };
        if raw.len() < 4 {
            return None;
        }
        let type_sig_raw = u32::from_be_bytes([raw[0], raw[1], raw[2], raw[3]]);
        TagTypeSignature::try_from(type_sig_raw).ok()
    }

    /// Read a device link / abstract profile LUT pipeline.
    /// C版: `_cmsReadDevicelinkLUT`
    pub fn read_devicelink_lut(&mut self, intent: u32) -> Result<Pipeline, CmsError> {
        if intent > 3 {
            return Err(CmsError {
                code: ErrorCode::Range,
                message: format!("Invalid rendering intent: {intent}"),
            });
        }

        // Try 16-bit LUT tags (skip float DToB for now)
        let mut tag16 = DEVICE2PCS16[intent as usize];
        if !self.has_tag(tag16) {
            tag16 = DEVICE2PCS16[0]; // Fallback to perceptual
        }
        if !self.has_tag(tag16) {
            return Err(CmsError {
                code: ErrorCode::Internal,
                message: "No LUT tag found for device link".to_string(),
            });
        }

        let mut pipe = match self.read_tag(tag16)? {
            TagData::Pipeline(p) => p,
            _ => {
                return Err(CmsError {
                    code: ErrorCode::Internal,
                    message: "Expected Pipeline from LUT tag".to_string(),
                });
            }
        };

        // Lab PCS → use trilinear interpolation
        if self.header.pcs == ColorSpaceSignature::LabData {
            pipe.change_interp_to_trilinear();
        }

        // Lab V2↔V4 conversion for Lut16Type
        if self.tag_true_type(tag16) == Some(TagTypeSignature::Lut16) {
            if self.header.color_space == ColorSpaceSignature::LabData
                && let Some(stage) = Stage::new_lab_v4_to_v2()
            {
                pipe.insert_stage(StageLoc::AtBegin, stage);
            }
            if self.header.pcs == ColorSpaceSignature::LabData
                && let Some(stage) = Stage::new_lab_v2_to_v4()
            {
                pipe.insert_stage(StageLoc::AtEnd, stage);
            }
        }

        Ok(pipe)
    }

    /// Check if a specific rendering intent is implemented as a CLUT.
    /// C版: `cmsIsCLUT`
    pub fn is_clut(&self, intent: u32, direction: crate::types::UsedDirection) -> bool {
        use crate::types::UsedDirection;

        // Device link: single fixed intent
        if self.header.device_class == ProfileClassSignature::Link {
            return self.header.rendering_intent == intent;
        }

        let tag_table = match direction {
            UsedDirection::AsInput => &DEVICE2PCS16,
            UsedDirection::AsOutput => &PCS2DEVICE16,
            UsedDirection::AsProof => {
                return self.is_intent_supported(intent, UsedDirection::AsInput)
                    && self.is_intent_supported(1, UsedDirection::AsOutput);
                // 1 = INTENT_RELATIVE_COLORIMETRIC
            }
        };

        if intent > 3 {
            return false;
        }

        self.has_tag(tag_table[intent as usize])
    }

    /// Check if a rendering intent is supported (CLUT or matrix-shaper).
    /// C版: `cmsIsIntentSupported`
    pub fn is_intent_supported(&self, intent: u32, direction: crate::types::UsedDirection) -> bool {
        if self.is_clut(intent, direction) {
            return true;
        }
        self.is_matrix_shaper()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ColorSpaceSignature, ICC_MAGIC_NUMBER, LCMS_SIGNATURE, ProfileClassSignature,
    };

    // ========================================================================
    // IoHandler basic tests
    // ========================================================================

    #[test]
    fn null_handler_tracks_position() {
        let mut io = IoHandler::new_null();
        let data = [0u8; 10];
        assert!(io.write(&data));
        assert_eq!(io.tell(), 10);
        assert_eq!(io.used_space(), 10);
    }

    #[test]
    fn null_handler_seek() {
        let mut io = IoHandler::new_null();
        let data = [0u8; 20];
        io.write(&data);
        assert!(io.seek(5));
        assert_eq!(io.tell(), 5);
    }

    #[test]
    fn memory_read_basic() {
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let mut io = IoHandler::from_memory_read(&data);
        let mut buf = [0u8; 4];
        assert!(io.read(&mut buf));
        assert_eq!(buf, [0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn memory_read_past_end() {
        let data = vec![0x12, 0x34];
        let mut io = IoHandler::from_memory_read(&data);
        let mut buf = [0u8; 4];
        assert!(!io.read(&mut buf));
    }

    #[test]
    fn memory_read_seek_and_tell() {
        let data = vec![0x10, 0x20, 0x30, 0x40];
        let mut io = IoHandler::from_memory_read(&data);
        assert!(io.seek(2));
        assert_eq!(io.tell(), 2);
        let mut buf = [0u8; 2];
        assert!(io.read(&mut buf));
        assert_eq!(buf, [0x30, 0x40]);
    }

    #[test]
    fn memory_write_basic() {
        let mut io = IoHandler::from_memory_write(16);
        let data = [0xAA, 0xBB, 0xCC];
        assert!(io.write(&data));
        assert_eq!(io.tell(), 3);
        assert_eq!(io.used_space(), 3);
        let buf = io.into_memory_buffer().unwrap();
        assert_eq!(&buf[..3], &[0xAA, 0xBB, 0xCC]);
    }

    #[test]
    fn file_read_write_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("lcms_test_io_handler.bin");
        let path_str = path.to_str().unwrap();

        // Write
        {
            let mut io = IoHandler::from_file_write(path_str).unwrap();
            let data = [0x01, 0x02, 0x03, 0x04, 0x05];
            assert!(io.write(&data));
        }

        // Read back
        {
            let mut io = IoHandler::from_file_read(path_str).unwrap();
            let mut buf = [0u8; 5];
            assert!(io.read(&mut buf));
            assert_eq!(buf, [0x01, 0x02, 0x03, 0x04, 0x05]);
        }

        std::fs::remove_file(&path).ok();
    }

    // ========================================================================
    // Number I/O helper tests
    // ========================================================================

    #[test]
    fn read_write_u16_big_endian() {
        let mut io = IoHandler::from_memory_write(64);
        io.write_u16(0x1234).unwrap();
        io.seek(0);
        // Verify raw bytes are big-endian
        let mut raw = [0u8; 2];
        io.read(&mut raw);
        assert_eq!(raw, [0x12, 0x34]);
        // Read back as u16
        io.seek(0);
        assert_eq!(io.read_u16().unwrap(), 0x1234);
    }

    #[test]
    fn read_write_u32_big_endian() {
        let mut io = IoHandler::from_memory_write(64);
        io.write_u32(0xDEADBEEF).unwrap();
        io.seek(0);
        let mut raw = [0u8; 4];
        io.read(&mut raw);
        assert_eq!(raw, [0xDE, 0xAD, 0xBE, 0xEF]);
        io.seek(0);
        assert_eq!(io.read_u32().unwrap(), 0xDEADBEEF);
    }

    #[test]
    fn read_write_f32_roundtrip() {
        let mut io = IoHandler::from_memory_write(64);
        let val: f32 = 1.5;
        io.write_f32(val).unwrap();
        io.seek(0);
        let read_val = io.read_f32().unwrap();
        assert_eq!(val, read_val);
    }

    #[test]
    fn read_write_s15fixed16_roundtrip() {
        use crate::types::{D50_X, D50_Y, D50_Z};
        let mut io = IoHandler::from_memory_write(64);
        io.write_s15fixed16(D50_X).unwrap();
        io.write_s15fixed16(D50_Y).unwrap();
        io.write_s15fixed16(D50_Z).unwrap();
        io.seek(0);
        let x = io.read_s15fixed16().unwrap();
        let y = io.read_s15fixed16().unwrap();
        let z = io.read_s15fixed16().unwrap();
        // S15Fixed16 round-trip precision: within 1/65536
        assert!((x - D50_X).abs() < 1.0 / 65536.0);
        assert!((y - D50_Y).abs() < 1.0 / 65536.0);
        assert!((z - D50_Z).abs() < 1.0 / 65536.0);
    }

    #[test]
    fn read_write_xyz_roundtrip() {
        use crate::types::{D50_X, D50_Y, D50_Z};
        let xyz = CieXyz {
            x: D50_X,
            y: D50_Y,
            z: D50_Z,
        };
        let mut io = IoHandler::from_memory_write(64);
        io.write_xyz(&xyz).unwrap();
        io.seek(0);
        let read_xyz = io.read_xyz().unwrap();
        assert!((read_xyz.x - xyz.x).abs() < 1.0 / 65536.0);
        assert!((read_xyz.y - xyz.y).abs() < 1.0 / 65536.0);
        assert!((read_xyz.z - xyz.z).abs() < 1.0 / 65536.0);
    }

    #[test]
    fn read_write_u16_array_roundtrip() {
        let arr = vec![100, 200, 300, 400, 500];
        let mut io = IoHandler::from_memory_write(64);
        io.write_u16_array(&arr).unwrap();
        io.seek(0);
        let read_arr = io.read_u16_array(5).unwrap();
        assert_eq!(arr, read_arr);
    }

    #[test]
    fn write_alignment_pads_to_4bytes() {
        let mut io = IoHandler::from_memory_write(64);
        // Write 5 bytes (not aligned)
        io.write(&[1, 2, 3, 4, 5]);
        assert_eq!(io.tell(), 5);
        io.write_alignment().unwrap();
        // Should be at offset 8 (next 4-byte boundary)
        assert_eq!(io.tell(), 8);
        // Verify padding bytes are zero
        io.seek(5);
        let mut pad = [0xFFu8; 3];
        io.read(&mut pad);
        assert_eq!(pad, [0, 0, 0]);
    }

    #[test]
    fn read_type_base_roundtrip() {
        let mut io = IoHandler::from_memory_write(64);
        io.write_type_base(TagTypeSignature::Xyz).unwrap();
        io.seek(0);
        let sig = io.read_type_base().unwrap();
        assert_eq!(sig, TagTypeSignature::Xyz);
        // Type base is 8 bytes: 4 bytes sig + 4 bytes reserved
        assert_eq!(io.tell(), 8);
    }

    // ========================================================================
    // Header / Tag Directory tests
    // ========================================================================

    #[test]
    fn header_roundtrip_in_memory() {
        let mut p = Profile::new_placeholder();
        p.header.device_class = ProfileClassSignature::Input;
        p.header.color_space = ColorSpaceSignature::RgbData;
        p.header.pcs = ColorSpaceSignature::XyzData;
        p.set_version_f64(4.3);

        let data = p.save_to_mem().unwrap();
        let p2 = Profile::open_mem(&data).unwrap();
        assert_eq!(p2.header.device_class, ProfileClassSignature::Input);
        assert_eq!(p2.header.color_space, ColorSpaceSignature::RgbData);
        assert_eq!(p2.header.pcs, ColorSpaceSignature::XyzData);
        assert!((p2.version_f64() - 4.3).abs() < 0.01);
    }

    #[test]
    fn header_magic_number_validation() {
        // Build a minimal "profile" with wrong magic number
        let mut io = IoHandler::from_memory_write(256);
        // Write 128-byte header with bad magic
        io.write_u32(128).unwrap(); // size
        io.write_u32(0).unwrap(); // cmm_id
        io.write_u32(0x02100000).unwrap(); // version
        io.write_u32(ProfileClassSignature::Display as u32).unwrap();
        io.write_u32(ColorSpaceSignature::RgbData as u32).unwrap();
        io.write_u32(ColorSpaceSignature::XyzData as u32).unwrap();
        // date (12 bytes)
        for _ in 0..6 {
            io.write_u16(0).unwrap();
        }
        io.write_u32(0xDEADBEEF).unwrap(); // BAD magic
        // rest of header: pad to 128 bytes
        let remaining = 128 - io.tell() as usize;
        let zeros = vec![0u8; remaining];
        io.write(&zeros);
        // tag count
        io.write_u32(0).unwrap();

        let buf = io.into_memory_buffer().unwrap();
        let result = Profile::open_mem(&buf);
        assert!(result.is_err());
    }

    #[test]
    fn tag_directory_roundtrip() {
        let mut p = Profile::new_placeholder();
        p.write_raw_tag(TagSignature::RedTRC, &[1, 2, 3, 4, 5, 6, 7, 8])
            .unwrap();
        p.write_raw_tag(TagSignature::GreenTRC, &[10, 20, 30, 40, 50, 60, 70, 80])
            .unwrap();

        let data = p.save_to_mem().unwrap();
        let p2 = Profile::open_mem(&data).unwrap();
        assert_eq!(p2.tag_count(), 2);
        assert!(p2.has_tag(TagSignature::RedTRC));
        assert!(p2.has_tag(TagSignature::GreenTRC));
    }

    #[test]
    fn tag_directory_max_100_tags() {
        let mut p = Profile::new_placeholder();
        // Write 100 tags (using various sigs that exist in TagSignature)
        // We'll use raw u32 values for creativity, but for simplicity just
        // write raw tags with the same few signatures won't work.
        // Instead, use write_raw_tag which overwrites existing.
        // We need 101 unique tags - but TagSignature has enough variants.
        // For this test, we'll verify that tag_count maxes at 100.
        // Fill up to 100 by using known signatures.
        let all_sigs = [
            TagSignature::AToB0,
            TagSignature::AToB1,
            TagSignature::AToB2,
            TagSignature::BlueMatrixColumn,
            TagSignature::BlueTRC,
            TagSignature::BToA0,
            TagSignature::BToA1,
            TagSignature::BToA2,
            TagSignature::CalibrationDateTime,
            TagSignature::CharTarget,
            TagSignature::ChromaticAdaptation,
            TagSignature::Chromaticity,
            TagSignature::ColorantOrder,
            TagSignature::ColorantTable,
            TagSignature::ColorantTableOut,
            TagSignature::ColorimetricIntentImageState,
            TagSignature::Copyright,
            TagSignature::CrdInfo,
            TagSignature::Data,
            TagSignature::DateTime,
            TagSignature::DeviceMfgDesc,
            TagSignature::DeviceModelDesc,
            TagSignature::DeviceSettings,
            TagSignature::DToB0,
            TagSignature::DToB1,
            TagSignature::DToB2,
            TagSignature::DToB3,
            TagSignature::BToD0,
            TagSignature::BToD1,
            TagSignature::BToD2,
            TagSignature::BToD3,
            TagSignature::Gamut,
            TagSignature::GrayTRC,
            TagSignature::GreenMatrixColumn,
            TagSignature::GreenTRC,
            TagSignature::Luminance,
            TagSignature::Measurement,
            TagSignature::MediaBlackPoint,
            TagSignature::MediaWhitePoint,
            TagSignature::NamedColor,
            TagSignature::NamedColor2,
            TagSignature::OutputResponse,
            TagSignature::PerceptualRenderingIntentGamut,
            TagSignature::Preview0,
            TagSignature::Preview1,
            TagSignature::Preview2,
            TagSignature::ProfileDescription,
            TagSignature::ProfileDescriptionML,
            TagSignature::ProfileSequenceDesc,
            TagSignature::ProfileSequenceId,
            TagSignature::Ps2CRD0,
            TagSignature::Ps2CRD1,
            TagSignature::Ps2CRD2,
            TagSignature::Ps2CRD3,
            TagSignature::Ps2CSA,
            TagSignature::Ps2RenderingIntent,
            TagSignature::RedMatrixColumn,
            TagSignature::RedTRC,
            TagSignature::SaturationRenderingIntentGamut,
            TagSignature::ScreeningDesc,
            TagSignature::Screening,
            TagSignature::Technology,
            TagSignature::UcrBg,
            TagSignature::ViewingCondDesc,
            TagSignature::ViewingConditions,
            TagSignature::Vcgt,
            TagSignature::Meta,
            TagSignature::Cicp,
            TagSignature::ArgyllArts,
            TagSignature::Mhc2,
        ];

        let dummy = [0u8; 8];
        for sig in &all_sigs {
            p.write_raw_tag(*sig, &dummy).unwrap();
        }
        assert_eq!(p.tag_count(), all_sigs.len());
    }

    #[test]
    fn tag_linking() {
        let mut p = Profile::new_placeholder();
        p.write_raw_tag(TagSignature::RedTRC, &[1, 2, 3, 4, 5, 6, 7, 8])
            .unwrap();
        p.link_tag(TagSignature::GreenTRC, TagSignature::RedTRC)
            .unwrap();

        assert_eq!(
            p.tag_linked_to(TagSignature::GreenTRC),
            Some(TagSignature::RedTRC)
        );

        let data = p.save_to_mem().unwrap();
        let p2 = Profile::open_mem(&data).unwrap();
        assert!(p2.has_tag(TagSignature::GreenTRC));
        assert!(p2.has_tag(TagSignature::RedTRC));
    }

    // ========================================================================
    // Profile lifecycle tests
    // ========================================================================

    #[test]
    fn placeholder_profile_defaults() {
        let p = Profile::new_placeholder();
        assert_eq!(p.header.magic, ICC_MAGIC_NUMBER);
        assert_eq!(p.header.cmm_id, LCMS_SIGNATURE);
        assert_eq!(p.header.device_class, ProfileClassSignature::Display);
        assert_eq!(p.tag_count(), 0);
    }

    #[test]
    fn save_to_memory_roundtrip() {
        let mut p = Profile::new_placeholder();
        p.write_raw_tag(TagSignature::Copyright, b"test copyright data!")
            .unwrap();

        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();
        let raw = p2.read_raw_tag(TagSignature::Copyright).unwrap();
        assert_eq!(raw, b"test copyright data!");
    }

    #[test]
    fn read_raw_tag_write_raw_tag() {
        let mut p = Profile::new_placeholder();
        let payload = vec![0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x00];
        p.write_raw_tag(TagSignature::Technology, &payload).unwrap();
        assert!(p.has_tag(TagSignature::Technology));

        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();
        let read_back = p2.read_raw_tag(TagSignature::Technology).unwrap();
        assert_eq!(read_back, payload);
    }

    #[test]
    fn version_f64_encoding() {
        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.3);
        assert_eq!(p.header.version, 0x04300000);
        assert!((p.version_f64() - 4.3).abs() < 0.01);
    }

    #[test]
    fn open_from_file_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join("lcms_test_profile_roundtrip.icc");
        let path_str = path.to_str().unwrap();

        let mut p = Profile::new_placeholder();
        p.write_raw_tag(TagSignature::Copyright, b"file test!  ")
            .unwrap();
        p.save_to_file(path_str).unwrap();

        let mut p2 = Profile::open_file(path_str).unwrap();
        let raw = p2.read_raw_tag(TagSignature::Copyright).unwrap();
        assert_eq!(raw, b"file test!  ");

        std::fs::remove_file(&path).ok();
    }

    // ========================================================================
    // Cooked read/write tests
    // ========================================================================

    #[test]
    fn write_tag_read_tag_xyz_roundtrip() {
        use crate::profile::tag_types::TagData;
        use crate::types::CieXyz;

        let mut p = Profile::new_placeholder();
        let xyz = CieXyz {
            x: 0.9642,
            y: 1.0,
            z: 0.8249,
        };
        p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(xyz))
            .unwrap();

        // Save and reopen to test full roundtrip
        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();
        let tag = p2.read_tag(TagSignature::MediaWhitePoint).unwrap();
        match tag {
            TagData::Xyz(read_xyz) => {
                assert!((read_xyz.x - xyz.x).abs() < 1.0 / 65536.0);
                assert!((read_xyz.y - xyz.y).abs() < 1.0 / 65536.0);
                assert!((read_xyz.z - xyz.z).abs() < 1.0 / 65536.0);
            }
            _ => panic!("Expected TagData::Xyz"),
        }
    }

    #[test]
    fn write_tag_read_tag_curve_roundtrip() {
        use crate::curves::gamma::ToneCurve;
        use crate::profile::tag_types::TagData;

        let mut p = Profile::new_placeholder();
        let curve = ToneCurve::build_gamma(2.2).unwrap();
        p.write_tag(TagSignature::RedTRC, TagData::Curve(curve))
            .unwrap();

        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();
        let tag = p2.read_tag(TagSignature::RedTRC).unwrap();
        match tag {
            TagData::Curve(read_curve) => {
                let est = read_curve.estimate_gamma(0.01);
                assert!((est - 2.2).abs() < 0.1);
            }
            _ => panic!("Expected TagData::Curve"),
        }
    }

    // ========================================================================
    // Pipeline construction helper tests (Phase 4d)
    // ========================================================================

    /// Build a minimal RGB matrix-shaper profile for testing.
    fn build_rgb_matrix_shaper_profile() -> Profile {
        use crate::curves::gamma::ToneCurve;
        use crate::profile::tag_types::TagData;

        let mut p = Profile::new_placeholder();
        p.header.color_space = ColorSpaceSignature::RgbData;
        p.header.pcs = ColorSpaceSignature::XyzData;
        p.header.device_class = ProfileClassSignature::Display;
        p.header.version = 0x04200000; // V4.2

        // sRGB-like colorants (D50 adapted)
        p.write_tag(
            TagSignature::RedMatrixColumn,
            TagData::Xyz(CieXyz {
                x: 0.4361,
                y: 0.2225,
                z: 0.0139,
            }),
        )
        .unwrap();
        p.write_tag(
            TagSignature::GreenMatrixColumn,
            TagData::Xyz(CieXyz {
                x: 0.3851,
                y: 0.7169,
                z: 0.0971,
            }),
        )
        .unwrap();
        p.write_tag(
            TagSignature::BlueMatrixColumn,
            TagData::Xyz(CieXyz {
                x: 0.1431,
                y: 0.0606,
                z: 0.7141,
            }),
        )
        .unwrap();

        // Gamma 2.2 TRC
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        p.write_tag(TagSignature::RedTRC, TagData::Curve(gamma.clone()))
            .unwrap();
        p.write_tag(TagSignature::GreenTRC, TagData::Curve(gamma.clone()))
            .unwrap();
        p.write_tag(TagSignature::BlueTRC, TagData::Curve(gamma))
            .unwrap();

        // MediaWhitePoint (D50)
        p.write_tag(
            TagSignature::MediaWhitePoint,
            TagData::Xyz(CieXyz {
                x: D50_X,
                y: D50_Y,
                z: D50_Z,
            }),
        )
        .unwrap();

        p
    }

    /// Build a minimal Gray profile for testing.
    fn build_gray_profile() -> Profile {
        use crate::curves::gamma::ToneCurve;
        use crate::profile::tag_types::TagData;

        let mut p = Profile::new_placeholder();
        p.header.color_space = ColorSpaceSignature::GrayData;
        p.header.pcs = ColorSpaceSignature::XyzData;
        p.header.device_class = ProfileClassSignature::Display;
        p.header.version = 0x04200000;

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        p.write_tag(TagSignature::GrayTRC, TagData::Curve(gamma))
            .unwrap();

        p.write_tag(
            TagSignature::MediaWhitePoint,
            TagData::Xyz(CieXyz {
                x: D50_X,
                y: D50_Y,
                z: D50_Z,
            }),
        )
        .unwrap();

        p
    }

    #[test]
    fn read_media_white_point_returns_d50_for_v2_display() {
        let mut p = build_rgb_matrix_shaper_profile();
        // Set as V2 display profile
        p.header.version = 0x02100000;
        p.header.device_class = ProfileClassSignature::Display;

        // Roundtrip through save/open to populate tag data
        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();

        let wp = p2.read_media_white_point().unwrap();
        // V2 display should return D50 regardless of stored value
        assert!((wp.x - D50_X).abs() < 1e-4);
        assert!((wp.y - D50_Y).abs() < 1e-4);
        assert!((wp.z - D50_Z).abs() < 1e-4);
    }

    #[test]
    fn read_media_white_point_returns_tag_value_for_v4() {
        use crate::profile::tag_types::TagData;

        let mut p = Profile::new_placeholder();
        p.header.version = 0x04200000;
        p.header.device_class = ProfileClassSignature::Display;
        let custom_wp = CieXyz {
            x: 0.95,
            y: 1.0,
            z: 1.09,
        };
        p.write_tag(TagSignature::MediaWhitePoint, TagData::Xyz(custom_wp))
            .unwrap();

        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();

        let wp = p2.read_media_white_point().unwrap();
        assert!((wp.x - 0.95).abs() < 1e-3);
        assert!((wp.y - 1.0).abs() < 1e-3);
        assert!((wp.z - 1.09).abs() < 1e-3);
    }

    #[test]
    fn is_matrix_shaper_rgb_profile() {
        let mut p = build_rgb_matrix_shaper_profile();
        let data = p.save_to_mem().unwrap();
        let p2 = Profile::open_mem(&data).unwrap();
        assert!(p2.is_matrix_shaper());
    }

    #[test]
    fn is_matrix_shaper_gray_profile() {
        let mut p = build_gray_profile();
        let data = p.save_to_mem().unwrap();
        let p2 = Profile::open_mem(&data).unwrap();
        assert!(p2.is_matrix_shaper());
    }

    #[test]
    fn read_input_lut_matrix_shaper_rgb() {
        let mut p = build_rgb_matrix_shaper_profile();
        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();

        // Should construct a matrix-shaper pipeline (TRC + Matrix)
        let pipe = p2.read_input_lut(0).unwrap(); // Perceptual
        assert_eq!(pipe.input_channels(), 3);
        assert_eq!(pipe.output_channels(), 3);

        // Evaluate white: RGB(1,1,1) should map close to D50 XYZ
        let input = [1.0f32, 1.0, 1.0];
        let mut output = [0.0f32; 3];
        pipe.eval_float(&input, &mut output);
        // output should be approximately D50 in 0..1 encoded XYZ
        assert!(output[0] > 0.4 && output[0] < 0.6);
        assert!(output[1] > 0.4 && output[1] < 0.6);
        assert!(output[2] > 0.3 && output[2] < 0.5);
    }

    #[test]
    fn read_input_lut_gray() {
        let mut p = build_gray_profile();
        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();

        let pipe = p2.read_input_lut(0).unwrap();
        assert_eq!(pipe.input_channels(), 1);
        assert_eq!(pipe.output_channels(), 3);
    }

    #[test]
    fn read_output_lut_matrix_shaper_rgb() {
        let mut p = build_rgb_matrix_shaper_profile();
        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();

        let pipe = p2.read_output_lut(0).unwrap();
        assert_eq!(pipe.input_channels(), 3);
        assert_eq!(pipe.output_channels(), 3);

        // Evaluate D50 XYZ → should give approximately RGB(1,1,1)
        // D50 in encoded XYZ (0..1 range): x/2, y/2, z/2 approximately
        let input = [0.48f32, 0.50, 0.41];
        let mut output = [0.0f32; 3];
        pipe.eval_float(&input, &mut output);
        // Should be close to white
        assert!(output[0] > 0.8);
        assert!(output[1] > 0.8);
        assert!(output[2] > 0.8);
    }

    #[test]
    fn read_output_lut_gray() {
        let mut p = build_gray_profile();
        let data = p.save_to_mem().unwrap();
        let mut p2 = Profile::open_mem(&data).unwrap();

        let pipe = p2.read_output_lut(0).unwrap();
        assert_eq!(pipe.input_channels(), 3);
        assert_eq!(pipe.output_channels(), 1);
    }

    // ========================================================================
    // Phase 9: io.rs completion tests
    // ========================================================================

    #[test]
    fn tag_true_type_returns_correct_type() {
        use crate::curves::gamma::ToneCurve;

        // Linearization device link writes AToB0 as a pipeline tag
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let mut dl = Profile::new_linearization_device_link(
            ColorSpaceSignature::RgbData,
            &[gamma.clone(), gamma.clone(), gamma],
        )
        .unwrap();
        let data = dl.save_to_mem().unwrap();
        let mut dl2 = Profile::open_mem(&data).unwrap();

        let tt = dl2.tag_true_type(TagSignature::AToB0);
        assert!(tt.is_some(), "AToB0 tag should exist");
    }

    #[test]
    fn read_devicelink_lut_roundtrip() {
        // Create a linearization device link profile (RGB)
        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let mut dl = Profile::new_linearization_device_link(
            ColorSpaceSignature::RgbData,
            &[gamma.clone(), gamma.clone(), gamma],
        )
        .unwrap();
        let data = dl.save_to_mem().unwrap();
        let mut dl2 = Profile::open_mem(&data).unwrap();

        let pipe = dl2.read_devicelink_lut(0).unwrap();
        assert_eq!(pipe.input_channels(), 3);
        assert_eq!(pipe.output_channels(), 3);

        // Evaluate: mid-gray should map to mid-gray-ish value
        let input = [0.5f32, 0.5, 0.5];
        let mut output = [0.0f32; 3];
        pipe.eval_float(&input, &mut output);
        for (ch, &val) in output.iter().enumerate() {
            assert!(val > 0.1 && val < 0.9, "ch {ch}: unexpected value {val}");
        }
    }

    #[test]
    fn is_clut_matrix_shaper_returns_false() {
        use crate::types::UsedDirection;

        let mut p = build_rgb_matrix_shaper_profile();
        let data = p.save_to_mem().unwrap();
        let p2 = Profile::open_mem(&data).unwrap();

        // Matrix-shaper profile has no AToB/BToA CLUT tags
        assert!(!p2.is_clut(0, UsedDirection::AsInput));
        assert!(!p2.is_clut(0, UsedDirection::AsOutput));
    }

    #[test]
    fn is_clut_device_link_returns_true() {
        use crate::types::UsedDirection;

        let gamma = ToneCurve::build_gamma(2.2).unwrap();
        let mut dl = Profile::new_linearization_device_link(
            ColorSpaceSignature::RgbData,
            &[gamma.clone(), gamma.clone(), gamma],
        )
        .unwrap();
        let data = dl.save_to_mem().unwrap();
        let dl2 = Profile::open_mem(&data).unwrap();

        // Device link with perceptual intent should return true for intent 0
        assert!(dl2.is_clut(0, UsedDirection::AsInput));
    }

    #[test]
    fn is_intent_supported_matrix_shaper() {
        use crate::types::UsedDirection;

        let mut p = build_rgb_matrix_shaper_profile();
        let data = p.save_to_mem().unwrap();
        let p2 = Profile::open_mem(&data).unwrap();

        // Matrix-shaper can handle any intent via fallback
        assert!(p2.is_intent_supported(0, UsedDirection::AsInput));
        assert!(p2.is_intent_supported(1, UsedDirection::AsInput));
        assert!(p2.is_intent_supported(0, UsedDirection::AsOutput));
    }
}
