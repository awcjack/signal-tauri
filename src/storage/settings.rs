//! Application settings storage

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Theme (dark/light/system)
    pub theme: Theme,

    /// Language/locale
    pub language: String,

    /// Whether to show typing indicators
    pub typing_indicators: bool,

    /// Whether to send read receipts
    pub read_receipts: bool,

    /// Whether to show link previews
    pub link_previews: bool,

    /// Notification settings
    pub notifications: NotificationSettings,

    /// Privacy settings
    pub privacy: PrivacySettings,

    /// Media settings
    pub media: MediaSettings,

    /// Keyboard shortcuts
    pub shortcuts: ShortcutSettings,

    /// Window settings
    pub window: WindowSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            language: "en".to_string(),
            typing_indicators: true,
            read_receipts: true,
            link_previews: true,
            notifications: NotificationSettings::default(),
            privacy: PrivacySettings::default(),
            media: MediaSettings::default(),
            shortcuts: ShortcutSettings::default(),
            window: WindowSettings::default(),
        }
    }
}

/// Theme options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

/// Notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Enable notifications
    pub enabled: bool,

    /// Show message preview in notification
    pub show_preview: bool,

    /// Show sender name in notification
    pub show_sender: bool,

    /// Play notification sound
    pub sound: bool,

    /// Notification sound file
    pub sound_file: Option<String>,

    /// Badge count on dock icon
    pub badge_count: bool,

    /// Flash taskbar on message
    pub flash_taskbar: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_preview: true,
            show_sender: true,
            sound: true,
            sound_file: None,
            badge_count: true,
            flash_taskbar: true,
        }
    }
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    /// Screen lock enabled
    pub screen_lock: bool,

    /// Screen lock timeout (seconds)
    pub screen_lock_timeout: u32,

    /// Block screenshots
    pub block_screenshots: bool,

    /// Incognito keyboard (disable learning)
    pub incognito_keyboard: bool,

    /// Registration lock enabled
    pub registration_lock: bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            screen_lock: false,
            screen_lock_timeout: 300,
            block_screenshots: false,
            incognito_keyboard: false,
            registration_lock: false,
        }
    }
}

/// Media settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSettings {
    /// Auto-download images on WiFi
    pub auto_download_images_wifi: bool,

    /// Auto-download images on mobile
    pub auto_download_images_mobile: bool,

    /// Auto-download videos on WiFi
    pub auto_download_videos_wifi: bool,

    /// Auto-download videos on mobile
    pub auto_download_videos_mobile: bool,

    /// Auto-download files on WiFi
    pub auto_download_files_wifi: bool,

    /// Auto-download files on mobile
    pub auto_download_files_mobile: bool,

    /// Default media quality
    pub media_quality: MediaQuality,

    /// Save received media to gallery
    pub save_to_gallery: bool,

    /// Gallery save path
    pub gallery_path: Option<PathBuf>,
}

impl Default for MediaSettings {
    fn default() -> Self {
        Self {
            auto_download_images_wifi: true,
            auto_download_images_mobile: true,
            auto_download_videos_wifi: true,
            auto_download_videos_mobile: false,
            auto_download_files_wifi: true,
            auto_download_files_mobile: false,
            media_quality: MediaQuality::Standard,
            save_to_gallery: false,
            gallery_path: None,
        }
    }
}

/// Media quality options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MediaQuality {
    Low,
    Standard,
    High,
}

impl Default for MediaQuality {
    fn default() -> Self {
        Self::Standard
    }
}

/// Keyboard shortcut settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutSettings {
    /// Global shortcut to open app
    pub global_open: Option<String>,

    /// Shortcut to start new conversation
    pub new_conversation: String,

    /// Shortcut to search
    pub search: String,

    /// Shortcut to go to next conversation
    pub next_conversation: String,

    /// Shortcut to go to previous conversation
    pub prev_conversation: String,

    /// Shortcut to archive conversation
    pub archive: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            global_open: None,
            new_conversation: "Ctrl+N".to_string(),
            search: "Ctrl+F".to_string(),
            next_conversation: "Ctrl+Tab".to_string(),
            prev_conversation: "Ctrl+Shift+Tab".to_string(),
            archive: "Ctrl+Shift+A".to_string(),
        }
    }
}

/// Window settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    /// Start minimized
    pub start_minimized: bool,

    /// Start on system startup
    pub start_on_boot: bool,

    /// Close to tray
    pub close_to_tray: bool,

    /// Window position X
    pub window_x: Option<i32>,

    /// Window position Y
    pub window_y: Option<i32>,

    /// Window width
    pub window_width: Option<u32>,

    /// Window height
    pub window_height: Option<u32>,

    /// Window maximized
    pub window_maximized: bool,

    /// Sidebar width
    pub sidebar_width: u32,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            start_minimized: false,
            start_on_boot: false,
            close_to_tray: true,
            window_x: None,
            window_y: None,
            window_width: Some(1200),
            window_height: Some(800),
            window_maximized: false,
            sidebar_width: 300,
        }
    }
}

/// Settings repository
pub struct SettingsRepository {
    settings_path: PathBuf,
    settings: Settings,
}

impl SettingsRepository {
    /// Create a new settings repository
    pub fn new(data_dir: &PathBuf) -> Self {
        let settings_path = data_dir.join("settings.json");

        let settings = if settings_path.exists() {
            std::fs::read_to_string(&settings_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Settings::default()
        };

        Self {
            settings_path,
            settings,
        }
    }

    /// Get current settings
    pub fn get(&self) -> &Settings {
        &self.settings
    }

    /// Get mutable settings
    pub fn get_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    /// Save settings
    pub fn save(&self) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(&self.settings)?;
        std::fs::write(&self.settings_path, content)?;
        Ok(())
    }

    /// Reset to defaults
    pub fn reset(&mut self) {
        self.settings = Settings::default();
    }
}
