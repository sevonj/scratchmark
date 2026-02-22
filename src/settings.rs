use gtk::gio::Settings;
use gtk::prelude::*;

/// Same as in gtk fontchooser
pub const EDITOR_FONT_SIZES: [u32; 14] = [6, 8, 9, 10, 11, 12, 13, 14, 16, 20, 24, 36, 48, 72];
pub const EDITOR_WIDTH_LIMIT_MAX: u32 = 6000;
pub const EDITOR_WIDTH_LIMIT_MIN: u32 = 500;

/// Resets crazy values that could break the app
pub fn sanity_filter(settings: &Settings) {
    let max_font_size = *EDITOR_FONT_SIZES.last().unwrap();
    let min_font_size = EDITOR_FONT_SIZES[0];
    if settings.uint("editor-font-size") > max_font_size
        || settings.uint("editor-font-size") < min_font_size
    {
        settings.reset("editor-font-size");
    }

    if settings.uint("editor-max-width") > EDITOR_WIDTH_LIMIT_MAX
        || settings.uint("editor-max-width") < EDITOR_WIDTH_LIMIT_MIN
    {
        settings.reset("editor-max-width");
    }
}
