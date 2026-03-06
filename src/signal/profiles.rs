//! Profile and avatar fetching

use crate::signal::SignalError;
use crate::storage::contacts::ContactRepository;
use crate::storage::conversations::{ConversationRepository, ConversationType};
use crate::storage::Storage;
use presage::libsignal_service::zkgroup::profiles::ProfileKey;
use presage::manager::Registered;
use presage::store::ContentsStore;
use presage::Manager;
use presage_store_sqlite::SqliteStore;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

/// Syncs profile keys for all contacts from presage store
/// This must be called BEFORE sync_contact_avatars to populate the profile keys
pub async fn sync_contact_profile_keys(
    presage_store: &SqliteStore,
    storage: &Arc<Storage>,
) -> Result<usize, SignalError> {
    let db = storage
        .database()
        .ok_or_else(|| SignalError::StorageError("App database not available".to_string()))?;

    let repo = ContactRepository::new(&db);
    let mut updated_count = 0;

    // Get all contacts from presage store
    let presage_contacts: Vec<_> = presage_store
        .contacts()
        .await
        .map_err(|e| {
            SignalError::StorageError(format!("Failed to get contacts from presage: {:?}", e))
        })?
        .filter_map(|r| r.ok())
        .collect();

    tracing::info!(
        "Checking {} contacts for profile key updates",
        presage_contacts.len()
    );

    for presage_contact in presage_contacts {
        let uuid_str = presage_contact.uuid.to_string();

        // Only process if presage has a valid profile key
        if presage_contact.profile_key.is_empty() || presage_contact.profile_key.len() != 32 {
            tracing::debug!("Skipping contact {} - no valid profile key", uuid_str);
            continue;
        }

        // Get or create contact
        let mut contact = repo.get_by_uuid(&uuid_str).unwrap_or_else(|| {
            crate::storage::contacts::StoredContact {
                id: uuid_str.clone(),
                uuid: uuid_str.clone(),
                phone_number: presage_contact.phone_number.as_ref().map(|p| p.to_string()),
                name: presage_contact.name.clone(),
                profile_name: if presage_contact.name.is_empty() {
                    None
                } else {
                    Some(presage_contact.name.clone())
                },
                avatar_path: None,
                profile_key: None,
                is_blocked: false,
                is_verified: false,
                created_at: chrono::Utc::now().timestamp(),
                updated_at: chrono::Utc::now().timestamp(),
            }
        });

        // Update profile key if missing or different
        let needs_update = contact.profile_key.is_none()
            || contact
                .profile_key
                .as_ref()
                .map(|k| k != &presage_contact.profile_key)
                .unwrap_or(true);

        if needs_update {
            contact.profile_key = Some(presage_contact.profile_key.clone());
            contact.name = presage_contact.name.clone();
            contact.profile_name = if presage_contact.name.is_empty() {
                None
            } else {
                Some(presage_contact.name)
            };
            contact.updated_at = chrono::Utc::now().timestamp();

            if let Err(e) = repo.save(&contact) {
                tracing::warn!("Failed to update contact {}: {}", uuid_str, e);
            } else {
                updated_count += 1;
                tracing::debug!("Updated profile key for contact {}", uuid_str);
            }
        }
    }

    tracing::info!("Updated {} contacts with profile keys", updated_count);
    Ok(updated_count)
}

pub async fn fetch_and_save_avatar(
    manager: &mut Manager<SqliteStore, Registered>,
    uuid: Uuid,
    profile_key_bytes: &[u8],
    avatars_dir: &PathBuf,
) -> Result<Option<PathBuf>, SignalError> {
    let profile_key_array: [u8; 32] = profile_key_bytes
        .try_into()
        .map_err(|_| SignalError::CryptoError("Invalid profile key length".into()))?;

    let profile_key = ProfileKey::create(profile_key_array);

    let avatar_bytes = match manager
        .retrieve_profile_avatar_by_uuid(uuid, profile_key)
        .await
    {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            tracing::debug!("No avatar available for {}", uuid);
            return Ok(None);
        }
        Err(e) => {
            tracing::warn!("Failed to fetch avatar for {}: {:?}", uuid, e);
            return Err(SignalError::NetworkError(format!(
                "Failed to fetch avatar: {:?}",
                e
            )));
        }
    };

    let avatar_path = avatars_dir.join(format!("{}.jpg", uuid));

    tokio::fs::write(&avatar_path, &avatar_bytes)
        .await
        .map_err(|e| SignalError::StorageError(format!("Failed to save avatar: {}", e)))?;

    tracing::info!("Saved avatar for {} to {:?}", uuid, avatar_path);
    Ok(Some(avatar_path))
}

pub async fn sync_contact_avatars(
    manager: &mut Manager<SqliteStore, Registered>,
    storage: &Arc<Storage>,
) -> Result<usize, SignalError> {
    let db = storage
        .database()
        .ok_or_else(|| SignalError::StorageError("App database not available".to_string()))?;

    let repo = ContactRepository::new(&db);
    let contacts = repo.list();

    let avatars_dir = storage.avatars_dir();
    let mut synced_count = 0;

    for contact in contacts {
        let profile_key = match &contact.profile_key {
            Some(key) if key.len() == 32 => key,
            _ => continue,
        };

        if contact.avatar_path.is_some() {
            if let Some(ref path) = contact.avatar_path {
                if std::path::Path::new(path).exists() {
                    continue;
                }
            }
        }

        let uuid = match Uuid::parse_str(&contact.uuid) {
            Ok(u) => u,
            Err(_) => {
                tracing::warn!("Invalid UUID for contact: {}", contact.uuid);
                continue;
            }
        };

        match fetch_and_save_avatar(manager, uuid, profile_key, avatars_dir).await {
            Ok(Some(path)) => {
                let mut updated_contact = contact.clone();
                updated_contact.avatar_path = Some(path.to_string_lossy().to_string());
                updated_contact.updated_at = chrono::Utc::now().timestamp();

                if let Err(e) = repo.save(&updated_contact) {
                    tracing::warn!("Failed to update contact avatar path: {}", e);
                } else {
                    synced_count += 1;
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::debug!("Failed to sync avatar for {}: {}", contact.uuid, e);
            }
        }
    }

    tracing::info!("Synced {} contact avatars", synced_count);
    Ok(synced_count)
}

pub fn update_conversations_from_contacts(storage: &Arc<Storage>) -> Result<usize, SignalError> {
    let db = storage
        .database()
        .ok_or_else(|| SignalError::StorageError("App database not available".to_string()))?;

    let contact_repo = ContactRepository::new(&db);
    let conv_repo = ConversationRepository::new(&db);

    let conversations = conv_repo.list();
    let mut updated_count = 0;

    for mut conv in conversations {
        if conv.conversation_type == ConversationType::Group {
            continue;
        }

        if let Some(contact) = contact_repo.get_by_uuid(&conv.id) {
            let mut needs_update = false;

            let contact_name = contact.display_name().to_string();
            if conv.name != contact_name && (conv.name == conv.id || conv.name.starts_with("Aci(")) {
                conv.name = contact_name;
                needs_update = true;
            }

            if conv.avatar_path.is_none() && contact.avatar_path.is_some() {
                conv.avatar_path = contact.avatar_path.clone();
                needs_update = true;
            }

            if needs_update {
                conv.updated_at = chrono::Utc::now();
                if conv_repo.save(&conv).is_ok() {
                    updated_count += 1;
                }
            }
        }
    }

    tracing::info!("Updated {} conversations with contact info", updated_count);
    Ok(updated_count)
}

pub async fn refresh_contact_avatar(
    manager: &mut Manager<SqliteStore, Registered>,
    storage: &Arc<Storage>,
    uuid_str: &str,
) -> Result<bool, SignalError> {
    let db = storage
        .database()
        .ok_or_else(|| SignalError::StorageError("App database not available".to_string()))?;

    let repo = ContactRepository::new(&db);

    let contact = repo
        .get_by_uuid(uuid_str)
        .ok_or_else(|| SignalError::StorageError("Contact not found".to_string()))?;

    let profile_key = contact
        .profile_key
        .as_ref()
        .ok_or_else(|| SignalError::CryptoError("No profile key for contact".to_string()))?;

    let uuid = Uuid::parse_str(uuid_str)
        .map_err(|e| SignalError::ProtocolError(format!("Invalid UUID: {}", e)))?;

    let avatars_dir = storage.avatars_dir();

    match fetch_and_save_avatar(manager, uuid, profile_key, avatars_dir).await? {
        Some(path) => {
            let mut updated_contact = contact.clone();
            updated_contact.avatar_path = Some(path.to_string_lossy().to_string());
            updated_contact.updated_at = chrono::Utc::now().timestamp();
            repo.save(&updated_contact)?;
            Ok(true)
        }
        None => Ok(false),
    }
}
