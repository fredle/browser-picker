//! Light theme: neutral surfaces, hairline borders, one blue accent.

use eframe::egui::Color32;

const fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, ((c >> 8) & 0xff) as u8, (c & 0xff) as u8)
}

/// Window ground - a hair off-white so white cards read as raised.
pub const BG: Color32 = hex(0xf4f6f8);
/// Cards, rows, popups.
pub const BG_CARD: Color32 = hex(0xffffff);
pub const BG_HOVER: Color32 = hex(0xeaeef4);
/// Pressed / selected.
pub const BG_ACTIVE: Color32 = hex(0xdfe5ee);

pub const FG: Color32 = hex(0x14171c);
pub const FG_DIM: Color32 = hex(0x6a7480);
pub const BORDER: Color32 = hex(0xdfe3e9);
/// Slightly stronger border for hovered/focused surfaces.
pub const BORDER_STRONG: Color32 = hex(0xc3cad4);

pub const ACCENT: Color32 = hex(0x2563eb);

pub const OK: Color32 = hex(0x15803d);
pub const WARN: Color32 = hex(0xb45309);
pub const BAD: Color32 = hex(0xb91c1c);

/// Dark slate behind the white padlock on private-mode avatars, so it stays
/// legible against light surfaces.
pub const LOCK_BG: Color32 = hex(0x475569);

/// Saturated enough to carry white initials on a light card.
pub const AVATAR_COLORS: [Color32; 8] = [
    hex(0xdc2626),
    hex(0xea580c),
    hex(0xca8a04),
    hex(0x16a34a),
    hex(0x0d9488),
    hex(0x2563eb),
    hex(0x7c3aed),
    hex(0x0891b2),
];

pub const RADIUS_CARD: u8 = 8;
pub const RADIUS_SMALL: u8 = 6;

pub fn accent(browser: &str) -> Color32 {
    match browser {
        "chrome" => hex(0x1a73e8),
        "edge" => hex(0x0f6cbd),
        _ => hex(0x94a3b8),
    }
}

pub fn browser_label(browser: &str) -> &'static str {
    match browser {
        "chrome" => "Chrome",
        "edge" => "Edge",
        _ => "",
    }
}
