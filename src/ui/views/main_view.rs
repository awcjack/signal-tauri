//! Main view - split panel with chat list and conversation view

use crate::app::SignalApp;
use crate::ui::theme::SignalColors;
use egui::{Color32, Rounding, Vec2};

/// Show the main application view with chat list and conversation panels
pub fn show(app: &mut SignalApp, ctx: &egui::Context) {
    // Top bar with search and menu
    egui::TopBottomPanel::top("top_bar")
        .exact_height(56.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(16.0);

                // Signal logo/title
                ui.heading("Signal");

                ui.add_space(ui.available_width() - 200.0);

                // Search bar
                let mut search_text = String::new();
                ui.add(
                    egui::TextEdit::singleline(&mut search_text)
                        .hint_text("Search...")
                        .desired_width(150.0),
                );

                ui.add_space(8.0);

                // Settings button
                if ui.button("⚙").clicked() {
                    // Navigate to settings
                }

                ui.add_space(8.0);
            });
        });

    // Left panel - Chat list
    egui::SidePanel::left("chat_list_panel")
        .resizable(true)
        .default_width(300.0)
        .min_width(250.0)
        .max_width(400.0)
        .show(ctx, |ui| {
            super::chat_list::show(app, ui);
        });

    // Right panel - Chat view
    egui::CentralPanel::default().show(ctx, |ui| {
        super::chat_view::show(app, ui);
    });
}
