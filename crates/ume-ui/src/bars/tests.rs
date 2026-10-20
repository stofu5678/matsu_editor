//! Tests of the toolbar and the function key bar in GPUI's headless test windows: what shows, what
//! a click runs, what the toggles write, and where the function keys are.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AnyWindowHandle, Bounds, Entity, Modifiers, Pixels, TestAppContext, VisualTestContext};
use ume_commands::keymap::{Key, KeyMap, KeyMapOptions, Platform, UserRow};
use ume_commands::sakura::FUNCTIONS;
use ume_config::Profile;
use ume_config::history::HistoryStore;

use super::{BarSettings, FuncKeyPlace};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window, workspace_of};
use crate::history::{HistoryOptions, HistoryState};
use crate::settings::UserSettings;
use crate::view::EditorView;

mod search_box;

/// A folder that is removed at the end of the test.
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The settings file of a test.
struct Ini {
    dir: TempDir,
}

impl Ini {
    fn path(&self) -> PathBuf {
        self.dir.0.join("ume.ini")
    }

    fn get(&self, key: &str) -> Option<String> {
        let profile = Profile::load(&self.path()).ok()?;
        profile.get("Common", key).map(str::to_owned)
    }
}

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// A settings file with `text`, read as `app::run` reads it, and one editor window.
fn setup(cx: &mut TestAppContext, text: &str) -> (Ini, AnyWindowHandle) {
    static N: AtomicU32 = AtomicU32::new(0);
    cx.update(gpui_component::init);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ume-ui-bars-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ini = Ini { dir: TempDir(dir) };
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    std::fs::write(ini.path(), bytes).unwrap();
    cx.update(|cx| {
        let user = UserSettings::load_from(&[ini.path()]);
        cx.set_global(HistoryState::on_disk(
            HistoryStore::new(ini.path(), false),
            HistoryOptions::default(),
        ));
        super::install(cx, &user);
        cx.set_global(user);
    });
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    (ini, handle)
}

fn workspace(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<Workspace> {
    cx.update(|app| workspace_of(app, handle)).unwrap()
}

fn view(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    workspace(cx, handle).read_with(cx, |ws, _| ws.editor().clone())
}

/// Runs a function of Sakura in the window, as a key does.
fn run(cx: &mut TestAppContext, handle: AnyWindowHandle, function: &str) {
    let code = code_of(function);
    let view = view(cx, handle);
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &[], window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn settings(cx: &mut TestAppContext) -> BarSettings {
    cx.update(|cx| super::current(cx))
}

fn text(cx: &mut VisualTestContext, view: &Entity<EditorView>) -> String {
    view.read_with(cx, |view, app| {
        view.document()
            .read(app)
            .doc()
            .line(0)
            .unwrap()
            .map(|line| line.text)
            .unwrap_or_default()
    })
}

/// The bounds a painted element reported under `selector`.
fn bounds(cx: &mut VisualTestContext, selector: String) -> Option<Bounds<Pixels>> {
    cx.debug_bounds(Box::leak(selector.into_boxed_str()))
}

fn center_of(cx: &mut VisualTestContext, selector: String) -> gpui::Point<Pixels> {
    bounds(cx, selector.clone())
        .unwrap_or_else(|| panic!("{selector} is painted"))
        .center()
}

fn click(cx: &mut VisualTestContext, selector: String, modifiers: Modifiers) {
    let at = center_of(cx, selector);
    cx.simulate_click(at, modifiers);
}

// ---- the settings ----

#[gpui::test]
fn without_a_file_the_toolbar_shows_and_the_function_keys_do_not(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(cx, "");
    assert_eq!(settings(cx), BarSettings::default());
    let s = settings(cx);
    assert!(s.toolbar && !s.funckeys);
    assert_eq!(
        (s.funckey_place, s.funckey_group_num),
        (FuncKeyPlace::Top, 4)
    );
}

#[gpui::test]
fn the_keys_of_the_file_are_read(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(
        cx,
        "[Common]\r\nbDispTOOLBAR=0\r\nbDispFUNCKEYWND=1\r\nnFUNCKEYWND_Place=1\r\nnFUNCKEYWND_GroupNum=6\r\n",
    );
    let s = settings(cx);
    assert!(!s.toolbar && s.funckeys);
    assert_eq!(
        (s.funckey_place, s.funckey_group_num),
        (FuncKeyPlace::Bottom, 6)
    );
}

#[gpui::test]
fn a_group_size_out_of_range_is_four_and_any_place_but_one_is_the_top(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(
        cx,
        "[Common]\r\nnFUNCKEYWND_Place=7\r\nnFUNCKEYWND_GroupNum=13\r\n",
    );
    let s = settings(cx);
    assert_eq!(
        (s.funckey_place, s.funckey_group_num),
        (FuncKeyPlace::Top, 4)
    );
}

#[gpui::test]
fn the_toggles_write_the_keys_and_every_window_follows(cx: &mut TestAppContext) {
    let (ini, first) = setup(cx, "[Other]\r\nx=1\r\n");
    let second = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    let shown = |cx: &mut TestAppContext, handle| {
        let ws = workspace(cx, handle);
        ws.update(cx, |ws, cx| ws.render_toolbar(false, cx).is_some())
    };
    assert!(shown(cx, first) && shown(cx, second));

    run(cx, first, "F_SHOWTOOLBAR");
    assert!(!settings(cx).toolbar);
    assert_eq!(ini.get("bDispTOOLBAR").as_deref(), Some("0"));
    assert!(!shown(cx, first) && !shown(cx, second), "both windows");
    // Only the key that changed is written.
    assert_eq!(ini.get("bDispFUNCKEYWND"), None);

    run(cx, second, "F_SHOWTOOLBAR");
    assert!(settings(cx).toolbar);
    assert_eq!(ini.get("bDispTOOLBAR").as_deref(), Some("1"));
    assert!(shown(cx, first) && shown(cx, second));

    run(cx, first, "F_SHOWFUNCKEY");
    assert!(settings(cx).funckeys);
    assert_eq!(ini.get("bDispFUNCKEYWND").as_deref(), Some("1"));
    run(cx, first, "F_SHOWFUNCKEY");
    assert_eq!(ini.get("bDispFUNCKEYWND").as_deref(), Some("0"));
    assert_eq!(ini.get("nFUNCKEYWND_Place"), None);
    // The rest of the file stays.
    let profile = Profile::load(&ini.path()).unwrap();
    assert_eq!(profile.get("Other", "x"), Some("1"));
}

#[gpui::test]
fn the_function_key_bar_is_where_the_setting_says(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\nbDispFUNCKEYWND=1\r\n");
    let ws = workspace(cx, handle);
    let at = |cx: &mut TestAppContext, place| {
        handle
            .update(cx, |_, window, app| {
                ws.update(app, |ws, cx| {
                    ws.render_funckeys(place, window, cx).is_some()
                })
            })
            .unwrap()
    };
    assert!(at(cx, FuncKeyPlace::Top) && !at(cx, FuncKeyPlace::Bottom));
    cx.update(|cx| super::update(cx, |s| s.funckey_place = FuncKeyPlace::Bottom));
    assert!(!at(cx, FuncKeyPlace::Top) && at(cx, FuncKeyPlace::Bottom));
    cx.update(|cx| super::update(cx, |s| s.funckeys = false));
    assert!(!at(cx, FuncKeyPlace::Top) && !at(cx, FuncKeyPlace::Bottom));
}

// ---- the toolbar ----

#[gpui::test]
fn a_click_on_a_toolbar_button_runs_its_function(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    let editor = view(cx, handle);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.simulate_input("x");
    assert_eq!(text(cx, &editor), "x");
    // Button 33 is the undo of Sakura's default toolbar; 34 the redo.
    let undo = center_of(cx, "tool-button-33".into());
    cx.simulate_click(undo, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(text(cx, &editor), "");
    let redo = center_of(cx, "tool-button-34".into());
    cx.simulate_click(redo, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(text(cx, &editor), "x");
}

#[gpui::test]
fn the_toolbar_lists_the_buttons_of_the_file(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(
        cx,
        "[Toolbar]\r\nnToolBarButtonNum=3\r\nnTBB[000]=33\r\nnTBB[001]=0\r\nnTBB[002]=34\r\n",
    );
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let undo = bounds(cx, "tool-button-33".into()).expect("undo");
    let redo = bounds(cx, "tool-button-34".into()).expect("redo");
    assert!(
        bounds(cx, "tool-button-1".into()).is_none(),
        "no new-file button"
    );
    // The separator is a gap between them: wider than the buttons side by side.
    assert!(redo.left() - undo.right() >= gpui::px(8.));
    assert_eq!(undo.top(), redo.top());
}

#[gpui::test]
fn the_default_toolbar_is_above_the_document_and_a_hidden_one_is_not_painted(
    cx: &mut TestAppContext,
) {
    let (_ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let bar = bounds(cx, "tool-bar".into()).expect("the toolbar");
    let new_file = bounds(cx, "tool-button-1".into()).expect("new file");
    assert!(bar.contains(&new_file.center()));
    cx.update(|_, app| super::update(app, |s| s.toolbar = false));
    cx.run_until_parked();
    assert!(bounds(cx, "tool-bar".into()).is_none());
}

#[gpui::test]
fn a_button_without_a_command_does_nothing(cx: &mut TestAppContext) {
    // Button 481 is a free place of the table: gray, no function.
    let (_ini, handle) = setup(cx, "[Toolbar]\r\nnToolBarButtonNum=1\r\nnTBB[000]=481\r\n");
    let editor = view(cx, handle);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.simulate_input("x");
    let at = center_of(cx, "tool-button-481".into());
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(text(cx, &editor), "x");
    assert!(!super::is_available(0));
}

// ---- the function key bar ----

/// F1 is undo, Shift+F1 redo, Ctrl+F1 nothing; F2 selects all.
fn user_map() -> KeyMap {
    let mut map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
    let mut f1 = [0u16; 8];
    f1[0] = code_of("F_UNDO");
    f1[1] = code_of("F_REDO");
    let mut f2 = [0u16; 8];
    f2[0] = code_of("F_SELECTALL");
    map.apply_user_rows(&[
        UserRow {
            key: Key::function(1).unwrap(),
            functions: f1,
        },
        UserRow {
            key: Key::function(2).unwrap(),
            functions: f2,
        },
    ]);
    map
}

#[gpui::test]
fn a_click_on_a_function_key_runs_the_function_of_the_key_map(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\nbDispFUNCKEYWND=1\r\n");
    cx.update(|cx| crate::app::set_key_map(cx, user_map()));
    let editor = view(cx, handle);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.simulate_input("x");
    // F1 is index 0.
    let f1 = center_of(cx, "function-key-0".into());
    cx.simulate_click(f1, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(text(cx, &editor), "");
}

#[gpui::test]
fn the_buttons_follow_the_modifiers_held(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\nbDispFUNCKEYWND=1\r\n");
    cx.update(|cx| crate::app::set_key_map(cx, user_map()));
    let editor = view(cx, handle);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    cx.simulate_input("x");
    click(cx, "function-key-0".into(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(text(cx, &editor), "");
    // With Shift held, the first button is the redo.
    cx.simulate_modifiers_change(Modifiers::shift());
    cx.run_until_parked();
    click(cx, "function-key-0".into(), Modifiers::shift());
    cx.run_until_parked();
    assert_eq!(text(cx, &editor), "x");
    // With Ctrl held it has no function: a click does nothing.
    cx.simulate_modifiers_change(Modifiers::control());
    cx.run_until_parked();
    click(cx, "function-key-0".into(), Modifiers::control());
    cx.run_until_parked();
    assert_eq!(text(cx, &editor), "x");
}

#[gpui::test]
fn the_groups_are_set_apart_by_the_group_size(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(
        cx,
        "[Common]\r\nbDispFUNCKEYWND=1\r\nnFUNCKEYWND_GroupNum=3\r\n",
    );
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let gap = |cx: &mut VisualTestContext, i: usize| {
        let before = bounds(cx, format!("function-key-{}", i - 1)).unwrap();
        let after = bounds(cx, format!("function-key-{i}")).unwrap();
        after.left() - before.right()
    };
    // Groups of 3: the gaps are before F4, F7 and F10.
    for i in 1..12 {
        let wide = gap(cx, i) > gpui::px(6.);
        assert_eq!(wide, i % 3 == 0, "before F{}", i + 1);
    }
    // 12 buttons in a row, equally wide.
    let first = bounds(cx, "function-key-0".into()).unwrap();
    let last = bounds(cx, "function-key-11".into()).unwrap();
    assert_eq!(first.size.width.round(), last.size.width.round());
    assert!(last.right() > first.left());
}

#[gpui::test]
fn the_function_key_bar_is_over_the_document_at_the_top_and_under_it_at_the_bottom(
    cx: &mut TestAppContext,
) {
    let (_ini, top) = setup(cx, "[Common]\r\nbDispFUNCKEYWND=1\r\n");
    let editor = view(cx, top);
    let at_top = {
        let cx = &mut VisualTestContext::from_window(top, cx);
        let bar = bounds(cx, "function-key-bar".into()).unwrap();
        let toolbar = bounds(cx, "tool-bar".into()).unwrap();
        // Under the toolbar.
        assert!(bar.top() >= toolbar.bottom());
        bar.top()
    };
    cx.update(|cx| super::update(cx, |s| s.funckey_place = FuncKeyPlace::Bottom));
    cx.run_until_parked();
    let cx = &mut VisualTestContext::from_window(top, cx);
    let at_bottom = bounds(cx, "function-key-bar".into()).unwrap().top();
    assert!(at_bottom > at_top, "{at_bottom:?} {at_top:?}");
    // Still the same window and document.
    assert_eq!(text(cx, &editor), "");
    assert_eq!(cx.update(|_, app| editors(app).len()), 1);
}

#[gpui::test]
fn registered_macros_and_custom_menus_are_named_on_the_bars(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(
        cx,
        "[Macro]\r\nName[001]=MyMacro\r\nFile[001]=a.mac\r\nName[002]=\r\nFile[002]=b.mac\r\n\
         [CustMenu]\r\nszCMN[01]=My menu\r\n",
    );
    // F1: macro 1 (has a name), F2: macro 2 (the file only), F3: macro 3 (not registered),
    // F4: custom menu 1, F5: the right-click menu.
    let mut map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
    let rows: Vec<UserRow> = [31601u16, 31602, 31603, 31501, 31580]
        .into_iter()
        .enumerate()
        .map(|(i, code)| {
            let mut functions = [0u16; 8];
            functions[0] = code;
            UserRow {
                key: Key::function(i as u8 + 1).unwrap(),
                functions,
            }
        })
        .collect();
    map.apply_user_rows(&rows);
    let keys = cx.update(|app| {
        let names = crate::func_name::FuncNames::new();
        super::funckeys::labels(&map, ume_commands::keymap::Modifiers::NONE, &|c| {
            names.name(app, c)
        })
    });
    let labels: Vec<&str> = keys.iter().take(5).map(|k| k.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "MyMacro",
            "b.mac",
            "マクロ 3 (未登録)",
            "My menu",
            "右クリックメニュー"
        ]
    );
    // The tooltips of the toolbar say the same (the keys are listed after the name).
    let tip = |cx: &mut TestAppContext, code| {
        let text = cx.update(|app| super::toolbar::tooltip_text(code, app));
        text.lines().next().unwrap_or_default().to_owned()
    };
    assert_eq!(tip(cx, 31601), "MyMacro");
    assert_eq!(tip(cx, 31603), "マクロ 3 (未登録)");
    assert_eq!(tip(cx, 31501), "My menu");
    assert_eq!(tip(cx, code_of("F_FILESAVE")), "上書き保存");
}

/// A screen reader reads the name of a toolbar button, which is the name of its function: every
/// button of Sakura's default toolbar has a real one.
#[gpui::test]
fn every_default_toolbar_button_has_a_name(cx: &mut TestAppContext) {
    let _ = setup(cx, "");
    let names: Vec<(u16, String)> = cx.update(|app| {
        let names = crate::func_name::FuncNames::new();
        super::toolbar::arrange(&super::toolbar::ToolbarButtons::default().0)
            .into_iter()
            .flatten()
            .filter_map(|item| match item {
                super::toolbar::Item::Button(button) => Some(button.click_code()),
                _ => None,
            })
            .map(|code| (code, names.name(app, code)))
            .collect()
    });
    assert!(names.len() > 15, "{names:?}");
    for (code, name) in names {
        assert!(!name.is_empty(), "{code}");
        assert_ne!(name, crate::func_name::UNKNOWN_NAME, "{code}");
    }
}
