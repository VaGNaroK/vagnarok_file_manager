//! Drivers nativos de sistema de arquivos (Linux/Windows), descoberta de volumes e engine de cópia.
//!
//! Implementações específicas de SO ficam confinadas em submódulos `#[cfg(target_os = ...)]`.

pub mod archive;
pub mod config;
pub mod copy_engine;
pub mod icons;
mod trash_bin;

pub use archive::is_archive_file;
pub use config::{config_file_path, load_config, save_config};
pub use copy_engine::run_transfer_job;
pub use icons::resolve_system_icon;

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
    DirectorySnapshot, DriveInfo, EntryExtraInfo, EntryKind, FileEntry, FsError, PermissionsMeta,
    UserPlaces, VfsProvider, VolumeSpace, compute_entry_id, is_virtual_trash_path,
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

    let trash = trash_bin::resolve_trash_dir();

    UserPlaces {
        home,
        desktop: user
            .as_ref()
            .and_then(|u| u.desktop_dir().map(Path::to_path_buf))
            .filter(|p| p.exists()),
        documents: user
            .as_ref()
            .and_then(|u| u.document_dir().map(Path::to_path_buf))
            .filter(|p| p.exists()),
        downloads: user
            .as_ref()
            .and_then(|u| u.download_dir().map(Path::to_path_buf))
            .filter(|p| p.exists()),
        music: user
            .as_ref()
            .and_then(|u| u.audio_dir().map(Path::to_path_buf))
            .filter(|p| p.exists()),
        pictures: user
            .as_ref()
            .and_then(|u| u.picture_dir().map(Path::to_path_buf))
            .filter(|p| p.exists()),
        videos: user
            .as_ref()
            .and_then(|u| u.video_dir().map(Path::to_path_buf))
            .filter(|p| p.exists()),
        public_dir: user
            .as_ref()
            .and_then(|u| u.public_dir().map(Path::to_path_buf))
            .filter(|p| p.exists()),
        templates: user
            .as_ref()
            .and_then(|u| u.template_dir().map(Path::to_path_buf))
            .filter(|p| p.exists()),
        trash,
    }
}

/// Lista subpastas não ocultas diretamente dentro do diretório pessoal (`home`) para a barra lateral.
pub async fn list_home_subdirs(home: PathBuf) -> Vec<PathBuf> {
    tokio::task::spawn_blocking(move || {
        let Ok(iter) = std::fs::read_dir(&home) else {
            return Vec::new();
        };
        let mut dirs: Vec<PathBuf> = iter
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name();
                let name_lossy = name.to_string_lossy();
                if name_lossy.starts_with('.') {
                    return None;
                }
                let is_dir = entry
                    .file_type()
                    .map(|ft| ft.is_dir() || (ft.is_symlink() && entry.path().is_dir()))
                    .unwrap_or(false);
                is_dir.then(|| entry.path())
            })
            .collect();
        dirs.sort_by(|a, b| {
            let na = a
                .file_name()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let nb = b
                .file_name()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            na.cmp(&nb)
        });
        dirs
    })
    .await
    .unwrap_or_default()
}

/// Monta uma unidade/partição em background e retorna o caminho de montagem resultante.
pub async fn mount_drive(device_path: PathBuf) -> Result<PathBuf, FsError> {
    let err_path = device_path.clone();
    tokio::task::spawn_blocking(move || platform::mount_drive(&device_path))
        .await
        .map_err(|err| FsError::Io {
            path: err_path,
            message: err.to_string(),
        })?
}

/// Desmonta ou ejeta uma unidade/ponto de montagem em background.
pub async fn unmount_drive(
    device_path: Option<PathBuf>,
    mount_path: Option<PathBuf>,
) -> Result<(), FsError> {
    let err_path = mount_path
        .clone()
        .or_else(|| device_path.clone())
        .unwrap_or_default();
    tokio::task::spawn_blocking(move || {
        platform::unmount_drive(device_path.as_deref(), mount_path.as_deref())
    })
    .await
    .map_err(|err| FsError::Io {
        path: err_path,
        message: err.to_string(),
    })?
}

/// Indica se a plataforma atual suporta a ação "Abrir como root" no menu de contexto.
pub fn supports_open_as_root() -> bool {
    platform::supports_open_as_root()
}

/// Indica se o processo atual está executando com privilégios elevados (UID 0 / root no Linux).
pub fn is_elevated_session() -> bool {
    platform::is_elevated_session()
}

/// Abre uma nova instância elevada do gerenciador de arquivos apontando para `path` (`pkexec` no Linux / `runas` no Windows).
pub async fn open_as_root(path: PathBuf) -> Result<(), FsError> {
    let err_path = path.clone();
    tokio::task::spawn_blocking(move || platform::open_as_root(&path))
        .await
        .map_err(|err| FsError::Io {
            path: err_path,
            message: err.to_string(),
        })?
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
        tokio::task::spawn_blocking(move || {
            let started = std::time::Instant::now();
            let result = if is_virtual_trash_path(&target) {
                trash_bin::list_virtual_trash(request_id, &cancel)
            } else if let Some((archive_path, inner_prefix, kind)) =
                archive::split_archive_path(&target)
            {
                archive::list_archive_dir(&archive_path, &inner_prefix, kind, request_id, &cancel)
            } else {
                platform::scan_directory_native(&target, request_id, &cancel)
            };
            if let Ok(snap) = &result {
                tracing::debug!(
                    path = %target.display(),
                    entries = snap.entries.len(),
                    elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
                    "list_dir_completed"
                );
            }
            result
        })
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
        if is_virtual_trash_path(path) {
            return Ok(None);
        }
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || platform::volume_space(&target))
            .await
            .map_err(|err| FsError::Io {
                path: path.to_path_buf(),
                message: format!("falha ao consultar espaço em disco: {err}"),
            })
    }

    async fn create_dir(&self, path: &Path) -> Result<(), FsError> {
        if archive::split_archive_path(path).is_some() {
            return Err(FsError::Io {
                path: path.to_path_buf(),
                message: "arquivos compactados são somente leitura".to_owned(),
            });
        }
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir(&target).map_err(|e| map_io_err(&target, e))
        })
        .await
        .map_err(|err| FsError::Io {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?
    }

    async fn create_file(&self, path: &Path) -> Result<(), FsError> {
        if archive::split_archive_path(path).is_some() {
            return Err(FsError::Io {
                path: path.to_path_buf(),
                message: "arquivos compactados são somente leitura".to_owned(),
            });
        }
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
        if archive::split_archive_path(from).is_some() || archive::split_archive_path(to).is_some()
        {
            return Err(FsError::Io {
                path: from.to_path_buf(),
                message: "arquivos compactados são somente leitura".to_owned(),
            });
        }
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
        if let Some(arch) = paths
            .iter()
            .find(|p| archive::split_archive_path(p).is_some())
        {
            return Err(FsError::Io {
                path: arch.clone(),
                message: "arquivos compactados são somente leitura".to_owned(),
            });
        }
        let targets = paths.to_vec();
        if targets.is_empty() {
            return Ok(());
        }
        tokio::task::spawn_blocking(move || trash_bin::move_to_trash(&targets))
            .await
            .map_err(|err| FsError::Io {
                path: PathBuf::new(),
                message: err.to_string(),
            })?
    }

    async fn restore_from_trash(&self, paths: &[PathBuf]) -> Result<usize, FsError> {
        let targets = paths.to_vec();
        if targets.is_empty() {
            return Ok(0);
        }
        tokio::task::spawn_blocking(move || trash_bin::restore_trash_items(&targets))
            .await
            .map_err(|err| FsError::Io {
                path: PathBuf::new(),
                message: err.to_string(),
            })?
    }

    async fn empty_trash(&self) -> Result<(), FsError> {
        tokio::task::spawn_blocking(trash_bin::purge_all_trash_items)
            .await
            .map_err(|err| FsError::Io {
                path: PathBuf::new(),
                message: err.to_string(),
            })?
    }

    async fn delete_permanent(&self, paths: &[PathBuf]) -> Result<(), FsError> {
        if let Some(arch) = paths
            .iter()
            .find(|p| archive::split_archive_path(p).is_some())
        {
            return Err(FsError::Io {
                path: arch.clone(),
                message: "arquivos compactados são somente leitura".to_owned(),
            });
        }
        let targets = paths.to_vec();
        tokio::task::spawn_blocking(move || {
            let remaining = trash_bin::purge_matching_trash_items(&targets)?;
            for target in &remaining {
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
        tokio::task::spawn_blocking(move || platform::open_default(&target))
            .await
            .map_err(|err| FsError::Io {
                path: path.to_path_buf(),
                message: err.to_string(),
            })?
    }

    async fn open_terminal(&self, path: &Path) -> Result<(), FsError> {
        let target = if let Some((archive_path, _, _)) = archive::split_archive_path(path) {
            archive_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or(archive_path)
        } else {
            path.to_path_buf()
        };
        let fallback = path.to_path_buf();
        tokio::task::spawn_blocking(move || platform::open_terminal_at(&target))
            .await
            .map_err(|err| FsError::Io {
                path: fallback,
                message: err.to_string(),
            })?
    }

    async fn open_properties(&self, path: &Path) -> Result<(), FsError> {
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || platform::open_properties(&target))
            .await
            .map_err(|err| FsError::Io {
                path: path.to_path_buf(),
                message: err.to_string(),
            })?
    }

    async fn read_bytes(
        &self,
        path: &Path,
        max_bytes: usize,
        cancel: CancellationToken,
    ) -> Result<Vec<u8>, FsError> {
        let target = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            if cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            if let Some((archive_path, inner_path, kind)) = archive::split_archive_path(&target) {
                return archive::read_archive_bytes(
                    &archive_path,
                    &inner_path,
                    kind,
                    max_bytes,
                    &cancel,
                );
            }
            use std::io::Read;
            let mut opts = std::fs::OpenOptions::new();
            opts.read(true);
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::fs::OpenOptionsExt;
                opts.share_mode(0x7);
            }
            let file = opts.open(&target).map_err(|e| map_io_err(&target, e))?;
            let mut out = Vec::with_capacity(max_bytes.min(64 * 1024));
            file.take(max_bytes as u64)
                .read_to_end(&mut out)
                .map_err(|e| map_io_err(&target, e))?;
            Ok(out)
        })
        .await
        .map_err(|err| FsError::Io {
            path: path.to_path_buf(),
            message: err.to_string(),
        })?
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn make_file_entry(
    name: CompactString,
    entry_path: PathBuf,
    kind: EntryKind,
    size_bytes: u64,
    modified_at: Option<std::time::SystemTime>,
    created_at: Option<std::time::SystemTime>,
    permissions: PermissionsMeta,
    is_hidden: bool,
) -> FileEntry {
    let extension = if kind.is_dir_like() {
        None
    } else {
        entry_path
            .extension()
            .map(|ext| CompactString::from(ext.to_string_lossy().as_ref()))
            .filter(|s| !s.is_empty())
    };

    let mime_badge = extension.as_ref().map(|ext| ext.to_ascii_uppercase());

    FileEntry {
        id: compute_entry_id(&entry_path),
        name,
        path: entry_path,
        extension,
        kind,
        size_bytes: if kind.is_dir_like() { 0 } else { size_bytes },
        modified_at,
        created_at,
        is_hidden,
        permissions,
        child_count: None,
        mime_badge,
    }
}

#[allow(dead_code)]
pub(crate) fn scan_directory_blocking(
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

        let size_bytes = effective_meta.len();
        let modified_at = effective_meta.modified().ok();
        let created_at = effective_meta.created().ok();
        let (permissions, is_hidden): (PermissionsMeta, bool) =
            platform::extract_permissions_and_hidden(&name, &effective_meta);

        entries.push(make_file_entry(
            name,
            entry_path,
            kind,
            size_bytes,
            modified_at,
            created_at,
            permissions,
            is_hidden,
        ));
    }

    let fs_type = platform::fs_type_for_path(path);

    Ok(DirectorySnapshot {
        request_id,
        path: path.to_path_buf(),
        entries,
        fs_type,
    })
}

pub(crate) fn default_permissions_meta() -> PermissionsMeta {
    #[cfg(target_os = "windows")]
    {
        PermissionsMeta::WinAttributes(0)
    }
    #[cfg(not(target_os = "windows"))]
    {
        PermissionsMeta::PosixMode(0)
    }
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

/// Alterações aplicáveis em metadados/permissões pela janela de propriedades.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PropertyPatch {
    pub readonly: Option<bool>,
    pub hidden: Option<bool>,
    pub posix_mode: Option<u32>,
}

/// Lê metadados adicionais sob demanda (último acesso, alvo de symlink, estado somente leitura).
pub async fn read_entry_extra_info(path: PathBuf) -> EntryExtraInfo {
    tokio::task::spawn_blocking(move || {
        let Ok(link_meta) = std::fs::symlink_metadata(&path) else {
            return EntryExtraInfo::default();
        };
        let symlink_target = if link_meta.file_type().is_symlink() {
            std::fs::read_link(&path).ok()
        } else {
            None
        };
        let effective = std::fs::metadata(&path).unwrap_or(link_meta);
        EntryExtraInfo {
            accessed_at: effective.accessed().ok(),
            symlink_target,
            is_readonly: effective.permissions().readonly(),
        }
    })
    .await
    .unwrap_or_default()
}

/// Aplica alterações de permissões / atributos no item de forma assíncrona.
pub async fn apply_entry_properties(path: PathBuf, patch: PropertyPatch) -> Result<(), FsError> {
    tokio::task::spawn_blocking(move || platform::apply_entry_properties(&path, &patch))
        .await
        .map_err(|err| FsError::Io {
            path: PathBuf::new(),
            message: err.to_string(),
        })?
}

/// Consulta metadados atualizados de um único arquivo/diretório no sistema local.
pub async fn stat_entry(path: PathBuf) -> Result<FileEntry, FsError> {
    let err_path = path.clone();
    tokio::task::spawn_blocking(move || stat_entry_blocking(&path))
        .await
        .map_err(|err| FsError::Io {
            path: err_path,
            message: err.to_string(),
        })?
}

fn stat_entry_blocking(entry_path: &Path) -> Result<FileEntry, FsError> {
    if is_virtual_trash_path(entry_path) {
        return Ok(make_file_entry(
            CompactString::from("Trash"),
            entry_path.to_path_buf(),
            EntryKind::Directory,
            0,
            None,
            None,
            default_permissions_meta(),
            false,
        ));
    }
    let link_meta = std::fs::symlink_metadata(entry_path).map_err(|e| map_io_err(entry_path, e))?;
    let name = entry_path
        .file_name()
        .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
        .unwrap_or_else(|| CompactString::from(entry_path.to_string_lossy().as_ref()));
    let file_type = link_meta.file_type();
    let (kind, effective_meta) = if file_type.is_symlink() {
        match std::fs::metadata(entry_path) {
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
    let size_bytes = effective_meta.len();
    let modified_at = effective_meta.modified().ok();
    let created_at = effective_meta.created().ok();
    let (permissions, is_hidden) = platform::extract_permissions_and_hidden(&name, &effective_meta);
    Ok(make_file_entry(
        name,
        entry_path.to_path_buf(),
        kind,
        size_bytes,
        modified_at,
        created_at,
        permissions,
        is_hidden,
    ))
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
            let path =
                std::env::temp_dir().join(format!("vagnarok_fm_test_{}_{}", std::process::id(), n));
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
            let env_path = tmp.0.join(".env");
            std::fs::write(&env_path, b"SECRET=1").expect("write .env");
            #[cfg(target_os = "windows")]
            apply_entry_properties(
                env_path,
                PropertyPatch {
                    readonly: None,
                    hidden: Some(true),
                    posix_mode: None,
                },
            )
            .await
            .expect("set FILE_ATTRIBUTE_HIDDEN on .env");

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
            assert!(main_entry.modified_at.is_some());
            let std_btime_supported = std::fs::metadata(tmp.0.join("main.rs"))
                .ok()
                .and_then(|m| m.created().ok())
                .is_some();
            if std_btime_supported {
                assert!(main_entry.created_at.is_some());
            }

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

            let space = fs
                .volume_space(&places.home)
                .await
                .expect("volume_space ok");
            #[cfg(target_os = "linux")]
            assert!(space.is_some());
            #[cfg(not(target_os = "linux"))]
            let _ = space;
        });
    }

    #[test]
    fn reads_extra_info_and_updates_readonly_property() {
        block_on(async {
            let tmp = TestDir::new();
            let file = tmp.0.join("sample.txt");
            std::fs::write(&file, b"hello").expect("write file");

            let extra = read_entry_extra_info(file.clone()).await;
            assert!(extra.symlink_target.is_none());

            apply_entry_properties(
                file.clone(),
                PropertyPatch {
                    readonly: Some(true),
                    hidden: None,
                    posix_mode: None,
                },
            )
            .await
            .expect("set readonly");
            let updated_extra = read_entry_extra_info(file.clone()).await;
            assert!(updated_extra.is_readonly);

            apply_entry_properties(
                file.clone(),
                PropertyPatch {
                    readonly: Some(false),
                    hidden: None,
                    posix_mode: None,
                },
            )
            .await
            .expect("clear readonly");
            let cleared_extra = read_entry_extra_info(file.clone()).await;
            assert!(!cleared_extra.is_readonly);

            let entry = stat_entry(file).await.expect("stat_entry");
            assert_eq!(entry.name, "sample.txt");
            assert_eq!(entry.size_bytes, 5);
        });
    }

    #[test]
    fn resolves_trash_place_and_virtual_trash_listing() {
        let places = user_places();
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        {
            assert_eq!(
                places.trash,
                Some(PathBuf::from(fm_core::VIRTUAL_TRASH_PATH))
            );
            assert!(is_virtual_trash_path(places.trash.as_deref().unwrap()));
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let _ = places;
    }
}
