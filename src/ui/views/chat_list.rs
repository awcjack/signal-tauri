//! Chat list panel - shows all conversations

use crate::app::SignalApp;
use crate::storage::contacts::{ContactRepository, StoredContact};
use crate::storage::conversations::{Conversation, ConversationType, ConversationRepository};
use crate::ui::avatar_cache::AvatarCache;
use crate::ui::theme::SignalColors;
use chrono::{DateTime, Local, Utc};
use egui::{Color32, Rounding, Sense, Vec2};
use std::sync::atomic::{AtomicBool, Ordering};

static mut SHOW_CONTACT_PICKER: bool = false;
static mut CONTACT_SEARCH: String = String::new();
static mut CACHED_CONVERSATIONS: Vec<ConversationItem> = Vec::new();
static mut CACHED_CONTACTS: Vec<StoredContact> = Vec::new();
static CONVERSATIONS_DIRTY: AtomicBool = AtomicBool::new(true);
static CONTACTS_DIRTY: AtomicBool = AtomicBool::new(true);

pub fn invalidate_conversations_cache() {
    CONVERSATIONS_DIRTY.store(true, Ordering::SeqCst);
}

pub fn invalidate_contacts_cache() {
    CONTACTS_DIRTY.store(true, Ordering::SeqCst);
}

#[derive(Debug, Clone)]
pub struct ConversationItem {
    pub id: String,
    pub name: String,
    pub avatar_color: Color32,
    pub avatar_path: Option<String>,
    pub last_message: Option<String>,
    pub last_message_time: Option<DateTime<Utc>>,
    pub unread_count: u32,
    pub is_group: bool,
    pub is_muted: bool,
    pub is_pinned: bool,
    pub typing_indicator: bool,
}

impl ConversationItem {
    /// Get initials for avatar
    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .take(2)
            .map(|word| word.chars().next().unwrap_or('?'))
            .collect::<String>()
            .to_uppercase()
    }

    fn avatar_color_from_id(id: &str) -> Color32 {
        let hash: u32 = id.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32).wrapping_mul(31));
        let colors = [
            Color32::from_rgb(0x4C, 0xAF, 0x50),
            Color32::from_rgb(0x21, 0x96, 0xF3),
            Color32::from_rgb(0xFF, 0x98, 0x00),
            Color32::from_rgb(0xE9, 0x1E, 0x63),
            Color32::from_rgb(0x9C, 0x27, 0xB0),
            Color32::from_rgb(0x00, 0xBC, 0xD4),
            Color32::from_rgb(0xFF, 0x57, 0x22),
            Color32::from_rgb(0x60, 0x7D, 0x8B),
        ];
        colors[(hash as usize) % colors.len()]
    }
}

impl From<&Conversation> for ConversationItem {
    fn from(conv: &Conversation) -> Self {
        ConversationItem {
            id: conv.id.clone(),
            name: conv.name.clone(),
            avatar_color: ConversationItem::avatar_color_from_id(&conv.id),
            avatar_path: conv.avatar_path.clone(),
            last_message: conv.last_message.clone(),
            last_message_time: conv.last_message_at,
            unread_count: conv.unread_count,
            is_group: matches!(conv.conversation_type, ConversationType::Group),
            is_muted: conv.is_currently_muted(),
            is_pinned: conv.is_pinned,
            typing_indicator: false,
        }
    }
}

pub fn show(app: &mut SignalApp, ui: &mut egui::Ui) {
    let show_picker = unsafe { &raw mut SHOW_CONTACT_PICKER };
    let show_picker = unsafe { &mut *show_picker };
    
    ui.horizontal(|ui| {
        ui.heading("Chats");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✏").on_hover_text("New conversation").clicked() {
                *show_picker = true;
            }
        });
    });

    ui.separator();

    let mut new_selection: Option<String> = None;

    if *show_picker {
        if let Some(selected) = show_contact_picker(app, ui) {
            new_selection = Some(selected);
            *show_picker = false;
        }
    } else {
        let conversations = load_conversations(app);
        let selected_id = app.selected_conversation_id();
        let avatar_cache = app.avatar_cache();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());

                for conv in &conversations {
                    if let Some(id) = show_conversation_item(ui, conv, selected_id, avatar_cache) {
                        new_selection = Some(id);
                    }
                }

                if conversations.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(40.0);
                        ui.label("No conversations yet");
                        ui.add_space(8.0);
                        ui.label("Start a new conversation to begin messaging");
                        ui.add_space(16.0);
                        if ui.button("Start Conversation").clicked() {
                            *show_picker = true;
                        }
                    });
                }
            });
    }

    if let Some(id) = new_selection {
        app.select_conversation(Some(id));
    }
}

fn show_contact_picker(app: &mut SignalApp, ui: &mut egui::Ui) -> Option<String> {
    let search = unsafe { &raw mut CONTACT_SEARCH };
    let search = unsafe { &mut *search };
    let show_picker = unsafe { &raw mut SHOW_CONTACT_PICKER };
    let show_picker = unsafe { &mut *show_picker };
    
    let mut selected_contact_id: Option<String> = None;

    ui.horizontal(|ui| {
        if ui.button("←").on_hover_text("Back").clicked() {
            *show_picker = false;
            search.clear();
        }
        ui.heading("New Conversation");
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.add(
            egui::TextEdit::singleline(search)
                .hint_text("Search contacts...")
                .desired_width(ui.available_width() - 16.0)
        );
    });

    ui.add_space(8.0);

    let contacts = load_contacts(app, search);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            if contacts.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    if search.is_empty() {
                        ui.label("No contacts available");
                    } else {
                        ui.label("No contacts found");
                    }
                });
            }

            for contact in &contacts {
                if show_contact_item(ui, contact) {
                    selected_contact_id = Some(contact.uuid.clone());
                }
            }
        });

    if let Some(ref contact_id) = selected_contact_id {
        ensure_conversation_exists(app, contact_id);
        search.clear();
    }

    selected_contact_id
}

fn load_contacts(app: &SignalApp, search: &str) -> Vec<StoredContact> {
    let cache = unsafe { &raw mut CACHED_CONTACTS };
    let cache = unsafe { &mut *cache };
    
    if CONTACTS_DIRTY.load(Ordering::SeqCst) {
        if let Some(db) = app.storage().database() {
            let contact_repo = ContactRepository::new(&*db);
            *cache = contact_repo.list();
            CONTACTS_DIRTY.store(false, Ordering::SeqCst);
        }
    }
    
    if search.is_empty() {
        return cache.clone();
    }
    
    let search_lower = search.to_lowercase();
    cache.iter()
        .filter(|c| {
            c.display_name().to_lowercase().contains(&search_lower)
                || c.phone_number.as_ref().map(|p| p.contains(search)).unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn show_contact_item(ui: &mut egui::Ui, contact: &StoredContact) -> bool {
    let row_height = 56.0;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), row_height),
        Sense::click(),
    );

    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            Rounding::ZERO,
            SignalColors::DARK_SURFACE_ELEVATED,
        );
    }

    let avatar_size = 40.0;
    let padding = 12.0;

    let avatar_rect = egui::Rect::from_min_size(
        rect.min + Vec2::new(padding, (row_height - avatar_size) / 2.0),
        Vec2::splat(avatar_size),
    );

    let center = avatar_rect.center();
    let radius = avatar_size / 2.0;
    let avatar_color = {
        let hash: u32 = contact.uuid.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32).wrapping_mul(31));
        let colors = [
            Color32::from_rgb(0x4C, 0xAF, 0x50),
            Color32::from_rgb(0x21, 0x96, 0xF3),
            Color32::from_rgb(0xFF, 0x98, 0x00),
            Color32::from_rgb(0xE9, 0x1E, 0x63),
        ];
        colors[(hash as usize) % colors.len()]
    };

    ui.painter().circle_filled(center, radius, avatar_color);

    let initials: String = contact.display_name()
        .split_whitespace()
        .take(2)
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_uppercase();

    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        &initials,
        egui::FontId::proportional(14.0),
        Color32::WHITE,
    );

    let text_left = avatar_rect.right() + padding;
    
    ui.painter().text(
        egui::Pos2::new(text_left, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        contact.display_name(),
        egui::FontId::proportional(15.0),
        SignalColors::TEXT_PRIMARY,
    );

    if let Some(phone) = &contact.phone_number {
        ui.painter().text(
            egui::Pos2::new(text_left, rect.min.y + 32.0),
            egui::Align2::LEFT_TOP,
            phone,
            egui::FontId::proportional(12.0),
            SignalColors::TEXT_SECONDARY,
        );
    }

    response.clicked()
}

fn ensure_conversation_exists(app: &SignalApp, contact_uuid: &str) {
    if let Some(db) = app.storage().database() {
        let conv_repo = ConversationRepository::new(&*db);
        
        if conv_repo.get(contact_uuid).is_none() {
            let contact_repo = ContactRepository::new(&*db);
            let name = contact_repo
                .get_by_uuid(contact_uuid)
                .map(|c| c.display_name().to_string())
                .unwrap_or_else(|| contact_uuid.to_string());
            
            let conv = Conversation::new_private(contact_uuid, &name);
            if let Err(e) = conv_repo.save(&conv) {
                tracing::error!("Failed to create conversation: {}", e);
            }
        }
    }
}

fn load_conversations(app: &SignalApp) -> Vec<ConversationItem> {
    let cache = unsafe { &raw mut CACHED_CONVERSATIONS };
    let cache = unsafe { &mut *cache };
    
    if !CONVERSATIONS_DIRTY.load(Ordering::SeqCst) {
        return cache.clone();
    }
    
    if let Some(db) = app.storage().database() {
        let conv_repo = ConversationRepository::new(&*db);
        let contact_repo = ContactRepository::new(&*db);
        
        let conversations: Vec<ConversationItem> = conv_repo.list_active()
            .iter()
            .map(|conv| {
                let mut item = ConversationItem::from(conv);
                
                if item.avatar_path.is_none() && !item.is_group {
                    if let Some(contact) = contact_repo.get_by_uuid(&conv.id) {
                        if item.name == conv.id || item.name.starts_with("Aci(") {
                            item.name = contact.display_name().to_string();
                        }
                        item.avatar_path = contact.avatar_path.clone();
                    }
                }
                
                item
            })
            .collect();
        
        *cache = conversations.clone();
        CONVERSATIONS_DIRTY.store(false, Ordering::SeqCst);
        conversations
    } else {
        Vec::new()
    }
}

fn show_conversation_item(
    ui: &mut egui::Ui,
    conv: &ConversationItem,
    selected_id: Option<&str>,
    avatar_cache: &AvatarCache,
) -> Option<String> {
    let mut clicked_id: Option<String> = None;
    let row_height = 72.0;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), row_height),
        Sense::click(),
    );

    let is_selected = selected_id == Some(conv.id.as_str());
    
    if is_selected {
        ui.painter().rect_filled(
            rect,
            Rounding::ZERO,
            SignalColors::SIGNAL_BLUE.linear_multiply(0.3),
        );
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            Rounding::ZERO,
            SignalColors::DARK_SURFACE_ELEVATED,
        );
    }

    let avatar_size = 48.0;
    let padding = 12.0;

    let avatar_rect = egui::Rect::from_min_size(
        rect.min + Vec2::new(padding, (row_height - avatar_size) / 2.0),
        Vec2::splat(avatar_size),
    );

    let initials = conv.initials();
    let center = avatar_rect.center();
    let radius = avatar_size / 2.0;

    if let Some(texture) = avatar_cache.get_or_load(ui.ctx(), &conv.id, conv.avatar_path.as_deref()) {
        ui.painter().image(
            texture.id(),
            avatar_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.painter().circle_stroke(center, radius, egui::Stroke::new(2.0, Color32::from_gray(30)));
    } else {
        ui.painter().circle_filled(center, radius, conv.avatar_color);
        ui.painter().text(
            center,
            egui::Align2::CENTER_CENTER,
            &initials,
            egui::FontId::proportional(16.0),
            Color32::WHITE,
        );
    }

    // Text area
    let text_left = avatar_rect.right() + padding;
    let text_right = rect.right() - padding;

    // Name
    ui.painter().text(
        egui::Pos2::new(text_left, rect.min.y + 16.0),
        egui::Align2::LEFT_TOP,
        &conv.name,
        egui::FontId::proportional(15.0),
        SignalColors::TEXT_PRIMARY,
    );

    // Timestamp
    if let Some(time) = &conv.last_message_time {
        let time_str = format_time(time);
        ui.painter().text(
            egui::Pos2::new(text_right, rect.min.y + 16.0),
            egui::Align2::RIGHT_TOP,
            &time_str,
            egui::FontId::proportional(12.0),
            SignalColors::TEXT_TERTIARY,
        );
    }

    // Last message preview
    if let Some(msg) = &conv.last_message {
        let preview = if conv.typing_indicator {
            "typing...".to_string()
        } else if msg.len() > 40 {
            format!("{}...", &msg[..40])
        } else {
            msg.clone()
        };

        let preview_color = if conv.typing_indicator {
            SignalColors::SIGNAL_BLUE
        } else {
            SignalColors::TEXT_SECONDARY
        };

        ui.painter().text(
            egui::Pos2::new(text_left, rect.min.y + 38.0),
            egui::Align2::LEFT_TOP,
            &preview,
            egui::FontId::proportional(13.0),
            preview_color,
        );
    }

    // Unread badge
    if conv.unread_count > 0 {
        let badge_center = egui::Pos2::new(text_right - 12.0, rect.min.y + 44.0);
        let badge_radius = 10.0;

        ui.painter().circle_filled(
            badge_center,
            badge_radius,
            SignalColors::UNREAD,
        );

        ui.painter().text(
            badge_center,
            egui::Align2::CENTER_CENTER,
            conv.unread_count.to_string(),
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );
    }

    // Muted icon
    if conv.is_muted {
        ui.painter().text(
            egui::Pos2::new(text_right - 30.0, rect.min.y + 16.0),
            egui::Align2::RIGHT_TOP,
            "🔇",
            egui::FontId::proportional(12.0),
            SignalColors::TEXT_TERTIARY,
        );
    }

    if response.clicked() {
        tracing::info!("Selected conversation: {}", conv.name);
        clicked_id = Some(conv.id.clone());
    }

    response.context_menu(|ui| {
        if ui.button("Pin conversation").clicked() {
            ui.close_menu();
        }
        if ui.button("Mute notifications").clicked() {
            ui.close_menu();
        }
        if ui.button("Mark as read").clicked() {
            ui.close_menu();
        }
        ui.separator();
        if ui.button("Archive").clicked() {
            ui.close_menu();
        }
        if ui.button("Delete").clicked() {
            ui.close_menu();
        }
    });

    clicked_id
}

/// Format timestamp for display
fn format_time(time: &DateTime<Utc>) -> String {
    let local: DateTime<Local> = time.with_timezone(&Local);
    let now = Local::now();
    let duration = now.signed_duration_since(local);

    if duration.num_hours() < 24 {
        local.format("%H:%M").to_string()
    } else if duration.num_days() < 7 {
        local.format("%a").to_string()
    } else {
        local.format("%d/%m/%y").to_string()
    }
}
