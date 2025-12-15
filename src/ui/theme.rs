//! Signal-inspired theme for egui

use egui::{Color32, FontFamily, FontId, Rounding, Stroke, Style, TextStyle, Visuals};

/// Signal-inspired color palette
pub struct SignalColors;

impl SignalColors {
    // Primary colors
    pub const SIGNAL_BLUE: Color32 = Color32::from_rgb(0x2C, 0x6B, 0xED);
    pub const SIGNAL_BLUE_HOVER: Color32 = Color32::from_rgb(0x1E, 0x5A, 0xD8);
    pub const SIGNAL_BLUE_PRESSED: Color32 = Color32::from_rgb(0x15, 0x4A, 0xC8);

    // Dark theme colors
    pub const DARK_BG: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1B);
    pub const DARK_SURFACE: Color32 = Color32::from_rgb(0x2D, 0x2D, 0x2D);
    pub const DARK_SURFACE_ELEVATED: Color32 = Color32::from_rgb(0x3D, 0x3D, 0x3D);
    pub const DARK_BORDER: Color32 = Color32::from_rgb(0x4A, 0x4A, 0x4A);

    // Light theme colors
    pub const LIGHT_BG: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    pub const LIGHT_SURFACE: Color32 = Color32::from_rgb(0xF6, 0xF6, 0xF6);
    pub const LIGHT_SURFACE_ELEVATED: Color32 = Color32::from_rgb(0xE9, 0xE9, 0xE9);
    pub const LIGHT_BORDER: Color32 = Color32::from_rgb(0xD9, 0xD9, 0xD9);

    // Text colors
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xB0, 0xB0, 0xB0);
    pub const TEXT_TERTIARY: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);
    pub const TEXT_DARK: Color32 = Color32::from_rgb(0x1B, 0x1B, 0x1B);

    // Message bubble colors
    pub const BUBBLE_SENT: Color32 = Color32::from_rgb(0x2C, 0x6B, 0xED);
    pub const BUBBLE_RECEIVED: Color32 = Color32::from_rgb(0x3D, 0x3D, 0x3D);

    // Status colors
    pub const SUCCESS: Color32 = Color32::from_rgb(0x4C, 0xAF, 0x50);
    pub const WARNING: Color32 = Color32::from_rgb(0xFF, 0xA7, 0x26);
    pub const ERROR: Color32 = Color32::from_rgb(0xF4, 0x43, 0x36);
    pub const INFO: Color32 = Color32::from_rgb(0x21, 0x96, 0xF3);

    // Unread indicator
    pub const UNREAD: Color32 = Color32::from_rgb(0x2C, 0x6B, 0xED);
}

/// Signal theme configuration
pub struct SignalTheme {
    pub is_dark: bool,
}

impl SignalTheme {
    /// Create dark theme
    pub fn dark() -> Self {
        Self { is_dark: true }
    }

    /// Create light theme
    pub fn light() -> Self {
        Self { is_dark: false }
    }

    /// Apply theme to egui context
    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = Style::default();

        // Configure visuals
        let visuals = if self.is_dark {
            self.dark_visuals()
        } else {
            self.light_visuals()
        };
        style.visuals = visuals;

        // Configure text styles
        style.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, FontFamily::Proportional)),
            (TextStyle::Name("heading2".into()), FontId::new(20.0, FontFamily::Proportional)),
            (TextStyle::Name("heading3".into()), FontId::new(16.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();

        // Configure spacing
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.window_margin = egui::Margin::same(16.0);

        ctx.set_style(style);
    }

    fn dark_visuals(&self) -> Visuals {
        let mut visuals = Visuals::dark();

        // Background colors
        visuals.panel_fill = SignalColors::DARK_BG;
        visuals.window_fill = SignalColors::DARK_SURFACE;
        visuals.extreme_bg_color = SignalColors::DARK_BG;
        visuals.faint_bg_color = SignalColors::DARK_SURFACE;

        // Widget colors
        visuals.widgets.noninteractive.bg_fill = SignalColors::DARK_SURFACE;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, SignalColors::TEXT_PRIMARY);
        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

        visuals.widgets.inactive.bg_fill = SignalColors::DARK_SURFACE_ELEVATED;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, SignalColors::TEXT_PRIMARY);
        visuals.widgets.inactive.rounding = Rounding::same(8.0);

        visuals.widgets.hovered.bg_fill = SignalColors::SIGNAL_BLUE_HOVER;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, SignalColors::TEXT_PRIMARY);
        visuals.widgets.hovered.rounding = Rounding::same(8.0);

        visuals.widgets.active.bg_fill = SignalColors::SIGNAL_BLUE_PRESSED;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, SignalColors::TEXT_PRIMARY);
        visuals.widgets.active.rounding = Rounding::same(8.0);

        // Selection
        visuals.selection.bg_fill = SignalColors::SIGNAL_BLUE.linear_multiply(0.5);
        visuals.selection.stroke = Stroke::new(1.0, SignalColors::SIGNAL_BLUE);

        // Hyperlinks
        visuals.hyperlink_color = SignalColors::SIGNAL_BLUE;

        // Window
        visuals.window_rounding = Rounding::same(12.0);
        visuals.window_shadow.blur = 16.0;

        visuals
    }

    fn light_visuals(&self) -> Visuals {
        let mut visuals = Visuals::light();

        // Background colors
        visuals.panel_fill = SignalColors::LIGHT_BG;
        visuals.window_fill = SignalColors::LIGHT_SURFACE;
        visuals.extreme_bg_color = SignalColors::LIGHT_BG;
        visuals.faint_bg_color = SignalColors::LIGHT_SURFACE;

        // Widget colors
        visuals.widgets.noninteractive.bg_fill = SignalColors::LIGHT_SURFACE;
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, SignalColors::TEXT_DARK);
        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

        visuals.widgets.inactive.bg_fill = SignalColors::LIGHT_SURFACE_ELEVATED;
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, SignalColors::TEXT_DARK);
        visuals.widgets.inactive.rounding = Rounding::same(8.0);

        visuals.widgets.hovered.bg_fill = SignalColors::SIGNAL_BLUE_HOVER;
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, SignalColors::TEXT_PRIMARY);
        visuals.widgets.hovered.rounding = Rounding::same(8.0);

        visuals.widgets.active.bg_fill = SignalColors::SIGNAL_BLUE_PRESSED;
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, SignalColors::TEXT_PRIMARY);
        visuals.widgets.active.rounding = Rounding::same(8.0);

        // Selection
        visuals.selection.bg_fill = SignalColors::SIGNAL_BLUE.linear_multiply(0.3);
        visuals.selection.stroke = Stroke::new(1.0, SignalColors::SIGNAL_BLUE);

        // Hyperlinks
        visuals.hyperlink_color = SignalColors::SIGNAL_BLUE;

        // Window
        visuals.window_rounding = Rounding::same(12.0);
        visuals.window_shadow.blur = 8.0;

        visuals
    }
}
