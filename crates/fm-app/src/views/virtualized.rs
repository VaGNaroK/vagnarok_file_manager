use std::ops::Range;

use fm_core::{FileEntry, Strings};

pub const DETAIL_ROW_HEIGHT: f32 = 34.0;
pub const DETAIL_ROW_SPACING: f32 = 4.0;
pub const DETAIL_ROW_STEP: f32 = DETAIL_ROW_HEIGHT + DETAIL_ROW_SPACING;

pub const GRID_HEADER_HEIGHT: f32 = 24.0;
pub const GRID_BAND_SPACING: f32 = 16.0;
pub const GRID_GAP: f32 = 18.0;
pub const GRID_CARD_WIDTH_MIN: f32 = 104.0;
pub const GRID_CARD_WIDTH_MAX: f32 = 260.0;
pub const GRID_CARD_WIDTH_DEFAULT: f32 = 140.0;

pub fn clamp_grid_card_width(width: f32) -> f32 {
    if width.is_finite() {
        width.clamp(GRID_CARD_WIDTH_MIN, GRID_CARD_WIDTH_MAX)
    } else {
        GRID_CARD_WIDTH_DEFAULT
    }
}

pub fn grid_card_height(card_width: f32) -> f32 {
    (clamp_grid_card_width(card_width) * 0.78)
        .round()
        .clamp(96.0, 196.0)
}

pub fn grid_columns_for_width(available_width: f32, card_width: f32) -> usize {
    let w = clamp_grid_card_width(card_width);
    let usable = (available_width - 28.0).max(w);
    (((usable + GRID_GAP) / (w + GRID_GAP)).floor() as usize).max(1)
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualWindow {
    pub range: Range<usize>,
    pub top_spacer: f32,
    pub bottom_spacer: f32,
}

/// Calcula a janela visível para uma lista homogênea de altura fixa (Detail Table / Column).
pub fn compute_fixed_window(
    total_items: usize,
    item_step: f32,
    scroll_y: f32,
    viewport_height: f32,
    overscan_rows: usize,
) -> VirtualWindow {
    if total_items == 0 {
        return VirtualWindow {
            range: 0..0,
            top_spacer: 0.0,
            bottom_spacer: 0.0,
        };
    }

    let safe_vh = viewport_height.max(400.0);
    let safe_scroll = scroll_y.max(0.0);

    let first_visible = (safe_scroll / item_step).floor() as usize;
    let visible_count = (safe_vh / item_step).ceil() as usize + 1;

    let start = first_visible.saturating_sub(overscan_rows).min(total_items);
    let end = (first_visible + visible_count + overscan_rows).min(total_items);

    let top_spacer = start as f32 * item_step;
    let bottom_spacer = (total_items.saturating_sub(end)) as f32 * item_step;

    VirtualWindow {
        range: start..end,
        top_spacer,
        bottom_spacer,
    }
}

pub enum GridBand<'a> {
    SectionHeader {
        title: &'a str,
        count: usize,
        perms: &'a str,
    },
    Items(Vec<(usize, &'a FileEntry)>),
}

impl<'a> GridBand<'a> {
    fn height_with(&self, row_height: f32) -> f32 {
        match self {
            Self::SectionHeader { .. } => GRID_HEADER_HEIGHT,
            Self::Items(_) => row_height,
        }
    }
}

/// Constrói bandas de cabeçalho + linhas de grade e calcula em O(log N) quais estão no viewport.
pub fn build_grid_bands<'a>(
    dirs: &[(usize, &'a FileEntry)],
    files: &[(usize, &'a FileEntry)],
    cols: usize,
    folders_title: &'a str,
    files_title: &'a str,
) -> Vec<GridBand<'a>> {
    let cols = cols.max(1);
    let mut bands = Vec::new();

    if !dirs.is_empty() {
        bands.push(GridBand::SectionHeader {
            title: folders_title,
            count: dirs.len(),
            perms: "drwxr-xr-x",
        });
        for chunk in dirs.chunks(cols) {
            bands.push(GridBand::Items(chunk.to_vec()));
        }
    }

    if !files.is_empty() {
        bands.push(GridBand::SectionHeader {
            title: files_title,
            count: files.len(),
            perms: "-rw-r--r--",
        });
        for chunk in files.chunks(cols) {
            bands.push(GridBand::Items(chunk.to_vec()));
        }
    }

    bands
}

/// Constrói bandas de grade unificada mantendo exatamente a ordem de `entries`, sem cabeçalhos de seção.
pub fn build_unified_grid_bands<'a>(entries: &'a [FileEntry], cols: usize) -> Vec<GridBand<'a>> {
    let cols = cols.max(1);
    let indexed: Vec<(usize, &'a FileEntry)> = entries.iter().enumerate().collect();
    indexed
        .chunks(cols)
        .map(|chunk| GridBand::Items(chunk.to_vec()))
        .collect()
}

/// Constrói as bandas da aba respeitando `group_folders` e `cols`.
pub fn build_tab_grid_bands<'a>(
    entries: &'a [FileEntry],
    cols: usize,
    group_folders: bool,
    strings: Strings,
) -> Vec<GridBand<'a>> {
    if !group_folders {
        return build_unified_grid_bands(entries, cols);
    }
    let dirs: Vec<(usize, &FileEntry)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind.is_dir_like())
        .collect();
    let files: Vec<(usize, &FileEntry)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.kind.is_dir_like())
        .collect();
    build_grid_bands(&dirs, &files, cols, strings.folders, strings.files)
}

pub fn compute_band_window_with_row_height(
    bands: &[GridBand<'_>],
    row_height: f32,
    scroll_y: f32,
    viewport_height: f32,
    overscan_px: f32,
) -> VirtualWindow {
    if bands.is_empty() {
        return VirtualWindow {
            range: 0..0,
            top_spacer: 0.0,
            bottom_spacer: 0.0,
        };
    }

    let mut offsets = Vec::with_capacity(bands.len() + 1);
    let mut acc = 0.0_f32;
    offsets.push(0.0);
    for (i, band) in bands.iter().enumerate() {
        acc += band.height_with(row_height);
        if i + 1 < bands.len() {
            acc += GRID_BAND_SPACING;
        }
        offsets.push(acc);
    }

    let total_height = *offsets.last().unwrap_or(&0.0);
    let safe_vh = viewport_height.max(500.0);
    let min_y = (scroll_y - overscan_px).max(0.0);
    let max_y = (scroll_y + safe_vh + overscan_px).min(total_height);

    // `offsets[i + 1]` é o fim da banda `i`; a primeira banda visível satisfaz `offsets[i + 1] > min_y`.
    let start = offsets[1..].partition_point(|&end_y| end_y <= min_y);
    let end = offsets[..bands.len()]
        .partition_point(|&start_y| start_y < max_y)
        .max(start);

    let top_spacer = offsets[start];
    let bottom_spacer = (total_height - offsets[end]).max(0.0);

    VirtualWindow {
        range: start..end,
        top_spacer,
        bottom_spacer,
    }
}

/// Retorna `(top_y, bottom_y)` da banda de grade que contém `entry_idx`.
pub fn grid_band_y_span(
    bands: &[GridBand<'_>],
    entry_idx: usize,
    row_height: f32,
) -> Option<(f32, f32)> {
    let mut y = 0.0_f32;
    for (i, band) in bands.iter().enumerate() {
        let h = band.height_with(row_height);
        if let GridBand::Items(chunk) = band
            && chunk.iter().any(|(idx, _)| *idx == entry_idx)
        {
            return Some((y, y + h));
        }
        y += h;
        if i + 1 < bands.len() {
            y += GRID_BAND_SPACING;
        }
    }
    None
}

/// Move a seleção na grade em 2D (`dx` horizontal, `dy` vertical por linha).
pub fn step_grid_selection(
    bands: &[GridBand<'_>],
    current_entry_idx: Option<usize>,
    dx: isize,
    dy: isize,
) -> Option<usize> {
    let rows: Vec<Vec<usize>> = bands
        .iter()
        .filter_map(|b| match b {
            GridBand::Items(chunk) if !chunk.is_empty() => {
                Some(chunk.iter().map(|(idx, _)| *idx).collect())
            }
            _ => None,
        })
        .collect();
    if rows.is_empty() {
        return None;
    }

    let Some(cur) = current_entry_idx else {
        return if dx < 0 || dy < 0 {
            rows.last().and_then(|r| r.last().copied())
        } else {
            rows.first().and_then(|r| r.first().copied())
        };
    };

    let mut pos: Option<(usize, usize)> = None;
    for (r_idx, row) in rows.iter().enumerate() {
        if let Some(c_idx) = row.iter().position(|&idx| idx == cur) {
            pos = Some((r_idx, c_idx));
            break;
        }
    }
    let Some((r_idx, c_idx)) = pos else {
        return rows.first().and_then(|r| r.first().copied());
    };

    if dy != 0 {
        let target_r = (r_idx as isize + dy).clamp(0, rows.len() as isize - 1) as usize;
        let target_row = &rows[target_r];
        let target_c = c_idx.min(target_row.len().saturating_sub(1));
        return Some(target_row[target_c]);
    }

    if dx != 0 {
        let flat: Vec<usize> = rows.into_iter().flatten().collect();
        if let Some(flat_pos) = flat.iter().position(|&idx| idx == cur) {
            let next = (flat_pos as isize + dx).clamp(0, flat.len() as isize - 1) as usize;
            return Some(flat[next]);
        }
    }

    Some(cur)
}

#[cfg(test)]
mod tests {
    use super::*;
    use compact_str::CompactString;
    use fm_core::{EntryId, EntryKind, PermissionsMeta};
    use std::path::PathBuf;

    fn dummy_file(i: usize) -> FileEntry {
        FileEntry {
            id: EntryId(i as u64 + 1),
            name: CompactString::from(format!("file_{i}.rs")),
            path: PathBuf::from(format!("/tmp/file_{i}.rs")),
            extension: Some(CompactString::new("rs")),
            kind: EntryKind::File,
            size_bytes: 1024,
            created_at: None,
            modified_at: None,
            is_hidden: false,
            permissions: PermissionsMeta::PosixMode(0o644),
            child_count: None,
            mime_badge: Some(CompactString::new("RS")),
        }
    }

    #[test]
    fn detail_window_caps_rendered_widgets_for_ten_thousand_items() {
        let win = compute_fixed_window(10_000, DETAIL_ROW_STEP, 4_000.0, 600.0, 4);
        assert!(win.range.len() < 30);
        assert!(win.top_spacer > 0.0);
        assert!(win.bottom_spacer > 0.0);
    }

    #[test]
    fn grid_band_window_preserves_total_height_and_limits_rows() {
        let entries: Vec<FileEntry> = (0..10_000).map(dummy_file).collect();
        let indexed: Vec<(usize, &FileEntry)> = entries.iter().enumerate().collect();
        let bands = build_grid_bands(&[], &indexed, 6, "PASTAS", "ARQUIVOS");

        let win = compute_band_window_with_row_height(&bands, 136.0, 10_000.0, 720.0, 200.0);
        assert!(win.range.len() <= 12);
        assert!(win.top_spacer > 9_000.0);
        assert!(win.bottom_spacer > 100_000.0);
    }

    #[test]
    fn unified_grid_builds_flat_rows_without_headers() {
        let entries: Vec<FileEntry> = (0..13).map(dummy_file).collect();
        let bands = build_unified_grid_bands(&entries, 4);
        assert_eq!(bands.len(), 4);
        assert!(bands.iter().all(|b| matches!(b, GridBand::Items(_))));
    }

    #[test]
    fn grid_2d_arrow_navigation_moves_vertically_across_rows_and_sections() {
        let mut entries: Vec<FileEntry> = (0..7).map(dummy_file).collect();
        for e in entries.iter_mut().take(4) {
            e.kind = EntryKind::Directory;
        }
        let dirs: Vec<(usize, &FileEntry)> = entries.iter().enumerate().take(4).collect();
        let files: Vec<(usize, &FileEntry)> = entries.iter().enumerate().skip(4).collect();
        let bands = build_grid_bands(&dirs, &files, 3, "PASTAS", "ARQUIVOS");

        // Linha 0 dirs: [0, 1, 2], Linha 1 dirs: [3], Linha 2 files: [4, 5, 6]
        assert_eq!(step_grid_selection(&bands, Some(1), 0, 1), Some(3));
        assert_eq!(step_grid_selection(&bands, Some(0), 0, 1), Some(3));
        assert_eq!(step_grid_selection(&bands, Some(3), 0, 1), Some(4));
        assert_eq!(step_grid_selection(&bands, Some(5), 0, -1), Some(3));
        assert_eq!(step_grid_selection(&bands, Some(2), 1, 0), Some(3));
        assert_eq!(step_grid_selection(&bands, Some(3), -1, 0), Some(2));
    }

    #[test]
    fn grid_columns_scale_with_card_width_and_viewport() {
        assert_eq!(grid_columns_for_width(900.0, 140.0), 5);
        assert_eq!(grid_columns_for_width(1400.0, 140.0), 8);
        assert_eq!(grid_columns_for_width(900.0, 220.0), 3);
    }
}
