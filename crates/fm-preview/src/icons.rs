//! Rasterização e cache LRU em memória para ícones do sistema hospedeiro.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use fm_core::{SystemIconBucket, SystemIconKey, SystemIconPayload};
use lru::LruCache;
use tokio_util::sync::CancellationToken;

use crate::ThumbnailRgba;
use crate::thumbs::generate_thumbnail_sync;

pub const DEFAULT_ICON_CACHE_CAPACITY: usize = 128;

type IconCacheKey = (SystemIconKey, SystemIconBucket);
type CachedIconEntry = Option<Arc<ThumbnailRgba>>;

/// Cache LRU em memória indexado por `(SystemIconKey, SystemIconBucket)` incluindo hits e misses.
#[derive(Debug)]
pub struct SystemIconCache {
    inner: Mutex<LruCache<IconCacheKey, CachedIconEntry>>,
}

impl Default for SystemIconCache {
    fn default() -> Self {
        Self::new(DEFAULT_ICON_CACHE_CAPACITY)
    }
}

impl SystemIconCache {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap_or(NonZeroUsize::MIN);
        Self {
            inner: Mutex::new(LruCache::new(cap)),
        }
    }

    pub fn get(
        &self,
        key: &(SystemIconKey, SystemIconBucket),
    ) -> Option<Option<Arc<ThumbnailRgba>>> {
        let mut guard = self.inner.lock().ok()?;
        guard.get(key).cloned()
    }

    pub fn insert(&self, key: (SystemIconKey, SystemIconBucket), rgba: Option<Arc<ThumbnailRgba>>) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.put(key, rgba);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.clear();
        }
    }
}

/// Converte um payload de ícone nativo (`File` SVG/PNG ou `Rgba8`) em `ThumbnailRgba` pronto para GPU.
pub fn rasterize_system_icon(
    payload: SystemIconPayload,
    bucket: SystemIconBucket,
    cancel: &CancellationToken,
) -> Option<Arc<ThumbnailRgba>> {
    if cancel.is_cancelled() {
        return None;
    }
    match payload {
        SystemIconPayload::File(path) => generate_thumbnail_sync(&path, bucket.target_px(), cancel),
        SystemIconPayload::Rgba8 {
            width,
            height,
            pixels,
        } => {
            if width == 0 || height == 0 || pixels.len() != (width * height * 4) as usize {
                return None;
            }
            Some(Arc::new(ThumbnailRgba {
                width,
                height,
                pixels,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caches_and_rasterizes_rgba8_system_icon_payload() {
        let cancel = CancellationToken::new();
        let payload = SystemIconPayload::Rgba8 {
            width: 2,
            height: 2,
            pixels: vec![255; 16],
        };
        let rgba =
            rasterize_system_icon(payload, SystemIconBucket::Small24, &cancel).expect("rgba icon");
        assert_eq!(rgba.width, 2);
        assert_eq!(rgba.height, 2);

        let cache = SystemIconCache::default();
        let key = (SystemIconKey::Directory, SystemIconBucket::Small24);
        cache.insert(key.clone(), Some(Arc::clone(&rgba)));
        assert!(cache.get(&key).flatten().is_some());
    }
}
