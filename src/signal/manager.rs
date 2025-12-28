//! Signal manager - main interface for Signal protocol operations

use crate::signal::provisioning;
use crate::signal::registration;
use crate::signal::SignalError;
use crate::storage::contacts::{ContactRepository, StoredContact};
use crate::storage::Storage;
use chrono::Utc;
use futures::channel::oneshot;
use futures::StreamExt;
use parking_lot::Mutex;
use presage::libsignal_service::configuration::SignalServers;
use presage::libsignal_service::prelude::Content;
use presage::libsignal_service::protocol::ServiceId;
use presage::libsignal_service::proto::DataMessage;
use presage::model::messages::Received;
use presage::manager::Registered;
use presage::store::ContentsStore;
use presage::Manager;
use presage_store_sqlite::{OnNewIdentity, SqliteStore};
use rand::distr::{Alphanumeric, SampleString};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use uuid::Uuid;

pub enum SendCommand {
    DirectMessage {
        recipient: Uuid,
        text: String,
        reply: oneshot::Sender<Result<(), SignalError>>,
    },
    GroupMessage {
        group_key: Vec<u8>,
        text: String,
        reply: oneshot::Sender<Result<(), SignalError>>,
    },
}

static SEND_TX: Mutex<Option<mpsc::UnboundedSender<SendCommand>>> = Mutex::new(None);

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
    /// Message history transfer available from primary device
    MessageHistoryAvailable,
    /// Message history sync progress
    MessageHistorySyncProgress { current: u32, total: u32 },
    /// Message history sync completed
    MessageHistorySyncCompleted { message_count: u32 },
    /// Message history sync failed
    MessageHistorySyncFailed(String),
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

    /// Perform the actual linking process using custom provisioning
    /// 
    /// This custom flow captures the ephemeral_backup_key from the provision message,
    /// which enables message history sync from the primary device.
    async fn perform_linking(
        storage: &Arc<Storage>,
        device_name: &str,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
    ) -> Result<(), SignalError> {
        tracing::info!("Starting device linking process with custom provisioning...");

        // Get database path
        let db_path = storage.signal_db_path();
        let db_url = format!("sqlite://{}", db_path.display());

        tracing::info!("Opening Signal store at: {}", db_url);

        // Create the SQLite store with encryption
        let passphrase = storage.get_encryption_key();

        let mut store = SqliteStore::open_with_passphrase(
            &db_url,
            passphrase.as_deref(),
            OnNewIdentity::Trust,
        )
        .await
        .map_err(|e| SignalError::StorageError(e.to_string()))?;

        tracing::info!("Signal store opened successfully");

        let password: String = Alphanumeric.sample_string(&mut rand::rng(), 24);

        tracing::info!("Starting custom provisioning...");
        let event_tx_for_url = event_tx.clone();
        let provision_msg = provisioning::run_provisioning_capture(
            SignalServers::Production,
            move |url| {
                tracing::info!("Provisioning URL ready: {}", url);
                let _ = event_tx_for_url.send(SignalEvent::ProvisioningUrlReady(url.to_string()));
            },
        )
        .await?;

        tracing::info!("Received provision message for phone: {}", provision_msg.phone_number);

        let has_backup_key = provision_msg.ephemeral_backup_key.is_some();
        if has_backup_key {
            tracing::info!("Ephemeral backup key available - message history sync possible!");
            let _ = event_tx.send(SignalEvent::MessageHistoryAvailable);
        } else {
            tracing::info!("No ephemeral backup key - message history sync not available");
        }

        tracing::info!("Completing device registration...");
        let reg_result = registration::complete_registration(
            &mut store,
            &provision_msg,
            device_name,
            &password,
            SignalServers::Production,
        )
        .await?;

        tracing::info!(
            "Device registered successfully! ACI: {}, Device ID: {}",
            reg_result.aci,
            reg_result.device_id
        );

        if let Err(e) = storage.save_account(&reg_result.phone_number, reg_result.device_id) {
            tracing::error!("Failed to save account: {}", e);
        }

        if has_backup_key {
            tracing::info!("Initiating message history sync...");
            if let Some(backup_key) = provisioning::get_ephemeral_backup_key() {
                match Self::sync_message_history(
                    &backup_key,
                    &reg_result.aci,
                    reg_result.device_id,
                    &reg_result.password,
                    storage,
                    event_tx.clone(),
                ).await {
                    Ok(count) => {
                        tracing::info!("Message history sync completed: {} messages", count);
                        let _ = event_tx.send(SignalEvent::MessageHistorySyncCompleted { 
                            message_count: count 
                        });
                    }
                    Err(e) => {
                        tracing::error!("Message history sync failed: {}", e);
                        let _ = event_tx.send(SignalEvent::MessageHistorySyncFailed(e.to_string()));
                    }
                }
            }
        }

        tracing::info!("Loading registered manager...");
        let mut manager = Manager::load_registered(store)
            .await
            .map_err(|e| SignalError::StorageError(format!("Failed to load manager: {:?}", e)))?;

        tracing::info!("Requesting contacts sync from primary device...");
        if let Err(e) = manager.request_contacts().await {
            tracing::error!("Failed to request contacts sync: {:?}", e);
        } else {
            tracing::info!("Contacts sync requested successfully");
        }

        tracing::info!("Device linked successfully!");
        Ok(())
    }

    async fn sync_message_history(
        backup_key: &[u8],
        aci: &uuid::Uuid,
        device_id: u32,
        password: &str,
        storage: &Arc<Storage>,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
    ) -> Result<u32, SignalError> {
        tracing::info!("Fetching transfer archive from Signal servers...");
        
        let auth_username = format!("{}.{}", aci, device_id);
        tracing::debug!("Using auth username: {}", auth_username);
        
        let _ = event_tx.send(SignalEvent::MessageHistorySyncProgress { 
            current: 0, 
            total: 0 
        });
        
        let backup_data = crate::signal::backup::sync_message_history(
            backup_key,
            aci,
            &auth_username,
            password,
        ).await?;
        
        let message_count = backup_data.messages.len() as u32;
        let conversation_count = backup_data.conversations.len();
        tracing::info!(
            "Backup sync complete: {} messages, {} conversations",
            message_count,
            conversation_count
        );
        
        let _ = event_tx.send(SignalEvent::MessageHistorySyncProgress { 
            current: message_count / 2, 
            total: message_count 
        });

        let (convs_imported, msgs_imported) = crate::signal::backup::import_backup_data(
            &backup_data,
            storage,
        )?;
        
        tracing::info!(
            "Imported to storage: {} conversations, {} messages",
            convs_imported,
            msgs_imported
        );
        
        let _ = event_tx.send(SignalEvent::MessageHistorySyncProgress { 
            current: message_count, 
            total: message_count 
        });

        Ok(msgs_imported as u32)
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
        let (send_tx, send_rx) = mpsc::unbounded_channel::<SendCommand>();
        
        {
            let mut guard = SEND_TX.lock();
            *guard = Some(send_tx);
        }
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for receiving");

            rt.block_on(async move {
                if let Err(e) = Self::receive_loop(&storage, event_tx.clone(), send_rx).await {
                    tracing::error!("Message receive loop failed: {}", e);
                    let _ = event_tx.send(SignalEvent::Error(e.to_string()));
                    let _ = event_tx.send(SignalEvent::ConnectionStateChanged(ConnectionState::Disconnected));
                }
                
                let mut guard = SEND_TX.lock();
                *guard = None;
            });
        });
    }

    async fn receive_loop(
        storage: &Arc<Storage>,
        event_tx: mpsc::UnboundedSender<SignalEvent>,
        mut send_rx: mpsc::UnboundedReceiver<SendCommand>,
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

        loop {
            tokio::select! {
                received = messages.next() => {
                    match received {
                        Some(Received::QueueEmpty) => {
                            tracing::info!("Message queue synchronized");
                            let _ = event_tx.send(SignalEvent::SyncCompleted);
                        }
                        Some(Received::Contacts) => {
                            tracing::info!("Contacts sync signal received, syncing to local database...");
                            if let Err(e) = Self::sync_contacts_to_local(manager.store(), storage).await {
                                tracing::error!("Failed to sync contacts to local storage: {}", e);
                            } else {
                                tracing::info!("Contacts synced to local database");
                                let _ = event_tx.send(SignalEvent::ContactUpdated { contact_id: "all".to_string() });
                            }
                        }
                        Some(Received::Content(content)) => {
                            Self::log_content_verbose(&content);
                            if let Some(incoming) = Self::process_content(&content) {
                                tracing::info!("Received message from {}", incoming.sender);
                                let _ = event_tx.send(SignalEvent::MessageReceived(incoming));
                            }
                        }
                        None => {
                            tracing::warn!("Message stream ended");
                            break;
                        }
                    }
                }
                cmd = send_rx.recv() => {
                    match cmd {
                        Some(SendCommand::DirectMessage { recipient, text, reply }) => {
                            let result = Self::send_dm_with_manager(&mut manager, recipient, &text).await;
                            let _ = reply.send(result);
                        }
                        Some(SendCommand::GroupMessage { group_key, text, reply }) => {
                            let result = Self::send_group_with_manager(&mut manager, &group_key, &text).await;
                            let _ = reply.send(result);
                        }
                        None => {
                            tracing::info!("Send channel closed");
                            break;
                        }
                    }
                }
            }
        }

        let _ = event_tx.send(SignalEvent::ConnectionStateChanged(ConnectionState::Disconnected));

        Ok(())
    }

    async fn sync_contacts_to_local(
        presage_store: &SqliteStore,
        storage: &Arc<Storage>,
    ) -> Result<(), SignalError> {
        let contacts_iter = presage_store
            .contacts()
            .await
            .map_err(|e| SignalError::StorageError(format!("Failed to get contacts from presage: {:?}", e)))?;

        let presage_contacts: Vec<_> = contacts_iter.filter_map(|r| r.ok()).collect();
        tracing::info!("Found {} contacts in presage store", presage_contacts.len());

        let db = storage
            .database()
            .ok_or_else(|| SignalError::StorageError("App database not available".to_string()))?;
        let repo = ContactRepository::new(&db);

        let now = Utc::now().timestamp();

        for presage_contact in presage_contacts {
            let uuid_str = presage_contact.uuid.to_string();
            let phone_str = presage_contact.phone_number.map(|p| p.to_string());

            let stored_contact = StoredContact {
                id: uuid_str.clone(),
                uuid: uuid_str,
                phone_number: phone_str,
                name: presage_contact.name.clone(),
                profile_name: if presage_contact.name.is_empty() {
                    None
                } else {
                    Some(presage_contact.name)
                },
                avatar_path: None,
                profile_key: if presage_contact.profile_key.is_empty() {
                    None
                } else {
                    Some(presage_contact.profile_key)
                },
                is_blocked: false,
                is_verified: false,
                created_at: now,
                updated_at: now,
            };

            if let Err(e) = repo.save(&stored_contact) {
                tracing::warn!("Failed to save contact {}: {}", stored_contact.id, e);
            }
        }

        tracing::info!("Synced {} contacts to local database", repo.count());
        Ok(())
    }
    
    async fn send_dm_with_manager(
        manager: &mut Manager<SqliteStore, Registered>,
        recipient: Uuid,
        text: &str,
    ) -> Result<(), SignalError> {
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
    
    async fn send_group_with_manager(
        manager: &mut Manager<SqliteStore, Registered>,
        master_key: &[u8],
        text: &str,
    ) -> Result<(), SignalError> {
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

    fn log_content_verbose(content: &Content) {
        use presage::libsignal_service::content::ContentBody;

        let sender = format!("{:?}", content.metadata.sender);
        let timestamp = content.metadata.timestamp;

        match &content.body {
            ContentBody::DataMessage(dm) => {
                tracing::debug!(
                    "[VERBOSE] DataMessage from={} ts={} body={:?} group={:?} attachments={}",
                    sender,
                    timestamp,
                    dm.body.as_ref().map(|b| if b.len() > 50 { format!("{}...", &b[..50]) } else { b.clone() }),
                    dm.group_v2.is_some(),
                    dm.attachments.len()
                );
            }
            ContentBody::SynchronizeMessage(sync) => {
                tracing::info!(
                    "[VERBOSE] SyncMessage from={} ts={} sent={} contacts={} blocked={} request={} keys={} fetch_latest={} message_request={} configuration={} sticker_pack={} view_once={} verified={} call_event={}",
                    sender,
                    timestamp,
                    sync.sent.is_some(),
                    sync.contacts.is_some(),
                    sync.blocked.is_some(),
                    sync.request.is_some(),
                    sync.keys.is_some(),
                    sync.fetch_latest.is_some(),
                    sync.message_request_response.is_some(),
                    sync.configuration.is_some(),
                    sync.sticker_pack_operation.len(),
                    sync.view_once_open.is_some(),
                    sync.verified.is_some(),
                    sync.call_event.is_some(),
                );
                
                if let Some(sent) = &sync.sent {
                    tracing::info!(
                        "[VERBOSE]   sent: dest={:?} ts={:?} has_message={} has_story={} edit_message={} unidentified_status={}",
                        sent.destination_service_id,
                        sent.timestamp,
                        sent.message.is_some(),
                        sent.story_message.is_some(),
                        sent.edit_message.is_some(),
                        sent.unidentified_status.len(),
                    );
                    if let Some(msg) = &sent.message {
                        tracing::info!(
                            "[VERBOSE]     message: body={:?} attachments={} group={:?}",
                            msg.body.as_ref().map(|b| if b.len() > 30 { format!("{}...", &b[..30]) } else { b.clone() }),
                            msg.attachments.len(),
                            msg.group_v2.is_some(),
                        );
                    }
                }
                
                if let Some(request) = &sync.request {
                    tracing::info!("[VERBOSE]   request: type={:?}", request.r#type);
                }
                
                if let Some(fetch) = &sync.fetch_latest {
                    tracing::info!("[VERBOSE]   fetch_latest: type={:?}", fetch.r#type);
                }
            }
            ContentBody::ReceiptMessage(r) => {
                tracing::debug!("[VERBOSE] ReceiptMessage from={} type={:?} timestamps={:?}", sender, r.r#type, r.timestamp);
            }
            ContentBody::TypingMessage(t) => {
                tracing::debug!("[VERBOSE] TypingMessage from={} action={:?}", sender, t.action);
            }
            ContentBody::CallMessage(_) => {
                tracing::debug!("[VERBOSE] CallMessage from={}", sender);
            }
            ContentBody::NullMessage(_) => {
                tracing::debug!("[VERBOSE] NullMessage from={}", sender);
            }
            ContentBody::StoryMessage(_) => {
                tracing::debug!("[VERBOSE] StoryMessage from={}", sender);
            }
            ContentBody::PniSignatureMessage(_) => {
                tracing::debug!("[VERBOSE] PniSignatureMessage from={}", sender);
            }
            ContentBody::EditMessage(_) => {
                tracing::debug!("[VERBOSE] EditMessage from={}", sender);
            }
        }
    }

    fn process_content(content: &Content) -> Option<IncomingMessage> {
        use presage::libsignal_service::content::ContentBody;

        let sender = format!("{:?}", content.metadata.sender);
        let timestamp = content.metadata.timestamp as i64;

        match &content.body {
            ContentBody::DataMessage(data_msg) => {
                Self::process_data_message(data_msg, &sender, timestamp)
            }
            ContentBody::SynchronizeMessage(sync_msg) => {
                Self::process_sync_message(sync_msg, &sender, timestamp)
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

    fn process_data_message(
        data_msg: &DataMessage,
        sender: &str,
        timestamp: i64,
    ) -> Option<IncomingMessage> {
        let text = data_msg.body.clone().unwrap_or_default();
        if text.is_empty() && data_msg.attachments.is_empty() {
            return None;
        }

        let conversation_id = if let Some(group) = &data_msg.group_v2 {
            if let Some(master_key) = &group.master_key {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(master_key)
            } else {
                sender.to_string()
            }
        } else {
            sender.to_string()
        };

        Some(IncomingMessage {
            id: uuid::Uuid::new_v4().to_string(),
            sender: sender.to_string(),
            conversation_id,
            content: MessageContent::Text(text),
            timestamp,
            server_timestamp: timestamp,
        })
    }

    fn process_sync_message(
        sync_msg: &presage::libsignal_service::proto::SyncMessage,
        _sender: &str,
        timestamp: i64,
    ) -> Option<IncomingMessage> {
        if let Some(sent) = &sync_msg.sent {
            if let Some(data_msg) = &sent.message {
                let text = data_msg.body.clone().unwrap_or_default();
                if text.is_empty() && data_msg.attachments.is_empty() {
                    return None;
                }

                let conversation_id = if let Some(group) = &data_msg.group_v2 {
                    if let Some(master_key) = &group.master_key {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(master_key)
                    } else if let Some(dest) = &sent.destination_service_id {
                        dest.clone()
                    } else {
                        return None;
                    }
                } else if let Some(dest) = &sent.destination_service_id {
                    dest.clone()
                } else {
                    return None;
                };

                let msg_timestamp = sent.timestamp.unwrap_or(timestamp as u64) as i64;

                tracing::info!(
                    "Received sync of sent message to {} at {}",
                    conversation_id,
                    msg_timestamp
                );

                return Some(IncomingMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    sender: "self".to_string(),
                    conversation_id,
                    content: MessageContent::Text(text),
                    timestamp: msg_timestamp,
                    server_timestamp: timestamp,
                });
            }
        }

        tracing::debug!("Received sync message without sent content");
        None
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
        
        let event_tx = self.event_tx.clone();
        let text = text.to_string();
        let msg_id = message_id.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for sending");
            
            rt.block_on(async move {
                match Self::send_via_channel(SendCommand::DirectMessage {
                    recipient: recipient_uuid,
                    text,
                    reply: oneshot::channel().0,
                }).await {
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
        
        let event_tx = self.event_tx.clone();
        let text = text.to_string();
        let msg_id = message_id.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for sending");
            
            rt.block_on(async move {
                match Self::send_via_channel(SendCommand::GroupMessage {
                    group_key: master_key,
                    text,
                    reply: oneshot::channel().0,
                }).await {
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
        _storage: &Arc<Storage>,
        recipient: &str,
        text: &str,
    ) -> Result<(), SignalError> {
        let recipient_uuid = Uuid::parse_str(recipient)
            .map_err(|e| SignalError::SendFailed(format!("Invalid recipient UUID: {}", e)))?;
        
        Self::send_via_channel(SendCommand::DirectMessage {
            recipient: recipient_uuid,
            text: text.to_string(),
            reply: oneshot::channel().0,
        }).await
    }
    
    pub async fn send_group_message_static(
        _storage: &Arc<Storage>,
        group_id: &str,
        text: &str,
    ) -> Result<(), SignalError> {
        let master_key = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            group_id,
        ).map_err(|e| SignalError::SendFailed(format!("Invalid group ID: {}", e)))?;
        
        Self::send_via_channel(SendCommand::GroupMessage {
            group_key: master_key,
            text: text.to_string(),
            reply: oneshot::channel().0,
        }).await
    }
    
    async fn send_via_channel(mut cmd: SendCommand) -> Result<(), SignalError> {
        let (tx, rx) = oneshot::channel();
        
        match &mut cmd {
            SendCommand::DirectMessage { reply, .. } => *reply = tx,
            SendCommand::GroupMessage { reply, .. } => *reply = tx,
        }
        
        let send_tx = {
            let guard = SEND_TX.lock();
            guard.clone()
        };
        
        let send_tx = send_tx.ok_or_else(|| {
            SignalError::SendFailed("Not connected - receive loop not running".to_string())
        })?;
        
        send_tx.send(cmd).map_err(|_| {
            SignalError::SendFailed("Send channel closed".to_string())
        })?;
        
        rx.await.map_err(|_| {
            SignalError::SendFailed("Response channel closed".to_string())
        })?
    }
}
