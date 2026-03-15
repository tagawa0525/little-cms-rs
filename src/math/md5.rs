//! MD5 hash computation for ICC profile ID (RFC 1321).
//!
//! Matches C版 `cmsmd5.c`. Used to compute profile IDs per ICC spec.

#[allow(dead_code)]
pub struct Md5 {
    state: [u32; 4],
    count: u64,
    buffer: [u8; 64],
}

impl Default for Md5 {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl Md5 {
    pub fn new() -> Self {
        todo!()
    }

    pub fn update(&mut self, _data: &[u8]) {
        todo!()
    }

    pub fn finish(self) -> [u8; 16] {
        todo!()
    }

    pub fn digest(_data: &[u8]) -> [u8; 16] {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "not yet implemented"]
    fn md5_empty() {
        // RFC 1321 test vector: MD5("") = d41d8cd98f00b204e9800998ecf8427e
        let digest = Md5::digest(b"");
        assert_eq!(
            digest,
            [
                0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04, 0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8,
                0x42, 0x7e
            ]
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn md5_a() {
        // MD5("a") = 0cc175b9c0f1b6a831c399e269772661
        let digest = Md5::digest(b"a");
        assert_eq!(
            digest,
            [
                0x0c, 0xc1, 0x75, 0xb9, 0xc0, 0xf1, 0xb6, 0xa8, 0x31, 0xc3, 0x99, 0xe2, 0x69, 0x77,
                0x26, 0x61
            ]
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn md5_abc() {
        // MD5("abc") = 900150983cd24fb0d6963f7d28e17f72
        let digest = Md5::digest(b"abc");
        assert_eq!(
            digest,
            [
                0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1,
                0x7f, 0x72
            ]
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn md5_alphabet() {
        // MD5("abcdefghijklmnopqrstuvwxyz") = c3fcd3d76192e4007dfb496cca67e13b
        let digest = Md5::digest(b"abcdefghijklmnopqrstuvwxyz");
        assert_eq!(
            digest,
            [
                0xc3, 0xfc, 0xd3, 0xd7, 0x61, 0x92, 0xe4, 0x00, 0x7d, 0xfb, 0x49, 0x6c, 0xca, 0x67,
                0xe1, 0x3b
            ]
        );
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn md5_incremental() {
        // Incremental update should produce same result as single digest
        let mut md5 = Md5::new();
        md5.update(b"abc");
        md5.update(b"defghijklmnopqrstuvwxyz");
        let incremental = md5.finish();
        let single = Md5::digest(b"abcdefghijklmnopqrstuvwxyz");
        assert_eq!(incremental, single);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn md5_message_digest() {
        // MD5("message digest") = f96b697d7cb7938d525a2f31aaf161d0
        let digest = Md5::digest(b"message digest");
        assert_eq!(
            digest,
            [
                0xf9, 0x6b, 0x69, 0x7d, 0x7c, 0xb7, 0x93, 0x8d, 0x52, 0x5a, 0x2f, 0x31, 0xaa, 0xf1,
                0x61, 0xd0
            ]
        );
    }
}
