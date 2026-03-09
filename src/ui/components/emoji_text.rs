//! Custom emoji text renderer
//!
//! Renders inline emoji using the swash-based system emoji rasterizer (Apple Color Emoji,
//! Segoe UI Emoji, Noto Color Emoji) with fallback to twemoji SVG assets.
//! Fixes the egui-twemoji ladder effect by using Align::Center in horizontal layout.

use crate::ui::emoji_rasterizer;
use egui::{Color32, ImageSource, Layout, RichText, Vec2};
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
