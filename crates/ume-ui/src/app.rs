//! The application: the window with a menu bar, the editor and a status bar.
//!
//! The menu is the smallest that shows the way: File > Exit and Edit > Undo / Redo. Sakura's whole
//! main menu is in `ume_commands::sakura::MAIN_MENU` and comes in a later PR.

use std::path::PathBuf;

use gpui::{
    App, AppContext as _, Bounds, Context, Decorations, Entity, Focusable, IntoElement, Menu,
    MenuItem, ParentElement, Render, Styled, Window, WindowBounds, WindowOptions, actions, div,
    prelude::FluentBuilder, px, rgb, size,
};
use gpui_component::menu::AppMenuBar;
use gpui_component::{GlobalState, Root, TitleBar};
use gpui_platform::application;

use crate::document::SharedDocument;
use crate::open::OpenError;
use crate::status::status_text;
use crate::view::{EditorView, Redo, Undo};

actions!(ume, [Exit]);

/// Why the editor could not start.
#[derive(Debug)]
pub struct StartError {
    pub path: PathBuf,
    pub error: OpenError,
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cannot open {}: {}", self.path.display(), self.error)
    }
}

impl std::error::Error for StartError {}

/// Opens the window and runs until it closes. `path` is the file to open (UTF-8 for now); without
/// it the editor starts with an empty document.
pub fn run(path: Option<PathBuf>) -> Result<(), StartError> {
    let shared = match &path {
        Some(path) => SharedDocument::open(path).map_err(|error| StartError {
            path: path.clone(),
            error,
        })?,
        None => SharedDocument::empty(),
    };
    let user = crate::settings::UserSettings::load();
    let app = application();
    #[cfg(target_os = "linux")]
    let app = app.with_assets(gpui_kit_assets::Assets);
    app.run(move |cx: &mut App| {
        gpui_component::init(cx);
        set_menus(cx);
        cx.on_action(|_: &Exit, cx| cx.quit());
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1000.), px(700.)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: Some("UME Editor".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        cx.open_window(options, move |window, cx| {
            let document = cx.new(|_| shared);
            let workspace = cx.new(|cx| Workspace::new(document, window, cx));
            // sakura.ini: the font, the layout, the colors and the key bindings.
            workspace.update(cx, |workspace, cx| {
                workspace
                    .editor
                    .update(cx, |editor, cx| editor.apply_user_settings(&user, cx));
            });
            cx.new(|cx| Root::new(workspace, window, cx))
        })
        .expect("failed to open the window");
        cx.activate(true);
    });
    Ok(())
}

/// Sets the menus of the application. macOS shows them in the menu bar of the screen; elsewhere
/// the window draws them (`AppMenuBar`), from the copy in gpui-component's global state.
fn set_menus(cx: &mut App) {
    let mut file = Vec::new();
    // macOS has Quit in the application menu, not in the File menu.
    if !cfg!(target_os = "macos") {
        file.push(MenuItem::action("終了(X)", Exit));
    }
    let mut menus = Vec::new();
    if cfg!(target_os = "macos") {
        menus.push(Menu::new("UME Editor").items([MenuItem::action("UME Editor を終了", Exit)]));
    }
    menus.push(Menu::new("ファイル(F)").items(file));
    menus.push(Menu::new("編集(E)").items([
        MenuItem::action("元に戻す(U)", Undo),
        MenuItem::action("やり直し(R)", Redo),
    ]));
    cx.set_menus(menus);
    let owned = cx.get_menus().unwrap_or_default();
    GlobalState::global_mut(cx).set_app_menus(owned);
}

/// The content of the window: menu bar, editor, status bar.
pub struct Workspace {
    editor: Entity<EditorView>,
    menu_bar: Entity<AppMenuBar>,
}

impl Workspace {
    pub fn new(
        document: Entity<SharedDocument>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = cx.new(|cx| EditorView::new(document, cx));
        window.focus(&editor.focus_handle(cx), cx);
        Self {
            editor,
            menu_bar: AppMenuBar::new(cx),
        }
    }

    pub fn editor(&self) -> &Entity<EditorView> {
        &self.editor
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.read(cx);
        window.set_window_title(&editor.title(cx));
        let status = status_text(&editor.status(cx));
        let show_status_bar = editor.status_bar_shown();
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
            .child(div().flex_1().overflow_hidden().child(self.editor.clone()))
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
    use gpui::TestAppContext;
    use ume_core::buffer::Document;
    use ume_core::model::DocumentModel;

    use super::*;

    /// The whole window (menu bar, editor, status bar) builds and paints, and the menu actions
    /// reach the editor.
    #[gpui::test]
    fn the_window_builds_and_its_menu_edits(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_component::init(cx);
            set_menus(cx);
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
    }
}
