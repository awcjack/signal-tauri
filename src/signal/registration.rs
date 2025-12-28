//! Device registration completion after provisioning
//!
//! This module handles the final steps of device linking after we've captured
//! the full provision message (including ephemeral_backup_key).

use presage::libsignal_service::{
    configuration::{ServiceConfiguration, SignalServers},
    pre_keys::{KyberPreKeyStoreExt, PreKeysStore},
    prelude::phonenumber::PhoneNumber,
    proto::DeviceName,
    push_service::{
        DeviceActivationRequest, HttpAuth, LinkAccountAttributes, 
        LinkCapabilities, LinkRequest, LinkResponse, PushService, ServiceIds,
    },
    utils::BASE64_RELAXED,
    zkgroup::profiles::ProfileKey,
};
use presage::libsignal_service::protocol::{
    GenericSignedPreKey, IdentityKey, IdentityKeyPair, KeyPair, KyberPreKeyRecord,
    PrivateKey, PublicKey, SignedPreKeyRecord, PreKeyRecord,
    Timestamp, kem,
};
use presage::manager::RegistrationData;
use presage_store_sqlite::SqliteStore;
use presage::store::{StateStore, Store};
use base64::Engine;
use hmac::Hmac;
use prost::Message;
use rand::{Rng, CryptoRng, RngCore};
use sha2::Sha256;
use std::time::SystemTime;

use crate::signal::provisioning::FullProvisionMessage;
use crate::signal::SignalError;

type Aes256Ctr128BE = ctr::Ctr128BE<aes::Aes256>;
type HmacSha256 = Hmac<Sha256>;

const PRE_KEY_MEDIUM_MAX_VALUE: u32 = 0xFFFFFF;

/// Get current timestamp for libsignal protocol
fn timestamp_now() -> Timestamp {
    let unix_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("unix epoch in the past");
    Timestamp::from_epoch_millis(unix_time.as_millis() as u64)
}

fn calculate_hmac256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, SignalError> {
    use hmac::Mac;
    
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|e| SignalError::CryptoError(format!("HMAC init failed: {:?}", e)))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn encrypt_device_name<R: rand::Rng + rand::CryptoRng>(
    csprng: &mut R,
    device_name: &str,
    identity_public: &IdentityKey,
) -> Result<DeviceName, SignalError> {
    use aes::cipher::{KeyIvInit, StreamCipher};
    
    let plaintext = device_name.as_bytes().to_vec();
    let ephemeral_key_pair = KeyPair::generate(csprng);

    let master_secret = ephemeral_key_pair
        .private_key
        .calculate_agreement(identity_public.public_key())
        .map_err(|e| SignalError::CryptoError(format!("Key agreement failed: {:?}", e)))?;

    let key1 = calculate_hmac256(&master_secret, b"auth")?;
    let synthetic_iv = calculate_hmac256(&key1, &plaintext)?;
    let synthetic_iv = &synthetic_iv[..16];

    let key2 = calculate_hmac256(&master_secret, b"cipher")?;
    let cipher_key = calculate_hmac256(&key2, synthetic_iv)?;

    let mut ciphertext = plaintext;

    const IV: [u8; 16] = [0; 16];
    let mut cipher = Aes256Ctr128BE::new(cipher_key.as_slice().into(), &IV.into());
    cipher.apply_keystream(&mut ciphertext);

    Ok(DeviceName {
        ephemeral_public: Some(ephemeral_key_pair.public_key.serialize().to_vec()),
        synthetic_iv: Some(synthetic_iv.to_vec()),
        ciphertext: Some(ciphertext),
    })
}

pub struct RegistrationResult {
    pub phone_number: String,
    pub device_id: u32,
    pub registration_id: u32,
    pub pni_registration_id: u32,
    pub aci: uuid::Uuid,
    pub pni: uuid::Uuid,
    pub aci_identity_key_pair: IdentityKeyPair,
    pub pni_identity_key_pair: IdentityKeyPair,
    pub profile_key: ProfileKey,
    pub password: String,
}

/// Complete device registration after receiving provision message
/// 
/// This function:
/// 1. Parses identity keys from the provision message
/// 2. Generates pre-keys using the store
/// 3. Encrypts device name
/// 4. Calls Signal API to complete registration
pub async fn complete_registration(
    store: &mut SqliteStore,
    provision_msg: &FullProvisionMessage,
    device_name: &str,
    password: &str,
    signal_servers: SignalServers,
) -> Result<RegistrationResult, SignalError> {
    let mut rng = rand::rng();
    
    // Generate registration IDs
    let registration_id: u32 = rng.random_range(1..256);
    let pni_registration_id: u32 = rng.random_range(1..256);
    
    // Parse identity keys
    let aci_public_key = PublicKey::deserialize(&provision_msg.aci_identity_key_public)
        .map_err(|e| SignalError::ProtocolError(format!("Invalid ACI public key: {:?}", e)))?;
    let aci_public_key = IdentityKey::new(aci_public_key);
    
    let aci_private_key = PrivateKey::deserialize(&provision_msg.aci_identity_key_private)
        .map_err(|e| SignalError::ProtocolError(format!("Invalid ACI private key: {:?}", e)))?;
    
    let pni_public_key = PublicKey::deserialize(&provision_msg.pni_identity_key_public)
        .map_err(|e| SignalError::ProtocolError(format!("Invalid PNI public key: {:?}", e)))?;
    let pni_public_key = IdentityKey::new(pni_public_key);
    
    let pni_private_key = PrivateKey::deserialize(&provision_msg.pni_identity_key_private)
        .map_err(|e| SignalError::ProtocolError(format!("Invalid PNI private key: {:?}", e)))?;
    
    let aci_key_pair = IdentityKeyPair::new(aci_public_key, aci_private_key);
    let pni_key_pair = IdentityKeyPair::new(pni_public_key, pni_private_key);
    
    // Parse profile key
    let profile_key_bytes: [u8; 32] = provision_msg.profile_key
        .as_slice()
        .try_into()
        .map_err(|_| SignalError::ProtocolError("Invalid profile key length".into()))?;
    let profile_key = ProfileKey::create(profile_key_bytes);
    
    // Generate pre-keys for ACI
    let (
        _aci_pre_keys,
        aci_signed_pre_key,
        _aci_pq_pre_keys,
        aci_pq_last_resort_pre_key,
    ) = generate_pre_keys(
        &mut store.aci_protocol_store(),
        &mut rng,
        &aci_key_pair,
        true, // use last resort key
        0,    // no regular pre-keys for linking
        0,    // no regular kyber pre-keys for linking
    ).await?;
    
    let aci_pq_last_resort_pre_key = aci_pq_last_resort_pre_key
        .ok_or_else(|| SignalError::ProtocolError("Missing ACI last resort key".into()))?;
    
    // Generate pre-keys for PNI
    let (
        _pni_pre_keys,
        pni_signed_pre_key,
        _pni_pq_pre_keys,
        pni_pq_last_resort_pre_key,
    ) = generate_pre_keys(
        &mut store.pni_protocol_store(),
        &mut rng,
        &pni_key_pair,
        true,
        0,
        0,
    ).await?;
    
    let pni_pq_last_resort_pre_key = pni_pq_last_resort_pre_key
        .ok_or_else(|| SignalError::ProtocolError("Missing PNI last resort key".into()))?;
    
    // Encrypt device name
    let encrypted_device_name = BASE64_RELAXED.encode(
        encrypt_device_name(&mut rng, device_name, &aci_public_key)
            .map_err(|e| SignalError::ProtocolError(format!("Failed to encrypt device name: {:?}", e)))?
            .encode_to_vec(),
    );
    
    // Create link request
    let request = LinkRequest {
        verification_code: provision_msg.provisioning_code.clone(),
        account_attributes: LinkAccountAttributes {
            registration_id,
            pni_registration_id,
            fetches_messages: true,
            capabilities: LinkCapabilities::default(),
            name: encrypted_device_name,
        },
        device_activation_request: DeviceActivationRequest {
            aci_signed_pre_key: aci_signed_pre_key.try_into()
                .map_err(|e| SignalError::ProtocolError(format!("ACI signed pre-key conversion failed: {:?}", e)))?,
            pni_signed_pre_key: pni_signed_pre_key.try_into()
                .map_err(|e| SignalError::ProtocolError(format!("PNI signed pre-key conversion failed: {:?}", e)))?,
            aci_pq_last_resort_pre_key: aci_pq_last_resort_pre_key.try_into()
                .map_err(|e| SignalError::ProtocolError(format!("ACI PQ last resort key conversion failed: {:?}", e)))?,
            pni_pq_last_resort_pre_key: pni_pq_last_resort_pre_key.try_into()
                .map_err(|e| SignalError::ProtocolError(format!("PNI PQ last resort key conversion failed: {:?}", e)))?,
        },
    };
    
    // Create push service and call link API
    let service_configuration: ServiceConfiguration = signal_servers.into();
    let mut push_service = PushService::new(service_configuration, None, "signal-tauri");
    
    let http_auth = HttpAuth {
        username: provision_msg.phone_number.clone(),
        password: password.to_owned(),
    };
    
    tracing::info!("Calling Signal API to complete device linking...");
    
    let LinkResponse { aci, pni, device_id } = push_service
        .link_device(&request, http_auth)
        .await
        .map_err(|e| SignalError::LinkingFailed(format!("Link API failed: {:?}", e)))?;
    
    tracing::info!("Device linked successfully! ACI: {}, Device ID: {:?}", aci, device_id);
    
    // Save identity keys to the store for presage Manager to use
    store.set_aci_identity_key_pair(aci_key_pair.clone()).await
        .map_err(|e| SignalError::StorageError(format!("Failed to save ACI identity: {:?}", e)))?;
    store.set_pni_identity_key_pair(pni_key_pair.clone()).await
        .map_err(|e| SignalError::StorageError(format!("Failed to save PNI identity: {:?}", e)))?;
    
    tracing::info!("Identity keys saved to store");
    
    let mut signaling_key = [0u8; 52];
    rng.fill_bytes(&mut signaling_key);
    
    let phone_number: PhoneNumber = provision_msg.phone_number.parse()
        .map_err(|e| SignalError::ProtocolError(format!("Invalid phone number: {:?}", e)))?;
    
    // RegistrationData has pub(crate) fields, so we construct via JSON deserialization
    let reg_data_json = serde_json::json!({
        "signal_servers": signal_servers,
        "device_name": device_name,
        "phone_number": phone_number,
        "uuid": aci.to_string(),
        "pni": pni.to_string(),
        "password": password,
        "signaling_key": BASE64_RELAXED.encode(&signaling_key),
        "device_id": device_id,
        "registration_id": registration_id,
        "pni_registration_id": pni_registration_id,
        "profile_key": base64::engine::general_purpose::STANDARD.encode(&profile_key_bytes),
    });
    
    let registration_data: RegistrationData = serde_json::from_value(reg_data_json)
        .map_err(|e| SignalError::StorageError(format!("Failed to create registration data: {:?}", e)))?;
    
    store.save_registration_data(&registration_data).await
        .map_err(|e| SignalError::StorageError(format!("Failed to save registration data: {:?}", e)))?;
    
    tracing::info!("Registration data saved to store");
    
    Ok(RegistrationResult {
        phone_number: provision_msg.phone_number.clone(),
        device_id: device_id.into(),
        registration_id,
        pni_registration_id,
        aci,
        pni,
        aci_identity_key_pair: aci_key_pair,
        pni_identity_key_pair: pni_key_pair,
        profile_key,
        password: password.to_string(),
    })
}

/// Generate pre-keys for registration
/// 
/// This is adapted from libsignal-service's `replenish_pre_keys` which is pub(crate)
async fn generate_pre_keys<R: Rng + CryptoRng, P: PreKeysStore>(
    protocol_store: &mut P,
    csprng: &mut R,
    identity_key_pair: &IdentityKeyPair,
    use_last_resort_key: bool,
    pre_key_count: u32,
    kyber_pre_key_count: u32,
) -> Result<
    (
        Vec<PreKeyRecord>,
        SignedPreKeyRecord,
        Vec<KyberPreKeyRecord>,
        Option<KyberPreKeyRecord>,
    ),
    SignalError,
> {
    let pre_keys_offset_id = protocol_store.next_pre_key_id().await
        .map_err(|e| SignalError::ProtocolError(format!("Failed to get next pre-key ID: {:?}", e)))?;
    let next_signed_pre_key_id = protocol_store.next_signed_pre_key_id().await
        .map_err(|e| SignalError::ProtocolError(format!("Failed to get next signed pre-key ID: {:?}", e)))?;
    let pq_pre_keys_offset_id = protocol_store.next_pq_pre_key_id().await
        .map_err(|e| SignalError::ProtocolError(format!("Failed to get next PQ pre-key ID: {:?}", e)))?;

    let mut pre_keys = vec![];
    let mut pq_pre_keys = vec![];

    // Generate EC pre-keys
    for i in 0..pre_key_count {
        let key_pair = KeyPair::generate(csprng);
        let pre_key_id = (((pre_keys_offset_id + i) % (PRE_KEY_MEDIUM_MAX_VALUE - 1)) + 1).into();
        let pre_key_record = PreKeyRecord::new(pre_key_id, &key_pair);
        
        protocol_store
            .save_pre_key(pre_key_id, &pre_key_record)
            .await
            .map_err(|e| SignalError::StorageError(format!("Failed to save pre-key: {:?}", e)))?;
        
        pre_keys.push(pre_key_record);
    }

    // Generate Kyber pre-keys
    for i in 0..kyber_pre_key_count {
        let pre_key_id = (((pq_pre_keys_offset_id + i) % (PRE_KEY_MEDIUM_MAX_VALUE - 1)) + 1).into();
        let pre_key_record = KyberPreKeyRecord::generate(
            kem::KeyType::Kyber1024,
            pre_key_id,
            identity_key_pair.private_key(),
        ).map_err(|e| SignalError::ProtocolError(format!("Failed to generate Kyber pre-key: {:?}", e)))?;
        
        protocol_store
            .save_kyber_pre_key(pre_key_id, &pre_key_record)
            .await
            .map_err(|e| SignalError::StorageError(format!("Failed to save Kyber pre-key: {:?}", e)))?;
        
        pq_pre_keys.push(pre_key_record);
    }

    // Generate signed pre-key
    let signed_pre_key_pair = KeyPair::generate(csprng);
    let signed_pre_key_public = signed_pre_key_pair.public_key;
    let signed_pre_key_signature = identity_key_pair
        .private_key()
        .calculate_signature(&signed_pre_key_public.serialize(), csprng)
        .map_err(|e| SignalError::ProtocolError(format!("Failed to sign pre-key: {:?}", e)))?;

    let signed_prekey_record = SignedPreKeyRecord::new(
        next_signed_pre_key_id.into(),
        timestamp_now(),
        &signed_pre_key_pair,
        &signed_pre_key_signature,
    );

    protocol_store
        .save_signed_pre_key(next_signed_pre_key_id.into(), &signed_prekey_record)
        .await
        .map_err(|e| SignalError::StorageError(format!("Failed to save signed pre-key: {:?}", e)))?;

    // Generate last resort Kyber key if requested
    let pq_last_resort_key = if use_last_resort_key {
        let pre_key_id = (((pq_pre_keys_offset_id + kyber_pre_key_count)
            % (PRE_KEY_MEDIUM_MAX_VALUE - 1)) + 1).into();

        let pre_key_record = KyberPreKeyRecord::generate(
            kem::KeyType::Kyber1024,
            pre_key_id,
            identity_key_pair.private_key(),
        ).map_err(|e| SignalError::ProtocolError(format!("Failed to generate last resort Kyber key: {:?}", e)))?;
        
        protocol_store
            .store_last_resort_kyber_pre_key(pre_key_id, &pre_key_record)
            .await
            .map_err(|e| SignalError::StorageError(format!("Failed to save last resort Kyber key: {:?}", e)))?;
        
        Some(pre_key_record)
    } else {
        None
    };

    Ok((pre_keys, signed_prekey_record, pq_pre_keys, pq_last_resort_key))
}
