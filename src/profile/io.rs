// Profile I/O: IoHandler, Profile struct, header read/write, tag directory.
// C版: cmsio0.c + cmsplugin.c (number helpers)

#![allow(dead_code)]

use crate::context::CmsError;
use crate::types::{IccHeader, TagSignature};

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
    pub fn new_null() -> Self {
        todo!()
    }

    pub fn from_memory_read(_data: &[u8]) -> Self {
        todo!()
    }

    pub fn from_memory_write(_capacity: usize) -> Self {
        todo!()
    }

    pub fn from_file_read(_path: &str) -> Result<Self, CmsError> {
        todo!()
    }

    pub fn from_file_write(_path: &str) -> Result<Self, CmsError> {
        todo!()
    }

    pub fn read(&mut self, _buf: &mut [u8]) -> bool {
        todo!()
    }

    pub fn write(&mut self, _data: &[u8]) -> bool {
        todo!()
    }

    pub fn seek(&mut self, _pos: u32) -> bool {
        todo!()
    }

    pub fn tell(&self) -> u32 {
        todo!()
    }

    pub fn used_space(&self) -> u32 {
        todo!()
    }

    pub fn into_memory_buffer(self) -> Option<Vec<u8>> {
        todo!()
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // IoHandler basic tests
    // ========================================================================

    #[test]
    #[ignore = "not yet implemented"]
    fn null_handler_tracks_position() {
        let mut io = IoHandler::new_null();
        let data = [0u8; 10];
        assert!(io.write(&data));
        assert_eq!(io.tell(), 10);
        assert_eq!(io.used_space(), 10);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn null_handler_seek() {
        let mut io = IoHandler::new_null();
        let data = [0u8; 20];
        io.write(&data);
        assert!(io.seek(5));
        assert_eq!(io.tell(), 5);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn memory_read_basic() {
        let data = vec![0x12, 0x34, 0x56, 0x78];
        let mut io = IoHandler::from_memory_read(&data);
        let mut buf = [0u8; 4];
        assert!(io.read(&mut buf));
        assert_eq!(buf, [0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn memory_read_past_end() {
        let data = vec![0x12, 0x34];
        let mut io = IoHandler::from_memory_read(&data);
        let mut buf = [0u8; 4];
        assert!(!io.read(&mut buf));
    }

    #[test]
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
    #[ignore = "not yet implemented"]
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
}
