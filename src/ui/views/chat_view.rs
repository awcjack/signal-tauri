//! Chat view - displays messages in a conversation

use crate::app::SignalApp;
use crate::signal::messages::{
    Content as StorageContent, Message as StorageMessage,
    MessageDirection as StorageDirection, MessageStatus as StorageStatus,
};
use crate::storage::conversations::ConversationRepository;
use crate::storage::messages::MessageRepository;
use crate::ui::theme::SignalColors;
use chrono::{DateTime, Local, Utc};
use egui::{Color32, Rounding, Sense, Vec2};
use crate::ui::components::emoji_text::show_emoji_text;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

static mut CACHED_CONVERSATION_ID: Option<String> = None;
static mut CACHED_CONVERSATION_NAME: String = String::new();
static mut CACHED_MESSAGES: Vec<MessageItem> = Vec::new();
static MESSAGES_DIRTY: AtomicBool = AtomicBool::new(true);

pub fn invalidate_messages_cache() {
    MESSAGES_DIRTY.store(true, Ordering::SeqCst);
}

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

impl MessageItem {
    fn from_storage(
        msg: &StorageMessage,
        my_id: Option<&str>,
        is_group: bool,
        conv_repo: &ConversationRepository,
    ) -> Self {
        let direction = match msg.direction {
            StorageDirection::Incoming => MessageDirection::Received,
            StorageDirection::Outgoing => MessageDirection::Sent,
        };

        let status = match msg.status {
            StorageStatus::Sending => MessageStatus::Sending,
            StorageStatus::Sent => MessageStatus::Sent,
            StorageStatus::Delivered => MessageStatus::Delivered,
            StorageStatus::Read => MessageStatus::Read,
            StorageStatus::Failed => MessageStatus::Failed,
        };

        let content = match &msg.content {
            StorageContent::Text { body, .. } => MessageContent::Text(body.clone()),
            StorageContent::Image { attachment_id, caption, .. } => MessageContent::Image {
                path: attachment_id.clone(),
                caption: caption.clone(),
            },
            StorageContent::Video { attachment_id, caption, .. } => MessageContent::Image {
                path: attachment_id.clone(),
                caption: caption.clone(),
            },
            StorageContent::Audio { duration_ms, .. } => MessageContent::Voice {
                duration_secs: (*duration_ms / 1000) as u32,
            },
            StorageContent::File { filename, size, .. } => MessageContent::File {
                name: filename.clone(),
                size: *size,
            },
            StorageContent::Sticker { pack_id, sticker_id, .. } => MessageContent::Sticker {
                pack_id: pack_id.clone(),
                sticker_id: sticker_id.to_string(),
            },
            StorageContent::Contact { name, .. } => MessageContent::Contact { name: name.clone() },
            StorageContent::Location { latitude, longitude, .. } => MessageContent::Location {
                lat: *latitude,
                lon: *longitude,
            },
            _ => MessageContent::Text("[Unsupported message type]".to_string()),
        };

        let mut reaction_counts: HashMap<String, (u32, bool)> = HashMap::new();
        for r in &msg.reactions {
            let entry = reaction_counts.entry(r.emoji.clone()).or_insert((0, false));
            entry.0 += 1;
            if my_id == Some(r.sender.as_str()) {
                entry.1 = true;
            }
        }
        let reactions: Vec<Reaction> = reaction_counts
            .into_iter()
            .map(|(emoji, (count, from_me))| Reaction { emoji, count, from_me })
            .collect();

        // Only show sender name for group conversations, and look up the contact name
        let sender_name = if is_group && direction == MessageDirection::Received {
            // Try to look up the sender's name from their conversation entry
            conv_repo.get(&msg.sender)
                .map(|c| c.name)
                .or_else(|| {
                    // If no conversation exists, try to extract a readable name from the sender ID
                    // The sender is typically a service ID like "<ACI:uuid>"
                    Some(msg.sender.clone())
                })
        } else {
            None
        };

        MessageItem {
            id: msg.id.clone(),
            direction,
            content,
            timestamp: msg.sent_at,
            status,
            sender_name,
            reply_to: None,
            reactions,
        }
    }
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

pub fn show(app: &mut SignalApp, ui: &mut egui::Ui) {
    let conversation_id = app.selected_conversation_id();

    if conversation_id.is_none() {
        show_empty_state(ui);
        return;
    }

    let conversation_id = conversation_id.unwrap();
    let (conversation_name, unread_count, messages) = load_conversation_data(app, conversation_id);

    // Mark read if new messages arrived while this conversation is open
    if unread_count > 0 {
        app.mark_conversation_read(conversation_id);
    }

    show_conversation_header(ui, app, conversation_id, &conversation_name);

    let available_height = ui.available_height() - 60.0;

    egui::ScrollArea::vertical()
        .max_height(available_height)
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            let mut last_date: Option<DateTime<Utc>> = None;

            for msg in &messages {
                if should_show_date_separator(&last_date, &msg.timestamp) {
                    show_date_separator(ui, &msg.timestamp);
                }
                last_date = Some(msg.timestamp);

                show_message(ui, msg);
                ui.add_space(4.0);
            }

            if messages.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label("No messages yet");
                    ui.add_space(8.0);
                    ui.label("Send a message to start the conversation");
                });
            }
        });

    ui.separator();
    show_message_input(app, ui, conversation_id);
}

fn load_conversation_data(app: &SignalApp, conversation_id: &str) -> (String, u32, Vec<MessageItem>) {
    let cached_id = unsafe { &raw mut CACHED_CONVERSATION_ID };
    let cached_id = unsafe { &mut *cached_id };
    let cached_name = unsafe { &raw mut CACHED_CONVERSATION_NAME };
    let cached_name = unsafe { &mut *cached_name };
    let cached_messages = unsafe { &raw mut CACHED_MESSAGES };
    let cached_messages = unsafe { &mut *cached_messages };

    let conversation_changed = cached_id.as_deref() != Some(conversation_id);
    let is_dirty = MESSAGES_DIRTY.load(Ordering::SeqCst);

    if !conversation_changed && !is_dirty {
        return (cached_name.clone(), 0, cached_messages.clone());
    }

    if let Some(db) = app.storage().database() {
        let conv_repo = ConversationRepository::new(&*db);
        let msg_repo = MessageRepository::new(&*db);

        let conversation = conv_repo.get(conversation_id);
        let name = conversation.as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        let unread_count = conversation.as_ref()
            .map(|c| c.unread_count)
            .unwrap_or(0);

        // Determine if this is a group conversation for sender name display
        let is_group = conversation.as_ref()
            .map(|c| c.conversation_type == crate::storage::conversations::ConversationType::Group)
            .unwrap_or(false);

        let my_id = app.storage().get_phone_number();
        let mut messages: Vec<MessageItem> = msg_repo
            .get_for_conversation(conversation_id, 100, None)
            .iter()
            .map(|m| MessageItem::from_storage(m, my_id.as_deref(), is_group, &conv_repo))
            .collect();
        messages.reverse();

        *cached_id = Some(conversation_id.to_string());
        *cached_name = name.clone();
        *cached_messages = messages.clone();
        MESSAGES_DIRTY.store(false, Ordering::SeqCst);

        (name, unread_count, messages)
    } else {
        ("Demo Conversation".to_string(), 0, get_placeholder_messages())
    }
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

fn show_conversation_header(ui: &mut egui::Ui, app: &SignalApp, conversation_id: &str, name: &str) {
    let header_height = 56.0;

    ui.horizontal(|ui| {
        ui.set_height(header_height);
        ui.add_space(8.0);

        let avatar_size = 40.0;

        // Get avatar from cache or show initials
        let avatar_color = {
            let hash: u32 = name.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32).wrapping_mul(31));
            let colors = [
                Color32::from_rgb(0x4C, 0xAF, 0x50),
                Color32::from_rgb(0x21, 0x96, 0xF3),
                Color32::from_rgb(0xFF, 0x98, 0x00),
                Color32::from_rgb(0xE9, 0x1E, 0x63),
            ];
            colors[(hash as usize) % colors.len()]
        };

        let initials: String = name
            .split_whitespace()
            .take(2)
            .filter_map(|w| w.chars().next())
            .collect::<String>()
            .to_uppercase();

        // Try to load avatar from cache
        let avatar_path = if let Some(db) = app.storage().database() {
            use crate::storage::conversations::ConversationRepository;
            let conv_repo = ConversationRepository::new(&*db);
            conv_repo.get(conversation_id).and_then(|conv| conv.avatar_path)
        } else {
            None
        };

        crate::ui::avatar_cache::draw_avatar(
            ui,
            app.avatar_cache(),
            conversation_id,
            avatar_path.as_deref(),
            &initials,
            avatar_color,
            avatar_size,
        );

        ui.add_space(12.0);

        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(name).strong().size(16.0));
            ui.label(egui::RichText::new("").size(12.0).color(SignalColors::TEXT_SECONDARY));
        });

        // Right side buttons
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);

            if ui.button("⋮").on_hover_text("More options").clicked() {
                tracing::info!("More options: not yet implemented");
            }

            if ui.button("📞").on_hover_text("Voice call").clicked() {
                tracing::info!("Voice call: not yet implemented (requires Signal protocol integration)");
            }

            if ui.button("📹").on_hover_text("Video call").clicked() {
                tracing::info!("Video call: not yet implemented (requires Signal protocol integration)");
            }

            if ui.button("🔍").on_hover_text("Search in conversation").clicked() {
                tracing::info!("Search in conversation: not yet implemented");
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

        // Calculate text width approximately (12pt font, ~7px per char)
        let approx_text_width = text.len() as f32 * 7.0;

        // Center the text, with minimum spacing of 12px
        let spacing = ((available_width - approx_text_width) / 2.0).max(12.0);
        ui.add_space(spacing);

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

    // Content-based sizing with max-width (like modern messaging apps)
    // Bubbles shrink to fit short messages, expand up to max-width for longer ones
    let max_bubble_width = 480.0; // Maximum width for message bubbles
    let frame_margin = 12.0; // Horizontal margin inside the frame
    let edge_margin = 16.0; // Margin from window edge on both sides

    // Calculate max content width (excluding frame margins)
    let max_content_width = max_bubble_width - (frame_margin * 2.0);

    let bubble_color = if is_sent {
        SignalColors::BUBBLE_SENT
    } else {
        SignalColors::BUBBLE_RECEIVED
    };

    if is_sent {
        // Pre-measure content width so we can right-align via a left spacer.
        // We use left_to_right layout (which correctly content-sizes Frames)
        // and push the bubble right with a calculated spacer.
        let body_font = egui::TextStyle::Body.resolve(ui.style());
        let content_text_width = match &msg.content {
            MessageContent::Text(text) => {
                ui.fonts(|f| f.layout(text.clone(), body_font.clone(), Color32::WHITE, max_content_width))
                    .size()
                    .x
            }
            MessageContent::Image { .. } => 200.0,
            MessageContent::File { name, .. } => {
                ui.fonts(|f| f.layout_no_wrap(name.clone(), body_font.clone(), Color32::WHITE))
                    .size()
                    .x
                    + 30.0
            }
            MessageContent::Voice { .. } => 80.0,
            _ => max_content_width,
        };

        // Measure timestamp + status width
        let time_str = msg.timestamp.with_timezone(&Local).format("%H:%M").to_string();
        let time_width = ui
            .fonts(|f| {
                f.layout_no_wrap(
                    time_str,
                    egui::FontId::proportional(10.0),
                    Color32::WHITE,
                )
            })
            .size()
            .x
            + 25.0; // status icon + spacing

        let inner_width = content_text_width.max(time_width);
        let bubble_width = (inner_width + frame_margin * 2.0 + 4.0).min(max_bubble_width);

        let available = ui.available_width();
        let spacer = (available - bubble_width - edge_margin).max(0.0);

        // left_to_right correctly content-sizes Frames (proven by received messages)
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.add_space(spacer);

            egui::Frame::none()
                .fill(bubble_color)
                .rounding(Rounding {
                    nw: 16.0,
                    ne: 4.0,
                    sw: 16.0,
                    se: 16.0,
                })
                .inner_margin(egui::Margin::symmetric(frame_margin, 8.0))
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_max_width(max_content_width);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                        // Message content
                        match &msg.content {
                            MessageContent::Text(text) => {
                                show_emoji_text(ui, text, Color32::WHITE);
                            }
                            MessageContent::Image { caption, .. } => {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::new(200.0, 150.0), Sense::click());
                                ui.painter().rect_filled(
                                    rect,
                                    Rounding::same(8.0),
                                    Color32::DARK_GRAY,
                                );
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "📷 Image",
                                    egui::FontId::proportional(14.0),
                                    Color32::WHITE,
                                );

                                if let Some(cap) = caption {
                                    show_emoji_text(ui, cap, Color32::WHITE);
                                }
                            }
                            MessageContent::File { name, size } => {
                                ui.horizontal(|ui| {
                                    ui.label("📄");
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(name).color(Color32::WHITE),
                                        );
                                        ui.label(
                                            egui::RichText::new(format_file_size(*size))
                                                .size(11.0)
                                                .color(SignalColors::TEXT_SECONDARY),
                                        );
                                    });
                                });
                            }
                            MessageContent::Voice { duration_secs } => {
                                ui.horizontal(|ui| {
                                    ui.label("🎤");
                                    ui.label(
                                        egui::RichText::new(format_duration(*duration_secs))
                                            .color(Color32::WHITE),
                                    );
                                });
                            }
                            _ => {
                                ui.label(
                                    egui::RichText::new("[Unsupported content]")
                                        .color(Color32::WHITE),
                                );
                            }
                        }

                        // Timestamp and status
                        ui.horizontal(|ui| {
                            let time_str = msg
                                .timestamp
                                .with_timezone(&Local)
                                .format("%H:%M")
                                .to_string();
                            ui.label(
                                egui::RichText::new(&time_str)
                                    .size(10.0)
                                    .color(Color32::from_rgba_unmultiplied(255, 255, 255, 180)),
                            );
                            ui.add_space(3.0);
                            let (status_icon, status_color) = match msg.status {
                                MessageStatus::Sending => ("◯", Color32::from_white_alpha(140)),
                                MessageStatus::Sent => ("✓", Color32::from_white_alpha(180)),
                                MessageStatus::Delivered => {
                                    ("✓✓", Color32::from_white_alpha(180))
                                }
                                MessageStatus::Read => ("✓✓", SignalColors::SIGNAL_BLUE),
                                MessageStatus::Failed => ("⚠", SignalColors::ERROR),
                            };
                            ui.label(
                                egui::RichText::new(status_icon)
                                    .size(10.0)
                                    .color(status_color),
                            );
                        });

                        // Reactions
                        if !msg.reactions.is_empty() {
                            ui.horizontal(|ui| {
                                for reaction in &msg.reactions {
                                    let text =
                                        format!("{} {}", reaction.emoji, reaction.count);
                                    ui.small_button(text);
                                }
                            });
                        }
                    });
                });
        });
    } else {
        // Left-align received messages
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.add_space(edge_margin);

            egui::Frame::none()
                .fill(bubble_color)
                .rounding(Rounding {
                    nw: 4.0,
                    ne: 16.0,
                    sw: 16.0,
                    se: 16.0,
                })
                .inner_margin(egui::Margin::symmetric(frame_margin, 8.0))
                .show(ui, |ui| {
                    // Force top-down layout for consistent rendering
                    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                        ui.set_max_width(max_content_width);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                        // Sender name for group messages
                        if let Some(sender) = &msg.sender_name {
                            ui.label(
                                egui::RichText::new(sender)
                                    .size(12.0)
                                    .color(SignalColors::SIGNAL_BLUE)
                                    .strong()
                            );
                        }

                        // Message content
                        match &msg.content {
                            MessageContent::Text(text) => {
                                show_emoji_text(ui, text, Color32::WHITE);
                            }
                            MessageContent::Image { caption, .. } => {
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
                                    show_emoji_text(ui, cap, Color32::WHITE);
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
                                });
                            }
                            _ => {
                                ui.label(egui::RichText::new("[Unsupported content]").color(Color32::WHITE));
                            }
                        }

                        // Timestamp
                        ui.horizontal(|ui| {
                            let time_str = msg.timestamp.with_timezone(&Local).format("%H:%M").to_string();
                            ui.label(
                                egui::RichText::new(&time_str)
                                    .size(10.0)
                                    .color(Color32::from_rgba_unmultiplied(255, 255, 255, 140))
                            );
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
        });
    }
}

fn show_message_input(app: &SignalApp, ui: &mut egui::Ui, conversation_id: &str) {
    static mut MESSAGE_INPUT: String = String::new();

    ui.horizontal(|ui| {
        ui.add_space(8.0);

        if ui.button("📎").on_hover_text("Attach file").clicked() {
            tracing::info!("Attach file: not yet implemented");
        }

        let input = unsafe { &raw mut MESSAGE_INPUT };
        let input = unsafe { &mut *input };
        let response = ui.add(
            egui::TextEdit::singleline(input)
                .hint_text("Message...")
                .desired_width(ui.available_width() - 100.0)
        );

        if ui.button("😀").on_hover_text("Emoji").clicked() {
            tracing::info!("Emoji picker: not yet implemented");
        }

        if input.is_empty() {
            if ui.button("🎤").on_hover_text("Voice message").clicked() {
                tracing::info!("Voice message: not yet implemented (requires audio recording)");
            }
        } else {
            let should_send = ui.button("➤").on_hover_text("Send").clicked() ||
               (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            
            if should_send {
                let text = input.clone();
                input.clear();
                send_message(app, conversation_id, &text);
            }
        }

        ui.add_space(8.0);
    });
}

fn send_message(app: &SignalApp, conversation_id: &str, text: &str) {
    use crate::signal::messages::{Content, Message, MessageDirection, MessageStatus};
    use crate::storage::messages::MessageRepository;
    use crate::storage::conversations::ConversationRepository;

    let Some(db) = app.storage().database() else {
        tracing::warn!("No database available, cannot send message");
        return;
    };

    let my_id = app.storage().get_phone_number().unwrap_or_else(|| "me".to_string());
    let message = Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        sender: my_id,
        direction: MessageDirection::Outgoing,
        status: MessageStatus::Sending,
        content: Content::Text {
            body: text.to_string(),
            mentions: Vec::new(),
        },
        sent_at: Utc::now(),
        server_timestamp: None,
        delivered_at: None,
        read_at: None,
        quote: None,
        reactions: Vec::new(),
        expires_in_seconds: None,
        expires_at: None,
    };

    let msg_repo = MessageRepository::new(&*db);
    if let Err(e) = msg_repo.save(&message) {
        tracing::error!("Failed to save outgoing message: {}", e);
        return;
    }

    let conv_repo = ConversationRepository::new(&*db);
    if let Some(mut conv) = conv_repo.get(conversation_id) {
        conv.update_last_message(text, message.sent_at);
        let _ = conv_repo.save(&conv);
    }
    
    invalidate_messages_cache();
    super::chat_list::invalidate_conversations_cache();

    // Get conversation type from database to determine if it's a group
    let is_group = conv_repo.get(conversation_id)
        .map(|conv| conv.conversation_type == crate::storage::conversations::ConversationType::Group)
        .unwrap_or(false);

    let storage = app.storage().clone();
    let conversation_id = conversation_id.to_string();
    let text = text.to_string();
    let text_for_log = text.clone();

    // Validate conversation ID format before attempting to send
    if is_group && !is_valid_base64_group_id(&conversation_id) {
        tracing::error!(
            "Invalid group ID format: {}. Group IDs must be 32-byte base64 encoded master keys.",
            conversation_id
        );
        return;
    }

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime for sending");

        rt.block_on(async move {
            use crate::signal::manager::SignalManager;

            if is_group {
                match SignalManager::send_group_message_static(&storage, &conversation_id, &text).await {
                    Ok(()) => tracing::info!("Group message sent"),
                    Err(e) => tracing::error!("Failed to send group message: {}", e),
                }
            } else {
                let recipient_uuid = extract_uuid_from_service_id(&conversation_id);
                match SignalManager::send_message_static(&storage, &recipient_uuid, &text).await {
                    Ok(()) => tracing::info!("Message sent to {}", recipient_uuid),
                    Err(e) => tracing::error!("Failed to send message: {}", e),
                }
            }
        });
    });

    tracing::info!("Queued message for sending: {}", text_for_log);
}

/// Validate if a conversation ID is a valid base64-encoded group ID
/// Group master keys are 32 bytes when decoded from base64
fn is_valid_base64_group_id(id: &str) -> bool {
    use base64::Engine;

    // Try to decode as base64
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(id) {
        // Group master keys should be exactly 32 bytes
        bytes.len() == 32
    } else {
        false
    }
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

fn extract_uuid_from_service_id(service_id: &str) -> String {
    if service_id.starts_with("<ACI:") && service_id.ends_with('>') {
        service_id[5..service_id.len() - 1].to_string()
    } else if service_id.starts_with("<PNI:") && service_id.ends_with('>') {
        service_id[5..service_id.len() - 1].to_string()
    } else {
        service_id.to_string()
    }
}

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
