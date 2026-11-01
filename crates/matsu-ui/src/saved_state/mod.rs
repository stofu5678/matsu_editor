//! States that Sakura keeps in the shared data and writes to `sakura.ini`, and that the settings
//! file keeps here under the same keys (`ShareData_IO_Common` in `sakura_core/env/CShareData_IO.cpp`):
//!
//! * [`search`]: the options of the find and replace dialogs (`bSearch*`);
//! * [`window`]: the position and the size of the window (`bSaveWindowSize`, `nWinSize*`,
//!   `nSaveWindowPos`, `nWinPos*`).
//!
//! The side, the size and the sort of the outline pane (`eOutlineDockSide` and the others) are kept
//! the same way by [`crate::outline::dock`].
//!
//! Sakura keeps them in shared memory and writes the file when the last window closes. Here the
//! value is a global of the process, and the file is written when the value changes, under the lock
//! of the settings file ([`crate::history::write_settings`]), so that a crash loses nothing.

pub(crate) mod search;
pub(crate) mod window;

use gpui::App;

use crate::settings::UserSettings;

/// Takes the states of the settings file when the application starts.
pub(crate) fn install(cx: &mut App, user: &UserSettings) {
    let empty = matsu_config::Profile::new();
    let profile = user.profile().unwrap_or(&empty);
    cx.set_global(search::SavedSearch::from_profile(profile));
    cx.set_global(window::WindowState::from_profile(profile));
    cx.set_global(crate::outline::dock::OutlineDock::from_profile(profile));
}
