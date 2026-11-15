//! Watchers de sistema de arquivos com debounce e busca fuzzy instantânea (`nucleo-matcher`).

pub mod search;
pub mod watcher;

pub use search::{SearchHit, deep_search, fuzzy_rank_entries};
pub use watcher::{WatchEvent, WatchEventKind, start_directory_watcher};
