use std::ops::Range;

use fm_core::{FileEntry, Strings};

pub const DETAIL_ROW_HEIGHT: f32 = 34.0;
pub const DETAIL_ROW_SPACING: f32 = 4.0;
pub const DETAIL_ROW_STEP: f32 = DETAIL_ROW_HEIGHT + DETAIL_ROW_SPACING;
pub const DETAIL_LIST_SIDE_PADDING: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContentRect {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl ContentRect {
    pub fn from_points(a_x: f32, a_y: f32, b_x: f32, b_y: f32) -> Self {
        Self {
            min_x: a_x.min(b_x),
            min_y: a_y.min(b_y),
            max_x: a_x.max(b_x),
            max_y: a_y.max(b_y),
        }
    }

    pub fn intersects(self, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
        self.max_x >= x0 && self.min_x <= x1 && self.max_y >= y0 && self.min_y <= y1
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridLayoutMetrics {
    pub fit_cols: usize,
    pub block_width: f32,
    pub list_x: f32,
    pub center_sparse_rows: bool,
}

/// Calcula largura e alinhamento do bloco da grade (esquerda->direita ou centralizado).
pub fn compute_grid_layout_metrics(
    bands: &[GridBand<'_>],
    viewport_width: f32,
    card_width: f32,
    center_items: bool,
) -> GridLayoutMetrics {
    let card_width = clamp_grid_card_width(card_width);
    let fit_cols = grid_columns_for_width(viewport_width, card_width);
    if !center_items {
        let side_pad = 12.0_f32;
        return GridLayoutMetrics {
            fit_cols,
            block_width: (viewport_width - side_pad * 2.0).max(card_width),
            list_x: side_pad,
            center_sparse_rows: false,
        };
    }

    let max_row_items = bands
        .iter()
        .filter_map(|b| match b {
            GridBand::Items(chunk) => Some(chunk.len()),
            GridBand::SectionHeader { .. } => None,
        })
        .max()
        .unwrap_or(0);
    let active_cols = if max_row_items > 0 && max_row_items < fit_cols {
        max_row_items
    } else {
        fit_cols
    };
    let grid_block_width =
        active_cols as f32 * card_width + active_cols.saturating_sub(1) as f32 * GRID_GAP;
    let min_header_w = 300.0_f32.min(viewport_width.max(card_width));
    let block_width = grid_block_width
        .max(min_header_w)
        .min(viewport_width.max(card_width));
    let list_x = ((viewport_width - block_width) * 0.5).max(0.0);

    GridLayoutMetrics {
        fit_cols,
        block_width,
        list_x,
        center_sparse_rows: max_row_items > 0 && max_row_items < fit_cols,
    }
}

/// Retorna os índices das entradas cujos cards no Grid intersectam `rect` em coordenadas do conteúdo.
pub fn grid_marquee_hits(
    entries: &[FileEntry],
    viewport_width: f32,
    card_width: f32,
    group_folders: bool,
    center_items: bool,
    strings: Strings,
    rect: ContentRect,
) -> Vec<usize> {
    if entries.is_empty() || viewport_width <= 0.0 {
        return Vec::new();
    }
    let card_width = clamp_grid_card_width(card_width);
    let card_height = grid_card_height(card_width);
    let box_width = (card_width - 14.0).max(88.0);
    let box_inset_x = (card_width - box_width) * 0.5;

    let fit_cols = grid_columns_for_width(viewport_width, card_width);
    let bands = build_tab_grid_bands(entries, fit_cols, group_folders, strings);
    let metrics = compute_grid_layout_metrics(&bands, viewport_width, card_width, center_items);

    let mut hits = Vec::new();
    let mut y = 0.0_f32;
    for (i, band) in bands.iter().enumerate() {
        let h = band.height_with(card_height);
        if let GridBand::Items(chunk) = band
            && rect.max_y >= y
            && rect.min_y <= y + h
        {
            let row_w =
                chunk.len() as f32 * card_width + chunk.len().saturating_sub(1) as f32 * GRID_GAP;
            let row_x = if metrics.center_sparse_rows {
                metrics.list_x + ((metrics.block_width - row_w) * 0.5).max(0.0)
            } else {
                metrics.list_x
            };
            for (col, (entry_idx, _)) in chunk.iter().enumerate() {
                let x0 = row_x + col as f32 * (card_width + GRID_GAP) + box_inset_x;
                let x1 = x0 + box_width;
                if rect.intersects(x0, y, x1, y + card_height) {
                    hits.push(*entry_idx);
                }
            }
        }
        y += h;
        if i + 1 < bands.len() {
            y += GRID_BAND_SPACING;
        }
    }
    hits
}

/// Retorna os índices das entradas em modo Lista/Colunas que intersectam `rect` em coordenadas do conteúdo.
pub fn list_marquee_hits(
    total_entries: usize,
    viewport_width: f32,
    side_padding: f32,
    rect: ContentRect,
) -> Vec<usize> {
    if total_entries == 0 || viewport_width <= 0.0 {
        return Vec::new();
    }
    let x0 = side_padding;
    let x1 = (viewport_width - side_padding).max(side_padding);
    if rect.max_x < x0 || rect.min_x > x1 || rect.max_y < 0.0 {
        return Vec::new();
    }
    let start = ((rect.min_y - DETAIL_ROW_HEIGHT).max(0.0) / DETAIL_ROW_STEP).floor() as usize;
    let end = ((rect.max_y.max(0.0)) / DETAIL_ROW_STEP).floor() as usize;
    let mut hits = Vec::new();
    for idx in start..=end.min(total_entries - 1) {
        let y0 = idx as f32 * DETAIL_ROW_STEP;
        let y1 = y0 + DETAIL_ROW_HEIGHT;
        if rect.intersects(x0, y0, x1, y1) {
            hits.push(idx);
        }
    }
    hits
}

/// Move a seleção na grade em 2D (`dx` horizontal, `dy` vertical por linha).
/// Retorna a ordem linear visual (esquerda->direita, cima->baixo) dos índices de entrada nas bandas da grade.
pub fn grid_visual_order(bands: &[GridBand<'_>]) -> Vec<usize> {
    bands
        .iter()
        .filter_map(|b| match b {
            GridBand::Items(chunk) => Some(chunk.iter().map(|(idx, _)| *idx)),
            _ => None,
        })
        .flatten()
        .collect()
}

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

    #[test]
    fn marquee_hits_select_expected_items_in_grid_and_list() {
        let entries: Vec<FileEntry> = (0..6).map(dummy_file).collect();
        let strings = Strings::for_language(fm_core::Language::PtBr);

        // Em grade esquerda->direita (~900px, card 140px => 5 colunas), primeira linha começa em x=12.
        let grid_hits_left = grid_marquee_hits(
            &entries,
            900.0,
            140.0,
            false,
            false,
            strings,
            ContentRect::from_points(14.0, 10.0, 350.0, 80.0),
        );
        assert_eq!(grid_hits_left, vec![0, 1, 2]);

        // Em modo centralizado, mesma largura desloca a linha para o centro.
        let grid_hits_centered = grid_marquee_hits(
            &entries,
            900.0,
            140.0,
            false,
            true,
            strings,
            ContentRect::from_points(100.0, 10.0, 460.0, 80.0),
        );
        assert!(grid_hits_centered.contains(&0));
        assert!(grid_hits_centered.contains(&1));
        assert!(grid_hits_centered.contains(&2));
        assert!(!grid_hits_centered.contains(&5));

        // Em lista, cada linha ocupa 38px de passo (34px altura + 4px gap).
        let list_hits = list_marquee_hits(
            entries.len(),
            800.0,
            DETAIL_LIST_SIDE_PADDING,
            ContentRect::from_points(20.0, 40.0, 300.0, 120.0),
        );
        assert_eq!(list_hits, vec![1, 2, 3]);
    }
}
