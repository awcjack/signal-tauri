//! Chat view - displays messages in a conversation

use crate::app::SignalApp;
use crate::ui::theme::SignalColors;
use chrono::{DateTime, Local, Utc};
use egui::{Color32, Pos2, Rect, Rounding, Sense, Vec2};

/// Message direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageDirection {
    Sent,
    Received,
}

/// Message status
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageStatus {
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
}

/// A message in the conversation
#[derive(Debug, Clone)]
pub struct MessageItem {
    pub id: String,
    pub direction: MessageDirection,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub status: MessageStatus,
    pub sender_name: Option<String>, // For group messages
    pub reply_to: Option<Box<MessageItem>>,
    pub reactions: Vec<Reaction>,
}

/// Message content types
#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Image { path: String, caption: Option<String> },
    File { name: String, size: u64 },
    Voice { duration_secs: u32 },
    Sticker { pack_id: String, sticker_id: String },
    Contact { name: String },
    Location { lat: f64, lon: f64 },
}

/// A reaction to a message
#[derive(Debug, Clone)]
pub struct Reaction {
    pub emoji: String,
    pub count: u32,
    pub from_me: bool,
}

/// Current chat view state
pub struct ChatViewState {
    pub conversation_id: Option<String>,
    pub messages: Vec<MessageItem>,
    pub message_input: String,
    pub scroll_to_bottom: bool,
}

impl Default for ChatViewState {
    fn default() -> Self {
        Self {
            conversation_id: None,
            messages: get_placeholder_messages(),
            message_input: String::new(),
            scroll_to_bottom: true,
        }
    }
}

/// Show the chat view
pub fn show(app: &mut SignalApp, ui: &mut egui::Ui) {
    // Check if a conversation is selected
    let has_conversation = true; // Placeholder - would check app state

    if !has_conversation {
        show_empty_state(ui);
        return;
    }

    // Conversation header
    show_conversation_header(ui);

    // Message area
    let available_height = ui.available_height() - 60.0; // Reserve space for input

    egui::ScrollArea::vertical()
        .max_height(available_height)
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            let messages = get_placeholder_messages();
            let mut last_date: Option<DateTime<Utc>> = None;

            for msg in &messages {
                // Date separator
                if should_show_date_separator(&last_date, &msg.timestamp) {
                    show_date_separator(ui, &msg.timestamp);
                }
                last_date = Some(msg.timestamp);

                show_message(ui, msg);
                ui.add_space(4.0);
            }
        });

    // Message input area
    ui.separator();
    show_message_input(ui);
}

/// Show empty state when no conversation is selected
fn show_empty_state(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() / 3.0);

        // Signal icon
        ui.painter().circle_filled(
            ui.cursor().center() + Vec2::new(0.0, -40.0),
            40.0,
            SignalColors::SIGNAL_BLUE,
        );

        ui.add_space(60.0);
        ui.heading("Welcome to Signal");
        ui.add_space(8.0);
        ui.label("Select a conversation to start messaging");
    });
}

/// Show conversation header
fn show_conversation_header(ui: &mut egui::Ui) {
    let header_height = 56.0;

    ui.horizontal(|ui| {
        ui.set_height(header_height);
        ui.add_space(8.0);

        // Avatar
        let avatar_size = 40.0;
        let (avatar_rect, _) = ui.allocate_exact_size(Vec2::splat(avatar_size), Sense::hover());
        ui.painter().circle_filled(
            avatar_rect.center(),
            avatar_size / 2.0,
            Color32::from_rgb(0x4C, 0xAF, 0x50),
        );
        ui.painter().text(
            avatar_rect.center(),
            egui::Align2::CENTER_CENTER,
            "AS",
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );

        ui.add_space(12.0);

        // Name and status
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Alice Smith").strong().size(16.0));
            ui.label(egui::RichText::new("Online").size(12.0).color(SignalColors::TEXT_SECONDARY));
        });

        // Right side buttons
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);

            if ui.button("⋮").on_hover_text("More options").clicked() {
                // Show menu
            }

            if ui.button("📞").on_hover_text("Voice call").clicked() {
                // Start voice call
            }

            if ui.button("📹").on_hover_text("Video call").clicked() {
                // Start video call
            }

            if ui.button("🔍").on_hover_text("Search in conversation").clicked() {
                // Open search
            }
        });
    });

    ui.separator();
}

/// Check if we should show a date separator
fn should_show_date_separator(last_date: &Option<DateTime<Utc>>, current: &DateTime<Utc>) -> bool {
    match last_date {
        None => true,
        Some(last) => last.date_naive() != current.date_naive(),
    }
}

/// Show date separator
fn show_date_separator(ui: &mut egui::Ui, date: &DateTime<Utc>) {
    ui.add_space(16.0);

    ui.horizontal(|ui| {
        let available_width = ui.available_width();
        let text = format_date(date);

        ui.add_space(available_width / 2.0 - 50.0);

        ui.label(
            egui::RichText::new(text)
                .size(12.0)
                .color(SignalColors::TEXT_TERTIARY)
        );
    });

    ui.add_space(16.0);
}

/// Format date for separator
fn format_date(date: &DateTime<Utc>) -> String {
    let local: DateTime<Local> = date.with_timezone(&Local);
    let now = Local::now();

    if local.date_naive() == now.date_naive() {
        "Today".to_string()
    } else if local.date_naive() == (now - chrono::Duration::days(1)).date_naive() {
        "Yesterday".to_string()
    } else {
        local.format("%B %d, %Y").to_string()
    }
}

/// Show a single message
fn show_message(ui: &mut egui::Ui, msg: &MessageItem) {
    let is_sent = msg.direction == MessageDirection::Sent;
    let max_width = ui.available_width() * 0.7;
    let bubble_color = if is_sent {
        SignalColors::BUBBLE_SENT
    } else {
        SignalColors::BUBBLE_RECEIVED
    };

    ui.horizontal(|ui| {
        if is_sent {
            ui.add_space(ui.available_width() - max_width - 20.0);
        } else {
            ui.add_space(12.0);
        }

        // Message bubble
        egui::Frame::none()
            .fill(bubble_color)
            .rounding(Rounding {
                nw: if is_sent { 16.0 } else { 4.0 },
                ne: if is_sent { 4.0 } else { 16.0 },
                sw: 16.0,
                se: 16.0,
            })
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.set_max_width(max_width);

                // Sender name for group messages
                if let Some(sender) = &msg.sender_name {
                    if !is_sent {
                        ui.label(
                            egui::RichText::new(sender)
                                .size(12.0)
                                .color(SignalColors::SIGNAL_BLUE)
                                .strong()
                        );
                    }
                }

                // Message content
                match &msg.content {
                    MessageContent::Text(text) => {
                        ui.label(egui::RichText::new(text).color(Color32::WHITE));
                    }
                    MessageContent::Image { caption, .. } => {
                        // Placeholder for image
                        let (rect, _) = ui.allocate_exact_size(Vec2::new(200.0, 150.0), Sense::click());
                        ui.painter().rect_filled(rect, Rounding::same(8.0), Color32::DARK_GRAY);
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "📷 Image",
                            egui::FontId::proportional(14.0),
                            Color32::WHITE,
                        );

                        if let Some(cap) = caption {
                            ui.label(egui::RichText::new(cap).color(Color32::WHITE));
                        }
                    }
                    MessageContent::File { name, size } => {
                        ui.horizontal(|ui| {
                            ui.label("📄");
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(name).color(Color32::WHITE));
                                ui.label(
                                    egui::RichText::new(format_file_size(*size))
                                        .size(11.0)
                                        .color(SignalColors::TEXT_SECONDARY)
                                );
                            });
                        });
                    }
                    MessageContent::Voice { duration_secs } => {
                        ui.horizontal(|ui| {
                            ui.label("🎤");
                            ui.label(egui::RichText::new(format_duration(*duration_secs)).color(Color32::WHITE));
                            // Play button would go here
                        });
                    }
                    _ => {
                        ui.label(egui::RichText::new("[Unsupported content]").color(Color32::WHITE));
                    }
                }

                // Timestamp and status
                ui.horizontal(|ui| {
                    let time_str = msg.timestamp.with_timezone(&Local).format("%H:%M").to_string();
                    ui.label(
                        egui::RichText::new(time_str)
                            .size(11.0)
                            .color(Color32::from_white_alpha(180))
                    );

                    if is_sent {
                        let status_icon = match msg.status {
                            MessageStatus::Sending => "○",
                            MessageStatus::Sent => "✓",
                            MessageStatus::Delivered => "✓✓",
                            MessageStatus::Read => "✓✓",
                            MessageStatus::Failed => "⚠",
                        };
                        let status_color = match msg.status {
                            MessageStatus::Read => SignalColors::SIGNAL_BLUE,
                            MessageStatus::Failed => SignalColors::ERROR,
                            _ => Color32::from_white_alpha(180),
                        };
                        ui.label(egui::RichText::new(status_icon).size(11.0).color(status_color));
                    }
                });

                // Reactions
                if !msg.reactions.is_empty() {
                    ui.horizontal(|ui| {
                        for reaction in &msg.reactions {
                            let text = format!("{} {}", reaction.emoji, reaction.count);
                            ui.small_button(text);
                        }
                    });
                }
            });
    });
}

/// Show message input area
fn show_message_input(ui: &mut egui::Ui) {
    static mut MESSAGE_INPUT: String = String::new();

    ui.horizontal(|ui| {
        ui.add_space(8.0);

        // Attachment button
        if ui.button("📎").on_hover_text("Attach file").clicked() {
            // Open file picker
        }

        // Text input
        let input = unsafe { &mut MESSAGE_INPUT };
        let response = ui.add(
            egui::TextEdit::singleline(input)
                .hint_text("Message...")
                .desired_width(ui.available_width() - 100.0)
        );

        // Emoji button
        if ui.button("😀").on_hover_text("Emoji").clicked() {
            // Open emoji picker
        }

        // Send button or voice button
        if input.is_empty() {
            if ui.button("🎤").on_hover_text("Voice message").clicked() {
                // Start recording
            }
        } else {
            if ui.button("➤").on_hover_text("Send").clicked() ||
               (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                tracing::info!("Sending message: {}", input);
                input.clear();
            }
        }

        ui.add_space(8.0);
    });
}

/// Format file size for display
fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format duration for voice messages
fn format_duration(secs: u32) -> String {
    let mins = secs / 60;
    let secs = secs % 60;
    format!("{}:{:02}", mins, secs)
}

/// Get placeholder messages for UI demonstration
fn get_placeholder_messages() -> Vec<MessageItem> {
    vec![
        MessageItem {
            id: "1".to_string(),
            direction: MessageDirection::Received,
            content: MessageContent::Text("Hey! How are you doing?".to_string()),
            timestamp: Utc::now() - chrono::Duration::hours(2),
            status: MessageStatus::Read,
            sender_name: None,
            reply_to: None,
            reactions: vec![],
        },
        MessageItem {
            id: "2".to_string(),
            direction: MessageDirection::Sent,
            content: MessageContent::Text("I'm doing great! Just working on this new project.".to_string()),
            timestamp: Utc::now() - chrono::Duration::hours(1) - chrono::Duration::minutes(55),
            status: MessageStatus::Read,
            sender_name: None,
            reply_to: None,
            reactions: vec![
                Reaction { emoji: "👍".to_string(), count: 1, from_me: false },
            ],
        },
        MessageItem {
            id: "3".to_string(),
            direction: MessageDirection::Received,
            content: MessageContent::Text("That sounds interesting! What kind of project?".to_string()),
            timestamp: Utc::now() - chrono::Duration::hours(1) - chrono::Duration::minutes(50),
            status: MessageStatus::Read,
            sender_name: None,
            reply_to: None,
            reactions: vec![],
        },
        MessageItem {
            id: "4".to_string(),
            direction: MessageDirection::Sent,
            content: MessageContent::Text("A native Signal client built with Rust and egui! It's much faster and uses way less memory than Electron.".to_string()),
            timestamp: Utc::now() - chrono::Duration::hours(1) - chrono::Duration::minutes(45),
            status: MessageStatus::Read,
            sender_name: None,
            reply_to: None,
            reactions: vec![],
        },
        MessageItem {
            id: "5".to_string(),
            direction: MessageDirection::Received,
            content: MessageContent::Image {
                path: "photo.jpg".to_string(),
                caption: Some("Check out this view!".to_string()),
            },
            timestamp: Utc::now() - chrono::Duration::minutes(30),
            status: MessageStatus::Read,
            sender_name: None,
            reply_to: None,
            reactions: vec![
                Reaction { emoji: "❤️".to_string(), count: 1, from_me: true },
            ],
        },
        MessageItem {
            id: "6".to_string(),
            direction: MessageDirection::Sent,
            content: MessageContent::Text("Wow, beautiful! Where is that?".to_string()),
            timestamp: Utc::now() - chrono::Duration::minutes(25),
            status: MessageStatus::Delivered,
            sender_name: None,
            reply_to: None,
            reactions: vec![],
        },
        MessageItem {
            id: "7".to_string(),
            direction: MessageDirection::Received,
            content: MessageContent::Voice { duration_secs: 15 },
            timestamp: Utc::now() - chrono::Duration::minutes(5),
            status: MessageStatus::Read,
            sender_name: None,
            reply_to: None,
            reactions: vec![],
        },
    ]
}
