//! Badge component for unread counts and notifications

use crate::ui::theme::SignalColors;
use egui::{Color32, Vec2};

/// Badge type
#[derive(Clone)]
pub enum BadgeType {
    /// Numeric badge showing count
    Count(u32),
    /// Dot indicator (no number)
    Dot,
    /// Custom text
    Text(String),
}

/// Badge component
pub struct Badge {
    badge_type: BadgeType,
    color: Color32,
    text_color: Color32,
}

impl Badge {
    /// Create a count badge
    pub fn count(count: u32) -> Self {
        Self {
            badge_type: BadgeType::Count(count),
            color: SignalColors::UNREAD,
            text_color: Color32::WHITE,
        }
    }

    /// Create a dot badge
    pub fn dot() -> Self {
        Self {
            badge_type: BadgeType::Dot,
            color: SignalColors::UNREAD,
            text_color: Color32::WHITE,
        }
    }

    /// Create a text badge
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            badge_type: BadgeType::Text(text.into()),
            color: SignalColors::UNREAD,
            text_color: Color32::WHITE,
        }
    }

    /// Set badge color
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Show the badge
    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        match self.badge_type.clone() {
            BadgeType::Count(count) => self.show_count(ui, count),
            BadgeType::Dot => self.show_dot(ui),
            BadgeType::Text(text) => self.show_text(ui, &text),
        }
    }

    fn show_count(self, ui: &mut egui::Ui, count: u32) -> egui::Response {
        let text = if count > 99 {
            "99+".to_string()
        } else {
            count.to_string()
        };

        let padding = 6.0;
        let font = egui::FontId::proportional(11.0);
        let text_size = ui.painter().layout_no_wrap(text.clone(), font.clone(), self.text_color);
        let width = (text_size.rect.width() + padding * 2.0).max(20.0);
        let height = 20.0;

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            // Draw pill shape
            painter.rect_filled(rect, egui::Rounding::same(height / 2.0), self.color);

            // Draw text
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &text,
                font,
                self.text_color,
            );
        }

        response
    }

    fn show_dot(self, ui: &mut egui::Ui) -> egui::Response {
        let size = 10.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            ui.painter().circle_filled(rect.center(), size / 2.0, self.color);
        }

        response
    }

    fn show_text(self, ui: &mut egui::Ui, text: &str) -> egui::Response {
        let padding = 8.0;
        let font = egui::FontId::proportional(11.0);
        let text_size = ui.painter().layout_no_wrap(text.to_string(), font.clone(), self.text_color);
        let width = text_size.rect.width() + padding * 2.0;
        let height = 20.0;

        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();

            painter.rect_filled(rect, egui::Rounding::same(height / 2.0), self.color);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                font,
                self.text_color,
            );
        }

        response
    }
}
