//! Drivers nativos de sistema de arquivos (Linux/Windows), descoberta de volumes e engine de cópia.
//!
//! Implementações específicas de SO ficam confinadas em submódulos `#[cfg(target_os = ...)]`.

pub mod config;
pub mod copy_engine;

pub use config::{config_file_path, load_config, save_config};
pub use copy_engine::run_transfer_job;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod fallback;
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
use fallback as platform;

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use compact_str::CompactString;
use directories::{BaseDirs, UserDirs};
use fm_core::{
    DirectorySnapshot, DriveInfo, EntryKind, FileEntry, FsError, PermissionsMeta, UserPlaces,
    VfsProvider, VolumeSpace, compute_entry_id,
};
use tokio_util::sync::CancellationToken;

/// Resolve diretórios padrão do usuário de forma multiplataforma (XDG no Linux / Known Folders no Windows).
pub fn user_places() -> UserPlaces {
    let base = BaseDirs::new();
    let user = UserDirs::new();

    let home = user
        .as_ref()
        .map(|u| u.home_dir().to_path_buf())
        .or_else(|| base.as_ref().map(|b| b.home_dir().to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));

    let trash = resolve_trash_dir(&base);

    UserPlaces {
        home,
        desktop: user.as_ref().and_then(|u| u.desktop_dir().map(Path::to_path_buf)),
        documents: user
            .as_ref()
            .and_then(|u| u.document_dir().map(Path::to_path_buf)),
        downloads: user
            .as_ref()
            .and_then(|u| u.download_dir().map(Path::to_path_buf)),
        pictures: user.as_ref().and_then(|u| u.picture_dir().map(Path::to_path_buf)),
        trash,
    }
}

fn resolve_trash_dir(base: &Option<BaseDirs>) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        base.as_ref()
            .map(|b| b.data_local_dir().join("Trash").join("files"))
            .filter(|p| p.exists())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = base;
        None
    }
}

/// Implementação local de `VfsProvider` sobre o sistema de arquivos real do SO.
#[derive(Debug, Default)]
pub struct LocalFs;

impl LocalFs {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl VfsProvider for LocalFs {
    async fn list_dir(
        &self,
        path: &Path,
        request_id: u64,
        cancel: CancellationToken,
    ) -> Result<DirectorySnapshot, FsError> {
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || scan_directory_blocking(&target, request_id, &cancel))
            .await
            .map_err(|err| FsError::Io {
                path: path.to_path_buf(),
                message: format!("falha na tarefa de listagem: {err}"),
            })?
    }

    async fn list_drives(&self) -> Result<Vec<DriveInfo>, FsError> {
        tokio::task::spawn_blocking(platform::list_drives)
            .await
            .map_err(|err| FsError::Io {
                path: PathBuf::new(),
                message: format!("falha ao listar unidades: {err}"),
            })
    }

    async fn volume_space(&self, path: &Path) -> Result<Option<VolumeSpace>, FsError> {
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || platform::volume_space(&target))
            .await
            .map_err(|err| FsError::Io {
                path: path.to_path_buf(),
                message: format!("falha ao consultar espaço em disco: {err}"),
            })
    }

    async fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || std::fs::create_dir(&target).map_err(|e| map_io_err(&target, e)))
            .await
            .map_err(|err| FsError::Io {
                path: path.to_path_buf(),
                message: err.to_string(),
            })?
    }

    async fn create_file(&self, path: &Path) -> Result<(), FsError> {
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&target)
                .map(|_| ())
                .map_err(|e| map_io_err(&target, e))
        })
        .await
        .map_err(|err| FsError::Io {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), FsError> {
        let src = from.to_path_buf();
        let dst = to.to_path_buf();
        tokio::task::spawn_blocking(move || {
            std::fs::rename(&src, &dst).map_err(|e| map_io_err(&src, e))
        })
        .await
        .map_err(|err| FsError::Io {
            path: from.to_path_buf(),
            message: err.to_string(),
        })?
    }

    async fn move_to_trash(&self, paths: &[PathBuf]) -> Result<(), FsError> {
        let targets = paths.to_vec();
        if targets.is_empty() {
            return Ok(());
        }
        tokio::task::spawn_blocking(move || {
            trash::delete_all(&targets).map_err(|err| FsError::Io {
                path: targets.first().cloned().unwrap_or_default(),
                message: format!("falha ao enviar para a lixeira: {err}"),
            })
        })
        .await
        .map_err(|err| FsError::Io {
            path: PathBuf::new(),
            message: err.to_string(),
        })?
    }

    async fn delete_permanent(&self, paths: &[PathBuf]) -> Result<(), FsError> {
        let targets = paths.to_vec();
        tokio::task::spawn_blocking(move || {
            for target in &targets {
                let meta = std::fs::symlink_metadata(target).map_err(|e| map_io_err(target, e))?;
                if meta.is_dir() {
                    std::fs::remove_dir_all(target).map_err(|e| map_io_err(target, e))?;
                } else {
                    std::fs::remove_file(target).map_err(|e| map_io_err(target, e))?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|err| FsError::Io {
            path: PathBuf::new(),
            message: err.to_string(),
        })?
    }

    async fn open_default(&self, path: &Path) -> Result<(), FsError> {
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            open::that_detached(&target).map_err(|e| map_io_err(&target, e))
        })
        .await
        .map_err(|err| FsError::Io {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?
    }
}

fn scan_directory_blocking(
    path: &Path,
    request_id: u64,
    cancel: &CancellationToken,
) -> Result<DirectorySnapshot, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let read_dir = std::fs::read_dir(path).map_err(|e| map_io_err(path, e))?;
    let mut entries = Vec::new();

    for (idx, item) in read_dir.enumerate() {
        if idx.is_multiple_of(256) && cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }

        let Ok(dir_entry) = item else {
            continue;
        };

        let entry_path = dir_entry.path();
        let name_os = dir_entry.file_name();
        let name = CompactString::from(name_os.to_string_lossy().as_ref());

        // `symlink_metadata` não segue links; apenas se for symlink consultamos o alvo.
        let Ok(link_meta) = std::fs::symlink_metadata(&entry_path) else {
            continue;
        };

        let file_type = link_meta.file_type();
        let (kind, effective_meta) = if file_type.is_symlink() {
            match std::fs::metadata(&entry_path) {
                Ok(target_meta) => (
                    EntryKind::Symlink {
                        target_is_dir: target_meta.is_dir(),
                    },
                    target_meta,
                ),
                Err(_) => (
                    EntryKind::Symlink {
                        target_is_dir: false,
                    },
                    link_meta,
                ),
            }
        } else if file_type.is_dir() {
            (EntryKind::Directory, link_meta)
        } else if file_type.is_file() {
            (EntryKind::File, link_meta)
        } else {
            (EntryKind::Special, link_meta)
        };

        let extension = if kind.is_dir_like() {
            None
        } else {
            entry_path
                .extension()
                .map(|ext| CompactString::from(ext.to_string_lossy().as_ref()))
                .filter(|s| !s.is_empty())
        };

        let mime_badge = extension
            .as_ref()
            .map(|ext| ext.to_ascii_uppercase());

        let size_bytes = if kind.is_dir_like() {
            0
        } else {
            effective_meta.len()
        };
        let modified_at = effective_meta.modified().ok();
        let (permissions, is_hidden): (PermissionsMeta, bool) =
            platform::extract_permissions_and_hidden(&name, &effective_meta);

        entries.push(FileEntry {
            id: compute_entry_id(&entry_path),
            name,
            path: entry_path,
            extension,
            kind,
            size_bytes,
            modified_at,
            is_hidden,
            permissions,
            child_count: None,
            mime_badge,
        });
    }

    let fs_type = platform::fs_type_for_path(path);

    Ok(DirectorySnapshot {
        request_id,
        path: path.to_path_buf(),
        entries,
        fs_type,
    })
}

fn map_io_err(path: &Path, err: std::io::Error) -> FsError {
    match err.kind() {
        ErrorKind::NotFound => FsError::NotFound(path.to_path_buf()),
        ErrorKind::PermissionDenied => FsError::PermissionDenied(path.to_path_buf()),
        _ => FsError::Io {
            path: path.to_path_buf(),
            message: err.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(1);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "vagnarok_fm_test_{}_{}",
                std::process::id(),
                n
            ));
            std::fs::create_dir_all(&path).expect("create temp test dir");
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test tokio runtime")
            .block_on(fut)
    }

    #[test]
    fn lists_real_directory_entries_and_respects_hidden_flag() {
        block_on(async {
            let tmp = TestDir::new();
            std::fs::create_dir(tmp.0.join("src")).expect("mkdir src");
            std::fs::write(tmp.0.join("main.rs"), b"fn main() {}").expect("write main.rs");
            std::fs::write(tmp.0.join(".env"), b"SECRET=1").expect("write .env");

            let fs = LocalFs::new();
            let snap = fs
                .list_dir(&tmp.0, 42, CancellationToken::new())
                .await
                .expect("list_dir ok");

            assert_eq!(snap.request_id, 42);
            assert_eq!(snap.entries.len(), 3);

            let main_entry = snap
                .entries
                .iter()
                .find(|e| e.name == "main.rs")
                .expect("main.rs present");
            assert_eq!(main_entry.extension.as_deref(), Some("rs"));
            assert_eq!(main_entry.size_bytes, 12);
            assert!(!main_entry.is_hidden);

            let hidden_entry = snap
                .entries
                .iter()
                .find(|e| e.name == ".env")
                .expect(".env present");
            assert!(hidden_entry.is_hidden);
        });
    }

    #[test]
    fn returns_cancelled_when_token_is_already_triggered() {
        block_on(async {
            let tmp = TestDir::new();
            let fs = LocalFs::new();
            let cancel = CancellationToken::new();
            cancel.cancel();

            let res = fs.list_dir(&tmp.0, 1, cancel).await;
            assert!(matches!(res, Err(FsError::Cancelled)));
        });
    }

    #[test]
    fn returns_not_found_for_missing_directory() {
        block_on(async {
            let fs = LocalFs::new();
            let missing = PathBuf::from("/path/that/definitely/does/not/exist_vagnarok_999");
            let res = fs.list_dir(&missing, 1, CancellationToken::new()).await;
            assert!(matches!(res, Err(FsError::NotFound(_))));
        });
    }

    #[test]
    fn discovers_user_places_and_volume_space() {
        block_on(async {
            let places = user_places();
            assert!(places.home.exists());

            let fs = LocalFs::new();
            let drives = fs.list_drives().await.expect("list_drives ok");
            #[cfg(target_os = "linux")]
            assert!(!drives.is_empty());
            #[cfg(not(target_os = "linux"))]
            let _ = drives;

            let space = fs.volume_space(&places.home).await.expect("volume_space ok");
            #[cfg(target_os = "linux")]
            assert!(space.is_some());
            #[cfg(not(target_os = "linux"))]
            let _ = space;
        });
    }
}
