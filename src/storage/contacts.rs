use crate::storage::database::Database;
use anyhow::Result;
use chrono::{TimeZone, Utc};
use rusqlite::params;

#[derive(Debug, Clone)]
pub struct StoredContact {
    pub id: String,
    pub uuid: String,
    pub phone_number: Option<String>,
    pub name: String,
    pub profile_name: Option<String>,
    pub avatar_path: Option<String>,
    pub profile_key: Option<Vec<u8>>,
    pub is_blocked: bool,
    pub is_verified: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl StoredContact {
    pub fn new(uuid: &str, name: &str) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: uuid.to_string(),
            uuid: uuid.to_string(),
            phone_number: None,
            name: name.to_string(),
            profile_name: None,
            avatar_path: None,
            profile_key: None,
            is_blocked: false,
            is_verified: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn display_name(&self) -> &str {
        self.profile_name
            .as_deref()
            .filter(|s| !s.is_empty())
            .or(Some(self.name.as_str()).filter(|s| !s.is_empty()))
            .or(self.phone_number.as_deref())
            .unwrap_or(&self.uuid)
    }
}

pub struct ContactRepository<'a> {
    db: &'a Database,
}

impl<'a> ContactRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn get(&self, id: &str) -> Option<StoredContact> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.query_row(
            "SELECT id, uuid, phone_number, name, profile_name, avatar_path, 
                    profile_key, is_blocked, is_verified, created_at, updated_at
             FROM contacts WHERE id = ?",
            params![id],
            |row| {
                Ok(StoredContact {
                    id: row.get(0)?,
                    uuid: row.get(1)?,
                    phone_number: row.get(2)?,
                    name: row.get(3)?,
                    profile_name: row.get(4)?,
                    avatar_path: row.get(5)?,
                    profile_key: row.get(6)?,
                    is_blocked: row.get::<_, i64>(7)? != 0,
                    is_verified: row.get::<_, i64>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .ok()
    }

    pub fn get_by_uuid(&self, uuid: &str) -> Option<StoredContact> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.query_row(
            "SELECT id, uuid, phone_number, name, profile_name, avatar_path, 
                    profile_key, is_blocked, is_verified, created_at, updated_at
             FROM contacts WHERE uuid = ?",
            params![uuid],
            |row| {
                Ok(StoredContact {
                    id: row.get(0)?,
                    uuid: row.get(1)?,
                    phone_number: row.get(2)?,
                    name: row.get(3)?,
                    profile_name: row.get(4)?,
                    avatar_path: row.get(5)?,
                    profile_key: row.get(6)?,
                    is_blocked: row.get::<_, i64>(7)? != 0,
                    is_verified: row.get::<_, i64>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .ok()
    }

    pub fn save(&self, contact: &StoredContact) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO contacts 
             (id, uuid, phone_number, name, profile_name, avatar_path, 
              profile_key, is_blocked, is_verified, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                contact.id,
                contact.uuid,
                contact.phone_number,
                contact.name,
                contact.profile_name,
                contact.avatar_path,
                contact.profile_key,
                contact.is_blocked as i64,
                contact.is_verified as i64,
                contact.created_at,
                contact.updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn list(&self) -> Vec<StoredContact> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        let mut stmt = match conn.prepare(
            "SELECT id, uuid, phone_number, name, profile_name, avatar_path, 
                    profile_key, is_blocked, is_verified, created_at, updated_at
             FROM contacts ORDER BY name ASC",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        stmt.query_map([], |row| {
            Ok(StoredContact {
                id: row.get(0)?,
                uuid: row.get(1)?,
                phone_number: row.get(2)?,
                name: row.get(3)?,
                profile_name: row.get(4)?,
                avatar_path: row.get(5)?,
                profile_key: row.get(6)?,
                is_blocked: row.get::<_, i64>(7)? != 0,
                is_verified: row.get::<_, i64>(8)? != 0,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();
        conn.execute("DELETE FROM contacts WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn count(&self) -> usize {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();

        conn.query_row("SELECT COUNT(*) FROM contacts", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
    }

    pub fn clear(&self) -> Result<()> {
        let conn = self.db.connection();
        let conn = conn.lock().unwrap();
        conn.execute("DELETE FROM contacts", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const TEST_KEY: &str = "test-passphrase-123";

    fn create_test_db() -> (Database, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open_encrypted(&db_path, TEST_KEY).unwrap();
        (db, dir)
    }

    #[test]
    fn test_save_and_get_contact() {
        let (db, _dir) = create_test_db();
        let repo = ContactRepository::new(&db);

        let contact = StoredContact::new("uuid-123", "John Doe");
        repo.save(&contact).unwrap();

        let retrieved = repo.get("uuid-123").unwrap();
        assert_eq!(retrieved.uuid, "uuid-123");
        assert_eq!(retrieved.name, "John Doe");
    }

    #[test]
    fn test_list_contacts() {
        let (db, _dir) = create_test_db();
        let repo = ContactRepository::new(&db);

        repo.save(&StoredContact::new("uuid-1", "Alice")).unwrap();
        repo.save(&StoredContact::new("uuid-2", "Bob")).unwrap();
        repo.save(&StoredContact::new("uuid-3", "Charlie")).unwrap();

        let contacts = repo.list();
        assert_eq!(contacts.len(), 3);
        assert_eq!(contacts[0].name, "Alice");
        assert_eq!(contacts[1].name, "Bob");
        assert_eq!(contacts[2].name, "Charlie");
    }

    #[test]
    fn test_delete_contact() {
        let (db, _dir) = create_test_db();
        let repo = ContactRepository::new(&db);

        repo.save(&StoredContact::new("uuid-1", "ToDelete")).unwrap();
        assert!(repo.get("uuid-1").is_some());

        repo.delete("uuid-1").unwrap();
        assert!(repo.get("uuid-1").is_none());
    }

    #[test]
    fn test_count_contacts() {
        let (db, _dir) = create_test_db();
        let repo = ContactRepository::new(&db);

        assert_eq!(repo.count(), 0);

        repo.save(&StoredContact::new("uuid-1", "A")).unwrap();
        repo.save(&StoredContact::new("uuid-2", "B")).unwrap();

        assert_eq!(repo.count(), 2);
    }
}
