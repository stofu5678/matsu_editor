//! 共通設定 (`F_OPTION`): Sakura's property sheet of the settings every window shares.
//!
//! Sakura: `CPropCommon::DoPropertySheet` (`sakura_core/prop/CPropCommon.cpp`) shows one page for
//! each group of settings, in the order of [`Page::ALL`]; `F_OPTION` opens the page used last
//! (`m_nPageNum`) and `F_OPTION_GENERAL`, `F_OPTION_WINDOW`, `F_OPTION_TAB`, `F_OPTION_EDIT`,
//! `F_OPTION_FILE`, `F_OPTION_BACKUP`, `F_OPTION_FORMAT`, `F_OPTION_GREP`, `F_OPTION_FNAME` and
//! `F_OPTION_STATUSBAR` open that page. OK writes the settings and every window takes them at once (`MYWM_CHANGESETTING`);
//! キャンセル changes nothing.
//!
//! * [`model`]: the description of a page (its controls, their keys, limits and defaults) and the
//!   values they hold. A page module lists its controls; the dialog draws them.
//! * `pages/`: one module for each page: `items()` lists the controls and `apply()` says what OK
//!   does to the windows. A page that is not here shows that it is not available yet; to add one,
//!   give it a module, and an arm in [`Page::items`] and [`Page::apply`].
//! * [`dialog`]: the window: the list of pages, the keys (Tab, Ctrl+Tab and Ctrl+PageDown between
//!   pages, Enter for OK, Esc for キャンセル, Alt with the access key of a label), OK and キャンセル.
//! * `apply`: writes the file and brings the windows up to date.
//!
//! # Controls that are not available
//!
//! One rule for the whole dialog: a control whose feature UME Editor does not have is shown greyed
//! out, with the value of the file, and the note 「（未対応）」. It is never written, so the file keeps
//! whatever it had (ADR-0010).
//!
//! # The file
//!
//! The keys are Sakura's (`CShareData_IO::ShareData_IO_Common`) in `[Common]`. OK writes through
//! `ume_config::history::update_profile`, the lock every writer of the file takes, so the other
//! sections and the other windows' changes stay.

mod apply;
mod dialog;
mod func_lookup;
pub(crate) mod model;
mod pages;
pub(crate) mod table;
pub(crate) mod transfer;
mod windows;

use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, Context, Entity, Global, WeakEntity,
    WindowBounds, WindowOptions, px, size,
};
use gpui_component::Root;
use ume_config::Profile;

pub(crate) use dialog::{CommonSettingsDialog, SheetPage};
pub(crate) use pages::window::splitter_sync;

use model::{Control, Item, Values};

/// What a page's `apply` needs: the application, and the window the dialog was opened from (the
/// windows gather in it when the tab mode changes).
pub(crate) struct Env<'a> {
    pub cx: &'a mut App,
    pub opener: Option<AnyWindowHandle>,
}

/// The pages of the property sheet, in Sakura's order (`ComPropSheetInfoList`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    General,
    Window,
    MainMenu,
    Toolbar,
    Tab,
    StatusBar,
    Edit,
    File,
    FileName,
    Backup,
    Format,
    Search,
    Keys,
    CustomMenu,
    Keyword,
    Support,
    Macro,
    Plugin,
}

impl Page {
    /// Every page, in the order of the sheet.
    pub(crate) const ALL: [Page; 18] = [
        Page::General,
        Page::Window,
        Page::MainMenu,
        Page::Toolbar,
        Page::Tab,
        Page::StatusBar,
        Page::Edit,
        Page::File,
        Page::FileName,
        Page::Backup,
        Page::Format,
        Page::Search,
        Page::Keys,
        Page::CustomMenu,
        Page::Keyword,
        Page::Support,
        Page::Macro,
        Page::Plugin,
    ];

    /// The name on the tab (`STR_PROPCOMMON_*`).
    pub(crate) fn title(self) -> &'static str {
        match self {
            Page::General => "全般",
            Page::Window => "ウィンドウ",
            Page::MainMenu => "メインメニュー",
            Page::Toolbar => "ツールバー",
            Page::Tab => "タブバー",
            Page::StatusBar => "ステータスバー",
            Page::Edit => "編集",
            Page::File => "ファイル",
            Page::FileName => "ファイル名表示",
            Page::Backup => "バックアップ",
            Page::Format => "書式",
            Page::Search => "検索",
            Page::Keys => "キー割り当て",
            Page::CustomMenu => "カスタムメニュー",
            Page::Keyword => "強調キーワード",
            Page::Support => "支援",
            Page::Macro => "マクロ",
            Page::Plugin => "プラグイン",
        }
    }

    pub(crate) fn index(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    /// The controls of the page, or `None` when the page is not available yet.
    pub(crate) fn items(self) -> Option<Vec<Item>> {
        match self {
            Page::General => Some(pages::general::items()),
            Page::Window => Some(pages::window::items()),
            Page::MainMenu => pages::main_menu::items(),
            Page::Toolbar => Some(pages::toolbar::items()),
            Page::Tab => Some(pages::tab::items()),
            Page::Edit => Some(pages::edit::items()),
            Page::StatusBar => Some(pages::status_bar::items()),
            Page::File => Some(pages::file::items()),
            Page::FileName => Some(pages::file_name::items()),
            Page::Backup => Some(pages::backup::items()),
            Page::Format => Some(pages::format::items()),
            Page::Search => Some(pages::search::items()),
            Page::Keys => pages::keys::items(),
            Page::CustomMenu => pages::custom_menu::items(),
            Page::Keyword => pages::keyword::items(),
            Page::Macro => pages::macros::items(),
            _ => None,
        }
    }

    /// What OK does to the windows. `before` is `None` when the application starts.
    fn apply(self, env: &mut Env, after: &Values, before: Option<&Values>) {
        match self {
            Page::General => pages::general::apply(env, after, before),
            Page::Window => pages::window::apply(env, after, before),
            Page::MainMenu => pages::main_menu::apply(env, after, before),
            Page::Toolbar => pages::toolbar::apply(env, after, before),
            Page::Tab => pages::tab::apply(env, after, before),
            Page::Edit => pages::edit::apply(env, after, before),
            Page::Backup => pages::backup::apply(env, after, before),
            Page::File => pages::file::apply(env, after, before),
            Page::FileName => pages::file_name::apply(env, after, before),
            Page::Format => pages::format::apply(env, after, before),
            Page::Search => pages::search::apply(env, after, before),
            Page::StatusBar => pages::status_bar::apply(env, after, before),
            Page::Keys => pages::keys::apply(env, after, before),
            Page::CustomMenu => pages::custom_menu::apply(env, after, before),
            Page::Keyword => pages::keyword::apply(env, after, before),
            Page::Macro => pages::macros::apply(env, after, before),
            // The other pages have nothing to tell the windows (yet).
            _ => {}
        }
    }
}

/// Every control of every page that is available.
pub(crate) fn all_controls() -> Vec<Control> {
    Page::ALL
        .iter()
        .filter_map(|page| page.items())
        .flat_map(|items| model::controls(&items))
        .collect()
}

/// Takes the settings of the file into the windows' globals when the application starts. Call it
/// after the tab settings and the history are installed.
pub(crate) fn install(cx: &mut App, user: &crate::settings::UserSettings) {
    // The pages look for the folder of the settings file (`%SAKURADATA%`) in the global.
    cx.set_global(user.clone());
    let profile = user.profile().cloned().unwrap_or_default();
    let values = Values::load(&all_controls(), &profile);
    let mut env = Env { cx, opener: None };
    for page in Page::ALL {
        if page.items().is_some() {
            page.apply(&mut env, &values, None);
        }
    }
    // The letter of the generations is in the file only (the settings are not a global yet).
    pages::backup::set_ext_char(env.cx, &profile);
}

/// The page used last (`CPropCommon::m_nPageNum`): `F_OPTION` opens it again.
#[derive(Clone, Copy)]
struct LastPage(Page);

impl Global for LastPage {}

/// The dialog that is open: there is one at a time.
struct OpenDialog {
    window: AnyWindowHandle,
    dialog: WeakEntity<CommonSettingsDialog>,
}

impl Global for OpenDialog {}

fn last_page(cx: &App) -> Page {
    cx.try_global::<LastPage>().map_or(Page::General, |p| p.0)
}

/// The dialog that is open.
pub(crate) fn current_dialog(cx: &App) -> Option<Entity<CommonSettingsDialog>> {
    let open = cx.try_global::<OpenDialog>()?;
    if !cx.windows().contains(&open.window) {
        return None;
    }
    open.dialog.upgrade()
}

/// The profile the dialog starts from: the settings file as it is now (another window may have
/// written it), with what the windows hold in their own state: the tab mode, the quote string, and
/// the status bar of the window the dialog was opened from.
fn start_profile(cx: &mut App, opener: Option<AnyWindowHandle>) -> Profile {
    let mut profile = crate::history::settings_path(cx)
        .and_then(|path| Profile::load(&path).ok())
        .or_else(|| {
            cx.try_global::<crate::settings::UserSettings>()
                .and_then(|user| user.profile().cloned())
        })
        .unwrap_or_default();
    let tabs = crate::tabs::settings::current(cx);
    crate::bars::put_into(&mut profile, crate::bars::current(cx));
    let mut put = |key: &str, on: bool| {
        // A key that cannot be written to the copy is left as the file has it.
        let _ = profile.set(model::SECTION, key, if on { "1" } else { "0" });
    };
    put("bDispTabWnd", tabs.show_bar);
    put("bDispTabWndMultiWin", !tabs.grouped);
    put("bNewWindow", tabs.new_window);
    let opener_view = opener
        .and_then(|handle| {
            crate::file_ops::windows::editors(cx)
                .into_iter()
                .find(|(h, _)| *h == handle)
        })
        .map(|(_, view)| view);
    if let Some(view) = opener_view {
        put("bDispSTATUSBAR", view.read(cx).status_bar_shown());
    }
    // `F_SET_QUOTESTRING` changes the quote of the session only (`m_szInyouKigou` in memory): the
    // dialog shows it, so that OK does not take it back.
    if let Some(quote) = cx.try_global::<crate::clipboard::QuoteString>() {
        let _ = profile.set(model::SECTION, "szInyouKigou", &quote.0);
    }
    profile
}

/// Shows the dialog at `page` (`None`: the page used last). A dialog that is open comes to the
/// front at that page instead of a second one. `opener` is the window of the command.
pub(crate) fn open(
    cx: &mut App,
    page: Option<Page>,
    opener: Option<AnyWindowHandle>,
) -> Option<Entity<CommonSettingsDialog>> {
    let page = page.unwrap_or_else(|| last_page(cx));
    if let Some(dialog) = current_dialog(cx) {
        let window = cx.global::<OpenDialog>().window;
        dialog.update(cx, |dialog, cx| dialog.select_page(page, cx));
        window
            .update(cx, |_, window, _| window.activate_window())
            .ok();
        return Some(dialog);
    }
    let profile = start_profile(cx, opener);
    let bounds = Bounds::centered(None, size(px(800.), px(620.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("共通設定".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let slot: std::rc::Rc<std::cell::RefCell<Option<Entity<CommonSettingsDialog>>>> =
        Default::default();
    let made = slot.clone();
    let opened = cx.open_window(options, move |window, cx| {
        let dialog = cx.new(|cx: &mut Context<CommonSettingsDialog>| {
            CommonSettingsDialog::new(page, opener, &profile, window, cx)
        });
        *made.borrow_mut() = Some(dialog.clone());
        cx.new(|cx| Root::new(dialog, window, cx))
    });
    let handle = opened.ok()?;
    let dialog = slot.borrow_mut().take()?;
    cx.set_global(OpenDialog {
        window: handle.into(),
        dialog: dialog.downgrade(),
    });
    cx.set_global(LastPage(page));
    Some(dialog)
}

/// The window of the dialog that is open.
#[cfg(test)]
pub(crate) fn dialog_window(cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<OpenDialog>().map(|open| open.window)
}

/// What the commands `F_OPTION` and `F_OPTION_*` do: open the dialog from the window of the
/// command, once the command has ended (the dialog looks at the window's view).
pub(crate) fn run_command(
    view: &crate::view::EditorView,
    window: &mut gpui::Window,
    cx: &mut Context<crate::view::EditorView>,
    page: Option<Page>,
) -> ume_commands::command::Result {
    let opener = window.window_handle();
    view.defer_in_window(window, cx, move |_, app| {
        open(app, page, Some(opener));
    })
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_more_pages;
#[cfg(test)]
mod tests_polish;
#[cfg(test)]
mod tests_table;
