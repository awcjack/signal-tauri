//! Search bar widget

use crate::ui::theme::SignalColors;
use egui::{Color32, Rounding, Vec2};

/// Search bar widget
pub struct SearchBar {
    placeholder: String,
    width: Option<f32>,
}

impl SearchBar {
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
            width: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn show(self, ui: &mut egui::Ui, text: &mut String) -> egui::Response {
        let desired_width = self.width.unwrap_or(ui.available_width());

        egui::Frame::none()
            .fill(SignalColors::DARK_SURFACE)
            .rounding(Rounding::same(20.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔍").color(SignalColors::TEXT_TERTIARY));

                    let response = ui.add(
                        egui::TextEdit::singleline(text)
                            .hint_text(&self.placeholder)
                            .desired_width(desired_width - 50.0)
                            .frame(false)
                    );

                    // Clear button when text is not empty
                    if !text.is_empty() {
                        if ui.button("✕").clicked() {
                            text.clear();
                        }
                    }

                    response
                }).inner
            }).inner
    }
}
