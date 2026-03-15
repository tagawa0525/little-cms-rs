// Profile I/O: IoHandler, Profile struct, header read/write, tag directory.
// C版: cmsio0.c + cmsplugin.c (number helpers)

#![allow(dead_code)]

use crate::context::{CmsError, ErrorCode};
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
                ..
            } => {
                let p = *pointer as usize;
                let size = *reported_size as usize;
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
}
