//! Tipos puros e regras de sanitização para extração e compactação em segundo plano.

use std::path::{Component, Path, PathBuf};

use compact_str::CompactString;

use crate::domain::JobState;

/// Formatos suportados para criação de pacotes na janela "Compactar como...".
///
/// `.rar` é intencionalmente ausente aqui por ser formato proprietário (suportado apenas na extração).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArchiveFormat {
    #[default]
    SevenZ,
    Zip,
    Tar,
    TarGz,
}

impl ArchiveFormat {
    pub const ALL: [Self; 4] = [Self::SevenZ, Self::Zip, Self::Tar, Self::TarGz];

    /// Extensão padrão do arquivo resultante (sem o ponto inicial).
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::SevenZ => "7z",
            Self::Zip => "zip",
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
        }
    }

    /// Indica se o formato aceita níveis de compressão configuráveis (`.tar` apenas armazena).
    #[must_use]
    pub const fn supports_compression_level(self) -> bool {
        !matches!(self, Self::Tar)
    }
}

/// Nível de compressão selecionado em dropdown (com `Maximum` obrigatoriamente por último).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionLevel {
    Store,
    Minimum,
    #[default]
    Normal,
    Maximum,
}

impl CompressionLevel {
    /// Ordem oficial exibida no dropdown da interface gráfica.
    pub const ALL: [Self; 4] = [Self::Store, Self::Minimum, Self::Normal, Self::Maximum];
}

/// Modo de destino ao extrair um arquivo compactado via menu de contexto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractTargetMode {
    Here,
    Subfolder,
}

/// Natureza da tarefa de empacotamento/extração em segundo plano.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveJobKind {
    Compress {
        format: ArchiveFormat,
        level: CompressionLevel,
    },
    Extract {
        mode: ExtractTargetMode,
    },
}

/// Progresso contínuo emitido pelos workers de compactação/extração para o rodapé e janela modal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveJobProgress {
    pub job_id: u64,
    pub kind: ArchiveJobKind,
    pub title: CompactString,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub items_done: usize,
    pub items_total: usize,
    pub current_item: CompactString,
    pub dest_dir: PathBuf,
    pub output_path: PathBuf,
    pub state: JobState,
}

/// Sugere um nome base limpo (sem extensão) para o pacote resultante a partir dos itens selecionados.
#[must_use]
pub fn suggest_archive_base_name(sources: &[PathBuf], dest_dir: &Path) -> String {
    if sources.len() == 1
        && let Some(only) = sources.first()
        && let Some(name) = only.file_name()
    {
        let raw = name.to_string_lossy();
        if only.is_dir() {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                return trimmed.to_owned();
            }
        } else if let Some((stem, ext)) = raw.rsplit_once('.')
            && !stem.is_empty()
            && !ext.is_empty()
        {
            return stem.to_owned();
        } else if !raw.trim().is_empty() {
            return raw.trim().to_owned();
        }
    }

    if let Some(parent_name) = dest_dir.file_name() {
        let trimmed = parent_name.to_string_lossy().trim().to_owned();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }

    "archive".to_owned()
}

/// Remove a extensão conhecida do arquivo compactado (e sufixo `.partN` em `.rar`) para criar subpasta.
#[must_use]
pub fn strip_known_archive_suffix(file_name: &str) -> String {
    let trimmed = file_name.trim();
    let lower = trimmed.to_ascii_lowercase();

    let stripped = if lower.ends_with(".tar.gz") {
        &trimmed[..trimmed.len() - ".tar.gz".len()]
    } else if lower.ends_with(".tgz") {
        &trimmed[..trimmed.len() - ".tgz".len()]
    } else if lower.ends_with(".zip") {
        &trimmed[..trimmed.len() - ".zip".len()]
    } else if lower.ends_with(".tar") {
        &trimmed[..trimmed.len() - ".tar".len()]
    } else if lower.ends_with(".7z") {
        &trimmed[..trimmed.len() - ".7z".len()]
    } else if lower.ends_with(".rar") {
        let without_rar = &trimmed[..trimmed.len() - ".rar".len()];
        strip_rar_part_suffix(without_rar)
    } else if let Some((stem, _)) = trimmed.rsplit_once('.')
        && !stem.is_empty()
    {
        stem
    } else {
        trimmed
    };

    let clean = stripped.trim().trim_end_matches(['.', ' ']);
    if clean.is_empty() || clean == "." || clean == ".." {
        "extracted".to_owned()
    } else {
        clean.to_owned()
    }
}

fn strip_rar_part_suffix(base: &str) -> &str {
    let lower = base.to_ascii_lowercase();
    if let Some(idx) = lower.rfind(".part") {
        let digits = &lower[idx + ".part".len()..];
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            return &base[..idx];
        }
    }
    base
}

/// Sanitiza um caminho relativo vindo de dentro de um arquivo compactado (proteção anti Zip-Slip).
///
/// Rejeita caminhos absolutos, prefixos de drive Windows (`C:\`), componentes `..`, bytes NUL
/// e qualquer segmento vazio/inválido.
#[must_use]
pub fn sanitize_archive_rel_path(raw: &str) -> Option<PathBuf> {
    if raw.is_empty() || raw.contains('\0') {
        return None;
    }

    let normalized = raw.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/');
    if normalized.starts_with('/') || looks_like_windows_drive_prefix(trimmed) {
        return None;
    }

    let mut clean = PathBuf::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            return None;
        }
        let sub_path = Path::new(segment);
        let mut comps = sub_path.components();
        match (comps.next(), comps.next()) {
            (Some(Component::Normal(part)), None) => clean.push(part),
            _ => return None,
        }
    }

    if clean.as_os_str().is_empty() {
        None
    } else {
        Some(clean)
    }
}

fn looks_like_windows_drive_prefix(s: &str) -> bool {
    let bytes = s.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_level_places_maximum_last() {
        assert_eq!(
            CompressionLevel::ALL,
            [
                CompressionLevel::Store,
                CompressionLevel::Minimum,
                CompressionLevel::Normal,
                CompressionLevel::Maximum,
            ]
        );
        assert!(ArchiveFormat::SevenZ.supports_compression_level());
        assert!(!ArchiveFormat::Tar.supports_compression_level());
    }

    #[test]
    fn sanitizes_archive_paths_and_blocks_zip_slip() {
        assert_eq!(
            sanitize_archive_rel_path("./docs/readme.txt"),
            Some(PathBuf::from("docs/readme.txt"))
        );
        assert_eq!(
            sanitize_archive_rel_path(r"pasta\sub\item.bin"),
            Some(PathBuf::from("pasta/sub/item.bin"))
        );
        assert_eq!(sanitize_archive_rel_path("../../etc/passwd"), None);
        assert_eq!(sanitize_archive_rel_path("/etc/shadow"), None);
        assert_eq!(sanitize_archive_rel_path(r"C:\Windows\win.ini"), None);
        assert_eq!(sanitize_archive_rel_path("a/../b.txt"), None);
        assert_eq!(sanitize_archive_rel_path("a\0b.txt"), None);
    }

    #[test]
    fn strips_archive_and_multipart_rar_suffixes() {
        assert_eq!(strip_known_archive_suffix("backup.tar.gz"), "backup");
        assert_eq!(strip_known_archive_suffix("pacote.7z"), "pacote");
        assert_eq!(
            strip_known_archive_suffix("Jogo_Setup.part01.rar"),
            "Jogo_Setup"
        );
        assert_eq!(strip_known_archive_suffix("arquivo.rar"), "arquivo");
    }

    #[test]
    fn suggests_base_name_for_single_and_multi_selection() {
        let dest = PathBuf::from("/home/user/Projetos");
        assert_eq!(
            suggest_archive_base_name(&[PathBuf::from("/home/user/Projetos/relatorio.pdf")], &dest),
            "relatorio"
        );
        assert_eq!(
            suggest_archive_base_name(
                &[
                    PathBuf::from("/home/user/Projetos/a.txt"),
                    PathBuf::from("/home/user/Projetos/b.txt"),
                ],
                &dest
            ),
            "Projetos"
        );
    }
}
