//! Storage module - encrypted local database

pub mod conversations;
pub mod messages;
pub mod settings;

use anyhow::Result;
use directories::ProjectDirs;
use std::path::PathBuf;

/// Main storage interface
pub struct Storage {
    /// Base directory for all data
    data_dir: PathBuf,

    /// Path to the database file
    db_path: PathBuf,

    /// Path to attachments directory
    attachments_dir: PathBuf,

    /// Path to avatars directory
    avatars_dir: PathBuf,

    /// Whether an account is registered
    has_account: bool,

    /// Phone number (if registered)
    phone_number: Option<String>,

    /// Device ID (if registered)
    device_id: Option<u32>,
}

impl Storage {
    /// Create a new storage instance
    pub fn new() -> Result<Self> {
        // Get platform-specific data directory
        let project_dirs = ProjectDirs::from("org", "signal-tauri", "Signal")
            .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

        let data_dir = project_dirs.data_dir().to_path_buf();

        // Ensure directories exist
        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("signal.db");
        let attachments_dir = data_dir.join("attachments");
        let avatars_dir = data_dir.join("avatars");

        std::fs::create_dir_all(&attachments_dir)?;
        std::fs::create_dir_all(&avatars_dir)?;

        tracing::info!("Storage initialized at: {:?}", data_dir);

        // Check for existing account
        let config_path = data_dir.join("config.json");
        let (has_account, phone_number, device_id) = if config_path.exists() {
            // Load existing config
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
                        let phone = config.get("phone_number")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let device = config.get("device_id")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32);
                        (phone.is_some(), phone, device)
                    } else {
                        (false, None, None)
                    }
                }
                Err(_) => (false, None, None),
            }
        } else {
            (false, None, None)
        };

        Ok(Self {
            data_dir,
            db_path,
            attachments_dir,
            avatars_dir,
            has_account,
            phone_number,
            device_id,
        })
    }

    /// Check if an account exists
    pub fn has_account(&self) -> bool {
        self.has_account
    }

    /// Get phone number
    pub fn get_phone_number(&self) -> Option<String> {
        self.phone_number.clone()
    }

    /// Get device ID
    pub fn get_device_id(&self) -> Option<u32> {
        self.device_id
    }

    /// Get data directory path
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Get database path
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    /// Get attachments directory
    pub fn attachments_dir(&self) -> &PathBuf {
        &self.attachments_dir
    }

    /// Get avatars directory
    pub fn avatars_dir(&self) -> &PathBuf {
        &self.avatars_dir
    }

    /// Save account credentials
    pub fn save_account(&mut self, phone_number: &str, device_id: u32) -> Result<()> {
        let config = serde_json::json!({
            "phone_number": phone_number,
            "device_id": device_id,
        });

        let config_path = self.data_dir.join("config.json");
        std::fs::write(&config_path, serde_json::to_string_pretty(&config)?)?;

        self.has_account = true;
        self.phone_number = Some(phone_number.to_string());
        self.device_id = Some(device_id);

        tracing::info!("Account saved for: {}", phone_number);
        Ok(())
    }

    /// Clear all data (for logout)
    pub fn clear_all(&mut self) -> Result<()> {
        // Delete database
        if self.db_path.exists() {
            std::fs::remove_file(&self.db_path)?;
        }

        // Delete config
        let config_path = self.data_dir.join("config.json");
        if config_path.exists() {
            std::fs::remove_file(&config_path)?;
        }

        // Clear attachments
        if self.attachments_dir.exists() {
            std::fs::remove_dir_all(&self.attachments_dir)?;
            std::fs::create_dir_all(&self.attachments_dir)?;
        }

        // Clear avatars
        if self.avatars_dir.exists() {
            std::fs::remove_dir_all(&self.avatars_dir)?;
            std::fs::create_dir_all(&self.avatars_dir)?;
        }

        self.has_account = false;
        self.phone_number = None;
        self.device_id = None;

        tracing::info!("All data cleared");
        Ok(())
    }

    /// Get total storage used
    pub fn storage_used(&self) -> Result<u64> {
        let mut total = 0u64;

        // Count database size
        if self.db_path.exists() {
            total += std::fs::metadata(&self.db_path)?.len();
        }

        // Count attachments
        total += dir_size(&self.attachments_dir)?;

        // Count avatars
        total += dir_size(&self.avatars_dir)?;

        Ok(total)
    }
}

/// Calculate total size of a directory
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

/// Format bytes as human-readable string
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
