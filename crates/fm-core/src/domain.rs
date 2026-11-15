use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use compact_str::CompactString;
use serde::{Deserialize, Serialize};

/// Identificador estável de uma entrada no snapshot de diretório.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EntryId(pub u64);

/// Calcula um identificador estável (FNV-1a 64-bit) a partir dos bytes reais do caminho.
pub fn compute_entry_id(path: &Path) -> EntryId {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in path.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    EntryId(hash)
}

/// Diretórios padrão do usuário resolvidos de forma nativa (XDG no Linux / Known Folders no Windows).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserPlaces {
    pub home: PathBuf,
    pub desktop: Option<PathBuf>,
    pub documents: Option<PathBuf>,
    pub downloads: Option<PathBuf>,
    pub pictures: Option<PathBuf>,
    pub trash: Option<PathBuf>,
}

/// Tipo lógico de entrada no sistema de arquivos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    Directory,
    File,
    Symlink { target_is_dir: bool },
    Special,
}

impl EntryKind {
    /// Retorna `true` quando a entrada deve se comportar como pasta na navegação e ordenação.
    pub fn is_dir_like(self) -> bool {
        matches!(
            self,
            Self::Directory
                | Self::Symlink {
                    target_is_dir: true
                }
        )
    }
}

/// Metadados de permissão ou atributos específicos da plataforma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionsMeta {
    PosixMode(u32),
    WinAttributes(u32),
}

impl PermissionsMeta {
    /// Formata permissões POSIX em estilo `drwxr-xr-x` ou atributos Windows em `RHS`.
    pub fn format_compact(self, is_dir: bool) -> CompactString {
        match self {
            Self::PosixMode(mode) => {
                let mut buf = [b'-'; 10];
                buf[0] = if is_dir { b'd' } else { b'-' };
                let bits = [
                    (0o400, b'r'),
                    (0o200, b'w'),
                    (0o100, b'x'),
                    (0o040, b'r'),
                    (0o020, b'w'),
                    (0o010, b'x'),
                    (0o004, b'r'),
                    (0o002, b'w'),
                    (0o001, b'x'),
                ];
                for (idx, (mask, ch)) in bits.iter().enumerate() {
                    if mode & mask != 0 {
                        buf[idx + 1] = *ch;
                    }
                }
                let s = std::str::from_utf8(&buf).unwrap_or("----------");
                CompactString::new(s)
            }
            Self::WinAttributes(attrs) => {
                // FILE_ATTRIBUTE_READONLY=0x1, HIDDEN=0x2, SYSTEM=0x4, ARCHIVE=0x20
                let mut out = CompactString::default();
                out.push(if is_dir { 'd' } else { '-' });
                out.push(if attrs & 0x1 != 0 { 'r' } else { 'w' });
                out.push(if attrs & 0x2 != 0 { 'h' } else { '-' });
                out.push(if attrs & 0x4 != 0 { 's' } else { '-' });
                out.push(if attrs & 0x20 != 0 { 'a' } else { '-' });
                out
            }
        }
    }
}

/// Entrada individual de arquivo ou pasta em um diretório.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub id: EntryId,
    pub name: CompactString,
    pub path: PathBuf,
    pub extension: Option<CompactString>,
    pub kind: EntryKind,
    pub size_bytes: u64,
    pub modified_at: Option<SystemTime>,
    pub is_hidden: bool,
    pub permissions: PermissionsMeta,
    pub child_count: Option<u32>,
    pub mime_badge: Option<CompactString>,
}

/// Snapshot rápido retornado pela listagem inicial de diretório.
#[derive(Debug, Clone)]
pub struct DirectorySnapshot {
    pub request_id: u64,
    pub path: PathBuf,
    pub entries: Vec<FileEntry>,
    pub fs_type: Option<CompactString>,
}

/// Resumo quantitativo de um diretório.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FolderSummary {
    pub dir_count: usize,
    pub file_count: usize,
    pub total_direct_bytes: u64,
}

impl FolderSummary {
    /// Calcula resumo direto a partir de um slice de entradas já carregadas.
    pub fn from_entries(entries: &[FileEntry]) -> Self {
        let mut summary = Self::default();
        for entry in entries {
            if entry.kind.is_dir_like() {
                summary.dir_count += 1;
            } else {
                summary.file_count += 1;
                summary.total_direct_bytes =
                    summary.total_direct_bytes.saturating_add(entry.size_bytes);
            }
        }
        summary
    }
}

/// Espaço em disco de um volume montado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VolumeSpace {
    pub label: CompactString,
    pub free_bytes: u64,
    pub total_bytes: u64,
}

/// Telemetria assíncrona do diretório ativo.
#[derive(Debug, Clone, Default)]
pub struct DirectoryTelemetry {
    pub git_branch: Option<CompactString>,
    pub folder_stats: FolderSummary,
    pub volume_space: Option<VolumeSpace>,
}

/// Telemetria computada sob demanda para o item selecionado.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectedFileTelemetry {
    pub sha256_prefix: Option<CompactString>,
    pub sha256_full: Option<CompactString>,
    pub mime_type: Option<CompactString>,
    pub preview_snippet: Option<String>,
}

/// Tipo de dispositivo/volume exibido na barra lateral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveKind {
    InternalSsd,
    SecondaryDisk,
    Removable,
    Network,
}

/// Dispositivo ou ponto de montagem descoberto no sistema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveInfo {
    pub label: CompactString,
    pub mount_path: PathBuf,
    pub fs_type: Option<CompactString>,
    pub kind: DriveKind,
    pub space: Option<VolumeSpace>,
}

/// Modo de visualização do canvas central.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ViewMode {
    #[default]
    Grid,
    DetailTable,
    Columns,
}

/// Operação de transferência na área de transferência ou fila do `CopyEngine`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMode {
    Copy,
    Move,
}

/// Política para tratamento de conflitos de nome durante cópia/movimentação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConflictPolicy {
    #[default]
    RenameAuto,
    Overwrite,
    Skip,
}

/// Estado atual de um job em andamento no `CopyEngine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Running,
    Completed,
    Cancelled,
    Failed(String),
}

/// Progresso emitido em streaming para operações de cópia/movimentação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobProgress {
    pub job_id: u64,
    pub mode: CopyMode,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub items_done: usize,
    pub items_total: usize,
    pub current_item: CompactString,
    pub state: JobState,
}

/// Itens armazenados para Colar (`Ctrl+C` / `Ctrl+X`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardTransfer {
    pub mode: CopyMode,
    pub paths: Vec<PathBuf>,
}

/// Campo de ordenação de entradas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortField {
    #[default]
    Name,
    Size,
    Modified,
    Kind,
}

/// Direção de ordenação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub fn toggled(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

/// Especificação completa de ordenação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortSpec {
    pub field: SortField,
    pub direction: SortDirection,
    pub dirs_first: bool,
}

impl Default for SortSpec {
    fn default() -> Self {
        Self {
            field: SortField::Name,
            direction: SortDirection::Ascending,
            dirs_first: true,
        }
    }
}

impl SortSpec {
    /// Compara duas entradas respeitando `dirs_first` e direção configurada.
    pub fn compare(&self, a: &FileEntry, b: &FileEntry) -> Ordering {
        if self.dirs_first {
            match (a.kind.is_dir_like(), b.kind.is_dir_like()) {
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                _ => {}
            }
        }

        let ord = match self.field {
            SortField::Name => a
                .name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
                .then_with(|| a.name.cmp(&b.name)),
            SortField::Size => a
                .size_bytes
                .cmp(&b.size_bytes)
                .then_with(|| a.name.cmp(&b.name)),
            SortField::Modified => a
                .modified_at
                .cmp(&b.modified_at)
                .then_with(|| a.name.cmp(&b.name)),
            SortField::Kind => a
                .extension
                .as_deref()
                .unwrap_or("")
                .cmp(b.extension.as_deref().unwrap_or(""))
                .then_with(|| a.name.cmp(&b.name)),
        };

        match self.direction {
            SortDirection::Ascending => ord,
            SortDirection::Descending => ord.reverse(),
        }
    }
}

/// Formata contagem de bytes em unidade legível (`842 B`, `8.9 KB`, `438.2 MB`).
pub fn format_bytes(bytes: u64) -> CompactString {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    const TB: f64 = GB * 1024.0;

    let b = bytes as f64;
    if bytes < 1024 {
        CompactString::from(format!("{bytes} B"))
    } else if b < MB {
        CompactString::from(format!("{:.1} KB", b / KB))
    } else if b < GB {
        CompactString::from(format!("{:.1} MB", b / MB))
    } else if b < TB {
        CompactString::from(format!("{:.1} GB", b / GB))
    } else {
        CompactString::from(format!("{:.1} TB", b / TB))
    }
}

/// Formata idade relativa de modificação de forma compacta (`agora`, `14m`, `3h`, `5d`).
pub fn format_age(modified: SystemTime, pt_br: bool) -> CompactString {
    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return CompactString::new(if pt_br { "agora" } else { "now" });
    };
    let secs = elapsed.as_secs();
    if secs < 60 {
        CompactString::new(if pt_br { "agora" } else { "now" })
    } else if secs < 3600 {
        let m = secs / 60;
        CompactString::from(if pt_br {
            format!("há {m}m")
        } else {
            format!("{m}m ago")
        })
    } else if secs < 86_400 {
        let h = secs / 3600;
        CompactString::from(if pt_br {
            format!("há {h}h")
        } else {
            format!("{h}h ago")
        })
    } else {
        let d = secs / 86_400;
        CompactString::from(if pt_br {
            format!("há {d}d")
        } else {
            format!("{d}d ago")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(name: &str, is_dir: bool, size: u64) -> FileEntry {
        FileEntry {
            id: EntryId(size + name.len() as u64),
            name: CompactString::new(name),
            path: PathBuf::from(name),
            extension: None,
            kind: if is_dir {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            size_bytes: size,
            modified_at: None,
            is_hidden: false,
            permissions: PermissionsMeta::PosixMode(0o755),
            child_count: None,
            mime_badge: None,
        }
    }

    #[test]
    fn formats_posix_permissions() {
        assert_eq!(
            PermissionsMeta::PosixMode(0o755).format_compact(true),
            "drwxr-xr-x"
        );
        assert_eq!(
            PermissionsMeta::PosixMode(0o644).format_compact(false),
            "-rw-r--r--"
        );
    }

    #[test]
    fn sorts_directories_first_then_case_insensitive_name() {
        let mut items = [
            sample_entry("zeta.rs", false, 10),
            sample_entry("Alpha", true, 0),
            sample_entry("beta.txt", false, 20),
        ];
        let spec = SortSpec::default();
        items.sort_by(|a, b| spec.compare(a, b));
        let names: Vec<_> = items.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "beta.txt", "zeta.rs"]);
    }

    #[test]
    fn formats_human_sizes() {
        assert_eq!(format_bytes(842), "842 B");
        assert_eq!(format_bytes(9114), "8.9 KB");
    }
}
