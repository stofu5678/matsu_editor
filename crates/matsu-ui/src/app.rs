//! The application: the window with a menu bar, the editor and a status bar.
//!
//! The menu is Sakura's main menu (`crate::menu`).

use gpui::{
    App, AppContext as _, Context, Decorations, Entity, Focusable, InteractiveElement as _,
    IntoElement, Menu, MenuItem, ParentElement, Render, Styled, Window, actions, div,
    prelude::FluentBuilder, rgb,
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
struct MenuStates(Option<crate::menu_state::FuncStates>);

impl gpui::Global for MenuStates {}

/// macOS: makes the native menu again if the marks of the active window are not the ones it shows.
/// Called when a window becomes active and after a function ran, because the native menu cannot
/// ask when it opens. Does nothing elsewhere (the window's menu bar asks).
///
/// `always` skips the comparison: the snapshot does not include what the disk says (whether a
/// file is there, a header of the same name), so a window that becomes active makes them again.
fn refresh_menu_states(cx: &mut App, always: bool) {
    let now = active_states(cx);
    if !always && cx.try_global::<MenuStates>().is_some_and(|m| m.0 == now) {
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
        crate::history::install(cx, &user);
        crate::tabs::settings::install(cx, &user);
        crate::bars::install(cx, &user);
        crate::minimap::install(cx, &user);
        crate::common_settings::install(cx, &user);
        // Every window that `open_editor_window` makes applies matsu.ini (read once, above). Before the
        // menus: the row for the custom menus reads it.
        cx.set_global(user);
        cx.set_global(MenuKeyMap(keymap.clone()));
        set_menus(cx, &keymap);
        cx.on_action(|_: &Exit, cx| cx.quit());
        // The files that are open go into the history, with the caret they have now.
        cx.on_app_quit(|cx| {
            crate::history::view::record_all(cx);
            std::future::ready(())
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
        crate::print_setting::install(cx, print_prefs);
        cx.spawn(async move |cx| {
            while let Ok(request) = requests.recv().await {
                cx.update(|cx| {
                    let launch = LaunchOptions::from_request(&request);
                    if let Err(e) = open_from_outside_with(cx, request.paths, &launch) {
                        eprintln!("matsu: cannot open a window: {e}");
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

/// Sets the menus of the application: Sakura's main menu (`crate::menu`). macOS shows them in the
/// menu bar of the screen; elsewhere the window draws them (`AppMenuBar`), from the copy in
/// gpui-component's global state. An item runs its Sakura function on the editor
/// ([`RunFunction`]); a function without a command is grayed out. The shortcuts shown are those
/// of `keymap`, the user's key map (matsu.ini).
fn set_menus(cx: &mut App, keymap: &KeyMap) {
    let style = if cfg!(target_os = "macos") {
        DisplayStyle::Mac
    } else {
        DisplayStyle::Sakura
    };
    let mut menus = Vec::new();
    // macOS has Quit in the application menu, not in the File menu.
    if cfg!(target_os = "macos") {
        menus
            .push(Menu::new("MATSU Editor").items([MenuItem::action("MATSU Editor を終了", Exit)]));
    }
    // The native menu shows the state of the window that is active now; elsewhere the window's
    // own menu bar asks when a menu opens (`crate::menu_bar`).
    let states = if cfg!(target_os = "macos") {
        active_states(cx)
    } else {
        None
    };
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
    let lists = ChainedLists(&recent, &rest);
    // The menu of the user (`[MainMenu]`), Sakura's default without one.
    let main_menu = cx
        .try_global::<crate::settings::UserSettings>()
        .map(crate::settings::UserSettings::main_menu)
        .unwrap_or_else(|| crate::menu::default_main_menu().clone());
    for top in main_menu_for(&main_menu, keymap, &Registered, &lists, &custom_menus) {
        let mut items = gpui_items(top.entries, style);
        if let Some(states) = &states {
            crate::menu_state::apply_to_menu_items(&mut items, states);
        }
        menus.push(Menu::new(top.label).items(items));
    }
    cx.set_menus(menus);
    cx.set_global(MenuStates(states));
    let owned = cx.get_menus().unwrap_or_default();
    GlobalState::global_mut(cx).set_app_menus(owned);
    let generation = cx.try_global::<MenuGeneration>().map_or(0, |g| g.0);
    cx.set_global(MenuGeneration(generation + 1));
}

/// How many documents the windows other than `except` have (the tabs of each).
pub(crate) fn documents_elsewhere(cx: &App, except: gpui::EntityId) -> usize {
    crate::file_ops::windows::listed_workspaces(cx)
        .iter()
        .filter(|(_, workspace)| workspace.entity_id() != except)
        .map(|(_, workspace)| workspace.read(cx).tab_count())
        .sum()
}

/// The state of the window that is active, for the marks of the native menu. `None` when no
/// editor window is active, or when it is busy (a command is running in it: the menus are made
/// again when that ends).
pub(crate) fn active_states(cx: &mut App) -> Option<crate::menu_state::FuncStates> {
    let handle = cx.active_window()?;
    // The root of a window is not the workspace (`Root` wraps it): the open windows know theirs.
    let workspace = crate::file_ops::windows::workspace_of(cx, handle)?;
    handle
        .update(cx, |_, window, cx| {
            let topmost = crate::window_ops::is_topmost(window, cx);
            workspace
                .read(cx)
                .func_states(topmost, workspace.entity_id(), cx)
        })
        .ok()
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
/// and its panes. The panes of the tab that is active are in `panes`; the other tabs keep theirs
/// in `tabs` (see `crate::tabs`).
pub struct Workspace {
    pub(crate) panes: PaneSet,
    pub(crate) tabs: crate::tabs::Tabs,
    /// The outline pane docked at the right of the window, while it is open (`crate::outline`).
    pub(crate) outline: Option<Entity<crate::outline::pane::OutlinePane>>,
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
            panes,
            tabs: crate::tabs::Tabs::new(),
            outline: None,
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
        // macOS: the native menu shows the marks of the active window.
        cx.observe_window_activation(window, |this, window, cx| {
            // The title has a format for the active window and another for the others.
            cx.notify();
            if window.is_window_active() {
                defer_refresh_menu_states(cx, true);
                // Another program may have changed the file while the window was in the background.
                crate::file_watch::spawn_check(this.panes.active().clone(), window, cx);
            }
        })
        .detach();
        crate::file_watch::start(window, cx);
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
            self.panes.active().read(cx),
            self.outline.is_some(),
            topmost,
            self.tab_count() + documents_elsewhere(cx, this),
            cx,
        )
    }

    /// The pane the user works in (the only pane unless the window is split).
    pub fn editor(&self) -> &Entity<EditorView> {
        self.panes.active()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_outline(cx);
        let active = self.panes.active().clone();
        let editor = active.read(cx);
        window.set_window_title(&editor.title_for(window.is_window_active(), cx));
        let status = editor.status(cx);
        let show_status_bar = editor.status_bar_shown();
        let macro_panel = editor.macro_panel(active.clone());
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
        let status_bar = if show_status_bar {
            Some(self.render_status_bar(&status, &ui_font, window, cx))
        } else {
            None
        };
        let tab_bar = self.render_tab_bar(window, cx);
        // The minimap draws the pane that is active.
        let minimap_shown = crate::minimap::current(cx).shown;
        self.minimap.update(cx, |minimap, cx| {
            if minimap_shown {
                minimap.follow(&active, cx);
            } else {
                minimap.forget();
            }
        });
        let minimap = minimap_shown.then(|| self.minimap.clone());
        let topmost = crate::window_ops::is_topmost(window, cx);
        let toolbar = self.render_toolbar_at_scale(topmost, window.scale_factor(), cx);
        let funckeys_top = self.render_funckeys(crate::bars::FuncKeyPlace::Top, window, cx);
        let funckeys_bottom = self.render_funckeys(crate::bars::FuncKeyPlace::Bottom, window, cx);
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
            // The whole frame takes dropped files (`DragAcceptFiles( GetHwnd(), TRUE )`).
            .on_drag_move(cx.listener(Self::note_file_drag))
            .child(crate::file_ops::drop_zone::drop_zone(cx))
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
            .children(tab_bar)
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .overflow_hidden()
                            .child(self.render_panes(cx)),
                    )
                    // Left of the outline, which is docked at the right (`CEditWnd::OnSize`).
                    .children(minimap)
                    .children(self.outline.clone()),
            )
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
