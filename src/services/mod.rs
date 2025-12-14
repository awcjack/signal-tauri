//! Background services and utilities

pub mod notifications;
pub mod sync;
pub mod updates;

use std::sync::Arc;
use tokio::sync::mpsc;

/// Service manager for background tasks
pub struct ServiceManager {
    /// Shutdown signal sender
    shutdown_tx: mpsc::Sender<()>,

    /// Shutdown signal receiver
    shutdown_rx: Option<mpsc::Receiver<()>>,
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new() -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        Self {
            shutdown_tx,
            shutdown_rx: Some(shutdown_rx),
        }
    }

    /// Start all background services
    pub async fn start(&mut self) -> anyhow::Result<()> {
        tracing::info!("Starting background services...");

        // TODO: Start notification service
        // TODO: Start sync service
        // TODO: Start update checker

        Ok(())
    }

    /// Shutdown all services
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        tracing::info!("Shutting down background services...");

        // Send shutdown signal
        let _ = self.shutdown_tx.send(()).await;

        Ok(())
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::new()
    }
}
