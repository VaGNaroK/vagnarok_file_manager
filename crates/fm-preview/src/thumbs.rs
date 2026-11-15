//! Geração SIMD de miniaturas de imagem e cache LRU em memória.

use std::io::BufReader;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, Mutex};

use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::ImageReader;
use lru::LruCache;
use tokio_util::sync::CancellationToken;

use crate::{ThumbnailKey, open_shared_read};

/// Capacidade padrão do cache LRU de miniaturas em RAM.
pub const DEFAULT_THUMBNAIL_CACHE_CAPACITY: usize = 512;

/// Buffer RGBA8 pronto para envio à GPU via `iced::widget::image::Handle::from_rgba`.
#[derive(Debug, Clone)]
pub struct ThumbnailRgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Cache thread-safe LRU limitado para miniaturas decodificadas.
#[derive(Debug)]
pub struct ThumbnailCache {
    inner: Mutex<LruCache<ThumbnailKey, Arc<ThumbnailRgba>>>,
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new(DEFAULT_THUMBNAIL_CACHE_CAPACITY)
    }
}

impl ThumbnailCache {
    /// Cria um cache com no máximo `capacity` entradas (mínimo 1).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap_or(NonZeroUsize::MIN);
        Self {
            inner: Mutex::new(LruCache::new(cap)),
        }
    }

    /// Obtém uma miniatura em cache promovendo-a como mais recente.
    pub fn get(&self, key: &ThumbnailKey) -> Option<Arc<ThumbnailRgba>> {
        let mut guard = self.inner.lock().ok()?;
        guard.get(key).cloned()
    }

    /// Insere ou substitui uma miniatura no cache.
    pub fn insert(&self, key: ThumbnailKey, thumb: Arc<ThumbnailRgba>) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.put(key, thumb);
        }
    }

    /// Remove todas as entradas em cache.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.clear();
        }
    }
}

/// Retorna `true` se a extensão corresponde a um formato de imagem suportado para miniatura/QuickLook.
#[must_use]
pub fn is_image_extension(ext: Option<&str>) -> bool {
    let Some(ext) = ext else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "ico" | "tiff" | "tif" | "tga" | "pnm"
    )
}

/// Decodifica uma imagem e redimensiona via SIMD preservando a proporção até caber em `max_side_px`.
///
/// Usa compartilhamento permissivo no Windows e aborta imediatamente quando `cancel` é acionado.
pub async fn generate_thumbnail(
    path: &Path,
    max_side_px: u32,
    cancel: CancellationToken,
) -> Option<Arc<ThumbnailRgba>> {
    if cancel.is_cancelled() || max_side_px == 0 {
        return None;
    }

    let target = path.to_path_buf();
    tokio::task::spawn_blocking(move || generate_thumbnail_sync(&target, max_side_px, &cancel))
        .await
        .ok()
        .flatten()
}

fn generate_thumbnail_sync(
    path: &Path,
    max_side_px: u32,
    cancel: &CancellationToken,
) -> Option<Arc<ThumbnailRgba>> {
    if cancel.is_cancelled() {
        return None;
    }

    let file = open_shared_read(path)?;
    let reader = BufReader::new(file);
    let guessed = ImageReader::new(reader).with_guessed_format().ok()?;

    if cancel.is_cancelled() {
        return None;
    }

    let decoded = guessed.decode().ok()?;
    if cancel.is_cancelled() {
        return None;
    }

    let src_w = decoded.width();
    let src_h = decoded.height();
    if src_w == 0 || src_h == 0 {
        return None;
    }

    let rgba = decoded.into_rgba8();
    let (dst_w, dst_h) = fit_dimensions(src_w, src_h, max_side_px);

    if dst_w == src_w && dst_h == src_h {
        return Some(Arc::new(ThumbnailRgba {
            width: src_w,
            height: src_h,
            pixels: rgba.into_raw(),
        }));
    }

    let src_raw = rgba.into_raw();
    let src_view = ImageRef::new(src_w, src_h, &src_raw, PixelType::U8x4).ok()?;
    let mut dst_image = Image::new(dst_w, dst_h, PixelType::U8x4);

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Bilinear))
        .use_alpha(true);

    resizer
        .resize(&src_view, &mut dst_image, Some(&options))
        .ok()?;

    if cancel.is_cancelled() {
        return None;
    }

    Some(Arc::new(ThumbnailRgba {
        width: dst_w,
        height: dst_h,
        pixels: dst_image.into_vec(),
    }))
}

fn fit_dimensions(src_w: u32, src_h: u32, max_side_px: u32) -> (u32, u32) {
    if src_w <= max_side_px && src_h <= max_side_px {
        return (src_w, src_h);
    }
    let scale = f64::from(max_side_px) / f64::from(src_w.max(src_h));
    let dst_w = ((f64::from(src_w) * scale).round() as u32).max(1);
    let dst_h = ((f64::from(src_h) * scale).round() as u32).max(1);
    (dst_w, dst_h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_png_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("vagnarok_preview_{name}_{nanos}.png"))
    }

    #[test]
    fn resizes_png_preserving_aspect_ratio() {
        let path = temp_png_path("resize");
        let img = RgbaImage::from_pixel(200, 100, Rgba([40, 160, 220, 255]));
        img.save_with_format(&path, ImageFormat::Png)
            .expect("salvar png temporário");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let thumb = rt
            .block_on(generate_thumbnail(&path, 64, CancellationToken::new()))
            .expect("miniatura gerada");

        let _ = std::fs::remove_file(&path);
        assert_eq!((thumb.width, thumb.height), (64, 32));
        assert_eq!(thumb.pixels.len(), (64 * 32 * 4) as usize);
    }

    #[test]
    fn thumbnail_cache_evicts_lru_entries() {
        let cache = ThumbnailCache::new(2);
        let mk_key = |name: &str| ThumbnailKey {
            path: PathBuf::from(name),
            size_bytes: 10,
            modified_nanos: 1,
        };
        let thumb = Arc::new(ThumbnailRgba {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 255],
        });

        cache.insert(mk_key("a"), Arc::clone(&thumb));
        cache.insert(mk_key("b"), Arc::clone(&thumb));
        assert!(cache.get(&mk_key("a")).is_some());
        cache.insert(mk_key("c"), thumb);

        assert!(cache.get(&mk_key("b")).is_none());
        assert!(cache.get(&mk_key("a")).is_some());
        assert!(cache.get(&mk_key("c")).is_some());
    }

    #[test]
    fn respects_cancellation_and_extension_check() {
        let token = CancellationToken::new();
        token.cancel();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        assert!(rt
            .block_on(generate_thumbnail(Path::new("nonexistent.png"), 64, token))
            .is_none());
        assert!(is_image_extension(Some("PNG")));
        assert!(is_image_extension(Some("webp")));
        assert!(!is_image_extension(Some("rs")));
        assert!(!is_image_extension(None));
    }
}
