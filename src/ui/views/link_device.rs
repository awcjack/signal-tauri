//! Device linking view - shows QR code for linking to existing Signal account

use crate::app::{LinkingState, SignalApp};
use crate::ui::theme::SignalColors;
use egui::{Color32, Rect, Rounding, Sense, Vec2};

/// Show the device linking view
pub fn show(app: &mut SignalApp, ctx: &egui::Context) {
    // Start linking if not already started
    app.start_linking();

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
            let (rect, _response) = ui.allocate_exact_size(
                Vec2::new(qr_size, qr_size),
                Sense::hover(),
            );

            // Draw QR code background
            ui.painter().rect_filled(
                rect,
                Rounding::same(12.0),
                Color32::WHITE,
            );

            // Show content based on linking state
            match app.linking_state() {
                LinkingState::NotStarted | LinkingState::Generating => {
                    // Show loading spinner/placeholder
                    draw_loading_placeholder(ui, rect);
                    ui.add_space(24.0);
                    ui.colored_label(
                        SignalColors::TEXT_SECONDARY,
                        "Generating QR code...",
                    );
                }
                LinkingState::WaitingForScan { qr_texture, .. } => {
                    // Show actual QR code
                    if let Some(texture) = qr_texture {
                        let qr_rect = Rect::from_center_size(
                            rect.center(),
                            Vec2::splat(qr_size - 20.0),
                        );
                        ui.painter().image(
                            texture.id(),
                            qr_rect,
                            Rect::from_min_max(
                                egui::pos2(0.0, 0.0),
                                egui::pos2(1.0, 1.0),
                            ),
                            Color32::WHITE,
                        );
                    } else {
                        draw_placeholder_qr(ui, rect);
                    }
                    ui.add_space(24.0);
                    ui.colored_label(
                        SignalColors::TEXT_SECONDARY,
                        "Waiting for phone to scan QR code...",
                    );
                }
                LinkingState::Processing => {
                    draw_loading_placeholder(ui, rect);
                    ui.add_space(24.0);
                    ui.colored_label(
                        SignalColors::SIGNAL_BLUE,
                        "Processing link...",
                    );
                }
                LinkingState::Success => {
                    // Show success - this shouldn't normally show as we transition to ChatList
                    ui.painter().circle_filled(
                        rect.center(),
                        60.0,
                        SignalColors::SIGNAL_BLUE,
                    );
                    ui.add_space(24.0);
                    ui.colored_label(
                        Color32::GREEN,
                        "Device linked successfully!",
                    );
                }
                LinkingState::Error(error) => {
                    // Show error state
                    draw_error_state(ui, rect);
                    ui.add_space(24.0);
                    ui.colored_label(
                        Color32::RED,
                        format!("Error: {}", error),
                    );
                }
            }

            ui.add_space(16.0);

            // Retry button (only show on error)
            if matches!(app.linking_state(), LinkingState::Error(_)) {
                if ui.button("Retry").clicked() {
                    app.retry_linking();
                }
            }
        });
    });
}

/// Draw a loading placeholder
fn draw_loading_placeholder(ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter();
    let center = rect.center();

    // Draw spinning circles animation
    let time = ui.input(|i| i.time);
    let num_dots = 8;
    let radius = 40.0;
    let dot_radius = 6.0;

    for i in 0..num_dots {
        let angle = (i as f64 / num_dots as f64) * std::f64::consts::TAU + time * 2.0;
        let x = center.x + (angle.cos() * radius) as f32;
        let y = center.y + (angle.sin() * radius) as f32;

        // Fade based on position in rotation
        let alpha = ((i as f64 / num_dots as f64 + time.fract()) % 1.0) as f32;
        let color = Color32::from_rgba_unmultiplied(
            SignalColors::SIGNAL_BLUE.r(),
            SignalColors::SIGNAL_BLUE.g(),
            SignalColors::SIGNAL_BLUE.b(),
            (alpha * 255.0) as u8,
        );

        painter.circle_filled(egui::pos2(x, y), dot_radius, color);
    }
}

/// Draw a placeholder QR code pattern (for visual testing)
fn draw_placeholder_qr(ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter();
    let module_count = 29;
    let module_size = (rect.width() - 20.0) / module_count as f32;
    let start = rect.min + Vec2::new(10.0, 10.0);

    for row in 0..module_count {
        for col in 0..module_count {
            let is_finder = (row < 7 && col < 7)
                || (row < 7 && col >= module_count - 7)
                || (row >= module_count - 7 && col < 7);

            let is_timing = (row == 6 || col == 6) && !is_finder;
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

/// Draw error state
fn draw_error_state(ui: &egui::Ui, rect: Rect) {
    let painter = ui.painter();
    let center = rect.center();

    // Draw X mark
    painter.circle_filled(center, 50.0, Color32::from_rgb(255, 100, 100));
    let stroke = egui::Stroke::new(6.0, Color32::WHITE);
    let offset = 20.0;
    painter.line_segment(
        [
            egui::pos2(center.x - offset, center.y - offset),
            egui::pos2(center.x + offset, center.y + offset),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(center.x + offset, center.y - offset),
            egui::pos2(center.x - offset, center.y + offset),
        ],
        stroke,
    );
}

/// Render a QR code from data
pub fn render_qr_code(data: &str) -> Option<egui::ColorImage> {
    use qrcode::QrCode;

    let code = QrCode::new(data.as_bytes()).ok()?;
    let modules = code.to_colors();
    let size = (modules.len() as f64).sqrt() as usize;

    // Add quiet zone (border)
    let quiet_zone = 4;
    let scale = 8; // Scale up for better visibility
    let full_size = (size + quiet_zone * 2) * scale;

    let mut pixels = vec![Color32::WHITE; full_size * full_size];

    for (i, module) in modules.iter().enumerate() {
        let row = i / size;
        let col = i % size;

        let is_dark = *module == qrcode::Color::Dark;
        let color = if is_dark { Color32::BLACK } else { Color32::WHITE };

        // Scale up and apply quiet zone offset
        for dy in 0..scale {
            for dx in 0..scale {
                let x = (col + quiet_zone) * scale + dx;
                let y = (row + quiet_zone) * scale + dy;
                if x < full_size && y < full_size {
                    pixels[y * full_size + x] = color;
                }
            }
        }
    }

    Some(egui::ColorImage {
        size: [full_size, full_size],
        pixels,
    })
}
