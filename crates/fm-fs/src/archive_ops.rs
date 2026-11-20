//! Engine assíncrono e seguro de compactação (`.7z`, `.zip`, `.tar`, `.tar.gz`)
//! e extração (`.zip`, `.tar`, `.tar.gz`, `.7z`, `.rar`), com proteção anti Zip-Slip.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

use compact_str::CompactString;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use fm_core::{
    ArchiveFormat, ArchiveJobKind, ArchiveJobProgress, CompressionLevel, ConflictPolicy,
    ExtractTargetMode, FsError, JobState, sanitize_archive_rel_path, strip_known_archive_suffix,
};
use sevenz_rust2::{
    ArchiveEntry as SevenZEntry, ArchiveReader as SevenZReader, ArchiveWriter as SevenZWriter,
    EncoderConfiguration, EncoderMethod, NtTime, Password as SevenZPassword,
    encoder_options::{EncoderOptions, Lzma2Options},
};
use tar::{Archive as TarArchive, Builder as TarBuilder, Header as TarHeader};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use zip::{CompressionMethod, ZipArchive, ZipWriter, result::ZipError, write::SimpleFileOptions};

use crate::archive::{ArchiveKind, detect_archive_kind, open_shared_archive};
use crate::map_io_err;

const PROGRESS_EMIT_INTERVAL_MS: u64 = 60;
const IO_BUF_BYTES: usize = 64 * 1024;
/// Limite de dicionário LZMA2 no nível Máximo (32 MiB) para evitar explosão de RAM em desktops.
const MAX_LZMA2_DICT_BYTES: u32 = 32 * 1024 * 1024;

/// Compacta `sources` em segundo plano no formato e nível escolhidos, emitindo progresso contínuo.
#[allow(clippy::too_many_arguments)]
pub async fn run_compress_job(
    job_id: u64,
    sources: Vec<PathBuf>,
    dest_dir: PathBuf,
    archive_name: String,
    format: ArchiveFormat,
    level: CompressionLevel,
    conflict: ConflictPolicy,
    cancel: CancellationToken,
    progress_tx: UnboundedSender<ArchiveJobProgress>,
) -> Result<PathBuf, FsError> {
    tokio::task::spawn_blocking(move || {
        compress_blocking(
            job_id,
            &sources,
            &dest_dir,
            &archive_name,
            format,
            level,
            conflict,
            &cancel,
            &progress_tx,
        )
    })
    .await
    .map_err(|err| FsError::Io {
        path: PathBuf::new(),
        message: format!("falha na tarefa de compactação: {err}"),
    })?
}

/// Extrai um arquivo compactado (`.zip`, `.tar`, `.tar.gz`, `.7z`, `.rar`) em segundo plano.
pub async fn run_extract_job(
    job_id: u64,
    archive_path: PathBuf,
    mode: ExtractTargetMode,
    conflict: ConflictPolicy,
    password: Option<String>,
    cancel: CancellationToken,
    progress_tx: UnboundedSender<ArchiveJobProgress>,
) -> Result<PathBuf, FsError> {
    tokio::task::spawn_blocking(move || {
        extract_blocking(
            job_id,
            &archive_path,
            mode,
            conflict,
            password.as_deref(),
            &cancel,
            &progress_tx,
        )
    })
    .await
    .map_err(|err| FsError::Io {
        path: PathBuf::new(),
        message: format!("falha na tarefa de extração: {err}"),
    })?
}

struct TempFileGuard {
    path: PathBuf,
    armed: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[derive(Debug, Clone)]
struct PackEntry {
    disk_path: PathBuf,
    archive_rel: String,
    is_dir: bool,
    size_bytes: u64,
}

struct ProgressTracker<'a> {
    job_id: u64,
    kind: ArchiveJobKind,
    title: CompactString,
    dest_dir: PathBuf,
    output_path: PathBuf,
    bytes_done: u64,
    bytes_total: u64,
    items_done: usize,
    items_total: usize,
    current_item: CompactString,
    last_emit: Instant,
    cancel: &'a CancellationToken,
    tx: &'a UnboundedSender<ArchiveJobProgress>,
}

impl<'a> ProgressTracker<'a> {
    fn new(
        job_id: u64,
        kind: ArchiveJobKind,
        title: String,
        dest_dir: PathBuf,
        output_path: PathBuf,
        cancel: &'a CancellationToken,
        tx: &'a UnboundedSender<ArchiveJobProgress>,
    ) -> Self {
        Self {
            job_id,
            kind,
            title: CompactString::from(title),
            dest_dir,
            output_path,
            bytes_done: 0,
            bytes_total: 0,
            items_done: 0,
            items_total: 0,
            current_item: CompactString::default(),
            last_emit: Instant::now(),
            cancel,
            tx,
        }
    }

    fn emit_force(&mut self, state: JobState) {
        self.last_emit = Instant::now();
        let _ = self.tx.send(ArchiveJobProgress {
            job_id: self.job_id,
            kind: self.kind,
            title: self.title.clone(),
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
            items_done: self.items_done,
            items_total: self.items_total,
            current_item: self.current_item.clone(),
            dest_dir: self.dest_dir.clone(),
            output_path: self.output_path.clone(),
            state,
        });
    }

    fn advance_bytes(&mut self, delta: u64) {
        self.bytes_done = self.bytes_done.saturating_add(delta);
        if self.last_emit.elapsed().as_millis() >= u128::from(PROGRESS_EMIT_INTERVAL_MS) {
            self.emit_force(JobState::Running);
        }
    }

    fn set_current_item(&mut self, label: &str) {
        self.current_item = CompactString::from(label);
        if self.last_emit.elapsed().as_millis() >= u128::from(PROGRESS_EMIT_INTERVAL_MS) {
            self.emit_force(JobState::Running);
        }
    }
}

struct TrackedCancelReader<'t, 'a, R: ?Sized> {
    inner: &'t mut R,
    tracker: &'t mut ProgressTracker<'a>,
}

impl<R: Read + ?Sized> Read for TrackedCancelReader<'_, '_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.tracker.cancel.is_cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.tracker.advance_bytes(n as u64);
        }
        Ok(n)
    }
}

struct OwnedTrackedCancelReader<'t, 'a, R> {
    inner: R,
    tracker: &'t mut ProgressTracker<'a>,
}

impl<R: Read> Read for OwnedTrackedCancelReader<'_, '_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.tracker.cancel.is_cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.tracker.advance_bytes(n as u64);
        }
        Ok(n)
    }
}

#[allow(clippy::too_many_arguments)]
fn compress_blocking(
    job_id: u64,
    sources: &[PathBuf],
    dest_dir: &Path,
    archive_name: &str,
    format: ArchiveFormat,
    level: CompressionLevel,
    conflict: ConflictPolicy,
    cancel: &CancellationToken,
    tx: &UnboundedSender<ArchiveJobProgress>,
) -> Result<PathBuf, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }
    if sources.is_empty() {
        return Err(FsError::Io {
            path: dest_dir.to_path_buf(),
            message: "nenhum item selecionado para compactação".to_owned(),
        });
    }

    let final_filename = build_archive_filename(archive_name, format)?;
    let raw_target = dest_dir.join(&final_filename);
    let output_path = resolve_compress_output_path(&raw_target, conflict)?;
    let display_name = output_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(final_filename);

    let effective_level = if format.supports_compression_level() {
        level
    } else {
        CompressionLevel::Store
    };

    let mut tracker = ProgressTracker::new(
        job_id,
        ArchiveJobKind::Compress {
            format,
            level: effective_level,
        },
        display_name,
        dest_dir.to_path_buf(),
        output_path.clone(),
        cancel,
        tx,
    );
    tracker.emit_force(JobState::Running);

    let mut pack_entries = Vec::new();
    let mut visited_dirs = HashSet::new();
    for src in sources {
        collect_pack_entries(src, None, cancel, &mut visited_dirs, &mut pack_entries)?;
    }

    tracker.items_total = pack_entries.len();
    tracker.bytes_total = pack_entries.iter().map(|e| e.size_bytes).sum();
    tracker.emit_force(JobState::Running);

    let temp_path = dest_dir.join(format!(".__vfm_pack_{job_id}__.tmp"));
    let mut temp_guard = TempFileGuard::new(temp_path.clone());

    let write_res = match format {
        ArchiveFormat::SevenZ => {
            write_sevenz_archive(&temp_path, &pack_entries, effective_level, &mut tracker)
        }
        ArchiveFormat::Zip => {
            write_zip_archive(&temp_path, &pack_entries, effective_level, &mut tracker)
        }
        ArchiveFormat::Tar => write_tar_archive(
            &temp_path,
            &pack_entries,
            false,
            effective_level,
            &mut tracker,
        ),
        ArchiveFormat::TarGz => write_tar_archive(
            &temp_path,
            &pack_entries,
            true,
            effective_level,
            &mut tracker,
        ),
    };

    match write_res {
        Ok(()) => {
            if cancel.is_cancelled() {
                tracker.emit_force(JobState::Cancelled);
                return Err(FsError::Cancelled);
            }
            if conflict == ConflictPolicy::Overwrite && output_path.exists() {
                let _ = fs::remove_file(&output_path);
            }
            fs::rename(&temp_path, &output_path).map_err(|e| map_io_err(&output_path, e))?;
            temp_guard.disarm();
            tracker.bytes_done = tracker.bytes_total;
            tracker.items_done = tracker.items_total;
            tracker.emit_force(JobState::Completed);
            Ok(output_path)
        }
        Err(FsError::Cancelled) => {
            tracker.emit_force(JobState::Cancelled);
            Err(FsError::Cancelled)
        }
        Err(err) => {
            tracker.emit_force(JobState::Failed(err.to_string()));
            Err(err)
        }
    }
}

fn build_archive_filename(raw_name: &str, format: ArchiveFormat) -> Result<String, FsError> {
    let trimmed = raw_name.trim().trim_matches(['/', '\\']);
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." || trimmed.contains(['/', '\\']) {
        return Err(FsError::Io {
            path: PathBuf::from(raw_name),
            message: "nome de arquivo inválido".to_owned(),
        });
    }
    let ext = format.extension();
    let lower = trimmed.to_ascii_lowercase();
    let suffix = format!(".{ext}");
    if lower.ends_with(&suffix) {
        Ok(trimmed.to_owned())
    } else {
        Ok(format!("{trimmed}{suffix}"))
    }
}

fn resolve_compress_output_path(
    target: &Path,
    conflict: ConflictPolicy,
) -> Result<PathBuf, FsError> {
    if !target.exists() {
        return Ok(target.to_path_buf());
    }
    match conflict {
        ConflictPolicy::Overwrite => Ok(target.to_path_buf()),
        ConflictPolicy::Skip => Err(FsError::Io {
            path: target.to_path_buf(),
            message: format!("arquivo de destino já existe: {}", target.to_string_lossy()),
        }),
        ConflictPolicy::RenameAuto => Ok(unique_sibling_file_path(target)),
    }
}

fn collect_pack_entries(
    disk_path: &Path,
    archive_parent: Option<&str>,
    cancel: &CancellationToken,
    visited_dirs: &mut HashSet<PathBuf>,
    out: &mut Vec<PackEntry>,
) -> Result<(), FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    let meta = fs::symlink_metadata(disk_path).map_err(|e| map_io_err(disk_path, e))?;
    // Não segue symlinks para evitar loops recursivos ou vazamento fora da árvore selecionada.
    if meta.file_type().is_symlink() {
        return Ok(());
    }

    let Some(name) = disk_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
    else {
        return Ok(());
    };
    let archive_rel = match archive_parent {
        Some(parent) if !parent.is_empty() => format!("{parent}/{name}"),
        _ => name,
    };

    if meta.is_dir() {
        if let Ok(canon) = fs::canonicalize(disk_path)
            && !visited_dirs.insert(canon)
        {
            return Ok(());
        }
        out.push(PackEntry {
            disk_path: disk_path.to_path_buf(),
            archive_rel: archive_rel.clone(),
            is_dir: true,
            size_bytes: 0,
        });
        let rd = fs::read_dir(disk_path).map_err(|e| map_io_err(disk_path, e))?;
        for child_res in rd {
            if cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            let Ok(child) = child_res else {
                continue;
            };
            collect_pack_entries(&child.path(), Some(&archive_rel), cancel, visited_dirs, out)?;
        }
    } else if meta.is_file() {
        out.push(PackEntry {
            disk_path: disk_path.to_path_buf(),
            archive_rel,
            is_dir: false,
            size_bytes: meta.len(),
        });
    }

    Ok(())
}

fn write_sevenz_archive(
    dest_file: &Path,
    entries: &[PackEntry],
    level: CompressionLevel,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    let file = File::create(dest_file).map_err(|e| map_io_err(dest_file, e))?;
    let mut writer =
        SevenZWriter::new(BufWriter::with_capacity(IO_BUF_BYTES, file)).map_err(|e| {
            FsError::Io {
                path: dest_file.to_path_buf(),
                message: e.to_string(),
            }
        })?;

    let threads = std::thread::available_parallelism()
        .map(|n| u32::try_from(n.get()).unwrap_or(2).clamp(1, 4))
        .unwrap_or(2);

    let config = match level {
        CompressionLevel::Store => EncoderConfiguration::new(EncoderMethod::COPY),
        CompressionLevel::Minimum => EncoderConfiguration::new(EncoderMethod::LZMA2)
            .with_options(EncoderOptions::Lzma2(Lzma2Options::from_level(1))),
        CompressionLevel::Normal => EncoderConfiguration::new(EncoderMethod::LZMA2).with_options(
            EncoderOptions::Lzma2(Lzma2Options::from_level_mt(5, threads, 16 * 1024 * 1024)),
        ),
        CompressionLevel::Maximum => {
            let mut opts = Lzma2Options::from_level_mt(7, threads, 32 * 1024 * 1024);
            opts.set_dictionary_size(MAX_LZMA2_DICT_BYTES);
            EncoderConfiguration::new(EncoderMethod::LZMA2)
                .with_options(EncoderOptions::Lzma2(opts))
        }
    };
    writer.set_content_methods(vec![config]);

    for item in entries {
        if tracker.cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        tracker.set_current_item(&item.archive_rel);

        let mut entry = SevenZEntry::new();
        entry.name = item.archive_rel.clone();
        entry.is_directory = item.is_dir;
        entry.has_last_modified_date = true;
        if let Ok(meta) = fs::metadata(&item.disk_path)
            && let Ok(mtime) = meta.modified()
            && let Ok(nt) = NtTime::try_from(mtime)
        {
            entry.last_modified_date = nt;
        }

        if item.is_dir {
            writer
                .push_archive_entry(entry, None::<std::io::Empty>)
                .map_err(|e| FsError::Io {
                    path: item.disk_path.clone(),
                    message: e.to_string(),
                })?;
        } else {
            let src_file = open_shared_archive(&item.disk_path)?;
            let buf_reader = BufReader::with_capacity(IO_BUF_BYTES, src_file);
            let tracked = OwnedTrackedCancelReader {
                inner: buf_reader,
                tracker,
            };
            let push_res = writer.push_archive_entry(entry, Some(tracked));
            if tracker.cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            push_res.map_err(|e| FsError::Io {
                path: item.disk_path.clone(),
                message: e.to_string(),
            })?;
        }
        tracker.items_done += 1;
    }

    writer.finish().map_err(|e| FsError::Io {
        path: dest_file.to_path_buf(),
        message: e.to_string(),
    })?;

    Ok(())
}

fn write_zip_archive(
    dest_file: &Path,
    entries: &[PackEntry],
    level: CompressionLevel,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    let file = File::create(dest_file).map_err(|e| map_io_err(dest_file, e))?;
    let mut writer = ZipWriter::new(BufWriter::with_capacity(IO_BUF_BYTES, file));

    let (method, comp_lvl) = match level {
        CompressionLevel::Store => (CompressionMethod::Stored, None),
        CompressionLevel::Minimum => (CompressionMethod::Deflated, Some(1)),
        CompressionLevel::Normal => (CompressionMethod::Deflated, Some(6)),
        CompressionLevel::Maximum => (CompressionMethod::Deflated, Some(9)),
    };

    for item in entries {
        if tracker.cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        tracker.set_current_item(&item.archive_rel);

        let opts = SimpleFileOptions::default()
            .compression_method(method)
            .compression_level(comp_lvl)
            .large_file(item.size_bytes >= u64::from(u32::MAX));

        if item.is_dir {
            writer
                .add_directory(&item.archive_rel, opts)
                .map_err(|e| FsError::Io {
                    path: item.disk_path.clone(),
                    message: e.to_string(),
                })?;
        } else {
            writer
                .start_file(&item.archive_rel, opts)
                .map_err(|e| FsError::Io {
                    path: item.disk_path.clone(),
                    message: e.to_string(),
                })?;
            let src_file = open_shared_archive(&item.disk_path)?;
            let mut buf_reader = BufReader::with_capacity(IO_BUF_BYTES, src_file);
            let copy_res = {
                let mut reader = OwnedTrackedCancelReader {
                    inner: &mut buf_reader,
                    tracker,
                };
                io::copy(&mut reader, &mut writer)
            };
            if let Err(err) = copy_res {
                if tracker.cancel.is_cancelled() {
                    return Err(FsError::Cancelled);
                }
                return Err(map_io_err(&item.disk_path, err));
            }
        }
        tracker.items_done += 1;
    }

    writer.finish().map_err(|e| FsError::Io {
        path: dest_file.to_path_buf(),
        message: e.to_string(),
    })?;

    Ok(())
}

fn write_tar_archive(
    dest_file: &Path,
    entries: &[PackEntry],
    gzip: bool,
    level: CompressionLevel,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    let file = File::create(dest_file).map_err(|e| map_io_err(dest_file, e))?;
    let buffered = BufWriter::with_capacity(IO_BUF_BYTES, file);

    if gzip {
        let compression = match level {
            CompressionLevel::Store => Compression::none(),
            CompressionLevel::Minimum => Compression::fast(),
            CompressionLevel::Normal => Compression::default(),
            CompressionLevel::Maximum => Compression::best(),
        };
        let mut builder = TarBuilder::new(GzEncoder::new(buffered, compression));
        write_tar_entries(&mut builder, entries, tracker)?;
        let gz = builder.into_inner().map_err(|e| map_io_err(dest_file, e))?;
        gz.finish().map_err(|e| map_io_err(dest_file, e))?;
    } else {
        let mut builder = TarBuilder::new(buffered);
        write_tar_entries(&mut builder, entries, tracker)?;
        builder.finish().map_err(|e| map_io_err(dest_file, e))?;
    }

    Ok(())
}

fn write_tar_entries<W: Write>(
    builder: &mut TarBuilder<W>,
    entries: &[PackEntry],
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    for item in entries {
        if tracker.cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        tracker.set_current_item(&item.archive_rel);

        if item.is_dir {
            let mut header = TarHeader::new_gnu();
            header.set_entry_type(tar::EntryType::Directory);
            header.set_mode(0o755);
            header.set_size(0);
            header.set_cksum();
            builder
                .append_data(&mut header, &item.archive_rel, std::io::empty())
                .map_err(|e| map_io_err(&item.disk_path, e))?;
        } else {
            let src_file = open_shared_archive(&item.disk_path)?;
            let meta = src_file
                .metadata()
                .map_err(|e| map_io_err(&item.disk_path, e))?;
            let mut header = TarHeader::new_gnu();
            header.set_entry_type(tar::EntryType::Regular);
            header.set_size(meta.len());
            header.set_mode(0o644);
            if let Ok(mtime) = meta.modified()
                && let Ok(dur) = mtime.duration_since(UNIX_EPOCH)
            {
                header.set_mtime(dur.as_secs());
            }
            header.set_cksum();
            let mut buf_reader = BufReader::with_capacity(IO_BUF_BYTES, src_file);
            let append_res = {
                let mut reader = OwnedTrackedCancelReader {
                    inner: &mut buf_reader,
                    tracker,
                };
                builder.append_data(&mut header, &item.archive_rel, &mut reader)
            };
            if let Err(err) = append_res {
                if tracker.cancel.is_cancelled() {
                    return Err(FsError::Cancelled);
                }
                return Err(map_io_err(&item.disk_path, err));
            }
        }
        tracker.items_done += 1;
    }
    Ok(())
}

fn extract_blocking(
    job_id: u64,
    archive_path: &Path,
    mode: ExtractTargetMode,
    conflict: ConflictPolicy,
    password: Option<&str>,
    cancel: &CancellationToken,
    tx: &UnboundedSender<ArchiveJobProgress>,
) -> Result<PathBuf, FsError> {
    if cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }
    let kind = detect_archive_kind(archive_path).ok_or_else(|| FsError::Io {
        path: archive_path.to_path_buf(),
        message: "formato de arquivo não suportado para extração".to_owned(),
    })?;

    let parent_dir = archive_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let archive_name = archive_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let (dest_root, created_subfolder) = match mode {
        ExtractTargetMode::Here => (parent_dir.clone(), None),
        ExtractTargetMode::Subfolder => {
            let base = strip_known_archive_suffix(&archive_name);
            let candidate = parent_dir.join(base);
            let resolved = unique_dir_path(&candidate, conflict);
            let existed_before = resolved.exists();
            fs::create_dir_all(&resolved).map_err(|e| map_io_err(&resolved, e))?;
            let created = (!existed_before).then(|| resolved.clone());
            (resolved, created)
        }
    };

    let mut tracker = ProgressTracker::new(
        job_id,
        ArchiveJobKind::Extract { mode },
        archive_name,
        parent_dir,
        dest_root.clone(),
        cancel,
        tx,
    );
    tracker.emit_force(JobState::Running);

    let res = match kind {
        ArchiveKind::Zip => extract_zip(archive_path, &dest_root, conflict, password, &mut tracker),
        ArchiveKind::Tar => extract_tar(archive_path, &dest_root, false, conflict, &mut tracker),
        ArchiveKind::TarGz => extract_tar(archive_path, &dest_root, true, conflict, &mut tracker),
        ArchiveKind::SevenZ => {
            extract_sevenz(archive_path, &dest_root, conflict, password, &mut tracker)
        }
        #[cfg(feature = "rar-extract")]
        ArchiveKind::Rar => extract_rar(archive_path, &dest_root, conflict, password, &mut tracker),
    };

    match res {
        Ok(()) => {
            tracker.emit_force(JobState::Completed);
            Ok(dest_root)
        }
        Err(FsError::Cancelled) => {
            if let Some(dir) = created_subfolder {
                let _ = fs::remove_dir(&dir);
            }
            tracker.emit_force(JobState::Cancelled);
            Err(FsError::Cancelled)
        }
        Err(err @ (FsError::PasswordRequired(_) | FsError::InvalidPassword(_))) => {
            if let Some(dir) = created_subfolder {
                let _ = fs::remove_dir(&dir);
            }
            Err(err)
        }
        Err(err) => {
            tracker.emit_force(JobState::Failed(err.to_string()));
            Err(err)
        }
    }
}

fn is_zip_decrypt_io_error(err: &FsError) -> bool {
    let FsError::Io { message, .. } = err else {
        return false;
    };
    let lower = message.to_ascii_lowercase();
    lower.contains("authentication code")
        || lower.contains("invalid password")
        || lower.contains("corrupt deflate")
        || lower.contains("invalid deflate")
        || lower.contains("invalid data")
        || lower.contains("crc")
        || lower.contains("unexpected end of file")
}

fn extract_zip(
    archive_path: &Path,
    dest_root: &Path,
    conflict: ConflictPolicy,
    password: Option<&str>,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    let file = open_shared_archive(archive_path)?;
    let mut archive =
        ZipArchive::new(BufReader::with_capacity(IO_BUF_BYTES, file)).map_err(|e| FsError::Io {
            path: archive_path.to_path_buf(),
            message: format!("arquivo ZIP inválido: {e}"),
        })?;

    tracker.items_total = archive.len();
    let mut total_bytes = 0u64;
    let mut has_encrypted = false;
    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index_data(i)
            && !entry.is_dir()
        {
            total_bytes = total_bytes.saturating_add(entry.size());
            if entry.encrypted() {
                has_encrypted = true;
            }
        }
    }
    if has_encrypted && password.is_none() {
        return Err(FsError::PasswordRequired(archive_path.to_path_buf()));
    }
    tracker.bytes_total = total_bytes;
    tracker.emit_force(JobState::Running);

    for i in 0..archive.len() {
        if tracker.cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        let open_res = match password {
            Some(pw) => archive.by_index_decrypt(i, pw.as_bytes()),
            None => archive.by_index(i),
        };
        let mut entry = open_res.map_err(|e| match e {
            ZipError::UnsupportedArchive(msg) if msg == ZipError::PASSWORD_REQUIRED => {
                FsError::PasswordRequired(archive_path.to_path_buf())
            }
            ZipError::InvalidPassword => FsError::InvalidPassword(archive_path.to_path_buf()),
            other => FsError::Io {
                path: archive_path.to_path_buf(),
                message: other.to_string(),
            },
        })?;

        if entry.is_symlink() {
            tracker.items_done += 1;
            continue;
        }

        let entry_encrypted = entry.encrypted();
        let raw_name = entry.name().unwrap_or_default().into_owned();
        let Some(rel_path) = sanitize_archive_rel_path(&raw_name) else {
            tracker.items_done += 1;
            continue;
        };

        tracker.set_current_item(&raw_name);
        let out_path = dest_root.join(&rel_path);

        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| map_io_err(&out_path, e))?;
        } else if let Err(err) = write_extracted_stream(&out_path, conflict, &mut entry, tracker) {
            if entry_encrypted && password.is_some() && is_zip_decrypt_io_error(&err) {
                return Err(FsError::InvalidPassword(archive_path.to_path_buf()));
            }
            return Err(err);
        }
        tracker.items_done += 1;
    }

    Ok(())
}

fn extract_tar(
    archive_path: &Path,
    dest_root: &Path,
    gzip: bool,
    conflict: ConflictPolicy,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    let file = open_shared_archive(archive_path)?;
    let reader = BufReader::with_capacity(IO_BUF_BYTES, file);
    if gzip {
        let mut archive = TarArchive::new(GzDecoder::new(reader));
        extract_tar_stream(&mut archive, archive_path, dest_root, conflict, tracker)
    } else {
        let mut archive = TarArchive::new(reader);
        extract_tar_stream(&mut archive, archive_path, dest_root, conflict, tracker)
    }
}

fn extract_tar_stream<R: Read>(
    archive: &mut TarArchive<R>,
    archive_path: &Path,
    dest_root: &Path,
    conflict: ConflictPolicy,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    if let Ok(meta) = fs::metadata(archive_path) {
        tracker.bytes_total = meta.len();
    }
    let entries = archive.entries().map_err(|e| map_io_err(archive_path, e))?;

    for item in entries {
        if tracker.cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        let mut entry = item.map_err(|e| map_io_err(archive_path, e))?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            continue;
        }
        let raw_path = entry
            .path()
            .map_err(|e| map_io_err(archive_path, e))?
            .to_string_lossy()
            .into_owned();
        let Some(rel_path) = sanitize_archive_rel_path(&raw_path) else {
            continue;
        };
        tracker.set_current_item(&raw_path);
        let out_path = dest_root.join(&rel_path);

        if entry_type.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| map_io_err(&out_path, e))?;
        } else if entry_type.is_file() {
            write_extracted_stream(&out_path, conflict, &mut entry, tracker)?;
        }
        tracker.items_done += 1;
    }

    Ok(())
}

fn map_sevenz_error(
    err: sevenz_rust2::Error,
    archive_path: &Path,
    had_password: bool,
    known_encrypted: bool,
) -> FsError {
    match err {
        sevenz_rust2::Error::PasswordRequired => {
            if had_password {
                FsError::InvalidPassword(archive_path.to_path_buf())
            } else {
                FsError::PasswordRequired(archive_path.to_path_buf())
            }
        }
        sevenz_rust2::Error::MaybeBadPassword(_) => {
            if had_password {
                FsError::InvalidPassword(archive_path.to_path_buf())
            } else {
                FsError::PasswordRequired(archive_path.to_path_buf())
            }
        }
        sevenz_rust2::Error::ChecksumVerificationFailed
        | sevenz_rust2::Error::NextHeaderCrcMismatch
            if had_password || known_encrypted =>
        {
            if had_password {
                FsError::InvalidPassword(archive_path.to_path_buf())
            } else {
                FsError::PasswordRequired(archive_path.to_path_buf())
            }
        }
        other => FsError::Io {
            path: archive_path.to_path_buf(),
            message: format!("arquivo 7z inválido ou protegido por senha: {other}"),
        },
    }
}

fn extract_sevenz(
    archive_path: &Path,
    dest_root: &Path,
    conflict: ConflictPolicy,
    password: Option<&str>,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    let file = open_shared_archive(archive_path)?;
    let sz_password = password
        .map(SevenZPassword::from)
        .unwrap_or_else(SevenZPassword::empty);
    let mut reader = SevenZReader::new(BufReader::with_capacity(IO_BUF_BYTES, file), sz_password)
        .map_err(|e| map_sevenz_error(e, archive_path, password.is_some(), false))?;

    let has_encrypted_blocks = reader.archive().blocks.iter().any(|b| {
        b.coders
            .iter()
            .any(|c| c.encoder_method_id() == EncoderMethod::AES256_SHA256.id())
    });
    if has_encrypted_blocks && password.is_none() {
        return Err(FsError::PasswordRequired(archive_path.to_path_buf()));
    }

    tracker.items_total = reader.archive().files.len();
    tracker.bytes_total = reader
        .archive()
        .files
        .iter()
        .filter(|f| !f.is_directory)
        .map(|f| f.size)
        .sum();
    tracker.emit_force(JobState::Running);

    reader
        .for_each_entries(|entry, r| {
            if tracker.cancel.is_cancelled() {
                return Ok(false);
            }
            if entry.is_anti_item {
                tracker.items_done += 1;
                return Ok(true);
            }
            let Some(rel_path) = sanitize_archive_rel_path(entry.name()) else {
                tracker.items_done += 1;
                return Ok(true);
            };
            tracker.set_current_item(entry.name());
            let out_path = dest_root.join(&rel_path);

            if entry.is_directory {
                fs::create_dir_all(&out_path)?;
            } else {
                write_extracted_stream(&out_path, conflict, r, tracker)
                    .map_err(|e| io::Error::other(e.to_string()))?;
            }
            tracker.items_done += 1;
            Ok(true)
        })
        .map_err(|e| {
            if tracker.cancel.is_cancelled() {
                FsError::Cancelled
            } else {
                map_sevenz_error(e, archive_path, password.is_some(), has_encrypted_blocks)
            }
        })?;

    if tracker.cancel.is_cancelled() {
        return Err(FsError::Cancelled);
    }

    Ok(())
}

#[cfg(feature = "rar-extract")]
fn map_rar_error(err: unrar_rs::RarError, archive_path: &Path, had_password: bool) -> FsError {
    match err {
        unrar_rs::RarError::EncryptedArchive | unrar_rs::RarError::EncryptedMember { .. } => {
            if had_password {
                FsError::InvalidPassword(archive_path.to_path_buf())
            } else {
                FsError::PasswordRequired(archive_path.to_path_buf())
            }
        }
        unrar_rs::RarError::InvalidPassword | unrar_rs::RarError::WrongPassword { .. } => {
            FsError::InvalidPassword(archive_path.to_path_buf())
        }
        other => FsError::Io {
            path: archive_path.to_path_buf(),
            message: other.to_string(),
        },
    }
}

#[cfg(feature = "rar-extract")]
fn extract_rar(
    archive_path: &Path,
    dest_root: &Path,
    conflict: ConflictPolicy,
    password: Option<&str>,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    let mut archive = open_rar_archive_with_volumes(archive_path, password)?;
    let members: Vec<_> = archive.entries().collect();

    if password.is_none() && members.iter().any(|m| m.is_encrypted) {
        return Err(FsError::PasswordRequired(archive_path.to_path_buf()));
    }

    tracker.items_total = members.len();
    tracker.bytes_total = members
        .iter()
        .filter(|m| !m.is_directory)
        .map(|m| m.unpacked_size.unwrap_or(0))
        .sum();
    tracker.emit_force(JobState::Running);

    for (idx, member) in members.iter().enumerate() {
        if tracker.cancel.is_cancelled() {
            return Err(FsError::Cancelled);
        }
        if member.is_symlink || member.is_hardlink {
            tracker.items_done += 1;
            continue;
        }
        let Some(rel_path) = sanitize_archive_rel_path(&member.raw_name) else {
            tracker.items_done += 1;
            continue;
        };
        tracker.set_current_item(&member.raw_name);
        let out_path = dest_root.join(&rel_path);

        if member.is_directory {
            fs::create_dir_all(&out_path).map_err(|e| map_io_err(&out_path, e))?;
        } else {
            let Some(final_path) = resolve_extract_target_path(&out_path, conflict) else {
                tracker.items_done += 1;
                continue;
            };
            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent).map_err(|e| map_io_err(parent, e))?;
            }
            let tmp_part = partial_extract_path(&final_path);
            let mut guard = TempFileGuard::new(tmp_part.clone());
            let out_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_part)
                .map_err(|e| map_io_err(&tmp_part, e))?;

            let mut writer = TrackedCancelWriter {
                inner: BufWriter::with_capacity(IO_BUF_BYTES, out_file),
                tracker,
            };
            let mut entry = archive
                .by_index(idx)
                .map_err(|e| map_rar_error(e, archive_path, password.is_some()))?;
            if let Some(pw) = password {
                entry = entry.with_password(pw);
            }
            let copy_res = entry.copy_to(&mut writer);
            let flush_res = writer.inner.flush();
            drop(writer);

            if tracker.cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            copy_res.map_err(|e| map_rar_error(e, archive_path, password.is_some()))?;
            flush_res.map_err(|e| map_io_err(&tmp_part, e))?;
            if final_path.exists() {
                let _ = fs::remove_file(&final_path);
            }
            fs::rename(&tmp_part, &final_path).map_err(|e| map_io_err(&final_path, e))?;
            guard.disarm();
        }
        tracker.items_done += 1;
    }

    Ok(())
}

#[cfg(feature = "rar-extract")]
fn open_rar_archive_with_volumes(
    archive_path: &Path,
    password: Option<&str>,
) -> Result<unrar_rs::RarArchive, FsError> {
    let volumes = discover_rar_volume_paths(archive_path);
    let first_file = open_shared_archive(
        volumes
            .first()
            .map(PathBuf::as_path)
            .unwrap_or(archive_path),
    )?;
    let mut archive = match password {
        Some(pw) => unrar_rs::RarArchive::open_with_password(first_file, pw),
        None => unrar_rs::RarArchive::open(first_file),
    }
    .map_err(|e| map_rar_error(e, archive_path, password.is_some()))?;

    for (idx, vol_path) in volumes.iter().enumerate().skip(1) {
        let file = open_shared_archive(vol_path)?;
        archive
            .add_volume(idx, Box::new(file))
            .map_err(|e| map_rar_error(e, archive_path, password.is_some()))?;
    }

    Ok(archive)
}

/// Descobre volumes sequenciais `.part1.rar`, `.part2.rar`, ... na mesma pasta caso o alvo seja multi-volume.
#[cfg(feature = "rar-extract")]
fn discover_rar_volume_paths(first_path: &Path) -> Vec<PathBuf> {
    let Some(parent) = first_path.parent() else {
        return vec![first_path.to_path_buf()];
    };
    let Some(file_name) = first_path.file_name().and_then(|n| n.to_str()) else {
        return vec![first_path.to_path_buf()];
    };
    let lower = file_name.to_ascii_lowercase();
    let Some(without_rar) = lower.strip_suffix(".rar") else {
        return vec![first_path.to_path_buf()];
    };
    let Some(part_idx) = without_rar.rfind(".part") else {
        return vec![first_path.to_path_buf()];
    };
    let digits = &without_rar[part_idx + ".part".len()..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return vec![first_path.to_path_buf()];
    }

    let prefix = &file_name[..part_idx + ".part".len()];
    let ext_orig = &file_name[part_idx + ".part".len() + digits.len()..];
    let width = digits.len();

    let mut volumes = Vec::new();
    for idx in 1usize..=999 {
        let formatted = format!("{prefix}{idx:0width$}{ext_orig}");
        let candidate = parent.join(formatted);
        if candidate.is_file() {
            volumes.push(candidate);
        } else if idx > 1 {
            break;
        }
    }

    if volumes.is_empty() {
        vec![first_path.to_path_buf()]
    } else {
        volumes
    }
}

#[cfg(feature = "rar-extract")]
struct TrackedCancelWriter<'t, 'a, W> {
    inner: W,
    tracker: &'t mut ProgressTracker<'a>,
}

#[cfg(feature = "rar-extract")]
impl<W: Write> Write for TrackedCancelWriter<'_, '_, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.tracker.cancel.is_cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        let n = self.inner.write(buf)?;
        if n > 0 {
            self.tracker.advance_bytes(n as u64);
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn write_extracted_stream<R: Read + ?Sized>(
    out_path: &Path,
    conflict: ConflictPolicy,
    reader: &mut R,
    tracker: &mut ProgressTracker<'_>,
) -> Result<(), FsError> {
    let Some(final_path) = resolve_extract_target_path(out_path, conflict) else {
        return Ok(());
    };
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent).map_err(|e| map_io_err(parent, e))?;
    }

    let tmp_part = partial_extract_path(&final_path);
    let mut guard = TempFileGuard::new(tmp_part.clone());
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_part)
        .map_err(|e| map_io_err(&tmp_part, e))?;
    let mut buf_writer = BufWriter::with_capacity(IO_BUF_BYTES, file);
    {
        let mut tracked_reader = TrackedCancelReader {
            inner: reader,
            tracker,
        };
        if let Err(err) = io::copy(&mut tracked_reader, &mut buf_writer) {
            if tracker.cancel.is_cancelled() {
                return Err(FsError::Cancelled);
            }
            return Err(map_io_err(&final_path, err));
        }
    }
    buf_writer.flush().map_err(|e| map_io_err(&tmp_part, e))?;
    drop(buf_writer);

    if final_path.exists() {
        let _ = fs::remove_file(&final_path);
    }
    fs::rename(&tmp_part, &final_path).map_err(|e| map_io_err(&final_path, e))?;
    guard.disarm();
    Ok(())
}

fn partial_extract_path(final_path: &Path) -> PathBuf {
    let parent = final_path.parent().unwrap_or_else(|| Path::new("."));
    let name = final_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "item".to_owned());
    parent.join(format!(".__vfm_ext_{name}.tmp"))
}

fn resolve_extract_target_path(target: &Path, conflict: ConflictPolicy) -> Option<PathBuf> {
    if !target.exists() {
        return Some(target.to_path_buf());
    }
    match conflict {
        ConflictPolicy::Overwrite => Some(target.to_path_buf()),
        ConflictPolicy::Skip => None,
        ConflictPolicy::RenameAuto => Some(unique_sibling_file_path(target)),
    }
}

fn unique_dir_path(target: &Path, conflict: ConflictPolicy) -> PathBuf {
    if !target.exists() || conflict == ConflictPolicy::Overwrite {
        return target.to_path_buf();
    }
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let base = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "extracted".to_owned());
    for idx in 2..10_000 {
        let candidate = parent.join(format!("{base} ({idx})"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{base}_ext"))
}

fn unique_sibling_file_path(target: &Path) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "archive".to_owned());
    let (stem, ext) = if let Some((s, e)) = name.rsplit_once('.')
        && !s.is_empty()
        && !e.is_empty()
    {
        (s.to_owned(), Some(e.to_owned()))
    } else {
        (name, None)
    };
    for idx in 2..10_000 {
        let candidate_name = match &ext {
            Some(e) => format!("{stem} ({idx}).{e}"),
            None => format!("{stem} ({idx})"),
        };
        let candidate = parent.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    target.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn temp_workspace(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("vfm_archive_ops_{nanos}_{label}"));
        fs::create_dir_all(&dir).expect("mkdir");
        dir
    }

    fn run_async<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("rt")
            .block_on(fut)
    }

    #[test]
    fn compress_and_extract_roundtrip_all_writable_formats() {
        let root = temp_workspace("roundtrip");
        let src_dir = root.join("projeto");
        let nested = src_dir.join("sub");
        fs::create_dir_all(&nested).expect("mkdir nested");
        fs::write(src_dir.join("hello.txt"), b"hello world vagnarok").expect("write hello");
        fs::write(nested.join("data.bin"), [42u8; 2048]).expect("write bin");

        for (format, level) in [
            (ArchiveFormat::SevenZ, CompressionLevel::Store),
            (ArchiveFormat::SevenZ, CompressionLevel::Normal),
            (ArchiveFormat::SevenZ, CompressionLevel::Maximum),
            (ArchiveFormat::Zip, CompressionLevel::Minimum),
            (ArchiveFormat::Tar, CompressionLevel::Store),
            (ArchiveFormat::TarGz, CompressionLevel::Normal),
        ] {
            let out_dir = root.join(format!("out_{}_{:?}", format.extension(), level));
            fs::create_dir_all(&out_dir).expect("mkdir out");
            let (tx, mut rx) = mpsc::unbounded_channel();
            let cancel = CancellationToken::new();

            let archive_path = run_async(run_compress_job(
                1,
                vec![src_dir.clone()],
                out_dir.clone(),
                "bundle".to_owned(),
                format,
                level,
                ConflictPolicy::Overwrite,
                cancel.clone(),
                tx.clone(),
            ))
            .expect("compress ok");
            assert!(archive_path.is_file());

            let (tx2, _rx2) = mpsc::unbounded_channel();
            let extracted_dir = run_async(run_extract_job(
                2,
                archive_path.clone(),
                ExtractTargetMode::Subfolder,
                ConflictPolicy::Overwrite,
                None,
                cancel,
                tx2,
            ))
            .expect("extract ok");

            assert_eq!(
                fs::read(extracted_dir.join("projeto/hello.txt")).expect("read hello"),
                b"hello world vagnarok"
            );
            assert_eq!(
                fs::read(extracted_dir.join("projeto/sub/data.bin")).expect("read bin"),
                vec![42u8; 2048]
            );
            assert!(rx.try_recv().is_ok());
        }

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extract_zip_blocks_zip_slip_traversal() {
        let root = temp_workspace("zipslip");
        let zip_path = root.join("evil.zip");
        let extract_parent = root.join("inside");
        fs::create_dir_all(&extract_parent).expect("mkdir inside");

        {
            let f = File::create(&zip_path).expect("create zip");
            let mut w = ZipWriter::new(f);
            w.start_file("../../escaped.txt", SimpleFileOptions::default())
                .expect("start evil");
            w.write_all(b"pwn").expect("write evil");
            w.start_file("safe.txt", SimpleFileOptions::default())
                .expect("start safe");
            w.write_all(b"ok").expect("write safe");
            w.finish().expect("finish zip");
        }

        // Move zip into inside/ and extract in Here
        let moved_zip = extract_parent.join("evil.zip");
        fs::rename(&zip_path, &moved_zip).expect("move zip");

        let (tx, _rx) = mpsc::unbounded_channel();
        run_async(run_extract_job(
            3,
            moved_zip,
            ExtractTargetMode::Here,
            ConflictPolicy::Overwrite,
            None,
            CancellationToken::new(),
            tx,
        ))
        .expect("extract done");

        assert!(!root.join("escaped.txt").exists());
        assert_eq!(
            fs::read(extract_parent.join("safe.txt")).expect("read safe"),
            b"ok"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cancelling_compress_removes_partial_temp_and_skips_output() {
        let root = temp_workspace("cancel");
        let src_file = root.join("input.txt");
        fs::write(&src_file, b"sample").expect("write input");

        let cancel = CancellationToken::new();
        cancel.cancel();
        let (tx, _rx) = mpsc::unbounded_channel();

        let res = run_async(run_compress_job(
            99,
            vec![src_file],
            root.clone(),
            "aborted".to_owned(),
            ArchiveFormat::SevenZ,
            CompressionLevel::Normal,
            ConflictPolicy::RenameAuto,
            cancel,
            tx,
        ));
        assert!(matches!(res, Err(FsError::Cancelled)));
        assert!(!root.join("aborted.7z").exists());
        assert!(!root.join(".__vfm_pack_99__.tmp").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(feature = "rar-extract")]
    #[test]
    fn discovers_multipart_rar_volumes_sequentially() {
        let root = temp_workspace("rar_parts");
        let p1 = root.join("Game.part01.rar");
        let p2 = root.join("Game.part02.rar");
        fs::write(&p1, b"x").expect("p1");
        fs::write(&p2, b"x").expect("p2");

        let found = discover_rar_volume_paths(&p1);
        assert_eq!(found, vec![p1, p2]);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extract_encrypted_zip_aes_requires_valid_password() {
        let root = temp_workspace("zip_aes");
        let zip_path = root.join("secret.zip");
        {
            let f = File::create(&zip_path).expect("create zip");
            let mut w = ZipWriter::new(f);
            let opts = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .with_aes_encryption(zip::AesMode::Aes256, "vagnarok123");
            w.start_file("segredo.txt", opts).expect("start encrypted");
            w.write_all(b"conteudo ultra secreto")
                .expect("write secret");
            w.finish().expect("finish zip");
        }

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let err_missing = run_async(run_extract_job(
            10,
            zip_path.clone(),
            ExtractTargetMode::Subfolder,
            ConflictPolicy::Overwrite,
            None,
            CancellationToken::new(),
            tx1,
        ));
        assert!(matches!(err_missing, Err(FsError::PasswordRequired(_))));
        assert!(!root.join("secret").exists());

        let (tx2, _rx2) = mpsc::unbounded_channel();
        let err_wrong = run_async(run_extract_job(
            11,
            zip_path.clone(),
            ExtractTargetMode::Subfolder,
            ConflictPolicy::Overwrite,
            Some("senha_errada".to_owned()),
            CancellationToken::new(),
            tx2,
        ));
        assert!(matches!(err_wrong, Err(FsError::InvalidPassword(_))));
        assert!(!root.join("secret").exists());

        let (tx3, _rx3) = mpsc::unbounded_channel();
        let extracted = run_async(run_extract_job(
            12,
            zip_path,
            ExtractTargetMode::Subfolder,
            ConflictPolicy::Overwrite,
            Some("vagnarok123".to_owned()),
            CancellationToken::new(),
            tx3,
        ))
        .expect("extract with valid password ok");
        assert_eq!(
            fs::read(extracted.join("segredo.txt")).expect("read secret"),
            b"conteudo ultra secreto"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extract_encrypted_sevenz_requires_valid_password() {
        let root = temp_workspace("7z_aes");
        let sz_path = root.join("cofre.7z");
        {
            let f = File::create(&sz_path).expect("create 7z");
            let mut writer = SevenZWriter::new(f).expect("7z writer");
            writer.set_content_methods(vec![
                sevenz_rust2::encoder_options::AesEncoderOptions::new(SevenZPassword::from(
                    "chave7z!",
                ))
                .into(),
                EncoderConfiguration::new(EncoderMethod::LZMA2),
            ]);
            let mut entry = SevenZEntry::new();
            entry.name = "dados.txt".to_owned();
            entry.has_stream = true;
            writer
                .push_archive_entry(entry, Some(&b"payload cifrado 7z"[..]))
                .expect("push entry");
            writer.finish().expect("finish 7z");
        }

        let (tx1, _rx1) = mpsc::unbounded_channel();
        let err_missing = run_async(run_extract_job(
            20,
            sz_path.clone(),
            ExtractTargetMode::Subfolder,
            ConflictPolicy::Overwrite,
            None,
            CancellationToken::new(),
            tx1,
        ));
        assert!(matches!(err_missing, Err(FsError::PasswordRequired(_))));
        assert!(!root.join("cofre").exists());

        let (tx2, _rx2) = mpsc::unbounded_channel();
        let err_wrong = run_async(run_extract_job(
            21,
            sz_path.clone(),
            ExtractTargetMode::Subfolder,
            ConflictPolicy::Overwrite,
            Some("errada".to_owned()),
            CancellationToken::new(),
            tx2,
        ));
        assert!(matches!(err_wrong, Err(FsError::InvalidPassword(_))));
        assert!(!root.join("cofre").exists());

        let (tx3, _rx3) = mpsc::unbounded_channel();
        let extracted = run_async(run_extract_job(
            22,
            sz_path,
            ExtractTargetMode::Subfolder,
            ConflictPolicy::Overwrite,
            Some("chave7z!".to_owned()),
            CancellationToken::new(),
            tx3,
        ))
        .expect("extract 7z with valid password ok");
        assert_eq!(
            fs::read(extracted.join("dados.txt")).expect("read dados"),
            b"payload cifrado 7z"
        );

        let _ = fs::remove_dir_all(&root);
    }
}
