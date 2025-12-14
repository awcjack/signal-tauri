//! Contact management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A Signal contact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// UUID (ACI - Account Identity)
    pub uuid: String,

    /// Phone number (E.164 format)
    pub phone_number: Option<String>,

    /// Profile name (first + last)
    pub profile_name: Option<String>,

    /// Contact name from address book
    pub system_name: Option<String>,

    /// Username (if set)
    pub username: Option<String>,

    /// Profile avatar path
    pub avatar_path: Option<String>,

    /// Profile about text
    pub about: Option<String>,

    /// Profile emoji
    pub about_emoji: Option<String>,

    /// Whether this contact is blocked
    pub blocked: bool,

    /// Whether this contact is in our address book
    pub is_system_contact: bool,

    /// When the profile was last fetched
    pub profile_fetched_at: Option<DateTime<Utc>>,

    /// Profile key for decrypting profile
    pub profile_key: Option<Vec<u8>>,

    /// Identity key
    pub identity_key: Option<Vec<u8>>,

    /// Whether we've verified their identity
    pub verified: VerificationState,

    /// When we last interacted with this contact
    pub last_interaction: Option<DateTime<Utc>>,
}

/// Identity verification state
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VerificationState {
    /// Default - not verified
    Default,
    /// Verified (safety number checked)
    Verified,
    /// Unverified (identity key changed)
    Unverified,
}

impl Default for VerificationState {
    fn default() -> Self {
        Self::Default
    }
}

impl Contact {
    /// Create a new contact from UUID
    pub fn new(uuid: &str) -> Self {
        Self {
            uuid: uuid.to_string(),
            phone_number: None,
            profile_name: None,
            system_name: None,
            username: None,
            avatar_path: None,
            about: None,
            about_emoji: None,
            blocked: false,
            is_system_contact: false,
            profile_fetched_at: None,
            profile_key: None,
            identity_key: None,
            verified: VerificationState::Default,
            last_interaction: None,
        }
    }

    /// Get display name (profile name > system name > phone number > uuid)
    pub fn display_name(&self) -> &str {
        self.profile_name
            .as_deref()
            .or(self.system_name.as_deref())
            .or(self.phone_number.as_deref())
            .unwrap_or(&self.uuid)
    }

    /// Get initials for avatar
    pub fn initials(&self) -> String {
        let name = self.display_name();
        name.split_whitespace()
            .take(2)
            .map(|word| word.chars().next().unwrap_or('?'))
            .collect::<String>()
            .to_uppercase()
    }

    /// Check if profile is stale and needs refresh
    pub fn needs_profile_refresh(&self) -> bool {
        match self.profile_fetched_at {
            None => true,
            Some(fetched) => {
                let age = Utc::now() - fetched;
                age.num_hours() > 24 // Refresh profiles older than 24 hours
            }
        }
    }
}

/// Contact repository for storage operations
pub struct ContactRepository {
    // TODO: Add storage backend
}

impl ContactRepository {
    /// Create a new contact repository
    pub fn new() -> Self {
        Self {}
    }

    /// Get a contact by UUID
    pub async fn get(&self, uuid: &str) -> Option<Contact> {
        // TODO: Implement storage lookup
        None
    }

    /// Get a contact by phone number
    pub async fn get_by_phone(&self, phone: &str) -> Option<Contact> {
        // TODO: Implement storage lookup
        None
    }

    /// Save a contact
    pub async fn save(&self, contact: &Contact) -> anyhow::Result<()> {
        // TODO: Implement storage save
        Ok(())
    }

    /// Get all contacts
    pub async fn list(&self) -> Vec<Contact> {
        // TODO: Implement storage list
        Vec::new()
    }

    /// Get blocked contacts
    pub async fn list_blocked(&self) -> Vec<Contact> {
        // TODO: Implement storage list with filter
        Vec::new()
    }

    /// Block a contact
    pub async fn block(&self, uuid: &str) -> anyhow::Result<()> {
        // TODO: Implement block
        Ok(())
    }

    /// Unblock a contact
    pub async fn unblock(&self, uuid: &str) -> anyhow::Result<()> {
        // TODO: Implement unblock
        Ok(())
    }

    /// Delete a contact
    pub async fn delete(&self, uuid: &str) -> anyhow::Result<()> {
        // TODO: Implement delete
        Ok(())
    }
}

impl Default for ContactRepository {
    fn default() -> Self {
        Self::new()
    }
}
