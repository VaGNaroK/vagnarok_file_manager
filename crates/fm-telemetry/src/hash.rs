use std::fmt::Write as _;
use std::io::Read;
use std::path::Path;

use compact_str::CompactString;
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::open_shared::open_shared_read;

/// Calcula o SHA-256 em streaming (chunks de 64 KB) respeitando `CancellationToken`.
///
/// Retorna `(prefixo_compacto, hash_completo_hex)`.
pub async fn compute_sha256(
    path: &Path,
    cancel: CancellationToken,
) -> Option<(CompactString, CompactString)> {
    if cancel.is_cancelled() {
        return None;
    }

    let target = path.to_path_buf();
    tokio::task::spawn_blocking(move || compute_sha256_blocking(&target, &cancel))
        .await
        .ok()?
}

fn compute_sha256_blocking(
    path: &Path,
    cancel: &CancellationToken,
) -> Option<(CompactString, CompactString)> {
    let mut file = open_shared_read(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];

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

    let digest = hasher.finalize();
    let mut full = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut full, "{byte:02x}");
    }

    let prefix = if full.len() >= 10 {
        CompactString::from(format!("{}..{}", &full[..6], &full[full.len() - 2..]))
    } else {
        CompactString::from(full.as_str())
    };

    Some((prefix, CompactString::from(full)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_known_sha256_and_compact_prefix() {
        let path = std::env::temp_dir().join(format!("fm_sha_test_{}.txt", std::process::id()));
        std::fs::write(&path, b"hello").expect("write");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        let res = rt
            .block_on(compute_sha256(&path, CancellationToken::new()))
            .expect("hash");
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            res.1,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(res.0, "2cf24d..24");
    }
}
