mod api;
mod crypto;

pub use api::{TransferArchiveInfo, fetch_transfer_archive, download_backup};
pub use crypto::decrypt_backup;

use crate::signal::messages::{Content, Message, MessageDirection, MessageStatus};
use crate::signal::SignalError;
use crate::storage::conversations::{Conversation, ConversationType, ConversationRepository};
use crate::storage::messages::MessageRepository;
use crate::storage::Storage;
use chrono::{TimeZone, Utc};
use flate2::read::GzDecoder;
use std::io::Read;
use std::sync::Arc;

pub struct BackupData {
    pub messages: Vec<BackupMessage>,
    pub conversations: Vec<BackupConversation>,
    pub frame_count: usize,
}

#[derive(Debug, Clone)]
pub struct BackupMessage {
    pub id: String,
    pub conversation_id: String,
    pub sender_uuid: String,
    pub body: Option<String>,
    pub timestamp: i64,
    pub is_outgoing: bool,
}

#[derive(Debug, Clone)]
pub struct BackupConversation {
    pub id: String,
    pub recipient_uuid: Option<String>,
    pub group_id: Option<Vec<u8>>,
    pub name: Option<String>,
}

pub async fn sync_message_history(
    ephemeral_backup_key: &[u8],
    aci: &uuid::Uuid,
    auth_username: &str,
    auth_password: &str,
) -> Result<BackupData, SignalError> {
    tracing::info!("Starting message history sync...");
    
    let archive_info = fetch_transfer_archive(auth_username, auth_password).await?;
    tracing::info!("Transfer archive located at CDN {}", archive_info.cdn);
    
    let encrypted_backup = download_backup(&archive_info).await?;
    tracing::info!("Downloaded {} bytes of encrypted backup", encrypted_backup.len());
    
    let decrypted = decrypt_backup(&encrypted_backup, ephemeral_backup_key, aci)?;
    tracing::info!("Decrypted {} bytes of backup data", decrypted.len());
    
    parse_backup(&decrypted)
}

/// Import backup data into local storage
/// 
/// Converts parsed backup conversations and messages into the app's storage format
/// and saves them to the local database.
pub fn import_backup_data(
    backup_data: &BackupData,
    storage: &Arc<Storage>,
) -> Result<(usize, usize), SignalError> {
    let db = storage.database().ok_or_else(|| {
        SignalError::StorageError("Database not available for backup import".to_string())
    })?;

    let conv_repo = ConversationRepository::new(&db);
    let msg_repo = MessageRepository::new(&db);

    let mut conversations_imported = 0;
    let mut messages_imported = 0;

    let conv_map: std::collections::HashMap<String, &BackupConversation> = backup_data
        .conversations
        .iter()
        .map(|c| (c.id.clone(), c))
        .collect();

    for backup_conv in &backup_data.conversations {
        let conversation = convert_backup_conversation(backup_conv);
        
        if let Err(e) = conv_repo.save(&conversation) {
            tracing::warn!("Failed to save conversation {}: {}", backup_conv.id, e);
            continue;
        }
        
        conversations_imported += 1;
        tracing::debug!(
            "Imported conversation: {} ({})",
            conversation.name,
            conversation.id
        );
    }

    for backup_msg in &backup_data.messages {
        if backup_msg.body.is_none() {
            continue;
        }

        let conv_info = conv_map.get(&backup_msg.conversation_id);
        
        let message = convert_backup_message(backup_msg, conv_info);
        
        if let Err(e) = msg_repo.save(&message) {
            tracing::warn!("Failed to save message {}: {}", backup_msg.id, e);
            continue;
        }
        
        messages_imported += 1;
    }

    tracing::info!(
        "Backup import complete: {} conversations, {} messages",
        conversations_imported,
        messages_imported
    );

    Ok((conversations_imported, messages_imported))
}

/// Convert a BackupConversation to a storage Conversation
fn convert_backup_conversation(backup: &BackupConversation) -> Conversation {
    let now = Utc::now();
    
    let (conv_type, id) = if backup.group_id.is_some() {
        let group_id = backup.group_id.as_ref().map(|g| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(g)
        }).unwrap_or_else(|| backup.id.clone());
        
        (ConversationType::Group, group_id)
    } else if let Some(ref uuid) = backup.recipient_uuid {
        (ConversationType::Private, uuid.clone())
    } else {
        (ConversationType::Private, backup.id.clone())
    };

    let name = backup.name.clone().unwrap_or_else(|| {
        if conv_type == ConversationType::Group {
            "Group".to_string()
        } else {
            backup.recipient_uuid.clone().unwrap_or_else(|| "Unknown".to_string())
        }
    });

    Conversation {
        id,
        conversation_type: conv_type,
        name,
        avatar_path: None,
        last_message: None,
        last_message_at: None,
        unread_count: 0,
        is_pinned: false,
        is_muted: false,
        muted_until: None,
        is_archived: false,
        is_blocked: false,
        disappearing_messages_timer: 0,
        draft: None,
        created_at: now,
        updated_at: now,
    }
}

/// Convert a BackupMessage to a storage Message
fn convert_backup_message(
    backup: &BackupMessage,
    conv_info: Option<&&BackupConversation>,
) -> Message {
    let conversation_id = if let Some(conv) = conv_info {
        if conv.group_id.is_some() {
            conv.group_id.as_ref().map(|g| {
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(g)
            }).unwrap_or_else(|| backup.conversation_id.clone())
        } else if let Some(ref uuid) = conv.recipient_uuid {
            uuid.clone()
        } else {
            backup.conversation_id.clone()
        }
    } else {
        backup.conversation_id.clone()
    };

    let sender = if backup.is_outgoing {
        "self".to_string()
    } else {
        backup.sender_uuid.clone()
    };

    let direction = if backup.is_outgoing {
        MessageDirection::Outgoing
    } else {
        MessageDirection::Incoming
    };

    let sent_at = Utc.timestamp_millis_opt(backup.timestamp).single()
        .unwrap_or_else(|| Utc.timestamp_opt(backup.timestamp, 0).single().unwrap_or_else(Utc::now));

    Message {
        id: backup.id.clone(),
        conversation_id,
        sender,
        direction,
        status: MessageStatus::Read,
        content: Content::Text {
            body: backup.body.clone().unwrap_or_default(),
            mentions: Vec::new(),
        },
        sent_at,
        server_timestamp: Some(sent_at),
        delivered_at: Some(sent_at),
        read_at: Some(sent_at),
        quote: None,
        reactions: Vec::new(),
        expires_in_seconds: None,
        expires_at: None,
    }
}

fn decompress_backup(data: &[u8]) -> Result<Vec<u8>, SignalError> {
    let mut decoder = GzDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| SignalError::ProtocolError(format!("Gzip decompression failed: {}", e)))?;
    Ok(decompressed)
}

fn read_varint(data: &[u8], offset: &mut usize) -> Option<u64> {
    let mut result: u64 = 0;
    let mut shift = 0;
    
    while *offset < data.len() {
        let byte = data[*offset];
        *offset += 1;
        
        result |= ((byte & 0x7F) as u64) << shift;
        
        if byte & 0x80 == 0 {
            return Some(result);
        }
        
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
    None
}

fn parse_backup(data: &[u8]) -> Result<BackupData, SignalError> {
    tracing::info!("Parsing backup data ({} bytes)...", data.len());
    
    let decompressed = decompress_backup(data)?;
    tracing::info!("Decompressed to {} bytes", decompressed.len());
    
    let mut messages = Vec::new();
    let mut conversations = Vec::new();
    let mut offset = 0;
    let mut frame_count = 0;
    
    while offset < decompressed.len() {
        let frame_len = match read_varint(&decompressed, &mut offset) {
            Some(len) => len as usize,
            None => break,
        };
        
        if offset + frame_len > decompressed.len() {
            tracing::warn!("Frame extends beyond data boundary, stopping");
            break;
        }
        
        let frame_data = &decompressed[offset..offset + frame_len];
        offset += frame_len;
        frame_count += 1;
        
        if let Err(e) = parse_frame(frame_data, &mut messages, &mut conversations) {
            tracing::debug!("Frame {} parse error (non-fatal): {}", frame_count, e);
        }
    }
    
    tracing::info!(
        "Parsed {} frames: {} conversations, {} messages",
        frame_count,
        conversations.len(),
        messages.len()
    );
    
    Ok(BackupData {
        messages,
        conversations,
        frame_count,
    })
}

fn parse_frame(
    data: &[u8],
    messages: &mut Vec<BackupMessage>,
    conversations: &mut Vec<BackupConversation>,
) -> Result<(), SignalError> {
    let mut field_offset = 0;
    
    while field_offset < data.len() {
        let tag_byte = data[field_offset];
        let wire_type = tag_byte & 0x07;
        let field_number = tag_byte >> 3;
        field_offset += 1;
        
        match (field_number, wire_type) {
            (2, 2) => {
                if let Some(len) = read_varint(data, &mut field_offset) {
                    let end = field_offset + len as usize;
                    if end <= data.len() {
                        if let Some(conv) = parse_recipient(&data[field_offset..end]) {
                            conversations.push(conv);
                        }
                        field_offset = end;
                    }
                }
            }
            (4, 2) => {
                if let Some(len) = read_varint(data, &mut field_offset) {
                    let end = field_offset + len as usize;
                    if end <= data.len() {
                        if let Some(msg) = parse_chat_item(&data[field_offset..end]) {
                            messages.push(msg);
                        }
                        field_offset = end;
                    }
                }
            }
            (_, 0) => { read_varint(data, &mut field_offset); }
            (_, 1) => { field_offset += 8; }
            (_, 2) => {
                if let Some(len) = read_varint(data, &mut field_offset) {
                    field_offset += len as usize;
                }
            }
            (_, 5) => { field_offset += 4; }
            _ => break,
        }
    }
    
    Ok(())
}

fn parse_recipient(data: &[u8]) -> Option<BackupConversation> {
    let mut id: Option<u64> = None;
    let mut recipient_uuid: Option<String> = None;
    let mut name: Option<String> = None;
    let mut group_id: Option<Vec<u8>> = None;
    let mut offset = 0;
    
    while offset < data.len() {
        let tag_byte = data[offset];
        let wire_type = tag_byte & 0x07;
        let field_number = tag_byte >> 3;
        offset += 1;
        
        match (field_number, wire_type) {
            (1, 0) => {
                id = read_varint(data, &mut offset);
            }
            (2, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() {
                        if let Some((uuid, contact_name)) = parse_contact(&data[offset..end]) {
                            recipient_uuid = uuid;
                            name = contact_name;
                        }
                        offset = end;
                    }
                }
            }
            (3, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() {
                        if let Some((gid, group_name)) = parse_group(&data[offset..end]) {
                            group_id = Some(gid);
                            name = group_name;
                        }
                        offset = end;
                    }
                }
            }
            (_, 0) => { read_varint(data, &mut offset); }
            (_, 1) => { offset += 8; }
            (_, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    offset += len as usize;
                }
            }
            (_, 5) => { offset += 4; }
            _ => break,
        }
    }
    
    id.map(|i| BackupConversation {
        id: i.to_string(),
        recipient_uuid,
        group_id,
        name,
    })
}

fn parse_contact(data: &[u8]) -> Option<(Option<String>, Option<String>)> {
    let mut aci: Option<Vec<u8>> = None;
    let mut profile_given_name: Option<String> = None;
    let mut offset = 0;
    
    while offset < data.len() {
        let tag_byte = data[offset];
        let wire_type = tag_byte & 0x07;
        let field_number = tag_byte >> 3;
        offset += 1;
        
        match (field_number, wire_type) {
            (1, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() && len == 16 {
                        aci = Some(data[offset..end].to_vec());
                    }
                    offset = end.min(data.len());
                }
            }
            (11, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() {
                        profile_given_name = String::from_utf8(data[offset..end].to_vec()).ok();
                    }
                    offset = end.min(data.len());
                }
            }
            (_, 0) => { read_varint(data, &mut offset); }
            (_, 1) => { offset = (offset + 8).min(data.len()); }
            (_, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    offset = (offset + len as usize).min(data.len());
                }
            }
            (_, 5) => { offset = (offset + 4).min(data.len()); }
            _ => break,
        }
    }
    
    let uuid_str = aci.map(|bytes| {
        uuid::Uuid::from_slice(&bytes)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| hex::encode(&bytes))
    });
    
    Some((uuid_str, profile_given_name))
}

fn parse_group(data: &[u8]) -> Option<(Vec<u8>, Option<String>)> {
    let mut master_key: Option<Vec<u8>> = None;
    let mut offset = 0;
    
    while offset < data.len() {
        let tag_byte = data[offset];
        let wire_type = tag_byte & 0x07;
        let field_number = tag_byte >> 3;
        offset += 1;
        
        match (field_number, wire_type) {
            (1, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() {
                        master_key = Some(data[offset..end].to_vec());
                    }
                    offset = end.min(data.len());
                }
            }
            (_, 0) => { read_varint(data, &mut offset); }
            (_, 1) => { offset = (offset + 8).min(data.len()); }
            (_, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    offset = (offset + len as usize).min(data.len());
                }
            }
            (_, 5) => { offset = (offset + 4).min(data.len()); }
            _ => break,
        }
    }
    
    master_key.map(|k| (k, None))
}

fn parse_chat_item(data: &[u8]) -> Option<BackupMessage> {
    let mut chat_id: Option<u64> = None;
    let mut author_id: Option<u64> = None;
    let mut date_sent: Option<u64> = None;
    let mut body: Option<String> = None;
    let mut is_outgoing = false;
    let mut offset = 0;
    
    while offset < data.len() {
        let tag_byte = data[offset];
        let wire_type = tag_byte & 0x07;
        let field_number = tag_byte >> 3;
        offset += 1;
        
        match (field_number, wire_type) {
            (1, 0) => { chat_id = read_varint(data, &mut offset); }
            (2, 0) => { author_id = read_varint(data, &mut offset); }
            (3, 0) => { date_sent = read_varint(data, &mut offset); }
            (9, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() {
                        is_outgoing = true;
                    }
                    offset = end.min(data.len());
                }
            }
            (11, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() {
                        body = parse_standard_message(&data[offset..end]);
                    }
                    offset = end.min(data.len());
                }
            }
            (_, 0) => { read_varint(data, &mut offset); }
            (_, 1) => { offset = (offset + 8).min(data.len()); }
            (_, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    offset = (offset + len as usize).min(data.len());
                }
            }
            (_, 5) => { offset = (offset + 4).min(data.len()); }
            _ => break,
        }
    }
    
    Some(BackupMessage {
        id: date_sent.map(|d| d.to_string()).unwrap_or_default(),
        conversation_id: chat_id.map(|c| c.to_string()).unwrap_or_default(),
        sender_uuid: author_id.map(|a| a.to_string()).unwrap_or_default(),
        body,
        timestamp: date_sent.map(|d| d as i64).unwrap_or(0),
        is_outgoing,
    })
}

fn parse_standard_message(data: &[u8]) -> Option<String> {
    let mut offset = 0;
    
    while offset < data.len() {
        let tag_byte = data[offset];
        let wire_type = tag_byte & 0x07;
        let field_number = tag_byte >> 3;
        offset += 1;
        
        match (field_number, wire_type) {
            (2, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() {
                        return parse_text(&data[offset..end]);
                    }
                }
            }
            (_, 0) => { read_varint(data, &mut offset); }
            (_, 1) => { offset = (offset + 8).min(data.len()); }
            (_, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    offset = (offset + len as usize).min(data.len());
                }
            }
            (_, 5) => { offset = (offset + 4).min(data.len()); }
            _ => break,
        }
    }
    None
}

fn parse_text(data: &[u8]) -> Option<String> {
    let mut offset = 0;
    
    while offset < data.len() {
        let tag_byte = data[offset];
        let wire_type = tag_byte & 0x07;
        let field_number = tag_byte >> 3;
        offset += 1;
        
        match (field_number, wire_type) {
            (1, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    let end = offset + len as usize;
                    if end <= data.len() {
                        return String::from_utf8(data[offset..end].to_vec()).ok();
                    }
                }
            }
            (_, 0) => { read_varint(data, &mut offset); }
            (_, 1) => { offset = (offset + 8).min(data.len()); }
            (_, 2) => {
                if let Some(len) = read_varint(data, &mut offset) {
                    offset = (offset + len as usize).min(data.len());
                }
            }
            (_, 5) => { offset = (offset + 4).min(data.len()); }
            _ => break,
        }
    }
    None
}
