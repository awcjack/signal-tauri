//! Signal-inspired theme for egui

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, Rounding, Stroke, Style, TextStyle, Visuals};
use std::sync::Once;

static FONTS_CONFIGURED: Once = Once::new();

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
    pub const BUBBLE_RECEIVED: Color32 = Color32::from_rgb(0x37, 0x3E, 0x47); // Lighter gray-blue for better contrast

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
        FONTS_CONFIGURED.call_once(|| {
            configure_system_fonts(ctx);
        });

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

/// Try to load a font from a list of candidate paths. Returns true on success.
fn try_load_font(fonts: &mut FontDefinitions, key: &str, paths: &[&str], tweak: Option<egui::FontTweak>) -> bool {
    for font_path in paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            let data = if let Some(tw) = tweak {
                FontData::from_owned(font_data).tweak(tw)
            } else {
                FontData::from_owned(font_data)
            };
            fonts.font_data.insert(key.to_owned(), data);
            tracing::info!("Loaded system font '{}' from {}", key, font_path);
            return true;
        }
    }
    false
}

fn configure_system_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // --- System emoji font ---
    // Note: egui's font renderer (ab_glyph) does not support color bitmap emoji
    // (SBIX/CBDT format). Loading the system emoji font provides monochrome
    // emoji glyph outlines as a fallback. Full-color emoji in the picker and
    // chat messages are rendered via twemoji SVG images.
    let emoji_font_paths: &[&str] = &[
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/Apple Color Emoji.ttc",
        #[cfg(target_os = "windows")]
        "C:\\Windows\\Fonts\\seguiemj.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/google-noto-color-emoji/NotoColorEmoji.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/noto-color-emoji/NotoColorEmoji.ttf",
    ];
    let emoji_loaded = try_load_font(&mut fonts, "emoji", emoji_font_paths, None);

    // --- System UI font ---
    let ui_font_paths: &[&str] = &[
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/SFNS.ttf",
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/Helvetica.ttc",
        #[cfg(target_os = "windows")]
        "C:\\Windows\\Fonts\\segoeui.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];
    let ui_font_loaded = try_load_font(&mut fonts, "system_ui", ui_font_paths, None);

    // --- CJK font ---
    let cjk_font_paths: &[&str] = &[
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/STHeiti Light.ttc",
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/Supplemental/Songti.ttc",
        #[cfg(target_os = "windows")]
        "C:\\Windows\\Fonts\\msyh.ttc",
        #[cfg(target_os = "windows")]
        "C:\\Windows\\Fonts\\simsun.ttc",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        #[cfg(target_os = "linux")]
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    ];
    let cjk_loaded = try_load_font(
        &mut fonts,
        "cjk_fallback",
        cjk_font_paths,
        Some(egui::FontTweak {
            scale: 1.0,
            y_offset_factor: 0.0,
            y_offset: 0.0,
            baseline_offset_factor: 0.15,
        }),
    );

    // --- Symbol font ---
    let symbol_font_paths: &[&str] = &[
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/Supplemental/Apple Symbols.ttf",
        #[cfg(target_os = "macos")]
        "/System/Library/Fonts/Symbol.ttf",
        #[cfg(target_os = "windows")]
        "C:\\Windows\\Fonts\\segmdl2.ttf",
        #[cfg(target_os = "windows")]
        "C:\\Windows\\Fonts\\symbol.ttf",
    ];
    try_load_font(&mut fonts, "symbols", symbol_font_paths, None);

    // --- Register fallback chain ---
    // Order: system UI → emoji → CJK → symbols (first match wins per glyph)
    for family_key in [FontFamily::Proportional, FontFamily::Monospace] {
        if let Some(family) = fonts.families.get_mut(&family_key) {
            if ui_font_loaded {
                family.push("system_ui".to_owned());
            }
            if emoji_loaded {
                family.push("emoji".to_owned());
            }
            if cjk_loaded {
                family.push("cjk_fallback".to_owned());
            }
            if fonts.font_data.contains_key("symbols") {
                family.push("symbols".to_owned());
            }
        }
    }

    ctx.set_fonts(fonts);
}
