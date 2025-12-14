//! Main application state and logic

use crate::signal::SignalManager;
use crate::storage::Storage;
use crate::ui::{theme::SignalTheme, views::ViewState};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Main application state
pub struct SignalApp {
    /// Async runtime for background tasks
    runtime: Arc<Runtime>,

    /// Signal protocol manager
    signal_manager: Arc<RwLock<Option<SignalManager>>>,

    /// Local storage
    storage: Arc<Storage>,

    /// Current view state
    view_state: ViewState,

    /// Theme configuration
    theme: SignalTheme,

    /// Connection status
    connection_status: ConnectionStatus,

    /// Error message to display
    error_message: Option<String>,

    /// Whether the app is initialized
    initialized: bool,
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

        let mut app = Self {
            runtime,
            signal_manager: Arc::new(RwLock::new(None)),
            storage,
            view_state,
            theme,
            connection_status: ConnectionStatus::Disconnected,
            error_message: None,
            initialized: false,
        };

        // If we have an account, initialize Signal manager
        if has_account {
            app.initialize_signal_manager();
        }

        app
    }

    /// Initialize the Signal manager with existing account
    fn initialize_signal_manager(&mut self) {
        let storage = self.storage.clone();
        let signal_manager = self.signal_manager.clone();
        let runtime = self.runtime.clone();

        runtime.spawn(async move {
            match SignalManager::from_storage(&storage).await {
                Ok(manager) => {
                    *signal_manager.write() = Some(manager);
                    tracing::info!("Signal manager initialized successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to initialize Signal manager: {}", e);
                }
            }
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

    /// Get Signal manager
    pub fn signal_manager(&self) -> &Arc<RwLock<Option<SignalManager>>> {
        &self.signal_manager
    }
}

impl eframe::App for SignalApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repainting for real-time updates
        ctx.request_repaint();

        // Show error toast if present
        if let Some(ref error) = self.error_message {
            egui::TopBottomPanel::top("error_panel").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
                    if ui.button("Dismiss").clicked() {
                        self.error_message = None;
                    }
                });
            });
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
