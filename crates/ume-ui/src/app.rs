//! The application: the window with a menu bar, the editor and a status bar.
//!
//! The menu is Sakura's main menu (`crate::menu`).

use std::path::PathBuf;

use gpui::{
    App, AppContext as _, Context, Decorations, Entity, Focusable, IntoElement, Menu, MenuItem,
    ParentElement, Render, Styled, Window, actions, div, prelude::FluentBuilder, px, rgb,
};
use gpui_component::{GlobalState, TitleBar};
use gpui_platform::application;

use crate::document::SharedDocument;
use crate::file_ops::windows::open_startup_windows;
use crate::menu::{Entry, MenuAction, Registered, label_with_shortcut, main_menu_with};
use crate::menu_bar::MenuBar;
use crate::panes::PaneSet;
use crate::status::status_text;
use crate::view::{EditorView, OpenRecentFile, RunFunction};
use ume_commands::keymap::{DisplayStyle, KeyMap};

actions!(
    ume,
    [
        Exit,
        /// A menu item that cannot run: no one handles this action.
        Unavailable
    ]
);

/// The key map the menus show the shortcuts of, kept so that the menus can be made again when the
/// history changes.
struct MenuKeyMap(KeyMap);

impl gpui::Global for MenuKeyMap {}

/// Counts how many times the menus were made. A window watches it to load the new menus.
pub(crate) struct MenuGeneration(u64);

impl gpui::Global for MenuGeneration {}

/// Makes the menus again (the recent files changed).
pub(crate) fn refresh_menus(cx: &mut App) {
    if let Some(map) = cx.try_global::<MenuKeyMap>().map(|m| m.0.clone()) {
        set_menus(cx, &map);
    }
}

/// Opens a window for each of `paths` (or one untitled window without any) and runs until the
/// last window closes. Each window reads its file itself, in the background, so a big file shows
/// its progress and an unreadable one shows its error in its own window.
pub fn run(paths: Vec<PathBuf>) {
    let user = crate::settings::UserSettings::load();
    let keymap = user.key_map(ume_commands::keymap::Platform::current());
    // gpui-component's icons (check marks, menu arrows, title bar buttons), on every OS.
    let app = application().with_assets(gpui_kit_assets::Assets);
    app.run(move |cx: &mut App| {
        gpui_component::init(cx);
        crate::history::install(cx, &user);
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

        // Every window that `open_editor_window` makes applies ume.ini (read once, above).
        cx.set_global(user);
        open_startup_windows(cx, paths).expect("failed to open the window");
        cx.activate(true);
    });
}

/// Sets the menus of the application: Sakura's main menu (`crate::menu`). macOS shows them in the
/// menu bar of the screen; elsewhere the window draws them (`AppMenuBar`), from the copy in
/// gpui-component's global state. An item runs its Sakura function on the editor
/// ([`RunFunction`]); a function without a command is grayed out. The shortcuts shown are those
/// of `keymap`, the user's key map (ume.ini).
fn set_menus(cx: &mut App, keymap: &KeyMap) {
    let style = if cfg!(target_os = "macos") {
        DisplayStyle::Mac
    } else {
        DisplayStyle::Sakura
    };
    let mut menus = Vec::new();
    // macOS has Quit in the application menu, not in the File menu.
    if cfg!(target_os = "macos") {
        menus.push(Menu::new("UME Editor").items([MenuItem::action("UME Editor を終了", Exit)]));
    }
    let lists = crate::history::menu::RecentLists::new(crate::history::current(cx));
    for top in main_menu_with(keymap, &Registered, &lists) {
        let items = gpui_items(top.entries, style);
        menus.push(Menu::new(top.label).items(items));
    }
    cx.set_menus(menus);
    let owned = cx.get_menus().unwrap_or_default();
    GlobalState::global_mut(cx).set_app_menus(owned);
    let generation = cx.try_global::<MenuGeneration>().map_or(0, |g| g.0);
    cx.set_global(MenuGeneration(generation + 1));
}

fn gpui_items(entries: Vec<Entry>, style: DisplayStyle) -> Vec<MenuItem> {
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
                    MenuAction::RecentFile(index) => {
                        Some(MenuItem::action(label, OpenRecentFile { index }))
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

/// The content of the window: menu bar, editor, status bar.
pub struct Workspace {
    pub(crate) panes: PaneSet,
    menu_bar: Entity<MenuBar>,
}

impl Workspace {
    pub fn new(
        document: Entity<SharedDocument>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| EditorView::new(document, cx));
        window.focus(&editor.focus_handle(cx), cx);
        let host = cx.weak_entity();
        editor.update(cx, |view, _| view.split_host = Some(host));
        let mut workspace = Self {
            panes: PaneSet::new(editor.clone()),
            menu_bar: MenuBar::new(cx),
        };
        // The menus are made again when the history changes: the bar loads them.
        cx.observe_global::<MenuGeneration>(|this: &mut Self, cx| {
            this.menu_bar.update(cx, |bar, cx| bar.reload(cx));
        })
        .detach();
        workspace.adopt(0, editor, window, cx);
        workspace
    }

    /// The pane the user works in (the only pane unless the window is split).
    pub fn editor(&self) -> &Entity<EditorView> {
        self.panes.active()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.panes.active().clone();
        let editor = active.read(cx);
        window.set_window_title(&editor.title(cx));
        let status = status_text(&editor.status(cx));
        let show_status_bar = editor.status_bar_shown();
        let macro_panel = editor.macro_panel(active.clone());
        let ui_font = gpui_component::Theme::global(cx).font_family.clone();
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
            .when(client_decorated, |this| {
                this.child(TitleBar::new().child(div().text_sm().child("UME Editor")))
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
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.render_panes(cx)),
            )
            .children(macro_panel)
            .when(show_status_bar, |this| {
                this.child(
                    div()
                        .px_2()
                        .py_0p5()
                        .text_size(px(12.))
                        .bg(rgb(0xe8e8e8))
                        .text_color(rgb(0x333333))
                        .child(status),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::view::{Redo, Undo};
    use gpui::TestAppContext;
    use gpui_component::Root;
    use ume_core::buffer::Document;
    use ume_core::model::DocumentModel;

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

    /// The recent files are in the File menu, and the menus are made again when the history
    /// changes.
    #[gpui::test]
    fn the_file_menu_lists_the_recent_files(cx: &mut TestAppContext) {
        use crate::history::{HistoryOptions, HistoryState};
        use ume_config::history::FileEntry;
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
