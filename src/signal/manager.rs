//! Signal manager - main interface for Signal protocol operations

use crate::signal::SignalError;
use crate::storage::Storage;
use futures::channel::oneshot;
use futures::StreamExt;
use presage::libsignal_service::configuration::SignalServers;
use presage::libsignal_service::prelude::Content;
use presage::libsignal_service::protocol::ServiceId;
use presage::libsignal_service::proto::DataMessage;
use presage::model::messages::Received;
use presage::Manager;
use presage_store_sqlite::{OnNewIdentity, SqliteStore};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use url::Url;
use uuid::Uuid;

/// Events emitted by the Signal manager
#[derive(Debug, Clone)]
pub enum SignalEvent {
    /// Connection state changed
    ConnectionStateChanged(ConnectionState),
    /// Provisioning URL ready for QR code
    ProvisioningUrlReady(String),
    /// Device linking completed
    LinkingCompleted,
    /// Device linking failed
    LinkingFailed(String),
    /// New message received
    MessageReceived(IncomingMessage),
    /// Message sent successfully
    MessageSent { message_id: String },
    /// Message delivery receipt
    DeliveryReceipt { message_id: String, recipient: String },
    /// Message read receipt
    ReadReceipt { message_id: String, recipient: String },
    /// Typing indicator
    TypingStarted { conversation_id: String },
    /// Typing stopped
    TypingStopped { conversation_id: String },
    /// Contact updated
    ContactUpdated { contact_id: String },
    /// Group updated
    GroupUpdated { group_id: String },
    /// Sync completed
    SyncCompleted,
    /// Error occurred
    Error(String),
}

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Incoming message
#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub id: String,
    pub sender: String,
    pub conversation_id: String,
    pub content: MessageContent,
    pub timestamp: i64,
    pub server_timestamp: i64,
}

/// Message content types
#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Attachment {
        content_type: String,
        filename: Option<String>,
        size: u64,
        attachment_id: String,
    },
    Sticker {
        pack_id: String,
        sticker_id: u32,
    },
    Reaction {
        emoji: String,
        target_message_id: String,
        remove: bool,
    },
    Quote {
        quoted_message_id: String,
        text: String,
    },
}

/// Result of device linking
pub struct LinkingResult {
    pub manager: SignalManager,
}

/// Signal manager for protocol operations
pub struct SignalManager {
    /// Storage reference
    storage: Arc<Storage>,
    /// Event sender
    event_tx: mpsc::UnboundedSender<SignalEvent>,
    /// Event receiver (taken by app)
    event_rx: Option<mpsc::UnboundedReceiver<SignalEvent>>,
    /// Connection state
    connection_state: ConnectionState,
    /// Account phone number (E.164 format)
    phone_number: Option<String>,
    /// Device ID
    device_id: Option<u32>,
}

impl SignalManager {
    /// Start the device linking process
    ///
    /// This spawns a background task that:
    /// 1. Creates the presage store
    /// 2. Initiates device linking
    /// 3. Sends the provisioning URL through the event channel
    /// 4. Completes linking when phone scans QR code
    pub fn start_linking(
        storage: Arc<Storage>,
        device_name: String,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
    ) {
        // Use a dedicated thread for presage operations since its futures aren't Send-safe
        std::thread::spawn(move || {
            // Create a new single-threaded runtime for presage
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for linking");

            rt.block_on(async move {
                match Self::perform_linking(&storage, &device_name, event_tx.clone()).await {
                    Ok(()) => {
                        tracing::info!("Device linking completed successfully");
                        let _ = event_tx.send(SignalEvent::LinkingCompleted);
                    }
                    Err(e) => {
                        tracing::error!("Device linking failed: {}", e);
                        let _ = event_tx.send(SignalEvent::LinkingFailed(e.to_string()));
                    }
                }
            });
        });
    }

    /// Perform the actual linking process
    async fn perform_linking(
        storage: &Arc<Storage>,
        device_name: &str,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
    ) -> Result<(), SignalError> {
        tracing::info!("Starting device linking process...");

        // Get database path
        let db_path = storage.signal_db_path();
        let db_url = format!("sqlite://{}", db_path.display());

        tracing::info!("Opening Signal store at: {}", db_url);

        // Create the SQLite store with encryption
        let passphrase = storage.get_encryption_key();

        let store = SqliteStore::open_with_passphrase(
            &db_url,
            passphrase.as_deref(),
            OnNewIdentity::Trust, // Trust new identities for now
        )
        .await
        .map_err(|e| SignalError::StorageError(e.to_string()))?;

        tracing::info!("Signal store opened successfully");

        // Create oneshot channel for provisioning URL
        let (tx, rx) = oneshot::channel::<Url>();

        // Run linking and URL receiving concurrently
        let device_name_clone = device_name.to_string();
        let event_tx_clone = event_tx.clone();

        let (link_result, _) = futures::future::join(
            Manager::link_secondary_device(
                store,
                SignalServers::Production,
                device_name_clone,
                tx,
            ),
            async move {
                match rx.await {
                    Ok(url) => {
                        tracing::info!("Provisioning URL received: {}", url);
                        let _ = event_tx_clone.send(SignalEvent::ProvisioningUrlReady(url.to_string()));
                    }
                    Err(e) => {
                        tracing::error!("Failed to receive provisioning URL: {:?}", e);
                    }
                }
            }
        ).await;

        let manager = link_result
            .map_err(|e| SignalError::LinkingFailed(format!("{:?}", e)))?;

        let reg_data = manager.registration_data();
        let phone_number = reg_data.phone_number.to_string();
        let device_id = reg_data.device_id.unwrap_or(1);

        let config = serde_json::json!({
            "phone_number": phone_number,
            "device_id": device_id,
        });

        let config_path = storage.data_dir().join("config.json");
        if let Err(e) = std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap_or_default()) {
            tracing::error!("Failed to save config: {}", e);
        } else {
            tracing::info!("Account saved for: {}", phone_number);
        }

        tracing::info!("Device linked successfully!");

        Ok(())
    }

    /// Create a new Signal manager for device linking (legacy interface)
    pub async fn link_device(
        storage: &Arc<Storage>,
        device_name: &str,
    ) -> Result<Self, SignalError> {
        tracing::info!("Creating Signal manager for device linking...");

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Start linking in background
        Self::start_linking(storage.clone(), device_name.to_string(), event_tx.clone());

        let manager = Self {
            storage: storage.clone(),
            event_tx,
            event_rx: Some(event_rx),
            connection_state: ConnectionState::Disconnected,
            phone_number: None,
            device_id: None,
        };

        Ok(manager)
    }

    /// Create a Signal manager from existing stored credentials
    /// Takes the app's event_tx to send events to the main event loop
    pub async fn from_storage(
        storage: &Arc<Storage>,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
    ) -> Result<Self, SignalError> {
        tracing::info!("Loading Signal manager from storage...");

        // Check if we have valid credentials
        let db_path = storage.signal_db_path();
        if !db_path.exists() {
            return Err(SignalError::NotRegistered);
        }

        let db_url = format!("sqlite://{}", db_path.display());
        let passphrase = storage.get_encryption_key();

        // Try to open existing store
        let store = SqliteStore::open_with_passphrase(
            &db_url,
            passphrase.as_deref(),
            OnNewIdentity::Trust,
        )
        .await
        .map_err(|e| SignalError::StorageError(e.to_string()))?;

        // Try to load existing manager
        let _manager = Manager::load_registered(store)
            .await
            .map_err(|e| SignalError::NotRegistered)?;

        let manager = Self {
            storage: storage.clone(),
            event_tx,
            event_rx: None,
            connection_state: ConnectionState::Disconnected,
            phone_number: storage.get_phone_number(),
            device_id: storage.get_device_id(),
        };

        Ok(manager)
    }

    /// Get the event receiver
    pub fn take_event_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<SignalEvent>> {
        self.event_rx.take()
    }

    /// Get connection state
    pub fn connection_state(&self) -> ConnectionState {
        self.connection_state.clone()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connection_state == ConnectionState::Connected
    }

    /// Get phone number
    pub fn phone_number(&self) -> Option<&str> {
        self.phone_number.as_deref()
    }

    /// Get device ID
    pub fn device_id(&self) -> Option<u32> {
        self.device_id
    }

    pub async fn connect(&mut self) -> Result<(), SignalError> {
        tracing::info!("Connecting to Signal servers...");

        self.connection_state = ConnectionState::Connecting;
        self.event_tx
            .send(SignalEvent::ConnectionStateChanged(ConnectionState::Connecting))
            .ok();

        Self::start_receiving(self.storage.clone(), self.event_tx.clone());

        Ok(())
    }

    pub fn start_receiving(
        storage: Arc<Storage>,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
    ) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for receiving");

            rt.block_on(async move {
                if let Err(e) = Self::receive_loop(&storage, event_tx.clone()).await {
                    tracing::error!("Message receive loop failed: {}", e);
                    let _ = event_tx.send(SignalEvent::Error(e.to_string()));
                    let _ = event_tx.send(SignalEvent::ConnectionStateChanged(ConnectionState::Disconnected));
                }
            });
        });
    }

    async fn receive_loop(
        storage: &Arc<Storage>,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
    ) -> Result<(), SignalError> {
        let db_path = storage.signal_db_path();
        let db_url = format!("sqlite://{}", db_path.display());
        let passphrase = storage.get_encryption_key();

        tracing::info!("Opening Signal store for receiving: {}", db_url);

        let store = SqliteStore::open_with_passphrase(
            &db_url,
            passphrase.as_deref(),
            OnNewIdentity::Trust,
        )
        .await
        .map_err(|e| SignalError::StorageError(e.to_string()))?;

        let mut manager = Manager::load_registered(store)
            .await
            .map_err(|_| SignalError::NotRegistered)?;

        tracing::info!("Starting message receive stream...");
        let _ = event_tx.send(SignalEvent::ConnectionStateChanged(ConnectionState::Connected));

        let messages = manager
            .receive_messages()
            .await
            .map_err(|e| SignalError::ConnectionFailed(format!("{:?}", e)))?;

        futures::pin_mut!(messages);

        while let Some(received) = messages.next().await {
            match received {
                Received::QueueEmpty => {
                    tracing::info!("Message queue synchronized");
                    let _ = event_tx.send(SignalEvent::SyncCompleted);
                }
                Received::Contacts => {
                    tracing::info!("Contacts synchronized");
                }
                Received::Content(content) => {
                    if let Some(incoming) = Self::process_content(&content) {
                        tracing::info!("Received message from {}", incoming.sender);
                        let _ = event_tx.send(SignalEvent::MessageReceived(incoming));
                    }
                }
            }
        }

        tracing::warn!("Message stream ended");
        let _ = event_tx.send(SignalEvent::ConnectionStateChanged(ConnectionState::Disconnected));

        Ok(())
    }

    fn process_content(content: &Content) -> Option<IncomingMessage> {
        use presage::libsignal_service::content::ContentBody;

        let sender = format!("{:?}", content.metadata.sender);
        let timestamp = content.metadata.timestamp as i64;

        match &content.body {
            ContentBody::DataMessage(data_msg) => {
                let text = data_msg.body.clone().unwrap_or_default();
                if text.is_empty() && data_msg.attachments.is_empty() {
                    return None;
                }

                let conversation_id = if let Some(group) = &data_msg.group_v2 {
                    if let Some(master_key) = &group.master_key {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(master_key)
                    } else {
                        sender.clone()
                    }
                } else {
                    sender.clone()
                };

                Some(IncomingMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    sender: sender.clone(),
                    conversation_id,
                    content: MessageContent::Text(text),
                    timestamp,
                    server_timestamp: timestamp,
                })
            }
            ContentBody::ReceiptMessage(receipt) => {
                tracing::debug!("Received receipt: {:?}", receipt);
                None
            }
            ContentBody::TypingMessage(typing) => {
                tracing::debug!("Received typing indicator: {:?}", typing);
                None
            }
            _ => {
                tracing::debug!("Received other message type");
                None
            }
        }
    }

    /// Disconnect from Signal servers
    pub async fn disconnect(&mut self) -> Result<(), SignalError> {
        tracing::info!("Disconnecting from Signal servers...");

        self.connection_state = ConnectionState::Disconnected;
        self.event_tx
            .send(SignalEvent::ConnectionStateChanged(ConnectionState::Disconnected))
            .ok();

        Ok(())
    }

    pub async fn send_message(
        &self,
        recipient: &str,
        text: &str,
    ) -> Result<String, SignalError> {
        let message_id = Uuid::new_v4().to_string();
        
        let recipient_uuid = Uuid::parse_str(recipient)
            .map_err(|e| SignalError::SendFailed(format!("Invalid recipient UUID: {}", e)))?;
        
        let storage = self.storage.clone();
        let event_tx = self.event_tx.clone();
        let text = text.to_string();
        let msg_id = message_id.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for sending");
            
            rt.block_on(async move {
                match Self::do_send_message(&storage, recipient_uuid, &text).await {
                    Ok(()) => {
                        tracing::info!("Message {} sent successfully", msg_id);
                        let _ = event_tx.send(SignalEvent::MessageSent { message_id: msg_id });
                    }
                    Err(e) => {
                        tracing::error!("Failed to send message {}: {}", msg_id, e);
                        let _ = event_tx.send(SignalEvent::Error(format!("Send failed: {}", e)));
                    }
                }
            });
        });
        
        Ok(message_id)
    }
    
    async fn do_send_message(
        storage: &Arc<Storage>,
        recipient: Uuid,
        text: &str,
    ) -> Result<(), SignalError> {
        let db_path = storage.signal_db_path();
        let db_url = format!("sqlite://{}", db_path.display());
        let passphrase = storage.get_encryption_key();
        
        let store = SqliteStore::open_with_passphrase(
            &db_url,
            passphrase.as_deref(),
            OnNewIdentity::Trust,
        )
        .await
        .map_err(|e| SignalError::StorageError(e.to_string()))?;
        
        let mut manager = Manager::load_registered(store)
            .await
            .map_err(|_| SignalError::NotRegistered)?;
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SignalError::SendFailed(e.to_string()))?
            .as_millis() as u64;
        
        let data_message = DataMessage {
            body: Some(text.to_string()),
            timestamp: Some(timestamp),
            ..Default::default()
        };
        
        let service_id = ServiceId::Aci(recipient.into());
        
        manager
            .send_message(service_id, data_message, timestamp)
            .await
            .map_err(|e| SignalError::SendFailed(format!("{:?}", e)))?;
        
        Ok(())
    }

    pub async fn send_group_message(
        &self,
        group_id: &str,
        text: &str,
    ) -> Result<String, SignalError> {
        let message_id = Uuid::new_v4().to_string();
        
        let master_key = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            group_id,
        ).map_err(|e| SignalError::SendFailed(format!("Invalid group ID: {}", e)))?;
        
        let storage = self.storage.clone();
        let event_tx = self.event_tx.clone();
        let text = text.to_string();
        let msg_id = message_id.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for sending");
            
            rt.block_on(async move {
                match Self::do_send_group_message(&storage, &master_key, &text).await {
                    Ok(()) => {
                        tracing::info!("Group message {} sent successfully", msg_id);
                        let _ = event_tx.send(SignalEvent::MessageSent { message_id: msg_id });
                    }
                    Err(e) => {
                        tracing::error!("Failed to send group message {}: {}", msg_id, e);
                        let _ = event_tx.send(SignalEvent::Error(format!("Send failed: {}", e)));
                    }
                }
            });
        });
        
        Ok(message_id)
    }
    
    async fn do_send_group_message(
        storage: &Arc<Storage>,
        master_key: &[u8],
        text: &str,
    ) -> Result<(), SignalError> {
        let db_path = storage.signal_db_path();
        let db_url = format!("sqlite://{}", db_path.display());
        let passphrase = storage.get_encryption_key();
        
        let store = SqliteStore::open_with_passphrase(
            &db_url,
            passphrase.as_deref(),
            OnNewIdentity::Trust,
        )
        .await
        .map_err(|e| SignalError::StorageError(e.to_string()))?;
        
        let mut manager = Manager::load_registered(store)
            .await
            .map_err(|_| SignalError::NotRegistered)?;
        
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| SignalError::SendFailed(e.to_string()))?
            .as_millis() as u64;
        
        let data_message = DataMessage {
            body: Some(text.to_string()),
            timestamp: Some(timestamp),
            ..Default::default()
        };
        
        manager
            .send_message_to_group(master_key, data_message, timestamp)
            .await
            .map_err(|e| SignalError::SendFailed(format!("{:?}", e)))?;
        
        Ok(())
    }

    /// Send a reaction
    pub async fn send_reaction(
        &self,
        _conversation_id: &str,
        message_id: &str,
        emoji: &str,
        remove: bool,
    ) -> Result<(), SignalError> {
        tracing::info!(
            "Sending reaction {} to message {} (remove: {})",
            emoji,
            message_id,
            remove
        );

        Ok(())
    }

    /// Mark messages as read
    pub async fn mark_read(
        &self,
        _conversation_id: &str,
        message_ids: &[String],
    ) -> Result<(), SignalError> {
        tracing::info!("Marking {} messages as read", message_ids.len());
        Ok(())
    }

    /// Send typing indicator
    pub async fn send_typing(&self, _conversation_id: &str, _is_typing: bool) -> Result<(), SignalError> {
        Ok(())
    }

    pub async fn request_sync(&self) -> Result<(), SignalError> {
        tracing::info!("Requesting sync from primary device...");
        Ok(())
    }
    
    pub async fn send_message_static(
        storage: &Arc<Storage>,
        recipient: &str,
        text: &str,
    ) -> Result<(), SignalError> {
        let recipient_uuid = Uuid::parse_str(recipient)
            .map_err(|e| SignalError::SendFailed(format!("Invalid recipient UUID: {}", e)))?;
        Self::do_send_message(storage, recipient_uuid, text).await
    }
    
    pub async fn send_group_message_static(
        storage: &Arc<Storage>,
        group_id: &str,
        text: &str,
    ) -> Result<(), SignalError> {
        let master_key = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            group_id,
        ).map_err(|e| SignalError::SendFailed(format!("Invalid group ID: {}", e)))?;
        Self::do_send_group_message(storage, &master_key, text).await
    }
}
