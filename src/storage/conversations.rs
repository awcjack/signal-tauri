use crate::storage::database::Database;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConversationType {
    Private,
    Group,
    NoteToSelf,
}

impl ConversationType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Group => "group",
            Self::NoteToSelf => "note_to_self",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "group" => Self::Group,
            "note_to_self" => Self::NoteToSelf,
            _ => Self::Private,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub conversation_type: ConversationType,
    pub name: String,
    pub avatar_path: Option<String>,
    pub last_message: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: u32,
    pub is_pinned: bool,
    pub is_muted: bool,
    pub muted_until: Option<DateTime<Utc>>,
    pub is_archived: bool,
    pub is_blocked: bool,
    pub disappearing_messages_timer: u32,
    pub draft: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Conversation {
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

    pub fn is_currently_muted(&self) -> bool {
        if !self.is_muted {
            return false;
        }
        match self.muted_until {
            None => true,
            Some(until) => Utc::now() < until,
        }
    }

    pub fn initials(&self) -> String {
        self.name
            .split_whitespace()
            .take(2)
            .map(|word| word.chars().next().unwrap_or('?'))
            .collect::<String>()
            .to_uppercase()
    }

    pub fn update_last_message(&mut self, message: &str, timestamp: DateTime<Utc>) {
        self.last_message = Some(message.to_string());
        self.last_message_at = Some(timestamp);
        self.updated_at = Utc::now();
    }

    pub fn increment_unread(&mut self) {
        self.unread_count += 1;
        self.updated_at = Utc::now();
    }

    pub fn mark_read(&mut self) {
        self.unread_count = 0;
        self.updated_at = Utc::now();
    }
}

pub struct ConversationRepository<'a> {
    db: &'a Database,
}

impl<'a> ConversationRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn get(&self, id: &str) -> Option<Conversation> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();
        
        conn.query_row(
            "SELECT id, conversation_type, name, avatar_path, last_message, 
                    last_message_at, unread_count, is_pinned, is_muted, muted_until,
                    is_archived, is_blocked, disappearing_timer, draft, created_at, updated_at
             FROM conversations WHERE id = ?",
            params![id],
            |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    conversation_type: ConversationType::from_str(&row.get::<_, String>(1)?),
                    name: row.get(2)?,
                    avatar_path: row.get(3)?,
                    last_message: row.get(4)?,
                    last_message_at: row.get::<_, Option<i64>>(5)?.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
                    unread_count: row.get::<_, i64>(6)? as u32,
                    is_pinned: row.get::<_, i64>(7)? != 0,
                    is_muted: row.get::<_, i64>(8)? != 0,
                    muted_until: row.get::<_, Option<i64>>(9)?.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
                    is_archived: row.get::<_, i64>(10)? != 0,
                    is_blocked: row.get::<_, i64>(11)? != 0,
                    disappearing_messages_timer: row.get::<_, i64>(12)? as u32,
                    draft: row.get(13)?,
                    created_at: Utc.timestamp_opt(row.get::<_, i64>(14)?, 0).unwrap(),
                    updated_at: Utc.timestamp_opt(row.get::<_, i64>(15)?, 0).unwrap(),
                })
            },
        ).ok()
    }

    pub fn save(&self, conv: &Conversation) -> anyhow::Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();
        
        conn.execute(
            "INSERT OR REPLACE INTO conversations 
             (id, conversation_type, name, avatar_path, last_message, last_message_at,
              unread_count, is_pinned, is_muted, muted_until, is_archived, is_blocked,
              disappearing_timer, draft, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                conv.id,
                conv.conversation_type.as_str(),
                conv.name,
                conv.avatar_path,
                conv.last_message,
                conv.last_message_at.map(|t| t.timestamp()),
                conv.unread_count as i64,
                conv.is_pinned as i64,
                conv.is_muted as i64,
                conv.muted_until.map(|t| t.timestamp()),
                conv.is_archived as i64,
                conv.is_blocked as i64,
                conv.disappearing_messages_timer as i64,
                conv.draft,
                conv.created_at.timestamp(),
                conv.updated_at.timestamp(),
            ],
        )?;
        Ok(())
    }

    pub fn list(&self) -> Vec<Conversation> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();
        
        let mut stmt = match conn.prepare(
            "SELECT id, conversation_type, name, avatar_path, last_message, 
                    last_message_at, unread_count, is_pinned, is_muted, muted_until,
                    is_archived, is_blocked, disappearing_timer, draft, created_at, updated_at
             FROM conversations 
             ORDER BY is_pinned DESC, updated_at DESC"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        stmt.query_map([], |row| {
            Ok(Conversation {
                id: row.get(0)?,
                conversation_type: ConversationType::from_str(&row.get::<_, String>(1)?),
                name: row.get(2)?,
                avatar_path: row.get(3)?,
                last_message: row.get(4)?,
                last_message_at: row.get::<_, Option<i64>>(5)?.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
                unread_count: row.get::<_, i64>(6)? as u32,
                is_pinned: row.get::<_, i64>(7)? != 0,
                is_muted: row.get::<_, i64>(8)? != 0,
                muted_until: row.get::<_, Option<i64>>(9)?.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
                is_archived: row.get::<_, i64>(10)? != 0,
                is_blocked: row.get::<_, i64>(11)? != 0,
                disappearing_messages_timer: row.get::<_, i64>(12)? as u32,
                draft: row.get(13)?,
                created_at: Utc.timestamp_opt(row.get::<_, i64>(14)?, 0).unwrap(),
                updated_at: Utc.timestamp_opt(row.get::<_, i64>(15)?, 0).unwrap(),
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    pub fn list_active(&self) -> Vec<Conversation> {
        self.list()
            .into_iter()
            .filter(|c| !c.is_archived && c.last_message.is_some())
            .collect()
    }

    pub fn list_archived(&self) -> Vec<Conversation> {
        self.list().into_iter().filter(|c| c.is_archived).collect()
    }

    pub fn delete(&self, id: &str) -> anyhow::Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();
        conn.execute("DELETE FROM conversations WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn update_unread(&self, id: &str, count: u32) -> anyhow::Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();
        conn.execute(
            "UPDATE conversations SET unread_count = ?, updated_at = ? WHERE id = ?",
            params![count as i64, Utc::now().timestamp(), id],
        )?;
        Ok(())
    }
}
