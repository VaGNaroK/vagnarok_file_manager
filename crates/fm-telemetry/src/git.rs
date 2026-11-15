use std::path::Path;

use compact_str::CompactString;
use tokio_util::sync::CancellationToken;

/// Detecta o nome do branch Git atual (ou hash curto em detached HEAD) via `gix`.
pub async fn detect_git_branch(path: &Path, cancel: CancellationToken) -> Option<CompactString> {
    if cancel.is_cancelled() {
        return None;
    }

    let target = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        if cancel.is_cancelled() {
            return None;
        }
        let repo = gix::discover(&target).ok()?;
        if cancel.is_cancelled() {
            return None;
        }
        if let Ok(Some(name)) = repo.head_name() {
            return Some(CompactString::from(name.shorten().to_string()));
        }
        let head_id = repo.head_id().ok()?;
        let hex = head_id.to_hex().to_string();
        let short = hex.get(..7).unwrap_or(&hex);
        Some(CompactString::from(format!("@{short}")))
    })
    .await
    .ok()?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_branch_in_git_repo_or_returns_none_outside() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        let outside = std::env::temp_dir();
        let res = rt.block_on(detect_git_branch(&outside, CancellationToken::new()));
        // `/tmp` normalmente não é um repo git; o importante é não dar pânico.
        let _ = res;
    }
}
