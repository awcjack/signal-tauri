use crate::app::SignalApp;
use egui::{Align, Layout, RichText};

static mut PASSWORD_INPUT: String = String::new();
static mut ERROR_MESSAGE: Option<String> = None;

pub fn show(app: &mut SignalApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.add_space(100.0);
            
            ui.heading(RichText::new("🔐").size(64.0));
            ui.add_space(20.0);
            
            ui.heading("Unlock Signal");
            ui.add_space(10.0);
            ui.label("Enter your encryption password to continue");
            ui.add_space(30.0);

            let password = unsafe { &mut PASSWORD_INPUT };
            let error = unsafe { &mut ERROR_MESSAGE };

            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - 300.0) / 2.0);
                ui.add_sized(
                    [300.0, 30.0],
                    egui::TextEdit::singleline(password)
                        .password(true)
                        .hint_text("Password"),
                );
            });

            ui.add_space(20.0);

            if let Some(ref err) = *error {
                ui.colored_label(egui::Color32::RED, err);
                ui.add_space(10.0);
            }

            let unlock_clicked = ui.button("Unlock").clicked();
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

            if (unlock_clicked || enter_pressed) && !password.is_empty() {
                match app.storage().unlock_database(Some(password.as_str())) {
                    Ok(()) => {
                        password.clear();
                        *error = None;
                        app.on_database_unlocked();
                    }
                    Err(e) => {
                        *error = Some(format!("Wrong password: {}", e));
                    }
                }
            }

            ui.add_space(40.0);
            ui.separator();
            ui.add_space(10.0);
            
            if ui.small_button("Reset App (Clear All Data)").clicked() {
                if let Err(e) = app.storage().clear_all() {
                    tracing::error!("Failed to clear data: {}", e);
                }
                password.clear();
                *error = None;
                app.on_data_cleared();
            }
        });
    });
}
