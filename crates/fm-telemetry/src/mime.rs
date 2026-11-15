use std::io::Read;
use std::path::Path;

use compact_str::CompactString;
use tokio_util::sync::CancellationToken;

use crate::open_shared::open_shared_read;

/// Identifica o tipo MIME real a partir dos magic bytes iniciais do arquivo.
pub async fn detect_mime_type(path: &Path, cancel: CancellationToken) -> Option<CompactString> {
    if cancel.is_cancelled() {
        return None;
    }

    let target = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        if cancel.is_cancelled() {
            return None;
        }
        let mut file = open_shared_read(&target).ok()?;
        let mut buf = [0u8; 8192];
        let n = file.read(&mut buf).ok()?;
        let slice = &buf[..n];

        if let Some(kind) = infer::get(slice) {
            return Some(CompactString::new(kind.mime_type()));
        }

        if !slice.contains(&0) && std::str::from_utf8(slice).is_ok() {
            return Some(CompactString::new("text/plain"));
        }

        Some(CompactString::new("application/octet-stream"))
    })
    .await
    .ok()?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_png_magic_bytes_and_plain_text() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");

        let png_path = std::env::temp_dir().join(format!("fm_mime_{}.bin", std::process::id()));
        std::fs::write(&png_path, b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR").expect("write png");
        let png_mime = rt.block_on(detect_mime_type(&png_path, CancellationToken::new()));
        let _ = std::fs::remove_file(&png_path);
        assert_eq!(png_mime.as_deref(), Some("image/png"));

        let txt_path = std::env::temp_dir().join(format!("fm_mime_{}.txt", std::process::id()));
        std::fs::write(&txt_path, b"fn main() {}").expect("write txt");
        let txt_mime = rt.block_on(detect_mime_type(&txt_path, CancellationToken::new()));
        let _ = std::fs::remove_file(&txt_path);
        assert_eq!(txt_mime.as_deref(), Some("text/plain"));
    }
}
