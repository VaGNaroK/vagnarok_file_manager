//! Cache persistente em `redb` para metadados computados (SHA-256, MIME e contagem rasa de pastas).

use std::path::Path;
use std::time::UNIX_EPOCH;

use compact_str::CompactString;
use directories::ProjectDirs;
use fm_core::RecursiveSize;
use redb::{Database, ReadableDatabase, TableDefinition};

const FILE_META_TABLE: TableDefinition<'_, &[u8], &str> = TableDefinition::new("file_meta_v1");
const DIR_COUNT_TABLE: TableDefinition<'_, &[u8], u32> = TableDefinition::new("dir_count_v1");
const DIR_SIZE_TABLE: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new("dir_size_v1");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedFileTelemetry {
    pub mime: Option<CompactString>,
    pub sha_prefix: Option<CompactString>,
    pub sha_full: Option<CompactString>,
}

#[derive(Debug)]
pub struct DiskMetadataCache {
    db: Database,
}

impl DiskMetadataCache {
    pub fn open_default() -> Option<Self> {
        let dirs = ProjectDirs::from("dev", "vagnarok", "vagnarok-file-manager")?;
        let cache_dir = dirs.cache_dir();
        std::fs::create_dir_all(cache_dir).ok()?;
        Self::open_at(&cache_dir.join("telemetry_v1.redb"))
    }

    pub fn open_at(path: &Path) -> Option<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        let db = Database::create(path).ok()?;
        let write_txn = db.begin_write().ok()?;
        {
            let _ = write_txn.open_table(FILE_META_TABLE).ok()?;
            let _ = write_txn.open_table(DIR_COUNT_TABLE).ok()?;
            let _ = write_txn.open_table(DIR_SIZE_TABLE).ok()?;
        }
        write_txn.commit().ok()?;
        Some(Self { db })
    }

    pub fn get_file(
        &self,
        path: &Path,
        size_bytes: u64,
        modified_nanos: u128,
    ) -> Option<CachedFileTelemetry> {
        let key = encode_file_key(path, size_bytes, modified_nanos);
        let read_txn = self.db.begin_read().ok()?;
        let table = read_txn.open_table(FILE_META_TABLE).ok()?;
        let guard = table.get(key.as_slice()).ok()??;
        let mut parts = guard.value().split('\n');
        let mime = parts
            .next()
            .filter(|s| !s.is_empty())
            .map(CompactString::from);
        let sha_prefix = parts
            .next()
            .filter(|s| !s.is_empty())
            .map(CompactString::from);
        let sha_full = parts
            .next()
            .filter(|s| !s.is_empty())
            .map(CompactString::from);
        Some(CachedFileTelemetry {
            mime,
            sha_prefix,
            sha_full,
        })
    }

    pub fn put_file(
        &self,
        path: &Path,
        size_bytes: u64,
        modified_nanos: u128,
        val: &CachedFileTelemetry,
    ) {
        let key = encode_file_key(path, size_bytes, modified_nanos);
        let payload = format!(
            "{}\n{}\n{}",
            val.mime.as_deref().unwrap_or(""),
            val.sha_prefix.as_deref().unwrap_or(""),
            val.sha_full.as_deref().unwrap_or(""),
        );
        let Ok(write_txn) = self.db.begin_write() else {
            return;
        };
        if let Ok(mut table) = write_txn.open_table(FILE_META_TABLE) {
            let _ = table.insert(key.as_slice(), payload.as_str());
        }
        let _ = write_txn.commit();
    }

    pub fn get_dir_count(&self, path: &Path, modified_nanos: u128) -> Option<u32> {
        let key = encode_file_key(path, 0, modified_nanos);
        let read_txn = self.db.begin_read().ok()?;
        let table = read_txn.open_table(DIR_COUNT_TABLE).ok()?;
        table.get(key.as_slice()).ok()?.map(|g| g.value())
    }

    pub fn put_dir_count(&self, path: &Path, modified_nanos: u128, count: u32) {
        let key = encode_file_key(path, 0, modified_nanos);
        let Ok(write_txn) = self.db.begin_write() else {
            return;
        };
        if let Ok(mut table) = write_txn.open_table(DIR_COUNT_TABLE) {
            let _ = table.insert(key.as_slice(), count);
        }
        let _ = write_txn.commit();
    }

    pub fn get_dir_size(&self, path: &Path, modified_nanos: u128) -> Option<RecursiveSize> {
        let key = encode_file_key(path, 1, modified_nanos);
        let read_txn = self.db.begin_read().ok()?;
        let table = read_txn.open_table(DIR_SIZE_TABLE).ok()?;
        let guard = table.get(key.as_slice()).ok()??;
        let bytes = guard.value();
        if bytes.len() != 25 {
            return None;
        }
        let total_bytes = u64::from_le_bytes(bytes[0..8].try_into().ok()?);
        let item_count = u64::from_le_bytes(bytes[8..16].try_into().ok()?);
        let hidden_count = u64::from_le_bytes(bytes[16..24].try_into().ok()?);
        let truncated = bytes[24] != 0;
        Some(RecursiveSize {
            total_bytes,
            item_count,
            hidden_count,
            truncated,
        })
    }

    pub fn put_dir_size(&self, path: &Path, modified_nanos: u128, val: RecursiveSize) {
        let key = encode_file_key(path, 1, modified_nanos);
        let mut payload = [0u8; 25];
        payload[0..8].copy_from_slice(&val.total_bytes.to_le_bytes());
        payload[8..16].copy_from_slice(&val.item_count.to_le_bytes());
        payload[16..24].copy_from_slice(&val.hidden_count.to_le_bytes());
        payload[24] = u8::from(val.truncated);
        let Ok(write_txn) = self.db.begin_write() else {
            return;
        };
        if let Ok(mut table) = write_txn.open_table(DIR_SIZE_TABLE) {
            let _ = table.insert(key.as_slice(), payload.as_slice());
        }
        let _ = write_txn.commit();
    }
}

pub fn mtime_nanos(path: &Path) -> u128 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn encode_file_key(path: &Path, size_bytes: u64, modified_nanos: u128) -> Vec<u8> {
    let p = path.to_string_lossy();
    let mut out = Vec::with_capacity(p.len() + 24);
    out.extend_from_slice(p.as_bytes());
    out.extend_from_slice(&size_bytes.to_le_bytes());
    out.extend_from_slice(&modified_nanos.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn persists_and_reads_file_and_dir_telemetry_roundtrip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("vagnarok_meta_cache_{nanos}"));
        let db_path = dir.join("telemetry.redb");

        let cache = DiskMetadataCache::open_at(&db_path).expect("open telemetry cache");
        let file = Path::new("/tmp/sample.rs");
        let meta = CachedFileTelemetry {
            mime: Some(CompactString::new("text/rust")),
            sha_prefix: Some(CompactString::new("abcdef12")),
            sha_full: Some(CompactString::new("abcdef1234567890")),
        };
        cache.put_file(file, 42, 999_000, &meta);
        assert_eq!(cache.get_file(file, 42, 999_000), Some(meta));
        assert!(cache.get_file(file, 43, 999_000).is_none());

        let folder = Path::new("/tmp/project");
        cache.put_dir_count(folder, 123_456, 17);
        assert_eq!(cache.get_dir_count(folder, 123_456), Some(17));

        let rsize = RecursiveSize {
            total_bytes: 987_654_321,
            item_count: 142,
            hidden_count: 7,
            truncated: true,
        };
        cache.put_dir_size(folder, 123_456, rsize);
        assert_eq!(cache.get_dir_size(folder, 123_456), Some(rsize));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
