//! Signal-Tauri: A native Rust Signal Desktop client
//!
//! This application provides a full-featured Signal messaging client using:
//! - egui for native UI rendering
//! - presage for Signal protocol implementation
//! - sled for encrypted local storage

#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod services;
mod signal;
mod storage;
mod ui;

use anyhow::Result;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() -> Result<()> {
    // macOS: Create a runtime app bundle trampoline so the app appears in the Dock
    // This is necessary because macOS only shows apps in the Dock if they're running
    // from a proper .app bundle. The trampoline creates a temporary bundle at runtime.
    #[cfg(target_os = "macos")]
    {
        use fruitbasket::ActivationPolicy;
        use fruitbasket::Trampoline;
        use fruitbasket::FruitApp;

        // Build the trampoline - this will relaunch the app from a bundle if needed
        let _ = Trampoline::new("Signal-Tauri", "signal-tauri", "com.signal-tauri.dev")
            .version(env!("CARGO_PKG_VERSION"))
            .plist_key("LSUIElement", "NO")  // Show in Dock
            .build(fruitbasket::InstallDir::Temp)
            .ok();

        // Create FruitApp instance and set activation policy
        let app = FruitApp::new();
        app.set_activation_policy(ActivationPolicy::Regular);
    }

    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("signal_tauri=debug,presage=debug,warn")
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Signal-Tauri v{}", env!("CARGO_PKG_VERSION"));

    // Configure native options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Signal-Tauri DEV")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(load_icon())
            .with_taskbar(true) // Show in taskbar/dock
            .with_active(true), // Ensure window is active and focused
        vsync: true,
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        "Signal-Tauri DEV",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::SignalApp::new(cc)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))
}

/// Load the application icon
fn load_icon() -> egui::IconData {
    // Signal blue icon placeholder - will be replaced with actual icon
    let size = 64;
    let mut rgba = vec![0u8; size * size * 4];

    // Create a simple Signal-blue circle
    let center = size as f32 / 2.0;
    let radius = center * 0.8;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            let idx = (y * size + x) * 4;
            if dist <= radius {
                // Signal Blue: #2C6BED
                rgba[idx] = 0x2C;     // R
                rgba[idx + 1] = 0x6B; // G
                rgba[idx + 2] = 0xED; // B
                rgba[idx + 3] = 0xFF; // A
            }
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
