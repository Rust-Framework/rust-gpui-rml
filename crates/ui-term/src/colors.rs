use gpui::{Hsla, Rgba};
use gpui_component::Theme;
use crate::config::ColorPalette;

fn hsla_to_rgb_bytes(color: Hsla) -> (u8, u8, u8) {
    let rgba: Rgba = color.into();
    (
        (rgba.r * 255.0).round() as u8,
        (rgba.g * 255.0).round() as u8,
        (rgba.b * 255.0).round() as u8,
    )
}

/// Build a terminal palette from RML theme tokens.
/// Default cell background is transparent so the host surface shows through.
pub fn palette_from_theme(theme: &Theme) -> ColorPalette {
    let (fg_r, fg_g, fg_b) = hsla_to_rgb_bytes(theme.foreground);
    let (cursor_r, cursor_g, cursor_b) = hsla_to_rgb_bytes(theme.accent_foreground);

    ColorPalette::builder()
        .foreground(fg_r, fg_g, fg_b)
        .cursor(cursor_r, cursor_g, cursor_b)
        .build()
        .transparent_default_background()
}
