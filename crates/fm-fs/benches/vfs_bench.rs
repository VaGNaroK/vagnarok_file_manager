use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use criterion::{Criterion, criterion_group, criterion_main};
use fm_core::{SortSpec, TabId, TabState, VfsProvider};
use fm_fs::LocalFs;
use fm_index::fuzzy_rank_entries;
use tokio_util::sync::CancellationToken;

fn create_synthetic_dir(count: usize) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("vagnarok_bench_{count}_{nanos}"));
    let _ = fs::create_dir_all(&dir);
    for i in 0..count {
        let path = dir.join(format!("entry_{i:05}.rs"));
        let _ = fs::write(path, b"fn b() {}");
    }
    dir
}

fn bench_list_sort_and_fuzzy(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio bench runtime");

    let dir_2k = create_synthetic_dir(2_000);
    let fs_provider = LocalFs::new();

    c.bench_function("localfs_list_dir_2000_entries", |b| {
        b.iter(|| {
            rt.block_on(async {
                let snap = fs_provider
                    .list_dir(black_box(&dir_2k), 1, CancellationToken::new())
                    .await
                    .expect("list_dir ok");
                black_box(snap.entries.len())
            })
        });
    });

    let snap_2k = rt
        .block_on(fs_provider.list_dir(&dir_2k, 1, CancellationToken::new()))
        .expect("snapshot");

    let mut replicated = Vec::with_capacity(10_000);
    for round in 0..5 {
        for mut e in snap_2k.entries.iter().cloned() {
            e.name = format!("{round}_{}", e.name).into();
            replicated.push(e);
        }
    }

    c.bench_function("tab_recompute_sort_10000_rayon", |b| {
        let mut tab = TabState::new(TabId(1), dir_2k.clone());
        tab.sort = SortSpec::default();
        tab.raw_entries = replicated.clone();
        b.iter(|| {
            tab.recompute_visible();
            black_box(tab.visible_entries.len())
        });
    });

    c.bench_function("nucleo_fuzzy_rank_10000", |b| {
        b.iter(|| {
            let ranked = fuzzy_rank_entries(black_box(&replicated), black_box("entry_01"));
            black_box(ranked.len())
        });
    });

    let _ = fs::remove_dir_all(&dir_2k);
}

criterion_group!(benches, bench_list_sort_and_fuzzy);
criterion_main!(benches);
