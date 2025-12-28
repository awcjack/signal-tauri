use crate::signal::SignalError;
use aes::cipher::{BlockDecryptMut, KeyIvInit};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;
type HmacSha256 = Hmac<Sha256>;

const BACKUP_ID_LEN: usize = 16;
const AES_KEY_LEN: usize = 32;
const HMAC_KEY_LEN: usize = 32;
const IV_LEN: usize = 16;
const MAC_LEN: usize = 32;

struct DerivedKeys {
    aes_key: [u8; AES_KEY_LEN],
    hmac_key: [u8; HMAC_KEY_LEN],
}

fn derive_backup_id(backup_key: &[u8], aci: &Uuid) -> [u8; BACKUP_ID_LEN] {
    const INFO: &[u8] = b"20241024_SIGNAL_BACKUP_ID:";
    
    let aci_bytes = aci.as_bytes();
    
    let mut backup_id = [0u8; BACKUP_ID_LEN];
    let hkdf = Hkdf::<Sha256>::new(None, backup_key);
    
    let mut info_with_aci = Vec::with_capacity(INFO.len() + aci_bytes.len());
    info_with_aci.extend_from_slice(INFO);
    info_with_aci.extend_from_slice(aci_bytes);
    
    hkdf.expand(&info_with_aci, &mut backup_id)
        .expect("valid HKDF output length");
    
    backup_id
}

fn derive_message_backup_keys(backup_key: &[u8], backup_id: &[u8; BACKUP_ID_LEN]) -> DerivedKeys {
    const INFO: &[u8] = b"20241007_SIGNAL_BACKUP_ENCRYPT_MESSAGE_BACKUP:";
    
    let mut full_key = [0u8; HMAC_KEY_LEN + AES_KEY_LEN];
    let hkdf = Hkdf::<Sha256>::new(None, backup_key);
    
    let mut info_with_id = Vec::with_capacity(INFO.len() + backup_id.len());
    info_with_id.extend_from_slice(INFO);
    info_with_id.extend_from_slice(backup_id);
    
    hkdf.expand(&info_with_id, &mut full_key)
        .expect("valid HKDF output length");
    
    let mut hmac_key = [0u8; HMAC_KEY_LEN];
    let mut aes_key = [0u8; AES_KEY_LEN];
    
    hmac_key.copy_from_slice(&full_key[..HMAC_KEY_LEN]);
    aes_key.copy_from_slice(&full_key[HMAC_KEY_LEN..]);
    
    DerivedKeys { aes_key, hmac_key }
}

pub fn decrypt_backup(
    encrypted_data: &[u8],
    ephemeral_backup_key: &[u8],
    aci: &Uuid,
) -> Result<Vec<u8>, SignalError> {
    if encrypted_data.len() < IV_LEN + MAC_LEN {
        return Err(SignalError::CryptoError("Encrypted data too short".into()));
    }
    
    tracing::debug!("Decrypting backup: {} bytes, key len: {}", encrypted_data.len(), ephemeral_backup_key.len());
    
    let backup_id = derive_backup_id(ephemeral_backup_key, aci);
    tracing::debug!("Derived backup ID: {:02x?}", &backup_id[..4]);
    
    let keys = derive_message_backup_keys(ephemeral_backup_key, &backup_id);
    tracing::debug!("Derived HMAC key prefix: {:02x?}", &keys.hmac_key[..4]);
    
    let (iv, rest) = encrypted_data.split_at(IV_LEN);
    let (ciphertext, mac) = rest.split_at(rest.len() - MAC_LEN);
    
    tracing::debug!("IV: {:02x?}, ciphertext len: {}, MAC: {:02x?}", 
        &iv[..4], ciphertext.len(), &mac[..4]);
    
    let mut hmac = HmacSha256::new_from_slice(&keys.hmac_key)
        .map_err(|_| SignalError::CryptoError("Invalid HMAC key length".into()))?;
    hmac.update(iv);
    hmac.update(ciphertext);
    
    hmac.verify_slice(mac)
        .map_err(|_| SignalError::CryptoError("HMAC verification failed - backup may be corrupted or key is wrong".into()))?;
    
    tracing::debug!("HMAC verification passed");
    
    let iv_array: [u8; IV_LEN] = iv.try_into()
        .map_err(|_| SignalError::CryptoError("Invalid IV length".into()))?;
    
    let mut buffer = ciphertext.to_vec();
    
    let decryptor = Aes256CbcDec::new_from_slices(&keys.aes_key, &iv_array)
        .map_err(|_| SignalError::CryptoError("Invalid AES key/IV".into()))?;
    
    let decrypted = decryptor
        .decrypt_padded_mut::<aes::cipher::block_padding::Pkcs7>(&mut buffer)
        .map_err(|_| SignalError::CryptoError("AES decryption failed".into()))?;
    
    Ok(decrypted.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_derive_backup_id() {
        let key = [0u8; 32];
        let aci = Uuid::nil();
        let backup_id = derive_backup_id(&key, &aci);
        
        assert_eq!(backup_id.len(), BACKUP_ID_LEN);
    }
    
    #[test]
    fn test_derive_message_backup_keys() {
        let key = [0u8; 32];
        let backup_id = [0u8; BACKUP_ID_LEN];
        let derived = derive_message_backup_keys(&key, &backup_id);
        
        assert_eq!(derived.aes_key.len(), AES_KEY_LEN);
        assert_eq!(derived.hmac_key.len(), HMAC_KEY_LEN);
    }
    
    #[test]
    fn test_decrypt_backup_too_short() {
        let key = [0u8; 32];
        let aci = Uuid::nil();
        let short_data = [0u8; 16];
        
        let result = decrypt_backup(&short_data, &key, &aci);
        assert!(result.is_err());
    }
}
