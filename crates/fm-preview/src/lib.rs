//! Pipeline de geração de thumbnails SIMD, SVG (`resvg`), cache LRU + persistente (`redb`) e QuickLook.

pub mod disk_cache;
pub mod thumbs;

pub use disk_cache::DiskThumbnailCache;
pub use thumbs::{
    DEFAULT_THUMBNAIL_CACHE_CAPACITY, ThumbnailCache, ThumbnailRgba, generate_or_load_thumbnail,
    generate_thumbnail, is_image_extension,
};

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use tokio_util::sync::CancellationToken;

/// Chave única para cache de miniaturas baseada em identidade temporal do arquivo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThumbnailKey {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified_nanos: u128,
}

/// Abre arquivo para leitura compartilhada sem bloquear locks no Windows.
pub(crate) fn open_shared_read(path: &Path) -> Option<File> {
    let mut opts = std::fs::OpenOptions::new();
    opts.read(true);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_SHARE_READ (1) | FILE_SHARE_WRITE (2) | FILE_SHARE_DELETE (4)
        opts.share_mode(0x7);
    }
    opts.open(path).ok()
}

/// Lê no máximo `max_bytes` de um arquivo de forma assíncrona e cancelável para prévia textual.
///
/// Retorna `None` quando o arquivo parece binário (contém byte NUL) ou quando ocorre erro/cancelamento.
pub async fn read_text_preview(
    path: &Path,
    max_bytes: usize,
    cancel: CancellationToken,
) -> Option<String> {
    if cancel.is_cancelled() {
        return None;
    }

    let target = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        if cancel.is_cancelled() {
            return None;
        }
        let file = open_shared_read(&target)?;
        let mut reader = file.take(max_bytes as u64);
        let mut buf = Vec::with_capacity(max_bytes.min(16 * 1024));
        reader.read_to_end(&mut buf).ok()?;

        if cancel.is_cancelled() || buf.contains(&0) {
            return None;
        }
        Some(String::from_utf8_lossy(&buf).into_owned())
    })
    .await
    .ok()?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test tokio runtime")
            .block_on(fut)
    }

    #[test]
    fn skips_binary_files_with_nul_bytes() {
        block_on(async {
            let path =
                std::env::temp_dir().join(format!("fm_preview_bin_{}.bin", std::process::id()));
            std::fs::write(&path, b"\x00\x01\x02hello").expect("write test bin");
            let res = read_text_preview(&path, 1024, CancellationToken::new()).await;
            let _ = std::fs::remove_file(&path);
            assert!(res.is_none());
        });
    }

    #[test]
    fn reads_bounded_text_preview() {
        block_on(async {
            let path =
                std::env::temp_dir().join(format!("fm_preview_txt_{}.rs", std::process::id()));
            std::fs::write(&path, b"fn main() { println!(\"hi\"); }").expect("write test rs");
            let res = read_text_preview(&path, 16, CancellationToken::new()).await;
            let _ = std::fs::remove_file(&path);
            assert_eq!(res.as_deref(), Some("fn main() { prin"));
        });
    }
}
