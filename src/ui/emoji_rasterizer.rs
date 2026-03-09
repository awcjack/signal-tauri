//! Swash-based color emoji rasterizer
//!
//! Uses the `swash` crate to rasterize color emoji from the system emoji font
//! (Apple Color Emoji on macOS, Segoe UI Emoji on Windows, Noto Color Emoji on Linux)
//! into RGBA bitmaps, cached as egui textures.
//!
//! This bypasses egui's built-in ab_glyph font renderer which cannot handle
//! color bitmap fonts (SBIX/CBDT/COLR formats).

use egui::{ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Lazily loaded system emoji font data
static EMOJI_FONT_DATA: OnceLock<Option<Vec<u8>>> = OnceLock::new();

/// Global texture cache: key = "emoji@size" → TextureHandle
static TEXTURE_CACHE: OnceLock<Mutex<HashMap<String, TextureHandle>>> = OnceLock::new();

fn load_emoji_font() -> Option<Vec<u8>> {
    let paths: &[&str] = &[
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/Apple Color Emoji.ttc",
        #[cfg(target_os = "windows")]
        "C:\\Windows\\Fonts\\seguiemj.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/google-noto-color-emoji/NotoColorEmoji.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/noto-color-emoji/NotoColorEmoji.ttf",
    ];

    for path in paths {
        if let Ok(data) = std::fs::read(path) {
            tracing::info!(
                "Emoji rasterizer: loaded system emoji font ({:.1} MB) from {}",
                data.len() as f64 / (1024.0 * 1024.0),
                path
            );
            return Some(data);
        }
    }
    tracing::warn!("Emoji rasterizer: no system emoji font found");
    None
}

/// Rasterize a single emoji string into an egui ColorImage using swash.
fn rasterize(font_data: &[u8], emoji: &str, size: f32) -> Option<ColorImage> {
    use swash::scale::{Render, ScaleContext, Source, StrikeWith};
    use swash::shape::ShapeContext;
    use swash::FontRef;

    // In swash 0.2, GlyphId is a type alias for u16
    let font = FontRef::from_index(font_data, 0)?;

    // Use the shaper to resolve multi-codepoint emoji sequences
    // (e.g., 👩‍🦱 = U+1F469 U+200D U+1F9B1) to a single glyph ID.
    let mut shape_ctx = ShapeContext::new();
    let mut shaper = shape_ctx.builder(font).size(size).build();
    shaper.add_str(emoji);

    let mut glyph_id: u16 = 0;
    shaper.shape_with(|cluster| {
        if glyph_id == 0 {
            if let Some(g) = cluster.glyphs.first() {
                glyph_id = g.id;
            }
        }
    });

    // Fallback: direct charmap lookup for single-character emoji
    if glyph_id == 0 {
        if let Some(ch) = emoji.chars().next() {
            let mapped = font.charmap().map(ch);
            if mapped != 0 {
                glyph_id = mapped;
            }
        }
    }

    if glyph_id == 0 {
        return None;
    }

    // Build the scaler and render pipeline
    let mut scale_ctx = ScaleContext::new();
    let mut scaler = scale_ctx.builder(font).size(size).build();

    let render = Render::new(&[
        Source::ColorBitmap(StrikeWith::BestFit),
        Source::ColorOutline(0),
        Source::Outline,
    ]);

    let image = render.render(&mut scaler, glyph_id)?;
    let w = image.placement.width as usize;
    let h = image.placement.height as usize;

    if w == 0 || h == 0 {
        return None;
    }

    // Convert to egui ColorImage.
    // SBIX bitmaps are decoded from embedded PNGs which use straight (non-premultiplied)
    // alpha, so we use from_rgba_unmultiplied to let egui handle premultiplication.
    use swash::scale::image::Content;
    match image.content {
        Content::Color => {
            Some(ColorImage::from_rgba_unmultiplied([w, h], &image.data))
        }
        Content::Mask => {
            // Alpha-only coverage mask → white with alpha
            let mut rgba = vec![0u8; w * h * 4];
            for (i, &a) in image.data.iter().enumerate() {
                rgba[i * 4] = 255;
                rgba[i * 4 + 1] = 255;
                rgba[i * 4 + 2] = 255;
                rgba[i * 4 + 3] = a;
            }
            Some(ColorImage::from_rgba_unmultiplied([w, h], &rgba))
        }
        Content::SubpixelMask => {
            // LCD subpixel mask → average to grayscale alpha
            let mut rgba = vec![0u8; w * h * 4];
            for (i, c) in image.data.chunks_exact(3).enumerate() {
                let avg = ((c[0] as u16 + c[1] as u16 + c[2] as u16) / 3) as u8;
                rgba[i * 4] = 255;
                rgba[i * 4 + 1] = 255;
                rgba[i * 4 + 2] = 255;
                rgba[i * 4 + 3] = avg;
            }
            Some(ColorImage::from_rgba_unmultiplied([w, h], &rgba))
        }
    }
}

/// Get (or rasterize and cache) an emoji texture at the given pixel size.
///
/// The caller should display the texture via `egui::Image::fit_to_exact_size()`
/// at the desired logical size. A minimum rasterization size of 32px is enforced
/// for quality.
pub fn get_emoji_texture(
    ctx: &egui::Context,
    emoji: &str,
    size: f32,
) -> Option<TextureHandle> {
    let size = size.max(32.0);
    let cache = TEXTURE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = format!("{}@{}", emoji, size as u32);

    // Return cached texture
    if let Ok(c) = cache.lock() {
        if let Some(tex) = c.get(&key) {
            return Some(tex.clone());
        }
    }

    // Load font data (lazily, once)
    let font_data = EMOJI_FONT_DATA.get_or_init(load_emoji_font);
    let font_data = font_data.as_ref()?;

    // Rasterize
    let color_image = rasterize(font_data, emoji, size)?;
    let texture = ctx.load_texture(
        format!("emoji_{}_{}", emoji, size as u32),
        color_image,
        TextureOptions::LINEAR,
    );

    // Cache
    if let Ok(mut c) = cache.lock() {
        c.insert(key, texture.clone());
    }

    Some(texture)
}

/// Returns true if the system emoji font is available.
pub fn is_available() -> bool {
    EMOJI_FONT_DATA.get_or_init(load_emoji_font).is_some()
}
