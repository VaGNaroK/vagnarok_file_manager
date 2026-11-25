use std::fmt::Write as _;
use std::sync::OnceLock;
use std::time::SystemTime;

use compact_str::CompactString;
use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource};
use unic_langid::LanguageIdentifier;

use crate::domain::{
    EntryKind, FileEntry, LocalTimestampKind, LocaleFormatSpec, format_grouped_u64,
    local_timestamp_kind,
};
use crate::state::Language;

const FTL_EN_US: &str = include_str!("../locales/en-US.ftl");
const FTL_PT_BR: &str = include_str!("../locales/pt-BR.ftl");
const FTL_ES_ES: &str = include_str!("../locales/es-ES.ftl");
const FTL_FR_FR: &str = include_str!("../locales/fr-FR.ftl");
const FTL_DE_DE: &str = include_str!("../locales/de-DE.ftl");
const FTL_IT_IT: &str = include_str!("../locales/it-IT.ftl");

struct StringsInner {
    language: Language,
    format: LocaleFormatSpec,
    bundle: FluentBundle<FluentResource>,
}

static CACHE_EN_US: OnceLock<Strings> = OnceLock::new();
static CACHE_PT_BR: OnceLock<Strings> = OnceLock::new();
static CACHE_ES_ES: OnceLock<Strings> = OnceLock::new();
static CACHE_FR_FR: OnceLock<Strings> = OnceLock::new();
static CACHE_DE_DE: OnceLock<Strings> = OnceLock::new();
static CACHE_IT_IT: OnceLock<Strings> = OnceLock::new();

/// Catálogo localizado de textos da interface backed por Project Fluent (`include_str!`).
///
/// Mantido como handle `Copy` apoiado em caches `OnceLock` estáticos por idioma
/// para zero alocações e zero I/O de disco na thread de UI.
#[derive(Clone, Copy)]
pub struct Strings {
    inner: &'static StringsInner,
    pub quick_access: &'static str,
    pub drives: &'static str,
    pub tags_and_system: &'static str,
    pub documents: &'static str,
    pub home: &'static str,
    pub desktop: &'static str,
    pub downloads: &'static str,
    pub pictures: &'static str,
    pub code_dev: &'static str,
    pub starred: &'static str,
    pub trash: &'static str,
    pub preferences: &'static str,
    pub search_placeholder: &'static str,
    pub no_active_tab: &'static str,
    pub follow_system: &'static str,
    pub new_item: &'static str,
    pub sort: &'static str,
    pub hidden: &'static str,
    pub folders: &'static str,
    pub files: &'static str,
    pub total_size: &'static str,
    pub inspector_title: &'static str,
    pub inspector_empty: &'static str,
    pub permissions: &'static str,
    pub modified: &'static str,
    pub checksum: &'static str,
    pub mime_type: &'static str,
    pub selected_none: &'static str,
    pub quicklook_hint: &'static str,
    pub quicklook_title: &'static str,
    pub close: &'static str,
    pub col_name: &'static str,
    pub col_size: &'static str,
    pub col_type: &'static str,
    pub col_perms: &'static str,
    pub col_modified: &'static str,
    pub free_of: &'static str,
    pub loading: &'static str,
    pub empty_dir: &'static str,
    pub go_back: &'static str,
    pub retry: &'static str,
    pub root_fs: &'static str,
    pub dir_badge: &'static str,
    pub file_badge: &'static str,
    pub binary_preview_hint: &'static str,
    pub deep_search: &'static str,
    pub deep_search_title: &'static str,
    pub deep_search_empty: &'static str,
    pub search_name_label: &'static str,
    pub search_containing_label: &'static str,
    pub search_containing_placeholder: &'static str,
    pub search_size_label: &'static str,
    pub search_date_label: &'static str,
    pub search_type_label: &'static str,
    pub search_hidden_label: &'static str,
    pub search_streaming_hint: &'static str,
    pub search_any: &'static str,
    pub search_today: &'static str,
    pub search_last_7d: &'static str,
    pub search_last_30d: &'static str,
    pub search_type_code: &'static str,
    pub search_type_docs: &'static str,
    pub search_type_images: &'static str,
    pub search_type_audio: &'static str,
    pub search_type_video: &'static str,
    pub search_type_archives: &'static str,
    pub cmd_palette_title: &'static str,
    pub cmd_palette_placeholder: &'static str,
    pub cmd_palette_empty: &'static str,
    pub cmd_palette_hint: &'static str,
    pub cmd_cat_nav: &'static str,
    pub cmd_cat_search: &'static str,
    pub cmd_cat_files: &'static str,
    pub cmd_cat_view: &'static str,
    pub cmd_cat_tools: &'static str,
    pub cmd_focus_quick_filter: &'static str,
    pub cmd_search_containing: &'static str,
    pub cmd_clear_filters: &'static str,
    pub cmd_check_updates: &'static str,
    pub cmd_toggle_theme: &'static str,
    pub cmd_view_grid: &'static str,
    pub cmd_view_list: &'static str,
    pub searching: &'static str,
    pub computing_hash: &'static str,
    pub new_folder: &'static str,
    pub new_file: &'static str,
    pub rename: &'static str,
    pub copy: &'static str,
    pub cut: &'static str,
    pub paste: &'static str,
    pub move_to_trash: &'static str,
    pub delete_permanent: &'static str,
    pub confirm: &'static str,
    pub cancel: &'static str,
    pub create: &'static str,
    pub name_label: &'static str,
    pub invalid_name: &'static str,
    pub confirm_trash_msg: &'static str,
    pub confirm_delete_msg: &'static str,
    pub clipboard_copy_tag: &'static str,
    pub clipboard_cut_tag: &'static str,
    pub transferring: &'static str,
    pub prefs_title: &'static str,
    pub theme_label: &'static str,
    pub lang_label: &'static str,
    pub hero_card_label: &'static str,
    pub decorations_label: &'static str,
    pub default_view_label: &'static str,
    pub view_grid: &'static str,
    pub view_list: &'static str,
    pub window_bar_native: &'static str,
    pub window_bar_csd: &'static str,
    pub pin_favorite: &'static str,
    pub unpin_favorite: &'static str,
    pub remove_favorite: &'static str,
    pub empty_subdir: &'static str,
    pub open_terminal: &'static str,
    pub git_status_label: &'static str,
    pub conflict_title: &'static str,
    pub conflict_msg: &'static str,
    pub conflict_rename: &'static str,
    pub conflict_overwrite: &'static str,
    pub conflict_skip: &'static str,
    pub created: &'static str,
    pub open_item: &'static str,
    pub open_in_new_tab: &'static str,
    pub copy_path: &'static str,
    pub select_all: &'static str,
    pub path_copied: &'static str,
    pub properties: &'static str,
    pub filter_all: &'static str,
    pub filter_ext: &'static str,
    pub directories_only: &'static str,
    pub archive_readonly: &'static str,
    pub grid_group_label: &'static str,
    pub dirs_first_label: &'static str,
    pub recursive_size_label: &'static str,
    pub empty_trash: &'static str,
    pub restore_item: &'static str,
    pub confirm_empty_trash_msg: &'static str,
    pub trash_emptied: &'static str,
    pub trash_restored: &'static str,
    pub location_label: &'static str,
    pub copy_sha256: &'static str,
    pub hash_copied: &'static str,
    pub properties_header_prefix: &'static str,
    pub prop_tab_general: &'static str,
    pub prop_tab_security: &'static str,
    pub prop_tab_history: &'static str,
    pub prop_tab_metadata: &'static str,
    pub prop_tab_permissions: &'static str,
    pub prop_kind_label: &'static str,
    pub prop_system_mime_label: &'static str,
    pub prop_accessed_label: &'static str,
    pub prop_quick_props: &'static str,
    pub prop_readonly: &'static str,
    pub prop_hidden: &'static str,
    pub prop_advanced_settings: &'static str,
    pub prop_confirm: &'static str,
    pub prop_apply: &'static str,
    pub prop_open_default: &'static str,
    pub prop_owner: &'static str,
    pub prop_group: &'static str,
    pub prop_others: &'static str,
    pub prop_octal_mode: &'static str,
    pub prop_symlink_target: &'static str,
    pub prop_archive_notice: &'static str,
    pub prop_applied_notice: &'static str,
    pub prop_pending_changes: &'static str,
    pub prop_calculating: &'static str,
    pub prop_sha_na_folders: &'static str,
    pub prop_sha_computing: &'static str,
    pub yes_label: &'static str,
    pub no_label: &'static str,
    pub ext_label: &'static str,
    pub perm_read: &'static str,
    pub perm_write: &'static str,
    pub perm_execute: &'static str,
    pub prefs_tab_general: &'static str,
    pub about_tab: &'static str,
    pub about_subtitle: &'static str,
    pub about_version_label: &'static str,
    pub about_developer_label: &'static str,
    pub about_developer_value: &'static str,
    pub about_stage_label: &'static str,
    pub about_stage_alpha: &'static str,
    pub about_license_label: &'static str,
    pub about_stack_label: &'static str,
    pub about_copy_info: &'static str,
    pub about_info_copied: &'static str,
    pub prop_contents_label: &'static str,
    pub prop_free_space_label: &'static str,
    pub tip_new_tab: &'static str,
    pub tip_close_tab: &'static str,
    pub tip_language: &'static str,
    pub tip_theme: &'static str,
    pub tip_back: &'static str,
    pub tip_forward: &'static str,
    pub tip_up: &'static str,
    pub tip_home: &'static str,
    pub tip_edit_path: &'static str,
    pub tip_deep_search: &'static str,
    pub tip_view_grid: &'static str,
    pub tip_view_list: &'static str,
    pub tip_new_item: &'static str,
    pub tip_group_folders: &'static str,
    pub tip_zoom_out: &'static str,
    pub tip_zoom_reset: &'static str,
    pub tip_zoom_in: &'static str,
    pub tip_sort: &'static str,
    pub tip_sort_dir: &'static str,
    pub tip_filter: &'static str,
    pub tip_hidden: &'static str,
    pub tip_terminal: &'static str,
    pub tip_favorite: &'static str,
    pub tip_inspector: &'static str,
    pub tip_preferences: &'static str,
    pub tip_toggle_sidebar: &'static str,
    pub music: &'static str,
    pub videos: &'static str,
    pub public_dir: &'static str,
    pub templates: &'static str,
    pub filesystem_root: &'static str,
    pub mount_drive: &'static str,
    pub unmount_drive: &'static str,
    pub unmounted_label: &'static str,
    pub refresh_drives: &'static str,
    pub tip_expand_home: &'static str,
    pub open_as_root: &'static str,
    pub root_mode_badge: &'static str,
    pub grid_center_label: &'static str,
    pub grid_align_left: &'static str,
    pub grid_align_center: &'static str,
    pub tip_grid_alignment: &'static str,
    pub tip_system_icons: &'static str,
    pub perm_allow_exec: &'static str,
    pub open_with: &'static str,
    pub prop_tab_open_with: &'static str,
    pub open_with_prompt: &'static str,
    pub open_with_default_app: &'static str,
    pub open_with_recommended_apps: &'static str,
    pub open_with_other_apps: &'static str,
    pub open_with_custom_hint: &'static str,
    pub open_with_custom_placeholder: &'static str,
    pub open_with_add_to_list: &'static str,
    pub open_with_set_default: &'static str,
    pub open_with_reset_default: &'static str,
    pub open_with_open_once: &'static str,
    pub open_with_loading: &'static str,
    pub open_with_empty: &'static str,
    pub open_with_saved: &'static str,
    pub open_with_reset_done: &'static str,
    pub open_with_pick_exec: &'static str,
    pub br_title: &'static str,
    pub br_menu_item: &'static str,
    pub br_clear_rules: &'static str,
    pub br_rules_header: &'static str,
    pub br_preview_header: &'static str,
    pub br_col_status: &'static str,
    pub br_col_original: &'static str,
    pub br_col_new: &'static str,
    pub br_unchanged: &'static str,
    pub br_skipped_folder: &'static str,
    pub br_conflict_internal: &'static str,
    pub br_conflict_external: &'static str,
    pub br_invalid_name: &'static str,
    pub br_replace_title: &'static str,
    pub br_find_label: &'static str,
    pub br_replace_label: &'static str,
    pub br_where_label: &'static str,
    pub br_scope_anywhere: &'static str,
    pub br_scope_start_name: &'static str,
    pub br_scope_end_name: &'static str,
    pub br_scope_word_start: &'static str,
    pub br_scope_word_end: &'static str,
    pub br_scope_whole_word: &'static str,
    pub br_match_case: &'static str,
    pub br_first_only: &'static str,
    pub br_advanced_regex: &'static str,
    pub br_use_regex: &'static str,
    pub br_remove_title: &'static str,
    pub br_remove_first: &'static str,
    pub br_remove_last: &'static str,
    pub br_remove_text_label: &'static str,
    pub br_remove_digits: &'static str,
    pub br_remove_symbols: &'static str,
    pub br_collapse_spaces: &'static str,
    pub br_insert_title: &'static str,
    pub br_insert_text_label: &'static str,
    pub br_insert_pos_label: &'static str,
    pub br_insert_start: &'static str,
    pub br_insert_end: &'static str,
    pub br_insert_offset_start: &'static str,
    pub br_insert_offset_end: &'static str,
    pub br_case_title: &'static str,
    pub br_case_keep: &'static str,
    pub br_case_upper: &'static str,
    pub br_case_lower: &'static str,
    pub br_case_title_words: &'static str,
    pub br_case_sentence: &'static str,
    pub br_number_title: &'static str,
    pub br_number_append: &'static str,
    pub br_number_prepend: &'static str,
    pub br_number_replace: &'static str,
    pub br_number_start: &'static str,
    pub br_number_step: &'static str,
    pub br_number_digits: &'static str,
    pub br_number_sep: &'static str,
    pub br_sort_label: &'static str,
    pub br_sort_selection: &'static str,
    pub br_sort_name_asc: &'static str,
    pub br_sort_name_desc: &'static str,
    pub br_sort_date_desc: &'static str,
    pub br_sort_date_asc: &'static str,
    pub br_sort_size_asc: &'static str,
    pub br_sort_size_desc: &'static str,
    pub br_media_title: &'static str,
    pub br_media_exif_ymd: &'static str,
    pub br_media_exif_datetime: &'static str,
    pub br_media_exif_camera: &'static str,
    pub br_media_id3_track: &'static str,
    pub br_media_id3_artist: &'static str,
    pub br_media_id3_album: &'static str,
    pub br_media_id3_title: &'static str,
    pub br_media_id3_year: &'static str,
    pub br_media_loading: &'static str,
    pub br_ext_title: &'static str,
    pub br_ext_keep: &'static str,
    pub br_ext_lower: &'static str,
    pub br_ext_upper: &'static str,
    pub br_ext_replace: &'static str,
    pub br_ext_remove: &'static str,
    pub br_include_folders: &'static str,
    pub br_apply: &'static str,
    pub br_applying: &'static str,
    pub br_undo_last: &'static str,
    pub br_readonly_error: &'static str,
    pub arch_extract_here: &'static str,
    pub arch_extract_subdir: &'static str,
    pub arch_compress_as: &'static str,
    pub arch_add_7z: &'static str,
    pub arch_modal_title: &'static str,
    pub arch_output_name: &'static str,
    pub arch_destination: &'static str,
    pub arch_format_label: &'static str,
    pub arch_level_label: &'static str,
    pub arch_format_7z: &'static str,
    pub arch_format_zip: &'static str,
    pub arch_format_tar: &'static str,
    pub arch_format_targz: &'static str,
    pub arch_level_store: &'static str,
    pub arch_level_min: &'static str,
    pub arch_level_normal: &'static str,
    pub arch_level_max: &'static str,
    pub arch_tar_store_hint: &'static str,
    pub arch_conflict_label: &'static str,
    pub arch_conflict_rename: &'static str,
    pub arch_conflict_overwrite: &'static str,
    pub arch_conflict_skip: &'static str,
    pub arch_start_compress: &'static str,
    pub arch_minimize: &'static str,
    pub arch_restore: &'static str,
    pub arch_compressing: &'static str,
    pub arch_extracting: &'static str,
    pub arch_done_compress: &'static str,
    pub arch_done_extract: &'static str,
    pub arch_cancelled: &'static str,
    pub arch_password_title: &'static str,
    pub arch_password_hint: &'static str,
    pub arch_password_placeholder: &'static str,
    pub arch_password_wrong: &'static str,
    pub arch_password_submit: &'static str,
    pub arch_password_toggle: &'static str,
    pub dup_menu_action: &'static str,
    pub dup_modal_title: &'static str,
    pub dup_readonly_error: &'static str,
    pub dup_confirm_level_label: &'static str,
    pub dup_level_quick: &'static str,
    pub dup_level_standard: &'static str,
    pub dup_level_sha256: &'static str,
    pub dup_level_byte: &'static str,
    pub dup_min_size_label: &'static str,
    pub dup_min_size_any: &'static str,
    pub dup_min_size_1kb: &'static str,
    pub dup_min_size_1mb: &'static str,
    pub dup_min_size_10mb: &'static str,
    pub dup_start_scan: &'static str,
    pub dup_rescan: &'static str,
    pub dup_stop_scan: &'static str,
    pub dup_opt_subfolders: &'static str,
    pub dup_opt_hidden: &'static str,
    pub dup_opt_ignore_empty: &'static str,
    pub dup_opt_ignore_hardlinks: &'static str,
    pub dup_quick_warning: &'static str,
    pub dup_quick_select_label: &'static str,
    pub dup_keep_oldest: &'static str,
    pub dup_keep_newest: &'static str,
    pub dup_keep_shortest: &'static str,
    pub dup_clear_selection: &'static str,
    pub dup_badge_oldest: &'static str,
    pub dup_badge_hardlink: &'static str,
    pub dup_verify_partial: &'static str,
    pub dup_verify_md5: &'static str,
    pub dup_verify_sha256: &'static str,
    pub dup_verify_byte: &'static str,
    pub dup_stage_walking: &'static str,
    pub dup_stage_partial: &'static str,
    pub dup_stage_full: &'static str,
    pub dup_stage_sha256: &'static str,
    pub dup_stage_byte: &'static str,
    pub dup_all_selected_warning: &'static str,
    pub dup_delete_permanent: &'static str,
    pub dup_move_to_trash: &'static str,
    pub dup_confirm_perm_delete: &'static str,
    pub dup_empty_idle: &'static str,
    pub dup_empty_none: &'static str,
    pub dup_group_prefix: &'static str,
    pub dup_copies_word: &'static str,
    pub dup_each_word: &'static str,
    pub dup_selected_word: &'static str,
    pub dup_groups_found: &'static str,
    pub dup_reclaimable: &'static str,
    pub dup_footer_prefix: &'static str,
    pub dup_cancelled: &'static str,
    pub dup_stale_skipped: &'static str,
    pub dup_last_copy_protected: &'static str,
    pub dup_delete_done: &'static str,
    pub ctx_send_to: &'static str,
    pub ctx_shortcut_desktop: &'static str,
    pub ctx_shortcut_here_unix: &'static str,
    pub ctx_shortcut_here_win: &'static str,
    pub shortcut_desktop_missing: &'static str,
    pub filter_ext_placeholder: &'static str,
    pub undo_action: &'static str,
    pub undo_trash_action: &'static str,
    pub moved_to_trash_undo_hint: &'static str,
    pub undo_nothing_in_trash: &'static str,
    pub undo_rename_done: &'static str,
    pub mount_disk_image: &'static str,
    pub unmount_disk_image: &'static str,
    pub eject_safely: &'static str,
    pub image_mounted: &'static str,
    pub drive_ejected: &'static str,
    pub drive_free_label: &'static str,
    pub drive_click_to_mount: &'static str,
    pub tip_split_view: &'static str,
    pub close_split_pane: &'static str,
    pub drag_move_prefix: &'static str,
    pub drag_copy_prefix: &'static str,
    pub drag_ctrl_copy_hint: &'static str,
    pub update_available_banner: &'static str,
    pub update_download_btn: &'static str,
    pub update_ignore_btn: &'static str,
    pub update_check_btn: &'static str,
    pub update_checking: &'static str,
    pub update_up_to_date: &'static str,
    pub update_check_failed: &'static str,
    pub update_auto_check_label: &'static str,
    pub update_open_releases: &'static str,
    pub tip_update_available: &'static str,
    pub recent_files: &'static str,
    pub prefs_recent_files_label: &'static str,
    pub clear_recent_files: &'static str,
    pub remove_from_recent: &'static str,
    pub open_file_location: &'static str,
    pub open_target_location: &'static str,
    pub prefs_startup_location_label: &'static str,
    pub startup_home: &'static str,
    pub startup_last_visited: &'static str,
    pub ntfs_object_name: &'static str,
    pub ntfs_principals_label: &'static str,
    pub ntfs_edit_hint: &'static str,
    pub ntfs_edit_active_hint: &'static str,
    pub ntfs_edit_btn: &'static str,
    pub ntfs_done_edit_btn: &'static str,
    pub ntfs_add_btn: &'static str,
    pub ntfs_remove_btn: &'static str,
    pub ntfs_permissions_for: &'static str,
    pub ntfs_allow_col: &'static str,
    pub ntfs_deny_col: &'static str,
    pub ntfs_full_control: &'static str,
    pub ntfs_list_folder: &'static str,
    pub ntfs_delete_modify: &'static str,
    pub ntfs_inherited_badge: &'static str,
    pub ntfs_explicit_badge: &'static str,
    pub ntfs_owner_label: &'static str,
    pub ntfs_inheritance_label: &'static str,
    pub ntfs_inheritance_on: &'static str,
    pub ntfs_inheritance_off: &'static str,
    pub ntfs_add_placeholder: &'static str,
    pub ntfs_loading: &'static str,
    pub posix_owner_group_header: &'static str,
    pub posix_octal_symbolic_header: &'static str,
    pub posix_presets_label: &'static str,
    pub posix_preset_default: &'static str,
    pub posix_preset_exec: &'static str,
    pub posix_preset_private: &'static str,
    pub posix_preset_readonly: &'static str,
    pub posix_preset_group: &'static str,
    pub posix_preset_shared: &'static str,
    pub posix_scope_col: &'static str,
    pub posix_summary_col: &'static str,
    pub posix_others_sub: &'static str,
    pub posix_special_bits_header: &'static str,
    pub posix_quick_action_header: &'static str,
    pub posix_recursive_header: &'static str,
    pub posix_apply_recursive: &'static str,
    pub analyze_space: &'static str,
    pub tip_analyze_space: &'static str,
    pub disk_scan_eyebrow: &'static str,
    pub disk_scan_title: &'static str,
    pub disk_scan_current_item: &'static str,
    pub disk_scan_files_checked: &'static str,
    pub disk_scan_data_checked: &'static str,
    pub disk_scan_elapsed: &'static str,
    pub disk_scan_hint: &'static str,
    pub disk_results_title: &'static str,
    pub disk_root_crumb: &'static str,
    pub disk_treemap_title: &'static str,
    pub disk_top_consumers: &'static str,
    pub disk_by_type: &'static str,
    pub disk_mark_cleanup: &'static str,
    pub disk_reanalyze: &'static str,
    pub disk_clear_selection: &'static str,
    pub disk_reclaim_suffix: &'static str,
    pub disk_mode_files: &'static str,
    pub disk_mode_folders: &'static str,
    pub disk_cat_iso: &'static str,
    pub disk_cat_build: &'static str,
    pub disk_cat_archives: &'static str,
    pub disk_cat_media: &'static str,
    pub disk_cat_docs: &'static str,
    pub disk_cat_other: &'static str,
}

impl std::fmt::Debug for Strings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Strings")
            .field("language", &self.inner.language)
            .finish()
    }
}

impl Strings {
    pub fn for_language(lang: Language) -> Self {
        match lang {
            Language::EnUs => *CACHE_EN_US.get_or_init(|| Self::build(Language::EnUs)),
            Language::PtBr => *CACHE_PT_BR.get_or_init(|| Self::build(Language::PtBr)),
            Language::EsEs => *CACHE_ES_ES.get_or_init(|| Self::build(Language::EsEs)),
            Language::FrFr => *CACHE_FR_FR.get_or_init(|| Self::build(Language::FrFr)),
            Language::DeDe => *CACHE_DE_DE.get_or_init(|| Self::build(Language::DeDe)),
            Language::ItIt => *CACHE_IT_IT.get_or_init(|| Self::build(Language::ItIt)),
        }
    }

    pub fn language(self) -> Language {
        self.inner.language
    }

    pub fn format_spec(self) -> LocaleFormatSpec {
        self.inner.format
    }

    /// Formata um inteiro usando o separador de milhares do idioma ativo.
    pub fn format_grouped(self, value: u64) -> CompactString {
        format_grouped_u64(value, self.inner.format.thousands_sep)
    }

    /// Formata a contagem exata de bytes (`"(14.931.200 bytes)"`).
    pub fn format_exact_bytes(self, value: u64) -> String {
        let grouped = self.format_grouped(value);
        let mut args = FluentArgs::new();
        args.set("bytes", grouped.as_str());
        self.format_msg("exact-bytes", Some(&args))
    }

    /// Formata contagem de itens usando regras plurais CLDR do Fluent.
    pub fn format_items_count(self, count: usize) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count as i64);
        let formatted = self.format_msg("items-count", Some(&args));
        if count >= 1000 {
            let grouped = self.format_grouped(count as u64);
            formatted.replacen(&count.to_string(), grouped.as_str(), 1)
        } else {
            formatted
        }
    }

    /// Formata contagem de itens ocultos (`"1 oculto"` / `"2.783 ocultos"`).
    pub fn format_hidden_items_count(self, count: u64) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count as i64);
        let formatted = self.format_msg("hidden-items-count", Some(&args));
        if count >= 1000 {
            let grouped = self.format_grouped(count);
            formatted.replacen(&count.to_string(), grouped.as_str(), 1)
        } else {
            formatted
        }
    }

    /// Formata contagem de pastas usando regras plurais CLDR do Fluent.
    pub fn format_folders_count(self, count: usize) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count as i64);
        self.format_msg("folders-count", Some(&args))
    }

    /// Formata contagem de arquivos usando regras plurais CLDR do Fluent.
    pub fn format_files_count(self, count: usize) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count as i64);
        self.format_msg("files-count", Some(&args))
    }

    /// Formata mensagem de sucesso na criação de 1 ou mais atalhos usando regras plurais CLDR.
    pub fn format_shortcuts_created(self, count: usize) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count as i64);
        self.format_msg("shortcuts-created", Some(&args))
    }

    /// Formata mensagem de itens restaurados da Lixeira via Desfazer (`Ctrl+Z`).
    pub fn format_undo_restored(self, count: usize) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count as i64);
        self.format_msg("undo-restored-count", Some(&args))
    }

    /// Formata resumo de seleção múltipla no rodapé (`"3 itens (12.4 KB)"`).
    pub fn format_selected_multi(self, count: usize, size_label: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count as i64);
        args.set("size", size_label);
        self.format_msg("selected-multi", Some(&args))
    }

    /// Formata idade relativa (`agora`, `há 14m`, `2h ago`, `vor 3 Tg.`).
    pub fn format_age(self, modified: SystemTime) -> String {
        let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
            return self.format_msg("age-now", None);
        };
        let secs = elapsed.as_secs();
        if secs < 60 {
            self.format_msg("age-now", None)
        } else if secs < 3600 {
            let mut args = FluentArgs::new();
            args.set("m", (secs / 60) as i64);
            self.format_msg("age-minutes", Some(&args))
        } else if secs < 86_400 {
            let mut args = FluentArgs::new();
            args.set("h", (secs / 3600) as i64);
            self.format_msg("age-hours", Some(&args))
        } else {
            let mut args = FluentArgs::new();
            args.set("d", (secs / 86_400) as i64);
            self.format_msg("age-days", Some(&args))
        }
    }

    /// Formata data/hora local curta (`Hoje, 17:45`, `Yesterday, 18:05`, `10 Nov 2026, 14:32`).
    pub fn format_timestamp_short(self, ts: SystemTime) -> String {
        match local_timestamp_kind(ts) {
            LocalTimestampKind::Today { hour, minute } => {
                let mut time = String::with_capacity(5);
                let _ = write!(&mut time, "{hour:02}:{minute:02}");
                let mut args = FluentArgs::new();
                args.set("time", time.as_str());
                self.format_msg("ts-today", Some(&args))
            }
            LocalTimestampKind::Yesterday { hour, minute } => {
                let mut time = String::with_capacity(5);
                let _ = write!(&mut time, "{hour:02}:{minute:02}");
                let mut args = FluentArgs::new();
                args.set("time", time.as_str());
                self.format_msg("ts-yesterday", Some(&args))
            }
            LocalTimestampKind::Date {
                day,
                month_idx,
                year,
                hour,
                minute,
            } => {
                let month = self
                    .inner
                    .format
                    .months_short
                    .get(month_idx)
                    .copied()
                    .unwrap_or("Jan");
                format!("{day:02} {month} {year:04}, {hour:02}:{minute:02}")
            }
        }
    }

    /// Descreve o tipo do item localizado para a janela de Propriedades.
    pub fn describe_entry_kind(self, entry: &FileEntry) -> String {
        match &entry.kind {
            EntryKind::Directory => self.format_msg("kind-folder", None),
            EntryKind::Symlink {
                target_is_dir: true,
            } => self.format_msg("kind-symlink-dir", None),
            EntryKind::Symlink {
                target_is_dir: false,
            } => self.format_msg("kind-symlink-file", None),
            EntryKind::Special => self.format_msg("kind-special", None),
            EntryKind::File => match &entry.extension {
                Some(ext) => {
                    let low = ext.to_ascii_lowercase();
                    let mut args = FluentArgs::new();
                    args.set("ext", low.as_str());
                    self.format_msg("kind-file-ext", Some(&args))
                }
                None => self.format_msg("kind-file", None),
            },
        }
    }

    fn format_msg(self, id: &str, args: Option<&FluentArgs<'_>>) -> String {
        if let Some(res) = try_format_in_bundle(&self.inner.bundle, id, args) {
            return res;
        }
        if self.inner.language != Language::EnUs {
            let en = Self::for_language(Language::EnUs);
            if let Some(res) = try_format_in_bundle(&en.inner.bundle, id, args) {
                return res;
            }
        }
        id.to_owned()
    }

    fn build(lang: Language) -> Self {
        let bundle = build_bundle(lang);
        let fallback = if lang == Language::EnUs {
            None
        } else {
            Some(&Self::for_language(Language::EnUs).inner.bundle)
        };
        let format = locale_format_spec(lang);

        let resolve = |id: &str| -> &'static str {
            let s = try_format_in_bundle(&bundle, id, None)
                .or_else(|| fallback.and_then(|fb| try_format_in_bundle(fb, id, None)))
                .unwrap_or_else(|| id.to_owned());
            Box::leak(s.into_boxed_str())
        };

        let quick_access = resolve("quick-access");
        let drives = resolve("drives");
        let tags_and_system = resolve("tags-and-system");
        let documents = resolve("documents");
        let home = resolve("home");
        let desktop = resolve("desktop");
        let downloads = resolve("downloads");
        let pictures = resolve("pictures");
        let code_dev = resolve("code-dev");
        let starred = resolve("starred");
        let trash = resolve("trash");
        let preferences = resolve("preferences");
        let search_placeholder = resolve("search-placeholder");
        let no_active_tab = resolve("no-active-tab");
        let follow_system = resolve("follow-system");
        let new_item = resolve("new-item");
        let sort = resolve("sort");
        let hidden = resolve("hidden");
        let folders = resolve("folders");
        let files = resolve("files");
        let total_size = resolve("total-size");
        let inspector_title = resolve("inspector-title");
        let inspector_empty = resolve("inspector-empty");
        let permissions = resolve("permissions");
        let modified = resolve("modified");
        let checksum = resolve("checksum");
        let mime_type = resolve("mime-type");
        let selected_none = resolve("selected-none");
        let quicklook_hint = resolve("quicklook-hint");
        let quicklook_title = resolve("quicklook-title");
        let close = resolve("close");
        let col_name = resolve("col-name");
        let col_size = resolve("col-size");
        let col_type = resolve("col-type");
        let col_perms = resolve("col-perms");
        let col_modified = resolve("col-modified");
        let free_of = resolve("free-of");
        let loading = resolve("loading");
        let empty_dir = resolve("empty-dir");
        let go_back = resolve("go-back");
        let retry = resolve("retry");
        let root_fs = resolve("root-fs");
        let dir_badge = resolve("dir-badge");
        let file_badge = resolve("file-badge");
        let binary_preview_hint = resolve("binary-preview-hint");
        let deep_search = resolve("deep-search");
        let deep_search_title = resolve("deep-search-title");
        let deep_search_empty = resolve("deep-search-empty");
        let search_name_label = resolve("search-name-label");
        let search_containing_label = resolve("search-containing-label");
        let search_containing_placeholder = resolve("search-containing-placeholder");
        let search_size_label = resolve("search-size-label");
        let search_date_label = resolve("search-date-label");
        let search_type_label = resolve("search-type-label");
        let search_hidden_label = resolve("search-hidden-label");
        let search_streaming_hint = resolve("search-streaming-hint");
        let search_any = resolve("search-any");
        let search_today = resolve("search-today");
        let search_last_7d = resolve("search-last-7d");
        let search_last_30d = resolve("search-last-30d");
        let search_type_code = resolve("search-type-code");
        let search_type_docs = resolve("search-type-docs");
        let search_type_images = resolve("search-type-images");
        let search_type_audio = resolve("search-type-audio");
        let search_type_video = resolve("search-type-video");
        let search_type_archives = resolve("search-type-archives");
        let cmd_palette_title = resolve("cmd-palette-title");
        let cmd_palette_placeholder = resolve("cmd-palette-placeholder");
        let cmd_palette_empty = resolve("cmd-palette-empty");
        let cmd_palette_hint = resolve("cmd-palette-hint");
        let cmd_cat_nav = resolve("cmd-cat-nav");
        let cmd_cat_search = resolve("cmd-cat-search");
        let cmd_cat_files = resolve("cmd-cat-files");
        let cmd_cat_view = resolve("cmd-cat-view");
        let cmd_cat_tools = resolve("cmd-cat-tools");
        let cmd_focus_quick_filter = resolve("cmd-focus-quick-filter");
        let cmd_search_containing = resolve("cmd-search-containing");
        let cmd_clear_filters = resolve("cmd-clear-filters");
        let cmd_check_updates = resolve("cmd-check-updates");
        let cmd_toggle_theme = resolve("cmd-toggle-theme");
        let cmd_view_grid = resolve("cmd-view-grid");
        let cmd_view_list = resolve("cmd-view-list");
        let searching = resolve("searching");
        let computing_hash = resolve("computing-hash");
        let new_folder = resolve("new-folder");
        let new_file = resolve("new-file");
        let rename = resolve("rename");
        let copy = resolve("copy");
        let cut = resolve("cut");
        let paste = resolve("paste");
        let move_to_trash = resolve("move-to-trash");
        let delete_permanent = resolve("delete-permanent");
        let confirm = resolve("confirm");
        let cancel = resolve("cancel");
        let create = resolve("create");
        let name_label = resolve("name-label");
        let invalid_name = resolve("invalid-name");
        let confirm_trash_msg = resolve("confirm-trash-msg");
        let confirm_delete_msg = resolve("confirm-delete-msg");
        let clipboard_copy_tag = resolve("clipboard-copy-tag");
        let clipboard_cut_tag = resolve("clipboard-cut-tag");
        let transferring = resolve("transferring");
        let prefs_title = resolve("prefs-title");
        let theme_label = resolve("theme-label");
        let lang_label = resolve("lang-label");
        let hero_card_label = resolve("hero-card-label");
        let decorations_label = resolve("decorations-label");
        let default_view_label = resolve("default-view-label");
        let view_grid = resolve("view-grid");
        let view_list = resolve("view-list");
        let window_bar_native = resolve("window-bar-native");
        let window_bar_csd = resolve("window-bar-csd");
        let pin_favorite = resolve("pin-favorite");
        let unpin_favorite = resolve("unpin-favorite");
        let remove_favorite = resolve("remove-favorite");
        let empty_subdir = resolve("empty-subdir");
        let open_terminal = resolve("open-terminal");
        let git_status_label = resolve("git-status-label");
        let conflict_title = resolve("conflict-title");
        let conflict_msg = resolve("conflict-msg");
        let conflict_rename = resolve("conflict-rename");
        let conflict_overwrite = resolve("conflict-overwrite");
        let conflict_skip = resolve("conflict-skip");
        let created = resolve("created");
        let open_item = resolve("open-item");
        let open_in_new_tab = resolve("open-in-new-tab");
        let copy_path = resolve("copy-path");
        let select_all = resolve("select-all");
        let path_copied = resolve("path-copied");
        let properties = resolve("properties");
        let filter_all = resolve("filter-all");
        let filter_ext = resolve("filter-ext");
        let directories_only = resolve("directories-only");
        let archive_readonly = resolve("archive-readonly");
        let grid_group_label = resolve("grid-group-label");
        let dirs_first_label = resolve("dirs-first-label");
        let recursive_size_label = resolve("recursive-size-label");
        let empty_trash = resolve("empty-trash");
        let restore_item = resolve("restore-item");
        let confirm_empty_trash_msg = resolve("confirm-empty-trash-msg");
        let trash_emptied = resolve("trash-emptied");
        let trash_restored = resolve("trash-restored");
        let location_label = resolve("location-label");
        let copy_sha256 = resolve("copy-sha256");
        let hash_copied = resolve("hash-copied");
        let properties_header_prefix = resolve("properties-header-prefix");
        let prop_tab_general = resolve("prop-tab-general");
        let prop_tab_security = resolve("prop-tab-security");
        let prop_tab_history = resolve("prop-tab-history");
        let prop_tab_metadata = resolve("prop-tab-metadata");
        let prop_tab_permissions = resolve("prop-tab-permissions");
        let prop_kind_label = resolve("prop-kind-label");
        let prop_system_mime_label = resolve("prop-system-mime-label");
        let prop_accessed_label = resolve("prop-accessed-label");
        let prop_quick_props = resolve("prop-quick-props");
        let prop_readonly = resolve("prop-readonly");
        let prop_hidden = resolve("prop-hidden");
        let prop_advanced_settings = resolve("prop-advanced-settings");
        let prop_confirm = resolve("prop-confirm");
        let prop_apply = resolve("prop-apply");
        let prop_open_default = resolve("prop-open-default");
        let prop_owner = resolve("prop-owner");
        let prop_group = resolve("prop-group");
        let prop_others = resolve("prop-others");
        let prop_octal_mode = resolve("prop-octal-mode");
        let prop_symlink_target = resolve("prop-symlink-target");
        let prop_archive_notice = resolve("prop-archive-notice");
        let prop_applied_notice = resolve("prop-applied-notice");
        let prop_pending_changes = resolve("prop-pending-changes");
        let prop_calculating = resolve("prop-calculating");
        let prop_sha_na_folders = resolve("prop-sha-na-folders");
        let prop_sha_computing = resolve("prop-sha-computing");
        let yes_label = resolve("yes-label");
        let no_label = resolve("no-label");
        let ext_label = resolve("ext-label");
        let perm_read = resolve("perm-read");
        let perm_write = resolve("perm-write");
        let perm_execute = resolve("perm-execute");
        let prefs_tab_general = resolve("prefs-tab-general");
        let about_tab = resolve("about-tab");
        let about_subtitle = resolve("about-subtitle");
        let about_version_label = resolve("about-version-label");
        let about_developer_label = resolve("about-developer-label");
        let about_developer_value = resolve("about-developer-value");
        let about_stage_label = resolve("about-stage-label");
        let about_stage_alpha = resolve("about-stage-alpha");
        let about_license_label = resolve("about-license-label");
        let about_stack_label = resolve("about-stack-label");
        let about_copy_info = resolve("about-copy-info");
        let about_info_copied = resolve("about-info-copied");
        let prop_contents_label = resolve("prop-contents-label");
        let prop_free_space_label = resolve("prop-free-space-label");
        let tip_new_tab = resolve("tip-new-tab");
        let tip_close_tab = resolve("tip-close-tab");
        let tip_language = resolve("tip-language");
        let tip_theme = resolve("tip-theme");
        let tip_back = resolve("tip-back");
        let tip_forward = resolve("tip-forward");
        let tip_up = resolve("tip-up");
        let tip_home = resolve("tip-home");
        let tip_edit_path = resolve("tip-edit-path");
        let tip_deep_search = resolve("tip-deep-search");
        let tip_view_grid = resolve("tip-view-grid");
        let tip_view_list = resolve("tip-view-list");
        let tip_new_item = resolve("tip-new-item");
        let tip_group_folders = resolve("tip-group-folders");
        let tip_zoom_out = resolve("tip-zoom-out");
        let tip_zoom_reset = resolve("tip-zoom-reset");
        let tip_zoom_in = resolve("tip-zoom-in");
        let tip_sort = resolve("tip-sort");
        let tip_sort_dir = resolve("tip-sort-dir");
        let tip_filter = resolve("tip-filter");
        let tip_hidden = resolve("tip-hidden");
        let tip_terminal = resolve("tip-terminal");
        let tip_favorite = resolve("tip-favorite");
        let tip_inspector = resolve("tip-inspector");
        let tip_preferences = resolve("tip-preferences");
        let tip_toggle_sidebar = resolve("tip-toggle-sidebar");
        let music = resolve("music");
        let videos = resolve("videos");
        let public_dir = resolve("public-dir");
        let templates = resolve("templates");
        let filesystem_root = resolve("filesystem-root");
        let mount_drive = resolve("mount-drive");
        let unmount_drive = resolve("unmount-drive");
        let unmounted_label = resolve("unmounted-label");
        let refresh_drives = resolve("refresh-drives");
        let tip_expand_home = resolve("tip-expand-home");
        let open_as_root = resolve("open-as-root");
        let root_mode_badge = resolve("root-mode-badge");
        let grid_center_label = resolve("grid-center-label");
        let grid_align_left = resolve("grid-align-left");
        let grid_align_center = resolve("grid-align-center");
        let tip_grid_alignment = resolve("tip-grid-alignment");
        let tip_system_icons = resolve("tip-system-icons");
        let perm_allow_exec = resolve("perm-allow-exec");
        let open_with = resolve("open-with");
        let prop_tab_open_with = resolve("prop-tab-open-with");
        let open_with_prompt = resolve("open-with-prompt");
        let open_with_default_app = resolve("open-with-default-app");
        let open_with_recommended_apps = resolve("open-with-recommended-apps");
        let open_with_other_apps = resolve("open-with-other-apps");
        let open_with_custom_hint = resolve("open-with-custom-hint");
        let open_with_custom_placeholder = resolve("open-with-custom-placeholder");
        let open_with_add_to_list = resolve("open-with-add-to-list");
        let open_with_set_default = resolve("open-with-set-default");
        let open_with_reset_default = resolve("open-with-reset-default");
        let open_with_open_once = resolve("open-with-open-once");
        let open_with_loading = resolve("open-with-loading");
        let open_with_empty = resolve("open-with-empty");
        let open_with_saved = resolve("open-with-saved");
        let open_with_reset_done = resolve("open-with-reset-done");
        let open_with_pick_exec = resolve("open-with-pick-exec");
        let br_title = resolve("br-title");
        let br_menu_item = resolve("br-menu-item");
        let br_clear_rules = resolve("br-clear-rules");
        let br_rules_header = resolve("br-rules-header");
        let br_preview_header = resolve("br-preview-header");
        let br_col_status = resolve("br-col-status");
        let br_col_original = resolve("br-col-original");
        let br_col_new = resolve("br-col-new");
        let br_unchanged = resolve("br-unchanged");
        let br_skipped_folder = resolve("br-skipped-folder");
        let br_conflict_internal = resolve("br-conflict-internal");
        let br_conflict_external = resolve("br-conflict-external");
        let br_invalid_name = resolve("br-invalid-name");
        let br_replace_title = resolve("br-replace-title");
        let br_find_label = resolve("br-find-label");
        let br_replace_label = resolve("br-replace-label");
        let br_where_label = resolve("br-where-label");
        let br_scope_anywhere = resolve("br-scope-anywhere");
        let br_scope_start_name = resolve("br-scope-start-name");
        let br_scope_end_name = resolve("br-scope-end-name");
        let br_scope_word_start = resolve("br-scope-word-start");
        let br_scope_word_end = resolve("br-scope-word-end");
        let br_scope_whole_word = resolve("br-scope-whole-word");
        let br_match_case = resolve("br-match-case");
        let br_first_only = resolve("br-first-only");
        let br_advanced_regex = resolve("br-advanced-regex");
        let br_use_regex = resolve("br-use-regex");
        let br_remove_title = resolve("br-remove-title");
        let br_remove_first = resolve("br-remove-first");
        let br_remove_last = resolve("br-remove-last");
        let br_remove_text_label = resolve("br-remove-text-label");
        let br_remove_digits = resolve("br-remove-digits");
        let br_remove_symbols = resolve("br-remove-symbols");
        let br_collapse_spaces = resolve("br-collapse-spaces");
        let br_insert_title = resolve("br-insert-title");
        let br_insert_text_label = resolve("br-insert-text-label");
        let br_insert_pos_label = resolve("br-insert-pos-label");
        let br_insert_start = resolve("br-insert-start");
        let br_insert_end = resolve("br-insert-end");
        let br_insert_offset_start = resolve("br-insert-offset-start");
        let br_insert_offset_end = resolve("br-insert-offset-end");
        let br_case_title = resolve("br-case-title");
        let br_case_keep = resolve("br-case-keep");
        let br_case_upper = resolve("br-case-upper");
        let br_case_lower = resolve("br-case-lower");
        let br_case_title_words = resolve("br-case-title-words");
        let br_case_sentence = resolve("br-case-sentence");
        let br_number_title = resolve("br-number-title");
        let br_number_append = resolve("br-number-append");
        let br_number_prepend = resolve("br-number-prepend");
        let br_number_replace = resolve("br-number-replace");
        let br_number_start = resolve("br-number-start");
        let br_number_step = resolve("br-number-step");
        let br_number_digits = resolve("br-number-digits");
        let br_number_sep = resolve("br-number-sep");
        let br_sort_label = resolve("br-sort-label");
        let br_sort_selection = resolve("br-sort-selection");
        let br_sort_name_asc = resolve("br-sort-name-asc");
        let br_sort_name_desc = resolve("br-sort-name-desc");
        let br_sort_date_desc = resolve("br-sort-date-desc");
        let br_sort_date_asc = resolve("br-sort-date-asc");
        let br_sort_size_asc = resolve("br-sort-size-asc");
        let br_sort_size_desc = resolve("br-sort-size-desc");
        let br_media_title = resolve("br-media-title");
        let br_media_exif_ymd = resolve("br-media-exif-ymd");
        let br_media_exif_datetime = resolve("br-media-exif-datetime");
        let br_media_exif_camera = resolve("br-media-exif-camera");
        let br_media_id3_track = resolve("br-media-id3-track");
        let br_media_id3_artist = resolve("br-media-id3-artist");
        let br_media_id3_album = resolve("br-media-id3-album");
        let br_media_id3_title = resolve("br-media-id3-title");
        let br_media_id3_year = resolve("br-media-id3-year");
        let br_media_loading = resolve("br-media-loading");
        let br_ext_title = resolve("br-ext-title");
        let br_ext_keep = resolve("br-ext-keep");
        let br_ext_lower = resolve("br-ext-lower");
        let br_ext_upper = resolve("br-ext-upper");
        let br_ext_replace = resolve("br-ext-replace");
        let br_ext_remove = resolve("br-ext-remove");
        let br_include_folders = resolve("br-include-folders");
        let br_apply = resolve("br-apply");
        let br_applying = resolve("br-applying");
        let br_undo_last = resolve("br-undo-last");
        let br_readonly_error = resolve("br-readonly-error");
        let arch_extract_here = resolve("arch-extract-here");
        let arch_extract_subdir = resolve("arch-extract-subdir");
        let arch_compress_as = resolve("arch-compress-as");
        let arch_add_7z = resolve("arch-add-7z");
        let arch_modal_title = resolve("arch-modal-title");
        let arch_output_name = resolve("arch-output-name");
        let arch_destination = resolve("arch-destination");
        let arch_format_label = resolve("arch-format-label");
        let arch_level_label = resolve("arch-level-label");
        let arch_format_7z = resolve("arch-format-7z");
        let arch_format_zip = resolve("arch-format-zip");
        let arch_format_tar = resolve("arch-format-tar");
        let arch_format_targz = resolve("arch-format-targz");
        let arch_level_store = resolve("arch-level-store");
        let arch_level_min = resolve("arch-level-min");
        let arch_level_normal = resolve("arch-level-normal");
        let arch_level_max = resolve("arch-level-max");
        let arch_tar_store_hint = resolve("arch-tar-store-hint");
        let arch_conflict_label = resolve("arch-conflict-label");
        let arch_conflict_rename = resolve("arch-conflict-rename");
        let arch_conflict_overwrite = resolve("arch-conflict-overwrite");
        let arch_conflict_skip = resolve("arch-conflict-skip");
        let arch_start_compress = resolve("arch-start-compress");
        let arch_minimize = resolve("arch-minimize");
        let arch_restore = resolve("arch-restore");
        let arch_compressing = resolve("arch-compressing");
        let arch_extracting = resolve("arch-extracting");
        let arch_done_compress = resolve("arch-done-compress");
        let arch_done_extract = resolve("arch-done-extract");
        let arch_cancelled = resolve("arch-cancelled");
        let arch_password_title = resolve("arch-password-title");
        let arch_password_hint = resolve("arch-password-hint");
        let arch_password_placeholder = resolve("arch-password-placeholder");
        let arch_password_wrong = resolve("arch-password-wrong");
        let arch_password_submit = resolve("arch-password-submit");
        let arch_password_toggle = resolve("arch-password-toggle");
        let dup_menu_action = resolve("dup-menu-action");
        let dup_modal_title = resolve("dup-modal-title");
        let dup_readonly_error = resolve("dup-readonly-error");
        let dup_confirm_level_label = resolve("dup-confirm-level-label");
        let dup_level_quick = resolve("dup-level-quick");
        let dup_level_standard = resolve("dup-level-standard");
        let dup_level_sha256 = resolve("dup-level-sha256");
        let dup_level_byte = resolve("dup-level-byte");
        let dup_min_size_label = resolve("dup-min-size-label");
        let dup_min_size_any = resolve("dup-min-size-any");
        let dup_min_size_1kb = resolve("dup-min-size-1kb");
        let dup_min_size_1mb = resolve("dup-min-size-1mb");
        let dup_min_size_10mb = resolve("dup-min-size-10mb");
        let dup_start_scan = resolve("dup-start-scan");
        let dup_rescan = resolve("dup-rescan");
        let dup_stop_scan = resolve("dup-stop-scan");
        let dup_opt_subfolders = resolve("dup-opt-subfolders");
        let dup_opt_hidden = resolve("dup-opt-hidden");
        let dup_opt_ignore_empty = resolve("dup-opt-ignore-empty");
        let dup_opt_ignore_hardlinks = resolve("dup-opt-ignore-hardlinks");
        let dup_quick_warning = resolve("dup-quick-warning");
        let dup_quick_select_label = resolve("dup-quick-select-label");
        let dup_keep_oldest = resolve("dup-keep-oldest");
        let dup_keep_newest = resolve("dup-keep-newest");
        let dup_keep_shortest = resolve("dup-keep-shortest");
        let dup_clear_selection = resolve("dup-clear-selection");
        let dup_badge_oldest = resolve("dup-badge-oldest");
        let dup_badge_hardlink = resolve("dup-badge-hardlink");
        let dup_verify_partial = resolve("dup-verify-partial");
        let dup_verify_md5 = resolve("dup-verify-md5");
        let dup_verify_sha256 = resolve("dup-verify-sha256");
        let dup_verify_byte = resolve("dup-verify-byte");
        let dup_stage_walking = resolve("dup-stage-walking");
        let dup_stage_partial = resolve("dup-stage-partial");
        let dup_stage_full = resolve("dup-stage-full");
        let dup_stage_sha256 = resolve("dup-stage-sha256");
        let dup_stage_byte = resolve("dup-stage-byte");
        let dup_all_selected_warning = resolve("dup-all-selected-warning");
        let dup_delete_permanent = resolve("dup-delete-permanent");
        let dup_move_to_trash = resolve("dup-move-to-trash");
        let dup_confirm_perm_delete = resolve("dup-confirm-perm-delete");
        let dup_empty_idle = resolve("dup-empty-idle");
        let dup_empty_none = resolve("dup-empty-none");
        let dup_group_prefix = resolve("dup-group-prefix");
        let dup_copies_word = resolve("dup-copies-word");
        let dup_each_word = resolve("dup-each-word");
        let dup_selected_word = resolve("dup-selected-word");
        let dup_groups_found = resolve("dup-groups-found");
        let dup_reclaimable = resolve("dup-reclaimable");
        let dup_footer_prefix = resolve("dup-footer-prefix");
        let dup_cancelled = resolve("dup-cancelled");
        let dup_stale_skipped = resolve("dup-stale-skipped");
        let dup_last_copy_protected = resolve("dup-last-copy-protected");
        let dup_delete_done = resolve("dup-delete-done");
        let ctx_send_to = resolve("ctx-send-to");
        let ctx_shortcut_desktop = resolve("ctx-shortcut-desktop");
        let ctx_shortcut_here_unix = resolve("ctx-shortcut-here-unix");
        let ctx_shortcut_here_win = resolve("ctx-shortcut-here-win");
        let shortcut_desktop_missing = resolve("shortcut-desktop-missing");
        let filter_ext_placeholder = resolve("filter-ext-placeholder");
        let undo_action = resolve("undo-action");
        let undo_trash_action = resolve("undo-trash-action");
        let moved_to_trash_undo_hint = resolve("moved-to-trash-undo-hint");
        let undo_nothing_in_trash = resolve("undo-nothing-in-trash");
        let undo_rename_done = resolve("undo-rename-done");
        let mount_disk_image = resolve("mount-disk-image");
        let unmount_disk_image = resolve("unmount-disk-image");
        let eject_safely = resolve("eject-safely");
        let image_mounted = resolve("image-mounted");
        let drive_ejected = resolve("drive-ejected");
        let drive_free_label = resolve("drive-free-label");
        let drive_click_to_mount = resolve("drive-click-to-mount");
        let tip_split_view = resolve("tip-split-view");
        let close_split_pane = resolve("close-split-pane");
        let drag_move_prefix = resolve("drag-move-prefix");
        let drag_copy_prefix = resolve("drag-copy-prefix");
        let drag_ctrl_copy_hint = resolve("drag-ctrl-copy-hint");
        let update_available_banner = resolve("update-available-banner");
        let update_download_btn = resolve("update-download-btn");
        let update_ignore_btn = resolve("update-ignore-btn");
        let update_check_btn = resolve("update-check-btn");
        let update_checking = resolve("update-checking");
        let update_up_to_date = resolve("update-up-to-date");
        let update_check_failed = resolve("update-check-failed");
        let update_auto_check_label = resolve("update-auto-check-label");
        let update_open_releases = resolve("update-open-releases");
        let tip_update_available = resolve("tip-update-available");
        let recent_files = resolve("recent-files");
        let prefs_recent_files_label = resolve("prefs-recent-files-label");
        let clear_recent_files = resolve("clear-recent-files");
        let remove_from_recent = resolve("remove-from-recent");
        let open_file_location = resolve("open-file-location");
        let open_target_location = resolve("open-target-location");
        let prefs_startup_location_label = resolve("prefs-startup-location-label");
        let startup_home = resolve("startup-home");
        let startup_last_visited = resolve("startup-last-visited");
        let ntfs_object_name = resolve("ntfs-object-name");
        let ntfs_principals_label = resolve("ntfs-principals-label");
        let ntfs_edit_hint = resolve("ntfs-edit-hint");
        let ntfs_edit_active_hint = resolve("ntfs-edit-active-hint");
        let ntfs_edit_btn = resolve("ntfs-edit-btn");
        let ntfs_done_edit_btn = resolve("ntfs-done-edit-btn");
        let ntfs_add_btn = resolve("ntfs-add-btn");
        let ntfs_remove_btn = resolve("ntfs-remove-btn");
        let ntfs_permissions_for = resolve("ntfs-permissions-for");
        let ntfs_allow_col = resolve("ntfs-allow-col");
        let ntfs_deny_col = resolve("ntfs-deny-col");
        let ntfs_full_control = resolve("ntfs-full-control");
        let ntfs_list_folder = resolve("ntfs-list-folder");
        let ntfs_delete_modify = resolve("ntfs-delete-modify");
        let ntfs_inherited_badge = resolve("ntfs-inherited-badge");
        let ntfs_explicit_badge = resolve("ntfs-explicit-badge");
        let ntfs_owner_label = resolve("ntfs-owner-label");
        let ntfs_inheritance_label = resolve("ntfs-inheritance-label");
        let ntfs_inheritance_on = resolve("ntfs-inheritance-on");
        let ntfs_inheritance_off = resolve("ntfs-inheritance-off");
        let ntfs_add_placeholder = resolve("ntfs-add-placeholder");
        let ntfs_loading = resolve("ntfs-loading");
        let posix_owner_group_header = resolve("posix-owner-group-header");
        let posix_octal_symbolic_header = resolve("posix-octal-symbolic-header");
        let posix_presets_label = resolve("posix-presets-label");
        let posix_preset_default = resolve("posix-preset-default");
        let posix_preset_exec = resolve("posix-preset-exec");
        let posix_preset_private = resolve("posix-preset-private");
        let posix_preset_readonly = resolve("posix-preset-readonly");
        let posix_preset_group = resolve("posix-preset-group");
        let posix_preset_shared = resolve("posix-preset-shared");
        let posix_scope_col = resolve("posix-scope-col");
        let posix_summary_col = resolve("posix-summary-col");
        let posix_others_sub = resolve("posix-others-sub");
        let posix_special_bits_header = resolve("posix-special-bits-header");
        let posix_quick_action_header = resolve("posix-quick-action-header");
        let posix_recursive_header = resolve("posix-recursive-header");
        let posix_apply_recursive = resolve("posix-apply-recursive");
        let analyze_space = resolve("analyze-space");
        let tip_analyze_space = resolve("tip-analyze-space");
        let disk_scan_eyebrow = resolve("disk-scan-eyebrow");
        let disk_scan_title = resolve("disk-scan-title");
        let disk_scan_current_item = resolve("disk-scan-current-item");
        let disk_scan_files_checked = resolve("disk-scan-files-checked");
        let disk_scan_data_checked = resolve("disk-scan-data-checked");
        let disk_scan_elapsed = resolve("disk-scan-elapsed");
        let disk_scan_hint = resolve("disk-scan-hint");
        let disk_results_title = resolve("disk-results-title");
        let disk_root_crumb = resolve("disk-root-crumb");
        let disk_treemap_title = resolve("disk-treemap-title");
        let disk_top_consumers = resolve("disk-top-consumers");
        let disk_by_type = resolve("disk-by-type");
        let disk_mark_cleanup = resolve("disk-mark-cleanup");
        let disk_reanalyze = resolve("disk-reanalyze");
        let disk_clear_selection = resolve("disk-clear-selection");
        let disk_reclaim_suffix = resolve("disk-reclaim-suffix");
        let disk_mode_files = resolve("disk-mode-files");
        let disk_mode_folders = resolve("disk-mode-folders");
        let disk_cat_iso = resolve("disk-cat-iso");
        let disk_cat_build = resolve("disk-cat-build");
        let disk_cat_archives = resolve("disk-cat-archives");
        let disk_cat_media = resolve("disk-cat-media");
        let disk_cat_docs = resolve("disk-cat-docs");
        let disk_cat_other = resolve("disk-cat-other");

        let inner = Box::leak(Box::new(StringsInner {
            language: lang,
            format,
            bundle,
        }));

        Self {
            inner,
            quick_access,
            drives,
            tags_and_system,
            documents,
            home,
            desktop,
            downloads,
            pictures,
            code_dev,
            starred,
            trash,
            preferences,
            search_placeholder,
            no_active_tab,
            follow_system,
            new_item,
            sort,
            hidden,
            folders,
            files,
            total_size,
            inspector_title,
            inspector_empty,
            permissions,
            modified,
            checksum,
            mime_type,
            selected_none,
            quicklook_hint,
            quicklook_title,
            close,
            col_name,
            col_size,
            col_type,
            col_perms,
            col_modified,
            free_of,
            loading,
            empty_dir,
            go_back,
            retry,
            root_fs,
            dir_badge,
            file_badge,
            binary_preview_hint,
            deep_search,
            deep_search_title,
            deep_search_empty,
            search_name_label,
            search_containing_label,
            search_containing_placeholder,
            search_size_label,
            search_date_label,
            search_type_label,
            search_hidden_label,
            search_streaming_hint,
            search_any,
            search_today,
            search_last_7d,
            search_last_30d,
            search_type_code,
            search_type_docs,
            search_type_images,
            search_type_audio,
            search_type_video,
            search_type_archives,
            cmd_palette_title,
            cmd_palette_placeholder,
            cmd_palette_empty,
            cmd_palette_hint,
            cmd_cat_nav,
            cmd_cat_search,
            cmd_cat_files,
            cmd_cat_view,
            cmd_cat_tools,
            cmd_focus_quick_filter,
            cmd_search_containing,
            cmd_clear_filters,
            cmd_check_updates,
            cmd_toggle_theme,
            cmd_view_grid,
            cmd_view_list,
            searching,
            computing_hash,
            new_folder,
            new_file,
            rename,
            copy,
            cut,
            paste,
            move_to_trash,
            delete_permanent,
            confirm,
            cancel,
            create,
            name_label,
            invalid_name,
            confirm_trash_msg,
            confirm_delete_msg,
            clipboard_copy_tag,
            clipboard_cut_tag,
            transferring,
            prefs_title,
            theme_label,
            lang_label,
            hero_card_label,
            decorations_label,
            default_view_label,
            view_grid,
            view_list,
            window_bar_native,
            window_bar_csd,
            pin_favorite,
            unpin_favorite,
            remove_favorite,
            empty_subdir,
            open_terminal,
            git_status_label,
            conflict_title,
            conflict_msg,
            conflict_rename,
            conflict_overwrite,
            conflict_skip,
            created,
            open_item,
            open_in_new_tab,
            copy_path,
            select_all,
            path_copied,
            properties,
            filter_all,
            filter_ext,
            directories_only,
            archive_readonly,
            grid_group_label,
            dirs_first_label,
            recursive_size_label,
            empty_trash,
            restore_item,
            confirm_empty_trash_msg,
            trash_emptied,
            trash_restored,
            location_label,
            copy_sha256,
            hash_copied,
            properties_header_prefix,
            prop_tab_general,
            prop_tab_security,
            prop_tab_history,
            prop_tab_metadata,
            prop_tab_permissions,
            prop_kind_label,
            prop_system_mime_label,
            prop_accessed_label,
            prop_quick_props,
            prop_readonly,
            prop_hidden,
            prop_advanced_settings,
            prop_confirm,
            prop_apply,
            prop_open_default,
            prop_owner,
            prop_group,
            prop_others,
            prop_octal_mode,
            prop_symlink_target,
            prop_archive_notice,
            prop_applied_notice,
            prop_pending_changes,
            prop_calculating,
            prop_sha_na_folders,
            prop_sha_computing,
            yes_label,
            no_label,
            ext_label,
            perm_read,
            perm_write,
            perm_execute,
            prefs_tab_general,
            about_tab,
            about_subtitle,
            about_version_label,
            about_developer_label,
            about_developer_value,
            about_stage_label,
            about_stage_alpha,
            about_license_label,
            about_stack_label,
            about_copy_info,
            about_info_copied,
            prop_contents_label,
            prop_free_space_label,
            tip_new_tab,
            tip_close_tab,
            tip_language,
            tip_theme,
            tip_back,
            tip_forward,
            tip_up,
            tip_home,
            tip_edit_path,
            tip_deep_search,
            tip_view_grid,
            tip_view_list,
            tip_new_item,
            tip_group_folders,
            tip_zoom_out,
            tip_zoom_reset,
            tip_zoom_in,
            tip_sort,
            tip_sort_dir,
            tip_filter,
            tip_hidden,
            tip_terminal,
            tip_favorite,
            tip_inspector,
            tip_preferences,
            tip_toggle_sidebar,
            music,
            videos,
            public_dir,
            templates,
            filesystem_root,
            mount_drive,
            unmount_drive,
            unmounted_label,
            refresh_drives,
            tip_expand_home,
            open_as_root,
            root_mode_badge,
            grid_center_label,
            grid_align_left,
            grid_align_center,
            tip_grid_alignment,
            tip_system_icons,
            perm_allow_exec,
            open_with,
            prop_tab_open_with,
            open_with_prompt,
            open_with_default_app,
            open_with_recommended_apps,
            open_with_other_apps,
            open_with_custom_hint,
            open_with_custom_placeholder,
            open_with_add_to_list,
            open_with_set_default,
            open_with_reset_default,
            open_with_open_once,
            open_with_loading,
            open_with_empty,
            open_with_saved,
            open_with_reset_done,
            open_with_pick_exec,
            br_title,
            br_menu_item,
            br_clear_rules,
            br_rules_header,
            br_preview_header,
            br_col_status,
            br_col_original,
            br_col_new,
            br_unchanged,
            br_skipped_folder,
            br_conflict_internal,
            br_conflict_external,
            br_invalid_name,
            br_replace_title,
            br_find_label,
            br_replace_label,
            br_where_label,
            br_scope_anywhere,
            br_scope_start_name,
            br_scope_end_name,
            br_scope_word_start,
            br_scope_word_end,
            br_scope_whole_word,
            br_match_case,
            br_first_only,
            br_advanced_regex,
            br_use_regex,
            br_remove_title,
            br_remove_first,
            br_remove_last,
            br_remove_text_label,
            br_remove_digits,
            br_remove_symbols,
            br_collapse_spaces,
            br_insert_title,
            br_insert_text_label,
            br_insert_pos_label,
            br_insert_start,
            br_insert_end,
            br_insert_offset_start,
            br_insert_offset_end,
            br_case_title,
            br_case_keep,
            br_case_upper,
            br_case_lower,
            br_case_title_words,
            br_case_sentence,
            br_number_title,
            br_number_append,
            br_number_prepend,
            br_number_replace,
            br_number_start,
            br_number_step,
            br_number_digits,
            br_number_sep,
            br_sort_label,
            br_sort_selection,
            br_sort_name_asc,
            br_sort_name_desc,
            br_sort_date_desc,
            br_sort_date_asc,
            br_sort_size_asc,
            br_sort_size_desc,
            br_media_title,
            br_media_exif_ymd,
            br_media_exif_datetime,
            br_media_exif_camera,
            br_media_id3_track,
            br_media_id3_artist,
            br_media_id3_album,
            br_media_id3_title,
            br_media_id3_year,
            br_media_loading,
            br_ext_title,
            br_ext_keep,
            br_ext_lower,
            br_ext_upper,
            br_ext_replace,
            br_ext_remove,
            br_include_folders,
            br_apply,
            br_applying,
            br_undo_last,
            br_readonly_error,
            arch_extract_here,
            arch_extract_subdir,
            arch_compress_as,
            arch_add_7z,
            arch_modal_title,
            arch_output_name,
            arch_destination,
            arch_format_label,
            arch_level_label,
            arch_format_7z,
            arch_format_zip,
            arch_format_tar,
            arch_format_targz,
            arch_level_store,
            arch_level_min,
            arch_level_normal,
            arch_level_max,
            arch_tar_store_hint,
            arch_conflict_label,
            arch_conflict_rename,
            arch_conflict_overwrite,
            arch_conflict_skip,
            arch_start_compress,
            arch_minimize,
            arch_restore,
            arch_compressing,
            arch_extracting,
            arch_done_compress,
            arch_done_extract,
            arch_cancelled,
            arch_password_title,
            arch_password_hint,
            arch_password_placeholder,
            arch_password_wrong,
            arch_password_submit,
            arch_password_toggle,
            dup_menu_action,
            dup_modal_title,
            dup_readonly_error,
            dup_confirm_level_label,
            dup_level_quick,
            dup_level_standard,
            dup_level_sha256,
            dup_level_byte,
            dup_min_size_label,
            dup_min_size_any,
            dup_min_size_1kb,
            dup_min_size_1mb,
            dup_min_size_10mb,
            dup_start_scan,
            dup_rescan,
            dup_stop_scan,
            dup_opt_subfolders,
            dup_opt_hidden,
            dup_opt_ignore_empty,
            dup_opt_ignore_hardlinks,
            dup_quick_warning,
            dup_quick_select_label,
            dup_keep_oldest,
            dup_keep_newest,
            dup_keep_shortest,
            dup_clear_selection,
            dup_badge_oldest,
            dup_badge_hardlink,
            dup_verify_partial,
            dup_verify_md5,
            dup_verify_sha256,
            dup_verify_byte,
            dup_stage_walking,
            dup_stage_partial,
            dup_stage_full,
            dup_stage_sha256,
            dup_stage_byte,
            dup_all_selected_warning,
            dup_delete_permanent,
            dup_move_to_trash,
            dup_confirm_perm_delete,
            dup_empty_idle,
            dup_empty_none,
            dup_group_prefix,
            dup_copies_word,
            dup_each_word,
            dup_selected_word,
            dup_groups_found,
            dup_reclaimable,
            dup_footer_prefix,
            dup_cancelled,
            dup_stale_skipped,
            dup_last_copy_protected,
            dup_delete_done,
            ctx_send_to,
            ctx_shortcut_desktop,
            ctx_shortcut_here_unix,
            ctx_shortcut_here_win,
            shortcut_desktop_missing,
            filter_ext_placeholder,
            undo_action,
            undo_trash_action,
            moved_to_trash_undo_hint,
            undo_nothing_in_trash,
            undo_rename_done,
            mount_disk_image,
            unmount_disk_image,
            eject_safely,
            image_mounted,
            drive_ejected,
            drive_free_label,
            drive_click_to_mount,
            tip_split_view,
            close_split_pane,
            drag_move_prefix,
            drag_copy_prefix,
            drag_ctrl_copy_hint,
            update_available_banner,
            update_download_btn,
            update_ignore_btn,
            update_check_btn,
            update_checking,
            update_up_to_date,
            update_check_failed,
            update_auto_check_label,
            update_open_releases,
            tip_update_available,
            recent_files,
            prefs_recent_files_label,
            clear_recent_files,
            remove_from_recent,
            open_file_location,
            open_target_location,
            prefs_startup_location_label,
            startup_home,
            startup_last_visited,
            ntfs_object_name,
            ntfs_principals_label,
            ntfs_edit_hint,
            ntfs_edit_active_hint,
            ntfs_edit_btn,
            ntfs_done_edit_btn,
            ntfs_add_btn,
            ntfs_remove_btn,
            ntfs_permissions_for,
            ntfs_allow_col,
            ntfs_deny_col,
            ntfs_full_control,
            ntfs_list_folder,
            ntfs_delete_modify,
            ntfs_inherited_badge,
            ntfs_explicit_badge,
            ntfs_owner_label,
            ntfs_inheritance_label,
            ntfs_inheritance_on,
            ntfs_inheritance_off,
            ntfs_add_placeholder,
            ntfs_loading,
            posix_owner_group_header,
            posix_octal_symbolic_header,
            posix_presets_label,
            posix_preset_default,
            posix_preset_exec,
            posix_preset_private,
            posix_preset_readonly,
            posix_preset_group,
            posix_preset_shared,
            posix_scope_col,
            posix_summary_col,
            posix_others_sub,
            posix_special_bits_header,
            posix_quick_action_header,
            posix_recursive_header,
            posix_apply_recursive,
            analyze_space,
            tip_analyze_space,
            disk_scan_eyebrow,
            disk_scan_title,
            disk_scan_current_item,
            disk_scan_files_checked,
            disk_scan_data_checked,
            disk_scan_elapsed,
            disk_scan_hint,
            disk_results_title,
            disk_root_crumb,
            disk_treemap_title,
            disk_top_consumers,
            disk_by_type,
            disk_mark_cleanup,
            disk_reanalyze,
            disk_clear_selection,
            disk_reclaim_suffix,
            disk_mode_files,
            disk_mode_folders,
            disk_cat_iso,
            disk_cat_build,
            disk_cat_archives,
            disk_cat_media,
            disk_cat_docs,
            disk_cat_other,
        }
    }
}

fn ftl_source(lang: Language) -> &'static str {
    match lang {
        Language::EnUs => FTL_EN_US,
        Language::PtBr => FTL_PT_BR,
        Language::EsEs => FTL_ES_ES,
        Language::FrFr => FTL_FR_FR,
        Language::DeDe => FTL_DE_DE,
        Language::ItIt => FTL_IT_IT,
    }
}

fn locale_format_spec(lang: Language) -> LocaleFormatSpec {
    match lang {
        Language::PtBr => LocaleFormatSpec {
            thousands_sep: '.',
            months_short: [
                "Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez",
            ],
        },
        Language::EnUs => LocaleFormatSpec {
            thousands_sep: ',',
            months_short: [
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ],
        },
        Language::EsEs => LocaleFormatSpec {
            thousands_sep: '.',
            months_short: [
                "Ene", "Feb", "Mar", "Abr", "May", "Jun", "Jul", "Ago", "Sep", "Oct", "Nov", "Dic",
            ],
        },
        Language::FrFr => LocaleFormatSpec {
            thousands_sep: ' ',
            months_short: [
                "Jan", "Fév", "Mar", "Avr", "Mai", "Jun", "Jul", "Aoû", "Sep", "Oct", "Nov", "Déc",
            ],
        },
        Language::DeDe => LocaleFormatSpec {
            thousands_sep: '.',
            months_short: [
                "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
            ],
        },
        Language::ItIt => LocaleFormatSpec {
            thousands_sep: '.',
            months_short: [
                "Gen", "Feb", "Mar", "Apr", "Mag", "Giu", "Lug", "Ago", "Set", "Ott", "Nov", "Dic",
            ],
        },
    }
}

fn build_bundle(lang: Language) -> FluentBundle<FluentResource> {
    let langid: LanguageIdentifier = lang
        .bcp47()
        .parse()
        .expect("embedded BCP-47 locale identifier must parse");
    let mut bundle = FluentBundle::new_concurrent(vec![langid]);
    // Disable Unicode BiDi isolation marks so Iced renders cleanly without box glyphs.
    bundle.set_use_isolating(false);
    let res = FluentResource::try_new(ftl_source(lang).to_owned())
        .expect("embedded FTL resource must parse cleanly");
    bundle
        .add_resource(res)
        .expect("embedded FTL resource has no duplicate IDs");
    bundle
}

fn try_format_in_bundle(
    bundle: &FluentBundle<FluentResource>,
    id: &str,
    args: Option<&FluentArgs<'_>>,
) -> Option<String> {
    let msg = bundle.get_message(id)?;
    let pattern = msg.value()?;
    let mut errors = Vec::new();
    let formatted = bundle.format_pattern(pattern, args, &mut errors);
    if errors.is_empty() {
        Some(formatted.into_owned())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::ast::{Entry, Expression, InlineExpression, PatternElement};
    use std::collections::{BTreeMap, BTreeSet};

    fn collect_ftl_keys_and_vars(source: &str) -> BTreeMap<String, BTreeSet<String>> {
        let ast = fluent_syntax::parser::parse(source).expect("FTL must parse");
        let mut out = BTreeMap::new();
        for entry in ast.body {
            if let Entry::Message(msg) = entry {
                let mut vars = BTreeSet::new();
                if let Some(pattern) = msg.value {
                    collect_pattern_vars(&pattern.elements, &mut vars);
                }
                out.insert(msg.id.name.to_owned(), vars);
            }
        }
        out
    }

    fn collect_pattern_vars<S: AsRef<str>>(
        elements: &[PatternElement<S>],
        vars: &mut BTreeSet<String>,
    ) {
        for elem in elements {
            if let PatternElement::Placeable { expression } = elem {
                collect_expr_vars(expression, vars);
            }
        }
    }

    fn collect_expr_vars<S: AsRef<str>>(expr: &Expression<S>, vars: &mut BTreeSet<String>) {
        match expr {
            Expression::Select { selector, variants } => {
                collect_inline_vars(selector, vars);
                for variant in variants {
                    collect_pattern_vars(&variant.value.elements, vars);
                }
            }
            Expression::Inline(inline) => collect_inline_vars(inline, vars),
        }
    }

    fn collect_inline_vars<S: AsRef<str>>(
        inline: &InlineExpression<S>,
        vars: &mut BTreeSet<String>,
    ) {
        if let InlineExpression::VariableReference { id } = inline {
            vars.insert(id.name.as_ref().to_owned());
        }
    }

    #[test]
    fn all_locales_match_canonical_keys_and_variables() {
        let base = collect_ftl_keys_and_vars(ftl_source(Language::EnUs));
        assert!(!base.is_empty());

        for lang in Language::ALL {
            let cur = collect_ftl_keys_and_vars(ftl_source(lang));
            let missing: Vec<_> = base.keys().filter(|k| !cur.contains_key(*k)).collect();
            let extra: Vec<_> = cur.keys().filter(|k| !base.contains_key(*k)).collect();
            assert!(
                missing.is_empty(),
                "{} missing keys: {missing:?}",
                lang.bcp47()
            );
            assert!(extra.is_empty(), "{} extra keys: {extra:?}", lang.bcp47());

            for (key, base_vars) in &base {
                let cur_vars = &cur[key];
                assert_eq!(
                    cur_vars,
                    base_vars,
                    "{} variable mismatch for key '{key}'",
                    lang.bcp47()
                );
            }

            let strings = Strings::for_language(lang);
            assert_ne!(strings.format_items_count(1), "");
            assert_ne!(strings.format_items_count(5), "");
            assert_ne!(strings.format_folders_count(2), "");
            assert_ne!(strings.format_files_count(3), "");
            assert_ne!(strings.format_shortcuts_created(1), "");
            assert_ne!(strings.format_shortcuts_created(3), "");
            assert_ne!(strings.format_undo_restored(1), "");
            assert_ne!(strings.format_undo_restored(3), "");
            assert_ne!(strings.format_selected_multi(2, "4.0 KB"), "");
            assert_ne!(strings.format_exact_bytes(1024), "");
            assert_ne!(strings.format_age(SystemTime::now()), "");
            assert_ne!(strings.format_timestamp_short(SystemTime::now()), "");
        }
    }

    #[test]
    fn resolves_locale_tags_and_plural_forms() {
        assert_eq!(
            Language::from_locale_tag("pt_BR.UTF-8"),
            Some(Language::PtBr)
        );
        assert_eq!(Language::from_locale_tag("es-419"), Some(Language::EsEs));
        assert_eq!(Language::from_locale_tag("de"), Some(Language::DeDe));
        assert_eq!(Language::from_locale_tag("unknown"), None);

        let pt = Strings::for_language(Language::PtBr);
        assert_eq!(pt.format_items_count(1), "1 item");
        assert_eq!(pt.format_items_count(4), "4 itens");
        assert_eq!(pt.format_grouped(14_931_200), "14.931.200");

        let en = Strings::for_language(Language::EnUs);
        assert_eq!(en.format_items_count(1), "1 item");
        assert_eq!(en.format_items_count(4), "4 items");
        assert_eq!(en.format_grouped(14_931_200), "14,931,200");

        let de = Strings::for_language(Language::DeDe);
        assert_eq!(de.format_items_count(1), "1 Element");
        assert_eq!(de.format_items_count(4), "4 Elemente");
    }
}
