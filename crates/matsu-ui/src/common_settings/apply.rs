//! OK: writes the settings file and brings the windows up to date.

use gpui::{AnyWindowHandle, App};
use matsu_config::Profile;

use super::model::{Control, Values};
use super::{Env, Page, pages};
use crate::settings::UserSettings;

/// Writes the values of the live controls to `[Common]` under the lock of the settings file (so
/// that the rest of the file, and what another window wrote meanwhile, stay), makes the settings
/// that the application reads from the file (the start folder of the file dialogs, the history)
/// the new ones, and lets every page apply its values to the windows.
///
/// A file that cannot be written (no place for it, no right to write) leaves the settings for this
/// session, as the tab keys and the history do.
pub(super) fn commit(
    cx: &mut App,
    controls: &[Control],
    before: &Values,
    after: &Values,
    opener: Option<AnyWindowHandle>,
) {
    let mut written: Option<Profile> = None;
    crate::history::write_settings(cx, |profile| {
        let changed = after.store(controls, profile)?;
        written = Some(profile.clone());
        Ok(changed)
    });
    let user = cx.try_global::<UserSettings>().cloned();
    let profile = written.unwrap_or_else(|| {
        let mut profile = user
            .as_ref()
            .and_then(|user| user.profile().cloned())
            .unwrap_or_default();
        // The values are valid, so this only fails for a key or value the file cannot hold.
        let _ = after.store(controls, &mut profile);
        profile
    });
    let path = user.as_ref().and_then(crate::settings::history_file);
    cx.set_global(UserSettings::from_profile(profile, path));

    let mut env = Env { cx, opener };
    for page in Page::ALL {
        if page.items().is_some() {
            page.apply(&mut env, after, Some(before));
        }
    }
    // What the user enabled may have changed the commands of the plugins, which the keys name.
    if crate::plugin_commands::refresh(env.cx) {
        pages::keys::rebuild_key_map(&mut env);
    }
    // The menus read the settings (the history, the custom menus): make them again from the new ones.
    crate::app::refresh_menus(env.cx);
}
