//! Custom emoji text renderer
//!
//! Renders inline emoji using the swash-based system emoji rasterizer (Apple Color Emoji,
//! Segoe UI Emoji, Noto Color Emoji) with fallback to twemoji SVG assets.
//! Fixes the egui-twemoji ladder effect by using Align::Center in horizontal layout.

use crate::ui::emoji_rasterizer;
use egui::{Color32, ImageSource, Layout, RichText, Vec2};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

fn is_emoji(text: &str) -> bool {
    twemoji_assets::svg::SvgTwemojiAsset::from_emoji(text).is_some()
}

fn get_twemoji_source(emoji: &str) -> Option<ImageSource<'static>> {
    let svg_data = twemoji_assets::svg::SvgTwemojiAsset::from_emoji(emoji)?;
    Some(ImageSource::Bytes {
        uri: format!("{emoji}.svg").into(),
        bytes: egui::load::Bytes::Static(svg_data.as_bytes()),
    })
}

enum Segment {
    Text(String),
    Emoji(String),
}

fn segment_text(input: &str) -> Vec<Segment> {
    let mut result = Vec::new();
    let mut text = String::new();

    for grapheme in UnicodeSegmentation::graphemes(input, true) {
        if is_emoji(grapheme) {
            if !text.is_empty() {
                result.push(Segment::Text(text.clone()));
                text.clear();
            }
            result.push(Segment::Emoji(grapheme.to_string()));
        } else {
            text.push_str(grapheme);
        }
    }

    if !text.is_empty() {
        result.push(Segment::Text(text));
    }

    result
}

/// Paint emoji-aware text at a specific position using the painter.
/// Drop-in replacement for `ui.painter().text()` with color emoji support.
pub fn paint_emoji_text(
    ui: &egui::Ui,
    pos: egui::Pos2,
    anchor: egui::Align2,
    text: &str,
    font_id: egui::FontId,
    color: Color32,
) -> egui::Rect {
    let segments = segment_text(text);
    let painter = ui.painter();

    // Fast path: no emoji, just use painter.text()
    if !segments.iter().any(|s| matches!(s, Segment::Emoji(_))) {
        return painter.text(pos, anchor, text, font_id, color);
    }

    let emoji_size = font_id.size;

    enum Item {
        Galley(Arc<egui::Galley>, f32, f32),
        Emoji(egui::TextureHandle, f32),
    }

    let mut items = Vec::new();
    let mut total_width = 0.0f32;
    let mut max_height = 0.0f32;

    for segment in segments {
        match segment {
            Segment::Text(t) => {
                let galley = ui.fonts(|f| f.layout_no_wrap(t, font_id.clone(), color));
                let s = galley.size();
                total_width += s.x;
                max_height = max_height.max(s.y);
                items.push(Item::Galley(galley, s.x, s.y));
            }
            Segment::Emoji(emoji) => {
                if let Some(tex) = emoji_rasterizer::get_emoji_texture(ui.ctx(), &emoji, 48.0) {
                    total_width += emoji_size;
                    max_height = max_height.max(emoji_size);
                    items.push(Item::Emoji(tex, emoji_size));
                } else {
                    let galley = ui.fonts(|f| f.layout_no_wrap(emoji, font_id.clone(), color));
                    let s = galley.size();
                    total_width += s.x;
                    max_height = max_height.max(s.y);
                    items.push(Item::Galley(galley, s.x, s.y));
                }
            }
        }
    }

    let rect = anchor.anchor_size(pos, Vec2::new(total_width, max_height));
    let mut x = rect.min.x;

    for item in items {
        match item {
            Item::Galley(galley, w, h) => {
                let y = rect.min.y + (max_height - h) / 2.0;
                painter.galley(egui::Pos2::new(x, y), galley, color);
                x += w;
            }
            Item::Emoji(tex, size) => {
                let y = rect.min.y + (max_height - size) / 2.0;
                painter.image(
                    tex.id(),
                    egui::Rect::from_min_size(egui::Pos2::new(x, y), Vec2::splat(size)),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                x += size;
            }
        }
    }

    rect
}

/// Show emoji text with custom font size and optional bold styling.
pub fn show_emoji_text_styled(
    ui: &mut egui::Ui,
    text: &str,
    size: f32,
    color: Color32,
    strong: bool,
) {
    let segments = segment_text(text);

    ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        for segment in segments {
            match segment {
                Segment::Text(t) => {
                    let mut rt = RichText::new(t).color(color).size(size);
                    if strong {
                        rt = rt.strong();
                    }
                    ui.label(rt);
                }
                Segment::Emoji(emoji) => {
                    if let Some(texture) =
                        emoji_rasterizer::get_emoji_texture(ui.ctx(), &emoji, 48.0)
                    {
                        ui.add(
                            egui::Image::new(egui::load::SizedTexture::from(&texture))
                                .fit_to_exact_size(Vec2::splat(size)),
                        );
                    } else if let Some(source) = get_twemoji_source(&emoji) {
                        ui.add(
                            egui::Image::new(source).fit_to_exact_size(Vec2::splat(size)),
                        );
                    } else {
                        ui.label(RichText::new(&emoji).color(color).size(size));
                    }
                }
            }
        }
    });
}

pub fn show_emoji_text(ui: &mut egui::Ui, text: &str, color: Color32) {
    let segments = segment_text(text);
    let font_height = ui.text_style_height(&egui::TextStyle::Body);

    ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;

        for segment in segments {
            match segment {
                Segment::Text(t) => {
                    ui.label(RichText::new(t).color(color));
                }
                Segment::Emoji(emoji) => {
                    // Try system emoji via swash rasterizer first
                    if let Some(texture) =
                        emoji_rasterizer::get_emoji_texture(ui.ctx(), &emoji, 48.0)
                    {
                        ui.add(
                            egui::Image::new(egui::load::SizedTexture::from(&texture))
                                .fit_to_exact_size(Vec2::splat(font_height)),
                        );
                    } else if let Some(source) = get_twemoji_source(&emoji) {
                        // Fallback to twemoji SVG
                        ui.add(
                            egui::Image::new(source).fit_to_exact_size(Vec2::splat(font_height)),
                        );
                    } else {
                        ui.label(RichText::new(&emoji).color(color));
                    }
                }
            }
        }
    });
}
