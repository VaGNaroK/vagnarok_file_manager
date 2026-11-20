//! Domínio puro do Buscador de Arquivos Duplicados (MD5 com múltiplos níveis de confirmação).
//!
//! Este módulo não realiza I/O de disco nem depende de `iced` ou APIs específicas de SO.

use std::collections::HashSet;
use std::path::PathBuf;

use compact_str::CompactString;

/// Nível progressivo de confirmação da duplicidade selecionado pelo usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicateConfirmLevel {
    /// Nível 1: Tamanho exato + MD5 Parcial (início + meio + fim; curto-circuito `<= 32 KiB`).
    QuickPartialMd5,
    /// Nível 2 (padrão): Tamanho + MD5 Parcial + MD5 Completo em streaming.
    #[default]
    StandardFullMd5,
    /// Nível 3: MD5 Completo + confirmação SHA-256 direto do disco (bypass de cache).
    StrictMd5AndSha256,
    /// Nível 4: MD5 Completo + comparação byte a byte 2-a-2 contra âncora (bypass de cache).
    ParanoidMd5AndByteCompare,
}

impl DuplicateConfirmLevel {
    pub const ALL: [Self; 4] = [
        Self::QuickPartialMd5,
        Self::StandardFullMd5,
        Self::StrictMd5AndSha256,
        Self::ParanoidMd5AndByteCompare,
    ];

    /// Modo Rápido (triagem parcial) bloqueia exclusão permanente em 1 clique por segurança.
    pub const fn allows_permanent_delete(self) -> bool {
        !matches!(self, Self::QuickPartialMd5)
    }

    /// Níveis 3 e 4 fazem bypass completo do cache `redb`, lendo sempre do disco real.
    pub const fn uses_disk_cache(self) -> bool {
        matches!(self, Self::QuickPartialMd5 | Self::StandardFullMd5)
    }

    /// Número total de estágios do funil para exibição na barra de progresso.
    pub const fn stage_total(self) -> u8 {
        match self {
            Self::QuickPartialMd5 => 2,
            Self::StandardFullMd5 => 3,
            Self::StrictMd5AndSha256 | Self::ParanoidMd5AndByteCompare => 4,
        }
    }
}

/// Presets de tamanho mínimo para o dropdown da interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplicateMinSizePreset {
    Any,
    #[default]
    OneKb,
    OneMb,
    TenMb,
}

impl DuplicateMinSizePreset {
    pub const ALL: [Self; 4] = [Self::Any, Self::OneKb, Self::OneMb, Self::TenMb];

    pub const fn bytes(self) -> u64 {
        match self {
            Self::Any => 1,
            Self::OneKb => 1_024,
            Self::OneMb => 1_048_576,
            Self::TenMb => 10_485_760,
        }
    }
}

/// Configuração da varredura de arquivos duplicados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateScanOptions {
    pub roots: Vec<PathBuf>,
    pub confirm_level: DuplicateConfirmLevel,
    pub recursive: bool,
    pub include_hidden: bool,
    pub ignore_empty_files: bool,
    pub ignore_hardlinks: bool,
    pub min_size_bytes: u64,
}

impl DuplicateScanOptions {
    pub fn for_root(root: PathBuf) -> Self {
        Self {
            roots: vec![root],
            confirm_level: DuplicateConfirmLevel::StandardFullMd5,
            recursive: true,
            include_hidden: false,
            ignore_empty_files: true,
            ignore_hardlinks: true,
            min_size_bytes: DuplicateMinSizePreset::OneKb.bytes(),
        }
    }

    /// Tamanho mínimo efetivo considerando `ignore_empty_files`.
    pub fn effective_min_size(&self) -> u64 {
        if self.ignore_empty_files {
            self.min_size_bytes.max(1)
        } else {
            self.min_size_bytes
        }
    }
}

/// Representação de um arquivo candidato confirmado em um grupo de duplicatas.
///
/// Usa estritamente `path: PathBuf` como chave de identificação e remoção (nunca `EntryId`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateFileItem {
    pub path: PathBuf,
    pub relative_display: CompactString,
    pub size_bytes: u64,
    pub modified_nanos: Option<u128>,
    pub modified_secs: Option<i64>,
    /// Identidade física `(dev, ino)` no Linux ou `(volume_serial, file_index)` no Windows.
    pub file_identity: Option<(u64, u64)>,
    pub is_hardlink_sibling: bool,
    pub selected_for_delete: bool,
}

/// Grupo de 2 ou mais arquivos confirmados como duplicados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub group_id: u64,
    pub size_bytes: u64,
    pub md5_hex: CompactString,
    pub md5_short: CompactString,
    pub sha256_prefix: Option<CompactString>,
    pub verified_level: DuplicateConfirmLevel,
    pub items: Vec<DuplicateFileItem>,
}

impl DuplicateGroup {
    /// Calcula bytes efetivamente recuperáveis no grupo considerando identidades físicas (hardlinks).
    ///
    /// Hardlinks que apontam para o mesmo `(dev, ino)` / `FileIdentity` compartilham o mesmo espaço
    /// em disco e contam como uma única cópia física.
    pub fn reclaimable_bytes(&self) -> u64 {
        let physical_copies = self.physical_copy_count();
        self.size_bytes
            .saturating_mul(physical_copies.saturating_sub(1) as u64)
    }

    /// Quantidade de cópias físicas distintas no grupo (deduplicando hardlinks).
    pub fn physical_copy_count(&self) -> usize {
        let mut seen_ids: HashSet<(u64, u64)> = HashSet::new();
        let mut count = 0usize;
        for item in &self.items {
            match item.file_identity {
                Some(id) => {
                    if seen_ids.insert(id) {
                        count += 1;
                    }
                }
                None => count += 1,
            }
        }
        count
    }

    /// Calcula quantos bytes serão realmente liberados em disco pelos itens atualmente marcados.
    ///
    /// Se um arquivo possui hardlink irmão que permanecerá desmarcado, apagar apenas um dos links
    /// libera `0 B`.
    pub fn selected_reclaimable_bytes(&self) -> u64 {
        let mut kept_identities: HashSet<(u64, u64)> = HashSet::new();
        for item in &self.items {
            if !item.selected_for_delete
                && let Some(id) = item.file_identity
            {
                kept_identities.insert(id);
            }
        }

        let mut freed_identities: HashSet<(u64, u64)> = HashSet::new();
        let mut freed_copies = 0u64;
        for item in &self.items {
            if !item.selected_for_delete {
                continue;
            }
            match item.file_identity {
                Some(id) => {
                    if !kept_identities.contains(&id) && freed_identities.insert(id) {
                        freed_copies += 1;
                    }
                }
                None => freed_copies += 1,
            }
        }
        self.size_bytes.saturating_mul(freed_copies)
    }

    /// Retorna `true` se TODOS os exemplares do grupo estiverem marcados para exclusão.
    pub fn all_items_selected(&self) -> bool {
        !self.items.is_empty() && self.items.iter().all(|item| item.selected_for_delete)
    }

    /// Quantidade de exemplares marcados para exclusão neste grupo.
    pub fn selected_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.selected_for_delete)
            .count()
    }

    /// Menor timestamp `modified_secs` no grupo (para badge "Mais antigo").
    pub fn oldest_mtime_secs(&self) -> Option<i64> {
        self.items.iter().filter_map(|i| i.modified_secs).min()
    }
}

/// Estratégias puras de seleção inteligente em 1 clique.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepStrategy {
    KeepOldest,
    KeepNewest,
    KeepShortestPath,
    KeepFirstInScan,
    ClearAll,
}

/// Aplica uma estratégia determinística de seleção em todos os grupos.
///
/// Exceto em `ClearAll`, preserva SEMPRE exatamente 1 cópia desmarcada por grupo (usando
/// `path` como critério determinístico de desempate em caso de timestamps ou comprimentos iguais).
pub fn apply_keep_strategy(groups: &mut [DuplicateGroup], strategy: KeepStrategy) {
    for group in groups {
        if group.items.is_empty() {
            continue;
        }
        if strategy == KeepStrategy::ClearAll {
            for item in &mut group.items {
                item.selected_for_delete = false;
            }
            continue;
        }

        let keeper_idx = match strategy {
            KeepStrategy::KeepFirstInScan => 0,
            KeepStrategy::KeepOldest => group
                .items
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let a_key = (
                        a.modified_secs.unwrap_or(i64::MAX),
                        a.modified_nanos.unwrap_or(u128::MAX),
                        a.path.as_os_str(),
                    );
                    let b_key = (
                        b.modified_secs.unwrap_or(i64::MAX),
                        b.modified_nanos.unwrap_or(u128::MAX),
                        b.path.as_os_str(),
                    );
                    a_key.cmp(&b_key)
                })
                .map(|(idx, _)| idx)
                .unwrap_or(0),
            KeepStrategy::KeepNewest => group
                .items
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    // Maior timestamp primeiro; desempate determinístico pelo menor caminho.
                    b.modified_secs
                        .unwrap_or(i64::MIN)
                        .cmp(&a.modified_secs.unwrap_or(i64::MIN))
                        .then_with(|| {
                            b.modified_nanos
                                .unwrap_or(0)
                                .cmp(&a.modified_nanos.unwrap_or(0))
                        })
                        .then_with(|| a.path.as_os_str().cmp(b.path.as_os_str()))
                })
                .map(|(idx, _)| idx)
                .unwrap_or(0),
            KeepStrategy::KeepShortestPath => group
                .items
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.path
                        .as_os_str()
                        .len()
                        .cmp(&b.path.as_os_str().len())
                        .then_with(|| a.path.as_os_str().cmp(b.path.as_os_str()))
                })
                .map(|(idx, _)| idx)
                .unwrap_or(0),
            KeepStrategy::ClearAll => unreachable!(),
        };

        for (idx, item) in group.items.iter_mut().enumerate() {
            item.selected_for_delete = idx != keeper_idx;
        }
    }
}

/// Estágio atual do funil de busca de duplicados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateScanStage {
    Walking,
    PartialMd5,
    FullMd5,
    Sha256Verify,
    ByteCompare,
    Finished,
}

/// Progresso throttled emitido pelo worker para a UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateScanProgress {
    pub stage: DuplicateScanStage,
    pub stage_index: u8,
    pub stage_total: u8,
    pub processed: usize,
    pub total: usize,
    pub groups_found: usize,
    pub reclaimable_bytes: u64,
    pub current_path: CompactString,
}

/// Eventos assíncronos enviados do motor de busca (`fm-telemetry`) para `fm-app`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DuplicateScanEvent {
    Progress(DuplicateScanProgress),
    Completed {
        groups: Vec<DuplicateGroup>,
        scanned_files: usize,
        skipped_errors: usize,
    },
    Cancelled,
    Failed(String),
}

/// Formata um hash hexadecimal longo de forma compacta (`9e107d..b4`).
pub fn format_short_hash(hex: &str) -> CompactString {
    if hex.len() >= 10 {
        CompactString::from(format!("{}..{}", &hex[..6], &hex[hex.len() - 2..]))
    } else {
        CompactString::from(hex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item(
        path: &str,
        size: u64,
        mtime: Option<i64>,
        identity: Option<(u64, u64)>,
    ) -> DuplicateFileItem {
        DuplicateFileItem {
            path: PathBuf::from(path),
            relative_display: CompactString::from(path),
            size_bytes: size,
            modified_nanos: mtime.map(|s| (s.max(0) as u128) * 1_000_000_000),
            modified_secs: mtime,
            file_identity: identity,
            is_hardlink_sibling: false,
            selected_for_delete: false,
        }
    }

    fn sample_group(items: Vec<DuplicateFileItem>, size: u64) -> DuplicateGroup {
        DuplicateGroup {
            group_id: 1,
            size_bytes: size,
            md5_hex: CompactString::new("9e107d9d372bb6826bd81d3542a419d6"),
            md5_short: format_short_hash("9e107d9d372bb6826bd81d3542a419d6"),
            sha256_prefix: None,
            verified_level: DuplicateConfirmLevel::StandardFullMd5,
            items,
        }
    }

    #[test]
    fn keep_strategies_always_preserve_one_copy_and_tiebreak_deterministically() {
        let mut groups = vec![sample_group(
            vec![
                sample_item("/a/very/long/path/dup.iso", 1000, Some(200), Some((1, 10))),
                sample_item("/a/b.iso", 1000, Some(100), Some((1, 11))),
                sample_item("/a/a.iso", 1000, Some(100), Some((1, 12))),
                sample_item("/a/newest.iso", 1000, Some(500), Some((1, 13))),
            ],
            1000,
        )];

        apply_keep_strategy(&mut groups, KeepStrategy::KeepOldest);
        // Empate em mtime=100 entre /a/b.iso e /a/a.iso -> mantém /a/a.iso (menor caminho).
        assert!(!groups[0].items[2].selected_for_delete);
        assert_eq!(groups[0].selected_count(), 3);
        assert!(!groups[0].all_items_selected());

        apply_keep_strategy(&mut groups, KeepStrategy::KeepNewest);
        assert!(!groups[0].items[3].selected_for_delete);
        assert_eq!(groups[0].selected_count(), 3);

        apply_keep_strategy(&mut groups, KeepStrategy::KeepShortestPath);
        // Empate em comprimento=8 entre /a/b.iso e /a/a.iso -> mantém /a/a.iso.
        assert!(!groups[0].items[2].selected_for_delete);
        assert_eq!(groups[0].selected_count(), 3);

        apply_keep_strategy(&mut groups, KeepStrategy::ClearAll);
        assert_eq!(groups[0].selected_count(), 0);
    }

    #[test]
    fn reclaimable_bytes_accounts_for_hardlinks_sharing_identity() {
        let mut group = sample_group(
            vec![
                sample_item("/dir/orig.bin", 500, Some(10), Some((7, 42))),
                sample_item("/dir/hardlink.bin", 500, Some(10), Some((7, 42))),
                sample_item("/dir/real_copy.bin", 500, Some(20), Some((7, 99))),
            ],
            500,
        );
        // 2 cópias físicas distintas -> apenas 500 bytes recuperáveis no grupo.
        assert_eq!(group.physical_copy_count(), 2);
        assert_eq!(group.reclaimable_bytes(), 500);

        // Marcar apenas o hardlink enquanto o original continua desmarcado libera 0 bytes.
        group.items[1].selected_for_delete = true;
        assert_eq!(group.selected_reclaimable_bytes(), 0);

        // Marcar a cópia física independente libera 500 bytes.
        group.items[2].selected_for_delete = true;
        assert_eq!(group.selected_reclaimable_bytes(), 500);

        // Marcar também o original (todos marcados) liberaria 1000 bytes (e aciona alerta).
        group.items[0].selected_for_delete = true;
        assert!(group.all_items_selected());
        assert_eq!(group.selected_reclaimable_bytes(), 1000);
    }

    #[test]
    fn confirm_level_policies_match_spec() {
        assert!(!DuplicateConfirmLevel::QuickPartialMd5.allows_permanent_delete());
        assert!(DuplicateConfirmLevel::StandardFullMd5.allows_permanent_delete());
        assert!(DuplicateConfirmLevel::StrictMd5AndSha256.allows_permanent_delete());
        assert!(DuplicateConfirmLevel::ParanoidMd5AndByteCompare.allows_permanent_delete());

        assert!(DuplicateConfirmLevel::QuickPartialMd5.uses_disk_cache());
        assert!(DuplicateConfirmLevel::StandardFullMd5.uses_disk_cache());
        assert!(!DuplicateConfirmLevel::StrictMd5AndSha256.uses_disk_cache());
        assert!(!DuplicateConfirmLevel::ParanoidMd5AndByteCompare.uses_disk_cache());

        assert_eq!(
            format_short_hash("9e107d9d372bb6826bd81d3542a419d6"),
            "9e107d..d6"
        );
    }
}
