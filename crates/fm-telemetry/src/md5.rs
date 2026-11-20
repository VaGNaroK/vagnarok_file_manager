//! Implementação pura em Rust do algoritmo MD5 (RFC 1321) em streaming.
//!
//! Mantém zero dependências externas adicionais no `Cargo.lock` (compatível com build
//! offline `--locked --offline`), suporta cancelamento cooperativo por chunk de `64 KiB`
//! e usa `open_shared_read` para leitura não-bloqueante no Windows.

use std::fmt::Write as _;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use tokio_util::sync::CancellationToken;

use crate::open_shared::open_shared_read;

/// Arquivos `<= 32 KiB` fazem curto-circuito no Estágio 2, calculando MD5 Completo direto
/// em uma única leitura sequencial (evita 3 `seek`s e sobreposição de blocos de 8 KiB).
pub const PARTIAL_SHORT_CIRCUIT_BYTES: u64 = 32 * 1024;
/// Tamanho de cada janela (início, meio e fim) do MD5 Parcial para arquivos `> 32 KiB`.
pub const PARTIAL_CHUNK_BYTES: usize = 8 * 1024;
/// Tamanho do buffer de streaming para MD5 Completo.
pub const STREAM_BUFFER_BYTES: usize = 64 * 1024;

const SHIFTS: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

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

/// Hasher MD5 incremental compatível com RFC 1321.
#[derive(Debug, Clone)]
pub struct Md5Hasher {
    state: [u32; 4],
    total_len: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Default for Md5Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Md5Hasher {
    pub fn new() -> Self {
        Self {
            state: [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476],
            total_len: 0,
            buffer: [0u8; 64],
            buffer_len: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.total_len = self.total_len.wrapping_add(data.len() as u64);

        if self.buffer_len > 0 {
            let need = 64 - self.buffer_len;
            if data.len() < need {
                self.buffer[self.buffer_len..self.buffer_len + data.len()].copy_from_slice(data);
                self.buffer_len += data.len();
                return;
            }
            self.buffer[self.buffer_len..64].copy_from_slice(&data[..need]);
            let block = self.buffer;
            self.compress(&block);
            self.buffer_len = 0;
            data = &data[need..];
        }

        let (chunks, rem) = data.as_chunks::<64>();
        for block in chunks {
            self.compress(block);
        }

        if !rem.is_empty() {
            self.buffer[..rem.len()].copy_from_slice(rem);
            self.buffer_len = rem.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 16] {
        let bit_len = self.total_len.wrapping_mul(8);
        let mut pad = [0u8; 64];
        pad[0] = 0x80;

        let pad_len = if self.buffer_len < 56 {
            56 - self.buffer_len
        } else {
            120 - self.buffer_len
        };
        self.update(&pad[..pad_len]);
        self.update(&bit_len.to_le_bytes());

        let mut out = [0u8; 16];
        for (i, word) in self.state.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_le_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut m = [0u32; 16];
        for (i, chunk) in block.as_chunks::<4>().0.iter().enumerate() {
            m[i] = u32::from_le_bytes(*chunk);
        }

        let [mut a, mut b, mut c, mut d] = self.state;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | ((!b) & d), i),
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            let sum = a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g]);
            b = b.wrapping_add(sum.rotate_left(SHIFTS[i]));
            a = temp;
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}

/// Converte o digest binário de 16 bytes para string hexadecimal minúscula de 32 caracteres.
pub fn format_md5_hex(digest: &[u8; 16]) -> String {
    let mut out = String::with_capacity(32);
    for b in digest {
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Calcula o MD5 Parcial (8 KiB início + 8 KiB meio + 8 KiB fim) para arquivos `> 32 KiB`.
///
/// Para arquivos `<= 32 KiB`, prefira `compute_full_md5_blocking` diretamente (curto-circuito).
pub fn compute_partial_md5_blocking(
    path: &Path,
    file_size: u64,
    cancel: &CancellationToken,
) -> Option<[u8; 16]> {
    if cancel.is_cancelled() {
        return None;
    }
    if file_size <= PARTIAL_SHORT_CIRCUIT_BYTES {
        return compute_full_md5_blocking(path, cancel);
    }

    let mut file = open_shared_read(path).ok()?;
    let mut hasher = Md5Hasher::new();
    hasher.update(&file_size.to_le_bytes());

    let mut buf = [0u8; PARTIAL_CHUNK_BYTES];
    let offsets = [
        0u64,
        (file_size / 2).saturating_sub((PARTIAL_CHUNK_BYTES / 2) as u64),
        file_size.saturating_sub(PARTIAL_CHUNK_BYTES as u64),
    ];

    for offset in offsets {
        if cancel.is_cancelled() {
            return None;
        }
        file.seek(SeekFrom::Start(offset)).ok()?;
        let n = read_up_to(&mut file, &mut buf, cancel)?;
        hasher.update(&buf[..n]);
    }

    Some(hasher.finalize())
}

/// Calcula o MD5 Completo em streaming (`64 KiB` por bloco) com cancelamento cooperativo.
pub fn compute_full_md5_blocking(path: &Path, cancel: &CancellationToken) -> Option<[u8; 16]> {
    if cancel.is_cancelled() {
        return None;
    }
    let mut file = open_shared_read(path).ok()?;
    let mut hasher = Md5Hasher::new();
    let mut buf = [0u8; STREAM_BUFFER_BYTES];

    loop {
        if cancel.is_cancelled() {
            return None;
        }
        let read = file.read(&mut buf).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }

    Some(hasher.finalize())
}

fn read_up_to<R: Read>(
    reader: &mut R,
    buf: &mut [u8],
    cancel: &CancellationToken,
) -> Option<usize> {
    let mut total = 0;
    while total < buf.len() {
        if cancel.is_cancelled() {
            return None;
        }
        let n = reader.read(&mut buf[total..]).ok()?;
        if n == 0 {
            break;
        }
        total += n;
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md5_str(input: &[u8]) -> String {
        let mut h = Md5Hasher::new();
        h.update(input);
        format_md5_hex(&h.finalize())
    }

    #[test]
    fn matches_all_official_rfc1321_test_vectors() {
        assert_eq!(md5_str(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_str(b"a"), "0cc175b9c0f1b6a831c399e269772661");
        assert_eq!(md5_str(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_str(b"message digest"),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            md5_str(b"abcdefghijklmnopqrstuvwxyz"),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
        assert_eq!(
            md5_str(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
        assert_eq!(
            md5_str(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            ),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn streaming_updates_match_single_update() {
        let data: Vec<u8> = (0..10_000).map(|i| (i % 251) as u8).collect();
        let expected = md5_str(&data);

        let mut h = Md5Hasher::new();
        for chunk in data.chunks(13) {
            h.update(chunk);
        }
        assert_eq!(format_md5_hex(&h.finalize()), expected);
    }
}
