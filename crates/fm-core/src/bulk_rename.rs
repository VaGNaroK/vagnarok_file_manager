//! Motor puro de renomeação em lote: regras visuais, detecção de conflitos e plano em 2 fases.
//!
//! Este módulo não executa I/O de disco nem depende de APIs de UI/SO; políticas dependentes
//! do sistema de arquivos (comparação case-insensitive e nomes reservados do Windows) são
//! recebidas explicitamente como parâmetros para permitir testes determinísticos em qualquer host.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use compact_str::CompactString;
use regex::RegexBuilder;

use crate::domain::EntryId;

/// Tamanho máximo compilado permitido para expressões regulares do modo avançado.
const MAX_REGEX_SIZE_BYTES: usize = 512 * 1024;

/// Metadados extraídos assincronamente (EXIF / ID3) usados pelas regras de composição.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BulkRenameMediaTags {
    pub exif_date_taken: Option<CompactString>,
    pub exif_date_ymd: Option<CompactString>,
    pub exif_camera_model: Option<CompactString>,
    pub id3_title: Option<CompactString>,
    pub id3_artist: Option<CompactString>,
    pub id3_album: Option<CompactString>,
    pub id3_track: Option<u32>,
    pub id3_year: Option<CompactString>,
}

impl BulkRenameMediaTags {
    /// Retorna `true` se ao menos um campo EXIF estiver preenchido.
    #[must_use]
    pub fn has_exif(&self) -> bool {
        self.exif_date_taken.is_some()
            || self.exif_date_ymd.is_some()
            || self.exif_camera_model.is_some()
    }

    /// Retorna `true` se ao menos um campo ID3 estiver preenchido.
    #[must_use]
    pub fn has_id3(&self) -> bool {
        self.id3_title.is_some()
            || self.id3_artist.is_some()
            || self.id3_album.is_some()
            || self.id3_track.is_some()
            || self.id3_year.is_some()
    }
}

/// Entrada imutável representando um item selecionado para renomeação em lote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRenameInput {
    pub entry_id: EntryId,
    pub path: PathBuf,
    pub parent_dir: PathBuf,
    pub original_name: OsString,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified_secs: Option<i64>,
}

/// Campos de metadados EXIF/ID3 disponíveis para inserção amigável.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaTagKind {
    ExifDateYmd,
    ExifDateTime,
    ExifCamera,
    Id3Track,
    Id3Artist,
    Id3Album,
    Id3Title,
    Id3Year,
}

impl MediaTagKind {
    /// Indica se o campo pertence a metadados EXIF de imagem.
    #[must_use]
    pub const fn is_exif(self) -> bool {
        matches!(
            self,
            Self::ExifDateYmd | Self::ExifDateTime | Self::ExifCamera
        )
    }

    /// Indica se o campo pertence a tags ID3 de áudio.
    #[must_use]
    pub const fn is_id3(self) -> bool {
        !self.is_exif()
    }
}

/// Como os campos de metadados selecionados são combinados ao nome base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaComposeMode {
    #[default]
    PrependToName,
    AppendToName,
    ReplaceName,
}

/// Escopo de posição para busca/remoção textual amigável.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchScope {
    #[default]
    Anywhere,
    StartOfName,
    EndOfName,
    WordStart,
    WordEnd,
    WholeWord,
}

/// Onde inserir texto literal no nome base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InsertPosition {
    #[default]
    StartOfName,
    EndOfName,
    FromStart(usize),
    FromEnd(usize),
}

/// Onde posicionar o contador sequencial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumberPosition {
    #[default]
    Append,
    Prepend,
    ReplaceName,
}

/// Transformação de maiúsculas/minúsculas sobre o nome base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextCaseMode {
    #[default]
    Keep,
    Uppercase,
    Lowercase,
    TitleCaseWords,
    SentenceCaseFirst,
}

/// Regra sobre a extensão do arquivo (não aplicada a pastas).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ExtensionMode {
    #[default]
    Keep,
    Lowercase,
    Uppercase,
    Replace(String),
    Remove,
}

/// Critério de ordenação dos itens antes da atribuição de números sequenciais.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BulkRenameSort {
    #[default]
    SelectionOrder,
    NameAsc,
    NameDesc,
    ModifiedDesc,
    ModifiedAsc,
    SizeAsc,
    SizeDesc,
}

/// Conjunto completo de regras configuráveis pela interface gráfica.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRenameRules {
    pub include_folders: bool,
    pub sort_order: BulkRenameSort,
    pub sanitize_invalid_chars: bool,

    // Bloco 1: Metadados EXIF / ID3
    pub media_enabled: bool,
    pub media_fields: Vec<MediaTagKind>,
    pub media_mode: MediaComposeMode,
    pub media_separator: String,
    pub media_track_digits: u8,

    // Bloco 2: Remover
    pub remove_enabled: bool,
    pub remove_first_n: usize,
    pub remove_last_n: usize,
    pub remove_text: String,
    pub remove_scope: MatchScope,
    pub remove_case_sensitive: bool,
    pub remove_digits: bool,
    pub remove_symbols: bool,
    pub collapse_spaces: bool,

    // Bloco 3: Procurar e Substituir (Regex apenas se `use_regex == true`)
    pub replace_enabled: bool,
    pub find_text: String,
    pub replace_with: String,
    pub replace_scope: MatchScope,
    pub replace_case_sensitive: bool,
    pub replace_first_only: bool,
    pub use_regex: bool,

    // Bloco 4: Inserir texto
    pub insert_enabled: bool,
    pub insert_text: String,
    pub insert_position: InsertPosition,

    // Bloco 5: Numeração sequencial
    pub number_enabled: bool,
    pub number_position: NumberPosition,
    pub number_separator: String,
    pub number_start: i64,
    pub number_step: i64,
    pub number_digits: u8,

    // Bloco 6: Maiúsculas / Minúsculas
    pub case_enabled: bool,
    pub case_mode: TextCaseMode,

    // Bloco 7: Extensão
    pub ext_enabled: bool,
    pub ext_mode: ExtensionMode,
}

impl Default for BulkRenameRules {
    fn default() -> Self {
        Self {
            include_folders: true,
            sort_order: BulkRenameSort::SelectionOrder,
            sanitize_invalid_chars: true,

            media_enabled: false,
            media_fields: Vec::new(),
            media_mode: MediaComposeMode::PrependToName,
            media_separator: " - ".to_owned(),
            media_track_digits: 2,

            remove_enabled: false,
            remove_first_n: 0,
            remove_last_n: 0,
            remove_text: String::new(),
            remove_scope: MatchScope::Anywhere,
            remove_case_sensitive: false,
            remove_digits: false,
            remove_symbols: false,
            collapse_spaces: false,

            replace_enabled: false,
            find_text: String::new(),
            replace_with: String::new(),
            replace_scope: MatchScope::Anywhere,
            replace_case_sensitive: false,
            replace_first_only: false,
            use_regex: false,

            insert_enabled: false,
            insert_text: String::new(),
            insert_position: InsertPosition::StartOfName,

            number_enabled: false,
            number_position: NumberPosition::Append,
            number_separator: "_".to_owned(),
            number_start: 1,
            number_step: 1,
            number_digits: 3,

            case_enabled: false,
            case_mode: TextCaseMode::Keep,

            ext_enabled: false,
            ext_mode: ExtensionMode::Keep,
        }
    }
}

impl BulkRenameRules {
    /// Indica se as regras ativas dependem de tags EXIF.
    #[must_use]
    pub fn requires_exif(&self) -> bool {
        self.media_enabled && self.media_fields.iter().any(|f| f.is_exif())
    }

    /// Indica se as regras ativas dependem de tags ID3.
    #[must_use]
    pub fn requires_id3(&self) -> bool {
        self.media_enabled && self.media_fields.iter().any(|f| f.is_id3())
    }

    /// Indica se qualquer leitura de metadados EXIF ou ID3 é necessária.
    #[must_use]
    pub fn requires_media_tags(&self) -> bool {
        self.requires_exif() || self.requires_id3()
    }
}

/// Erro específico de validação de nome resultante.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkRenameNameError {
    Empty,
    ReservedDot,
    ContainsPathSeparator,
    WindowsInvalidChar,
    WindowsReservedName,
    TooLong,
}

/// Estado calculado para cada linha do preview em tempo real.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkRenameRowStatus {
    Ready,
    Unchanged,
    SkippedFolder,
    InvalidName(BulkRenameNameError),
    ConflictInternal,
    ConflictExternal,
}

/// Linha avaliada para exibição no preview virtualizado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRenamePreviewRow {
    pub input_index: usize,
    pub sequence_pos: usize,
    pub entry_id: EntryId,
    pub src_path: PathBuf,
    pub parent_dir: PathBuf,
    pub original_name: OsString,
    pub new_name: OsString,
    pub dst_path: PathBuf,
    pub is_dir: bool,
    pub status: BulkRenameRowStatus,
}

/// Resultado completo da avaliação das regras sobre um lote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkRenameEvaluation {
    pub rows: Vec<BulkRenamePreviewRow>,
    pub regex_error: Option<String>,
    pub ready_count: usize,
    pub unchanged_count: usize,
    pub skipped_count: usize,
    pub conflict_count: usize,
    pub invalid_count: usize,
}

impl BulkRenameEvaluation {
    /// Retorna `true` somente quando há alterações prontas e zero erros/conflitos.
    #[must_use]
    pub fn can_apply(&self) -> bool {
        self.regex_error.is_none()
            && self.ready_count > 0
            && self.conflict_count == 0
            && self.invalid_count == 0
    }
}

/// Operação unitária de renomeação no sistema de arquivos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameStep {
    pub from: PathBuf,
    pub to: PathBuf,
    pub input_index: usize,
}

/// Plano seguro em ordem topológica + 2 fases para ciclos e trocas apenas de caixa.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BulkRenameExecutionPlan {
    pub direct_steps: Vec<RenameStep>,
    pub phase1_temp_steps: Vec<RenameStep>,
    pub phase2_final_steps: Vec<RenameStep>,
    pub rollback_temp_steps: Vec<RenameStep>,
}

impl BulkRenameExecutionPlan {
    /// Número total de itens do lote efetivamente renomeados.
    #[must_use]
    pub fn total_renames(&self) -> usize {
        self.direct_steps.len() + self.phase2_final_steps.len()
    }
}

/// Opções de política do sistema de arquivos alvo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkRenameTargetPolicy {
    pub case_insensitive_fs: bool,
    pub windows_name_rules: bool,
}

impl BulkRenameTargetPolicy {
    /// Política padrão compatível com o sistema operacional corrente em compilação.
    #[must_use]
    pub const fn host_default() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self {
                case_insensitive_fs: true,
                windows_name_rules: true,
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self {
                case_insensitive_fs: false,
                windows_name_rules: false,
            }
        }
    }
}

/// Avalia todas as regras sobre `inputs` e detecta nomes inválidos e colisões.
pub fn evaluate_bulk_rename(
    inputs: &[BulkRenameInput],
    tags_by_input: &[Option<BulkRenameMediaTags>],
    rules: &BulkRenameRules,
    existing_names_in_dirs: &HashSet<PathBuf>,
    policy: BulkRenameTargetPolicy,
) -> BulkRenameEvaluation {
    let mut ordered_indices: Vec<usize> = (0..inputs.len()).collect();
    sort_input_indices(&mut ordered_indices, inputs, rules.sort_order);

    let (compiled_regex, regex_error) = compile_optional_regex(rules);

    let mut rows = Vec::with_capacity(inputs.len());
    let mut ready_count = 0usize;
    let mut unchanged_count = 0usize;
    let mut skipped_count = 0usize;
    let mut invalid_count = 0usize;

    let mut active_counter_index = 0usize;
    for (seq_idx, &input_idx) in ordered_indices.iter().enumerate() {
        let input = &inputs[input_idx];
        let tags = tags_by_input.get(input_idx).and_then(Option::as_ref);

        if input.is_dir && !rules.include_folders {
            skipped_count += 1;
            rows.push(BulkRenamePreviewRow {
                input_index: input_idx,
                sequence_pos: seq_idx + 1,
                entry_id: input.entry_id,
                src_path: input.path.clone(),
                parent_dir: input.parent_dir.clone(),
                original_name: input.original_name.clone(),
                new_name: input.original_name.clone(),
                dst_path: input.path.clone(),
                is_dir: input.is_dir,
                status: BulkRenameRowStatus::SkippedFolder,
            });
            continue;
        }

        let seq_number_step = active_counter_index;
        active_counter_index += 1;

        let new_name = transform_entry_name(
            &input.original_name,
            input.is_dir,
            seq_number_step,
            tags,
            rules,
            compiled_regex.as_ref(),
            policy,
        );

        let dst_path = input.parent_dir.join(&new_name);
        let new_name_lossy = new_name.to_string_lossy();

        let status =
            if let Err(err) = validate_file_name(&new_name_lossy, policy.windows_name_rules) {
                invalid_count += 1;
                BulkRenameRowStatus::InvalidName(err)
            } else if new_name == input.original_name {
                unchanged_count += 1;
                BulkRenameRowStatus::Unchanged
            } else {
                ready_count += 1;
                BulkRenameRowStatus::Ready
            };

        rows.push(BulkRenamePreviewRow {
            input_index: input_idx,
            sequence_pos: seq_idx + 1,
            entry_id: input.entry_id,
            src_path: input.path.clone(),
            parent_dir: input.parent_dir.clone(),
            original_name: input.original_name.clone(),
            new_name,
            dst_path,
            is_dir: input.is_dir,
            status,
        });
    }

    let conflict_count = mark_conflicts(
        &mut rows,
        existing_names_in_dirs,
        policy.case_insensitive_fs,
        &mut ready_count,
    );

    BulkRenameEvaluation {
        rows,
        regex_error,
        ready_count,
        unchanged_count,
        skipped_count,
        conflict_count,
        invalid_count,
    }
}

/// Constrói plano de execução seguro para ciclos e trocas apenas de maiúsculas/minúsculas.
pub fn build_execution_plan(
    eval: &BulkRenameEvaluation,
    existing_paths: &HashSet<PathBuf>,
    batch_id: u64,
    policy: BulkRenameTargetPolicy,
) -> Option<BulkRenameExecutionPlan> {
    if !eval.can_apply() {
        return None;
    }

    let mut remaining: Vec<&BulkRenamePreviewRow> = eval
        .rows
        .iter()
        .filter(|r| r.status == BulkRenameRowStatus::Ready)
        .collect();

    if remaining.is_empty() {
        return Some(BulkRenameExecutionPlan::default());
    }

    let mut direct_steps = Vec::new();
    let mut used_keys: HashSet<String> = existing_paths
        .iter()
        .map(|p| normalized_path_key(p, policy.case_insensitive_fs))
        .collect();

    for row in &eval.rows {
        used_keys.insert(normalized_path_key(
            &row.src_path,
            policy.case_insensitive_fs,
        ));
        used_keys.insert(normalized_path_key(
            &row.dst_path,
            policy.case_insensitive_fs,
        ));
    }

    // Descascamento topológico: itens cujo destino não colide com nenhuma origem pendente saem direto.
    loop {
        let active_sources: HashSet<String> = remaining
            .iter()
            .map(|r| normalized_path_key(&r.src_path, policy.case_insensitive_fs))
            .collect();

        let mut progressed = false;
        let mut next_remaining = Vec::with_capacity(remaining.len());

        for row in remaining {
            let src_key = normalized_path_key(&row.src_path, policy.case_insensitive_fs);
            let dst_key = normalized_path_key(&row.dst_path, policy.case_insensitive_fs);
            let is_case_only_self = policy.case_insensitive_fs && src_key == dst_key;

            if !is_case_only_self && !active_sources.contains(&dst_key) {
                direct_steps.push(RenameStep {
                    from: row.src_path.clone(),
                    to: row.dst_path.clone(),
                    input_index: row.input_index,
                });
                progressed = true;
            } else {
                next_remaining.push(row);
            }
        }

        remaining = next_remaining;
        if !progressed || remaining.is_empty() {
            break;
        }
    }

    // Itens restantes pertencem a ciclos puros ou renomeação apenas de caixa no mesmo caminho.
    let mut phase1_temp_steps = Vec::with_capacity(remaining.len());
    let mut phase2_final_steps = Vec::with_capacity(remaining.len());
    let mut rollback_temp_steps = Vec::with_capacity(remaining.len());

    let mut temp_seq = 0usize;
    for row in remaining {
        let temp_path = allocate_unique_temp_path(
            &row.parent_dir,
            batch_id,
            &mut temp_seq,
            &mut used_keys,
            policy.case_insensitive_fs,
        );
        phase1_temp_steps.push(RenameStep {
            from: row.src_path.clone(),
            to: temp_path.clone(),
            input_index: row.input_index,
        });
        phase2_final_steps.push(RenameStep {
            from: temp_path.clone(),
            to: row.dst_path.clone(),
            input_index: row.input_index,
        });
        rollback_temp_steps.push(RenameStep {
            from: temp_path,
            to: row.src_path.clone(),
            input_index: row.input_index,
        });
    }

    Some(BulkRenameExecutionPlan {
        direct_steps,
        phase1_temp_steps,
        phase2_final_steps,
        rollback_temp_steps,
    })
}

fn allocate_unique_temp_path(
    parent_dir: &Path,
    batch_id: u64,
    seq: &mut usize,
    used_keys: &mut HashSet<String>,
    case_insensitive_fs: bool,
) -> PathBuf {
    loop {
        let current = *seq;
        *seq += 1;
        let candidate = parent_dir.join(format!(".__vfm_br_{batch_id}_{current}__.tmp"));
        let key = normalized_path_key(&candidate, case_insensitive_fs);
        if used_keys.insert(key) {
            return candidate;
        }
    }
}

fn compile_optional_regex(rules: &BulkRenameRules) -> (Option<regex::Regex>, Option<String>) {
    if !rules.replace_enabled || !rules.use_regex || rules.find_text.is_empty() {
        return (None, None);
    }

    let mut builder = RegexBuilder::new(&rules.find_text);
    builder
        .case_insensitive(!rules.replace_case_sensitive)
        .size_limit(MAX_REGEX_SIZE_BYTES);

    match builder.build() {
        Ok(compiled) => (Some(compiled), None),
        Err(err) => (None, Some(err.to_string())),
    }
}

fn sort_input_indices(indices: &mut [usize], inputs: &[BulkRenameInput], sort: BulkRenameSort) {
    match sort {
        BulkRenameSort::SelectionOrder => {}
        BulkRenameSort::NameAsc => indices.sort_by(|&a, &b| {
            inputs[a]
                .original_name
                .to_string_lossy()
                .to_lowercase()
                .cmp(&inputs[b].original_name.to_string_lossy().to_lowercase())
        }),
        BulkRenameSort::NameDesc => indices.sort_by(|&a, &b| {
            inputs[b]
                .original_name
                .to_string_lossy()
                .to_lowercase()
                .cmp(&inputs[a].original_name.to_string_lossy().to_lowercase())
        }),
        BulkRenameSort::ModifiedDesc => indices.sort_by(|&a, &b| {
            inputs[b]
                .modified_secs
                .cmp(&inputs[a].modified_secs)
                .then_with(|| a.cmp(&b))
        }),
        BulkRenameSort::ModifiedAsc => indices.sort_by(|&a, &b| {
            inputs[a]
                .modified_secs
                .cmp(&inputs[b].modified_secs)
                .then_with(|| a.cmp(&b))
        }),
        BulkRenameSort::SizeAsc => indices.sort_by(|&a, &b| {
            inputs[a]
                .size_bytes
                .cmp(&inputs[b].size_bytes)
                .then_with(|| a.cmp(&b))
        }),
        BulkRenameSort::SizeDesc => indices.sort_by(|&a, &b| {
            inputs[b]
                .size_bytes
                .cmp(&inputs[a].size_bytes)
                .then_with(|| a.cmp(&b))
        }),
    }
}

fn transform_entry_name(
    original_name: &OsString,
    is_dir: bool,
    seq_step: usize,
    tags: Option<&BulkRenameMediaTags>,
    rules: &BulkRenameRules,
    compiled_regex: Option<&regex::Regex>,
    policy: BulkRenameTargetPolicy,
) -> OsString {
    if !has_any_active_transformation(rules) {
        return original_name.clone();
    }

    let lossy = original_name.to_string_lossy();
    let (mut stem, mut ext) = split_stem_and_ext(lossy.as_ref(), is_dir);

    // 1. Composição por metadados EXIF / ID3
    if rules.media_enabled && !rules.media_fields.is_empty() {
        let composed = compose_media_prefix(tags, rules);
        if !composed.is_empty() {
            match rules.media_mode {
                MediaComposeMode::ReplaceName => stem = composed,
                MediaComposeMode::PrependToName => {
                    if stem.is_empty() {
                        stem = composed;
                    } else {
                        stem = format!("{composed}{}{stem}", rules.media_separator);
                    }
                }
                MediaComposeMode::AppendToName => {
                    if stem.is_empty() {
                        stem = composed;
                    } else {
                        stem = format!("{stem}{}{composed}", rules.media_separator);
                    }
                }
            }
        }
    }

    // 2. Remover caracteres / trechos
    if rules.remove_enabled {
        stem = apply_remove_rules(stem, rules);
    }

    // 3. Procurar e substituir (texto simples por padrão, regex apenas se ativado)
    if rules.replace_enabled && !rules.find_text.is_empty() {
        if rules.use_regex {
            if let Some(re) = compiled_regex {
                stem = if rules.replace_first_only {
                    re.replace(&stem, rules.replace_with.as_str()).into_owned()
                } else {
                    re.replace_all(&stem, rules.replace_with.as_str())
                        .into_owned()
                };
            }
        } else {
            stem = apply_plain_replace(
                &stem,
                &rules.find_text,
                &rules.replace_with,
                rules.replace_scope,
                rules.replace_case_sensitive,
                rules.replace_first_only,
            );
        }
    }

    // 4. Inserir texto
    if rules.insert_enabled && !rules.insert_text.is_empty() {
        stem = apply_insert_text(&stem, &rules.insert_text, rules.insert_position);
    }

    // 5. Numeração sequencial
    if rules.number_enabled {
        let formatted = format_sequence_number(seq_step, rules);
        stem = match rules.number_position {
            NumberPosition::Append => {
                if stem.is_empty() {
                    formatted
                } else {
                    format!("{stem}{}{formatted}", rules.number_separator)
                }
            }
            NumberPosition::Prepend => {
                if stem.is_empty() {
                    formatted
                } else {
                    format!("{formatted}{}{stem}", rules.number_separator)
                }
            }
            NumberPosition::ReplaceName => formatted,
        };
    }

    // 6. Transformação de maiúsculas/minúsculas
    if rules.case_enabled {
        stem = apply_text_case(&stem, rules.case_mode);
    }

    // 7. Regra de extensão (apenas para arquivos)
    if !is_dir && rules.ext_enabled {
        ext = apply_extension_rule(ext, &rules.ext_mode);
    }

    // 8. Sanitização opcional
    if rules.sanitize_invalid_chars {
        stem = sanitize_component(&stem, policy.windows_name_rules);
        ext = ext.map(|e| sanitize_component(&e, policy.windows_name_rules));
    }

    let final_name = match ext {
        Some(e) if !e.is_empty() => format!("{stem}.{e}"),
        _ => stem,
    };

    OsString::from(final_name)
}

fn has_any_active_transformation(rules: &BulkRenameRules) -> bool {
    rules.media_enabled
        || rules.remove_enabled
        || rules.replace_enabled
        || rules.insert_enabled
        || rules.number_enabled
        || (rules.case_enabled && rules.case_mode != TextCaseMode::Keep)
        || (rules.ext_enabled && rules.ext_mode != ExtensionMode::Keep)
}

fn split_stem_and_ext(name: &str, is_dir: bool) -> (String, Option<String>) {
    if is_dir {
        return (name.to_owned(), None);
    }
    if let Some((stem, ext)) = name.rsplit_once('.')
        && !stem.is_empty()
        && !ext.is_empty()
    {
        return (stem.to_owned(), Some(ext.to_owned()));
    }
    (name.to_owned(), None)
}

fn compose_media_prefix(tags: Option<&BulkRenameMediaTags>, rules: &BulkRenameRules) -> String {
    let Some(tags) = tags else {
        return String::new();
    };
    let track_digits = usize::from(rules.media_track_digits.clamp(1, 4));
    let parts: Vec<String> = rules
        .media_fields
        .iter()
        .filter_map(|field| match field {
            MediaTagKind::ExifDateYmd => tags.exif_date_ymd.as_ref().map(|s| s.to_string()),
            MediaTagKind::ExifDateTime => tags.exif_date_taken.as_ref().map(|s| s.to_string()),
            MediaTagKind::ExifCamera => tags.exif_camera_model.as_ref().map(|s| s.to_string()),
            MediaTagKind::Id3Track => tags
                .id3_track
                .map(|track| format!("{track:0width$}", width = track_digits)),
            MediaTagKind::Id3Artist => tags.id3_artist.as_ref().map(|s| s.to_string()),
            MediaTagKind::Id3Album => tags.id3_album.as_ref().map(|s| s.to_string()),
            MediaTagKind::Id3Title => tags.id3_title.as_ref().map(|s| s.to_string()),
            MediaTagKind::Id3Year => tags.id3_year.as_ref().map(|s| s.to_string()),
        })
        .filter(|part| !part.trim().is_empty())
        .collect();

    parts.join(&rules.media_separator)
}

fn apply_remove_rules(mut stem: String, rules: &BulkRenameRules) -> String {
    if rules.remove_first_n > 0 {
        stem = stem.chars().skip(rules.remove_first_n).collect();
    }
    if rules.remove_last_n > 0 {
        let len = stem.chars().count();
        let keep = len.saturating_sub(rules.remove_last_n);
        stem = stem.chars().take(keep).collect();
    }
    if !rules.remove_text.is_empty() {
        stem = apply_plain_replace(
            &stem,
            &rules.remove_text,
            "",
            rules.remove_scope,
            rules.remove_case_sensitive,
            false,
        );
    }
    if rules.remove_digits {
        stem.retain(|c| !c.is_ascii_digit());
    }
    if rules.remove_symbols {
        stem.retain(|c| c.is_alphanumeric() || c.is_whitespace() || c == '-' || c == '_');
    }
    if rules.collapse_spaces {
        let mut normalized = String::with_capacity(stem.len());
        let mut prev_space = false;
        for ch in stem.trim().chars() {
            if ch.is_whitespace() {
                if !prev_space {
                    normalized.push(' ');
                    prev_space = true;
                }
            } else {
                normalized.push(ch);
                prev_space = false;
            }
        }
        stem = normalized;
    }
    stem
}

fn apply_plain_replace(
    input: &str,
    find: &str,
    replace_with: &str,
    scope: MatchScope,
    case_sensitive: bool,
    first_only: bool,
) -> String {
    if find.is_empty() {
        return input.to_owned();
    }

    match scope {
        MatchScope::StartOfName => {
            if let Some(byte_end) = match_prefix_len(input, find, case_sensitive) {
                return format!("{replace_with}{}", &input[byte_end..]);
            }
            input.to_owned()
        }
        MatchScope::EndOfName => {
            if let Some(byte_start) = match_suffix_start(input, find, case_sensitive) {
                return format!("{}{replace_with}", &input[..byte_start]);
            }
            input.to_owned()
        }
        MatchScope::Anywhere
        | MatchScope::WordStart
        | MatchScope::WordEnd
        | MatchScope::WholeWord => {
            let mut out = String::with_capacity(input.len());
            let mut cursor = 0usize;
            let mut replaced = 0usize;

            while cursor < input.len() {
                if first_only && replaced > 0 {
                    out.push_str(&input[cursor..]);
                    break;
                }
                if let Some(match_len) = match_at_start(&input[cursor..], find, case_sensitive) {
                    let match_end = cursor + match_len;
                    let left_ok = match scope {
                        MatchScope::Anywhere | MatchScope::WordEnd => true,
                        MatchScope::WordStart | MatchScope::WholeWord => {
                            is_word_boundary_before(input, cursor)
                        }
                        MatchScope::StartOfName | MatchScope::EndOfName => unreachable!(),
                    };
                    let right_ok = match scope {
                        MatchScope::Anywhere | MatchScope::WordStart => true,
                        MatchScope::WordEnd | MatchScope::WholeWord => {
                            is_word_boundary_after(input, match_end)
                        }
                        MatchScope::StartOfName | MatchScope::EndOfName => unreachable!(),
                    };
                    if left_ok && right_ok {
                        out.push_str(replace_with);
                        cursor = match_end;
                        replaced += 1;
                        continue;
                    }
                }
                if let Some(ch) = input[cursor..].chars().next() {
                    out.push(ch);
                    cursor += ch.len_utf8();
                } else {
                    break;
                }
            }
            out
        }
    }
}

fn match_at_start(slice: &str, needle: &str, case_sensitive: bool) -> Option<usize> {
    if case_sensitive {
        return slice.starts_with(needle).then_some(needle.len());
    }
    let needle_chars = needle.chars().count();
    let mut byte_len = 0usize;
    let mut prefix = String::new();
    for (idx, ch) in slice.chars().enumerate() {
        if idx >= needle_chars {
            break;
        }
        prefix.push(ch);
        byte_len += ch.len_utf8();
    }
    if prefix.chars().count() == needle_chars && prefix.to_lowercase() == needle.to_lowercase() {
        Some(byte_len)
    } else {
        None
    }
}

fn match_prefix_len(input: &str, needle: &str, case_sensitive: bool) -> Option<usize> {
    match_at_start(input, needle, case_sensitive)
}

fn match_suffix_start(input: &str, needle: &str, case_sensitive: bool) -> Option<usize> {
    if case_sensitive {
        return input
            .ends_with(needle)
            .then(|| input.len().saturating_sub(needle.len()));
    }
    let needle_chars = needle.chars().count();
    let total_chars = input.chars().count();
    if needle_chars > total_chars {
        return None;
    }
    let start_char_idx = total_chars - needle_chars;
    let byte_start = input
        .char_indices()
        .nth(start_char_idx)
        .map_or(input.len(), |(idx, _)| idx);
    if input[byte_start..].to_lowercase() == needle.to_lowercase() {
        Some(byte_start)
    } else {
        None
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric()
}

fn is_word_boundary_before(input: &str, byte_idx: usize) -> bool {
    input[..byte_idx]
        .chars()
        .next_back()
        .is_none_or(|prev| !is_word_char(prev))
}

fn is_word_boundary_after(input: &str, byte_idx: usize) -> bool {
    input[byte_idx..]
        .chars()
        .next()
        .is_none_or(|next| !is_word_char(next))
}

fn apply_insert_text(stem: &str, text_to_insert: &str, position: InsertPosition) -> String {
    match position {
        InsertPosition::StartOfName => format!("{text_to_insert}{stem}"),
        InsertPosition::EndOfName => format!("{stem}{text_to_insert}"),
        InsertPosition::FromStart(offset) => {
            let char_count = stem.chars().count();
            let clamped = offset.min(char_count);
            let split_byte = stem
                .char_indices()
                .nth(clamped)
                .map_or(stem.len(), |(i, _)| i);
            format!(
                "{}{text_to_insert}{}",
                &stem[..split_byte],
                &stem[split_byte..]
            )
        }
        InsertPosition::FromEnd(offset) => {
            let char_count = stem.chars().count();
            let clamped = char_count.saturating_sub(offset);
            let split_byte = stem
                .char_indices()
                .nth(clamped)
                .map_or(stem.len(), |(i, _)| i);
            format!(
                "{}{text_to_insert}{}",
                &stem[..split_byte],
                &stem[split_byte..]
            )
        }
    }
}

fn format_sequence_number(seq_step: usize, rules: &BulkRenameRules) -> String {
    let offset = i64::try_from(seq_step)
        .unwrap_or(i64::MAX)
        .saturating_mul(rules.number_step);
    let value = rules.number_start.saturating_add(offset);
    let width = usize::from(rules.number_digits.clamp(1, 6));
    if value < 0 {
        let abs = value.unsigned_abs();
        format!("-{abs:0width$}")
    } else {
        format!("{value:0width$}")
    }
}

fn apply_text_case(stem: &str, mode: TextCaseMode) -> String {
    match mode {
        TextCaseMode::Keep => stem.to_owned(),
        TextCaseMode::Uppercase => stem.to_uppercase(),
        TextCaseMode::Lowercase => stem.to_lowercase(),
        TextCaseMode::TitleCaseWords => {
            let mut out = String::with_capacity(stem.len());
            let mut at_word_start = true;
            for ch in stem.chars() {
                if !ch.is_alphanumeric() {
                    out.push(ch);
                    at_word_start = true;
                } else if at_word_start {
                    let _ = write!(out, "{}", ch.to_uppercase());
                    at_word_start = false;
                } else {
                    let _ = write!(out, "{}", ch.to_lowercase());
                }
            }
            out
        }
        TextCaseMode::SentenceCaseFirst => {
            let mut out = String::with_capacity(stem.len());
            let mut capitalized = false;
            for ch in stem.chars() {
                if !capitalized && ch.is_alphabetic() {
                    let _ = write!(out, "{}", ch.to_uppercase());
                    capitalized = true;
                } else {
                    let _ = write!(out, "{}", ch.to_lowercase());
                }
            }
            out
        }
    }
}

fn apply_extension_rule(ext: Option<String>, mode: &ExtensionMode) -> Option<String> {
    match mode {
        ExtensionMode::Keep => ext,
        ExtensionMode::Lowercase => ext.map(|e| e.to_lowercase()),
        ExtensionMode::Uppercase => ext.map(|e| e.to_uppercase()),
        ExtensionMode::Replace(new_ext) => {
            let trimmed = new_ext.trim().trim_start_matches('.');
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            }
        }
        ExtensionMode::Remove => None,
    }
}

fn sanitize_component(component: &str, windows_rules: bool) -> String {
    component
        .chars()
        .filter(|&ch| {
            if ch == '\0' || ch == '/' {
                return false;
            }
            if windows_rules && matches!(ch, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                return false;
            }
            true
        })
        .collect()
}

fn validate_file_name(name: &str, windows_rules: bool) -> Result<(), BulkRenameNameError> {
    let trimmed = if windows_rules {
        name.trim_end_matches([' ', '.'])
    } else {
        name
    };
    if name.trim().is_empty() || trimmed.is_empty() {
        return Err(BulkRenameNameError::Empty);
    }
    if name == "." || name == ".." {
        return Err(BulkRenameNameError::ReservedDot);
    }
    if name.contains(['/', '\\']) {
        return Err(BulkRenameNameError::ContainsPathSeparator);
    }
    if name.contains('\0') {
        return Err(BulkRenameNameError::WindowsInvalidChar);
    }
    if windows_rules {
        if name
            .chars()
            .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*') || ch.is_control())
        {
            return Err(BulkRenameNameError::WindowsInvalidChar);
        }
        let stem = name.split('.').next().unwrap_or(name);
        let upper = stem.to_ascii_uppercase();
        if matches!(
            upper.as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        ) {
            return Err(BulkRenameNameError::WindowsReservedName);
        }
    }
    if name.len() > 255 {
        return Err(BulkRenameNameError::TooLong);
    }
    Ok(())
}

fn mark_conflicts(
    rows: &mut [BulkRenamePreviewRow],
    existing_names_in_dirs: &HashSet<PathBuf>,
    case_insensitive_fs: bool,
    ready_count: &mut usize,
) -> usize {
    let existing_keys: HashSet<String> = existing_names_in_dirs
        .iter()
        .map(|p| normalized_path_key(p, case_insensitive_fs))
        .collect();

    // Origens de itens que efetivamente mudarão de caminho/nome saem do caminho na execução.
    let moving_source_keys: HashSet<String> = rows
        .iter()
        .filter(|r| r.status == BulkRenameRowStatus::Ready)
        .map(|r| normalized_path_key(&r.src_path, case_insensitive_fs))
        .collect();

    // Conta quantos itens do lote (prontos ou inalterados) reivindicam cada destino.
    let mut claimed_dst_counts: HashMap<String, usize> = HashMap::with_capacity(rows.len());
    for row in rows.iter() {
        if matches!(
            row.status,
            BulkRenameRowStatus::Ready
                | BulkRenameRowStatus::Unchanged
                | BulkRenameRowStatus::SkippedFolder
        ) {
            let key = normalized_path_key(&row.dst_path, case_insensitive_fs);
            *claimed_dst_counts.entry(key).or_default() += 1;
        }
    }

    let mut conflict_count = 0usize;
    for row in rows.iter_mut() {
        if row.status != BulkRenameRowStatus::Ready {
            continue;
        }
        let dst_key = normalized_path_key(&row.dst_path, case_insensitive_fs);
        let src_key = normalized_path_key(&row.src_path, case_insensitive_fs);

        let claims = claimed_dst_counts.get(&dst_key).copied().unwrap_or(0);
        if claims > 1 {
            row.status = BulkRenameRowStatus::ConflictInternal;
            *ready_count = ready_count.saturating_sub(1);
            conflict_count += 1;
            continue;
        }

        // Caso diferente apenas em maiúscula/minúscula no mesmo caminho não colide com ele mesmo no disco.
        let is_same_case_insensitive_path = case_insensitive_fs && src_key == dst_key;
        if !is_same_case_insensitive_path
            && existing_keys.contains(&dst_key)
            && !moving_source_keys.contains(&dst_key)
        {
            row.status = BulkRenameRowStatus::ConflictExternal;
            *ready_count = ready_count.saturating_sub(1);
            conflict_count += 1;
        }
    }

    conflict_count
}

fn normalized_path_key(path: &Path, case_insensitive_fs: bool) -> String {
    let raw = path.to_string_lossy();
    if case_insensitive_fs {
        raw.to_lowercase()
    } else {
        raw.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_input(id: u64, name: &str, is_dir: bool) -> BulkRenameInput {
        let parent = PathBuf::from("/tmp/album");
        BulkRenameInput {
            entry_id: EntryId(id),
            path: parent.join(name),
            parent_dir: parent,
            original_name: OsString::from(name),
            is_dir,
            size_bytes: id * 100,
            modified_secs: Some(i64::try_from(id).unwrap_or(0)),
        }
    }

    #[test]
    fn plain_replace_supports_word_boundaries_and_name_positions() {
        let inputs = vec![
            make_input(1, "foto ferias praia.jpg", false),
            make_input(2, "ferias_foto_antiga.jpg", false),
        ];
        let rules = BulkRenameRules {
            replace_enabled: true,
            find_text: "foto".to_owned(),
            replace_with: "img".to_owned(),
            replace_scope: MatchScope::StartOfName,
            ..Default::default()
        };
        let eval = evaluate_bulk_rename(
            &inputs,
            &[],
            &rules,
            &HashSet::new(),
            BulkRenameTargetPolicy::host_default(),
        );
        assert_eq!(
            eval.rows[0].new_name,
            OsString::from("img ferias praia.jpg")
        );
        assert_eq!(
            eval.rows[1].new_name,
            OsString::from("ferias_foto_antiga.jpg")
        );

        // Somente início da palavra (altera "foto", "fotografia" -> "foto..." sem afetar "autofoto")
        let word_inputs = vec![
            make_input(1, "foto autofoto.jpg", false),
            make_input(2, "minha_foto.jpg", false),
        ];
        let word_rules = BulkRenameRules {
            replace_enabled: true,
            find_text: "foto".to_owned(),
            replace_with: "pic".to_owned(),
            replace_scope: MatchScope::WholeWord,
            ..Default::default()
        };
        let word_eval = evaluate_bulk_rename(
            &word_inputs,
            &[],
            &word_rules,
            &HashSet::new(),
            BulkRenameTargetPolicy::host_default(),
        );
        assert_eq!(
            word_eval.rows[0].new_name,
            OsString::from("pic autofoto.jpg")
        );
        // Sublinhado `_` é considerado alfanumérico ou pontuação? Em Rust `'_'` NÃO é alfanumérico,
        // portanto separa palavras amigavelmente em nomes de arquivo como `minha_foto.jpg`.
        assert_eq!(word_eval.rows[1].new_name, OsString::from("minha_pic.jpg"));
    }

    #[test]
    fn remove_insert_case_numbering_and_extension_pipeline() {
        let inputs = vec![
            make_input(1, "00_minha musica copia.MP3", false),
            make_input(2, "01_outra faixa copia.WAV", false),
        ];
        let rules = BulkRenameRules {
            remove_enabled: true,
            remove_first_n: 3,
            remove_text: " copia".to_owned(),
            remove_scope: MatchScope::EndOfName,
            insert_enabled: true,
            insert_text: "Rock - ".to_owned(),
            insert_position: InsertPosition::StartOfName,
            number_enabled: true,
            number_position: NumberPosition::Prepend,
            number_separator: " ".to_owned(),
            number_start: 1,
            number_step: 1,
            number_digits: 2,
            case_enabled: true,
            case_mode: TextCaseMode::TitleCaseWords,
            ext_enabled: true,
            ext_mode: ExtensionMode::Lowercase,
            ..Default::default()
        };

        let eval = evaluate_bulk_rename(
            &inputs,
            &[],
            &rules,
            &HashSet::new(),
            BulkRenameTargetPolicy::host_default(),
        );
        assert!(eval.can_apply());
        assert_eq!(
            eval.rows[0].new_name,
            OsString::from("01 Rock - Minha Musica.mp3")
        );
        assert_eq!(
            eval.rows[1].new_name,
            OsString::from("02 Rock - Outra Faixa.wav")
        );
    }

    #[test]
    fn exif_and_id3_media_composition_skips_missing_tags_cleanly() {
        let inputs = vec![
            make_input(1, "DSC001.JPG", false),
            make_input(2, "track.mp3", false),
        ];
        let tags = vec![
            Some(BulkRenameMediaTags {
                exif_date_ymd: Some(CompactString::from("2026-08-14")),
                exif_camera_model: Some(CompactString::from("Sony A7")),
                ..Default::default()
            }),
            None,
        ];
        let rules = BulkRenameRules {
            media_enabled: true,
            media_fields: vec![MediaTagKind::ExifDateYmd, MediaTagKind::ExifCamera],
            media_mode: MediaComposeMode::PrependToName,
            media_separator: " - ".to_owned(),
            ext_enabled: true,
            ext_mode: ExtensionMode::Lowercase,
            ..Default::default()
        };

        let eval = evaluate_bulk_rename(
            &inputs,
            &tags,
            &rules,
            &HashSet::new(),
            BulkRenameTargetPolicy::host_default(),
        );
        assert_eq!(
            eval.rows[0].new_name,
            OsString::from("2026-08-14 - Sony A7 - DSC001.jpg")
        );
        assert_eq!(eval.rows[1].new_name, OsString::from("track.mp3"));
    }

    #[test]
    fn swap_cycle_and_case_only_windows_rename_use_two_phase_plan() {
        let policy = BulkRenameTargetPolicy {
            case_insensitive_fs: true,
            windows_name_rules: true,
        };
        // Ciclo A <-> B + item case-only C -> C_upper
        let inputs = vec![
            make_input(1, "a.txt", false),
            make_input(2, "b.txt", false),
            make_input(3, "c.txt", false),
        ];
        let rules = BulkRenameRules {
            replace_enabled: true,
            find_text: "a".to_owned(),
            replace_with: "b".to_owned(),
            replace_scope: MatchScope::StartOfName,
            case_enabled: true,
            case_mode: TextCaseMode::Uppercase,
            ..Default::default()
        };
        let eval = evaluate_bulk_rename(&inputs, &[], &rules, &HashSet::new(), policy);
        // `a.txt` -> `B.txt` e `b.txt` -> `B.txt` colidem; testemos um ciclo real com substituição regex controlada:
        assert_eq!(eval.conflict_count, 2);

        let swap_inputs = [
            make_input(1, "01.txt", false),
            make_input(2, "02.txt", false),
        ];

        let manual_eval = BulkRenameEvaluation {
            rows: vec![
                BulkRenamePreviewRow {
                    input_index: 0,
                    sequence_pos: 1,
                    entry_id: swap_inputs[0].entry_id,
                    src_path: swap_inputs[0].path.clone(),
                    parent_dir: swap_inputs[0].parent_dir.clone(),
                    original_name: OsString::from("01.txt"),
                    new_name: OsString::from("02.txt"),
                    dst_path: swap_inputs[0].parent_dir.join("02.txt"),
                    is_dir: false,
                    status: BulkRenameRowStatus::Ready,
                },
                BulkRenamePreviewRow {
                    input_index: 1,
                    sequence_pos: 2,
                    entry_id: swap_inputs[1].entry_id,
                    src_path: swap_inputs[1].path.clone(),
                    parent_dir: swap_inputs[1].parent_dir.clone(),
                    original_name: OsString::from("02.txt"),
                    new_name: OsString::from("01.txt"),
                    dst_path: swap_inputs[1].parent_dir.join("01.txt"),
                    is_dir: false,
                    status: BulkRenameRowStatus::Ready,
                },
                BulkRenamePreviewRow {
                    input_index: 2,
                    sequence_pos: 3,
                    entry_id: inputs[2].entry_id,
                    src_path: inputs[2].path.clone(),
                    parent_dir: inputs[2].parent_dir.clone(),
                    original_name: OsString::from("c.txt"),
                    new_name: OsString::from("C.txt"),
                    dst_path: inputs[2].parent_dir.join("C.txt"),
                    is_dir: false,
                    status: BulkRenameRowStatus::Ready,
                },
            ],
            regex_error: None,
            ready_count: 3,
            unchanged_count: 0,
            skipped_count: 0,
            conflict_count: 0,
            invalid_count: 0,
        };

        let plan =
            build_execution_plan(&manual_eval, &HashSet::new(), 7, policy).expect("plano válido");
        assert!(plan.direct_steps.is_empty());
        assert_eq!(plan.phase1_temp_steps.len(), 3);
        assert_eq!(plan.phase2_final_steps.len(), 3);
        assert_eq!(plan.rollback_temp_steps.len(), 3);
    }

    #[test]
    fn chain_renames_peel_topologically_without_temp_files() {
        // 01 -> 02, 02 -> 03 (cadeia sem ciclo): precisa renomear 02 -> 03 ANTES de 01 -> 02
        let parent = PathBuf::from("/tmp/album");
        let eval = BulkRenameEvaluation {
            rows: vec![
                BulkRenamePreviewRow {
                    input_index: 0,
                    sequence_pos: 1,
                    entry_id: EntryId(1),
                    src_path: parent.join("01.txt"),
                    parent_dir: parent.clone(),
                    original_name: OsString::from("01.txt"),
                    new_name: OsString::from("02.txt"),
                    dst_path: parent.join("02.txt"),
                    is_dir: false,
                    status: BulkRenameRowStatus::Ready,
                },
                BulkRenamePreviewRow {
                    input_index: 1,
                    sequence_pos: 2,
                    entry_id: EntryId(2),
                    src_path: parent.join("02.txt"),
                    parent_dir: parent.clone(),
                    original_name: OsString::from("02.txt"),
                    new_name: OsString::from("03.txt"),
                    dst_path: parent.join("03.txt"),
                    is_dir: false,
                    status: BulkRenameRowStatus::Ready,
                },
            ],
            regex_error: None,
            ready_count: 2,
            unchanged_count: 0,
            skipped_count: 0,
            conflict_count: 0,
            invalid_count: 0,
        };

        let plan = build_execution_plan(
            &eval,
            &HashSet::new(),
            1,
            BulkRenameTargetPolicy::host_default(),
        )
        .expect("plano válido");
        assert!(plan.phase1_temp_steps.is_empty());
        assert_eq!(plan.direct_steps.len(), 2);
        assert_eq!(plan.direct_steps[0].from, parent.join("02.txt"));
        assert_eq!(plan.direct_steps[0].to, parent.join("03.txt"));
        assert_eq!(plan.direct_steps[1].from, parent.join("01.txt"));
        assert_eq!(plan.direct_steps[1].to, parent.join("02.txt"));
    }

    #[test]
    fn detects_external_and_case_insensitive_disk_collisions() {
        let inputs = vec![make_input(1, "foto.jpg", false)];
        let mut existing = HashSet::new();
        existing.insert(PathBuf::from("/tmp/album/FERIAS.JPG"));

        let rules = BulkRenameRules {
            replace_enabled: true,
            find_text: "foto".to_owned(),
            replace_with: "ferias".to_owned(),
            ..Default::default()
        };

        let eval = evaluate_bulk_rename(
            &inputs,
            &[],
            &rules,
            &existing,
            BulkRenameTargetPolicy {
                case_insensitive_fs: true,
                windows_name_rules: true,
            },
        );
        assert_eq!(eval.conflict_count, 1);
        assert_eq!(eval.rows[0].status, BulkRenameRowStatus::ConflictExternal);
        assert!(!eval.can_apply());
    }

    #[test]
    fn optional_advanced_regex_supports_capture_groups_when_enabled() {
        let inputs = [make_input(1, "IMG_2026_09.jpg", false)];
        let rules = BulkRenameRules {
            replace_enabled: true,
            use_regex: true,
            find_text: r"^IMG_(\d+)_(\d+)$".to_owned(),
            replace_with: "Viagem_${1}_${2}".to_owned(),
            ..Default::default()
        };
        let eval = evaluate_bulk_rename(
            &inputs,
            &[],
            &rules,
            &HashSet::new(),
            BulkRenameTargetPolicy::host_default(),
        );
        assert!(eval.regex_error.is_none());
        assert_eq!(eval.rows[0].new_name, OsString::from("Viagem_2026_09.jpg"));
    }

    #[test]
    fn invalid_regex_reports_clean_error_without_panic() {
        let inputs = vec![make_input(1, "arquivo.txt", false)];
        let rules = BulkRenameRules {
            replace_enabled: true,
            use_regex: true,
            find_text: "[unclosed".to_owned(),
            replace_with: "x".to_owned(),
            ..Default::default()
        };
        let eval = evaluate_bulk_rename(
            &inputs,
            &[],
            &rules,
            &HashSet::new(),
            BulkRenameTargetPolicy::host_default(),
        );
        assert!(eval.regex_error.is_some());
        assert!(!eval.can_apply());
    }
}
