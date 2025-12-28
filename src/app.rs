//! Main application state and logic

use crate::signal::manager::{IncomingMessage, MessageContent};
use crate::signal::messages::{Content, Message, MessageDirection, MessageStatus};
use crate::signal::{ConnectionState as SignalConnectionState, SignalEvent, SignalManager};
use crate::storage::conversations::{Conversation, ConversationRepository};
use crate::storage::messages::MessageRepository;
use crate::storage::Storage;
use crate::ui::{theme::SignalTheme, views::ViewState};
use chrono::{TimeZone, Utc};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

/// State for the device linking process
#[derive(Clone)]
pub enum LinkingState {
    /// Not started
    NotStarted,
    /// Generating provisioning URL
    Generating,
    /// Waiting for phone to scan QR code
    WaitingForScan {
        provisioning_url: String,
        qr_texture: Option<egui::TextureHandle>,
    },
    /// Processing link after scan
    Processing,
    /// Successfully linked
    Success,
    /// Error occurred
    Error(String),
}

impl std::fmt::Debug for LinkingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "NotStarted"),
            Self::Generating => write!(f, "Generating"),
            Self::WaitingForScan { provisioning_url, .. } => {
                f.debug_struct("WaitingForScan")
                    .field("provisioning_url", provisioning_url)
                    .field("qr_texture", &"<texture>")
                    .finish()
            }
            Self::Processing => write!(f, "Processing"),
            Self::Success => write!(f, "Success"),
            Self::Error(e) => f.debug_tuple("Error").field(e).finish(),
        }
    }
}

impl Default for LinkingState {
    fn default() -> Self {
        Self::NotStarted
    }
}

pub struct SignalApp {
    runtime: Arc<Runtime>,
    signal_manager: Arc<RwLock<Option<SignalManager>>>,
    storage: Arc<Storage>,
    view_state: ViewState,
    theme: SignalTheme,
    connection_status: ConnectionStatus,
    error_message: Option<String>,
    initialized: bool,
    linking_state: LinkingState,
    event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<SignalEvent>>>>,
    event_tx: mpsc::UnboundedSender<SignalEvent>,
    selected_conversation_id: Option<String>,
}

/// Connection status to Signal servers
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error(String),
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// Get a default device name based on hostname
fn get_device_name() -> String {
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "Desktop".to_string());

    format!("Signal Desktop ({})", hostname)
}

fn incoming_to_message(incoming: &IncomingMessage) -> Message {
    let content = match &incoming.content {
        MessageContent::Text(text) => Content::Text {
            body: text.clone(),
            mentions: Vec::new(),
        },
        MessageContent::Attachment { content_type, filename, size, attachment_id } => {
            if content_type.starts_with("image/") {
                Content::Image {
                    attachment_id: attachment_id.clone(),
                    content_type: content_type.clone(),
                    width: 0,
                    height: 0,
                    size: *size,
                    caption: None,
                    blurhash: None,
                }
            } else if content_type.starts_with("video/") {
                Content::Video {
                    attachment_id: attachment_id.clone(),
                    content_type: content_type.clone(),
                    width: 0,
                    height: 0,
                    duration_ms: 0,
                    size: *size,
                    caption: None,
                    thumbnail_id: None,
                }
            } else if content_type.starts_with("audio/") {
                Content::Audio {
                    attachment_id: attachment_id.clone(),
                    content_type: content_type.clone(),
                    duration_ms: 0,
                    size: *size,
                    waveform: None,
                }
            } else {
                Content::File {
                    attachment_id: attachment_id.clone(),
                    content_type: content_type.clone(),
                    filename: filename.clone().unwrap_or_else(|| "attachment".to_string()),
                    size: *size,
                }
            }
        }
        MessageContent::Sticker { pack_id, sticker_id } => Content::Sticker {
            pack_id: pack_id.clone(),
            pack_key: String::new(),
            sticker_id: *sticker_id,
            emoji: None,
        },
        MessageContent::Reaction { emoji, target_message_id: _, remove: _ } => {
            // Reactions are usually handled separately, but store as text for now
            Content::Text {
                body: format!("Reacted with {} to message", emoji),
                mentions: Vec::new(),
            }
        }
        MessageContent::Quote { quoted_message_id: _, text } => Content::Text {
            body: text.clone(),
            mentions: Vec::new(),
        },
    };

    let sent_at = Utc.timestamp_opt(incoming.timestamp / 1000, 0)
        .single()
        .unwrap_or_else(Utc::now);
    let server_timestamp = Utc.timestamp_opt(incoming.server_timestamp / 1000, 0).single();

    Message {
        id: incoming.id.clone(),
        conversation_id: incoming.conversation_id.clone(),
        sender: incoming.sender.clone(),
        direction: MessageDirection::Incoming,
        status: MessageStatus::Delivered,
        content,
        sent_at,
        server_timestamp,
        delivered_at: Some(Utc::now()),
        read_at: None,
        quote: None,
        reactions: Vec::new(),
        expires_in_seconds: None,
        expires_at: None,
    }
}

impl SignalApp {
    /// Create a new application instance
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Apply custom theme
        let theme = SignalTheme::dark();
        theme.apply(&cc.egui_ctx);

        // Create async runtime
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime"),
        );

        // Initialize storage
        let storage = Arc::new(Storage::new().expect("Failed to initialize storage"));

        // Check if we have an existing account
        let has_account = storage.has_account();

        // Determine initial view
        let view_state = if has_account {
            ViewState::ChatList
        } else {
            ViewState::LinkDevice
        };

        // Create event channel
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let mut app = Self {
            runtime,
            signal_manager: Arc::new(RwLock::new(None)),
            storage,
            view_state,
            theme,
            connection_status: ConnectionStatus::Disconnected,
            error_message: None,
            initialized: false,
            linking_state: LinkingState::default(),
            event_rx: Arc::new(RwLock::new(Some(event_rx))),
            event_tx,
            selected_conversation_id: None,
        };

        // If we have an account, initialize Signal manager
        if has_account {
            app.initialize_signal_manager();
        }

        app
    }

    /// Process pending Signal events
    fn process_events(&mut self, ctx: &egui::Context) {
        // Try to receive events without blocking
        let mut events: Vec<SignalEvent> = Vec::new();
        if let Some(ref mut rx) = *self.event_rx.write() {
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
        }

        for event in events {
            self.handle_event(event, ctx);
        }
    }

    /// Handle a Signal event
    fn handle_event(&mut self, event: SignalEvent, ctx: &egui::Context) {
        match event {
            SignalEvent::ProvisioningUrlReady(url) => {
                tracing::info!("Provisioning URL received, creating QR code");
                // Create QR code texture
                if let Some(image) = crate::ui::views::link_device::render_qr_code(&url) {
                    let texture = ctx.load_texture(
                        "qr_code",
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.linking_state = LinkingState::WaitingForScan {
                        provisioning_url: url,
                        qr_texture: Some(texture),
                    };
                } else {
                    self.linking_state = LinkingState::Error("Failed to generate QR code".to_string());
                }
            }
            SignalEvent::LinkingCompleted => {
                tracing::info!("Device linking completed!");
                self.linking_state = LinkingState::Success;
                self.view_state = ViewState::ChatList;
                self.connection_status = ConnectionStatus::Connected;
                // Reload the signal manager
                self.initialize_signal_manager();
            }
            SignalEvent::LinkingFailed(error) => {
                tracing::error!("Device linking failed: {}", error);
                self.linking_state = LinkingState::Error(error);
            }
            SignalEvent::ConnectionStateChanged(state) => {
                self.connection_status = match state {
                    SignalConnectionState::Connected => ConnectionStatus::Connected,
                    SignalConnectionState::Connecting => ConnectionStatus::Connecting,
                    SignalConnectionState::Reconnecting => ConnectionStatus::Reconnecting,
                    SignalConnectionState::Disconnected => ConnectionStatus::Disconnected,
                };
            }
            SignalEvent::Error(error) => {
                self.error_message = Some(error);
            }
            SignalEvent::MessageReceived(incoming) => {
                self.handle_incoming_message(&incoming);
            }
            _ => {
                tracing::debug!("Received event: {:?}", event);
            }
        }
    }

    fn handle_incoming_message(&self, incoming: &IncomingMessage) {
        let Some(db) = self.storage.database() else {
            tracing::warn!("No database available, cannot save message");
            return;
        };

        let message = incoming_to_message(incoming);
        let message_repo = MessageRepository::new(db);
        let conv_repo = ConversationRepository::new(db);

        let text_preview = match &incoming.content {
            MessageContent::Text(t) => t.clone(),
            MessageContent::Attachment { .. } => "[Attachment]".to_string(),
            MessageContent::Sticker { .. } => "[Sticker]".to_string(),
            MessageContent::Reaction { emoji, .. } => format!("Reacted {}", emoji),
            MessageContent::Quote { text, .. } => text.clone(),
        };

        if conv_repo.get(&incoming.conversation_id).is_none() {
            let is_group = incoming.conversation_id != incoming.sender;
            let conv = if is_group {
                Conversation::new_group(&incoming.conversation_id, "Group")
            } else {
                Conversation::new_private(&incoming.conversation_id, &incoming.sender)
            };
            if let Err(e) = conv_repo.save(&conv) {
                tracing::error!("Failed to create conversation: {}", e);
                return;
            }
        }

        if let Err(e) = message_repo.save(&message) {
            tracing::error!("Failed to save message: {}", e);
            return;
        }

        if let Some(mut conv) = conv_repo.get(&incoming.conversation_id) {
            conv.update_last_message(&text_preview, message.sent_at);
            conv.increment_unread();
            if let Err(e) = conv_repo.save(&conv) {
                tracing::error!("Failed to update conversation: {}", e);
            }
        }

        tracing::info!("Saved message {} from {}", incoming.id, incoming.sender);
    }

    /// Start the device linking process
    pub fn start_linking(&mut self) {
        // Only start if not already started (don't auto-retry on error)
        if matches!(self.linking_state, LinkingState::NotStarted) {
            self.linking_state = LinkingState::Generating;
            let storage = self.storage.clone();
            let event_tx = self.event_tx.clone();
            let device_name = get_device_name();

            SignalManager::start_linking(storage, device_name, event_tx);
        }
    }

    /// Retry device linking after an error
    pub fn retry_linking(&mut self) {
        if matches!(self.linking_state, LinkingState::Error(_)) {
            self.linking_state = LinkingState::Generating;
            let storage = self.storage.clone();
            let event_tx = self.event_tx.clone();
            let device_name = get_device_name();

            SignalManager::start_linking(storage, device_name, event_tx);
        }
    }

    /// Get the current linking state
    pub fn linking_state(&self) -> &LinkingState {
        &self.linking_state
    }

    fn initialize_signal_manager(&mut self) {
        let storage = self.storage.clone();
        let signal_manager = self.signal_manager.clone();
        let event_tx = self.event_tx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for Signal manager");

            rt.block_on(async move {
                match SignalManager::from_storage(&storage, event_tx.clone()).await {
                    Ok(manager) => {
                        *signal_manager.write() = Some(manager);
                        tracing::info!("Signal manager initialized, starting receive loop...");
                        SignalManager::start_receiving(storage, event_tx);
                    }
                    Err(e) => {
                        tracing::error!("Failed to initialize Signal manager: {}", e);
                    }
                }
            });
        });

        self.initialized = true;
    }

    /// Handle device linking completion
    pub fn on_device_linked(&mut self, manager: SignalManager) {
        *self.signal_manager.write() = Some(manager);
        self.view_state = ViewState::ChatList;
        self.connection_status = ConnectionStatus::Connected;
        self.initialized = true;
    }

    /// Set error message
    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
    }

    /// Clear error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Get the async runtime
    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }

    /// Get storage reference
    pub fn storage(&self) -> &Arc<Storage> {
        &self.storage
    }

    pub fn signal_manager(&self) -> &Arc<RwLock<Option<SignalManager>>> {
        &self.signal_manager
    }

    pub fn selected_conversation_id(&self) -> Option<&str> {
        self.selected_conversation_id.as_deref()
    }

    pub fn select_conversation(&mut self, id: Option<String>) {
        self.selected_conversation_id = id;
    }
}

impl eframe::App for SignalApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process any pending Signal events
        self.process_events(ctx);

        // Only repaint periodically to check for new events (not every frame)
        // This reduces CPU usage from 100% to near 0% when idle
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        // Show error toast if present
        let mut dismiss_error = false;
        if let Some(ref error) = self.error_message {
            let error_text = error.clone();
            egui::TopBottomPanel::top("error_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error_text));
                    if ui.button("Dismiss").clicked() {
                        dismiss_error = true;
                    }
                });
            });
        }
        if dismiss_error {
            self.error_message = None;
        }

        // Show connection status bar
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (color, text) = match &self.connection_status {
                        ConnectionStatus::Connected => (egui::Color32::GREEN, "Connected"),
                        ConnectionStatus::Connecting => (egui::Color32::YELLOW, "Connecting..."),
                        ConnectionStatus::Reconnecting => (egui::Color32::YELLOW, "Reconnecting..."),
                        ConnectionStatus::Disconnected => (egui::Color32::GRAY, "Disconnected"),
                        ConnectionStatus::Error(e) => (egui::Color32::RED, e.as_str()),
                    };
                    ui.colored_label(color, format!("● {}", text));
                });
            });

        // Main content based on current view
        match &self.view_state {
            ViewState::LinkDevice => {
                crate::ui::views::link_device::show(self, ctx);
            }
            ViewState::ChatList => {
                crate::ui::views::main_view::show(self, ctx);
            }
            ViewState::Settings => {
                crate::ui::views::settings::show(self, ctx);
            }
        }
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // Save application state if needed
    }
}
