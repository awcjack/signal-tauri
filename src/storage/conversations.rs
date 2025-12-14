//! Conversation storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Conversation type
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConversationType {
    /// 1:1 private conversation
    Private,
    /// Group conversation
    Group,
    /// Note to self
    NoteToSelf,
}

/// A conversation (thread)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique conversation ID (UUID for private, group ID for groups)
    pub id: String,

    /// Conversation type
    pub conversation_type: ConversationType,

    /// Display name
    pub name: String,

    /// Avatar path (if custom)
    pub avatar_path: Option<String>,

    /// Last message preview
    pub last_message: Option<String>,

    /// Last message timestamp
    pub last_message_at: Option<DateTime<Utc>>,

    /// Unread message count
    pub unread_count: u32,

    /// Whether the conversation is pinned
    pub is_pinned: bool,

    /// Whether notifications are muted
    pub is_muted: bool,

    /// Mute expiration time (None = muted forever)
    pub muted_until: Option<DateTime<Utc>>,

    /// Whether the conversation is archived
    pub is_archived: bool,

    /// Whether the conversation is blocked
    pub is_blocked: bool,

    /// Disappearing messages timer (0 = disabled)
    pub disappearing_messages_timer: u32,

    /// Draft message (if any)
    pub draft: Option<String>,

    /// When the conversation was created
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
    /// Create a new private conversation
    pub fn new_private(id: &str, name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            conversation_type: ConversationType::Private,
            name: name.to_string(),
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

    /// Create a new group conversation
    pub fn new_group(id: &str, name: &str) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            conversation_type: ConversationType::Group,
            name: name.to_string(),
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

    /// Check if notifications are currently muted
    pub fn is_currently_muted(&self) -> bool {
        if !self.is_muted {
            return false;
        }

        match self.muted_until {
            None => true, // Muted forever
            Some(until) => Utc::now() < until,
        }
    }

    /// Get initials for avatar
    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .take(2)
            .map(|word| word.chars().next().unwrap_or('?'))
            .collect::<String>()
            .to_uppercase()
    }

    /// Update last message
    pub fn update_last_message(&mut self, message: &str, timestamp: DateTime<Utc>) {
        self.last_message = Some(message.to_string());
        self.last_message_at = Some(timestamp);
        self.updated_at = Utc::now();
    }

    /// Increment unread count
    pub fn increment_unread(&mut self) {
        self.unread_count += 1;
        self.updated_at = Utc::now();
    }

    /// Mark as read
    pub fn mark_read(&mut self) {
        self.unread_count = 0;
        self.updated_at = Utc::now();
    }
}

/// Conversation repository
pub struct ConversationRepository {
    // TODO: Add database connection
}

impl ConversationRepository {
    /// Create a new repository
    pub fn new() -> Self {
        Self {}
    }

    /// Get a conversation by ID
    pub async fn get(&self, id: &str) -> Option<Conversation> {
        // TODO: Implement database lookup
        None
    }

    /// Save a conversation
    pub async fn save(&self, conversation: &Conversation) -> anyhow::Result<()> {
        // TODO: Implement database save
        Ok(())
    }

    /// Get all conversations
    pub async fn list(&self) -> Vec<Conversation> {
        // TODO: Implement database list
        Vec::new()
    }

    /// Get active conversations (not archived)
    pub async fn list_active(&self) -> Vec<Conversation> {
        // TODO: Implement database list with filter
        Vec::new()
    }

    /// Get archived conversations
    pub async fn list_archived(&self) -> Vec<Conversation> {
        // TODO: Implement database list with filter
        Vec::new()
    }

    /// Get pinned conversations
    pub async fn list_pinned(&self) -> Vec<Conversation> {
        // TODO: Implement database list with filter
        Vec::new()
    }

    /// Search conversations by name
    pub async fn search(&self, query: &str) -> Vec<Conversation> {
        // TODO: Implement search
        Vec::new()
    }

    /// Delete a conversation and all its messages
    pub async fn delete(&self, id: &str) -> anyhow::Result<()> {
        // TODO: Implement delete
        Ok(())
    }
}

impl Default for ConversationRepository {
    fn default() -> Self {
        Self::new()
    }
}
