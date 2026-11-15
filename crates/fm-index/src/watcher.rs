use std::path::PathBuf;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebouncedEventKind, Debouncer, new_debouncer};
use tokio::sync::mpsc::UnboundedSender;

/// Eventos normalizados de mudança em diretórios observados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventKind {
    Changed,
}

/// Evento emitido pelo watcher após debounce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEvent {
    pub watched_root: PathBuf,
    pub kind: WatchEventKind,
    pub has_visible_change: bool,
}

/// Inicia observação não-recursiva sobre `root` com janela de debounce de 250ms.
pub fn start_directory_watcher(
    root: PathBuf,
    tx: UnboundedSender<WatchEvent>,
) -> notify::Result<Debouncer<RecommendedWatcher>> {
    let watched_root = root.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(250),
        move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            let Ok(events) = res else {
                return;
            };
            let mut any_relevant = false;
            let mut has_visible_change = false;
            for ev in &events {
                if matches!(
                    ev.kind,
                    DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
                ) {
                    any_relevant = true;
                    let is_hidden = ev
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with('.'));
                    if !is_hidden && ev.path != watched_root {
                        has_visible_change = true;
                    }
                }
            }
            if any_relevant {
                let _ = tx.send(WatchEvent {
                    watched_root: watched_root.clone(),
                    kind: WatchEventKind::Changed,
                    has_visible_change,
                });
            }
        },
    )?;

    debouncer
        .watcher()
        .watch(&root, RecursiveMode::NonRecursive)?;
    Ok(debouncer)
}
