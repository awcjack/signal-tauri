//! Emoji picker widget — uses swash rasterizer for system color emoji

use crate::ui::emoji_rasterizer;

/// Common emoji categories
pub const EMOJI_CATEGORIES: &[(&str, &str, &[&str])] = &[
    ("😀", "Smileys", &["😀", "😃", "😄", "😁", "😆", "😅", "🤣", "😂", "🙂", "🙃", "😉", "😊", "😇", "🥰", "😍", "🤩", "😘", "😗", "😚", "😙", "🥲", "😋", "😛", "😜", "🤪", "😝", "🤑", "🤗", "🤭", "🤫", "🤔", "🤐", "🤨", "😐", "😑", "😶", "😏", "😒", "🙄", "😬", "😮‍💨", "🤥"]),
    ("👍", "Gestures", &["👍", "👎", "👊", "✊", "🤛", "🤜", "👏", "🙌", "👐", "🤲", "🤝", "🙏", "✌️", "🤞", "🤟", "🤘", "🤙", "👈", "👉", "👆", "🖕", "👇", "☝️", "👋", "🤚", "🖐️", "✋", "🖖", "👌", "🤌", "🤏", "✍️", "🤳", "💪"]),
    ("❤️", "Hearts", &["❤️", "🧡", "💛", "💚", "💙", "💜", "🖤", "🤍", "🤎", "💔", "❣️", "💕", "💞", "💓", "💗", "💖", "💘", "💝", "💟"]),
    ("🎉", "Celebration", &["🎉", "🎊", "🎈", "🎁", "🎀", "🪅", "🪄", "🎂", "🍰", "🧁", "🥳", "🥂", "🍾", "✨", "🌟", "⭐", "🏆", "🥇", "🎖️", "🏅"]),
    ("👤", "People", &["👶", "👧", "🧒", "👦", "👩", "🧑", "👨", "👩‍🦱", "🧑‍🦱", "👨‍🦱", "👩‍🦰", "🧑‍🦰", "👨‍🦰", "👱‍♀️", "👱", "👱‍♂️", "👩‍🦳", "🧑‍🦳", "👨‍🦳", "👩‍🦲", "🧑‍🦲", "👨‍🦲", "🧔‍♀️", "🧔", "🧔‍♂️", "👵", "🧓", "👴"]),
    ("🐱", "Animals", &["🐶", "🐱", "🐭", "🐹", "🐰", "🦊", "🐻", "🐼", "🐻‍❄️", "🐨", "🐯", "🦁", "🐮", "🐷", "🐸", "🐵", "🐔", "🐧", "🐦", "🐤", "🦆", "🦅", "🦉", "🦇", "🐺", "🐗", "🐴", "🦄", "🐝", "🐛", "🦋", "🐌", "🐞"]),
    ("🍎", "Food", &["🍏", "🍎", "🍐", "🍊", "🍋", "🍌", "🍉", "🍇", "🍓", "🫐", "🍈", "🍒", "🍑", "🥭", "🍍", "🥥", "🥝", "🍅", "🍆", "🥑", "🥦", "🥬", "🥒", "🌶️", "🫑", "🌽", "🥕", "🫒", "🧄", "🧅", "🥔", "🍠"]),
    ("⚽", "Activities", &["⚽", "🏀", "🏈", "⚾", "🥎", "🎾", "🏐", "🏉", "🥏", "🎱", "🪀", "🏓", "🏸", "🏒", "🏑", "🥍", "🏏", "🪃", "🥅", "⛳", "🪁", "🏹", "🎣", "🤿", "🥊", "🥋", "🎽", "🛹", "🛼", "🛷", "⛸️", "🥌", "🎿"]),
    ("🚗", "Travel", &["🚗", "🚕", "🚙", "🚌", "🚎", "🏎️", "🚓", "🚑", "🚒", "🚐", "🛻", "🚚", "🚛", "🚜", "🛵", "🏍️", "🛺", "🚲", "🛴", "🚨", "🚔", "🚍", "🚘", "🚖", "✈️", "🛫", "🛬", "🛩️", "🚀", "🛸", "🚁", "🛶", "⛵", "🚤"]),
    ("💡", "Objects", &["⌚", "📱", "📲", "💻", "⌨️", "🖥️", "🖨️", "🖱️", "🖲️", "💽", "💾", "💿", "📀", "📼", "📷", "📸", "📹", "🎥", "📽️", "🎞️", "📞", "☎️", "📟", "📠", "📺", "📻", "🎙️", "🎚️", "🎛️", "🧭", "⏱️", "⏲️", "⏰", "🕰️", "💡", "🔦"]),
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

        ui.add_space(8.0);

        // Category tabs
        ui.horizontal(|ui| {
            for (i, (icon, name, _)) in EMOJI_CATEGORIES.iter().enumerate() {
                let selected = i == self.selected_category;
                if rasterizer_ok {
                    if let Some(texture) = emoji_rasterizer::get_emoji_texture(ui.ctx(), icon, 48.0)
                    {
                        let img =
                            egui::Image::new(egui::load::SizedTexture::from(&texture))
                                .fit_to_exact_size(egui::Vec2::splat(18.0));
                        let btn = egui::ImageButton::new(img).selected(selected);
                        if ui.add(btn).on_hover_text(*name).clicked() {
                            self.selected_category = i;
                        }
                        continue;
                    }
                }
                // Fallback: text-based tab
                if ui.selectable_label(selected, *icon).clicked() {
                    self.selected_category = i;
                }
            }
        });

        ui.separator();

        // Emoji grid
        let emojis = EMOJI_CATEGORIES[self.selected_category].2;

        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                // Tighter spacing for the emoji grid
                ui.spacing_mut().button_padding = egui::Vec2::splat(2.0);
                ui.spacing_mut().item_spacing = egui::Vec2::splat(2.0);

                ui.horizontal_wrapped(|ui| {
                    let emoji_display_size = 28.0;
                    for emoji in emojis {
                        if rasterizer_ok {
                            if let Some(texture) =
                                emoji_rasterizer::get_emoji_texture(ui.ctx(), emoji, 48.0)
                            {
                                let img =
                                    egui::Image::new(egui::load::SizedTexture::from(&texture))
                                        .fit_to_exact_size(egui::Vec2::splat(emoji_display_size));
                                if ui.add(egui::ImageButton::new(img).frame(false)).clicked() {
                                    selected_emoji = Some(emoji.to_string());
                                }
                                continue;
                            }
                        }
                        // Fallback: text button
                        if ui.button(*emoji).clicked() {
                            selected_emoji = Some(emoji.to_string());
                        }
                    }
                });
            });

        selected_emoji
    }
}
