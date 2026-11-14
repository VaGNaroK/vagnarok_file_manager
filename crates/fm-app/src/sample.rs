use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use compact_str::CompactString;
use fm_core::{
    DirectorySnapshot, DirectoryTelemetry, DriveInfo, DriveKind, EntryId, EntryKind, FileEntry,
    FolderSummary, PermissionsMeta, SelectedFileTelemetry, VolumeSpace,
};

pub fn default_home_path() -> PathBuf {
    PathBuf::from("/home/aether/Documents/Workspace")
}

pub fn sample_drives() -> Vec<DriveInfo> {
    vec![
        DriveInfo {
            label: CompactString::new("SSD Principal"),
            mount_path: PathBuf::from("/"),
            fs_type: Some(CompactString::new("ext4")),
            kind: DriveKind::InternalSsd,
            space: Some(VolumeSpace {
                label: CompactString::new("SSD Principal"),
                free_bytes: 266_717_462_528,   // ~248.4 GB
                total_bytes: 1_099_511_627_776, // 1.0 TB
            }),
        },
        DriveInfo {
            label: CompactString::new("Disco Secundário"),
            mount_path: PathBuf::from("/mnt/storage"),
            fs_type: Some(CompactString::new("btrfs")),
            kind: DriveKind::SecondaryDisk,
            space: Some(VolumeSpace {
                label: CompactString::new("Disco Secundário"),
                free_bytes: 1_288_490_188_800,
                total_bytes: 2_199_023_255_552,
            }),
        },
    ]
}

/// Retorna snapshot mockado para navegação interativa durante a Fase P0.
pub fn sample_snapshot(path: &Path, request_id: u64) -> DirectorySnapshot {
    let entries = match path.to_string_lossy().as_ref() {
        "/home/aether/Documents/Workspace/Projects" => vec![
            dir(101, path, "vagnarok-fm", 14),
            dir(102, path, "aether-shell", 9),
            file(103, path, "Cargo.toml", Some("toml"), 1_420, 0o644),
            file(104, path, "README.md", Some("md"), 3_840, 0o644),
        ],
        "/home/aether/Documents/Workspace/Design System" => vec![
            dir(201, path, "tokens", 6),
            file(202, path, "obsidian-flow.json", Some("json"), 6_140, 0o644),
            file(203, path, "aether-light.json", Some("json"), 5_820, 0o644),
        ],
        "/home/aether/Documents" => vec![
            dir(301, path, "Workspace", 14),
            dir(302, path, "Specs", 6),
            file(303, path, "architecture.md", Some("md"), 12_480, 0o644),
        ],
        _ => workspace_entries(path),
    };

    DirectorySnapshot {
        request_id,
        path: path.to_path_buf(),
        entries,
        fs_type: Some(CompactString::new("ext4")),
    }
}

pub fn sample_telemetry(path: &Path, entries: &[FileEntry]) -> DirectoryTelemetry {
    let git_branch = if path.to_string_lossy().contains("Workspace") {
        Some(CompactString::new("main"))
    } else {
        None
    };

    DirectoryTelemetry {
        git_branch,
        folder_stats: FolderSummary::from_entries(entries),
        volume_space: sample_drives().into_iter().next().and_then(|d| d.space),
    }
}

pub fn sample_selected_telemetry(entry: &FileEntry) -> SelectedFileTelemetry {
    let mime = match entry.extension.as_deref() {
        Some("rs") => "text/x-rust",
        Some("json") => "application/json",
        Some("sh") => "application/x-sh",
        Some("md") => "text/markdown",
        Some("yml" | "yaml") => "application/yaml",
        Some("pdf") => "application/pdf",
        Some("gz" | "tar") => "application/gzip",
        _ if entry.kind.is_dir_like() => "inode/directory",
        _ => "application/octet-stream",
    };

    let preview_snippet = match entry.extension.as_deref() {
        Some("rs") => Some(
            "use iced::{Element, Task};\n\npub fn main() -> iced::Result {\n    tracing_subscriber::fmt::init();\n    fm_app::run()\n}\n"
                .to_owned(),
        ),
        Some("json") => Some(
            "{\n  \"profile\": \"aether-workspace\",\n  \"renderer\": \"wgpu\",\n  \"virtualization\": true\n}\n"
                .to_owned(),
        ),
        Some("sh") => Some(
            "#!/usr/bin/env bash\nset -euo pipefail\ncargo check --workspace --all-targets\n"
                .to_owned(),
        ),
        Some("md") | Some("toml") | Some("yml") => Some(format!(
            "# {}\nPrévia rápida (QuickLook P0) — {} bytes.\n",
            entry.name, entry.size_bytes
        )),
        _ => None,
    };

    let sha_prefix = if entry.kind.is_dir_like() {
        None
    } else {
        Some(CompactString::from(format!(
            "{:04x}{:02x}..{:02x}",
            (entry.id.0 * 7919) & 0xFFFF,
            (entry.size_bytes & 0xFF),
            ((entry.id.0 + entry.size_bytes) & 0xFF)
        )))
    };

    SelectedFileTelemetry {
        sha256_prefix: sha_prefix.clone(),
        sha256_full: sha_prefix.map(|p| {
            CompactString::from(format!(
                "{p}8f3c2d9a01b4e6f7a8c9d0e1f2a3b4c5d6e7f8"
            ))
        }),
        mime_type: Some(CompactString::new(mime)),
        preview_snippet,
    }
}

fn workspace_entries(base: &Path) -> Vec<FileEntry> {
    vec![
        dir(1, base, "Projects", 12),
        dir(2, base, "Design System", 24),
        dir(3, base, "Wallpapers", 48),
        dir(4, base, "Backups", 3),
        dir(5, base, "Dotfiles", 18),
        dir(6, base, "Scripts", 9),
        file(7, base, "kernel-config.json", Some("json"), 4_300, 0o644),
        file(8, base, "arch-setup.sh", Some("sh"), 1_843, 0o755),
        file(9, base, "main.rs", Some("rs"), 9_114, 0o644),
        file(10, base, "wireframes.fig", Some("fig"), 33_973_862, 0o644),
        file(11, base, "presentation.pdf", Some("pdf"), 5_347_737, 0o644),
        file(12, base, "notes.md", Some("md"), 842, 0o644),
        file(13, base, "docker-compose.yml", Some("yml"), 1_228, 0o644),
        file(
            14,
            base,
            "release_v2.tar.gz",
            Some("tar"),
            198_810_009,
            0o644,
        ),
        hidden_file(15, base, ".gitignore", 256),
    ]
}

fn dir(id: u64, base: &Path, name: &str, child_count: u32) -> FileEntry {
    FileEntry {
        id: EntryId(id),
        name: CompactString::new(name),
        path: base.join(name),
        extension: None,
        kind: EntryKind::Directory,
        size_bytes: 0,
        modified_at: Some(SystemTime::now() - Duration::from_secs(id * 600)),
        is_hidden: name.starts_with('.'),
        permissions: PermissionsMeta::PosixMode(0o755),
        child_count: Some(child_count),
        mime_badge: None,
    }
}

fn file(
    id: u64,
    base: &Path,
    name: &str,
    ext: Option<&str>,
    size_bytes: u64,
    mode: u32,
) -> FileEntry {
    FileEntry {
        id: EntryId(id),
        name: CompactString::new(name),
        path: base.join(name),
        extension: ext.map(CompactString::new),
        kind: EntryKind::File,
        size_bytes,
        modified_at: Some(SystemTime::now() - Duration::from_secs(id * 840)),
        is_hidden: name.starts_with('.'),
        permissions: PermissionsMeta::PosixMode(mode),
        child_count: None,
        mime_badge: ext.map(|e| CompactString::from(e.to_ascii_uppercase())),
    }
}

fn hidden_file(id: u64, base: &Path, name: &str, size_bytes: u64) -> FileEntry {
    file(id, base, name, None, size_bytes, 0o644)
}
