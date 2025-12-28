use parking_lot::Mutex;
use presage::libsignal_service::{
    configuration::{SignalServers, ServiceConfiguration},
    proto::{ProvisionEnvelope, ProvisioningAddress, 
            web_socket_message, WebSocketMessage, WebSocketRequestMessage, WebSocketResponseMessage},
    provisioning::ProvisioningCipher,
    push_service::PushService,
    protocol::KeyPair,
};
use bytes::Bytes;
use prost::Message;
use url::Url;
use base64::Engine;
use futures::{SinkExt, StreamExt};
use reqwest::Certificate;
use reqwest_websocket::{Message as WsMessage, RequestBuilderExt};

use crate::signal::SignalError;

/// Signal's root CA certificate for TLS verification
const SIGNAL_CA_CERT: &[u8] = include_bytes!("../../certs/signal-ca.pem");

static CAPTURED_BACKUP_KEY: Mutex<Option<CapturedProvisioningData>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub struct CapturedProvisioningData {
    pub ephemeral_backup_key: Option<Vec<u8>>,
    pub master_key: Option<Vec<u8>>,
    pub media_root_backup_key: Option<Vec<u8>>,
}

impl CapturedProvisioningData {
    pub fn new() -> Self {
        Self {
            ephemeral_backup_key: None,
            master_key: None,
            media_root_backup_key: None,
        }
    }
}

pub fn store_captured_data(data: CapturedProvisioningData) {
    let mut guard = CAPTURED_BACKUP_KEY.lock();
    *guard = Some(data);
}

pub fn take_captured_data() -> Option<CapturedProvisioningData> {
    let mut guard = CAPTURED_BACKUP_KEY.lock();
    guard.take()
}

pub fn has_backup_key() -> bool {
    let guard = CAPTURED_BACKUP_KEY.lock();
    guard.as_ref()
        .map(|d| d.ephemeral_backup_key.is_some())
        .unwrap_or(false)
}

pub fn get_ephemeral_backup_key() -> Option<Vec<u8>> {
    let guard = CAPTURED_BACKUP_KEY.lock();
    guard.as_ref()
        .and_then(|d| d.ephemeral_backup_key.clone())
}

#[derive(Debug, Clone)]
pub struct FullProvisionMessage {
    pub phone_number: String,
    pub aci: Option<String>,
    pub pni: Option<String>,
    pub provisioning_code: String,
    pub aci_identity_key_public: Vec<u8>,
    pub aci_identity_key_private: Vec<u8>,
    pub pni_identity_key_public: Vec<u8>,
    pub pni_identity_key_private: Vec<u8>,
    pub profile_key: Vec<u8>,
    pub ephemeral_backup_key: Option<Vec<u8>>,
    pub master_key: Option<Vec<u8>>,
    pub media_root_backup_key: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum ProvisioningResult {
    Url(Url),
    Message(FullProvisionMessage),
}

const BASE64_RELAXED: base64::engine::GeneralPurpose = base64::engine::GeneralPurpose::new(
    &base64::alphabet::STANDARD,
    base64::engine::GeneralPurposeConfig::new()
        .with_decode_padding_mode(base64::engine::DecodePaddingMode::Indifferent),
);

/// Run provisioning flow with immediate URL callback.
/// 
/// The `on_url` callback is invoked as soon as the provisioning URL is available,
/// allowing the caller to display the QR code immediately before the user scans it.
/// The function then continues waiting for the provision message from the primary device.
pub async fn run_provisioning_capture<F>(
    signal_servers: SignalServers,
    on_url: F,
) -> Result<FullProvisionMessage, SignalError>
where
    F: FnOnce(Url),
{
    let service_configuration: ServiceConfiguration = signal_servers.into();
    let push_service = PushService::new(service_configuration, None, "signal-tauri");
    
    let ws_url = build_provisioning_ws_url(&push_service)?;
    
    let signal_ca = Certificate::from_pem(SIGNAL_CA_CERT)
        .map_err(|e| SignalError::NetworkError(format!("Invalid Signal CA certificate: {}", e)))?;
    
    let client = reqwest::Client::builder()
        .user_agent("Signal-Desktop/7.0.0 Linux")
        .add_root_certificate(signal_ca)
        .build()
        .map_err(|e| SignalError::NetworkError(format!("Failed to build client: {}", e)))?;
    
    tracing::debug!("Connecting to WebSocket: {}", ws_url);
    
    let response = client
        .get(&ws_url)
        .header("X-Signal-Agent", "Signal-Desktop/7.0.0")
        .upgrade()
        .send()
        .await
        .map_err(|e| SignalError::NetworkError(format!("WebSocket upgrade request failed: {}", e)))?;
    
    tracing::debug!("WebSocket upgrade response status: {}", response.status());
    
    let mut ws = response
        .into_websocket()
        .await
        .map_err(|e| SignalError::NetworkError(format!("WebSocket connection failed: {}", e)))?;
    
    tracing::info!("WebSocket connection established");
    
    let mut rng = rand::rng();
    let key_pair = KeyPair::generate(&mut rng);
    let cipher = ProvisioningCipher::from_key_pair(key_pair);
    
    let mut url_callback: Option<F> = Some(on_url);
    let mut full_message: Option<FullProvisionMessage> = None;
    
    while let Some(msg) = ws.next().await {
        let msg = msg.map_err(|e| SignalError::NetworkError(e.to_string()))?;
        
        match msg {
            WsMessage::Binary(data) => {
                let ws_msg = WebSocketMessage::decode(Bytes::from(data))
                    .map_err(|e| SignalError::ProtocolError(e.to_string()))?;
                
                if let Some(request) = ws_msg.request {
                    let request_id = request.id;
                    let result = process_provisioning_request(&cipher, request).await?;
                    
                    let response = WebSocketResponseMessage {
                        id: request_id,
                        status: Some(200),
                        message: Some("OK".into()),
                        body: None,
                        headers: vec![],
                    };
                    
                    let response_msg = WebSocketMessage {
                        r#type: Some(web_socket_message::Type::Response as i32),
                        request: None,
                        response: Some(response),
                    };
                    
                    let encoded = response_msg.encode_to_vec();
                    ws.send(WsMessage::Binary(encoded.into()))
                        .await
                        .map_err(|e| SignalError::NetworkError(e.to_string()))?;
                    
                    match result {
                        ProvisioningResult::Url(url) => {
                            if let Some(callback) = url_callback.take() {
                                tracing::info!("Provisioning URL available, invoking callback");
                                callback(url);
                            }
                        }
                        ProvisioningResult::Message(msg) => {
                            let captured = CapturedProvisioningData {
                                ephemeral_backup_key: msg.ephemeral_backup_key.clone(),
                                master_key: msg.master_key.clone(),
                                media_root_backup_key: msg.media_root_backup_key.clone(),
                            };
                            store_captured_data(captured);
                            
                            full_message = Some(msg);
                            break;
                        }
                    }
                }
            }
            WsMessage::Close { .. } => {
                break;
            }
            _ => {}
        }
    }
    
    full_message.ok_or_else(|| SignalError::ProtocolError("Provisioning incomplete - no message received".into()))
}

async fn process_provisioning_request(
    cipher: &ProvisioningCipher,
    request: WebSocketRequestMessage,
) -> Result<ProvisioningResult, SignalError> {
    let verb = request.verb.as_deref().unwrap_or("");
    let path = request.path.as_deref().unwrap_or("");
    
    match (verb, path) {
        ("PUT", "/v1/address") => {
            let body = request.body.ok_or_else(|| 
                SignalError::ProtocolError("Missing body in address message".into()))?;
            
            let address = ProvisioningAddress::decode(Bytes::from(body))
                .map_err(|e| SignalError::ProtocolError(e.to_string()))?;
            
            let uuid = address.address.ok_or_else(||
                SignalError::ProtocolError("Missing UUID in address".into()))?;
            
            let mut url = Url::parse("sgnl://linkdevice")
                .map_err(|e| SignalError::ProtocolError(e.to_string()))?;
            
            url.query_pairs_mut()
                .append_pair("uuid", &uuid)
                .append_pair("pub_key", &BASE64_RELAXED.encode(cipher.public_key().serialize()))
                .append_pair("capabilities", "backup4,backup5");
            
            tracing::info!("Generated provisioning URL with backup capabilities: backup4,backup5");
            
            Ok(ProvisioningResult::Url(url))
        }
        ("PUT", "/v1/message") => {
            let body = request.body.ok_or_else(||
                SignalError::ProtocolError("Missing body in message".into()))?;
            
            let envelope = ProvisionEnvelope::decode(Bytes::from(body))
                .map_err(|e| SignalError::ProtocolError(e.to_string()))?;
            
            let message = cipher.decrypt(envelope)
                .map_err(|e| SignalError::ProtocolError(format!("Decryption failed: {:?}", e)))?;
            
            tracing::info!(
                "Captured provisioning message with ephemeral_backup_key: {}",
                message.ephemeral_backup_key.is_some()
            );
            
            let full_msg = FullProvisionMessage {
                phone_number: message.number.ok_or_else(||
                    SignalError::ProtocolError("Missing phone number".into()))?,
                aci: message.aci,
                pni: message.pni,
                provisioning_code: message.provisioning_code.ok_or_else(||
                    SignalError::ProtocolError("Missing provisioning code".into()))?,
                aci_identity_key_public: message.aci_identity_key_public.ok_or_else(||
                    SignalError::ProtocolError("Missing ACI public key".into()))?,
                aci_identity_key_private: message.aci_identity_key_private.ok_or_else(||
                    SignalError::ProtocolError("Missing ACI private key".into()))?,
                pni_identity_key_public: message.pni_identity_key_public.ok_or_else(||
                    SignalError::ProtocolError("Missing PNI public key".into()))?,
                pni_identity_key_private: message.pni_identity_key_private.ok_or_else(||
                    SignalError::ProtocolError("Missing PNI private key".into()))?,
                profile_key: message.profile_key.ok_or_else(||
                    SignalError::ProtocolError("Missing profile key".into()))?,
                ephemeral_backup_key: message.ephemeral_backup_key,
                master_key: message.master_key,
                media_root_backup_key: message.media_root_backup_key,
            };
            
            Ok(ProvisioningResult::Message(full_msg))
        }
        _ => Err(SignalError::ProtocolError(format!("Unknown request: {} {}", verb, path))),
    }
}

fn build_provisioning_ws_url(push_service: &PushService) -> Result<String, SignalError> {
    Ok("wss://chat.signal.org/v1/websocket/provisioning/".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captured_data_storage() {
        let _ = take_captured_data();
        
        let data = CapturedProvisioningData {
            ephemeral_backup_key: Some(vec![1, 2, 3, 4]),
            master_key: Some(vec![5, 6, 7, 8]),
            media_root_backup_key: None,
        };
        
        store_captured_data(data.clone());
        assert!(has_backup_key());
        
        let retrieved = take_captured_data();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().ephemeral_backup_key, Some(vec![1, 2, 3, 4]));
        
        assert!(!has_backup_key());
    }

    #[test]
    fn test_no_backup_key() {
        let _ = take_captured_data();
        assert!(!has_backup_key());
        assert!(get_ephemeral_backup_key().is_none());
    }

    #[test]
    fn test_provisioning_url_format() {
        let mut url = Url::parse("sgnl://linkdevice").unwrap();
        let test_uuid = "test-uuid-1234";
        let test_pubkey = "dGVzdC1wdWJrZXk=";
        
        url.query_pairs_mut()
            .append_pair("uuid", test_uuid)
            .append_pair("pub_key", test_pubkey)
            .append_pair("capabilities", "backup4,backup5");
        
        let url_str = url.to_string();
        assert!(url_str.contains("uuid=test-uuid-1234"));
        assert!(url_str.contains("pub_key="));
        assert!(url_str.contains("capabilities=backup4%2Cbackup5"));
    }
}
