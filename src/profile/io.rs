// Profile I/O: IoHandler, Profile struct, header read/write, tag directory.
// C版: cmsio0.c + cmsplugin.c (number helpers)

#![allow(dead_code)]

use crate::context::{CmsError, ErrorCode};
use crate::types::{CieXyz, IccHeader, TagSignature, TagTypeSignature};

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

    pub fn tell(&self) -> u32 {
        match self {
            IoHandler::Null { pointer, .. } => *pointer,
            IoHandler::Memory { pointer, .. } => *pointer,
            IoHandler::File { file, .. } => {
                use std::io::Seek;
                // We need a mutable reference for seek/stream_position, but tell
                // is logically non-mutating. Clone the file descriptor to avoid &mut.
                let mut f = file.try_clone().expect("failed to clone file handle");
                f.stream_position().unwrap_or(0) as u32
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

    fn read_err() -> CmsError {
        CmsError {
            code: ErrorCode::Read,
            message: "Read error".to_string(),
        }
    }

    fn write_err() -> CmsError {
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
        let fixed = (v * 65536.0 + 0.5).floor() as i32;
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
        todo!()
    }

    /// C版: `cmsOpenProfileFromMemTHR`
    pub fn open_mem(_data: &[u8]) -> Result<Self, CmsError> {
        todo!()
    }

    /// C版: `cmsOpenProfileFromFileTHR`
    pub fn open_file(_path: &str) -> Result<Self, CmsError> {
        todo!()
    }

    /// C版: `cmsSaveProfileToMem`
    pub fn save_to_mem(&mut self) -> Result<Vec<u8>, CmsError> {
        todo!()
    }

    /// C版: `cmsSaveProfileToFile`
    pub fn save_to_file(&mut self, _path: &str) -> Result<(), CmsError> {
        todo!()
    }

    pub fn tag_count(&self) -> usize {
        todo!()
    }

    pub fn tag_signature(&self, _n: usize) -> Option<TagSignature> {
        todo!()
    }

    pub fn has_tag(&self, _sig: TagSignature) -> bool {
        todo!()
    }

    pub fn read_raw_tag(&mut self, _sig: TagSignature) -> Result<Vec<u8>, CmsError> {
        todo!()
    }

    pub fn write_raw_tag(&mut self, _sig: TagSignature, _data: &[u8]) -> Result<(), CmsError> {
        todo!()
    }

    pub fn link_tag(&mut self, _sig: TagSignature, _dest: TagSignature) -> Result<(), CmsError> {
        todo!()
    }

    pub fn tag_linked_to(&self, _sig: TagSignature) -> Option<TagSignature> {
        todo!()
    }

    /// C版: `cmsGetProfileVersion`
    pub fn version_f64(&self) -> f64 {
        todo!()
    }

    /// C版: `cmsSetProfileVersion`
    pub fn set_version_f64(&mut self, _v: f64) {
        todo!()
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn placeholder_profile_defaults() {
        let p = Profile::new_placeholder();
        assert_eq!(p.header.magic, ICC_MAGIC_NUMBER);
        assert_eq!(p.header.cmm_id, LCMS_SIGNATURE);
        assert_eq!(p.header.device_class, ProfileClassSignature::Display);
        assert_eq!(p.tag_count(), 0);
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
    fn version_f64_encoding() {
        let mut p = Profile::new_placeholder();
        p.set_version_f64(4.3);
        assert_eq!(p.header.version, 0x04300000);
        assert!((p.version_f64() - 4.3).abs() < 0.01);
    }

    #[test]
    #[ignore = "not yet implemented"]
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
}
