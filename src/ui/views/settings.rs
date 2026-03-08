//! Settings view

use crate::app::SignalApp;
use crate::storage::contacts::ContactRepository;
use crate::storage::conversations::{ConversationRepository, ConversationType};
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

/// Cached profile info to avoid DB lookups every frame
struct ProfileInfo {
    display_name: String,
    phone_number: String,
    initials: String,
}

fn load_profile_info(app: &SignalApp) -> ProfileInfo {
    let phone_number = app.storage().get_phone_number().unwrap_or_default();

    // Try to find user's display name from NoteToSelf conversation or contacts
    let display_name = if let Some(db) = app.storage().database() {
        let conv_repo = ConversationRepository::new(&*db);
        let contact_repo = ContactRepository::new(&*db);

        // First: look for NoteToSelf conversation (the user's own conversation)
        let note_to_self_name = conv_repo.list().into_iter()
            .find(|c| c.conversation_type == ConversationType::NoteToSelf)
            .and_then(|c| {
                // The NoteToSelf conv ID is the user's own UUID — try to find their contact
                contact_repo.get_by_uuid(&c.id)
                    .map(|contact| contact.display_name().to_string())
                    .filter(|name| name != &c.id && !name.starts_with("Aci("))
            });

        if let Some(name) = note_to_self_name {
            name
        } else if !phone_number.is_empty() {
            phone_number.clone()
        } else {
            String::new()
        }
    } else if !phone_number.is_empty() {
        phone_number.clone()
    } else {
        String::new()
    };

    let initials = if display_name.is_empty() {
        "?".to_string()
    } else {
        display_name
            .split_whitespace()
            .take(2)
            .filter_map(|w| w.chars().next())
            .collect::<String>()
            .to_uppercase()
    };

    ProfileInfo {
        display_name,
        phone_number,
        initials,
    }
}

/// Show the settings view
pub fn show(app: &mut SignalApp, ctx: &egui::Context) {
    let mut go_back = false;

    // Top bar with back button
    egui::TopBottomPanel::top("settings_top_bar")
        .exact_height(48.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                if ui.button("← Back").clicked() {
                    go_back = true;
                }
                ui.heading("Settings");
            });
        });

    // Left sidebar panel
    egui::SidePanel::left("settings_sidebar")
        .resizable(false)
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.add_space(8.0);
            show_settings_sidebar(ui);
        });

    // Main content area
    let profile = load_profile_info(app);
    egui::CentralPanel::default().show(ctx, |ui| {
        show_profile_settings(ui, &profile);
    });

    if go_back {
        app.navigate_to_chat_list();
    }
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

    for (icon, label, _category) in &categories {
        let button = ui.add(
            egui::Button::new(format!("{} {}", icon, label))
                .min_size(Vec2::new(180.0, 36.0))
        );
        if button.clicked() {
            // Set selected category
        }
    }
}

fn show_profile_settings(ui: &mut egui::Ui, profile: &ProfileInfo) {
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
            &profile.initials,
            egui::FontId::proportional(28.0),
            Color32::WHITE,
        );

        if response.clicked() {
            // Open avatar picker
        }

        ui.vertical(|ui| {
            ui.add_space(16.0);
            if profile.display_name.is_empty() {
                ui.label(egui::RichText::new("No profile name").size(20.0).strong().color(SignalColors::TEXT_SECONDARY));
            } else {
                ui.label(egui::RichText::new(&profile.display_name).size(20.0).strong());
            }
            if !profile.phone_number.is_empty() {
                ui.label(egui::RichText::new(&profile.phone_number).color(SignalColors::TEXT_SECONDARY));
            }
            ui.add_space(8.0);
            if ui.button("Edit Profile").clicked() {
                tracing::info!("Edit Profile: not yet implemented");
            }
        });
    });

    ui.add_space(24.0);
    ui.separator();
    ui.add_space(16.0);

    // Name (read-only display)
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if profile.display_name.is_empty() {
                ui.label(egui::RichText::new("Not set").color(SignalColors::TEXT_TERTIARY));
            } else {
                ui.label(&profile.display_name);
            }
        });
    });

    ui.add_space(12.0);

    // Phone number (read-only)
    ui.horizontal(|ui| {
        ui.label("Phone Number:");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if profile.phone_number.is_empty() {
                ui.label(egui::RichText::new("Not available").color(SignalColors::TEXT_TERTIARY));
            } else {
                ui.label(egui::RichText::new(&profile.phone_number).color(SignalColors::TEXT_SECONDARY));
            }
        });
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
