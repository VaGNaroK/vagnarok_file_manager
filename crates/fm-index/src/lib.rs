//! Watchers de sistema de arquivos com debounce, filtros rápidos e busca avançada.

pub mod quick_filter;
pub mod search;
pub mod watcher;

pub use quick_filter::quick_filter_entries;
pub use search::{
    AdvancedSearchQuery, ContentMatchSnippet, DEFAULT_MAX_CONTENT_FILE_BYTES, DateFilter,
    FileTypeFilter, SearchHit, SizeFilter, advanced_search, deep_search, deep_search_with_hidden,
    fuzzy_rank_entries,
};
pub use watcher::{WatchEvent, WatchEventKind, start_directory_watcher};
