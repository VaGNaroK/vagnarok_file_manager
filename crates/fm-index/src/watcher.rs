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
}

/// Inicia observação não-recursiva sobre `root` com janela de debounce de 80ms.
pub fn start_directory_watcher(
    root: PathBuf,
    tx: UnboundedSender<WatchEvent>,
) -> notify::Result<Debouncer<RecommendedWatcher>> {
    let watched_root = root.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(80),
        move |res: Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>| {
            let Ok(events) = res else {
                return;
            };
            let relevant = events.iter().any(|ev| {
                matches!(
                    ev.kind,
                    DebouncedEventKind::Any | DebouncedEventKind::AnyContinuous
                )
            });
            if relevant {
                let _ = tx.send(WatchEvent {
                    watched_root: watched_root.clone(),
                    kind: WatchEventKind::Changed,
                });
            }
        },
    )?;

    debouncer
        .watcher()
        .watch(&root, RecursiveMode::NonRecursive)?;
    Ok(debouncer)
}
