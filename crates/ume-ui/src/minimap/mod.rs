//! The minimap (Sakura's ミニマップ, `F_SHOWMINIMAP`): a narrow second view of the document in a
//! tiny font at the right of the text, with the rows the window shows marked.
//!
//! Sakura's `CMiniMapView` is a `CEditView` that draws the same layout in its own small font
//! (`sakura_core/view/CMiniMapView.cpp`, `CViewFont::CreateFonts`); the help is `HLP000371.html`.
//!
//! * [`MiniMapSettings`]: `bDispMiniMap`, `nMiniMapFontSize`, `nMiniMapQuality` and
//!   `nMiniMapWidth` of `[Common]`. `F_SHOWMINIMAP` flips the first and writes the file. Sakura's
//!   common settings dialog has no page for the other three (the help says they are in
//!   `sakura.ini` only), and neither has this program's.
//! * [`geometry`]: the arithmetic of the rows and the drag, without a window.
//! * [`panel`]: the pane that shows the minimap in a window, and the mouse.
//! * [`paint`]: the painting. Only the rows in view are colored and shaped.
//! * `crate::view::minimap`: what the pane takes from the active view of the window.

pub(crate) mod geometry;
mod paint;
pub(crate) mod panel;

#[cfg(test)]
mod tests;

use gpui::{App, Global};
use ume_config::Profile;

/// `bDispMiniMap`.
const KEY_SHOWN: &str = "bDispMiniMap";
/// `nMiniMapFontSize`.
const KEY_FONT_SIZE: &str = "nMiniMapFontSize";
/// `nMiniMapQuality`.
const KEY_QUALITY: &str = "nMiniMapQuality";
/// `nMiniMapWidth`.
const KEY_WIDTH: &str = "nMiniMapWidth";

/// What the common settings say about the minimap. The defaults are `CShareData::InitShareData`'s
/// (`sakura_core/env/CShareData.cpp`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MiniMapSettings {
    /// `m_bDispMiniMap`: the minimap is shown. Off by default.
    pub shown: bool,
    /// `m_nMiniMapFontSize`: the `lfHeight` of the font: negative is the height of a character,
    /// positive the height of a cell, 0 the default height of the system.
    pub font_size: i32,
    /// `m_nMiniMapQuality`: `lfQuality` of the font (`NONANTIALIASED_QUALITY`, 3, by default).
    /// GPUI draws every text the same way, so this is read and kept, and does not change the text.
    pub quality: i32,
    /// `m_nMiniMapWidth`: the width in pixels.
    pub width: i32,
}

impl Default for MiniMapSettings {
    fn default() -> Self {
        Self {
            shown: false,
            font_size: -2,
            quality: 3,
            width: 150,
        }
    }
}

impl Global for MiniMapSettings {}

impl MiniMapSettings {
    pub(crate) fn from_profile(profile: &Profile) -> Self {
        let default = Self::default();
        Self {
            shown: profile
                .get_bool("Common", KEY_SHOWN)
                .unwrap_or(default.shown),
            font_size: profile
                .get_i32("Common", KEY_FONT_SIZE)
                .unwrap_or(default.font_size),
            quality: profile
                .get_i32("Common", KEY_QUALITY)
                .unwrap_or(default.quality),
            width: profile
                .get_i32("Common", KEY_WIDTH)
                .unwrap_or(default.width),
        }
    }

    /// The width the pane gets: at least as wide as the scroll bar and a few columns, so that a
    /// value in the file that makes no sense cannot hide the pane's scroll bar.
    pub(crate) fn pane_width(&self) -> f32 {
        (self.width as f32).max(geometry::MIN_WIDTH)
    }
}

/// Starts the settings from the file that was read.
pub(crate) fn install(cx: &mut App, user: &crate::settings::UserSettings) {
    cx.set_global(
        user.profile()
            .map_or_else(MiniMapSettings::default, MiniMapSettings::from_profile),
    );
}

/// The settings now. Sakura's defaults before the application installs them.
pub(crate) fn current(cx: &App) -> MiniMapSettings {
    cx.try_global::<MiniMapSettings>()
        .copied()
        .unwrap_or_default()
}

/// Shows or hides the minimap in every window (`Command_SHOWMINIMAP` flips `m_bDispMiniMap` and
/// tells all the editors) and writes `bDispMiniMap` to `[Common]` of the settings file.
pub(crate) fn toggle(cx: &mut App) {
    let mut settings = current(cx);
    settings.shown = !settings.shown;
    cx.set_global(settings);
    crate::history::write_settings(cx, |profile| {
        let value = if settings.shown { "1" } else { "0" };
        if profile.get("Common", KEY_SHOWN) == Some(value) {
            return Ok(false);
        }
        profile.set("Common", KEY_SHOWN, value)?;
        Ok(true)
    });
}

/// The key of the settings now that the common settings dialog starts from (the file may be older
/// than the window: a window without a file keeps the state for the session).
pub(crate) fn put_into(profile: &mut Profile, settings: MiniMapSettings) {
    // A key that cannot be written to the copy is left as the file has it.
    let _ = profile.set("Common", KEY_SHOWN, if settings.shown { "1" } else { "0" });
}
