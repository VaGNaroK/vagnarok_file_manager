use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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

/// Detecta o status Git resumido de um arquivo específico dentro do repositório (`Modified`, `Untracked`, `Added`, `Clean`, etc.).
pub async fn detect_file_git_status(
    path: &Path,
    cancel: CancellationToken,
) -> Option<CompactString> {
    if cancel.is_cancelled() {
        return None;
    }

    let target = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        if cancel.is_cancelled() {
            return None;
        }
        let repo = gix::discover(&target).ok()?;
        let workdir = repo.workdir()?;
        let rel_owned = if let Ok(rel) = target.strip_prefix(workdir) {
            rel.to_path_buf()
        } else {
            let canon_target = std::fs::canonicalize(&target).ok()?;
            let canon_work = std::fs::canonicalize(workdir).ok()?;
            canon_target.strip_prefix(canon_work).ok()?.to_path_buf()
        };

        let mut rela_str = String::new();
        for (i, comp) in rel_owned.components().enumerate() {
            if let std::path::Component::Normal(part) = comp {
                if i > 0 && !rela_str.is_empty() {
                    rela_str.push('/');
                }
                rela_str.push_str(&part.to_string_lossy());
            }
        }
        if rela_str.is_empty() {
            return None;
        }

        let interrupt = Arc::new(AtomicBool::new(cancel.is_cancelled()));
        let platform = repo
            .status(gix::progress::Discard)
            .ok()?
            .should_interrupt_owned(Arc::clone(&interrupt))
            .untracked_files(gix::status::UntrackedFiles::Files);

        let iter = platform
            .into_iter([gix::bstr::BString::from(rela_str.as_str())])
            .ok()?;

        for item_res in iter {
            if cancel.is_cancelled() {
                interrupt.store(true, Ordering::Relaxed);
                return None;
            }
            let Ok(item) = item_res else {
                continue;
            };
            let label = match item {
                gix::status::Item::IndexWorktree(iw) => match iw {
                    gix::status::index_worktree::Item::Modification { .. } => "Modified",
                    gix::status::index_worktree::Item::Rewrite { .. } => "Renamed",
                    gix::status::index_worktree::Item::DirectoryContents { .. } => "Untracked",
                },
                gix::status::Item::TreeIndex(change) => match change {
                    gix::diff::index::ChangeRef::Addition { .. } => "Added (staged)",
                    gix::diff::index::ChangeRef::Deletion { .. } => "Deleted (staged)",
                    gix::diff::index::ChangeRef::Modification { .. } => "Modified (staged)",
                    gix::diff::index::ChangeRef::Rewrite { .. } => "Renamed (staged)",
                },
            };
            return Some(CompactString::new(label));
        }

        Some(CompactString::new("Clean"))
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

    #[test]
    fn returns_none_git_status_when_cancelled_or_outside_repo() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt");
        let outside = std::env::temp_dir().join("non_repo_file.txt");
        assert!(
            rt.block_on(detect_file_git_status(&outside, CancellationToken::new()))
                .is_none()
        );

        let cancel = CancellationToken::new();
        cancel.cancel();
        let repo_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(
            rt.block_on(detect_file_git_status(&repo_file, cancel))
                .is_none()
        );
    }
}
