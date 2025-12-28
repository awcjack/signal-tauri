use crate::signal::SignalError;
use reqwest::Certificate;
use serde::Deserialize;
use std::time::Duration;

const SIGNAL_API_BASE: &str = "https://chat.signal.org";
const SIGNAL_CDN2_BASE: &str = "https://cdn2.signal.org";
const SIGNAL_CDN3_BASE: &str = "https://cdn3.signal.org";
const SIGNAL_CA_CERT: &[u8] = include_bytes!("../../../certs/signal-ca.pem");

#[derive(Debug, Deserialize)]
pub struct TransferArchiveInfo {
    pub cdn: u32,
    pub key: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TransferArchiveResponse {
    Success(TransferArchiveInfo),
    Error { error: String },
}

fn build_signal_client() -> Result<reqwest::Client, SignalError> {
    let signal_ca = Certificate::from_pem(SIGNAL_CA_CERT)
        .map_err(|e| SignalError::NetworkError(format!("Invalid Signal CA certificate: {}", e)))?;
    
    reqwest::Client::builder()
        .user_agent("Signal-Desktop/7.0.0 Linux")
        .add_root_certificate(signal_ca)
        .timeout(Duration::from_secs(330))
        .build()
        .map_err(|e| SignalError::NetworkError(format!("Failed to build HTTP client: {}", e)))
}

pub async fn fetch_transfer_archive(
    username: &str,
    password: &str,
) -> Result<TransferArchiveInfo, SignalError> {
    let client = build_signal_client()?;
    
    let timeout_secs = 300;
    let url = format!("{}/v1/devices/transfer_archive?timeout={}", SIGNAL_API_BASE, timeout_secs);
    
    tracing::info!("Fetching transfer archive with {}s timeout...", timeout_secs);
    
    let response = client
        .get(&url)
        .basic_auth(username, Some(password))
        .header("X-Signal-Agent", "Signal-Desktop/7.0.0")
        .send()
        .await
        .map_err(|e| SignalError::NetworkError(format!("Failed to fetch transfer archive: {}", e)))?;
    
    let status = response.status();
    tracing::debug!("Transfer archive response status: {}", status);
    
    if status == reqwest::StatusCode::NO_CONTENT {
        return Err(SignalError::NetworkError(
            "Transfer archive not ready yet (204), phone may still be uploading".into()
        ));
    }
    
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(SignalError::NetworkError(format!(
            "Transfer archive request failed with status {}: {}",
            status, body
        )));
    }
    
    let archive_response: TransferArchiveResponse = response
        .json()
        .await
        .map_err(|e| SignalError::ProtocolError(format!("Failed to parse transfer archive response: {}", e)))?;
    
    match archive_response {
        TransferArchiveResponse::Success(info) => {
            tracing::info!("Transfer archive available on CDN {}", info.cdn);
            Ok(info)
        }
        TransferArchiveResponse::Error { error } => {
            Err(SignalError::ProtocolError(format!("Transfer archive error: {}", error)))
        }
    }
}

pub async fn download_backup(archive_info: &TransferArchiveInfo) -> Result<Vec<u8>, SignalError> {
    let client = build_signal_client()?;
    
    let cdn_base = match archive_info.cdn {
        2 => SIGNAL_CDN2_BASE,
        3 => SIGNAL_CDN3_BASE,
        n => return Err(SignalError::ProtocolError(format!("Unknown CDN number: {}", n))),
    };
    
    // Transfer archives use /attachments/ path (ephemeral backups), not /backups/
    let encoded_key = urlencoding::encode(&archive_info.key);
    let url = format!("{}/attachments/{}", cdn_base, encoded_key);
    
    tracing::info!("Downloading transfer archive from CDN {} with key prefix {}...", 
        archive_info.cdn, 
        &archive_info.key[..std::cmp::min(20, archive_info.key.len())]);
    
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| SignalError::NetworkError(format!("Failed to download backup: {}", e)))?;
    
    if !response.status().is_success() {
        let status = response.status();
        return Err(SignalError::NetworkError(format!(
            "Backup download failed with status {}",
            status
        )));
    }
    
    let bytes = response
        .bytes()
        .await
        .map_err(|e| SignalError::NetworkError(format!("Failed to read backup bytes: {}", e)))?;
    
    tracing::info!("Downloaded {} bytes from backup", bytes.len());
    
    Ok(bytes.to_vec())
}
