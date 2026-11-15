use std::ops::Range;

use fm_core::FileEntry;

pub const DETAIL_ROW_HEIGHT: f32 = 34.0;
pub const DETAIL_ROW_SPACING: f32 = 4.0;
pub const DETAIL_ROW_STEP: f32 = DETAIL_ROW_HEIGHT + DETAIL_ROW_SPACING;

pub const GRID_HEADER_HEIGHT: f32 = 24.0;
pub const GRID_ROW_HEIGHT: f32 = 136.0;
pub const GRID_BAND_SPACING: f32 = 12.0;

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
    fn height(&self) -> f32 {
        match self {
            Self::SectionHeader { .. } => GRID_HEADER_HEIGHT,
            Self::Items(_) => GRID_ROW_HEIGHT,
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

pub fn compute_band_window(
    bands: &[GridBand<'_>],
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
        acc += band.height();
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

        let win = compute_band_window(&bands, 10_000.0, 720.0, 200.0);
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
}
