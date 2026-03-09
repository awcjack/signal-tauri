//! Custom emoji text renderer
//!
//! Renders inline emoji using the swash-based system emoji rasterizer (Apple Color Emoji,
//! Segoe UI Emoji, Noto Color Emoji) with fallback to twemoji SVG assets.
//! Fixes the egui-twemoji ladder effect by using Align::Center in horizontal layout.

use crate::ui::emoji_rasterizer;
use egui::text::LayoutJob;
use egui::widgets::text_edit::TextEditOutput;
use egui::{Color32, ImageSource, Layout, RichText, TextFormat, Vec2};
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

/// Custom layouter for TextEdit that makes emoji characters transparent.
/// This hides the □ placeholder glyphs that ab_glyph renders for emoji,
/// allowing `overlay_emoji_on_textedit` to paint color emoji on top.
pub fn emoji_aware_layouter(ui: &egui::Ui, text: &str, wrap_width: f32) -> Arc<egui::Galley> {
    let segments = segment_text(text);

    // Fast path: no emoji
    if !segments.iter().any(|s| matches!(s, Segment::Emoji(_))) {
        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let color = ui.visuals().text_color();
        return ui.fonts(|f| {
            f.layout_job(LayoutJob::simple(text.to_owned(), font_id, color, wrap_width))
        });
    }

    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let text_color = ui.visuals().text_color();

    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap_width;
    job.text = text.to_owned();

    let mut byte_offset = 0;
    for segment in &segments {
        let s = match segment {
            Segment::Text(t) => t.as_str(),
            Segment::Emoji(e) => e.as_str(),
        };
        let byte_len = s.len();
        let format = match segment {
            Segment::Text(_) => TextFormat {
                font_id: font_id.clone(),
                color: text_color,
                ..Default::default()
            },
            Segment::Emoji(_) => TextFormat {
                font_id: font_id.clone(),
                color: Color32::TRANSPARENT,
                ..Default::default()
            },
        };
        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: byte_offset..byte_offset + byte_len,
            format,
        });
        byte_offset += byte_len;
    }

    ui.fonts(|f| f.layout_job(job))
}

/// Overlay color emoji textures on top of a TextEdit output.
/// Call this immediately after `TextEdit::show()` to paint emoji
/// at the correct positions within the text edit widget.
pub fn overlay_emoji_on_textedit(
    ui: &egui::Ui,
    output: &TextEditOutput,
    text: &str,
) {
    let segments = segment_text(text);

    // Fast path: no emoji
    if !segments.iter().any(|s| matches!(s, Segment::Emoji(_))) {
        return;
    }

    let galley = &output.galley;
    let galley_pos = output.galley_pos;
    let painter = ui.painter_at(output.text_clip_rect);

    // Build a set of byte ranges that are emoji
    let mut emoji_ranges: Vec<(usize, usize, String)> = Vec::new();
    let mut byte_offset = 0;
    for segment in &segments {
        let s = match segment {
            Segment::Text(t) => t.as_str(),
            Segment::Emoji(e) => e.as_str(),
        };
        let byte_len = s.len();
        if let Segment::Emoji(e) = segment {
            emoji_ranges.push((byte_offset, byte_offset + byte_len, e.clone()));
        }
        byte_offset += byte_len;
    }

    if emoji_ranges.is_empty() {
        return;
    }

    // Walk glyphs to find positions for each emoji
    // Glyphs are per-char, and the galley text matches our input text.
    // We track byte position by iterating chars.
    let galley_text = &galley.job.text;
    let char_byte_offsets: Vec<usize> = galley_text
        .char_indices()
        .map(|(i, _)| i)
        .collect();

    for (emoji_start, emoji_end, emoji_str) in &emoji_ranges {
        let tex = match emoji_rasterizer::get_emoji_texture(ui.ctx(), emoji_str, 48.0) {
            Some(t) => t,
            None => continue,
        };

        // Find char indices that fall within this emoji's byte range
        let first_char_idx = char_byte_offsets
            .iter()
            .position(|&b| b >= *emoji_start);
        let last_char_idx = char_byte_offsets
            .iter()
            .rposition(|&b| b >= *emoji_start && b < *emoji_end);

        let (first_char_idx, last_char_idx) = match (first_char_idx, last_char_idx) {
            (Some(f), Some(l)) => (f, l),
            _ => continue,
        };

        // Walk rows to find the glyphs and their containing row rect
        let mut glyph_idx = 0;
        for row in &galley.rows {
            let row_glyph_start = glyph_idx;
            let row_glyph_end = glyph_idx + row.glyphs.len();

            if first_char_idx >= row_glyph_start && first_char_idx < row_glyph_end {
                let first_in_row = first_char_idx - row_glyph_start;
                let last_in_row =
                    (last_char_idx - row_glyph_start).min(row.glyphs.len() - 1);

                let first_glyph = &row.glyphs[first_in_row];
                let last_glyph = &row.glyphs[last_in_row];

                let span_width =
                    (last_glyph.pos.x + last_glyph.advance_width) - first_glyph.pos.x;
                let size = first_glyph.font_height.max(span_width);

                // Use row.rect for vertical positioning (logical bounding rect of the row)
                let row_height = row.rect.height();
                let x = galley_pos.x + first_glyph.pos.x + (span_width - size) / 2.0;
                let y = galley_pos.y + row.rect.min.y + (row_height - size) / 2.0;

                let rect =
                    egui::Rect::from_min_size(egui::pos2(x, y), Vec2::splat(size));

                painter.image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(
                        egui::Pos2::ZERO,
                        egui::Pos2::new(1.0, 1.0),
                    ),
                    Color32::WHITE,
                );
                break;
            }

            glyph_idx = row_glyph_end;
            if row.ends_with_newline {
                glyph_idx += 1;
            }
        }
    }
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
