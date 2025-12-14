//! Message handling and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Message direction
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MessageDirection {
    Incoming,
    Outgoing,
}

/// Message status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MessageStatus {
    /// Message is being sent
    Sending,
    /// Message was sent to server
    Sent,
    /// Message was delivered to recipient
    Delivered,
    /// Message was read by recipient
    Read,
    /// Message failed to send
    Failed,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: String,

    /// Conversation this message belongs to
    pub conversation_id: String,

    /// Sender's identifier (phone number or UUID)
    pub sender: String,

    /// Message direction
    pub direction: MessageDirection,

    /// Message status
    pub status: MessageStatus,

    /// Message content
    pub content: Content,

    /// When the message was sent (client timestamp)
    pub sent_at: DateTime<Utc>,

    /// When the message was received by server
    pub server_timestamp: Option<DateTime<Utc>>,

    /// When the message was delivered
    pub delivered_at: Option<DateTime<Utc>>,

    /// When the message was read
    pub read_at: Option<DateTime<Utc>>,

    /// Message this is replying to
    pub quote: Option<Quote>,

    /// Reactions on this message
    pub reactions: Vec<Reaction>,

    /// Whether this is a disappearing message
    pub expires_in_seconds: Option<u32>,

    /// When this message expires
    pub expires_at: Option<DateTime<Utc>>,
}

/// Message content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    /// Text message
    Text {
        body: String,
        /// Parsed mentions
        mentions: Vec<Mention>,
    },

    /// Image attachment
    Image {
        attachment_id: String,
        content_type: String,
        width: u32,
        height: u32,
        size: u64,
        caption: Option<String>,
        blurhash: Option<String>,
    },

    /// Video attachment
    Video {
        attachment_id: String,
        content_type: String,
        width: u32,
        height: u32,
        duration_ms: u64,
        size: u64,
        caption: Option<String>,
        thumbnail_id: Option<String>,
    },

    /// Audio/voice note
    Audio {
        attachment_id: String,
        content_type: String,
        duration_ms: u64,
        size: u64,
        /// Voice note waveform for visualization
        waveform: Option<Vec<u8>>,
    },

    /// Generic file attachment
    File {
        attachment_id: String,
        content_type: String,
        filename: String,
        size: u64,
    },

    /// Sticker
    Sticker {
        pack_id: String,
        pack_key: String,
        sticker_id: u32,
        emoji: Option<String>,
    },

    /// Contact card
    Contact {
        name: String,
        phone_numbers: Vec<String>,
        email: Option<String>,
    },

    /// Location
    Location {
        latitude: f64,
        longitude: f64,
        name: Option<String>,
        address: Option<String>,
    },

    /// Group update message
    GroupUpdate {
        update_type: GroupUpdateType,
        details: String,
    },

    /// Profile key update
    ProfileKeyUpdate,

    /// End session message
    EndSession,
}

/// Group update types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupUpdateType {
    Created,
    NameChanged,
    AvatarChanged,
    MembersAdded,
    MembersRemoved,
    MemberJoined,
    MemberLeft,
    AdminsChanged,
    DescriptionChanged,
    DisappearingMessagesChanged,
}

/// A mention in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    /// Start position in text
    pub start: usize,
    /// Length of mention
    pub length: usize,
    /// UUID of mentioned user
    pub uuid: String,
}

/// A quoted message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    /// ID of the quoted message
    pub message_id: String,
    /// Author of the quoted message
    pub author: String,
    /// Text preview of the quoted message
    pub text: Option<String>,
    /// Attachment preview
    pub attachment_preview: Option<AttachmentPreview>,
}

/// Attachment preview for quotes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentPreview {
    pub content_type: String,
    pub filename: Option<String>,
    pub thumbnail_id: Option<String>,
}

/// A reaction on a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    /// Emoji used for reaction
    pub emoji: String,
    /// Who sent this reaction
    pub sender: String,
    /// When the reaction was sent
    pub timestamp: DateTime<Utc>,
}

impl Message {
    /// Create a new outgoing text message
    pub fn new_text(conversation_id: &str, sender: &str, body: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            sender: sender.to_string(),
            direction: MessageDirection::Outgoing,
            status: MessageStatus::Sending,
            content: Content::Text {
                body: body.to_string(),
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
        }
    }

    /// Check if message has any attachments
    pub fn has_attachment(&self) -> bool {
        matches!(
            self.content,
            Content::Image { .. }
                | Content::Video { .. }
                | Content::Audio { .. }
                | Content::File { .. }
        )
    }

    /// Get text content if available
    pub fn text(&self) -> Option<&str> {
        match &self.content {
            Content::Text { body, .. } => Some(body),
            Content::Image { caption, .. } | Content::Video { caption, .. } => caption.as_deref(),
            _ => None,
        }
    }

    /// Add a reaction
    pub fn add_reaction(&mut self, emoji: &str, sender: &str) {
        // Remove existing reaction from same sender
        self.reactions.retain(|r| r.sender != sender);

        self.reactions.push(Reaction {
            emoji: emoji.to_string(),
            sender: sender.to_string(),
            timestamp: Utc::now(),
        });
    }

    /// Remove a reaction
    pub fn remove_reaction(&mut self, sender: &str) {
        self.reactions.retain(|r| r.sender != sender);
    }
}
