//! タイプ別設定 (`F_OPTION_TYPE`): Sakura's property sheet of one document type.
//!
//! Sakura: `CEditApp::OpenPropertySheetTypes` (`sakura_core/_main/CEditApp.cpp`) shows
//! `CPropTypes::DoPropertySheet` (`sakura_core/typeprop/CPropTypes.cpp`) for the type of the
//! document (`Command_OPTION_TYPE`, `sakura_core/cmd/CViewCommander_Settings.cpp`) or for the type
//! chosen in the list `F_TYPE_LIST` (設定変更, `CDlgTypeList`). The sheet has six pages, in the
//! order of [`PAGE_TITLES`]. OK writes the type (`CDocTypeManager::SetTypeConfig`) and every
//! window takes it at once (`MYWM_CHANGESETTING` with `PM_CHANGESETTING_TYPE`); キャンセル changes
//! nothing.
//!
//! The window is the one of the common settings (`crate::common_settings`): the pages here list
//! their controls (`pages/`), [`values`] turns a [`TypeSettings`] into the values of the controls
//! and back, and this file opens the sheet and does what OK does.
//!
//! # Pages
//!
//! All six pages are here. A control whose feature MATSU Editor does not have is shown greyed out
//! with the value of the file and is never written (the rule of the common settings). The lists of
//! 正規表現キーワード (`RxKey[NNN]`) and キーワードヘルプ (`KDct[NN]`) are the editable list of the
//! settings (`crate::common_settings::table`); their rows are kept in the type
//! ([`TypeSettings::regex_keywords`], [`TypeSettings::key_help`]).
//!
//! # The file
//!
//! The keys are Sakura's (`CShareData_IO::ShareData_IO_Type_One`) in `[Types(N)]`, written by
//! `TypeSettings::write` under the lock every writer of the file takes
//! (`matsu_config::history::update_profile`), so the other sections and the other windows' changes
//! stay. A type of MATSU Editor's own (ADR-0017) in a number the file does not use yet is written
//! with all its keys, so that the next read gives the same type.

mod ids;
mod pages;
mod values;

#[cfg(test)]
mod tests;

use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, Context, Entity, Global, WeakEntity,
    WindowBounds, WindowOptions, px, size,
};
use gpui_component::Root;
use matsu_config::Profile;
use matsu_config::settings::builtin_types::{BUILTIN_TYPE_COUNT, added_type_name};
use matsu_config::settings::type_rules::MAX_REGEX_KEYWORDS;
use matsu_config::settings::{DocumentTypes, TypeSettings};

use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Control, Values};
use crate::common_settings::{SheetPage, model};
use crate::settings::UserSettings;

/// The names of the pages, in Sakura's order (`TypePropSheetInfoList`, `STR_PROPTYPE_*`).
pub(crate) const PAGE_TITLES: [&str; 6] = [
    "スクリーン",
    "カラー",
    "ウィンドウ",
    "支援",
    "正規表現キーワード",
    "キーワードヘルプ",
];

/// What OK needs to know about the type the sheet was opened for.
pub(crate) struct TypeTarget {
    /// The number `N` of `Types(N)`.
    pub index: usize,
    /// The type as the sheet started from it (the values for a new document are the effective
    /// ones, [`values::shown_type`]).
    pub original: Box<TypeSettings>,
    /// The number is one the file does not use for a type of its own, so the type is one of MATSU
    /// Editor's (ADR-0017): it is written with all its keys.
    pub unused_slot: bool,
}

/// What [`CommonSettingsDialog`] needs to show the sheet.
pub(crate) struct TypeSheet {
    pub pages: Vec<SheetPage>,
    pub controls: Vec<Control>,
    pub values: Values,
    pub target: TypeTarget,
}

/// The page used last (`CPropTypes::m_nPageNum`): the next sheet opens at it.
#[derive(Clone, Copy)]
struct LastTypePage(usize);

impl Global for LastTypePage {}

/// Remembers the page for the next sheet.
pub(crate) fn remember_page(cx: &mut App, page: usize) {
    cx.set_global(LastTypePage(page));
}

fn last_page(cx: &App) -> usize {
    cx.try_global::<LastTypePage>().map_or(0, |p| p.0)
}

/// The sheet that is open: there is one at a time.
struct OpenSheet {
    window: AnyWindowHandle,
    dialog: WeakEntity<CommonSettingsDialog>,
    /// The type the sheet is for.
    index: usize,
}

impl Global for OpenSheet {}

/// The sheet that is open.
pub(crate) fn current_dialog(cx: &App) -> Option<Entity<CommonSettingsDialog>> {
    let open = cx.try_global::<OpenSheet>()?;
    if !cx.windows().contains(&open.window) {
        return None;
    }
    open.dialog.upgrade()
}

/// The window of the sheet that is open.
#[cfg(test)]
pub(crate) fn dialog_window(cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<OpenSheet>().map(|open| open.window)
}

/// The settings file as it is now (another window may have written it), or what the application
/// read when there is no file on disk.
fn current_profile(cx: &App) -> Profile {
    crate::history::settings_path(cx)
        .and_then(|path| Profile::load(&path).ok())
        .or_else(|| {
            cx.try_global::<UserSettings>()
                .and_then(|user| user.profile().cloned())
        })
        .unwrap_or_default()
}

/// Is `Types(index)` a number the file does not use for a type of its own (no section, or what
/// Sakura writes for a number it does not use: the name `設定N` and no extensions)? The same rule
/// as `DocumentTypes::from_profile`: the types of MATSU Editor's own (ADR-0017) go there.
fn is_unused_slot(profile: &Profile, index: usize) -> bool {
    let Ok(number) = i32::try_from(index) else {
        return false;
    };
    if index < BUILTIN_TYPE_COUNT {
        return false;
    }
    let section = TypeSettings::section_name(number);
    if profile.entries(&section).next().is_none() {
        return true;
    }
    let no_exts = profile
        .get(&section, "szTypeExts")
        .is_none_or(|v| v.trim().is_empty());
    let placeholder = profile
        .get(&section, "szTypeName")
        .is_none_or(|v| v == added_type_name(number));
    no_exts && placeholder
}

/// The sheet for type `index` read from `profile`; `None` when there is no such type.
pub(crate) fn sheet(profile: &Profile, index: usize) -> Option<TypeSheet> {
    let types = DocumentTypes::from_profile(profile);
    let original = values::shown_type(&types, profile, index)?;
    let pages = pages::pages(index);
    let controls: Vec<Control> = pages
        .iter()
        .filter_map(|page| page.items.as_deref())
        .flat_map(model::controls)
        .collect();
    // The name of the first keyword set, which the page shows (the choice is not available yet).
    let keyword1 = types
        .keywords()
        .get(original.keyword_set_idx[0])
        .map(|set| set.name.clone())
        .unwrap_or_default();
    let values = values::load(&controls, &original, profile, index, &keyword1);
    Some(TypeSheet {
        pages,
        controls,
        values,
        target: TypeTarget {
            index,
            unused_slot: is_unused_slot(profile, index),
            original: Box::new(original),
        },
    })
}

/// Shows the sheet for type `index` at `page` (`None`: the page used last). A sheet that is open
/// comes to the front instead of a second one. Returns `None` when there is no such type.
/// `opener` is the window of the command.
pub(crate) fn open(
    cx: &mut App,
    index: usize,
    page: Option<usize>,
    opener: Option<AnyWindowHandle>,
) -> Option<Entity<CommonSettingsDialog>> {
    if let Some(dialog) = current_dialog(cx) {
        let (window, open_index) = {
            let open = cx.global::<OpenSheet>();
            (open.window, open.index)
        };
        if open_index != index {
            // The sheet of another type is open: it is closed (nothing is written) and the one
            // that was asked for takes its place.
            window
                .update(cx, |_, window, _| window.remove_window())
                .ok();
        } else {
            if let Some(page) = page {
                dialog.update(cx, |dialog, cx| dialog.select_index(page, cx));
            }
            window
                .update(cx, |_, window, _| window.activate_window())
                .ok();
            return Some(dialog);
        }
    }
    let page = page.unwrap_or_else(|| last_page(cx));
    let profile = current_profile(cx);
    let sheet = sheet(&profile, index)?;
    let title = "タイプ別設定";
    let bounds = Bounds::centered(None, size(px(800.), px(620.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        icon: crate::app_icon::window_icon(),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some(title.into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<CommonSettingsDialog>>>> =
        Default::default();
    let made = slot.clone();
    let mut sheet = Some(sheet);
    let opened = cx.open_window(options, move |window, cx| {
        let sheet = sheet.take().expect("the window is made once");
        let dialog = cx.new(|cx: &mut Context<CommonSettingsDialog>| {
            CommonSettingsDialog::for_type(page, sheet, opener, window, cx)
        });
        *made.borrow_mut() = Some(dialog.clone());
        cx.new(|cx| Root::new(dialog, window, cx))
    });
    let handle = opened.ok()?;
    let dialog = slot.borrow_mut().take()?;
    cx.set_global(OpenSheet {
        window: handle.into(),
        dialog: dialog.downgrade(),
        index,
    });
    remember_page(cx, page);
    Some(dialog)
}

/// What the command `F_OPTION_TYPE` does: opens the sheet for the type of the document of the
/// window, once the command has ended (the sheet looks at the window's view).
pub(crate) fn run_command(
    view: &crate::view::EditorView,
    window: &mut gpui::Window,
    cx: &mut Context<crate::view::EditorView>,
) -> matsu_commands::command::Result {
    run_command_at(view, window, cx, None)
}

/// The pages of the sheet by their position in [`PAGE_TITLES`], for the commands that open the sheet
/// at one (`F_TYPE_SCREEN` ...).
pub(crate) mod page {
    pub(crate) const SCREEN: usize = 0;
    pub(crate) const COLOR: usize = 1;
    pub(crate) const WINDOW: usize = 2;
    pub(crate) const HELPER: usize = 3;
    pub(crate) const REGEX_KEYWORD: usize = 4;
    pub(crate) const KEYHELP: usize = 5;
}

/// [`run_command`] at `page` (`None`: the page used last).
pub(crate) fn run_command_at(
    view: &crate::view::EditorView,
    window: &mut gpui::Window,
    cx: &mut Context<crate::view::EditorView>,
    page: Option<usize>,
) -> matsu_commands::command::Result {
    let opener = window.window_handle();
    let index = view.type_index();
    view.defer_in_window(window, cx, move |_, app| {
        open(app, index, page, Some(opener));
    })
}

/// OK: writes the type and brings the windows up to date.
///
/// Sakura (`CEditApp::OpenPropertySheetTypes`): the type is stored (`SetTypeConfig`) and every
/// editor window gets `MYWM_CHANGESETTING`. Nothing is written when nothing changed. A file that
/// cannot be written (no place for it, no right to write) leaves the settings for this session,
/// as the common settings do.
pub(crate) fn commit(cx: &mut App, target: &TypeTarget, values: &Values) {
    let new = values::store(values, &target.original);
    if new == *target.original {
        return;
    }
    let number = i32::try_from(target.index).unwrap_or(0);
    let mut written: Option<Profile> = None;
    crate::history::write_settings(cx, |profile| {
        write_type(profile, &new, number, target)?;
        written = Some(profile.clone());
        Ok(true)
    });
    let user = cx.try_global::<UserSettings>().cloned();
    let profile = written.unwrap_or_else(|| {
        let mut profile = user
            .as_ref()
            .and_then(|user| user.profile().cloned())
            .unwrap_or_default();
        // The values are valid, so this only fails for a key or value the file cannot hold.
        let _ = write_type(&mut profile, &new, number, target);
        profile
    });
    let path = user.as_ref().and_then(crate::settings::history_file);
    cx.set_global(UserSettings::from_profile(profile, path));
    crate::doc_types::refresh_open_views(cx, target.index);
}

/// Writes what the user changed into type `number` of `profile`, over the type as `profile` has it
/// now (another window or process may have changed it since the sheet opened), except that the
/// new-file keys (`eDefaultCodetype`, `eDefaultEoltype`, `bDefaultBom`) the user did not change
/// and the file does not set stay unset, so that the line ending still follows the OS. A type of
/// MATSU Editor's own (ADR-0017) is written with all its keys.
fn write_type(
    profile: &mut Profile,
    new: &TypeSettings,
    number: i32,
    target: &TypeTarget,
) -> Result<(), matsu_config::ProfileError> {
    let types = DocumentTypes::from_profile(profile);
    let base = values::shown_type(&types, profile, target.index)
        .unwrap_or_else(|| (*target.original).clone());
    let merged = values::merge(&base, &target.original, new);
    let section = TypeSettings::section_name(number);
    // The new-file keys as the file has them now, to put back the ones the user did not change:
    // a key that is not there, or says "like the basic type" (`eDefaultEoltype=0`) or an encoding
    // MATSU Editor does not have, stays as it is.
    let raw = |key: &str| profile.get(&section, key).map(str::to_owned);
    let kept = [
        (
            "eDefaultCodetype",
            merged.default_codetype == base.default_codetype,
            raw("eDefaultCodetype"),
        ),
        (
            "eDefaultEoltype",
            merged.default_eoltype == base.default_eoltype,
            raw("eDefaultEoltype"),
        ),
        (
            "bDefaultBom",
            // Another encoding has another default BOM, so the BOM is the user's choice then.
            merged.default_bom == base.default_bom
                && merged.default_codetype == base.default_codetype,
            raw("bDefaultBom"),
        ),
    ];
    merged.write(profile, number)?;
    // A built-in type starts with keywords that the file does not have. When the user took some
    // away, a blank key ends the list, so that they do not come back when the file is read.
    let regex_kept = merged
        .regex_keywords
        .iter()
        .filter(|k| !k.pattern.is_empty())
        .count();
    if regex_kept < base.regex_keywords.len() && regex_kept < MAX_REGEX_KEYWORDS {
        let key = format!("RxKey[{regex_kept:03}]");
        if profile.get(&section, &key).is_none() {
            profile.set_string(&section, &key, "")?;
        }
    }
    if target.unused_slot {
        return Ok(());
    }
    for (key, unchanged, before) in kept {
        if !unchanged {
            continue;
        }
        match before {
            Some(text) => profile.set(&section, key, &text)?,
            None => {
                profile.remove(&section, key);
            }
        }
    }
    Ok(())
}
