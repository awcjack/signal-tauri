//! Avatar component for displaying user/group profile images

use egui::{Color32, Pos2, Rect, Rounding, Vec2};

/// Avatar display component
pub struct Avatar {
    /// Size in pixels
    pub size: f32,
    /// Background color
    pub color: Color32,
    /// Initials to display (if no image)
    pub initials: String,
    /// Image texture (if available)
    pub image: Option<egui::TextureHandle>,
    /// Whether to show online indicator
    pub show_online: bool,
    /// Whether the user is online
    pub is_online: bool,
}

impl Avatar {
    /// Create a new avatar with initials
    pub fn new(initials: impl Into<String>, color: Color32) -> Self {
        Self {
            size: 40.0,
            color,
            initials: initials.into(),
            image: None,
            show_online: false,
            is_online: false,
        }
    }

    /// Set avatar size
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set image texture
    pub fn image(mut self, image: egui::TextureHandle) -> Self {
        self.image = Some(image);
        self
    }

    /// Show online indicator
    pub fn with_online_indicator(mut self, is_online: bool) -> Self {
        self.show_online = true;
        self.is_online = is_online;
        self
    }

    /// Show the avatar
    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::splat(self.size),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let center = rect.center();
            let radius = self.size / 2.0;

            // Draw avatar circle
            if let Some(image) = &self.image {
                // Draw image
                painter.image(
                    image.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                // Draw colored circle with initials
                painter.circle_filled(center, radius, self.color);

                // Draw initials
                let font_size = self.size * 0.4;
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    &self.initials,
                    egui::FontId::proportional(font_size),
                    Color32::WHITE,
                );
            }

            // Draw online indicator
            if self.show_online {
                let indicator_radius = self.size * 0.15;
                let indicator_center = center + Vec2::new(
                    radius * 0.7,
                    radius * 0.7,
                );

                // White border
                painter.circle_filled(
                    indicator_center,
                    indicator_radius + 2.0,
                    Color32::from_gray(30),
                );

                // Status color
                let status_color = if self.is_online {
                    Color32::from_rgb(0x4C, 0xAF, 0x50) // Green
                } else {
                    Color32::GRAY
                };
                painter.circle_filled(indicator_center, indicator_radius, status_color);
            }
        }

        response
    }
}

/// Generate a color from a string (for consistent avatar colors)
pub fn color_from_string(s: &str) -> Color32 {
    let hash = s.bytes().fold(0u32, |acc, b| acc.wrapping_add(b as u32).wrapping_mul(31));

    let colors = [
        Color32::from_rgb(0xE9, 0x1E, 0x63), // Pink
        Color32::from_rgb(0x9C, 0x27, 0xB0), // Purple
        Color32::from_rgb(0x67, 0x3A, 0xB7), // Deep Purple
        Color32::from_rgb(0x3F, 0x51, 0xB5), // Indigo
        Color32::from_rgb(0x21, 0x96, 0xF3), // Blue
        Color32::from_rgb(0x00, 0x96, 0x88), // Teal
        Color32::from_rgb(0x4C, 0xAF, 0x50), // Green
        Color32::from_rgb(0xFF, 0x98, 0x00), // Orange
        Color32::from_rgb(0xFF, 0x57, 0x22), // Deep Orange
        Color32::from_rgb(0x79, 0x55, 0x48), // Brown
    ];

    colors[(hash as usize) % colors.len()]
}
