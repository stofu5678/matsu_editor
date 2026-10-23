//! Changing the fonts of the settings file at run time (`F_FONT`, and `F_SETFONTSIZE` in the modes
//! that change the settings), and bringing the windows up to date.
//!
//! Sakura keeps the settings in memory shared by all the windows and writes the file when the last
//! window closes; the windows get `MYWM_CHANGESETTING` with `PM_CHANGESETTING_FONT` or
//! `PM_CHANGESETTING_FONTSIZE` (`sakura_core/cmd/CViewCommander_Settings.cpp`). MATSU Editor writes
//! the file at once, under the lock every writer takes, so that the rest of the file and what other
//! windows wrote stay, and then every window reads the settings again.

use gpui::App;
use matsu_config::settings::{FontSetting, ViewFont};
use matsu_config::{Profile, ProfileError};

use crate::settings::UserSettings;
use crate::settings::view_settings::DPI;

/// The common font (`[Font]`) as the settings now have it.
pub(crate) fn common_font(cx: &App) -> ViewFont {
    cx.try_global::<UserSettings>()
        .and_then(UserSettings::profile)
        .map(ViewFont::read)
        .unwrap_or_default()
}

/// The font of document type `index` when it has one of its own, else the common font: the font
/// a view of that type shows (`CEditWnd::GetLogfont( false )`).
pub(crate) fn font_of_type(cx: &App, index: usize) -> FontSetting {
    let own = cx
        .try_global::<UserSettings>()
        .and_then(|user| user.document_types().get(index))
        .filter(|t| t.use_type_font)
        .map(|t| t.font.clone());
    own.unwrap_or_else(|| common_font(cx).font)
}

/// `font` with the size `tenths` (1/10 point) and the height of that size at 96 dpi
/// (`DpiPointsToPixels(-nPointSize, 10)`).
fn resized(font: &FontSetting, tenths: i32) -> FontSetting {
    let mut font = FontSetting {
        point_size: tenths,
        ..font.clone()
    };
    font.log_font.height = font.height_for_dpi(DPI);
    font
}

/// Writes a change to the settings file and makes the result the settings of the application. A
/// file that cannot be written (no place for it, no right to write) leaves the change for this
/// session, as the settings dialogs do.
fn update_settings(cx: &mut App, change: impl Fn(&mut Profile) -> Result<bool, ProfileError>) {
    let mut written: Option<Profile> = None;
    crate::history::write_settings(cx, |profile| {
        let changed = change(profile)?;
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
        let _ = change(&mut profile);
        profile
    });
    let path = user.as_ref().and_then(crate::settings::history_file);
    cx.set_global(UserSettings::from_profile(profile, path));
}

/// Every view of every window reads the common font again (`PM_CHANGESETTING_FONT`).
fn refresh_views(cx: &mut App) {
    let Some(user) = cx.try_global::<UserSettings>().cloned() else {
        return;
    };
    for (_, workspace) in crate::file_ops::windows::workspaces(cx) {
        for view in workspace.read(cx).all_views() {
            view.update(cx, |view, cx| view.refresh_font(&user, cx));
        }
    }
}

/// `Command_FONT`: the new common font. `[Font]` gets the `LOGFONT`, the size and
/// `bFontIs_FIXED_PITCH`, which Sakura sets to true whatever the font is (2010.09.06: "プロポーショナル
/// フォントでもたぶん矩形も使えるはず"). The windows whose type has a font of its own keep it.
pub(crate) fn set_common_font(cx: &mut App, font: &FontSetting) {
    let new = ViewFont {
        font: font.clone(),
        fixed_pitch: true,
    };
    update_settings(cx, |profile| {
        new.write(profile)?;
        Ok(true)
    });
    refresh_views(cx);
}

/// `Command_SETFONTSIZE` in mode 0: the size of the common font, `tenths` in 1/10 point; the face
/// stays. A file without a face name (the program's own font is shown) gets the size only, so that
/// the font does not change to Sakura's default face.
pub(crate) fn set_common_size(cx: &mut App, tenths: i32) {
    let current = common_font(cx);
    let new = ViewFont {
        font: resized(&current.font, tenths),
        ..current
    };
    update_settings(cx, |profile| {
        if profile
            .get(ViewFont::SECTION, "lfFaceName")
            .is_some_and(|face| !face.is_empty())
        {
            new.write(profile)?;
        } else {
            profile.set_i32(ViewFont::SECTION, "nPointSize", new.font.point_size)?;
        }
        Ok(true)
    });
    refresh_views(cx);
}

/// `Command_SETFONTSIZE` in mode 1: the size of the font of document type `index`, which gets its
/// own font if it has none (`m_bUseTypeFont = true`, the face of the font the view showed).
pub(crate) fn set_type_size(cx: &mut App, index: usize, tenths: i32) {
    let mut base = font_of_type(cx, index);
    // A view with no font in the file shows the program's own font: the type keeps that face, not
    // Sakura's default one.
    let has_own = cx
        .try_global::<UserSettings>()
        .and_then(|user| user.document_types().get(index))
        .is_some_and(|t| t.use_type_font);
    let has_face = cx
        .try_global::<UserSettings>()
        .and_then(UserSettings::profile)
        .and_then(|p| p.get(ViewFont::SECTION, "lfFaceName"))
        .is_some_and(|face| !face.is_empty());
    if !has_own && !has_face {
        base.log_font.face_name = crate::view::FONT_FAMILY.to_owned();
    }
    let font = resized(&base, tenths);
    let Ok(number) = i32::try_from(index) else {
        return;
    };
    update_settings(cx, |profile| {
        let types = matsu_config::settings::DocumentTypes::from_profile(profile);
        let Some(mut t) = types.get(index).cloned() else {
            return Ok(false);
        };
        t.use_type_font = true;
        t.font = font.clone();
        t.write(profile, number)?;
        Ok(true)
    });
    // Only the font is read again (`OnChangeSetting( true, false )`): a temporary wrap stays, and a
    // temporary size ends because the size of the font of the file changed.
    refresh_views(cx);
}
