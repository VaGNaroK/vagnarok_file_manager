//! Motor em funil multi-estágios para busca de arquivos duplicados (MD5 + SHA-256 / Byte-a-Byte).
//!
//! - Pool `rayon` isolado (`available_parallelism().clamp(1, 4)`), nunca usando o pool global.
//! - Curto-circuito para arquivos `<= 32 KiB` (calcula MD5 Completo direto na Fase 2).
//! - Cache `redb` em lote (`put_md5_batch`) com bypass total nos níveis 3 e 4.
//! - Comparação byte a byte pareada 2-a-2 contra âncora (`<= 2` handles abertos por worker).

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, UNIX_EPOCH};

use compact_str::CompactString;
use fm_core::{
    DuplicateConfirmLevel, DuplicateFileItem, DuplicateGroup, DuplicateScanEvent,
    DuplicateScanOptions, DuplicateScanProgress, DuplicateScanStage, format_short_hash,
};
use ignore::{WalkBuilder, WalkState};
use rayon::prelude::*;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::hardlink::{FileIdentity, cache_integrity_stamp, file_identity};
use crate::hash::compute_sha256_blocking;
use crate::md5::{
    PARTIAL_SHORT_CIRCUIT_BYTES, STREAM_BUFFER_BYTES, compute_full_md5_blocking,
    compute_partial_md5_blocking, format_md5_hex,
};
use crate::meta_cache::DiskMetadataCache;
use crate::open_shared::open_shared_read;
use crate::stats::is_pseudo_fs_path;

const PROGRESS_THROTTLE: Duration = Duration::from_millis(70);

type Md5CacheEntry = (PathBuf, u64, u128, u128, [u8; 16]);
type Sha256HashedItem = (u64, [u8; 16], CompactString, CompactString, Candidate);
type Sha256BucketKey = (u64, [u8; 16], CompactString);
type Sha256BucketVal = (CompactString, Vec<Candidate>);

#[derive(Debug, Clone)]
struct Candidate {
    path: PathBuf,
    root_index: usize,
    size_bytes: u64,
    modified_nanos: Option<u128>,
    modified_secs: Option<i64>,
    integrity_stamp: u128,
    file_identity: Option<FileIdentity>,
    is_hardlink_sibling: bool,
    full_md5: Option<[u8; 16]>,
}

struct ProgressEmitter {
    tx: mpsc::UnboundedSender<DuplicateScanEvent>,
    stage_total: u8,
    last_sent: Mutex<Instant>,
}

impl ProgressEmitter {
    fn new(tx: mpsc::UnboundedSender<DuplicateScanEvent>, stage_total: u8) -> Self {
        Self {
            tx,
            stage_total,
            // Permite emitir o primeiro evento imediatamente.
            last_sent: Mutex::new(Instant::now() - PROGRESS_THROTTLE),
        }
    }

    fn emit(
        &self,
        force: bool,
        stage: DuplicateScanStage,
        stage_index: u8,
        processed: usize,
        total: usize,
        current_path: &Path,
    ) {
        if !force {
            let Ok(mut guard) = self.last_sent.lock() else {
                return;
            };
            if guard.elapsed() < PROGRESS_THROTTLE {
                return;
            }
            *guard = Instant::now();
        } else if let Ok(mut guard) = self.last_sent.lock() {
            *guard = Instant::now();
        }

        let name = current_path
            .file_name()
            .map(|n| CompactString::from(n.to_string_lossy().as_ref()))
            .unwrap_or_else(|| CompactString::from(current_path.to_string_lossy().as_ref()));

        let _ = self
            .tx
            .send(DuplicateScanEvent::Progress(DuplicateScanProgress {
                stage,
                stage_index,
                stage_total: self.stage_total,
                processed,
                total,
                groups_found: 0,
                reclaimable_bytes: 0,
                current_path: name,
            }));
    }
}

/// Executa o funil completo de busca de duplicados em uma tarefa bloqueante dedicada.
pub async fn run_duplicate_scan(
    options: DuplicateScanOptions,
    cache: Option<Arc<DiskMetadataCache>>,
    cancel: CancellationToken,
    tx: mpsc::UnboundedSender<DuplicateScanEvent>,
) {
    let tx_err = tx.clone();
    let res = tokio::task::spawn_blocking(move || {
        run_duplicate_scan_blocking(options, cache, cancel, tx)
    })
    .await;
    if let Err(err) = res {
        let _ = tx_err.send(DuplicateScanEvent::Failed(format!(
            "duplicate scan worker panicked: {err}"
        )));
    }
}

fn worker_thread_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().clamp(1, 4))
        .unwrap_or(4)
}

fn run_duplicate_scan_blocking(
    options: DuplicateScanOptions,
    cache: Option<Arc<DiskMetadataCache>>,
    cancel: CancellationToken,
    tx: mpsc::UnboundedSender<DuplicateScanEvent>,
) {
    if options.roots.is_empty() {
        let _ = tx.send(DuplicateScanEvent::Completed {
            groups: Vec::new(),
            scanned_files: 0,
            skipped_errors: 0,
        });
        return;
    }

    let workers = worker_thread_count();
    let pool = match rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .thread_name(|i| format!("fm-dup-{i}"))
        .build()
    {
        Ok(p) => p,
        Err(err) => {
            let _ = tx.send(DuplicateScanEvent::Failed(format!(
                "failed to create duplicate worker pool: {err}"
            )));
            return;
        }
    };

    let stage_total = options.confirm_level.stage_total();
    let emitter = Arc::new(ProgressEmitter::new(tx.clone(), stage_total));
    let skipped_errors = AtomicUsize::new(0);

    // =========================================================================
    // ESTÁGIO 1: Walk + Metadata + Bucketing por Tamanho + Filtro de Hardlinks
    // =========================================================================
    let (size_buckets, scanned_files) =
        stage1_collect_by_size(&options, workers, &cancel, &emitter, &skipped_errors);
    if cancel.is_cancelled() {
        let _ = tx.send(DuplicateScanEvent::Cancelled);
        return;
    }

    let mut stage1_candidates: Vec<Candidate> = Vec::new();
    for (_size, mut bucket) in size_buckets {
        if bucket.len() < 2 {
            continue;
        }
        apply_hardlink_filter(&mut bucket, options.ignore_hardlinks);
        if bucket.len() >= 2 {
            stage1_candidates.extend(bucket);
        }
    }

    if stage1_candidates.is_empty() {
        let _ = tx.send(DuplicateScanEvent::Completed {
            groups: Vec::new(),
            scanned_files,
            skipped_errors: skipped_errors.load(Ordering::Relaxed),
        });
        return;
    }

    // =========================================================================
    // ESTÁGIO 2: MD5 Parcial + Curto-Circuito para arquivos <= 32 KiB
    // =========================================================================
    let stage2_total = stage1_candidates.len();
    let stage2_done = AtomicUsize::new(0);
    emitter.emit(
        true,
        DuplicateScanStage::PartialMd5,
        2,
        0,
        stage2_total,
        &options.roots[0],
    );

    let stage2_results: Vec<Option<([u8; 16], Candidate)>> = pool.install(|| {
        stage1_candidates
            .into_par_iter()
            .map(|mut cand| {
                if cancel.is_cancelled() {
                    return None;
                }
                let done = stage2_done.fetch_add(1, Ordering::Relaxed) + 1;
                emitter.emit(
                    false,
                    DuplicateScanStage::PartialMd5,
                    2,
                    done,
                    stage2_total,
                    &cand.path,
                );

                let key = if cand.size_bytes <= PARTIAL_SHORT_CIRCUIT_BYTES {
                    match compute_full_md5_blocking(&cand.path, &cancel) {
                        Some(digest) => {
                            cand.full_md5 = Some(digest);
                            digest
                        }
                        None => {
                            if !cancel.is_cancelled() {
                                skipped_errors.fetch_add(1, Ordering::Relaxed);
                            }
                            return None;
                        }
                    }
                } else {
                    match compute_partial_md5_blocking(&cand.path, cand.size_bytes, &cancel) {
                        Some(digest) => digest,
                        None => {
                            if !cancel.is_cancelled() {
                                skipped_errors.fetch_add(1, Ordering::Relaxed);
                            }
                            return None;
                        }
                    }
                };
                Some((key, cand))
            })
            .collect()
    });

    if cancel.is_cancelled() {
        let _ = tx.send(DuplicateScanEvent::Cancelled);
        return;
    }

    let mut partial_buckets: HashMap<(u64, [u8; 16]), Vec<Candidate>> = HashMap::new();
    for entry in stage2_results.into_iter().flatten() {
        partial_buckets
            .entry((entry.1.size_bytes, entry.0))
            .or_default()
            .push(entry.1);
    }

    // Se o usuário pediu apenas Nível 1 (Rápido / Triagem), finaliza aqui!
    if options.confirm_level == DuplicateConfirmLevel::QuickPartialMd5 {
        let mut groups = Vec::new();
        for ((size_bytes, digest), items) in partial_buckets {
            if items.len() < 2 {
                continue;
            }
            let verified = if items.iter().all(|c| c.full_md5.is_some()) {
                DuplicateConfirmLevel::StandardFullMd5
            } else {
                DuplicateConfirmLevel::QuickPartialMd5
            };
            groups.push(build_duplicate_group(
                size_bytes,
                digest,
                None,
                verified,
                items,
                &options.roots,
            ));
        }
        finalize_and_sort_groups(&mut groups);
        let _ = tx.send(DuplicateScanEvent::Completed {
            groups,
            scanned_files,
            skipped_errors: skipped_errors.load(Ordering::Relaxed),
        });
        return;
    }

    let stage3_candidates: Vec<Candidate> = partial_buckets
        .into_values()
        .filter(|b| b.len() >= 2)
        .flatten()
        .collect();

    if stage3_candidates.is_empty() {
        let _ = tx.send(DuplicateScanEvent::Completed {
            groups: Vec::new(),
            scanned_files,
            skipped_errors: skipped_errors.load(Ordering::Relaxed),
        });
        return;
    }

    // =========================================================================
    // ESTÁGIO 3: MD5 Completo + Cache redb em Lote (com Bypass nos níveis 3 e 4)
    // =========================================================================
    let stage3_total = stage3_candidates.len();
    let stage3_done = AtomicUsize::new(0);
    let allow_cache_read = options.confirm_level.uses_disk_cache();
    let new_cache_entries: Mutex<Vec<Md5CacheEntry>> = Mutex::new(Vec::new());

    emitter.emit(
        true,
        DuplicateScanStage::FullMd5,
        3,
        0,
        stage3_total,
        &options.roots[0],
    );

    let stage3_results: Vec<Option<([u8; 16], Candidate)>> = pool.install(|| {
        stage3_candidates
            .into_par_iter()
            .map(|mut cand| {
                if cancel.is_cancelled() {
                    return None;
                }
                let done = stage3_done.fetch_add(1, Ordering::Relaxed) + 1;
                emitter.emit(
                    false,
                    DuplicateScanStage::FullMd5,
                    3,
                    done,
                    stage3_total,
                    &cand.path,
                );

                // 1) Reaproveita curto-circuito da Fase 2 (<= 32 KiB) sem reabrir o arquivo.
                if let Some(digest) = cand.full_md5 {
                    if let (Some(_), Some(mtime)) = (&cache, cand.modified_nanos)
                        && let Ok(mut guard) = new_cache_entries.lock()
                    {
                        guard.push((
                            cand.path.clone(),
                            cand.size_bytes,
                            mtime,
                            cand.integrity_stamp,
                            digest,
                        ));
                    }
                    return Some((digest, cand));
                }

                // 2) Consulta cache redb apenas quando permitido (níveis 1 e 2; bypass total em 3 e 4).
                if allow_cache_read
                    && let (Some(disk_cache), Some(mtime)) = (&cache, cand.modified_nanos)
                    && let Some(cached) =
                        disk_cache.get_md5(&cand.path, cand.size_bytes, mtime, cand.integrity_stamp)
                {
                    cand.full_md5 = Some(cached);
                    return Some((cached, cand));
                }

                // 3) Calcula MD5 completo via streaming 64 KiB.
                let Some(digest) = compute_full_md5_blocking(&cand.path, &cancel) else {
                    if !cancel.is_cancelled() {
                        skipped_errors.fetch_add(1, Ordering::Relaxed);
                    }
                    return None;
                };
                cand.full_md5 = Some(digest);
                if let (Some(_), Some(mtime)) = (&cache, cand.modified_nanos)
                    && let Ok(mut guard) = new_cache_entries.lock()
                {
                    guard.push((
                        cand.path.clone(),
                        cand.size_bytes,
                        mtime,
                        cand.integrity_stamp,
                        digest,
                    ));
                }
                Some((digest, cand))
            })
            .collect()
    });

    if cancel.is_cancelled() {
        let _ = tx.send(DuplicateScanEvent::Cancelled);
        return;
    }

    // Persiste todos os novos hashes MD5 em lote (evita fsync por arquivo).
    if let Some(disk_cache) = &cache
        && let Ok(entries) = new_cache_entries.into_inner()
        && !entries.is_empty()
    {
        let refs: Vec<(&Path, u64, u128, u128, [u8; 16])> = entries
            .iter()
            .map(|(p, sz, mt, st, d)| (p.as_path(), *sz, *mt, *st, *d))
            .collect();
        disk_cache.put_md5_batch(&refs);
    }

    let mut md5_buckets: HashMap<(u64, [u8; 16]), Vec<Candidate>> = HashMap::new();
    for entry in stage3_results.into_iter().flatten() {
        md5_buckets
            .entry((entry.1.size_bytes, entry.0))
            .or_default()
            .push(entry.1);
    }

    let matched_md5_groups: Vec<(u64, [u8; 16], Vec<Candidate>)> = md5_buckets
        .into_iter()
        .filter_map(|((sz, digest), items)| (items.len() >= 2).then_some((sz, digest, items)))
        .collect();

    if matched_md5_groups.is_empty() {
        let _ = tx.send(DuplicateScanEvent::Completed {
            groups: Vec::new(),
            scanned_files,
            skipped_errors: skipped_errors.load(Ordering::Relaxed),
        });
        return;
    }

    // =========================================================================
    // ESTÁGIO 4: Confirmação Zero-Colisão (SHA-256 ou Byte-a-Byte 2-a-2)
    // =========================================================================
    let mut groups = match options.confirm_level {
        DuplicateConfirmLevel::QuickPartialMd5 | DuplicateConfirmLevel::StandardFullMd5 => {
            matched_md5_groups
                .into_iter()
                .map(|(sz, digest, items)| {
                    build_duplicate_group(
                        sz,
                        digest,
                        None,
                        DuplicateConfirmLevel::StandardFullMd5,
                        items,
                        &options.roots,
                    )
                })
                .collect()
        }
        DuplicateConfirmLevel::StrictMd5AndSha256 => stage4_verify_sha256(
            matched_md5_groups,
            &options.roots,
            &pool,
            &cancel,
            &emitter,
            &skipped_errors,
        ),
        DuplicateConfirmLevel::ParanoidMd5AndByteCompare => stage4_verify_byte_by_byte(
            matched_md5_groups,
            &options.roots,
            &pool,
            &cancel,
            &emitter,
            &skipped_errors,
        ),
    };

    if cancel.is_cancelled() {
        let _ = tx.send(DuplicateScanEvent::Cancelled);
        return;
    }

    finalize_and_sort_groups(&mut groups);
    let _ = tx.send(DuplicateScanEvent::Completed {
        groups,
        scanned_files,
        skipped_errors: skipped_errors.load(Ordering::Relaxed),
    });
}

fn stage1_collect_by_size(
    options: &DuplicateScanOptions,
    workers: usize,
    cancel: &CancellationToken,
    emitter: &Arc<ProgressEmitter>,
    skipped_errors: &AtomicUsize,
) -> (HashMap<u64, Vec<Candidate>>, usize) {
    let min_size = options.effective_min_size();
    let buckets: Arc<Mutex<HashMap<u64, Vec<Candidate>>>> = Arc::new(Mutex::new(HashMap::new()));
    let scanned_files = Arc::new(AtomicUsize::new(0));

    emitter.emit(
        true,
        DuplicateScanStage::Walking,
        1,
        0,
        0,
        &options.roots[0],
    );

    for (root_idx, root) in options.roots.iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }

        let mut builder = WalkBuilder::new(root);
        builder
            .standard_filters(false)
            .hidden(!options.include_hidden)
            .follow_links(false)
            .threads(workers)
            .filter_entry(|entry| {
                // Nunca desce em pseudo-filesystems Linux caso o usuário escaneie '/'.
                !is_pseudo_fs_path(entry.path())
            });
        if !options.recursive {
            builder.max_depth(Some(1));
        }

        let walker = builder.build_parallel();
        walker.run(|| {
            let buckets = Arc::clone(&buckets);
            let scanned_files = Arc::clone(&scanned_files);
            let emitter = Arc::clone(emitter);
            let cancel = cancel.clone();

            Box::new(move |entry_res| {
                if cancel.is_cancelled() {
                    return WalkState::Quit;
                }
                let Ok(entry) = entry_res else {
                    skipped_errors.fetch_add(1, Ordering::Relaxed);
                    return WalkState::Continue;
                };
                let Some(ft) = entry.file_type() else {
                    return WalkState::Continue;
                };
                if ft.is_symlink() || !ft.is_file() {
                    return WalkState::Continue;
                }
                let Ok(meta) = entry.metadata() else {
                    skipped_errors.fetch_add(1, Ordering::Relaxed);
                    return WalkState::Continue;
                };
                let size_bytes = meta.len();
                let count = scanned_files.fetch_add(1, Ordering::Relaxed) + 1;
                emitter.emit(
                    false,
                    DuplicateScanStage::Walking,
                    1,
                    count,
                    0,
                    entry.path(),
                );

                if size_bytes < min_size {
                    return WalkState::Continue;
                }

                let (modified_nanos, modified_secs) = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| (Some(d.as_nanos()), Some(d.as_secs() as i64)))
                    .unwrap_or((None, None));
                let integrity_stamp = cache_integrity_stamp(&meta);

                // No Linux/Unix, `(dev, ino)` já vem de graça em `meta` sem syscall extra.
                #[cfg(unix)]
                let cached_id = file_identity(entry.path(), &meta);
                #[cfg(not(unix))]
                let cached_id = None;

                let cand = Candidate {
                    path: entry.into_path(),
                    root_index: root_idx,
                    size_bytes,
                    modified_nanos,
                    modified_secs,
                    integrity_stamp,
                    file_identity: cached_id,
                    is_hardlink_sibling: false,
                    full_md5: None,
                };

                if let Ok(mut map) = buckets.lock() {
                    map.entry(size_bytes).or_default().push(cand);
                }
                WalkState::Continue
            })
        });
    }

    let total_scanned = scanned_files.load(Ordering::Relaxed);
    let final_map = match Arc::try_unwrap(buckets) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => arc.lock().map(|m| m.clone()).unwrap_or_default(),
    };
    (final_map, total_scanned)
}

/// Resolve identidade física para candidatos que empataram em tamanho e filtra ou marca hardlinks.
fn apply_hardlink_filter(bucket: &mut Vec<Candidate>, ignore_hardlinks: bool) {
    // No Windows, obtemos `file_identity` apenas aqui (buckets com >= 2 arquivos), evitando
    // abrir handles desnecessários para arquivos de tamanho único.
    #[cfg(not(unix))]
    for cand in bucket.iter_mut() {
        if cand.file_identity.is_none()
            && let Ok(meta) = std::fs::symlink_metadata(&cand.path)
        {
            cand.file_identity = file_identity(&cand.path, &meta);
        }
    }

    // Ordenação determinística por caminho garante escolha estável ao ignorar hardlinks.
    bucket.sort_by(|a, b| a.path.as_os_str().cmp(b.path.as_os_str()));

    if ignore_hardlinks {
        let mut seen: HashSet<FileIdentity> = HashSet::new();
        bucket.retain(|cand| match cand.file_identity {
            Some(id) => seen.insert(id),
            None => true,
        });
    } else {
        let mut counts: HashMap<FileIdentity, usize> = HashMap::new();
        for cand in bucket.iter() {
            if let Some(id) = cand.file_identity {
                *counts.entry(id).or_insert(0) += 1;
            }
        }
        for cand in bucket.iter_mut() {
            if let Some(id) = cand.file_identity
                && counts.get(&id).copied().unwrap_or(0) >= 2
            {
                cand.is_hardlink_sibling = true;
            }
        }
    }
}

fn stage4_verify_sha256(
    md5_groups: Vec<(u64, [u8; 16], Vec<Candidate>)>,
    roots: &[PathBuf],
    pool: &rayon::ThreadPool,
    cancel: &CancellationToken,
    emitter: &Arc<ProgressEmitter>,
    skipped_errors: &AtomicUsize,
) -> Vec<DuplicateGroup> {
    let total_files: usize = md5_groups.iter().map(|(_, _, items)| items.len()).sum();
    let done_counter = AtomicUsize::new(0);
    emitter.emit(
        true,
        DuplicateScanStage::Sha256Verify,
        4,
        0,
        total_files,
        &roots[0],
    );

    let flat_items: Vec<(u64, [u8; 16], Candidate)> = md5_groups
        .into_iter()
        .flat_map(|(sz, md5, items)| items.into_iter().map(move |c| (sz, md5, c)))
        .collect();

    let hashed: Vec<Option<Sha256HashedItem>> = pool.install(|| {
        flat_items
            .into_par_iter()
            .map(|(sz, md5, cand)| {
                if cancel.is_cancelled() {
                    return None;
                }
                let done = done_counter.fetch_add(1, Ordering::Relaxed) + 1;
                emitter.emit(
                    false,
                    DuplicateScanStage::Sha256Verify,
                    4,
                    done,
                    total_files,
                    &cand.path,
                );
                let Some((prefix, full)) = compute_sha256_blocking(&cand.path, cancel) else {
                    if !cancel.is_cancelled() {
                        skipped_errors.fetch_add(1, Ordering::Relaxed);
                    }
                    return None;
                };
                Some((sz, md5, prefix, full, cand))
            })
            .collect()
    });

    if cancel.is_cancelled() {
        return Vec::new();
    }

    let mut sha_buckets: HashMap<Sha256BucketKey, Sha256BucketVal> = HashMap::new();
    for (sz, md5, prefix, full, cand) in hashed.into_iter().flatten() {
        sha_buckets
            .entry((sz, md5, full))
            .or_insert_with(|| (prefix, Vec::new()))
            .1
            .push(cand);
    }

    sha_buckets
        .into_iter()
        .filter_map(|((sz, md5, _), (sha_prefix, items))| {
            (items.len() >= 2).then(|| {
                build_duplicate_group(
                    sz,
                    md5,
                    Some(sha_prefix),
                    DuplicateConfirmLevel::StrictMd5AndSha256,
                    items,
                    roots,
                )
            })
        })
        .collect()
}

fn stage4_verify_byte_by_byte(
    md5_groups: Vec<(u64, [u8; 16], Vec<Candidate>)>,
    roots: &[PathBuf],
    pool: &rayon::ThreadPool,
    cancel: &CancellationToken,
    emitter: &Arc<ProgressEmitter>,
    skipped_errors: &AtomicUsize,
) -> Vec<DuplicateGroup> {
    let total_files: usize = md5_groups.iter().map(|(_, _, items)| items.len()).sum();
    let done_counter = AtomicUsize::new(0);
    emitter.emit(
        true,
        DuplicateScanStage::ByteCompare,
        4,
        0,
        total_files,
        &roots[0],
    );

    let partitioned: Vec<Vec<DuplicateGroup>> = pool.install(|| {
        md5_groups
            .into_par_iter()
            .map(|(sz, md5, items)| {
                if cancel.is_cancelled() {
                    return Vec::new();
                }
                let mut buf_a = vec![0u8; STREAM_BUFFER_BYTES];
                let mut buf_b = vec![0u8; STREAM_BUFFER_BYTES];

                let subgroups = partition_bucket_byte_by_byte(
                    items,
                    &mut buf_a,
                    &mut buf_b,
                    cancel,
                    |path| {
                        let done = done_counter.fetch_add(1, Ordering::Relaxed) + 1;
                        emitter.emit(
                            false,
                            DuplicateScanStage::ByteCompare,
                            4,
                            done,
                            total_files,
                            path,
                        );
                    },
                    || {
                        skipped_errors.fetch_add(1, Ordering::Relaxed);
                    },
                );

                subgroups
                    .into_iter()
                    .filter(|sg| sg.len() >= 2)
                    .map(|sg| {
                        build_duplicate_group(
                            sz,
                            md5,
                            None,
                            DuplicateConfirmLevel::ParanoidMd5AndByteCompare,
                            sg,
                            roots,
                        )
                    })
                    .collect()
            })
            .collect()
    });

    partitioned.into_iter().flatten().collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PairCompareResult {
    Equal,
    Different,
    AnchorReadError,
    CandidateReadError,
    Cancelled,
}

/// Particiona um bucket de candidatos por comparação física 2-a-2 contra âncoras de subgrupos.
///
/// Mantém no máximo 2 file handles abertos simultaneamente e reutiliza `buf_a` e `buf_b` sem
/// alocações no heap dentro do loop de leitura.
fn partition_bucket_byte_by_byte<FProgress, FErr>(
    items: Vec<Candidate>,
    buf_a: &mut [u8],
    buf_b: &mut [u8],
    cancel: &CancellationToken,
    on_progress: FProgress,
    on_error: FErr,
) -> Vec<Vec<Candidate>>
where
    FProgress: Fn(&Path),
    FErr: Fn(),
{
    let mut subgroups: Vec<Vec<Candidate>> = Vec::new();

    'candidates: for cand in items {
        if cancel.is_cancelled() {
            return Vec::new();
        }
        on_progress(&cand.path);

        let mut sg_idx = 0;
        while sg_idx < subgroups.len() {
            if cancel.is_cancelled() {
                return Vec::new();
            }
            if subgroups[sg_idx].is_empty() {
                subgroups.swap_remove(sg_idx);
                continue;
            }
            let anchor_path = subgroups[sg_idx][0].path.clone();
            match compare_two_files_bytes(&anchor_path, &cand.path, buf_a, buf_b, cancel) {
                PairCompareResult::Equal => {
                    subgroups[sg_idx].push(cand);
                    continue 'candidates;
                }
                PairCompareResult::Different => {
                    sg_idx += 1;
                }
                PairCompareResult::AnchorReadError => {
                    // Promove o próximo elemento do subgrupo sem descartar o restante.
                    on_error();
                    subgroups[sg_idx].remove(0);
                    if subgroups[sg_idx].is_empty() {
                        subgroups.swap_remove(sg_idx);
                    }
                }
                PairCompareResult::CandidateReadError => {
                    on_error();
                    continue 'candidates;
                }
                PairCompareResult::Cancelled => {
                    return Vec::new();
                }
            }
        }

        subgroups.push(vec![cand]);
    }

    subgroups
}

fn compare_two_files_bytes(
    anchor_path: &Path,
    cand_path: &Path,
    buf_a: &mut [u8],
    buf_b: &mut [u8],
    cancel: &CancellationToken,
) -> PairCompareResult {
    if cancel.is_cancelled() {
        return PairCompareResult::Cancelled;
    }
    let Ok(mut file_a) = open_shared_read(anchor_path) else {
        return PairCompareResult::AnchorReadError;
    };
    let Ok(mut file_b) = open_shared_read(cand_path) else {
        return PairCompareResult::CandidateReadError;
    };

    loop {
        if cancel.is_cancelled() {
            return PairCompareResult::Cancelled;
        }
        let n_a = match read_full_chunk(&mut file_a, buf_a, cancel) {
            ChunkRead::Bytes(n) => n,
            ChunkRead::Error => return PairCompareResult::AnchorReadError,
            ChunkRead::Cancelled => return PairCompareResult::Cancelled,
        };
        let n_b = match read_full_chunk(&mut file_b, buf_b, cancel) {
            ChunkRead::Bytes(n) => n,
            ChunkRead::Error => return PairCompareResult::CandidateReadError,
            ChunkRead::Cancelled => return PairCompareResult::Cancelled,
        };

        if n_a != n_b {
            return PairCompareResult::Different;
        }
        if n_a == 0 {
            return PairCompareResult::Equal;
        }
        if buf_a[..n_a] != buf_b[..n_b] {
            return PairCompareResult::Different;
        }
    }
}

enum ChunkRead {
    Bytes(usize),
    Error,
    Cancelled,
}

fn read_full_chunk<R: Read>(
    reader: &mut R,
    buf: &mut [u8],
    cancel: &CancellationToken,
) -> ChunkRead {
    let mut filled = 0;
    while filled < buf.len() {
        if cancel.is_cancelled() {
            return ChunkRead::Cancelled;
        }
        match reader.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return ChunkRead::Error,
        }
    }
    ChunkRead::Bytes(filled)
}

fn build_duplicate_group(
    size_bytes: u64,
    md5_digest: [u8; 16],
    sha256_prefix: Option<CompactString>,
    verified_level: DuplicateConfirmLevel,
    mut candidates: Vec<Candidate>,
    roots: &[PathBuf],
) -> DuplicateGroup {
    candidates.sort_by(|a, b| a.path.as_os_str().cmp(b.path.as_os_str()));
    let md5_hex = CompactString::from(format_md5_hex(&md5_digest));
    let md5_short = format_short_hash(md5_hex.as_str());

    let items = candidates
        .into_iter()
        .map(|cand| {
            let root = roots.get(cand.root_index);
            let rel = root
                .and_then(|r| cand.path.strip_prefix(r).ok())
                .map(|r| {
                    let s = r.to_string_lossy();
                    if s.is_empty() {
                        cand.path.to_string_lossy().into_owned()
                    } else {
                        format!("/{s}")
                    }
                })
                .unwrap_or_else(|| cand.path.to_string_lossy().into_owned());

            DuplicateFileItem {
                path: cand.path,
                relative_display: CompactString::from(rel),
                size_bytes: cand.size_bytes,
                modified_nanos: cand.modified_nanos,
                modified_secs: cand.modified_secs,
                file_identity: cand.file_identity,
                is_hardlink_sibling: cand.is_hardlink_sibling,
                selected_for_delete: false,
            }
        })
        .collect();

    DuplicateGroup {
        group_id: 0,
        size_bytes,
        md5_hex,
        md5_short,
        sha256_prefix,
        verified_level,
        items,
    }
}

fn finalize_and_sort_groups(groups: &mut [DuplicateGroup]) {
    groups.sort_by(|a, b| {
        b.reclaimable_bytes()
            .cmp(&a.reclaimable_bytes())
            .then_with(|| b.size_bytes.cmp(&a.size_bytes))
            .then_with(|| a.md5_hex.cmp(&b.md5_hex))
    });
    for (idx, group) in groups.iter_mut().enumerate() {
        group.group_id = (idx + 1) as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "vagnarok_dup_test_{tag}_{}_{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn collect_scan_result(
        options: DuplicateScanOptions,
        cache: Option<Arc<DiskMetadataCache>>,
        cancel: CancellationToken,
    ) -> DuplicateScanEvent {
        let rt = tokio::runtime::Runtime::new().expect("tokio rt");
        rt.block_on(async move {
            let (tx, mut rx) = mpsc::unbounded_channel();
            run_duplicate_scan(options, cache, cancel, tx).await;
            let mut last_terminal = None;
            while let Some(ev) = rx.recv().await {
                if !matches!(ev, DuplicateScanEvent::Progress(_)) {
                    last_terminal = Some(ev);
                }
            }
            last_terminal.expect("terminal scan event")
        })
    }

    #[test]
    fn small_files_short_circuit_and_large_files_distinguish_middle_and_header_diffs() {
        let root = make_temp_dir("funnel");
        // Arquivos <= 32 KiB (curto-circuito direto para MD5 completo).
        std::fs::write(root.join("small_a.bin"), vec![0x42u8; 4096]).unwrap();
        std::fs::write(root.join("small_b.bin"), vec![0x42u8; 4096]).unwrap();

        // Arquivos > 32 KiB (64 KiB): par idêntico + 1 com diferença fora dos 3 blocos de 8 KiB
        // + 1 com diferença no cabeçalho.
        let base_large = vec![0x11u8; 64 * 1024];
        std::fs::write(root.join("big_a.bin"), &base_large).unwrap();
        std::fs::write(root.join("big_b.bin"), &base_large).unwrap();

        let mut diff_gap = base_large.clone();
        // Offset 12 KiB fica entre o bloco inicial [0..8K] e o bloco central [28K..36K].
        diff_gap[12 * 1024] = 0xFF;
        std::fs::write(root.join("big_diff_gap.bin"), &diff_gap).unwrap();

        let mut diff_head = base_large.clone();
        diff_head[10] = 0xEE;
        std::fs::write(root.join("big_diff_head.bin"), &diff_head).unwrap();

        // 1) Modo Rápido: agrupa big_a + big_b + big_diff_gap (descarta big_diff_head!),
        //    e small_a + small_b já saem promovidos a StandardFullMd5 pelo curto-circuito!
        let mut quick_opts = DuplicateScanOptions::for_root(root.clone());
        quick_opts.confirm_level = DuplicateConfirmLevel::QuickPartialMd5;
        quick_opts.min_size_bytes = 1;
        let DuplicateScanEvent::Completed { groups, .. } =
            collect_scan_result(quick_opts, None, CancellationToken::new())
        else {
            panic!("expected completed");
        };
        assert_eq!(groups.len(), 2);
        let big_group = groups.iter().find(|g| g.size_bytes == 64 * 1024).unwrap();
        assert_eq!(big_group.items.len(), 3);
        assert_eq!(
            big_group.verified_level,
            DuplicateConfirmLevel::QuickPartialMd5
        );
        let small_group = groups.iter().find(|g| g.size_bytes == 4096).unwrap();
        assert_eq!(small_group.items.len(), 2);
        assert_eq!(
            small_group.verified_level,
            DuplicateConfirmLevel::StandardFullMd5
        );

        // 2) Modo Padrão (MD5 Completo): separa big_diff_gap, mantendo apenas big_a + big_b.
        let mut std_opts = DuplicateScanOptions::for_root(root.clone());
        std_opts.confirm_level = DuplicateConfirmLevel::StandardFullMd5;
        std_opts.min_size_bytes = 1;
        let DuplicateScanEvent::Completed { groups, .. } =
            collect_scan_result(std_opts, None, CancellationToken::new())
        else {
            panic!("expected completed");
        };
        assert_eq!(groups.len(), 2);
        let big_group = groups.iter().find(|g| g.size_bytes == 64 * 1024).unwrap();
        assert_eq!(big_group.items.len(), 2);
        assert_eq!(
            big_group.verified_level,
            DuplicateConfirmLevel::StandardFullMd5
        );

        // 3) Modo Seguro (SHA-256) e Modo Máximo (Byte-a-Byte).
        for level in [
            DuplicateConfirmLevel::StrictMd5AndSha256,
            DuplicateConfirmLevel::ParanoidMd5AndByteCompare,
        ] {
            let mut opts = DuplicateScanOptions::for_root(root.clone());
            opts.confirm_level = level;
            opts.min_size_bytes = 1;
            let DuplicateScanEvent::Completed { groups, .. } =
                collect_scan_result(opts, None, CancellationToken::new())
            else {
                panic!("expected completed for {level:?}");
            };
            assert_eq!(groups.len(), 2);
            let big = groups.iter().find(|g| g.size_bytes == 64 * 1024).unwrap();
            assert_eq!(big.items.len(), 2);
            assert_eq!(big.verified_level, level);
            if level == DuplicateConfirmLevel::StrictMd5AndSha256 {
                assert!(big.sha256_prefix.is_some());
            }
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn hardlinks_ignored_by_default_and_tagged_when_enabled() {
        let root = make_temp_dir("hardlinks");
        let orig = root.join("orig.dat");
        let link = root.join("link.dat");
        std::fs::write(&orig, vec![0x77u8; 2048]).unwrap();
        if std::fs::hard_link(&orig, &link).is_err() {
            // Filesystems sem suporte a hardlink: pula graciosamente.
            let _ = std::fs::remove_dir_all(&root);
            return;
        }

        // Padrão (ignore_hardlinks = true): 0 duplicatas porque é o mesmo inode.
        let mut opts_ignore = DuplicateScanOptions::for_root(root.clone());
        opts_ignore.min_size_bytes = 1;
        opts_ignore.ignore_hardlinks = true;
        let DuplicateScanEvent::Completed { groups, .. } =
            collect_scan_result(opts_ignore, None, CancellationToken::new())
        else {
            panic!("expected completed");
        };
        assert!(groups.is_empty());

        // ignore_hardlinks = false: aparece como grupo com flag `is_hardlink_sibling = true`
        // e `reclaimable_bytes() == 0`.
        let mut opts_show = DuplicateScanOptions::for_root(root.clone());
        opts_show.min_size_bytes = 1;
        opts_show.ignore_hardlinks = false;
        let DuplicateScanEvent::Completed { groups, .. } =
            collect_scan_result(opts_show, None, CancellationToken::new())
        else {
            panic!("expected completed");
        };
        assert_eq!(groups.len(), 1);
        assert!(groups[0].items.iter().all(|i| i.is_hardlink_sibling));
        assert_eq!(groups[0].reclaimable_bytes(), 0);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cache_batch_populates_and_strict_mode_bypasses_poisoned_cache() {
        let root = make_temp_dir("cache");
        let db_dir = make_temp_dir("cache_db");
        let cache = Arc::new(
            DiskMetadataCache::open_at(&db_dir.join("test.redb")).expect("open test redb"),
        );

        let a = root.join("a.bin");
        let b = root.join("b.bin");
        std::fs::write(&a, vec![0xAAu8; 40 * 1024]).unwrap();
        std::fs::write(&b, vec![0xAAu8; 40 * 1024]).unwrap();

        // Executa no nível Padrão para popular o cache em lote.
        let mut std_opts = DuplicateScanOptions::for_root(root.clone());
        std_opts.min_size_bytes = 1;
        let DuplicateScanEvent::Completed { groups, .. } =
            collect_scan_result(std_opts.clone(), Some(Arc::clone(&cache)), CancellationToken::new())
        else {
            panic!("expected completed");
        };
        assert_eq!(groups.len(), 1);

        // Envenena o cache para 'b.bin' com digest falso simulando cache obsoleto.
        let meta_b = std::fs::metadata(&b).unwrap();
        let mtime_b = meta_b
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let stamp_b = cache_integrity_stamp(&meta_b);
        cache.put_md5_batch(&[(b.as_path(), meta_b.len(), mtime_b, stamp_b, [0x00u8; 16])]);

        // Nível 3 (StrictMd5AndSha256) faz bypass completo do cache e ainda encontra a duplicata real!
        let mut strict_opts = DuplicateScanOptions::for_root(root.clone());
        strict_opts.min_size_bytes = 1;
        strict_opts.confirm_level = DuplicateConfirmLevel::StrictMd5AndSha256;
        let DuplicateScanEvent::Completed { groups, .. } =
            collect_scan_result(strict_opts, Some(Arc::clone(&cache)), CancellationToken::new())
        else {
            panic!("expected completed");
        };
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].items.len(), 2);

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&db_dir);
    }

    #[test]
    fn paranoid_partitioner_splits_colliding_bucket_into_multiple_valid_subgroups() {
        let root = make_temp_dir("partition");
        let p1 = root.join("g1_a.bin");
        let p2 = root.join("g2_a.bin");
        let p3 = root.join("g1_b.bin");
        let p4 = root.join("g2_b.bin");
        let p5 = root.join("unique.bin");

        std::fs::write(&p1, vec![0x10u8; 70 * 1024]).unwrap();
        std::fs::write(&p2, vec![0x20u8; 70 * 1024]).unwrap();
        std::fs::write(&p3, vec![0x10u8; 70 * 1024]).unwrap();
        std::fs::write(&p4, vec![0x20u8; 70 * 1024]).unwrap();
        std::fs::write(&p5, vec![0x30u8; 70 * 1024]).unwrap();

        let candidates: Vec<Candidate> = [p1, p2, p3, p4, p5]
            .into_iter()
            .map(|path| Candidate {
                path,
                root_index: 0,
                size_bytes: 70 * 1024,
                modified_nanos: None,
                modified_secs: None,
                integrity_stamp: 0,
                file_identity: None,
                is_hardlink_sibling: false,
                full_md5: None,
            })
            .collect();

        let mut buf_a = vec![0u8; STREAM_BUFFER_BYTES];
        let mut buf_b = vec![0u8; STREAM_BUFFER_BYTES];
        let cancel = CancellationToken::new();
        let subgroups = partition_bucket_byte_by_byte(
            candidates,
            &mut buf_a,
            &mut buf_b,
            &cancel,
            |_| {},
            || {},
        );

        let valid: Vec<_> = subgroups.into_iter().filter(|sg| sg.len() >= 2).collect();
        assert_eq!(valid.len(), 2);
        assert_eq!(valid[0].len(), 2);
        assert_eq!(valid[1].len(), 2);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cancellation_returns_cancelled_event() {
        let root = make_temp_dir("cancel");
        std::fs::write(root.join("a.bin"), vec![1u8; 4096]).unwrap();
        std::fs::write(root.join("b.bin"), vec![1u8; 4096]).unwrap();

        let cancel = CancellationToken::new();
        cancel.cancel();
        let opts = DuplicateScanOptions::for_root(root.clone());
        let ev = collect_scan_result(opts, None, cancel);
        assert_eq!(ev, DuplicateScanEvent::Cancelled);

        let _ = std::fs::remove_dir_all(&root);
    }
}
