//! Message storage with SQLite

use crate::signal::messages::{Content, Message, MessageDirection, MessageStatus, Quote, Reaction};
use crate::storage::database::Database;
use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::params;

/// Message repository for database operations
pub struct MessageRepository<'a> {
    db: &'a Database,
}

impl<'a> MessageRepository<'a> {
    /// Create a new repository with database reference
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get a message by ID
    pub fn get(&self, id: &str) -> Option<Message> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.query_row(
            "SELECT id, conversation_id, sender, direction, status, content_type, content_json,
                    sent_at, server_timestamp, delivered_at, read_at, quote_json, reactions_json,
                    expires_in_seconds, expires_at
             FROM messages WHERE id = ?",
            params![id],
            |row| Ok(Self::row_to_message(row)),
        )
        .ok()
        .flatten()
    }

    /// Save a message (insert or update)
    pub fn save(&self, message: &Message) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        let direction = match message.direction {
            MessageDirection::Incoming => "incoming",
            MessageDirection::Outgoing => "outgoing",
        };

        let status = match message.status {
            MessageStatus::Sending => "sending",
            MessageStatus::Sent => "sent",
            MessageStatus::Delivered => "delivered",
            MessageStatus::Read => "read",
            MessageStatus::Failed => "failed",
        };

        let (content_type, content_json) = Self::serialize_content(&message.content);
        let quote_json = message.quote.as_ref().map(|q| serde_json::to_string(q).unwrap_or_default());
        let reactions_json = if message.reactions.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&message.reactions).unwrap_or_default())
        };

        conn.execute(
            "INSERT OR REPLACE INTO messages 
             (id, conversation_id, sender, direction, status, content_type, content_json,
              sent_at, server_timestamp, delivered_at, read_at, quote_json, reactions_json,
              expires_in_seconds, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                message.id,
                message.conversation_id,
                message.sender,
                direction,
                status,
                content_type,
                content_json,
                message.sent_at.timestamp(),
                message.server_timestamp.map(|t| t.timestamp()),
                message.delivered_at.map(|t| t.timestamp()),
                message.read_at.map(|t| t.timestamp()),
                quote_json,
                reactions_json,
                message.expires_in_seconds,
                message.expires_at.map(|t| t.timestamp()),
            ],
        )?;

        Ok(())
    }

    /// Get messages for a conversation with pagination
    pub fn get_for_conversation(
        &self,
        conversation_id: &str,
        limit: usize,
        before: Option<DateTime<Utc>>,
    ) -> Vec<Message> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        let result = if let Some(before_time) = before {
            conn.prepare(
                "SELECT id, conversation_id, sender, direction, status, content_type, content_json,
                        sent_at, server_timestamp, delivered_at, read_at, quote_json, reactions_json,
                        expires_in_seconds, expires_at
                 FROM messages 
                 WHERE conversation_id = ? AND sent_at < ?
                 ORDER BY sent_at DESC
                 LIMIT ?",
            )
            .and_then(|mut stmt| {
                stmt.query_map(
                    params![conversation_id, before_time.timestamp(), limit as i64],
                    |row| Ok(Self::row_to_message(row)),
                )
                .map(|rows| rows.filter_map(|r| r.ok().flatten()).collect())
            })
        } else {
            conn.prepare(
                "SELECT id, conversation_id, sender, direction, status, content_type, content_json,
                        sent_at, server_timestamp, delivered_at, read_at, quote_json, reactions_json,
                        expires_in_seconds, expires_at
                 FROM messages 
                 WHERE conversation_id = ?
                 ORDER BY sent_at DESC
                 LIMIT ?",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![conversation_id, limit as i64], |row| {
                    Ok(Self::row_to_message(row))
                })
                .map(|rows| rows.filter_map(|r| r.ok().flatten()).collect())
            })
        };

        result.unwrap_or_default()
    }

    /// Get unread messages for a conversation
    pub fn get_unread(&self, conversation_id: &str) -> Vec<Message> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.prepare(
            "SELECT id, conversation_id, sender, direction, status, content_type, content_json,
                    sent_at, server_timestamp, delivered_at, read_at, quote_json, reactions_json,
                    expires_in_seconds, expires_at
             FROM messages 
             WHERE conversation_id = ? AND direction = 'incoming' AND read_at IS NULL
             ORDER BY sent_at ASC",
        )
        .and_then(|mut stmt| {
            stmt.query_map(params![conversation_id], |row| {
                Ok(Self::row_to_message(row))
            })
            .map(|rows| rows.filter_map(|r| r.ok().flatten()).collect())
        })
        .unwrap_or_default()
    }

    /// Search messages containing text
    pub fn search(
        &self,
        conversation_id: Option<&str>,
        query: &str,
        limit: usize,
    ) -> Vec<Message> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        let search_pattern = format!("%{}%", query);

        let result = if let Some(conv_id) = conversation_id {
            conn.prepare(
                "SELECT id, conversation_id, sender, direction, status, content_type, content_json,
                        sent_at, server_timestamp, delivered_at, read_at, quote_json, reactions_json,
                        expires_in_seconds, expires_at
                 FROM messages 
                 WHERE conversation_id = ? AND content_json LIKE ?
                 ORDER BY sent_at DESC
                 LIMIT ?",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![conv_id, search_pattern, limit as i64], |row| {
                    Ok(Self::row_to_message(row))
                })
                .map(|rows| rows.filter_map(|r| r.ok().flatten()).collect())
            })
        } else {
            conn.prepare(
                "SELECT id, conversation_id, sender, direction, status, content_type, content_json,
                        sent_at, server_timestamp, delivered_at, read_at, quote_json, reactions_json,
                        expires_in_seconds, expires_at
                 FROM messages 
                 WHERE content_json LIKE ?
                 ORDER BY sent_at DESC
                 LIMIT ?",
            )
            .and_then(|mut stmt| {
                stmt.query_map(params![search_pattern, limit as i64], |row| {
                    Ok(Self::row_to_message(row))
                })
                .map(|rows| rows.filter_map(|r| r.ok().flatten()).collect())
            })
        };

        result.unwrap_or_default()
    }

    /// Update message status
    pub fn update_status(&self, id: &str, status: MessageStatus) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        let status_str = match status {
            MessageStatus::Sending => "sending",
            MessageStatus::Sent => "sent",
            MessageStatus::Delivered => "delivered",
            MessageStatus::Read => "read",
            MessageStatus::Failed => "failed",
        };

        conn.execute(
            "UPDATE messages SET status = ? WHERE id = ?",
            params![status_str, id],
        )?;

        Ok(())
    }

    /// Mark messages as delivered
    pub fn mark_delivered(&self, message_ids: &[String], timestamp: DateTime<Utc>) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        for id in message_ids {
            conn.execute(
                "UPDATE messages SET status = 'delivered', delivered_at = ? WHERE id = ?",
                params![timestamp.timestamp(), id],
            )?;
        }

        Ok(())
    }

    /// Mark messages as read up to a timestamp
    pub fn mark_read(&self, conversation_id: &str, up_to_timestamp: DateTime<Utc>) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        let now = Utc::now().timestamp();

        conn.execute(
            "UPDATE messages 
             SET status = 'read', read_at = ?
             WHERE conversation_id = ? AND sent_at <= ? AND direction = 'incoming' AND read_at IS NULL",
            params![now, conversation_id, up_to_timestamp.timestamp()],
        )?;

        Ok(())
    }

    /// Delete a message
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.execute("DELETE FROM messages WHERE id = ?", params![id])?;

        Ok(())
    }

    /// Delete all messages in a conversation
    pub fn delete_for_conversation(&self, conversation_id: &str) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.execute(
            "DELETE FROM messages WHERE conversation_id = ?",
            params![conversation_id],
        )?;

        Ok(())
    }

    /// Delete expired disappearing messages
    pub fn delete_expired(&self) -> Result<usize> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        let now = Utc::now().timestamp();
        let deleted = conn.execute(
            "DELETE FROM messages WHERE expires_at IS NOT NULL AND expires_at < ?",
            params![now],
        )?;

        Ok(deleted)
    }

    /// Get message count for a conversation
    pub fn count(&self, conversation_id: &str) -> usize {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE conversation_id = ?",
            params![conversation_id],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize
    }

    /// Get total message count
    pub fn total_count(&self) -> usize {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.query_row("SELECT COUNT(*) FROM messages", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
    }

    /// Get the latest message for a conversation
    pub fn get_latest(&self, conversation_id: &str) -> Option<Message> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.query_row(
            "SELECT id, conversation_id, sender, direction, status, content_type, content_json,
                    sent_at, server_timestamp, delivered_at, read_at, quote_json, reactions_json,
                    expires_in_seconds, expires_at
             FROM messages 
             WHERE conversation_id = ?
             ORDER BY sent_at DESC
             LIMIT 1",
            params![conversation_id],
            |row| Ok(Self::row_to_message(row)),
        )
        .ok()
        .flatten()
    }

    fn serialize_content(content: &Content) -> (String, String) {
        let content_type = match content {
            Content::Text { .. } => "text",
            Content::Image { .. } => "image",
            Content::Video { .. } => "video",
            Content::Audio { .. } => "audio",
            Content::File { .. } => "file",
            Content::Sticker { .. } => "sticker",
            Content::Contact { .. } => "contact",
            Content::Location { .. } => "location",
            Content::GroupUpdate { .. } => "group_update",
            Content::ProfileKeyUpdate => "profile_key_update",
            Content::EndSession => "end_session",
        };
        let json = serde_json::to_string(content).unwrap_or_default();
        (content_type.to_string(), json)
    }

    fn row_to_message(row: &rusqlite::Row<'_>) -> Option<Message> {
        let id: String = row.get(0).ok()?;
        let conversation_id: String = row.get(1).ok()?;
        let sender: String = row.get(2).ok()?;

        let direction_str: String = row.get(3).ok()?;
        let direction = match direction_str.as_str() {
            "incoming" => MessageDirection::Incoming,
            "outgoing" => MessageDirection::Outgoing,
            _ => return None,
        };

        let status_str: String = row.get(4).ok()?;
        let status = match status_str.as_str() {
            "sending" => MessageStatus::Sending,
            "sent" => MessageStatus::Sent,
            "delivered" => MessageStatus::Delivered,
            "read" => MessageStatus::Read,
            "failed" => MessageStatus::Failed,
            _ => return None,
        };

        let _content_type: String = row.get(5).ok()?;
        let content_json: String = row.get(6).ok()?;
        let content: Content = serde_json::from_str(&content_json).ok()?;

        let sent_at_ts: i64 = row.get(7).ok()?;
        let sent_at = Utc.timestamp_opt(sent_at_ts, 0).single()?;

        let server_timestamp: Option<DateTime<Utc>> = row
            .get::<_, Option<i64>>(8)
            .ok()
            .flatten()
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single());

        let delivered_at: Option<DateTime<Utc>> = row
            .get::<_, Option<i64>>(9)
            .ok()
            .flatten()
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single());

        let read_at: Option<DateTime<Utc>> = row
            .get::<_, Option<i64>>(10)
            .ok()
            .flatten()
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single());

        let quote: Option<Quote> = row
            .get::<_, Option<String>>(11)
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok());

        let reactions: Vec<Reaction> = row
            .get::<_, Option<String>>(12)
            .ok()
            .flatten()
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let expires_in_seconds: Option<u32> = row.get(13).ok();

        let expires_at: Option<DateTime<Utc>> = row
            .get::<_, Option<i64>>(14)
            .ok()
            .flatten()
            .and_then(|ts| Utc.timestamp_opt(ts, 0).single());

        Some(Message {
            id,
            conversation_id,
            sender,
            direction,
            status,
            content,
            sent_at,
            server_timestamp,
            delivered_at,
            read_at,
            quote,
            reactions,
            expires_in_seconds,
            expires_at,
        })
    }
}

/// Message search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub message: Message,
    pub conversation_name: String,
    pub match_preview: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    fn create_test_db() -> (Database, TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        (db, dir)
    }

    fn create_test_conversation(db: &Database, id: &str) {
        let conn = db.connection();
        let conn = conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            "INSERT INTO conversations (id, conversation_type, name, created_at, updated_at) VALUES (?, 'private', 'Test', ?, ?)",
            params![id, now, now],
        ).unwrap();
    }

    #[test]
    fn test_save_and_get_message() {
        let (db, _dir) = create_test_db();
        create_test_conversation(&db, "conv1");
        let repo = MessageRepository::new(&db);

        let msg = Message::new_text("conv1", "sender1", "Hello World");
        repo.save(&msg).unwrap();

        let retrieved = repo.get(&msg.id).unwrap();
        assert_eq!(retrieved.id, msg.id);
        assert_eq!(retrieved.conversation_id, "conv1");
        assert_eq!(retrieved.text(), Some("Hello World"));
    }

    #[test]
    fn test_get_for_conversation() {
        let (db, _dir) = create_test_db();
        create_test_conversation(&db, "conv1");
        let repo = MessageRepository::new(&db);

        for i in 0..5 {
            let mut msg = Message::new_text("conv1", "sender1", &format!("Message {}", i));
            msg.sent_at = Utc::now() + chrono::Duration::seconds(i);
            repo.save(&msg).unwrap();
        }

        let messages = repo.get_for_conversation("conv1", 10, None);
        assert_eq!(messages.len(), 5);
        assert!(messages[0].sent_at > messages[4].sent_at);
    }

    #[test]
    fn test_update_status() {
        let (db, _dir) = create_test_db();
        create_test_conversation(&db, "conv1");
        let repo = MessageRepository::new(&db);

        let msg = Message::new_text("conv1", "sender1", "Test");
        repo.save(&msg).unwrap();

        repo.update_status(&msg.id, MessageStatus::Delivered).unwrap();

        let retrieved = repo.get(&msg.id).unwrap();
        assert_eq!(retrieved.status, MessageStatus::Delivered);
    }

    #[test]
    fn test_search() {
        let (db, _dir) = create_test_db();
        create_test_conversation(&db, "conv1");
        let repo = MessageRepository::new(&db);

        repo.save(&Message::new_text("conv1", "sender1", "Hello World")).unwrap();
        repo.save(&Message::new_text("conv1", "sender1", "Goodbye World")).unwrap();
        repo.save(&Message::new_text("conv1", "sender1", "Test message")).unwrap();

        let results = repo.search(None, "World", 10);
        assert_eq!(results.len(), 2);

        let results = repo.search(Some("conv1"), "Test", 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_delete() {
        let (db, _dir) = create_test_db();
        create_test_conversation(&db, "conv1");
        let repo = MessageRepository::new(&db);

        let msg = Message::new_text("conv1", "sender1", "To be deleted");
        repo.save(&msg).unwrap();

        assert!(repo.get(&msg.id).is_some());
        repo.delete(&msg.id).unwrap();
        assert!(repo.get(&msg.id).is_none());
    }

    #[test]
    fn test_count() {
        let (db, _dir) = create_test_db();
        create_test_conversation(&db, "conv1");
        create_test_conversation(&db, "conv2");
        let repo = MessageRepository::new(&db);

        assert_eq!(repo.count("conv1"), 0);

        repo.save(&Message::new_text("conv1", "sender1", "Msg 1")).unwrap();
        repo.save(&Message::new_text("conv1", "sender1", "Msg 2")).unwrap();
        repo.save(&Message::new_text("conv2", "sender1", "Msg 3")).unwrap();

        assert_eq!(repo.count("conv1"), 2);
        assert_eq!(repo.count("conv2"), 1);
        assert_eq!(repo.total_count(), 3);
    }
}
