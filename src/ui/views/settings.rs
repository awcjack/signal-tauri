//! Settings view

use crate::app::SignalApp;
use crate::ui::theme::SignalColors;
use egui::{Color32, Vec2};

/// Settings categories
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsCategory {
    Profile,
    Privacy,
    Notifications,
    Appearance,
    ChatsAndMedia,
    LinkedDevices,
    Advanced,
    Help,
}

impl Default for SettingsCategory {
    fn default() -> Self {
        Self::Profile
    }
}

/// Show the settings view
pub fn show(app: &mut SignalApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            // Back button
            if ui.button("← Back").clicked() {
                // Navigate back to chat list
            }
            ui.heading("Settings");
        });

        ui.separator();

        ui.horizontal(|ui| {
            // Settings sidebar
            egui::SidePanel::left("settings_sidebar")
                .resizable(false)
                .default_width(200.0)
                .show_inside(ui, |ui| {
                    show_settings_sidebar(ui);
                });

            // Settings content
            ui.vertical(|ui| {
                show_settings_content(ui);
            });
        });
    });
}

fn show_settings_sidebar(ui: &mut egui::Ui) {
    let categories = [
        ("👤", "Profile", SettingsCategory::Profile),
        ("🔒", "Privacy", SettingsCategory::Privacy),
        ("🔔", "Notifications", SettingsCategory::Notifications),
        ("🎨", "Appearance", SettingsCategory::Appearance),
        ("💬", "Chats & Media", SettingsCategory::ChatsAndMedia),
        ("📱", "Linked Devices", SettingsCategory::LinkedDevices),
        ("⚙️", "Advanced", SettingsCategory::Advanced),
        ("❓", "Help", SettingsCategory::Help),
    ];

    ui.vertical(|ui| {
        for (icon, label, _category) in &categories {
            let button = ui.add(
                egui::Button::new(format!("{} {}", icon, label))
                    .min_size(Vec2::new(180.0, 36.0))
            );
            if button.clicked() {
                // Set selected category
            }
        }
    });
}

fn show_settings_content(ui: &mut egui::Ui) {
    // Profile settings (default view)
    show_profile_settings(ui);
}

fn show_profile_settings(ui: &mut egui::Ui) {
    ui.heading("Profile");
    ui.add_space(16.0);

    // Avatar
    ui.horizontal(|ui| {
        let avatar_size = 80.0;
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(avatar_size), egui::Sense::click());

        ui.painter().circle_filled(
            rect.center(),
            avatar_size / 2.0,
            SignalColors::SIGNAL_BLUE,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "YN",
            egui::FontId::proportional(28.0),
            Color32::WHITE,
        );

        if response.clicked() {
            // Open avatar picker
        }

        ui.vertical(|ui| {
            ui.add_space(16.0);
            ui.label(egui::RichText::new("Your Name").size(20.0).strong());
            ui.label(egui::RichText::new("+1 234 567 8900").color(SignalColors::TEXT_SECONDARY));
            ui.add_space(8.0);
            if ui.button("Edit Profile").clicked() {
                // Open profile editor
            }
        });
    });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);

    // Name
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.add_space(ui.available_width() - 250.0);
        let mut name = "Your Name".to_string();
        ui.add(egui::TextEdit::singleline(&mut name).desired_width(200.0));
    });

    ui.add_space(12.0);

    // About
    ui.horizontal(|ui| {
        ui.label("About:");
        ui.add_space(ui.available_width() - 250.0);
        let mut about = "Available".to_string();
        ui.add(egui::TextEdit::singleline(&mut about).desired_width(200.0));
    });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);

    // Phone number (read-only)
    ui.horizontal(|ui| {
        ui.label("Phone Number:");
        ui.add_space(ui.available_width() - 250.0);
        ui.label(egui::RichText::new("+1 234 567 8900").color(SignalColors::TEXT_SECONDARY));
    });
}

fn show_privacy_settings(ui: &mut egui::Ui) {
    ui.heading("Privacy");
    ui.add_space(16.0);

    // Read receipts
    let mut read_receipts = true;
    ui.checkbox(&mut read_receipts, "Read Receipts");
    ui.label(
        egui::RichText::new("If turned off, you won't be able to see read receipts from others.")
            .size(12.0)
            .color(SignalColors::TEXT_SECONDARY)
    );

    ui.add_space(16.0);

    // Typing indicators
    let mut typing_indicators = true;
    ui.checkbox(&mut typing_indicators, "Typing Indicators");
    ui.label(
        egui::RichText::new("If turned off, you won't be able to see typing indicators from others.")
            .size(12.0)
            .color(SignalColors::TEXT_SECONDARY)
    );

    ui.add_space(16.0);

    // Screen lock
    let mut screen_lock = false;
    ui.checkbox(&mut screen_lock, "Screen Lock");
    ui.label(
        egui::RichText::new("Require password or biometrics to open Signal.")
            .size(12.0)
            .color(SignalColors::TEXT_SECONDARY)
    );

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);

    ui.label(egui::RichText::new("Blocked Contacts").strong());
    ui.add_space(8.0);
    if ui.button("Manage Blocked Contacts").clicked() {
        // Open blocked contacts
    }
}

fn show_notification_settings(ui: &mut egui::Ui) {
    ui.heading("Notifications");
    ui.add_space(16.0);

    // Message notifications
    let mut message_notifications = true;
    ui.checkbox(&mut message_notifications, "Message Notifications");

    ui.add_space(12.0);

    // Notification content
    ui.label("Show:");
    let mut show_name_and_message = true;
    ui.radio_value(&mut show_name_and_message, true, "Name and Message");
    ui.radio_value(&mut show_name_and_message, false, "Name Only");

    ui.add_space(16.0);

    // Sound
    let mut notification_sound = true;
    ui.checkbox(&mut notification_sound, "Notification Sound");

    ui.add_space(16.0);

    // Call notifications
    let mut call_notifications = true;
    ui.checkbox(&mut call_notifications, "Call Notifications");
}

fn show_appearance_settings(ui: &mut egui::Ui) {
    ui.heading("Appearance");
    ui.add_space(16.0);

    // Theme
    ui.label(egui::RichText::new("Theme").strong());
    ui.add_space(8.0);

    let mut theme = 0; // 0 = Dark, 1 = Light, 2 = System
    ui.horizontal(|ui| {
        ui.selectable_value(&mut theme, 0, "Dark");
        ui.selectable_value(&mut theme, 1, "Light");
        ui.selectable_value(&mut theme, 2, "System");
    });

    ui.add_space(24.0);

    // Chat wallpaper
    ui.label(egui::RichText::new("Chat Wallpaper").strong());
    ui.add_space(8.0);
    if ui.button("Choose Wallpaper").clicked() {
        // Open wallpaper picker
    }

    ui.add_space(24.0);

    // Message font size
    ui.label(egui::RichText::new("Message Font Size").strong());
    ui.add_space(8.0);
    let mut font_size: f32 = 14.0;
    ui.add(egui::Slider::new(&mut font_size, 12.0..=20.0).text("px"));
}

fn show_linked_devices(ui: &mut egui::Ui) {
    ui.heading("Linked Devices");
    ui.add_space(16.0);

    ui.label("This device:");
    ui.add_space(8.0);

    // Current device
    egui::Frame::none()
        .fill(SignalColors::DARK_SURFACE)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("💻");
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Signal Desktop").strong());
                    ui.label(
                        egui::RichText::new("Linked: Today")
                            .size(12.0)
                            .color(SignalColors::TEXT_SECONDARY)
                    );
                });
            });
        });

    ui.add_space(24.0);

    ui.label("Other devices:");
    ui.add_space(8.0);

    // Example linked device
    egui::Frame::none()
        .fill(SignalColors::DARK_SURFACE)
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("📱");
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("iPhone").strong());
                    ui.label(
                        egui::RichText::new("Last seen: Today at 10:30 AM")
                            .size(12.0)
                            .color(SignalColors::TEXT_SECONDARY)
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Unlink").clicked() {
                        // Unlink device
                    }
                });
            });
        });

    ui.add_space(24.0);

    if ui.button("Link New Device").clicked() {
        // Show QR code for linking
    }
}
