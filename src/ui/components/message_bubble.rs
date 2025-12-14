//! Message bubble component

use crate::ui::theme::SignalColors;
use egui::{Color32, Rounding, Vec2};

/// Message bubble direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BubbleDirection {
    Sent,
    Received,
}

/// Message bubble component
pub struct MessageBubble {
    direction: BubbleDirection,
    max_width: f32,
}

impl MessageBubble {
    /// Create a new message bubble
    pub fn new(direction: BubbleDirection) -> Self {
        Self {
            direction,
            max_width: 400.0,
        }
    }

    /// Set maximum width
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// Show the message bubble with content
    pub fn show<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        let is_sent = self.direction == BubbleDirection::Sent;

        let bubble_color = if is_sent {
            SignalColors::BUBBLE_SENT
        } else {
            SignalColors::BUBBLE_RECEIVED
        };

        // Asymmetric rounding for chat bubble shape
        let rounding = Rounding {
            nw: if is_sent { 16.0 } else { 4.0 },
            ne: if is_sent { 4.0 } else { 16.0 },
            sw: 16.0,
            se: 16.0,
        };

        ui.horizontal(|ui| {
            if is_sent {
                ui.add_space(ui.available_width() - self.max_width - 20.0);
            } else {
                ui.add_space(12.0);
            }

            egui::Frame::none()
                .fill(bubble_color)
                .rounding(rounding)
                .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                .show(ui, |ui| {
                    ui.set_max_width(self.max_width);
                    add_contents(ui)
                })
        }).inner
    }
}

/// Quick function to show a simple text message
pub fn text_message(
    ui: &mut egui::Ui,
    text: &str,
    direction: BubbleDirection,
    timestamp: &str,
) {
    MessageBubble::new(direction).show(ui, |ui| {
        ui.label(egui::RichText::new(text).color(Color32::WHITE));

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(timestamp)
                    .size(11.0)
                    .color(Color32::from_white_alpha(180))
            );
        });
    });
}
