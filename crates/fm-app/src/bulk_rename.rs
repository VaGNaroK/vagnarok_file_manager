use std::collections::HashSet;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

use fm_core::{
    BulkRenameEvaluation, BulkRenameInput, BulkRenameMediaTags, BulkRenameRules, BulkRenameSort,
    BulkRenameTargetPolicy, ExtensionMode, FileEntry, InsertPosition, MatchScope, MediaComposeMode,
    MediaTagKind, NumberPosition, TextCaseMode, build_execution_plan, evaluate_bulk_rename,
    is_virtual_trash_path,
};
use fm_fs::BulkRenameBatchOutcome;
use fm_telemetry::media_tags::{ExtractedMediaTagItem, extract_media_tags_stream};
use iced::futures::SinkExt;
use iced::{Point, Size, Task, stream};
use tokio_util::sync::CancellationToken;

use crate::app::{App, Message};

pub const BULK_RENAME_DEFAULT_W: f32 = 980.0;
pub const BULK_RENAME_DEFAULT_H: f32 = 660.0;
pub const BULK_RENAME_MIN_W: f32 = 740.0;
pub const BULK_RENAME_MIN_H: f32 = 500.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkRenameInsertKind {
    StartOfName,
    EndOfName,
    FromStart,
    FromEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulkRenameExtensionKind {
    Keep,
    Lowercase,
    Uppercase,
    Replace,
    Remove,
}

#[derive(Debug, Clone)]
pub enum BulkRenameMsg {
    Close,
    ClearRules,
    DragStart,
    ResizeStart,
    PreviewScrolled {
        offset_y: f32,
        viewport_height: f32,
    },
    ToggleIncludeFolders,
    SetSort(BulkRenameSort),
    // Procurar & Substituir
    ToggleReplace,
    FindChanged(String),
    ReplaceWithChanged(String),
    SetReplaceScope(MatchScope),
    ToggleReplaceCaseSensitive,
    ToggleReplaceFirstOnly,
    ToggleAdvancedRegexPanel,
    ToggleUseRegex,
    // Remover
    ToggleRemove,
    RemoveFirstChanged(String),
    RemoveLastChanged(String),
    RemoveTextChanged(String),
    SetRemoveScope(MatchScope),
    ToggleRemoveDigits,
    ToggleRemoveSymbols,
    ToggleCollapseSpaces,
    // Inserir
    ToggleInsert,
    InsertTextChanged(String),
    SetInsertPositionKind(BulkRenameInsertKind),
    InsertOffsetChanged(String),
    // Maiúsculas / Minúsculas
    ToggleCase,
    SetCaseMode(TextCaseMode),
    // Numeração Sequencial
    ToggleNumber,
    SetNumberPosition(NumberPosition),
    NumberSeparatorChanged(String),
    NumberStartChanged(String),
    NumberStepChanged(String),
    NumberDigitsChanged(u8),
    // Dados da Foto / Música (EXIF / ID3)
    ToggleMedia,
    ToggleMediaField(MediaTagKind),
    SetMediaMode(MediaComposeMode),
    MediaSeparatorChanged(String),
    MediaTagsBatchLoaded {
        request_id: u64,
        batch: Vec<ExtractedMediaTagItem>,
    },
    MediaTagsFinished {
        request_id: u64,
    },
    // Extensão
    ToggleExt,
    SetExtKind(BulkRenameExtensionKind),
    ExtReplaceChanged(String),
    // Execução
    Apply,
    ApplyFinished(BulkRenameBatchOutcome),
}

#[derive(Debug, Clone)]
pub struct BulkRenameModalState {
    pub inputs: Vec<BulkRenameInput>,
    pub tags_by_input: Vec<Option<BulkRenameMediaTags>>,
    pub existing_names_in_dir: HashSet<PathBuf>,
    pub rules: BulkRenameRules,
    pub eval: BulkRenameEvaluation,
    pub regex_panel_expanded: bool,
    pub applying: bool,
    pub tags_loading: bool,
    pub tags_loaded_count: usize,
    pub tags_fetched_exif: bool,
    pub tags_fetched_id3: bool,
    pub tags_cancel: Option<CancellationToken>,
    pub tags_request_id: u64,
    pub remove_first_draft: String,
    pub remove_last_draft: String,
    pub insert_offset_draft: String,
    pub number_start_draft: String,
    pub number_step_draft: String,
    pub ext_replace_draft: String,
    pub preview_scroll_y: f32,
    pub preview_viewport_height: f32,
    pub modal_size: (f32, f32),
    pub modal_offset: Option<(f32, f32)>,
    pub dragging_modal: Option<(Point, f32, f32)>,
    pub resizing_modal: Option<(Point, f32, f32, f32, f32)>,
}

impl BulkRenameModalState {
    pub fn from_entries(
        selected_entries: &[&FileEntry],
        all_dir_entries: &[FileEntry],
        window_size: Size,
    ) -> Self {
        let mut inputs = Vec::with_capacity(selected_entries.len());
        for entry in selected_entries {
            let parent_dir = entry.path.parent().map(PathBuf::from).unwrap_or_default();
            let modified_secs = entry
                .modified_at
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .and_then(|d| i64::try_from(d.as_secs()).ok());
            inputs.push(BulkRenameInput {
                entry_id: entry.id,
                path: entry.path.clone(),
                parent_dir,
                original_name: entry.name.as_str().into(),
                is_dir: entry.kind.is_dir_like(),
                size_bytes: entry.size_bytes,
                modified_secs,
            });
        }

        let existing_names_in_dir: HashSet<PathBuf> =
            all_dir_entries.iter().map(|e| e.path.clone()).collect();
        let tags_by_input = vec![None; inputs.len()];

        // Inicia com "Procurar e Substituir" aberto e vazio para uso imediato e intuitivo.
        let rules = BulkRenameRules {
            replace_enabled: true,
            ..Default::default()
        };

        let eval = evaluate_bulk_rename(
            &inputs,
            &tags_by_input,
            &rules,
            &existing_names_in_dir,
            BulkRenameTargetPolicy::host_default(),
        );

        let modal_size = clamp_modal_size(
            BULK_RENAME_DEFAULT_W,
            BULK_RENAME_DEFAULT_H,
            BULK_RENAME_MIN_W,
            BULK_RENAME_MIN_H,
            window_size,
        );

        Self {
            inputs,
            tags_by_input,
            existing_names_in_dir,
            rules,
            eval,
            regex_panel_expanded: false,
            applying: false,
            tags_loading: false,
            tags_loaded_count: 0,
            tags_fetched_exif: false,
            tags_fetched_id3: false,
            tags_cancel: None,
            tags_request_id: 0,
            remove_first_draft: "0".to_owned(),
            remove_last_draft: "0".to_owned(),
            insert_offset_draft: "0".to_owned(),
            number_start_draft: "1".to_owned(),
            number_step_draft: "1".to_owned(),
            ext_replace_draft: String::new(),
            preview_scroll_y: 0.0,
            preview_viewport_height: 460.0,
            modal_size,
            modal_offset: None,
            dragging_modal: None,
            resizing_modal: None,
        }
    }

    pub fn recompute_preview(&mut self) {
        self.eval = evaluate_bulk_rename(
            &self.inputs,
            &self.tags_by_input,
            &self.rules,
            &self.existing_names_in_dir,
            BulkRenameTargetPolicy::host_default(),
        );
    }

    fn clear_rules(&mut self) {
        self.rules = BulkRenameRules {
            replace_enabled: true,
            ..Default::default()
        };
        self.regex_panel_expanded = false;
        self.remove_first_draft = "0".to_owned();
        self.remove_last_draft = "0".to_owned();
        self.insert_offset_draft = "0".to_owned();
        self.number_start_draft = "1".to_owned();
        self.number_step_draft = "1".to_owned();
        self.ext_replace_draft.clear();
        self.recompute_preview();
    }
}

/// Retorna `(x, y, width, height)` normalizado dentro da janela principal para o modal de Renomeação em Lote.
pub fn effective_bulk_rename_modal_rect(
    app: &App,
    modal: &BulkRenameModalState,
) -> (f32, f32, f32, f32) {
    effective_modal_rect(
        modal.modal_size,
        modal.modal_offset,
        BULK_RENAME_MIN_W,
        BULK_RENAME_MIN_H,
        app.window_size,
    )
}

pub fn effective_modal_rect(
    raw_size: (f32, f32),
    offset: Option<(f32, f32)>,
    min_w: f32,
    min_h: f32,
    window_size: Size,
) -> (f32, f32, f32, f32) {
    let (w, h) = clamp_modal_size(raw_size.0, raw_size.1, min_w, min_h, window_size);
    let (raw_x, raw_y) = offset.unwrap_or_else(|| {
        (
            ((window_size.width - w) * 0.5).max(8.0),
            ((window_size.height - h) * 0.5).max(8.0),
        )
    });
    let (x, y) = clamp_modal_pos(raw_x, raw_y, w, h, window_size);
    (x, y, w, h)
}

pub fn clamp_modal_pos(x: f32, y: f32, width: f32, height: f32, window_size: Size) -> (f32, f32) {
    let max_x = (window_size.width - width - 8.0).max(8.0);
    let max_y = (window_size.height - height - 8.0).max(8.0);
    (x.clamp(8.0, max_x), y.clamp(8.0, max_y))
}

pub fn clamp_modal_size(
    width: f32,
    height: f32,
    min_w: f32,
    min_h: f32,
    window_size: Size,
) -> (f32, f32) {
    let max_w = (window_size.width - 16.0).max(min_w);
    let max_h = (window_size.height - 16.0).max(min_h);
    (width.clamp(min_w, max_w), height.clamp(min_h, max_h))
}

pub fn compute_dragged_modal_size(
    anchor: (f32, f32),
    initial_size: (f32, f32),
    min_size: (f32, f32),
    start: Point,
    current: Point,
    window_size: Size,
) -> (f32, f32) {
    let max_w = (window_size.width - anchor.0 - 8.0).max(min_size.0);
    let max_h = (window_size.height - anchor.1 - 8.0).max(min_size.1);
    let next_w = (initial_size.0 + (current.x - start.x)).clamp(min_size.0, max_w);
    let next_h = (initial_size.1 + (current.y - start.y)).clamp(min_size.1, max_h);
    (next_w, next_h)
}

/// Abre o modal de Renomeação em Lote para os itens selecionados na aba ativa.
pub fn open_bulk_rename_modal(app: &mut App) -> Task<Message> {
    app.context_menu = None;
    let strings = app.strings();

    let Some(tab) = app.current_tab() else {
        return Task::none();
    };
    if is_virtual_trash_path(&tab.current_path)
        || fm_fs::archive::split_archive_path(&tab.current_path).is_some()
    {
        app.status_notice = Some(strings.br_readonly_error.to_owned());
        return Task::none();
    }

    let selected: Vec<&FileEntry> = tab
        .visible_entries
        .iter()
        .filter(|e| tab.selection.selected_ids.contains(&e.id))
        .collect();
    if selected.is_empty() {
        return Task::none();
    }

    let modal = BulkRenameModalState::from_entries(&selected, &tab.raw_entries, app.window_size);

    if let Some(mut existing) = app.bulk_rename_modal.take()
        && let Some(cancel) = existing.tags_cancel.take()
    {
        cancel.cancel();
    }

    app.bulk_rename_modal = Some(modal);
    Task::none()
}

pub fn handle_bulk_rename_msg(app: &mut App, msg: BulkRenameMsg) -> Task<Message> {
    match msg {
        BulkRenameMsg::Close => {
            if let Some(mut modal) = app.bulk_rename_modal.take()
                && let Some(cancel) = modal.tags_cancel.take()
            {
                cancel.cancel();
            }
            Task::none()
        }
        BulkRenameMsg::ClearRules => {
            if let Some(modal) = &mut app.bulk_rename_modal {
                modal.clear_rules();
            }
            Task::none()
        }
        BulkRenameMsg::DragStart => {
            if let Some(modal) = &mut app.bulk_rename_modal {
                let (x, y, _, _) = effective_modal_rect(
                    modal.modal_size,
                    modal.modal_offset,
                    BULK_RENAME_MIN_W,
                    BULK_RENAME_MIN_H,
                    app.window_size,
                );
                modal.modal_offset = Some((x, y));
                modal.dragging_modal = Some((app.cursor_position, x, y));
            }
            Task::none()
        }
        BulkRenameMsg::ResizeStart => {
            if let Some(modal) = &mut app.bulk_rename_modal {
                let (x, y, w, h) = effective_modal_rect(
                    modal.modal_size,
                    modal.modal_offset,
                    BULK_RENAME_MIN_W,
                    BULK_RENAME_MIN_H,
                    app.window_size,
                );
                modal.modal_offset = Some((x, y));
                modal.resizing_modal = Some((app.cursor_position, x, y, w, h));
            }
            Task::none()
        }
        BulkRenameMsg::PreviewScrolled {
            offset_y,
            viewport_height,
        } => {
            if let Some(modal) = &mut app.bulk_rename_modal {
                modal.preview_scroll_y = offset_y;
                if viewport_height > 0.0 {
                    modal.preview_viewport_height = viewport_height;
                }
            }
            Task::none()
        }
        BulkRenameMsg::ToggleIncludeFolders => mutate_rules(app, |m| {
            m.rules.include_folders = !m.rules.include_folders;
        }),
        BulkRenameMsg::SetSort(sort) => mutate_rules(app, |m| {
            m.rules.sort_order = sort;
        }),
        BulkRenameMsg::ToggleReplace => mutate_rules(app, |m| {
            m.rules.replace_enabled = !m.rules.replace_enabled;
        }),
        BulkRenameMsg::FindChanged(val) => mutate_rules(app, |m| {
            m.rules.replace_enabled = true;
            m.rules.find_text = val;
        }),
        BulkRenameMsg::ReplaceWithChanged(val) => mutate_rules(app, |m| {
            m.rules.replace_enabled = true;
            m.rules.replace_with = val;
        }),
        BulkRenameMsg::SetReplaceScope(scope) => mutate_rules(app, |m| {
            m.rules.replace_scope = scope;
        }),
        BulkRenameMsg::ToggleReplaceCaseSensitive => mutate_rules(app, |m| {
            m.rules.replace_case_sensitive = !m.rules.replace_case_sensitive;
        }),
        BulkRenameMsg::ToggleReplaceFirstOnly => mutate_rules(app, |m| {
            m.rules.replace_first_only = !m.rules.replace_first_only;
        }),
        BulkRenameMsg::ToggleAdvancedRegexPanel => {
            if let Some(modal) = &mut app.bulk_rename_modal {
                modal.regex_panel_expanded = !modal.regex_panel_expanded;
            }
            Task::none()
        }
        BulkRenameMsg::ToggleUseRegex => mutate_rules(app, |m| {
            m.rules.use_regex = !m.rules.use_regex;
        }),
        BulkRenameMsg::ToggleRemove => mutate_rules(app, |m| {
            m.rules.remove_enabled = !m.rules.remove_enabled;
        }),
        BulkRenameMsg::RemoveFirstChanged(val) => mutate_rules(app, |m| {
            m.rules.remove_enabled = true;
            m.remove_first_draft = keep_unsigned_digits(&val);
            m.rules.remove_first_n = m.remove_first_draft.parse::<usize>().unwrap_or(0);
        }),
        BulkRenameMsg::RemoveLastChanged(val) => mutate_rules(app, |m| {
            m.rules.remove_enabled = true;
            m.remove_last_draft = keep_unsigned_digits(&val);
            m.rules.remove_last_n = m.remove_last_draft.parse::<usize>().unwrap_or(0);
        }),
        BulkRenameMsg::RemoveTextChanged(val) => mutate_rules(app, |m| {
            m.rules.remove_enabled = true;
            m.rules.remove_text = val;
        }),
        BulkRenameMsg::SetRemoveScope(scope) => mutate_rules(app, |m| {
            m.rules.remove_scope = scope;
        }),
        BulkRenameMsg::ToggleRemoveDigits => mutate_rules(app, |m| {
            m.rules.remove_digits = !m.rules.remove_digits;
        }),
        BulkRenameMsg::ToggleRemoveSymbols => mutate_rules(app, |m| {
            m.rules.remove_symbols = !m.rules.remove_symbols;
        }),
        BulkRenameMsg::ToggleCollapseSpaces => mutate_rules(app, |m| {
            m.rules.collapse_spaces = !m.rules.collapse_spaces;
        }),
        BulkRenameMsg::ToggleInsert => mutate_rules(app, |m| {
            m.rules.insert_enabled = !m.rules.insert_enabled;
        }),
        BulkRenameMsg::InsertTextChanged(val) => mutate_rules(app, |m| {
            m.rules.insert_enabled = true;
            m.rules.insert_text = val;
        }),
        BulkRenameMsg::SetInsertPositionKind(kind) => mutate_rules(app, |m| {
            let offset = m.insert_offset_draft.parse::<usize>().unwrap_or(0);
            m.rules.insert_position = match kind {
                BulkRenameInsertKind::StartOfName => InsertPosition::StartOfName,
                BulkRenameInsertKind::EndOfName => InsertPosition::EndOfName,
                BulkRenameInsertKind::FromStart => InsertPosition::FromStart(offset),
                BulkRenameInsertKind::FromEnd => InsertPosition::FromEnd(offset),
            };
        }),
        BulkRenameMsg::InsertOffsetChanged(val) => mutate_rules(app, |m| {
            m.insert_offset_draft = keep_unsigned_digits(&val);
            let offset = m.insert_offset_draft.parse::<usize>().unwrap_or(0);
            m.rules.insert_position = match m.rules.insert_position {
                InsertPosition::FromEnd(_) => InsertPosition::FromEnd(offset),
                _ => InsertPosition::FromStart(offset),
            };
        }),
        BulkRenameMsg::ToggleCase => mutate_rules(app, |m| {
            m.rules.case_enabled = !m.rules.case_enabled;
        }),
        BulkRenameMsg::SetCaseMode(mode) => mutate_rules(app, |m| {
            m.rules.case_enabled = true;
            m.rules.case_mode = mode;
        }),
        BulkRenameMsg::ToggleNumber => mutate_rules(app, |m| {
            m.rules.number_enabled = !m.rules.number_enabled;
        }),
        BulkRenameMsg::SetNumberPosition(pos) => mutate_rules(app, |m| {
            m.rules.number_position = pos;
        }),
        BulkRenameMsg::NumberSeparatorChanged(val) => mutate_rules(app, |m| {
            m.rules.number_separator = val;
        }),
        BulkRenameMsg::NumberStartChanged(val) => mutate_rules(app, |m| {
            m.number_start_draft = keep_signed_digits(&val);
            m.rules.number_start = m.number_start_draft.parse::<i64>().unwrap_or(0);
        }),
        BulkRenameMsg::NumberStepChanged(val) => mutate_rules(app, |m| {
            m.number_step_draft = keep_signed_digits(&val);
            m.rules.number_step = m.number_step_draft.parse::<i64>().unwrap_or(1);
        }),
        BulkRenameMsg::NumberDigitsChanged(digits) => mutate_rules(app, |m| {
            m.rules.number_digits = digits.clamp(1, 6);
        }),
        BulkRenameMsg::ToggleMedia => {
            if let Some(modal) = &mut app.bulk_rename_modal {
                modal.rules.media_enabled = !modal.rules.media_enabled;
                modal.recompute_preview();
            }
            maybe_dispatch_media_tags_worker(app)
        }
        BulkRenameMsg::ToggleMediaField(kind) => {
            if let Some(modal) = &mut app.bulk_rename_modal {
                modal.rules.media_enabled = true;
                if let Some(pos) = modal.rules.media_fields.iter().position(|&k| k == kind) {
                    modal.rules.media_fields.remove(pos);
                } else {
                    modal.rules.media_fields.push(kind);
                }
                modal.recompute_preview();
            }
            maybe_dispatch_media_tags_worker(app)
        }
        BulkRenameMsg::SetMediaMode(mode) => mutate_rules(app, |m| {
            m.rules.media_mode = mode;
        }),
        BulkRenameMsg::MediaSeparatorChanged(val) => mutate_rules(app, |m| {
            m.rules.media_separator = val;
        }),
        BulkRenameMsg::MediaTagsBatchLoaded { request_id, batch } => {
            if let Some(modal) = &mut app.bulk_rename_modal
                && modal.tags_request_id == request_id
            {
                for item in batch {
                    if let Some(slot) = modal.tags_by_input.get_mut(item.input_index) {
                        match slot {
                            Some(existing) => merge_media_tags(existing, item.tags),
                            None => *slot = Some(item.tags),
                        }
                    }
                    modal.tags_loaded_count = (modal.tags_loaded_count + 1).min(modal.inputs.len());
                }
                modal.recompute_preview();
            }
            Task::none()
        }
        BulkRenameMsg::MediaTagsFinished { request_id } => {
            if let Some(modal) = &mut app.bulk_rename_modal
                && modal.tags_request_id == request_id
            {
                modal.tags_loading = false;
                modal.tags_cancel = None;
            }
            Task::none()
        }
        BulkRenameMsg::ToggleExt => mutate_rules(app, |m| {
            m.rules.ext_enabled = !m.rules.ext_enabled;
        }),
        BulkRenameMsg::SetExtKind(kind) => mutate_rules(app, |m| {
            m.rules.ext_enabled = true;
            m.rules.ext_mode = match kind {
                BulkRenameExtensionKind::Keep => ExtensionMode::Keep,
                BulkRenameExtensionKind::Lowercase => ExtensionMode::Lowercase,
                BulkRenameExtensionKind::Uppercase => ExtensionMode::Uppercase,
                BulkRenameExtensionKind::Replace => {
                    ExtensionMode::Replace(m.ext_replace_draft.clone())
                }
                BulkRenameExtensionKind::Remove => ExtensionMode::Remove,
            };
        }),
        BulkRenameMsg::ExtReplaceChanged(val) => mutate_rules(app, |m| {
            m.rules.ext_enabled = true;
            m.ext_replace_draft = val;
            m.rules.ext_mode = ExtensionMode::Replace(m.ext_replace_draft.clone());
        }),
        BulkRenameMsg::Apply => {
            let Some(modal) = &mut app.bulk_rename_modal else {
                return Task::none();
            };
            if modal.applying || !modal.eval.can_apply() {
                return Task::none();
            }
            let batch_id = app.next_job_id;
            app.next_job_id = app.next_job_id.wrapping_add(1).max(1);
            let Some(plan) = build_execution_plan(
                &modal.eval,
                &modal.existing_names_in_dir,
                batch_id,
                BulkRenameTargetPolicy::host_default(),
            ) else {
                return Task::none();
            };

            if let Some(cancel) = modal.tags_cancel.take() {
                cancel.cancel();
            }
            modal.tags_loading = false;
            modal.applying = true;

            Task::perform(fm_fs::execute_bulk_rename_plan(plan), |outcome| {
                Message::BulkRename(BulkRenameMsg::ApplyFinished(outcome))
            })
        }
        BulkRenameMsg::ApplyFinished(outcome) => {
            if let Some(mut modal) = app.bulk_rename_modal.take()
                && let Some(cancel) = modal.tags_cancel.take()
            {
                cancel.cancel();
            }

            if !outcome.undo_pairs.is_empty() {
                app.last_bulk_rename_undo = Some(outcome.undo_pairs.clone());
                app.push_undo(crate::app::UndoAction::BulkRename {
                    pairs: outcome.undo_pairs,
                });
            }
            if !outcome.resulting_paths.is_empty() {
                app.pending_select_paths = outcome.resulting_paths;
            }

            if let Some((path, err)) = outcome.failed {
                let rollback_note = if outcome.rolled_back_temps > 0 {
                    format!(
                        " ({} temporário(s) restaurado(s))",
                        outcome.rolled_back_temps
                    )
                } else {
                    String::new()
                };
                app.status_notice =
                    Some(format!("{}: {err}{rollback_note}", path.to_string_lossy()));
            } else {
                app.status_notice = Some(app.strings().format_items_count(outcome.renamed_count));
            }

            app.update(Message::RefreshActiveTab)
        }
    }
}

pub fn undo_last_bulk_rename(app: &mut App) -> Task<Message> {
    app.context_menu = None;
    let Some(pairs) = app.last_bulk_rename_undo.take() else {
        return Task::none();
    };
    if let Some(pos) = app
        .undo_stack
        .iter()
        .rposition(|a| matches!(a, crate::app::UndoAction::BulkRename { .. }))
    {
        app.undo_stack.remove(pos);
    }
    let batch_id = app.next_job_id;
    app.next_job_id = app.next_job_id.wrapping_add(1).max(1);
    app.pending_select_paths = pairs.iter().map(|(_, old)| old.clone()).collect();

    Task::perform(
        fm_fs::undo_bulk_rename_pairs(pairs, batch_id, BulkRenameTargetPolicy::host_default()),
        Message::UndoBulkRenameFinished,
    )
}

fn mutate_rules(app: &mut App, mutator: impl FnOnce(&mut BulkRenameModalState)) -> Task<Message> {
    if let Some(modal) = &mut app.bulk_rename_modal {
        mutator(modal);
        modal.recompute_preview();
    }
    Task::none()
}

fn maybe_dispatch_media_tags_worker(app: &mut App) -> Task<Message> {
    let Some(modal) = &mut app.bulk_rename_modal else {
        return Task::none();
    };
    let need_exif = modal.rules.requires_exif() && !modal.tags_fetched_exif;
    let need_id3 = modal.rules.requires_id3() && !modal.tags_fetched_id3;
    if !need_exif && !need_id3 {
        return Task::none();
    }

    if let Some(prev) = modal.tags_cancel.take() {
        prev.cancel();
    }
    modal.tags_request_id = modal.tags_request_id.wrapping_add(1);
    let request_id = modal.tags_request_id;
    let cancel = CancellationToken::new();
    modal.tags_cancel = Some(cancel.clone());
    modal.tags_loading = true;
    modal.tags_loaded_count = 0;
    if need_exif {
        modal.tags_fetched_exif = true;
    }
    if need_id3 {
        modal.tags_fetched_id3 = true;
    }

    let items: Vec<(usize, PathBuf)> = modal
        .inputs
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.is_dir)
        .map(|(idx, item)| (idx, item.path.clone()))
        .collect();

    if items.is_empty() {
        modal.tags_loading = false;
        modal.tags_cancel = None;
        return Task::none();
    }

    let worker_stream = stream::channel(32, async move |mut output| {
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let stream_cancel = cancel.clone();
        tokio::spawn(async move {
            extract_media_tags_stream(items, need_exif, need_id3, stream_cancel, tx).await;
        });

        while let Some(batch) = rx.recv().await {
            let _ = output
                .send(Message::BulkRename(BulkRenameMsg::MediaTagsBatchLoaded {
                    request_id,
                    batch,
                }))
                .await;
        }
        let _ = output
            .send(Message::BulkRename(BulkRenameMsg::MediaTagsFinished {
                request_id,
            }))
            .await;
    });

    Task::run(worker_stream, std::convert::identity)
}

fn merge_media_tags(target: &mut BulkRenameMediaTags, incoming: BulkRenameMediaTags) {
    if target.exif_date_taken.is_none() {
        target.exif_date_taken = incoming.exif_date_taken;
    }
    if target.exif_date_ymd.is_none() {
        target.exif_date_ymd = incoming.exif_date_ymd;
    }
    if target.exif_camera_model.is_none() {
        target.exif_camera_model = incoming.exif_camera_model;
    }
    if target.id3_title.is_none() {
        target.id3_title = incoming.id3_title;
    }
    if target.id3_artist.is_none() {
        target.id3_artist = incoming.id3_artist;
    }
    if target.id3_album.is_none() {
        target.id3_album = incoming.id3_album;
    }
    if target.id3_track.is_none() {
        target.id3_track = incoming.id3_track;
    }
    if target.id3_year.is_none() {
        target.id3_year = incoming.id3_year;
    }
}

fn keep_unsigned_digits(raw: &str) -> String {
    raw.chars().filter(char::is_ascii_digit).take(5).collect()
}

fn keep_signed_digits(raw: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if (idx == 0 && ch == '-') || ch.is_ascii_digit() {
            out.push(ch);
        }
        if out.len() >= 7 {
            break;
        }
    }
    out
}
