//! Avatar texture cache with lazy loading and fallback to initials

use egui::{ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

#[derive(Default)]
pub struct AvatarCache {
    textures: RwLock<HashMap<String, TextureHandle>>,
    failed: RwLock<HashMap<String, ()>>,
}

impl AvatarCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_load(
        &self,
        ctx: &egui::Context,
        id: &str,
        avatar_path: Option<&str>,
    ) -> Option<TextureHandle> {
        let avatar_path = avatar_path?;

        if let Some(texture) = self.textures.read().ok()?.get(id) {
            return Some(texture.clone());
        }

        if self.failed.read().ok()?.contains_key(id) {
            return None;
        }

        match load_image_from_path(avatar_path) {
            Some(image) => {
                let texture = ctx.load_texture(
                    format!("avatar_{}", id),
                    image,
                    TextureOptions::LINEAR,
                );
                if let Ok(mut textures) = self.textures.write() {
                    textures.insert(id.to_string(), texture.clone());
                }
                Some(texture)
            }
            None => {
                if let Ok(mut failed) = self.failed.write() {
                    failed.insert(id.to_string(), ());
                }
                None
            }
        }
    }

    pub fn invalidate(&self, id: &str) {
        if let Ok(mut textures) = self.textures.write() {
            textures.remove(id);
        }
        if let Ok(mut failed) = self.failed.write() {
            failed.remove(id);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut textures) = self.textures.write() {
            textures.clear();
        }
        if let Ok(mut failed) = self.failed.write() {
            failed.clear();
        }
    }
}

fn load_image_from_path(path: &str) -> Option<ColorImage> {
    let path = Path::new(path);
    
    if !path.exists() {
        return None;
    }

    let data = std::fs::read(path).ok()?;
    let image = image::load_from_memory(&data).ok()?;
    let rgba = image.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let pixels = rgba.into_raw();

    Some(ColorImage::from_rgba_unmultiplied(size, &pixels))
}

pub fn draw_avatar(
    ui: &mut egui::Ui,
    cache: &AvatarCache,
    id: &str,
    avatar_path: Option<&str>,
    initials: &str,
    color: egui::Color32,
    size: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::Vec2::splat(size),
        egui::Sense::hover(),
    );

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = rect.center();
        let radius = size / 2.0;

        if let Some(texture) = cache.get_or_load(ui.ctx(), id, avatar_path) {
            painter.image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            painter.circle_stroke(center, radius, egui::Stroke::new(2.0, egui::Color32::from_gray(30)));
        } else {
            painter.circle_filled(center, radius, color);
            let font_size = size * 0.4;
            painter.text(
                center,
                egui::Align2::CENTER_CENTER,
                initials,
                egui::FontId::proportional(font_size),
                egui::Color32::WHITE,
            );
        }
    }

    response
}
