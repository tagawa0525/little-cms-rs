//! MD5 hash computation for ICC profile ID (RFC 1321).
//!
//! Matches C版 `cmsmd5.c`. Used to compute profile IDs per ICC spec.

// Per-round shift amounts (RFC 1321 §3.4)
const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

// Precomputed T[i] = floor(2^32 * abs(sin(i+1)))  (RFC 1321 §3.4)
const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

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

impl Md5 {
    pub fn new() -> Self {
        Self {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            count: 0,
            buffer: [0u8; 64],
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        let mut offset = 0;
        let buf_used = (self.count as usize) & 63;

        self.count += data.len() as u64;

        // Fill remaining buffer space
        if buf_used > 0 {
            let space = 64 - buf_used;
            if data.len() < space {
                self.buffer[buf_used..buf_used + data.len()].copy_from_slice(data);
                return;
            }
            self.buffer[buf_used..64].copy_from_slice(&data[..space]);
            Self::transform(&mut self.state, &self.buffer);
            offset = space;
        }

        // Process full 64-byte blocks
        while offset + 64 <= data.len() {
            Self::transform(&mut self.state, &data[offset..offset + 64]);
            offset += 64;
        }

        // Buffer remaining bytes
        let remaining = data.len() - offset;
        if remaining > 0 {
            self.buffer[..remaining].copy_from_slice(&data[offset..]);
        }
    }

    pub fn finish(mut self) -> [u8; 16] {
        let bit_count = self.count * 8;
        let buf_used = (self.count as usize) & 63;

        // Append 0x80 padding byte
        self.buffer[buf_used] = 0x80;
        let pad_start = buf_used + 1;

        if pad_start > 56 {
            // Not enough room for length — fill, transform, then pad a new block
            self.buffer[pad_start..64].fill(0);
            Self::transform(&mut self.state, &self.buffer);
            self.buffer[..56].fill(0);
        } else {
            self.buffer[pad_start..56].fill(0);
        }

        // Append 64-bit length in little-endian
        self.buffer[56..64].copy_from_slice(&bit_count.to_le_bytes());
        Self::transform(&mut self.state, &self.buffer);

        // Output state as little-endian bytes
        let mut digest = [0u8; 16];
        for (i, &word) in self.state.iter().enumerate() {
            digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        digest
    }

    pub fn digest(data: &[u8]) -> [u8; 16] {
        let mut md5 = Self::new();
        md5.update(data);
        md5.finish()
    }

    fn transform(state: &mut [u32; 4], block: &[u8]) {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            *word = u32::from_le_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }

        let [mut a, mut b, mut c, mut d] = *state;

        for i in 0..64 {
            let (f, g) = match i {
                0..16 => ((b & c) | (!b & d), i),
                16..32 => ((d & b) | (!d & c), (5 * i + 1) & 15),
                32..48 => (b ^ c ^ d, (3 * i + 5) & 15),
                _ => (c ^ (b | !d), (7 * i) & 15),
            };

            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(K[i])
                    .wrapping_add(m[g])
                    .rotate_left(S[i]),
            );
            a = temp;
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
