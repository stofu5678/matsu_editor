//! States that Sakura keeps in the shared data and writes to `sakura.ini`, and that the settings
//! file keeps here under the same keys (`ShareData_IO_Common`, `ShareData_IO_Other` in
//! `sakura_core/env/CShareData_IO.cpp`):
//!
//! * [`search`]: the options of the find and replace dialogs (`bSearch*`);
//! * [`grep`]: the choices of the Grep and Grep 置換 dialogs (`bGrep*`, `nGrepCharSet`);
//! * [`other`]: the choices of the DIFF, the tag file and the tag jump dialogs (`nDiffFlgOpt`,
//!   `nTagsOpt`, `szTagsCmdLine`, `m_bTagJumpICase`, `m_bTagJumpAnyWhere`);
//! * [`dialog_rect`]: the position and the size of the dialogs (`rc*Dialog`);
//! * [`window`]: the position and the size of the window (`bSaveWindowSize`, `nWinSize*`,
//!   `nSaveWindowPos`, `nWinPos*`).
//!
//! The side, the size and the sort of the outline pane (`eOutlineDockSide` and the others) are kept
//! the same way by [`crate::outline::dock`].
//!
//! Sakura keeps them in shared memory and writes the file when the last window closes. Here the
//! value is a global of the process, and the file is written when the value changes, under the lock
//! of the settings file ([`crate::history::write_settings`]), so that a crash loses nothing.

pub(crate) mod dialog_rect;
pub(crate) mod grep;
pub(crate) mod other;
pub(crate) mod search;
pub(crate) mod window;

use gpui::App;

use crate::settings::UserSettings;

/// Takes the states of the settings file when the application starts. Call it after the
/// settings of the common settings dialog (the tag jump modes) are installed.
pub(crate) fn install(cx: &mut App, user: &UserSettings) {
    let empty = matsu_config::Profile::new();
    let profile = user.profile().unwrap_or(&empty);
    cx.set_global(search::SavedSearch::from_profile(profile));
    cx.set_global(grep::SavedGrep::from_profile(profile));
    cx.set_global(dialog_rect::DialogRects::from_profile(profile));
    other::install(cx, profile);
    cx.set_global(window::WindowState::from_profile(profile));
    cx.set_global(crate::outline::dock::OutlineDock::from_profile(profile));
}
