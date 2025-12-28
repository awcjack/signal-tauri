use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn open_encrypted(path: &Path, passphrase: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "key", passphrase)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.initialize()?;
        Ok(db)
    }

    fn initialize(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                conversation_type TEXT NOT NULL,
                name TEXT NOT NULL,
                avatar_path TEXT,
                last_message TEXT,
                last_message_at INTEGER,
                unread_count INTEGER DEFAULT 0,
                is_pinned INTEGER DEFAULT 0,
                is_muted INTEGER DEFAULT 0,
                muted_until INTEGER,
                is_archived INTEGER DEFAULT 0,
                is_blocked INTEGER DEFAULT 0,
                disappearing_timer INTEGER DEFAULT 0,
                draft TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                sender TEXT NOT NULL,
                direction TEXT NOT NULL,
                status TEXT NOT NULL,
                content_type TEXT NOT NULL,
                content_json TEXT NOT NULL,
                sent_at INTEGER NOT NULL,
                server_timestamp INTEGER,
                delivered_at INTEGER,
                read_at INTEGER,
                quote_json TEXT,
                reactions_json TEXT,
                expires_in_seconds INTEGER,
                expires_at INTEGER,
                FOREIGN KEY (conversation_id) REFERENCES conversations(id)
            );

            CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                phone_number TEXT,
                uuid TEXT UNIQUE,
                name TEXT NOT NULL,
                profile_name TEXT,
                avatar_path TEXT,
                profile_key BLOB,
                is_blocked INTEGER DEFAULT 0,
                is_verified INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_conversation 
                ON messages(conversation_id, sent_at DESC);
            CREATE INDEX IF NOT EXISTS idx_messages_sender 
                ON messages(sender);
            CREATE INDEX IF NOT EXISTS idx_conversations_updated 
                ON conversations(updated_at DESC);
            CREATE INDEX IF NOT EXISTS idx_contacts_uuid 
                ON contacts(uuid);
            "
        )?;

        Ok(())
    }

    pub fn connection(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const TEST_PASSPHRASE: &str = "test-encryption-key-12345";

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open_encrypted(&db_path, TEST_PASSPHRASE).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn test_tables_created() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open_encrypted(&db_path, TEST_PASSPHRASE).unwrap();
        
        let conn = db.conn.lock().unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"conversations".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"contacts".to_string()));
        assert!(tables.contains(&"settings".to_string()));
    }

    #[test]
    fn test_encrypted_db_requires_correct_key() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("encrypted.db");
        
        {
            let db = Database::open_encrypted(&db_path, TEST_PASSPHRASE).unwrap();
            let conn = db.conn.lock().unwrap();
            conn.execute("INSERT INTO settings (key, value) VALUES ('test', 'data')", []).unwrap();
        }
        
        let wrong_key_result = Database::open_encrypted(&db_path, "wrong-key");
        assert!(wrong_key_result.is_err());
    }
}
