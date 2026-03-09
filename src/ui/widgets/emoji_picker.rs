//! Emoji picker widget — full Unicode emoji set via `emojis` crate,
//! rendered with swash system color emoji rasterizer.

use crate::ui::emoji_rasterizer;
use emojis::Group;

/// Category definition: (representative emoji, display name, Group variant)
const CATEGORIES: &[(&str, &str, Group)] = &[
    ("😀", "Smileys & Emotion", Group::SmileysAndEmotion),
    ("👋", "People & Body", Group::PeopleAndBody),
    ("🐱", "Animals & Nature", Group::AnimalsAndNature),
    ("🍎", "Food & Drink", Group::FoodAndDrink),
    ("🚗", "Travel & Places", Group::TravelAndPlaces),
    ("⚽", "Activities", Group::Activities),
    ("💡", "Objects", Group::Objects),
    ("🔣", "Symbols", Group::Symbols),
    ("🏁", "Flags", Group::Flags),
];

/// Emoji picker widget
pub struct EmojiPicker {
    selected_category: usize,
    search_query: String,
}

impl Default for EmojiPicker {
    fn default() -> Self {
        Self {
            selected_category: 0,
            search_query: String::new(),
        }
    }
}

impl EmojiPicker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the emoji picker and return selected emoji if any
    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let mut selected_emoji = None;
        let rasterizer_ok = emoji_rasterizer::is_available();

        // Search bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);
        });

        ui.add_space(4.0);

        // Category tabs
        ui.horizontal(|ui| {
            for (i, (icon, name, _)) in CATEGORIES.iter().enumerate() {
                let selected = i == self.selected_category;
                if rasterizer_ok {
                    if let Some(texture) =
                        emoji_rasterizer::get_emoji_texture(ui.ctx(), icon, 48.0)
                    {
                        let img = egui::Image::new(egui::load::SizedTexture::from(&texture))
                            .fit_to_exact_size(egui::Vec2::splat(18.0));
                        let btn = egui::ImageButton::new(img).selected(selected);
                        if ui.add(btn).on_hover_text(*name).clicked() {
                            self.selected_category = i;
                        }
                        continue;
                    }
                }
                if ui.selectable_label(selected, *icon).clicked() {
                    self.selected_category = i;
                }
            }
        });

        ui.separator();

        // Build emoji list — either search results or current category
        let searching = !self.search_query.is_empty();
        let query_lower = self.search_query.to_lowercase();

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                ui.spacing_mut().button_padding = egui::Vec2::splat(2.0);
                ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);

                ui.horizontal_wrapped(|ui| {
                    let emoji_display_size = 28.0;

                    let emojis_iter: Box<dyn Iterator<Item = &emojis::Emoji>> = if searching {
                        // Search across all emoji by name/shortcode
                        Box::new(emojis::iter().filter(|e| {
                            e.name().to_lowercase().contains(&query_lower)
                                || e.shortcode()
                                    .map_or(false, |sc| sc.to_lowercase().contains(&query_lower))
                        }))
                    } else {
                        let group = CATEGORIES[self.selected_category].2;
                        Box::new(group.emojis())
                    };

                    for emoji in emojis_iter {
                        // Skip skin tone variants to keep the grid clean
                        if emoji.skin_tone().is_some() {
                            continue;
                        }

                        let emoji_str = emoji.as_str();
                        if rasterizer_ok {
                            if let Some(texture) =
                                emoji_rasterizer::get_emoji_texture(ui.ctx(), emoji_str, 48.0)
                            {
                                let img =
                                    egui::Image::new(egui::load::SizedTexture::from(&texture))
                                        .fit_to_exact_size(egui::Vec2::splat(emoji_display_size));
                                if ui
                                    .add(egui::ImageButton::new(img).frame(false))
                                    .on_hover_text(emoji.name())
                                    .clicked()
                                {
                                    selected_emoji = Some(emoji_str.to_string());
                                }
                                continue;
                            }
                        }
                        if ui
                            .button(emoji_str)
                            .on_hover_text(emoji.name())
                            .clicked()
                        {
                            selected_emoji = Some(emoji_str.to_string());
                        }
                    }
                });
            });

        selected_emoji
    }
}
