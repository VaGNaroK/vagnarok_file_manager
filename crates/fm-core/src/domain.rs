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

/// Caminho virtual usado para representar a Lixeira do SO quando não há diretório único navegável (ex.: Windows Recycle Bin).
pub const VIRTUAL_TRASH_PATH: &str = "trash://";

/// Indica se `path` aponta para a raiz virtual da Lixeira.
pub fn is_virtual_trash_path(path: &Path) -> bool {
    path == Path::new(VIRTUAL_TRASH_PATH)
}

/// Diretórios padrão do usuário resolvidos de forma nativa (XDG no Linux / Known Folders no Windows).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserPlaces {
    pub home: PathBuf,
    pub desktop: Option<PathBuf>,
    pub documents: Option<PathBuf>,
    pub downloads: Option<PathBuf>,
    pub music: Option<PathBuf>,
    pub pictures: Option<PathBuf>,
    pub videos: Option<PathBuf>,
    pub public_dir: Option<PathBuf>,
    pub templates: Option<PathBuf>,
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
    pub created_at: Option<SystemTime>,
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

/// Tamanho recursivo computado em background (com indicação se foi truncado por limite).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecursiveSize {
    pub total_bytes: u64,
    pub item_count: u64,
    pub hidden_count: u64,
    pub truncated: bool,
}

/// Telemetria assíncrona do diretório ativo.
#[derive(Debug, Clone, Default)]
pub struct DirectoryTelemetry {
    pub git_branch: Option<CompactString>,
    pub folder_stats: FolderSummary,
    pub recursive_size: Option<RecursiveSize>,
    pub volume_space: Option<VolumeSpace>,
}

/// Telemetria computada sob demanda para o item selecionado.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectedFileTelemetry {
    pub sha256_prefix: Option<CompactString>,
    pub sha256_full: Option<CompactString>,
    pub mime_type: Option<CompactString>,
    pub git_status: Option<CompactString>,
    pub preview_snippet: Option<String>,
}

/// Metadados adicionais sob demanda exibidos na janela de propriedades.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryExtraInfo {
    pub accessed_at: Option<SystemTime>,
    pub symlink_target: Option<PathBuf>,
    pub is_readonly: bool,
}

/// Tipo de dispositivo/volume exibido na barra lateral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DriveKind {
    InternalSsd,
    SecondaryDisk,
    Removable,
    OpticalOrIso,
    Network,
}

/// Dispositivo ou ponto de montagem descoberto no sistema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveInfo {
    pub label: CompactString,
    pub mount_path: PathBuf,
    pub device_path: Option<PathBuf>,
    pub model: Option<CompactString>,
    pub fs_type: Option<CompactString>,
    pub kind: DriveKind,
    pub is_mounted: bool,
    pub can_unmount: bool,
    pub is_ejectable: bool,
    pub is_disk_image: bool,
    pub total_bytes_hint: Option<u64>,
    pub space: Option<VolumeSpace>,
}

/// Ordena unidades para exibição na barra lateral:
/// 1) Sistema principal (`InternalSsd` montado, ex.: `Root (/)` / `C:`),
/// 2) Mídias removíveis e imagens `.iso`/`.img` montadas,
/// 3) Demais volumes montados,
/// 4) Volumes desmontados (removíveis antes de internos, por nome e dispositivo).
pub fn sort_drives_for_display(drives: &mut [DriveInfo]) {
    drives.sort_by(|a, b| {
        let rank = |d: &DriveInfo| -> u8 {
            if d.is_mounted {
                match d.kind {
                    DriveKind::InternalSsd => 0,
                    DriveKind::OpticalOrIso => 1,
                    DriveKind::Removable => 2,
                    DriveKind::SecondaryDisk => 3,
                    DriveKind::Network => 4,
                }
            } else {
                match d.kind {
                    DriveKind::OpticalOrIso => 5,
                    DriveKind::Removable => 6,
                    _ => 7,
                }
            }
        };
        rank(a)
            .cmp(&rank(b))
            .then_with(|| {
                a.label
                    .to_ascii_lowercase()
                    .cmp(&b.label.to_ascii_lowercase())
            })
            .then_with(|| a.device_path.cmp(&b.device_path))
            .then_with(|| a.mount_path.cmp(&b.mount_path))
    });
}

/// Extrai o nome curto do dispositivo de bloco (ex.: `/dev/sdc1` -> `Some("sdc1")`).
#[must_use]
pub fn drive_short_device_name(drive: &DriveInfo) -> Option<&str> {
    drive
        .device_path
        .as_deref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
}

/// Normaliza o rótulo de uma unidade para detectar nomes iguais ou quase iguais
/// (ex.: `"JOGOS"` vs `"Jogos"`, `"Download"` vs `"Downloads"`).
fn normalize_drive_label_for_collision(label: &str) -> String {
    let trimmed = label.trim().to_ascii_lowercase();
    let trimmed = trimmed.strip_suffix('s').unwrap_or(&trimmed);
    trimmed.to_owned()
}

/// Retorna `true` quando existe outra unidade na lista com rótulo igual ou quase igual,
/// indicando que convém exibir um marcador discreto do dispositivo (`sdc1` / `sdd2`).
#[must_use]
pub fn drive_needs_disambiguation(drive: &DriveInfo, all_drives: &[DriveInfo]) -> bool {
    let key = normalize_drive_label_for_collision(&drive.label);
    if key.is_empty() {
        return false;
    }
    let matches = all_drives
        .iter()
        .filter(|other| normalize_drive_label_for_collision(&other.label) == key)
        .count();
    matches > 1
}

/// Indica se `path` possui extensão de imagem de disco montável (`.iso` ou `.img`).
#[must_use]
pub fn is_mountable_disk_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("iso") || ext.eq_ignore_ascii_case("img"))
}

/// Modo de visualização do canvas central.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ViewMode {
    #[default]
    #[serde(alias = "Columns")]
    Grid,
    DetailTable,
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

/// Convenções regionais de formatação de números e datas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocaleFormatSpec {
    pub thousands_sep: char,
    pub months_short: [&'static str; 12],
}

/// Formata um inteiro sem sinal com separador de milhares configurável.
pub fn format_grouped_u64(value: u64, thousands_sep: char) -> CompactString {
    let raw = value.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (idx, ch) in raw.chars().rev().enumerate() {
        if idx > 0 && idx.is_multiple_of(3) {
            out.push(thousands_sep);
        }
        out.push(ch);
    }
    let grouped: String = out.chars().rev().collect();
    CompactString::from(grouped)
}

/// Decomposição de timestamp no fuso horário local para formatação multilíngue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTimestampKind {
    Today {
        hour: u32,
        minute: u32,
    },
    Yesterday {
        hour: u32,
        minute: u32,
    },
    Date {
        day: u32,
        month_idx: usize,
        year: i32,
        hour: u32,
        minute: u32,
    },
}

/// Converte um `SystemTime` em componentes locais (`Hoje`, `Ontem` ou data completa).
pub fn local_timestamp_kind(ts: SystemTime) -> LocalTimestampKind {
    use chrono::{DateTime, Datelike, Local, Timelike};

    let dt: DateTime<Local> = DateTime::from(ts);
    let now = Local::now();
    let today = now.date_naive();
    let date = dt.date_naive();
    let hour = dt.hour();
    let minute = dt.minute();

    if date == today {
        LocalTimestampKind::Today { hour, minute }
    } else if today.pred_opt() == Some(date) {
        LocalTimestampKind::Yesterday { hour, minute }
    } else {
        LocalTimestampKind::Date {
            day: dt.day(),
            month_idx: dt.month0() as usize,
            year: dt.year(),
            hour,
            minute,
        }
    }
}

/// Locais padrão do usuário para mapeamento de ícones temáticos do sistema operacional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemPlaceKind {
    Home,
    Desktop,
    Documents,
    Downloads,
    Music,
    Pictures,
    Videos,
    Public,
    Templates,
    Trash,
}

/// Chave semântica de cache para resolução de ícones do sistema hospedeiro.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SystemIconKey {
    Directory,
    Archive,
    Extension(CompactString),
    GenericFile,
    SpecialPlace(SystemPlaceKind),
    Drive(DriveKind),
}

impl SystemIconKey {
    /// Constrói a chave semântica de ícone para uma entrada de diretório ou arquivo.
    pub fn for_entry(entry: &FileEntry, is_archive: bool) -> Self {
        if entry.kind.is_dir_like() {
            return Self::Directory;
        }
        if is_archive {
            return Self::Archive;
        }
        if let Some(ext) = entry
            .extension
            .as_deref()
            .filter(|e| !e.is_empty())
            .map(str::to_ascii_lowercase)
        {
            return Self::Extension(CompactString::from(ext));
        }
        Self::GenericFile
    }
}

/// Buckets de resolução para rasterização e cache de ícones do sistema em GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemIconBucket {
    Small24,
    Large64,
}

impl SystemIconBucket {
    /// Tamanho lógico em pixels associado ao bucket.
    #[must_use]
    pub const fn target_px(self) -> u32 {
        match self {
            Self::Small24 => 48,
            Self::Large64 => 128,
        }
    }

    /// Seleciona o bucket apropriado para um tamanho de exibição na UI.
    #[must_use]
    pub fn for_display_size(size: f32) -> Self {
        if size >= 32.0 {
            Self::Large64
        } else {
            Self::Small24
        }
    }
}

/// Retorna `true` se o modo POSIX possui ao menos um bit de execução (`0o111`).
#[must_use]
pub const fn is_posix_executable(mode: u32) -> bool {
    (mode & 0o111) != 0
}

/// Alterna os bits de execução de um arquivo preservando os níveis existentes de leitura.
#[must_use]
pub const fn toggle_posix_executable_mode(mode: u32) -> u32 {
    let base = mode & 0o7777;
    if is_posix_executable(base) {
        base & !0o111
    } else {
        base | 0o100 | ((base & 0o040) >> 2) | ((base & 0o004) >> 2)
    }
}

/// Payload bruto retornado pelos backends de sistema antes da rasterização/upload.
#[derive(Debug, Clone)]
pub enum SystemIconPayload {
    File(PathBuf),
    Rgba8 {
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    },
}

/// Descritor de aplicativo disponível para abrir um arquivo via "Abrir com...".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenWithApp {
    /// Identificador estável (`org.videolan.VLC.desktop`, ProgId/caminho `.exe` ou `custom:<cmd>`).
    pub id: CompactString,
    pub display_name: CompactString,
    /// Linha de comando (`Exec=` no Linux ou caminho do executável no Windows).
    pub exec: String,
    /// Nome de ícone FreeDesktop ou caminho de executável/arquivo de ícone.
    #[serde(default)]
    pub icon_hint: Option<String>,
    #[serde(default)]
    pub is_recommended: bool,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub is_custom: bool,
    #[serde(default)]
    pub runs_in_terminal: bool,
}

impl OpenWithApp {
    /// Converte o item descoberto em associação persistível no `AppConfig`.
    #[must_use]
    pub fn to_association(&self) -> OpenWithAssociation {
        OpenWithAssociation {
            app_id: self.id.to_string(),
            display_name: self.display_name.to_string(),
            exec: self.exec.clone(),
            icon_hint: self.icon_hint.clone(),
            runs_in_terminal: self.runs_in_terminal,
        }
    }
}

/// Preferência persistida de "Abrir com..." para uma extensão ou tipo MIME.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenWithAssociation {
    pub app_id: String,
    pub display_name: String,
    pub exec: String,
    #[serde(default)]
    pub icon_hint: Option<String>,
    #[serde(default)]
    pub runs_in_terminal: bool,
}

impl OpenWithAssociation {
    /// Constrói um `OpenWithApp` pronto para exibição na lista de aplicativos.
    #[must_use]
    pub fn to_app(&self, is_recommended: bool, is_default: bool, is_custom: bool) -> OpenWithApp {
        OpenWithApp {
            id: CompactString::from(self.app_id.as_str()),
            display_name: CompactString::from(self.display_name.as_str()),
            exec: self.exec.clone(),
            icon_hint: self.icon_hint.clone(),
            is_recommended,
            is_default,
            is_custom,
            runs_in_terminal: self.runs_in_terminal,
        }
    }
}

/// Catálogo categorizado de aplicativos para um arquivo específico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenWithCatalog {
    pub type_key: CompactString,
    pub type_label: CompactString,
    pub system_default: Option<OpenWithApp>,
    pub recommended: Vec<OpenWithApp>,
    pub others: Vec<OpenWithApp>,
}

/// Calcula a chave normalizada de tipo (`mime` específico, `.ext` em minúsculas ou `application/octet-stream`).
#[must_use]
pub fn open_with_type_key(path: &Path, mime: Option<&str>) -> String {
    if let Some(m) = mime
        .map(str::trim)
        .filter(|m| !m.is_empty() && !m.eq_ignore_ascii_case("application/octet-stream"))
    {
        return m.to_ascii_lowercase();
    }
    if let Some(ext) = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::trim)
        .filter(|e| !e.is_empty())
    {
        return format!(".{}", ext.to_ascii_lowercase());
    }
    "application/octet-stream".to_owned()
}

/// Gera o nome de arquivo candidato (sem perda de `OsString`) para um atalho ou link simbólico.
///
/// - Quando `is_windows_lnk` é `true`, produz um arquivo `.lnk` (ex.: `relatorio.lnk`, `relatorio (2).lnk`).
/// - Quando `is_windows_lnk` é `false` (Linux/Unix), preserva o nome original na primeira tentativa
///   fora da mesma pasta (`attempt == 0 && !force_suffix`) e adiciona ` (link)` / ` (link N)` antes
///   da extensão em colisões ou ao criar na mesma pasta (`force_suffix == true`).
#[must_use]
pub fn suggest_shortcut_name(
    target: &Path,
    is_dir: bool,
    is_windows_lnk: bool,
    attempt: u32,
    force_suffix: bool,
) -> std::ffi::OsString {
    use std::ffi::{OsStr, OsString};

    let raw_name: &OsStr = target
        .file_name()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| OsStr::new("shortcut"));

    let (stem, ext): (&OsStr, Option<&OsStr>) = if is_dir {
        (raw_name, None)
    } else {
        let p = Path::new(raw_name);
        match (p.file_stem(), p.extension()) {
            (Some(s), Some(e)) if !s.is_empty() && !e.is_empty() => (s, Some(e)),
            _ => (raw_name, None),
        }
    };

    if is_windows_lnk {
        let mut out = OsString::from(stem);
        let suffix_num = if force_suffix {
            attempt.saturating_add(2)
        } else {
            attempt.saturating_add(1)
        };
        if suffix_num >= 2 {
            out.push(format!(" ({suffix_num})"));
        }
        out.push(".lnk");
        return out;
    }

    if attempt == 0 && !force_suffix {
        return raw_name.to_os_string();
    }

    let link_index = if force_suffix {
        attempt.saturating_add(1)
    } else {
        attempt
    };

    let mut out = OsString::from(stem);
    if link_index <= 1 {
        out.push(" (link)");
    } else {
        out.push(format!(" (link {link_index})"));
    }
    if let Some(e) = ext {
        out.push(".");
        out.push(e);
    }
    out
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
            created_at: None,
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

    #[test]
    fn formats_grouped_integers_and_timestamps() {
        assert_eq!(format_grouped_u64(0, '.'), "0");
        assert_eq!(format_grouped_u64(999, '.'), "999");
        assert_eq!(format_grouped_u64(14_931_200, '.'), "14.931.200");
        assert_eq!(format_grouped_u64(14_931_200, ','), "14,931,200");

        assert!(matches!(
            local_timestamp_kind(SystemTime::now()),
            LocalTimestampKind::Today { .. }
        ));
        let past = std::time::UNIX_EPOCH + std::time::Duration::from_secs(86_400 * 400);
        assert!(matches!(
            local_timestamp_kind(past),
            LocalTimestampKind::Date { year: 1971, .. }
        ));
    }

    #[test]
    fn toggles_posix_executable_bits_preserving_read_scope() {
        assert!(!is_posix_executable(0o644));
        assert_eq!(toggle_posix_executable_mode(0o644), 0o755);
        assert_eq!(toggle_posix_executable_mode(0o640), 0o750);
        assert_eq!(toggle_posix_executable_mode(0o600), 0o700);
        assert!(is_posix_executable(0o755));
        assert_eq!(toggle_posix_executable_mode(0o755), 0o644);
        assert_eq!(toggle_posix_executable_mode(0o750), 0o640);
        assert_eq!(toggle_posix_executable_mode(0o700), 0o600);
    }

    #[test]
    fn suggests_shortcut_names_for_unix_and_windows() {
        // Unix: pasta diferente preserva nome exato na 1ª tentativa e usa (link) em colisões.
        assert_eq!(
            suggest_shortcut_name(Path::new("/home/u/relatorio.pdf"), false, false, 0, false),
            "relatorio.pdf"
        );
        assert_eq!(
            suggest_shortcut_name(Path::new("/home/u/relatorio.pdf"), false, false, 1, false),
            "relatorio (link).pdf"
        );
        assert_eq!(
            suggest_shortcut_name(Path::new("/home/u/relatorio.pdf"), false, false, 2, false),
            "relatorio (link 2).pdf"
        );
        // Unix: mesma pasta força sufixo já na tentativa 0.
        assert_eq!(
            suggest_shortcut_name(Path::new("/home/u/relatorio.pdf"), false, false, 0, true),
            "relatorio (link).pdf"
        );
        assert_eq!(
            suggest_shortcut_name(Path::new("/home/u/relatorio.pdf"), false, false, 1, true),
            "relatorio (link 2).pdf"
        );
        // Pasta com ponto no nome não deve ser tratada como extensão.
        assert_eq!(
            suggest_shortcut_name(Path::new("/home/u/v1.2.3"), true, false, 0, true),
            "v1.2.3 (link)"
        );
        // Dotfile sem extensão.
        assert_eq!(
            suggest_shortcut_name(Path::new("/home/u/.bashrc"), false, false, 0, true),
            ".bashrc (link)"
        );
        // Windows: sempre produz .lnk baseado no stem (arquivo) ou nome inteiro (pasta).
        assert_eq!(
            suggest_shortcut_name(Path::new("/docs/relatorio.pdf"), false, true, 0, false),
            "relatorio.lnk"
        );
        assert_eq!(
            suggest_shortcut_name(Path::new("/docs/relatorio.pdf"), false, true, 1, false),
            "relatorio (2).lnk"
        );
        assert_eq!(
            suggest_shortcut_name(Path::new("/docs/v1.2.3"), true, true, 0, false),
            "v1.2.3.lnk"
        );
        assert_eq!(
            suggest_shortcut_name(Path::new("/docs/v1.2.3"), true, true, 1, false),
            "v1.2.3 (2).lnk"
        );
    }

    #[test]
    fn detects_mountable_disk_images_case_insensitively() {
        assert!(is_mountable_disk_image(Path::new("/tmp/ubuntu-24.04.iso")));
        assert!(is_mountable_disk_image(Path::new("/tmp/WINDOWS.ISO")));
        assert!(is_mountable_disk_image(Path::new("/tmp/raspi-os.img")));
        assert!(is_mountable_disk_image(Path::new("/tmp/backup.IMG")));
        assert!(!is_mountable_disk_image(Path::new("/tmp/archive.zip")));
        assert!(!is_mountable_disk_image(Path::new("/tmp/noext")));
    }

    #[test]
    fn sorts_drives_with_system_first_and_disambiguates_similar_labels() {
        let mk = |label: &str, dev: &str, kind: DriveKind, mounted: bool| DriveInfo {
            label: CompactString::new(label),
            mount_path: if mounted {
                PathBuf::from(if kind == DriveKind::InternalSsd {
                    "/"
                } else {
                    "/mnt/x"
                })
            } else {
                PathBuf::new()
            },
            device_path: Some(PathBuf::from(dev)),
            model: None,
            fs_type: Some(CompactString::new("ntfs")),
            kind,
            is_mounted: mounted,
            can_unmount: mounted && kind != DriveKind::InternalSsd,
            is_ejectable: false,
            is_disk_image: kind == DriveKind::OpticalOrIso,
            total_bytes_hint: Some(1000),
            space: None,
        };

        let mut drives = vec![
            mk("Download", "/dev/sdc1", DriveKind::SecondaryDisk, false),
            mk("Downloads", "/dev/sdd2", DriveKind::SecondaryDisk, false),
            mk("Backup", "/dev/sde1", DriveKind::SecondaryDisk, false),
            mk("Ubuntu ISO", "/dev/loop0", DriveKind::OpticalOrIso, true),
            mk("Root (/)", "/dev/nvme0n1p4", DriveKind::InternalSsd, true),
        ];

        sort_drives_for_display(&mut drives);
        assert_eq!(drives[0].label, "Root (/)");
        assert_eq!(drives[1].label, "Ubuntu ISO");
        assert_eq!(drives[2].label, "Backup");

        assert!(drive_needs_disambiguation(&drives[3], &drives));
        assert!(drive_needs_disambiguation(&drives[4], &drives));
        assert!(!drive_needs_disambiguation(&drives[2], &drives));
        assert!(!drive_needs_disambiguation(&drives[0], &drives));
        assert_eq!(drive_short_device_name(&drives[3]), Some("sdc1"));
    }
}
