//! Cache persistente em disco baseado em `redb` (100% Rust) para miniaturas já decodificadas.

use std::path::Path;
use std::sync::Arc;

use directories::ProjectDirs;
use redb::{Database, ReadableDatabase, TableDefinition};

use crate::{ThumbnailKey, ThumbnailRgba};

const THUMBS_TABLE: TableDefinition<'_, &[u8], &[u8]> = TableDefinition::new("thumbs_v1");
const MAX_PERSISTED_SIDE_PX: u32 = 256;

/// Cache em disco seguro para concorrência via `redb`.
#[derive(Debug)]
pub struct DiskThumbnailCache {
    db: Database,
}

impl DiskThumbnailCache {
    /// Abre (ou cria) o arquivo `thumbs.redb` no diretório padrão de cache do usuário.
    pub fn open_default() -> Option<Self> {
        let dirs = ProjectDirs::from("dev", "vagnarok", "vagnarok-file-manager")?;
        let cache_dir = dirs.cache_dir();
        std::fs::create_dir_all(cache_dir).ok()?;
        Self::open_at(&cache_dir.join("thumbs.redb"))
    }

    /// Abre (ou cria) o cache persistente em um caminho específico.
    pub fn open_at(path: &Path) -> Option<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        let db = Database::create(path).ok()?;
        let write_txn = db.begin_write().ok()?;
        {
            let _ = write_txn.open_table(THUMBS_TABLE).ok()?;
        }
        write_txn.commit().ok()?;
        Some(Self { db })
    }

    /// Consulta miniatura já persistida no banco para `(key, max_side_px)`.
    pub fn get(&self, key: &ThumbnailKey, max_side_px: u32) -> Option<Arc<ThumbnailRgba>> {
        if max_side_px > MAX_PERSISTED_SIDE_PX {
            return None;
        }
        let encoded_key = encode_key(key, max_side_px);
        let read_txn = self.db.begin_read().ok()?;
        let table = read_txn.open_table(THUMBS_TABLE).ok()?;
        let guard = table.get(encoded_key.as_slice()).ok()??;
        decode_value(guard.value()).map(Arc::new)
    }

    /// Grava miniatura no banco para reutilização entre sessões.
    pub fn put(&self, key: &ThumbnailKey, max_side_px: u32, thumb: &ThumbnailRgba) {
        if max_side_px > MAX_PERSISTED_SIDE_PX
            || thumb.width > MAX_PERSISTED_SIDE_PX
            || thumb.height > MAX_PERSISTED_SIDE_PX
        {
            return;
        }
        let encoded_key = encode_key(key, max_side_px);
        let encoded_val = encode_value(thumb);

        let Ok(write_txn) = self.db.begin_write() else {
            return;
        };
        {
            let Ok(mut table) = write_txn.open_table(THUMBS_TABLE) else {
                return;
            };
            let _ = table.insert(encoded_key.as_slice(), encoded_val.as_slice());
        }
        let _ = write_txn.commit();
    }
}

fn encode_key(key: &ThumbnailKey, max_side_px: u32) -> Vec<u8> {
    let path_bytes = key.path.as_os_str().as_encoded_bytes();
    let mut out = Vec::with_capacity(4 + 8 + 16 + path_bytes.len());
    out.extend_from_slice(&max_side_px.to_le_bytes());
    out.extend_from_slice(&key.size_bytes.to_le_bytes());
    out.extend_from_slice(&key.modified_nanos.to_le_bytes());
    out.extend_from_slice(path_bytes);
    out
}

fn encode_value(thumb: &ThumbnailRgba) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + thumb.pixels.len());
    out.extend_from_slice(&thumb.width.to_le_bytes());
    out.extend_from_slice(&thumb.height.to_le_bytes());
    out.extend_from_slice(&thumb.pixels);
    out
}

fn decode_value(bytes: &[u8]) -> Option<ThumbnailRgba> {
    if bytes.len() < 8 {
        return None;
    }
    let width = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
    let height = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
    let expected_len = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    let pixels = &bytes[8..];
    if pixels.len() != expected_len || width == 0 || height == 0 {
        return None;
    }
    Some(ThumbnailRgba {
        width,
        height,
        pixels: pixels.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn persists_and_loads_thumbnail_roundtrip() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let db_path = std::env::temp_dir().join(format!("vagnarok_thumbs_{nanos}.redb"));

        let cache = DiskThumbnailCache::open_at(&db_path).expect("abrir redb");
        let key = ThumbnailKey {
            path: PathBuf::from("/tmp/sample.png"),
            size_bytes: 2048,
            modified_nanos: 999_999,
        };
        let thumb = ThumbnailRgba {
            width: 2,
            height: 2,
            pixels: vec![
                10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, 100, 110, 120, 255,
            ],
        };

        cache.put(&key, 96, &thumb);
        let loaded = cache.get(&key, 96).expect("miniatura recuperada do redb");
        assert_eq!((loaded.width, loaded.height), (2, 2));
        assert_eq!(loaded.pixels, thumb.pixels);
        assert!(cache.get(&key, 128).is_none());

        drop(cache);
        let _ = std::fs::remove_file(&db_path);
    }
}
