//! Device linking view - shows QR code for linking to existing Signal account

use crate::app::SignalApp;
use crate::signal::SignalManager;
use crate::ui::theme::SignalColors;
use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use std::sync::Arc;
use tokio::sync::mpsc;

/// State for device linking process
#[derive(Debug, Clone)]
pub enum LinkingState {
    /// Generating QR code
    Generating,
    /// Displaying QR code, waiting for scan
    WaitingForScan { qr_data: String, provisioning_url: String },
    /// User scanned, processing link
    Processing,
    /// Successfully linked
    Success,
    /// Error occurred
    Error(String),
}

/// Device linking view state
pub struct LinkDeviceView {
    state: LinkingState,
    device_name: String,
}

impl Default for LinkDeviceView {
    fn default() -> Self {
        Self {
            state: LinkingState::Generating,
            device_name: get_device_name(),
        }
    }
}

/// Get a default device name
fn get_device_name() -> String {
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "Desktop".to_string());

    format!("Signal Desktop ({})", hostname)
}

/// Show the device linking view
pub fn show(app: &mut SignalApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            // Signal logo/title
            ui.heading("Signal");
            ui.add_space(8.0);
            ui.label("Link your phone to Signal Desktop");
            ui.add_space(40.0);

            // Instructions
            ui.group(|ui| {
                ui.set_width(500.0);
                ui.vertical(|ui| {
                    ui.label("1. Open Signal on your phone");
                    ui.add_space(4.0);
                    ui.label("2. Go to Settings → Linked Devices");
                    ui.add_space(4.0);
                    ui.label("3. Tap the + button to add a new device");
                    ui.add_space(4.0);
                    ui.label("4. Scan the QR code below");
                });
            });

            ui.add_space(32.0);

            // QR Code display area
            let qr_size = 280.0;
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(qr_size, qr_size),
                Sense::hover(),
            );

            // Draw QR code background
            ui.painter().rect_filled(
                rect,
                Rounding::same(12.0),
                Color32::WHITE,
            );

            // Draw placeholder QR code pattern (will be replaced with real QR)
            draw_placeholder_qr(ui, rect);

            ui.add_space(24.0);

            // Device name input
            ui.horizontal(|ui| {
                ui.label("Device name:");
                // We'd have a text input here for device name
            });

            ui.add_space(16.0);

            // Status message
            ui.colored_label(
                SignalColors::TEXT_SECONDARY,
                "Waiting for phone to scan QR code...",
            );

            ui.add_space(40.0);

            // Generate new QR button
            if ui.button("Generate New QR Code").clicked() {
                start_linking_process(app);
            }
        });
    });

    // Start linking process if not already started
    static LINKING_STARTED: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    if !LINKING_STARTED.load(std::sync::atomic::Ordering::SeqCst) {
        LINKING_STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
        start_linking_process(app);
    }
}

/// Draw a placeholder QR code pattern
fn draw_placeholder_qr(ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter();
    let module_count = 29; // Standard QR code size
    let module_size = (rect.width() - 20.0) / module_count as f32;
    let start = rect.min + Vec2::new(10.0, 10.0);

    // Draw a pattern that looks like a QR code
    for row in 0..module_count {
        for col in 0..module_count {
            // Draw finder patterns (corners)
            let is_finder = (row < 7 && col < 7)
                || (row < 7 && col >= module_count - 7)
                || (row >= module_count - 7 && col < 7);

            // Draw timing patterns
            let is_timing = (row == 6 || col == 6) && !is_finder;

            // Random data pattern for visual effect
            let is_data = !is_finder && !is_timing && ((row * col * 7) % 3 == 0);

            if is_finder || is_timing || is_data {
                let pos = start + Vec2::new(col as f32 * module_size, row as f32 * module_size);
                painter.rect_filled(
                    Rect::from_min_size(pos, Vec2::splat(module_size * 0.9)),
                    Rounding::ZERO,
                    Color32::BLACK,
                );
            }
        }
    }

    // Draw Signal logo in center
    let center = rect.center();
    let logo_size = 40.0;
    painter.rect_filled(
        Rect::from_center_size(center, Vec2::splat(logo_size + 8.0)),
        Rounding::same(4.0),
        Color32::WHITE,
    );
    painter.circle_filled(center, logo_size / 2.0 - 4.0, SignalColors::SIGNAL_BLUE);
}

/// Start the device linking process
fn start_linking_process(app: &mut SignalApp) {
    let runtime = app.runtime().clone();
    let storage = app.storage().clone();

    runtime.spawn(async move {
        tracing::info!("Starting device linking process...");

        // TODO: Implement actual linking with presage
        // This will:
        // 1. Generate provisioning URL
        // 2. Create QR code from URL
        // 3. Wait for phone to scan
        // 4. Complete provisioning handshake
        // 5. Store credentials

        match SignalManager::link_device(&storage, "Signal Desktop").await {
            Ok(manager) => {
                tracing::info!("Device linked successfully!");
                // Update app state - need to communicate back to main thread
            }
            Err(e) => {
                tracing::error!("Failed to link device: {}", e);
            }
        }
    });
}

/// Render a QR code from data
pub fn render_qr_code(data: &str) -> Option<egui::ColorImage> {
    use qrcode::{QrCode, Color as QrColor};

    let code = QrCode::new(data.as_bytes()).ok()?;
    let image = code.render::<QrColor>().build();

    let width = image.width() as usize;
    let height = image.height() as usize;

    let pixels: Vec<Color32> = image
        .pixels()
        .map(|p| {
            if *p == QrColor::Dark {
                Color32::BLACK
            } else {
                Color32::WHITE
            }
        })
        .collect();

    Some(egui::ColorImage {
        size: [width, height],
        pixels,
    })
}
