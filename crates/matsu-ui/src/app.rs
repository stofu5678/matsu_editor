//! The application: the window with a menu bar, the editor and a status bar.
//!
//! The menu is Sakura's main menu (`crate::menu`).

use gpui::{
    App, AppContext as _, Context, Decorations, Entity, Focusable, InteractiveElement as _,
    IntoElement, Menu, MenuItem, MouseButton, ParentElement, Render, Styled, Window, actions, div,
    prelude::FluentBuilder, px, rgb,
};
use gpui_component::{GlobalState, TitleBar};
use gpui_platform::application;

use crate::custom_menu::{ChainedLists, model::CustomMenuList};
use crate::document::SharedDocument;
use crate::file_ops::start_options::LaunchOptions;
use crate::file_ops::windows::{open_from_outside, open_from_outside_with};
use crate::launch::OpenRequest;
use crate::menu::{Entry, MenuAction, Registered, label_with_shortcut, main_menu_for};
use crate::menu_bar::MenuBar;
use crate::panes::PaneSet;
use crate::tabs::content::MenuKind;
use crate::view::{EditorView, OpenRecentFile, OpenRecentFolder, RunFunction};
use matsu_commands::keymap::{DisplayStyle, KeyMap};

actions!(
    matsu,
    [
        Exit,
        /// A menu item that cannot run: no one handles this action.
        Unavailable
    ]
);

/// The key map the menus show the shortcuts of, kept so that the menus can be made again when the
/// history changes.
pub(crate) struct MenuKeyMap(pub(crate) KeyMap);

impl gpui::Global for MenuKeyMap {}

/// Runs `f` with the key map of the application (Sakura's default one before `run` installs the
/// user's). The function key bar reads it to label its buttons.
pub(crate) fn with_key_map<R>(cx: &App, f: impl FnOnce(&KeyMap) -> R) -> R {
    match cx.try_global::<MenuKeyMap>() {
        Some(map) => f(&map.0),
        None => f(&KeyMap::default()),
    }
}

/// Makes `map` the key map of the application (the menus and the function key bar read it): the
/// common settings do so at OK, the tests of the function key bar set one.
pub(crate) fn set_key_map(cx: &mut App, map: KeyMap) {
    cx.set_global(MenuKeyMap(map));
}

/// Counts how many times the menus were made. A window watches it to load the new menus.
pub(crate) struct MenuGeneration(u64);

impl gpui::Global for MenuGeneration {}

/// The state that the native menu was made with (macOS), to make the menus again only when it
/// changes.
struct MenuStates {
    states: Option<crate::menu_state::FuncStates>,
    kind: MenuKind,
}

impl gpui::Global for MenuStates {}

/// The menus of every kind of tab (ADR-0024), as the menu bars of the windows show them: a window
/// shows the menus of the kind of its active tab. The native menu of macOS shows those of the
/// active window's active tab, through `cx.set_menus`.
pub(crate) struct KindMenus(std::collections::HashMap<MenuKind, Vec<gpui::OwnedMenu>>);

impl gpui::Global for KindMenus {}

impl KindMenus {
    /// The menus of `kind`, once the application has made them.
    pub(crate) fn of(cx: &App, kind: MenuKind) -> Option<Vec<gpui::OwnedMenu>> {
        cx.try_global::<Self>()?.0.get(&kind).cloned()
    }
}

/// macOS: makes the native menu again if the marks of the active window are not the ones it shows.
/// Called when a window becomes active and after a function ran, because the native menu cannot
/// ask when it opens. Does nothing elsewhere (the window's menu bar asks).
///
/// `always` skips the comparison: the snapshot does not include what the disk says (whether a
/// file is there, a header of the same name), so a window that becomes active makes them again.
fn refresh_menu_states(cx: &mut App, always: bool) {
    let (kind, states) = active_menu_context(cx);
    if !always
        && cx
            .try_global::<MenuStates>()
            .is_some_and(|m| m.kind == kind && m.states == states)
    {
        return;
    }
    refresh_menus(cx);
}

/// Asks for [`refresh_menu_states`] once the update that is running ends (macOS only). It has to
/// be the application's `defer`, not a window's: a window's runs inside an update of that window,
/// and the state is read by updating the active window.
pub(crate) fn defer_refresh_menu_states(cx: &mut App, always: bool) {
    if cfg!(target_os = "macos") {
        cx.defer(move |cx| refresh_menu_states(cx, always));
    }
}

/// Makes the menus again (the recent files changed).
pub(crate) fn refresh_menus(cx: &mut App) {
    if let Some(map) = cx.try_global::<MenuKeyMap>().map(|m| m.0.clone()) {
        set_menus(cx, &map);
    }
}

/// Runs the application until the last window closes. The windows are opened for what arrives on
/// `requests`: first the command line of this process, then, for as long as the process runs,
/// the launches that hand over their files (`crate::launch`) and the files that macOS sends. Each
/// window reads its file itself, in the background, so a big file shows its progress and an
/// unreadable one shows its error in its own window.
pub(crate) fn run(
    user: crate::settings::UserSettings,
    sender: async_channel::Sender<OpenRequest>,
    requests: async_channel::Receiver<OpenRequest>,
) {
    // The key bindings and the menus name the commands of the plugins that are enabled.
    crate::plugin_commands::refresh_for(&user);
    let keymap = user.key_map(matsu_commands::keymap::Platform::current());
    // The search and replace strings of the last runs (`[Keys]`), saved to the same file.
    let search_history = {
        let candidates = matsu_config::location::candidate_paths(
            matsu_config::location::Os::current(),
            &matsu_config::location::Environment::from_process(),
        );
        crate::search::history::SearchHistory::load(
            user.path(),
            matsu_config::location::write_path(&candidates).map(std::path::Path::to_path_buf),
        )
    };
    // The commands and folders of 外部コマンド実行 (`[Cmd]`), saved to the same file.
    let exec_prefs = {
        let candidates = matsu_config::location::candidate_paths(
            matsu_config::location::Os::current(),
            &matsu_config::location::Environment::from_process(),
        );
        crate::exec::ExecPrefs::load(
            user.path(),
            matsu_config::location::write_path(&candidates).map(std::path::Path::to_path_buf),
        )
    };
    // The connections of the database functions (`databases.ini`), next to the settings file.
    let database_prefs = {
        let candidates = matsu_config::location::candidate_paths(
            matsu_config::location::Os::current(),
            &matsu_config::location::Environment::from_process(),
        );
        crate::database::DatabasePrefs::beside(
            matsu_config::location::write_path(&candidates).map(std::path::Path::to_path_buf),
        )
    };
    // The eight print settings (`[Print]`), saved to the same file.
    let print_prefs = {
        let candidates = matsu_config::location::candidate_paths(
            matsu_config::location::Os::current(),
            &matsu_config::location::Environment::from_process(),
        );
        crate::print_setting::PrintPrefs::load(
            user.path(),
            matsu_config::location::write_path(&candidates).map(std::path::Path::to_path_buf),
        )
    };
    // gpui-component's icons (check marks, menu arrows, title bar buttons), on every OS.
    let app = application().with_assets(gpui_kit_assets::Assets);
    // macOS: the files that Finder and the Dock open in the running application.
    app.on_open_urls(move |urls| {
        let request = OpenRequest::from_urls(&urls);
        if !request.paths.is_empty() {
            sender.send_blocking(request).ok();
        }
    });
    app.on_reopen(reopen);
    app.run(move |cx: &mut App| {
        gpui_component::init(cx);
        crate::view::EditorView::bind_keys(cx);
        crate::preview::bind_keys(cx);
        // the title of the confirmation boxes (Windows), as Sakura's: the name of the application
        matsu_platform::dialog_title::install();
        crate::history::install(cx, &user);
        crate::tabs::settings::install(cx, &user);
        crate::bars::install(cx, &user);
        crate::minimap::install(cx, &user);
        crate::common_settings::install(cx, &user);
        crate::saved_state::install(cx, &user);
        // Every window that `open_editor_window` makes applies matsu.ini (read once, above). Before the
        // menus: the row for the custom menus reads it.
        cx.set_global(user);
        cx.set_global(MenuKeyMap(keymap.clone()));
        set_menus(cx, &keymap);
        cx.on_action(|_: &Exit, cx| cx.quit());
        // The files that are open go into the history, with the caret they have now.
        cx.on_app_quit(|cx| {
            // The processes of the plugins end first: none is left running behind the editor.
            crate::macros::plugin_call::shutdown_all(cx);
            crate::history::view::record_all(cx);
            crate::saved_state::window::flush(cx);
            // The copies of the unsaved documents have their latest content when the process ends, however
            // it was asked to end (ADR-0022).
            crate::unsaved::settle_at_quit(cx)
        })
        .detach();
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        crate::search::history::install(cx, search_history);
        crate::exec::install(cx, exec_prefs);
        crate::database::install(cx, database_prefs);
        crate::print_setting::install(cx, print_prefs);
        cx.spawn(async move |cx| {
            // The documents that were not saved the last time come back first (ADR-0022); the files of the
            // command line are opened after them.
            let restore = cx.update(crate::unsaved::restore_all);
            let mut restored = restore.documents;
            let mut pending_notice = restore.pending_notice;
            while let Ok(request) = requests.recv().await {
                cx.update(|cx| {
                    let launch = LaunchOptions::from_request(&request);
                    // A launch that asks for nothing but a window needs none when documents came back.
                    let nothing_to_open =
                        request.paths.is_empty() && restored > 0 && launch.asks_only_for_a_window();
                    restored = 0;
                    if !nothing_to_open
                        && let Err(e) = open_from_outside_with(cx, request.paths, &launch)
                    {
                        eprintln!("matsu: cannot open a window: {e}");
                    }
                    // What the restore could not say (nothing came back, a file was rejected) is said in the window
                    // that opened.
                    if let Some(text) = pending_notice.take()
                        && let Some((_, view)) =
                            crate::file_ops::windows::editors(cx).into_iter().last()
                    {
                        view.update(cx, |view, cx| {
                            view.notice = Some(text);
                            cx.notify();
                        });
                    }
                    // The windows of a launch that was handed over come to the front.
                    cx.activate(true);
                    if cx.windows().is_empty() {
                        cx.quit();
                    }
                });
            }
        })
        .detach();
    });
}

/// The Dock icon was clicked while the application runs (macOS): with no window, a new empty one;
/// otherwise the windows come to the front.
fn reopen(cx: &mut App) {
    if cx.windows().is_empty() {
        open_from_outside(cx, Vec::new()).ok();
    }
    cx.activate(true);
}

/// Sets the menus of the application: Sakura's main menu (`crate::menu`) for an editor tab, and
/// the menus of the other kinds of tab (ADR-0024). macOS shows the menus of the active window's
/// active tab in the menu bar of the screen; elsewhere each window draws the menus of its active
/// tab (`AppMenuBar`), from [`KindMenus`]. An item runs its Sakura function on the editor
/// ([`RunFunction`]); a function without a command is grayed out. The shortcuts shown are those
/// of `keymap`, the user's key map (matsu.ini).
fn set_menus(cx: &mut App, keymap: &KeyMap) {
    let style = if cfg!(target_os = "macos") {
        DisplayStyle::Mac
    } else {
        DisplayStyle::Sakura
    };
    // The native menu shows the kind and the state of the window that is active now; elsewhere the
    // window's own menu bar asks when a menu opens (`crate::menu_bar`).
    let (kind_now, states) = if cfg!(target_os = "macos") {
        active_menu_context(cx)
    } else {
        (MenuKind::Editor, None)
    };
    let by_kind = MenuKind::ALL
        .iter()
        .map(|&kind| {
            let menus = menus_of(cx, keymap, kind, style, None).menus;
            (kind, menus.into_iter().map(Menu::owned).collect())
        })
        .collect();
    cx.set_global(KindMenus(by_kind));
    let native = menus_of(cx, keymap, kind_now, style, states.as_ref());
    cx.set_menus(native.menus);
    // The native menu has just been made again, without pictures: give them (macOS; Windows and
    // Linux draw the menu themselves). `menu_icons::enabled` is Sakura's 「アイコン付きメニュー」.
    if cfg!(target_os = "macos") && crate::menu_icons::enabled(cx) {
        matsu_platform::menu_icons::apply(
            &native.icons,
            f64::from(crate::bars::icons::ICON_SIZE as u32),
        );
    }
    // For `scripts/mac-checks/menu-icons.sh`: write what the menu bar has, pictures included.
    if cfg!(target_os = "macos")
        && let Some(path) = std::env::var_os("MATSU_DUMP_MENU_ICONS")
    {
        let _ = std::fs::write(path, matsu_platform::menu_icons::describe());
    }
    cx.set_global(MenuStates {
        states,
        kind: kind_now,
    });
    let owned = cx.get_menus().unwrap_or_default();
    GlobalState::global_mut(cx).set_app_menus(owned);
    let generation = cx.try_global::<MenuGeneration>().map_or(0, |g| g.0);
    cx.set_global(MenuGeneration(generation + 1));
}

/// The menus of one kind of tab, and the pictures of their items (macOS, `native_menu_icons`).
struct BuiltMenus {
    menus: Vec<Menu>,
    icons: Vec<matsu_platform::menu_icons::Menu>,
}

/// Makes the menus of `kind`, with the marks of `states` on the items of the editor's.
fn menus_of(
    cx: &App,
    keymap: &KeyMap,
    kind: MenuKind,
    style: DisplayStyle,
    states: Option<&crate::menu_state::FuncStates>,
) -> BuiltMenus {
    let mut built = BuiltMenus {
        menus: Vec::new(),
        icons: Vec::new(),
    };
    // macOS has Quit in the application menu, not in the File menu, whatever the kind.
    if cfg!(target_os = "macos") {
        built
            .menus
            .push(Menu::new("MATSU Editor").items([MenuItem::action("MATSU Editor を終了", Exit)]));
        built.icons.push(matsu_platform::menu_icons::Menu {
            title: "MATSU Editor".to_owned(),
            items: vec![matsu_platform::menu_icons::Node::Plain],
        });
    }
    match kind {
        MenuKind::Editor => editor_menus(cx, keymap, style, states, &mut built),
        MenuKind::Preview => {
            for menu in crate::preview::menus::menus(!cfg!(target_os = "macos")) {
                // No Sakura function, so no picture.
                built.icons.push(matsu_platform::menu_icons::Menu {
                    title: menu.name.to_string(),
                    items: vec![matsu_platform::menu_icons::Node::Plain; menu.items.len()],
                });
                built.menus.push(menu);
            }
        }
        #[cfg(test)]
        MenuKind::Probe => {
            built
                .menus
                .push(Menu::new("Probe").items([MenuItem::action("Probe item", Unavailable)]));
            built.icons.push(matsu_platform::menu_icons::Menu {
                title: "Probe".to_owned(),
                items: vec![matsu_platform::menu_icons::Node::Plain],
            });
        }
    }
    built
}

/// The menus of an editor tab: Sakura's main menu, or the user's (`[MainMenu]`).
fn editor_menus(
    cx: &App,
    keymap: &KeyMap,
    style: DisplayStyle,
    states: Option<&crate::menu_state::FuncStates>,
    built: &mut BuiltMenus,
) {
    let recent = crate::history::menu::RecentLists::new(crate::history::current(cx))
        .with_names(crate::file_names::current(cx));
    // The row for the custom menus (`F_CUSTMENU_LIST`) lists the menus that have items.
    let custom_menus = cx
        .try_global::<crate::settings::UserSettings>()
        .map(crate::settings::UserSettings::custom_menus)
        .unwrap_or_default();
    let custom = CustomMenuList(&custom_menus);
    let macro_list = crate::macros::MacroList(crate::macros::registered_table(cx));
    let rest = ChainedLists(&custom, &macro_list);
    let rest = ChainedLists(&rest, &crate::plugin_commands::PluginList);
    let lists = ChainedLists(&recent, &rest);
    // The menu of the user (`[MainMenu]`), Sakura's default without one.
    let main_menu = cx
        .try_global::<crate::settings::UserSettings>()
        .map(crate::settings::UserSettings::main_menu)
        .unwrap_or_else(|| crate::menu::default_main_menu().clone());
    for top in main_menu_for(&main_menu, keymap, &Registered, &lists, &custom_menus) {
        if cfg!(target_os = "macos") {
            built.icons.push(matsu_platform::menu_icons::Menu {
                title: top.label.clone(),
                items: crate::native_menu_icons::tree(&top.entries, style),
            });
        }
        let mut items = gpui_items(top.entries, style);
        if let Some(states) = states {
            crate::menu_state::apply_to_menu_items(&mut items, states);
        }
        built.menus.push(Menu::new(top.label).items(items));
    }
}

/// How many documents the windows other than `except` have (the tabs of each).
pub(crate) fn documents_elsewhere(cx: &App, except: gpui::EntityId) -> usize {
    crate::file_ops::windows::listed_workspaces(cx)
        .iter()
        .filter(|(_, workspace)| workspace.entity_id() != except)
        .map(|(_, workspace)| workspace.read(cx).tab_count())
        .sum()
}

/// The kind of tab and the state of the window that is active, for the native menu: the state is
/// `None` when no editor window is active, when the active tab is not an editor, or when the
/// window is busy (a command is running in it: the menus are made again when that ends).
pub(crate) fn active_menu_context(
    cx: &mut App,
) -> (MenuKind, Option<crate::menu_state::FuncStates>) {
    let none = (MenuKind::Editor, None);
    let Some(handle) = cx.active_window() else {
        return none;
    };
    // The root of a window is not the workspace (`Root` wraps it): the open windows know theirs.
    let Some(workspace) = crate::file_ops::windows::workspace_of(cx, handle) else {
        return none;
    };
    handle
        .update(cx, |_, window, cx| {
            let topmost = crate::window_ops::is_topmost(window, cx);
            let workspace_ref = workspace.read(cx);
            (
                workspace_ref.menu_kind(),
                workspace_ref.menu_states(topmost, workspace.entity_id(), cx),
            )
        })
        .unwrap_or(none)
}

/// The state of the window that is active, for the marks of the native menu.
#[cfg(test)]
pub(crate) fn active_states(cx: &mut App) -> Option<crate::menu_state::FuncStates> {
    active_menu_context(cx).1
}

pub(crate) fn gpui_items(entries: Vec<Entry>, style: DisplayStyle) -> Vec<MenuItem> {
    entries
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Separator => Some(MenuItem::separator()),
            Entry::Submenu { label, entries } => Some(MenuItem::submenu(
                Menu::new(label).items(gpui_items(entries, style)),
            )),
            Entry::Item(item) => {
                let label = label_with_shortcut(&item, style);
                match item.action {
                    MenuAction::Exit if cfg!(target_os = "macos") => None,
                    MenuAction::Exit => Some(MenuItem::action(label, Exit)),
                    MenuAction::Function(code) if item.enabled => {
                        Some(MenuItem::action(label, RunFunction { code }))
                    }
                    MenuAction::RecentFile(path) => {
                        Some(MenuItem::action(label, OpenRecentFile { path }))
                    }
                    MenuAction::RecentFolder(path) => {
                        Some(MenuItem::action(label, OpenRecentFolder { path }))
                    }
                    // Nobody handles `Unavailable`, so the native menu of macOS, which asks
                    // whether the action can be handled, keeps the item gray.
                    MenuAction::Function(_) | MenuAction::Placeholder => {
                        Some(MenuItem::action(label, Unavailable).disabled(true))
                    }
                }
            }
        })
        .collect()
}

/// The content of the window: menu bar, tab bar, editor, status bar.
///
/// A window holds its documents in tabs (ADR-0015); a tab is what a window held before: a document
/// and its panes. The content of the tab that is active is `content`; the other tabs keep theirs
/// in `tabs` (see `crate::tabs`).
pub struct Workspace {
    pub(crate) content: crate::tabs::content::TabContent,
    pub(crate) tabs: crate::tabs::Tabs,
    /// The outline pane, while it is open (`crate::outline`). The settings say at which side of
    /// the window it is docked, or that it floats over the text.
    pub(crate) outline: Option<Entity<crate::outline::pane::OutlinePane>>,
    /// Where the pane and the text are as of the last paint, and the drag of the bar between them.
    pub(crate) outline_layout: crate::outline::layout::OutlineLayout,
    /// The type of the file a new window is loading, until the outline is restored: the document
    /// has the basic type until its file is read (`crate::outline::Workspace::restore_outline`).
    pub(crate) outline_restore_type: Option<usize>,
    /// The outline is being opened again for a new window: that is not the user's choice.
    pub(crate) outline_restoring: bool,
    pub(crate) menu_bar: Entity<MenuBar>,
    /// The search box of the toolbar (`F_SEARCH_BOX`), shown where the toolbar has it.
    pub(crate) search_box: Entity<crate::bars::search_box::SearchBox>,
    /// The minimap at the right of the text, while it is shown (`crate::minimap`).
    pub(crate) minimap: Entity<crate::minimap::panel::MiniMap>,
    /// The files that are being dragged over the window (`crate::file_ops::drop_zone`).
    pub(crate) file_drop: Option<Vec<std::path::PathBuf>>,
}

impl Workspace {
    pub fn new(
        document: Entity<SharedDocument>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| EditorView::new(document, cx));
        Self::with_panes(PaneSet::new(editor), window, cx)
    }

    /// A window whose only tab has `panes` (a tab that moved here from another window, or a new
    /// document).
    pub(crate) fn with_panes(panes: PaneSet, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panes = panes;
        panes.forget_subscriptions();
        let numbered = panes.numbered();
        let active = panes.active().clone();
        window.focus(&active.focus_handle(cx), cx);
        let mut workspace = Self {
            content: crate::tabs::content::TabContent::Editor(panes),
            tabs: crate::tabs::Tabs::new(),
            outline: None,
            outline_layout: Default::default(),
            outline_restore_type: None,
            outline_restoring: false,
            menu_bar: MenuBar::new(cx),
            search_box: crate::bars::search_box::SearchBox::new(cx.weak_entity(), window, cx),
            minimap: cx.new(crate::minimap::panel::MiniMap::new),
            file_drop: None,
        };
        // The menu bar asks this window for the marks of its items when a menu opens.
        let this = cx.weak_entity();
        workspace
            .menu_bar
            .update(cx, |bar, _| bar.set_workspace(this));
        // The keys of a marked menu bar (Windows and Linux) and the letters of an open popup menu
        // come before the key bindings.
        Self::watch_menu_keys(window, cx);
        // The menus are made again when the history changes: the bar loads them.
        cx.observe_global::<MenuGeneration>(|this: &mut Self, cx| {
            this.menu_bar.update(cx, |bar, cx| bar.reload(cx));
        })
        .detach();
        // The tab bar is shown or hidden for every window at once.
        cx.observe_global::<crate::tabs::settings::TabSettings>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::tabs::settings::TabOptions>(|_, cx| cx.notify())
            .detach();
        // The titles and the tooltips of the tabs follow the file name display and the formats.
        cx.observe_global::<crate::file_names::FileNames>(|_, cx| cx.notify())
            .detach();
        cx.observe_global::<crate::file_names::title::TitleFormats>(|_, cx| cx.notify())
            .detach();
        // The toolbar and the function key bar are shown or hidden for every window at once.
        cx.observe_global::<crate::bars::BarSettings>(|_, cx| cx.notify())
            .detach();
        // The minimap is shown or hidden for every window at once.
        cx.observe_global::<crate::minimap::MiniMapSettings>(|_, cx| cx.notify())
            .detach();
        // OK on the toolbar page changes the buttons of every window at once.
        cx.observe_global::<crate::bars::toolbar::ToolbarButtons>(|_, cx| cx.notify())
            .detach();
        // So does the flat form of the buttons.
        cx.observe_global::<crate::bars::toolbar::ToolbarFlat>(|_, cx| cx.notify())
            .detach();
        // macOS: the native menu shows the marks of the active window.
        cx.observe_window_activation(window, |this, window, cx| {
            // The title has a format for the active window and another for the others.
            cx.notify();
            if !window.is_window_active() {
                // A button let go in another program never reaches the view: forget the buttons
                // that the wheel combinations watch.
                this.panes()
                    .active()
                    .clone()
                    .update(cx, |view, _| view.forget_held_buttons());
                // An Alt let go in another program (Alt+Tab) is not a lone Alt tap here.
                this.menu_bar.update(cx, |bar, cx| bar.mouse_down(cx));
            }
            if window.is_window_active() {
                // The click that made the window active may be the next press.
                crate::view::mouse::window_activated(cx, window.window_handle().window_id());
                defer_refresh_menu_states(cx, true);
                // Another program may have changed the file while the window was in the background.
                crate::file_watch::spawn_check(this.panes().active().clone(), window, cx);
            }
        })
        .detach();
        crate::file_watch::start(window, cx);
        crate::unsaved::start(window, cx);
        // The message of the migration shows for 20 seconds: repaint the window then, so that it
        // goes at its time (nothing else repaints an idle window).
        if let Some(left) = crate::settings::startup_notice_left() {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(left).await;
                crate::settings::expire_startup_notice();
                this.update(cx, |_, cx| cx.notify()).ok();
            })
            .detach();
        }
        for (index, pane) in numbered {
            workspace.adopt_into_window(index, pane, window, cx);
        }
        workspace
    }

    /// The state that the check marks and gray items of the menus show (`crate::menu_state`):
    /// the active pane of this window. `topmost` is what the OS says of the window.
    ///
    /// `this` is the entity of this workspace: it may be borrowed (the window is being drawn), so
    /// the documents of the other windows are counted without reading it.
    pub(crate) fn func_states(
        &self,
        topmost: bool,
        this: gpui::EntityId,
        cx: &App,
    ) -> crate::menu_state::FuncStates {
        crate::menu_state::FuncStates::of_view(
            self.panes().active().read(cx),
            self.outline.is_some(),
            topmost,
            self.tab_count() + documents_elsewhere(cx, this),
            cx,
        )
    }

    /// The state the check marks and gray items of the menus show, for the kind of the active tab:
    /// `None` for a kind that is not an editor (its items stay as they are made).
    pub(crate) fn menu_states(
        &self,
        topmost: bool,
        this: gpui::EntityId,
        cx: &App,
    ) -> Option<crate::menu_state::FuncStates> {
        match self.menu_kind() {
            MenuKind::Editor => Some(self.func_states(topmost, this, cx)),
            MenuKind::Preview => None,
            #[cfg(test)]
            MenuKind::Probe => None,
        }
    }

    /// The pane the user works in (the only pane unless the window is split).
    pub fn editor(&self) -> &Entity<EditorView> {
        self.panes().active()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_outline(cx);
        let active = self.panes().active().clone();
        let editor = active.read(cx);
        // A preview (ADR-0024) draws its own view; the bars of the editor are not shown with it.
        let preview = self.content.preview().map(|p| p.view().clone());
        // The list of the tabs (the ▼ button of the bar) belongs to the view that runs its function:
        // with a preview that is the hidden view, which is not drawn, so the list is drawn here.
        let tab_list = editor.tab_list.clone();
        let title = editor.title_for(window.is_window_active(), cx);
        window.set_window_title(&if preview.is_some() {
            format!("プレビュー: {title}")
        } else {
            title
        });
        let status = self.content.panel().status(cx);
        let show_status_bar = editor.status_bar_shown();
        let macro_panel = if preview.is_some() {
            None
        } else {
            editor.macro_panel(active.clone())
        };
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        // A kind of tab with no status bar has no status (`TabPanel::status`).
        let status_bar = match (show_status_bar, &status) {
            (true, Some(status)) => Some(self.render_status_bar(status, &ui_font, window, cx)),
            _ => None,
        };
        // `m_eTabPosition`: under the toolbar, or above the function keys and the status bar.
        let tab_at_bottom = crate::tabs::settings::options(cx).position
            == crate::tabs::settings::TabPosition::Bottom;
        let tab_bar = self.render_tab_bar(window, cx);
        let (tab_bar_top, tab_bar_bottom) = if tab_at_bottom {
            (None, tab_bar)
        } else {
            (tab_bar, None)
        };
        // The minimap draws the pane that is active.
        let minimap_shown = crate::minimap::current(cx).shown && preview.is_none();
        self.minimap.update(cx, |minimap, cx| {
            if minimap_shown {
                minimap.follow(&active, cx);
            } else {
                minimap.forget();
            }
        });
        let minimap = minimap_shown.then(|| self.minimap.clone());
        let topmost = crate::window_ops::is_topmost(window, cx);
        let (toolbar, funckeys_top, funckeys_bottom) = if preview.is_some() {
            (None, None, None)
        } else {
            (
                self.render_toolbar_at_scale(topmost, window.scale_factor(), cx),
                self.render_funckeys(crate::bars::FuncKeyPlace::Top, window, cx),
                self.render_funckeys(crate::bars::FuncKeyPlace::Bottom, window, cx),
            )
        };
        let body = match preview {
            Some(view) => div()
                .size_full()
                .relative()
                .child(view)
                .children(tab_list)
                .into_any_element(),
            None => self.render_panes(cx).into_any_element(),
        };
        let outline = if self.content.preview().is_some() {
            None
        } else {
            self.outline.clone()
        };
        // A Wayland compositor without server-side decorations, such as WSLg's, leaves the title
        // bar and its buttons to the app.
        let client_decorated = cfg!(target_os = "linux")
            && matches!(window.window_decorations(), Decorations::Client { .. });
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x1e1e1e))
            .font_family(ui_font)
            // A dragged tab that is dropped outside the window leaves it (`OnTabLButtonUp`); a
            // drop inside is the business of the elements under the pointer.
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseUpEvent, window, cx| {
                    this.finish_tab_drag(event.position, window, cx);
                }),
            )
            .on_mouse_up(MouseButton::Left, |_, _, cx| {
                crate::tabs::bar::end_tab_drag(cx);
            })
            // The access keys of the menu bar (Windows and Linux; macOS has a native menu).
            .when(!cfg!(target_os = "macos"), |this| {
                this.on_key_down(cx.listener(Self::menu_key))
                    .on_modifiers_changed(cx.listener(Self::menu_modifiers))
                    .capture_any_mouse_down(cx.listener(Self::menu_mouse_down))
            })
            // The whole frame takes dropped files (`DragAcceptFiles( GetHwnd(), TRUE )`).
            .on_drag_move(cx.listener(Self::note_file_drag))
            .child(crate::file_ops::drop_zone::drop_zone(cx))
            // Counts the mouse moves for the letters of the popup menus.
            .child(crate::menu_bar::popup_keys::watch_mouse())
            .when(client_decorated, |this| {
                this.child(TitleBar::new().child(div().text_sm().child("MATSU Editor")))
            })
            // macOS shows the menus in its own menu bar (`cx.set_menus`).
            .when(!cfg!(target_os = "macos"), |this| {
                this.child(
                    div()
                        .border_b_1()
                        .border_color(rgb(0xd0d0d0))
                        .child(self.menu_bar.clone()),
                )
            })
            // Sakura's order under the menu: the toolbar, the function keys (at the top), the tabs.
            .children(toolbar)
            .children(funckeys_top)
            .children(tab_bar_top)
            // The text, the minimap and the outline (`CEditWnd::OnSize`).
            .child(
                div()
                    .flex_1()
                    .flex()
                    .min_h(px(0.))
                    .overflow_hidden()
                    .child(self.render_text_area(body, minimap, outline, window, cx)),
            )
            .children(tab_bar_bottom)
            .children(macro_panel)
            .children(funckeys_bottom)
            .children(status_bar)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::view::{Redo, Undo};
    use gpui::TestAppContext;
    use gpui_component::Root;
    use matsu_core::buffer::Document;
    use matsu_core::model::DocumentModel;

    use super::*;

    /// The icons gpui-component draws are in the assets given to the application on every OS: a
    /// checked box draws `IconName::Check` (`gpui_component::checkbox`), and without the file it
    /// is a filled square.
    #[test]
    fn the_icons_of_the_components_are_in_the_assets() {
        use gpui::AssetSource as _;
        use gpui_component::{IconName, IconNamed as _};
        for icon in [IconName::Check, IconName::ChevronRight, IconName::Close] {
            let path = icon.path();
            let svg = gpui_kit_assets::Assets.load(&path).unwrap();
            assert!(svg.is_some_and(|bytes| !bytes.is_empty()), "{path}");
        }
    }

    /// The whole window (menu bar, editor, status bar) builds and paints, and the menu actions
    /// reach the editor.
    #[gpui::test]
    fn the_window_builds_and_its_menu_edits(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            set_menus(cx, &KeyMap::default());
        });
        let doc = Document::from_utf8(b"abc".to_vec()).unwrap();
        let shared = cx.new(|_| {
            SharedDocument::new(
                DocumentModel::new(doc),
                Some(PathBuf::from("/tmp/memo.txt")),
            )
        });
        let probe = shared.clone();
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let workspace = cx.new(|cx| Workspace::new(shared, window, cx));
            Root::new(workspace, window, cx)
        });
        cx.run_until_parked();
        let menus = cx.update(|_, cx| cx.get_menus().unwrap_or_default());
        assert!(menus.iter().any(|m| m.name.contains("編集")));
        let text = |cx: &mut gpui::VisualTestContext| {
            probe.read_with(cx, |s, _| s.doc().line(0).unwrap().unwrap().text)
        };
        cx.simulate_input("x");
        assert_eq!(text(cx), "xabc");
        cx.dispatch_action(Undo);
        assert_eq!(text(cx), "abc");
        cx.dispatch_action(Redo);
        assert_eq!(text(cx), "xabc");
        // The Edit menu items of Sakura's table run by function code (F_UNDO, F_REDO).
        cx.dispatch_action(RunFunction { code: 30210 });
        assert_eq!(text(cx), "abc");
        cx.dispatch_action(RunFunction { code: 30211 });
        assert_eq!(text(cx), "xabc");
    }

    /// Tab and Shift+Tab reach the editor through the whole window. gpui-component's `Root` binds
    /// them to "move the focus" (`root::Tab`), and GPUI runs key bindings before the key listeners
    /// of the views, so the editor's own handler never saw the key on a real window: Tab typed
    /// nothing and moved the focus to the menu bar.
    #[gpui::test]
    fn tab_in_the_window_is_not_taken_by_the_focus_move(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            crate::view::EditorView::bind_keys(cx);
            set_menus(cx, &KeyMap::default());
        });
        let doc = Document::from_utf8(b"abc".to_vec()).unwrap();
        let shared = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
        let probe = shared.clone();
        let (_root, cx) = cx.add_window_view(|window, cx| {
            let workspace = cx.new(|cx| Workspace::new(shared, window, cx));
            Root::new(workspace, window, cx)
        });
        cx.run_until_parked();
        let focused_before = cx.update(|window, cx| window.focused(cx));
        cx.simulate_keystrokes("tab");
        let text = probe.read_with(cx, |s, _| s.doc().line(0).unwrap().unwrap().text);
        assert_eq!(text, "\tabc");
        cx.simulate_keystrokes("shift-tab");
        assert_eq!(cx.update(|window, cx| window.focused(cx)), focused_before);
    }

    /// The recent files are in the File menu, and the menus are made again when the history
    /// changes.
    #[gpui::test]
    fn the_file_menu_lists_the_recent_files(cx: &mut TestAppContext) {
        use crate::history::{HistoryOptions, HistoryState};
        use matsu_config::history::FileEntry;
        cx.update(|cx| {
            gpui_component::init(cx);
            cx.set_global(HistoryState::in_memory(HistoryOptions::default()));
            cx.set_global(MenuKeyMap(KeyMap::default()));
            set_menus(cx, &KeyMap::default());
        });
        let recent = |cx: &mut TestAppContext| -> Vec<String> {
            let menus = cx.update(|cx| cx.get_menus().unwrap_or_default());
            let file = menus.iter().find(|m| m.name.contains("ファイル")).unwrap();
            let submenu = file
                .items
                .iter()
                .find_map(|i| match i {
                    gpui::OwnedMenuItem::Submenu(m) if m.name.starts_with("最近使ったファイル") => {
                        Some(m)
                    }
                    _ => None,
                })
                .unwrap();
            submenu
                .items
                .iter()
                .filter_map(|i| match i {
                    gpui::OwnedMenuItem::Action { name, .. } => Some(name.to_string()),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(recent(cx), ["(なし)"]);
        let before = cx.update(|cx| cx.global::<MenuGeneration>().0);
        cx.update(|cx| {
            crate::history::update(cx, |h| {
                h.add_file(FileEntry::new("/home/a/メモ.txt"));
                h.add_file(FileEntry::new("/home/a/b.txt"));
            });
        });
        assert_eq!(recent(cx), ["1 /home/a/b.txt", "2 /home/a/メモ.txt"]);
        assert!(cx.update(|cx| cx.global::<MenuGeneration>().0) > before);
    }

    /// The menus handed to GPUI are Sakura's: the File menu has submenus and separators, items
    /// without a command are disabled.
    #[gpui::test]
    fn the_menus_are_sakuras(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            set_menus(cx, &KeyMap::default());
        });
        let menus = cx.update(|cx| cx.get_menus().unwrap_or_default());
        let file = menus
            .iter()
            .find(|m| m.name.contains("ファイル"))
            .expect("the File menu");
        let has = |pred: fn(&gpui::OwnedMenuItem) -> bool| file.items.iter().any(pred);
        assert!(has(|i| matches!(i, gpui::OwnedMenuItem::Separator)));
        assert!(has(|i| matches!(i, gpui::OwnedMenuItem::Submenu(_))));
        // Which items are enabled depends on the commands linked in; `menu::tests` checks that.
    }
}
