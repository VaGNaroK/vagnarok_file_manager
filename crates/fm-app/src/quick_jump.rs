use std::collections::HashSet;
use std::path::{Path, PathBuf};

use compact_str::CompactString;
use fm_core::Strings;
use fm_index::QuickJumpDirHit;

use crate::app::App;

/// Origem de uma pasta exibida no Quick Jump (`Ctrl+P`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickJumpSource {
    CurrentDir,
    Favorite,
    Recent,
    Subfolder,
}

impl QuickJumpSource {
    pub fn label(self, strings: Strings) -> &'static str {
        match self {
            Self::CurrentDir => strings.quick_jump_src_current,
            Self::Favorite => strings.quick_jump_src_favorite,
            Self::Recent => strings.quick_jump_src_recent,
            Self::Subfolder => strings.quick_jump_src_subfolder,
        }
    }

    fn priority_bonus(self) -> u32 {
        match self {
            Self::CurrentDir => 40_000,
            Self::Favorite => 30_000,
            Self::Recent => 20_000,
            Self::Subfolder => 0,
        }
    }
}

/// Item individual ranqueado no Quick Jump (`Ctrl+P`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickJumpItem {
    pub path: PathBuf,
    pub name: CompactString,
    pub display_path: CompactString,
    pub source: QuickJumpSource,
    pub score: u32,
}

fn dir_name_for_path(path: &Path) -> CompactString {
    path.file_name()
        .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
        .unwrap_or_else(|| CompactString::from(path.to_string_lossy().as_ref()))
}

/// Coleta candidatos instantâneos em memória (0 ms de disco) a partir do estado atual do `App`.
pub fn collect_memory_candidates(app: &App) -> Vec<(PathBuf, QuickJumpSource)> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    // 1. Subpastas da aba ativa
    if let Some(tab) = app.current_tab() {
        for entry in &tab.raw_entries {
            if entry.kind.is_dir_like()
                && !fm_core::is_virtual_readonly_dir(&entry.path)
                && seen.insert(entry.path.clone())
            {
                out.push((entry.path.clone(), QuickJumpSource::CurrentDir));
            }
        }
    }

    // 2. Favoritos e locais do usuário / unidades
    for fav in &app.config.favorites {
        if seen.insert(fav.clone()) {
            out.push((fav.clone(), QuickJumpSource::Favorite));
        }
    }
    if seen.insert(app.places.home.clone()) {
        out.push((app.places.home.clone(), QuickJumpSource::Favorite));
    }
    let opt_places = [
        &app.places.desktop,
        &app.places.documents,
        &app.places.downloads,
        &app.places.music,
        &app.places.pictures,
        &app.places.videos,
    ];
    for place in opt_places.into_iter().flatten() {
        if seen.insert(place.clone()) {
            out.push((place.clone(), QuickJumpSource::Favorite));
        }
    }
    for drive in &app.drives {
        if seen.insert(drive.mount_path.clone()) {
            out.push((drive.mount_path.clone(), QuickJumpSource::Favorite));
        }
    }

    // 3. Saltos recentes e histórico das abas
    for path in &app.quick_jump_recent {
        if seen.insert(path.clone()) {
            out.push((path.clone(), QuickJumpSource::Recent));
        }
    }
    for tab in &app.tabs {
        if !fm_core::is_virtual_readonly_dir(&tab.current_path)
            && seen.insert(tab.current_path.clone())
        {
            out.push((tab.current_path.clone(), QuickJumpSource::Recent));
        }
        for path in tab
            .history_back
            .iter()
            .rev()
            .chain(tab.history_forward.iter())
        {
            if !fm_core::is_virtual_readonly_dir(path) && seen.insert(path.clone()) {
                out.push((path.clone(), QuickJumpSource::Recent));
            }
        }
    }
    for recent_file in &app.config.recent_files {
        if let Some(parent) = recent_file.parent()
            && seen.insert(parent.to_path_buf())
        {
            out.push((parent.to_path_buf(), QuickJumpSource::Recent));
        }
    }

    out
}

/// Mescla candidatos em memória com resultados assíncronos de disco, deduplica e ranqueia.
pub fn rank_quick_jump_items(
    memory_candidates: &[(PathBuf, QuickJumpSource)],
    disk_hits: &[QuickJumpDirHit],
    query: &str,
    max_results: usize,
) -> Vec<QuickJumpItem> {
    let trimmed = query.trim();
    let mut merged: Vec<(PathBuf, QuickJumpSource, u32)> = Vec::new();
    let mut seen = HashSet::new();

    for (path, source) in memory_candidates {
        if seen.insert(path.clone()) {
            merged.push((path.clone(), *source, 0));
        }
    }
    for hit in disk_hits {
        if seen.insert(hit.path.clone()) {
            merged.push((hit.path.clone(), QuickJumpSource::Subfolder, hit.score));
        }
    }

    if trimmed.is_empty() {
        let mut items: Vec<QuickJumpItem> = merged
            .into_iter()
            .take(max_results)
            .map(|(path, source, _)| QuickJumpItem {
                name: dir_name_for_path(&path),
                display_path: CompactString::from(path.to_string_lossy().as_ref()),
                path,
                source,
                score: source.priority_bonus(),
            })
            .collect();
        items.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        return items;
    }

    let query_has_sep = trimmed.contains('/') || trimmed.contains('\\');
    let query_norm = trimmed.replace('\\', "/").to_lowercase();

    let candidates_str: Vec<String> = merged
        .iter()
        .map(|(path, _, _)| {
            if query_has_sep {
                path.to_string_lossy().replace('\\', "/")
            } else {
                dir_name_for_path(path).to_string()
            }
        })
        .collect();
    let refs: Vec<&str> = candidates_str.iter().map(String::as_str).collect();
    let ranked_indices = fm_index::fuzzy_rank_indices(&refs, trimmed);

    let mut items: Vec<QuickJumpItem> = Vec::new();
    for (rank_pos, idx) in ranked_indices.into_iter().enumerate() {
        let (ref path, source, disk_score) = merged[idx];
        let name = dir_name_for_path(path);
        let name_lower = name.to_lowercase();

        let match_bonus = if name_lower == query_norm {
            100_000
        } else if name_lower.starts_with(&query_norm) {
            60_000
        } else if name_lower.contains(&query_norm) {
            30_000
        } else {
            0
        };
        let rank_score = (10_000_usize.saturating_sub(rank_pos * 50)) as u32;
        let total_score = source
            .priority_bonus()
            .saturating_add(match_bonus)
            .saturating_add(disk_score.min(25_000))
            .saturating_add(rank_score);

        items.push(QuickJumpItem {
            path: path.clone(),
            name,
            display_path: CompactString::from(path.to_string_lossy().as_ref()),
            source,
            score: total_score,
        });
    }

    items.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.display_path.len().cmp(&b.display_path.len()))
            .then_with(|| a.display_path.cmp(&b.display_path))
    });
    items.truncate(max_results);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_quick_jump_deduplicates_and_prioritizes_exact_and_current_dir() {
        let mem = vec![
            (
                PathBuf::from("/home/user/Downloads"),
                QuickJumpSource::Favorite,
            ),
            (
                PathBuf::from("/home/user/projetos/vagnarok"),
                QuickJumpSource::CurrentDir,
            ),
        ];
        let disk = vec![
            QuickJumpDirHit {
                path: PathBuf::from("/home/user/projetos/vagnarok"),
                name: CompactString::from("vagnarok"),
                score: 90_000,
            },
            QuickJumpDirHit {
                path: PathBuf::from("/home/user/backup/vagnarok_old"),
                name: CompactString::from("vagnarok_old"),
                score: 50_000,
            },
        ];

        let ranked = rank_quick_jump_items(&mem, &disk, "vagnarok", 20);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].path, PathBuf::from("/home/user/projetos/vagnarok"));
        assert_eq!(ranked[0].source, QuickJumpSource::CurrentDir);
        assert_eq!(
            ranked[1].path,
            PathBuf::from("/home/user/backup/vagnarok_old")
        );
    }
}
