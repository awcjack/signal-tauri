pub mod contacts;
pub mod conversations;
pub mod database;
pub mod encryption;
pub mod messages;
pub mod settings;

use anyhow::Result;
use database::Database;
use directories::ProjectDirs;
use encryption::{EncryptionConfig, EncryptionMethod, EncryptionProvider};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub use encryption::{
    EncryptionConfig as StorageEncryptionConfig, EncryptionMethod as StorageEncryptionMethod,
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct AppConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<u32>,
    #[serde(default)]
    pub encryption: EncryptionConfig,
}

impl AppConfig {
    fn load(path: &Path) -> Option<Self> {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
        } else {
            None
        }
    }

    fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub struct Storage {
    data_dir: PathBuf,
    attachments_dir: PathBuf,
    avatars_dir: PathBuf,
    has_account: AtomicBool,
    phone_number: RwLock<Option<String>>,
    device_id: RwLock<Option<u32>>,
    database: RwLock<Option<Database>>,
    encryption_provider: RwLock<EncryptionProvider>,
    database_unlocked: AtomicBool,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let project_dirs = ProjectDirs::from("org", "signal-tauri", "Signal")
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

        let data_dir = project_dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;

        let attachments_dir = data_dir.join("attachments");
        let avatars_dir = data_dir.join("avatars");
        std::fs::create_dir_all(&attachments_dir)?;
        std::fs::create_dir_all(&avatars_dir)?;

        tracing::info!("Storage initialized at: {:?}", data_dir);

        let config_path = data_dir.join("config.json");
        let app_config = AppConfig::load(&config_path).unwrap_or_default();

        let has_account = app_config.phone_number.is_some();
        let phone_number = app_config.phone_number.clone();
        let device_id = app_config.device_id;

        let encryption_provider = EncryptionProvider::new(&data_dir, app_config.encryption.clone());

        let (database, database_unlocked) = if encryption_provider.method() == EncryptionMethod::Password {
            tracing::info!("Password encryption - database locked until password provided");
            (None, false)
        } else if encryption_provider.is_configured() {
            match encryption_provider.get_key(None) {
                Ok(key) => {
                    let app_db_path = data_dir.join("app.db");
                    match Database::open_encrypted(&app_db_path, &key) {
                        Ok(db) => {
                            tracing::info!("Encrypted app database initialized at: {:?}", app_db_path);
                            (Some(db), true)
                        }
                        Err(e) => {
                            tracing::error!("Failed to open encrypted database: {}", e);
                            (None, false)
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get encryption key: {}", e);
                    (None, false)
                }
            }
        } else {
            tracing::info!("Encryption not yet configured");
            (None, false)
        };

        Ok(Self {
            data_dir,
            attachments_dir,
            avatars_dir,
            has_account: AtomicBool::new(has_account),
            phone_number: RwLock::new(phone_number),
            device_id: RwLock::new(device_id),
            database: RwLock::new(database),
            encryption_provider: RwLock::new(encryption_provider),
            database_unlocked: AtomicBool::new(database_unlocked),
        })
    }

    pub fn encryption_method(&self) -> EncryptionMethod {
        self.encryption_provider.read().method()
    }

    pub fn is_encryption_configured(&self) -> bool {
        self.encryption_provider.read().is_configured()
    }

    pub fn needs_password(&self) -> bool {
        self.encryption_provider.read().method() == EncryptionMethod::Password 
            && !self.database_unlocked.load(Ordering::SeqCst)
    }

    pub fn is_database_unlocked(&self) -> bool {
        self.database_unlocked.load(Ordering::SeqCst)
    }

    pub fn setup_encryption(
        &self,
        method: EncryptionMethod,
        password: Option<&str>,
    ) -> Result<()> {
        let config = EncryptionConfig {
            method,
            salt: None,
        };

        let mut provider = EncryptionProvider::new(&self.data_dir, config);
        let key = provider.setup(password)?;

        let app_db_path = self.data_dir.join("app.db");
        let db = Database::open_encrypted(&app_db_path, &key)?;

        *self.database.write() = Some(db);
        self.database_unlocked.store(true, Ordering::SeqCst);
        *self.encryption_provider.write() = provider;

        self.save_config()?;

        tracing::info!("Encryption setup complete with method: {:?}", method);
        Ok(())
    }

    pub fn unlock_database(&self, password: Option<&str>) -> Result<()> {
        if self.database_unlocked.load(Ordering::SeqCst) {
            return Ok(());
        }

        let key = self.encryption_provider.read().get_key(password)?;
        let app_db_path = self.data_dir.join("app.db");

        let db = Database::open_encrypted(&app_db_path, &key)?;
        *self.database.write() = Some(db);
        self.database_unlocked.store(true, Ordering::SeqCst);

        tracing::info!("Database unlocked successfully");
        Ok(())
    }

    pub fn change_encryption_password(
        &self,
        old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        if self.encryption_provider.read().method() != EncryptionMethod::Password {
            return Err(anyhow::anyhow!(
                "Password change only available for password-based encryption"
            ));
        }

        let (_old_key, new_key) = self.encryption_provider.write().change_password(old_password, new_password)?;

        if let Some(ref db) = *self.database.read() {
            let conn = db.connection();
            let conn = conn.lock().unwrap();
            conn.pragma_update(None, "rekey", &new_key)?;
        }

        self.save_config()?;

        tracing::info!("Encryption password changed successfully");
        Ok(())
    }

    pub fn migrate_encryption(
        &self,
        new_method: EncryptionMethod,
        current_password: Option<&str>,
        new_password: Option<&str>,
    ) -> Result<()> {
        let old_method = self.encryption_provider.read().method();
        let _old_key = self.encryption_provider.read().get_key(current_password)?;

        let new_config = EncryptionConfig {
            method: new_method,
            salt: None,
        };
        let mut new_provider = EncryptionProvider::new(&self.data_dir, new_config);
        let new_key = new_provider.setup(new_password)?;

        if let Some(ref db) = *self.database.read() {
            let conn = db.connection();
            let conn = conn.lock().unwrap();
            conn.pragma_update(None, "rekey", &new_key)?;
        }

        match old_method {
            EncryptionMethod::AutoGenerated => {
                let key_path = self.data_dir.join(".encryption_key");
                if key_path.exists() {
                    std::fs::remove_file(&key_path)?;
                }
            }
            EncryptionMethod::Keychain => {
                let _ = EncryptionProvider::clear_keychain();
            }
            EncryptionMethod::Password => {}
        }

        *self.encryption_provider.write() = new_provider;
        
        let is_unlocked = new_method != EncryptionMethod::Password || self.database.read().is_some();
        self.database_unlocked.store(is_unlocked, Ordering::SeqCst);

        self.save_config()?;

        tracing::info!("Encryption migrated to method: {:?}", new_method);
        Ok(())
    }

    pub fn has_account(&self) -> bool {
        self.has_account.load(Ordering::SeqCst)
    }

    pub fn get_phone_number(&self) -> Option<String> {
        self.phone_number.read().clone()
    }

    pub fn get_device_id(&self) -> Option<u32> {
        *self.device_id.read()
    }

    pub fn save_account(&self, phone_number: &str, device_id: u32) -> Result<()> {
        // Only setup encryption if not already configured AND database doesn't exist yet
        // (database existence is a reliable indicator that encryption was set up)
        let app_db_path = self.data_dir.join("app.db");
        if !self.encryption_provider.read().is_configured() && !app_db_path.exists() {
            self.setup_encryption(EncryptionMethod::AutoGenerated, None)?;
        }

        *self.phone_number.write() = Some(phone_number.to_string());
        *self.device_id.write() = Some(device_id);
        self.has_account.store(true, Ordering::SeqCst);

        self.save_config()?;

        tracing::info!("Account saved for: {}", phone_number);
        Ok(())
    }

    fn save_config(&self) -> Result<()> {
        let config = AppConfig {
            phone_number: self.phone_number.read().clone(),
            device_id: *self.device_id.read(),
            encryption: self.encryption_provider.read().config().clone(),
        };

        let config_path = self.data_dir.join("config.json");
        config.save(&config_path)?;
        Ok(())
    }

    pub fn clear_all(&self) -> Result<()> {
        *self.database.write() = None;
        self.database_unlocked.store(false, Ordering::SeqCst);

        match self.encryption_provider.read().method() {
            EncryptionMethod::AutoGenerated => {
                let key_path = self.data_dir.join(".encryption_key");
                if key_path.exists() {
                    std::fs::remove_file(&key_path)?;
                    tracing::info!("Removed auto-generated encryption key");
                }
            }
            EncryptionMethod::Keychain => {
                if let Err(e) = EncryptionProvider::clear_keychain() {
                    tracing::warn!("Failed to clear keychain: {}", e);
                } else {
                    tracing::info!("Cleared keychain entry");
                }
            }
            EncryptionMethod::Password => {
                tracing::info!("Password encryption - salt will be cleared with config");
            }
        }

        let app_db = self.data_dir.join("app.db");
        if app_db.exists() {
            std::fs::remove_file(&app_db)?;
        }

        let signal_db = self.data_dir.join("signal_protocol.db");
        if signal_db.exists() {
            std::fs::remove_file(&signal_db)?;
        }

        let config_path = self.data_dir.join("config.json");
        if config_path.exists() {
            std::fs::remove_file(&config_path)?;
        }

        if self.attachments_dir.exists() {
            std::fs::remove_dir_all(&self.attachments_dir)?;
            std::fs::create_dir_all(&self.attachments_dir)?;
        }

        if self.avatars_dir.exists() {
            std::fs::remove_dir_all(&self.avatars_dir)?;
            std::fs::create_dir_all(&self.avatars_dir)?;
        }

        self.has_account.store(false, Ordering::SeqCst);
        *self.phone_number.write() = None;
        *self.device_id.write() = None;

        *self.encryption_provider.write() = EncryptionProvider::new(
            &self.data_dir,
            EncryptionConfig::default(),
        );

        tracing::info!("All data cleared");
        Ok(())
    }

    pub fn database(&self) -> Option<parking_lot::MappedRwLockReadGuard<'_, Database>> {
        let guard = self.database.read();
        parking_lot::RwLockReadGuard::try_map(guard, |opt| opt.as_ref()).ok()
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn attachments_dir(&self) -> &PathBuf {
        &self.attachments_dir
    }

    pub fn avatars_dir(&self) -> &PathBuf {
        &self.avatars_dir
    }

    pub fn signal_db_path(&self) -> PathBuf {
        self.data_dir.join("signal_protocol.db")
    }

    pub fn get_encryption_key(&self) -> Option<String> {
        if self.encryption_provider.read().method() == EncryptionMethod::Password 
            && !self.database_unlocked.load(Ordering::SeqCst) 
        {
            tracing::warn!("Cannot get encryption key - database not unlocked");
            return None;
        }
        self.encryption_provider.read().get_key(None).ok()
    }

    pub fn storage_used(&self) -> Result<u64> {
        let mut total = 0u64;

        let app_db = self.data_dir.join("app.db");
        if app_db.exists() {
            total += std::fs::metadata(&app_db)?.len();
        }

        total += dir_size(&self.attachments_dir)?;
        total += dir_size(&self.avatars_dir)?;

        Ok(total)
    }
}

fn dir_size(path: &PathBuf) -> Result<u64> {
    let mut total = 0u64;

    if path.exists() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                total += metadata.len();
            } else if metadata.is_dir() {
                total += dir_size(&entry.path())?;
            }
        }
    }

    Ok(total)
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_storage(dir: &Path) -> Storage {
        let attachments_dir = dir.join("attachments");
        let avatars_dir = dir.join("avatars");
        std::fs::create_dir_all(&attachments_dir).unwrap();
        std::fs::create_dir_all(&avatars_dir).unwrap();

        Storage {
            data_dir: dir.to_path_buf(),
            attachments_dir,
            avatars_dir,
            has_account: AtomicBool::new(false),
            phone_number: RwLock::new(None),
            device_id: RwLock::new(None),
            database: RwLock::new(None),
            encryption_provider: RwLock::new(EncryptionProvider::new(dir, EncryptionConfig::default())),
            database_unlocked: AtomicBool::new(false),
        }
    }

    #[test]
    fn test_setup_auto_generated_encryption() {
        let dir = tempdir().unwrap();
        let storage = create_test_storage(dir.path());

        storage
            .setup_encryption(EncryptionMethod::AutoGenerated, None)
            .unwrap();

        assert!(storage.is_encryption_configured());
        assert!(storage.is_database_unlocked());
        assert!(!storage.needs_password());
        assert!(storage.database().is_some());
    }

    #[test]
    fn test_setup_password_encryption() {
        let dir = tempdir().unwrap();
        let storage = create_test_storage(dir.path());

        storage
            .setup_encryption(EncryptionMethod::Password, Some("my-secret-password"))
            .unwrap();

        assert!(storage.is_encryption_configured());
        assert!(storage.is_database_unlocked());
        assert!(storage.database().is_some());
    }

    #[test]
    fn test_password_unlock_flow() {
        let dir = tempdir().unwrap();

        {
            let storage = create_test_storage(dir.path());
            storage
                .setup_encryption(EncryptionMethod::Password, Some("my-password"))
                .unwrap();
            storage.save_account("+1234567890", 1).unwrap();
        }

        let config_path = dir.path().join("config.json");
        let config = AppConfig::load(&config_path).unwrap();
        assert_eq!(config.encryption.method, EncryptionMethod::Password);
        assert!(config.encryption.salt.is_some());

        let storage = Storage {
            data_dir: dir.path().to_path_buf(),
            attachments_dir: dir.path().join("attachments"),
            avatars_dir: dir.path().join("avatars"),
            has_account: AtomicBool::new(true),
            phone_number: RwLock::new(Some("+1234567890".to_string())),
            device_id: RwLock::new(Some(1)),
            database: RwLock::new(None),
            encryption_provider: RwLock::new(EncryptionProvider::new(dir.path(), config.encryption)),
            database_unlocked: AtomicBool::new(false),
        };

        assert!(storage.needs_password());
        assert!(!storage.is_database_unlocked());

        storage.unlock_database(Some("my-password")).unwrap();
        assert!(storage.is_database_unlocked());
        assert!(!storage.needs_password());
    }

    #[test]
    fn test_save_and_load_config() {
        let dir = tempdir().unwrap();
        let storage = create_test_storage(dir.path());

        storage
            .setup_encryption(EncryptionMethod::AutoGenerated, None)
            .unwrap();
        storage.save_account("+1234567890", 42).unwrap();

        let config_path = dir.path().join("config.json");
        let loaded = AppConfig::load(&config_path).unwrap();

        assert_eq!(loaded.phone_number, Some("+1234567890".to_string()));
        assert_eq!(loaded.device_id, Some(42));
        assert_eq!(loaded.encryption.method, EncryptionMethod::AutoGenerated);
    }

    #[test]
    fn test_clear_all_removes_encryption() {
        let dir = tempdir().unwrap();
        let storage = create_test_storage(dir.path());

        storage
            .setup_encryption(EncryptionMethod::AutoGenerated, None)
            .unwrap();
        storage.save_account("+1234567890", 1).unwrap();

        let key_path = dir.path().join(".encryption_key");
        assert!(key_path.exists());

        storage.clear_all().unwrap();

        assert!(!key_path.exists());
        assert!(!dir.path().join("app.db").exists());
        assert!(!dir.path().join("config.json").exists());
        assert!(!storage.has_account());
        assert!(storage.database().is_none());
    }
}
