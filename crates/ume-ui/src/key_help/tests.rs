//! Tests of the keyword help in GPUI's headless test windows: the tip when the mouse rests, on the
//! selection and on the word at the caret, the switch `F_TOGGLE_KEY_SEARCH`, the items of the
//! right-click menu and the dictionary that they open. The rules of the lookup are tested in
//! [`super::lookup`].
//!
//! Expectations cite `sakura_core/view/CEditView_Search.cpp`, `sakura_core/view/CEditView.cpp`
//! (`OnTimer`, `CreatePopUpMenu_R`) and `sakura_core/cmd/CViewCommander_CustMenu.cpp`.

use std::time::Duration;

use gpui::{
    AppContext as _, Entity, MenuItem, Modifiers, TestAppContext, VisualTestContext, point, px,
};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use super::{caret_key_word, set_caret_key_word};
use crate::document::SharedDocument;
use crate::settings::UserSettings;
use crate::view::EditorView;

/// `UTF-8` with the BOM: two entries, the first with a line break in the meaning.
const DICT: &[u8] = "\u{FEFF}alpha,Alpha2 /// the first letter\\nsecond line\r\n\
beta /// second\r\n\
日本語 /// Japanese\r\n"
    .as_bytes();

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// A folder with the settings file and the dictionary `dict.txt`, removed at the end of the test.
struct Files {
    dir: tempfile::TempDir,
}

impl Files {
    fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.dir.path().join(name), bytes).unwrap();
    }
}

/// Settings with `types_0` as `[Types(0)]` (the type of an untitled document), and the folder
/// with `dict.txt`.
fn settings(types_0: &str) -> (Files, UserSettings) {
    let files = Files {
        dir: tempfile::tempdir().unwrap(),
    };
    files.write("dict.txt", DICT);
    let user = user_for(&files, types_0);
    (files, user)
}

/// The settings as the application reads them (again) in the folder of `files`.
fn user_for(files: &Files, types_0: &str) -> UserSettings {
    let path = files.dir.path().join("ume.ini");
    let profile = ume_config::Profile::from_bytes(format!("[Types(0)]\r\n{types_0}").as_bytes());
    UserSettings::from_profile(profile, Some(path))
}

/// The type with the keyword help on and the dictionary.
const ON: &str = "bUseKeyWordHelp=1\r\nKDct[00]=1,Dict,dict.txt\r\n";

struct Fixture {
    view: Entity<EditorView>,
}

/// A view over `text` in an application with `user`, the caret at the start.
fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    user: UserSettings,
) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(|app| {
        app.set_global(user);
        // The switch is for the session; start from off whatever a test before left.
        set_caret_key_word(app, false);
    });
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (Fixture { view }, vcx)
}

impl Fixture {
    fn tip(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, _| {
            v.support.key_help.tip.as_ref().map(|t| t.text.clone())
        })
    }

    fn tip_at(&self, cx: &mut VisualTestContext) -> Option<(f32, f32)> {
        self.view.read_with(cx, |v, _| {
            v.support
                .key_help
                .tip
                .as_ref()
                .map(|t| (f32::from(t.at.x), f32::from(t.at.y)))
        })
    }

    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
        let code = code_of(function);
        self.view
            .update_in(cx, |v, window, cx| v.run_function(code, args, window, cx));
    }
}

/// Moves the mouse over the text and lets `millis` pass.
fn rest(cx: &mut VisualTestContext, x: f32, y: f32, millis: u64) {
    cx.simulate_mouse_move(point(px(x), px(y)), None, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(millis));
    cx.run_until_parked();
}

// ---- the tip ----

#[gpui::test]
fn the_tip_shows_when_the_mouse_has_rested_for_300_ms_on_a_selection(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta\ngamma", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    // `300 < GetTickCount() - m_dwTipTimer`
    rest(cx, 120.0, 50.0, 250);
    assert_eq!(f.tip(cx), None);
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.run_until_parked();
    // the meaning, with the escaped line break (`UnEscapeInfoText`)
    assert_eq!(f.tip(cx).as_deref(), Some("the first letter\nsecond line"));
    // `Show( po.x, po.y + GetHankakuHeight() )`: one row below the pointer
    let (x, y) = f.tip_at(cx).unwrap();
    let line_height = f.view.read_with(cx, |v, _| v.metrics.line_height);
    assert_eq!((x, y), (120.0, 50.0 + line_height));
}

#[gpui::test]
fn a_move_of_the_pointer_hides_the_tip_and_the_word_is_looked_up_again(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    rest(cx, 120.0, 50.0, 400);
    assert!(f.tip(cx).is_some());
    cx.simulate_mouse_move(point(px(130.), px(50.)), None, Modifiers::default());
    assert_eq!(f.tip(cx), None);
    // it rests again: the key was found before, so it is searched again
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    assert!(f.tip(cx).is_some());
}

#[gpui::test]
fn the_pointer_that_keeps_moving_shows_no_tip(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    for step in 0..5 {
        rest(cx, 100.0 + step as f32 * 5.0, 50.0, 200);
    }
    assert_eq!(f.tip(cx), None);
}

#[gpui::test]
fn a_key_hides_the_tip(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    rest(cx, 120.0, 50.0, 400);
    assert!(f.tip(cx).is_some());
    cx.simulate_keystrokes("right");
    assert_eq!(f.tip(cx), None);
}

#[gpui::test]
fn without_a_selection_the_word_at_the_caret_is_looked_up_only_when_the_switch_is_on(
    cx: &mut TestAppContext,
) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "beta alpha", user);
    // the caret is in "beta"; the switch is off
    rest(cx, 120.0, 50.0, 400);
    assert_eq!(f.tip(cx), None);
    // `F_TOGGLE_KEY_SEARCH`
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[]);
    assert!(cx.update(|_, app| caret_key_word(app)));
    rest(cx, 130.0, 50.0, 400);
    assert_eq!(f.tip(cx).as_deref(), Some("second"));
}

#[gpui::test]
fn a_word_that_was_not_found_is_not_searched_again_until_it_changes(cx: &mut TestAppContext) {
    let (files, user) = settings(ON);
    let (f, cx) = setup(cx, "gamma beta", user);
    cx.update(|_, app| set_caret_key_word(app, true));
    rest(cx, 120.0, 50.0, 400);
    assert_eq!(f.tip(cx), None);
    // the dictionary gets the word, but the key was searched and not found: no search
    files.write("dict.txt", "gamma /// third\r\n".as_bytes());
    rest(cx, 130.0, 50.0, 400);
    assert_eq!(f.tip(cx), None);
    // another word is searched, and then the first one again
    cx.simulate_keystrokes("ctrl-end");
    rest(cx, 140.0, 50.0, 400);
    assert_eq!(f.tip(cx), None);
    cx.simulate_keystrokes("ctrl-home");
    rest(cx, 150.0, 50.0, 400);
    assert_eq!(f.tip(cx).as_deref(), Some("third"));
}

#[gpui::test]
fn the_type_decides_whether_the_help_is_on(cx: &mut TestAppContext) {
    let (_files, user) = settings("KDct[00]=1,Dict,dict.txt\r\n");
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    rest(cx, 120.0, 50.0, 400);
    assert_eq!(f.tip(cx), None);
}

#[gpui::test]
fn the_options_of_the_type_shape_the_text(cx: &mut TestAppContext) {
    // 「キーワードも表示する」 and 「選択範囲で前方一致検索」
    let (_files, user) = settings(&format!(
        "{ON}bUseKeyHelpKeyDisp=1\r\nbUseKeyHelpPrefix=1\r\n"
    ));
    let (f, cx) = setup(cx, "alp beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right");
    rest(cx, 120.0, 50.0, 400);
    assert_eq!(
        f.tip(cx).as_deref(),
        Some("[ alp ]\n--------------------\nalpha >>\nthe first letter\nsecond line")
    );
}

#[gpui::test]
fn the_japanese_word_is_found(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "日本語 beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right");
    rest(cx, 120.0, 50.0, 400);
    assert_eq!(f.tip(cx).as_deref(), Some("Japanese"));
}

#[gpui::test]
fn a_dictionary_that_is_missing_gives_no_tip(cx: &mut TestAppContext) {
    let (_files, user) = settings("bUseKeyWordHelp=1\r\nKDct[00]=1,Gone,gone.txt\r\n");
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    rest(cx, 120.0, 50.0, 400);
    assert_eq!(f.tip(cx), None);
}

// ---- F_TOGGLE_KEY_SEARCH ----

#[gpui::test]
fn the_switch_toggles_with_no_number_and_follows_the_numbers(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha", user);
    let on = |cx: &mut VisualTestContext| cx.update(|_, app| caret_key_word(app));
    assert!(!on(cx));
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[]);
    assert!(on(cx));
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[Arg::Int(0)]);
    assert!(!on(cx));
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[Arg::Int(1)]);
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[Arg::Int(1)]);
    assert!(on(cx));
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[Arg::Int(2)]);
    assert!(!on(cx));
    // another number changes nothing
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[Arg::Int(1)]);
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[Arg::Int(9)]);
    assert!(on(cx));
}

#[gpui::test]
fn the_menu_item_is_checked_while_the_switch_is_on(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha", user);
    let code = code_of("F_TOGGLE_KEY_SEARCH");
    let checked = |f: &Fixture, cx: &mut VisualTestContext| {
        f.view.update_in(cx, |v, _, cx| {
            crate::menu_state::FuncStates::of_view(v, false, false, 1, cx).is_checked(code)
        })
    };
    assert!(!checked(&f, cx));
    f.run(cx, "F_TOGGLE_KEY_SEARCH", &[]);
    assert!(checked(&f, cx));
    // the function can be chosen: it has a command
    let enabled = f.view.update_in(cx, |v, _, cx| {
        crate::menu_state::FuncStates::of_view(v, false, false, 1, cx).is_enabled(code)
    });
    assert!(enabled);
}

// ---- the right-click menu ----

fn labels(items: &[MenuItem]) -> Vec<String> {
    items
        .iter()
        .map(|item| match item {
            MenuItem::Action { name, .. } => name.to_string(),
            MenuItem::Separator => "-".to_owned(),
            _ => "?".to_owned(),
        })
        .collect()
}

fn menu_items(f: &Fixture, cx: &mut VisualTestContext) -> (Vec<String>, Vec<String>) {
    f.view.update_in(cx, |v, window, cx| {
        let (top, bottom) = v.key_help_menu_items(true, window, cx);
        (labels(&top), labels(&bottom))
    })
}

#[gpui::test]
fn the_menu_has_the_items_at_the_top_as_the_type_says(cx: &mut TestAppContext) {
    // `nKeyHelpRMenuShowType` 1 is the default
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    let (top, bottom) = menu_items(&f, cx);
    assert_eq!(
        top,
        [
            "キーワードの説明をクリップボードにコピー(K)",
            "キーワード辞書を開く(L)",
            "-"
        ]
    );
    assert!(bottom.is_empty());
}

#[gpui::test]
fn the_menu_has_the_items_at_the_bottom_or_not_at_all(cx: &mut TestAppContext) {
    let (_files, user) = settings(&format!("{ON}nKeyHelpRMenuShowType=2\r\n"));
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    let (top, bottom) = menu_items(&f, cx);
    assert!(top.is_empty());
    assert_eq!(
        bottom,
        [
            "-",
            "キーワードの説明をクリップボードにコピー(K)",
            "キーワード辞書を開く(L)"
        ]
    );
    let (_files, user) = settings(&format!("{ON}nKeyHelpRMenuShowType=0\r\n"));
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    assert_eq!(menu_items(&f, cx), (Vec::new(), Vec::new()));
}

#[gpui::test]
fn the_menu_has_no_items_for_a_word_that_is_not_in_a_dictionary(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    // the word at the caret is not looked up with the switch off
    assert_eq!(menu_items(&f, cx), (Vec::new(), Vec::new()));
    cx.update(|_, app| set_caret_key_word(app, true));
    cx.simulate_keystrokes("ctrl-end");
    // "beta" is there
    assert_eq!(menu_items(&f, cx).0.len(), 3);
}

#[gpui::test]
fn the_right_click_menu_opens_with_the_items_alone_when_it_has_no_others(cx: &mut TestAppContext) {
    // `CreatePopUpMenu_R` does not look at the number of items of the menu
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    let code = code_of("F_MENU_RBUTTON");
    f.view.update_in(cx, |v, window, cx| {
        v.run_function_at(code, point(px(40.), px(40.)), window, cx);
    });
    assert!(f.view.read_with(cx, |v, _| v.custom_menu.is_open()));
}

#[gpui::test]
fn the_copy_item_puts_the_text_on_the_clipboard_with_crlf(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    menu_items(&f, cx);
    f.view.update(cx, |v, cx| v.copy_key_help(cx));
    assert_eq!(
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .as_deref(),
        Some("the first letter\r\nsecond line")
    );
}

#[gpui::test]
fn the_open_item_opens_the_dictionary_at_the_line_of_the_hit(cx: &mut TestAppContext) {
    use crate::file_ops::windows::editors;
    let (files, user) = settings(ON);
    cx.update(gpui_component::init);
    cx.update(|app| {
        app.set_global(user);
        set_caret_key_word(app, false);
    });
    let handle = cx
        .update(|app| {
            crate::file_ops::windows::open_editor_window(app, SharedDocument::empty(), None)
        })
        .unwrap();
    cx.run_until_parked();
    let view = cx.update(|app| {
        editors(app)
            .into_iter()
            .find(|(h, _)| *h == handle)
            .map(|(_, v)| v)
            .unwrap()
    });
    // the word "beta" (line 2 of the dictionary) is selected
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| {
            c.insert_text("beta").ok();
            c.select_range(Pos::new(0, 0), Pos::new(0, 4)).ok();
        });
        assert!(v.key_help_lookup(cx));
    });
    handle
        .update(cx, |_, window, app| {
            view.update(app, |v, cx| v.open_key_help_dict(window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    let (opened, _) = cx.update(editors).pop().unwrap();
    assert_ne!(opened, handle, "the dictionary is in a window of its own");
    let dictionary = cx.update(|app| {
        editors(app)
            .into_iter()
            .find(|(h, _)| *h == opened)
            .map(|(_, v)| v)
            .unwrap()
    });
    let (name, caret) = dictionary.read_with(cx, |v, app| {
        (v.document().read(app).file_name(), v.cursor().caret())
    });
    assert_eq!(name.as_deref(), Some("dict.txt"));
    assert_eq!(caret.line, 1);
    drop(files);
}

// ---- beside the list of the input completion ----

#[gpui::test]
fn the_chosen_candidate_has_its_meaning_beside_the_list(cx: &mut TestAppContext) {
    // `CHokanMgr::ShowTip`: the dictionaries of the type, with the candidate as the key
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alp alpha\nal", user);
    cx.simulate_keystrokes("ctrl-end");
    f.run(cx, "F_HOKAN", &[]);
    let chosen = |f: &Fixture, cx: &mut VisualTestContext| {
        f.view.read_with(cx, |v, app| {
            let word = v
                .support
                .completion
                .as_ref()
                .and_then(|c| c.chosen().map(str::to_owned));
            (word.clone(), word.and_then(|w| v.completion_help(&w, app)))
        })
    };
    // "alp" is in no dictionary
    assert_eq!(chosen(&f, cx), (Some("alp".to_owned()), None));
    cx.simulate_keystrokes("down");
    assert_eq!(
        chosen(&f, cx),
        (
            Some("alpha".to_owned()),
            Some("the first letter\nsecond line".to_owned())
        )
    );
}

#[gpui::test]
fn no_meaning_is_looked_up_beside_the_list_when_the_help_is_off(cx: &mut TestAppContext) {
    let (_files, user) = settings("KDct[00]=1,Dict,dict.txt\r\n");
    let (f, cx) = setup(cx, "alp alpha\nal", user);
    let help = f
        .view
        .read_with(cx, |v, app| v.completion_help("alpha", app));
    assert_eq!(help, None);
}

// ---- the settings change ----

#[gpui::test]
fn a_change_of_the_settings_makes_a_word_that_was_not_found_be_searched_again(
    cx: &mut TestAppContext,
) {
    let (files, user) = settings(ON);
    let (f, cx) = setup(cx, "gamma beta", user);
    cx.update(|_, app| set_caret_key_word(app, true));
    rest(cx, 120.0, 50.0, 400);
    assert_eq!(f.tip(cx), None);
    files.write("dict.txt", "gamma /// third\r\n".as_bytes());
    // the same key is not searched again ...
    rest(cx, 130.0, 50.0, 400);
    assert_eq!(f.tip(cx), None);
    // ... until the settings are read again (OK of the type settings)
    cx.update(|_, app| app.set_global(user_for(&files, ON)));
    rest(cx, 140.0, 50.0, 400);
    assert_eq!(f.tip(cx).as_deref(), Some("third"));
}

#[gpui::test]
fn a_change_of_the_settings_changes_the_meaning_beside_the_list(cx: &mut TestAppContext) {
    let (files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha", user);
    let help = |f: &Fixture, cx: &mut VisualTestContext| {
        f.view
            .read_with(cx, |v, app| v.completion_help("alpha", app))
    };
    assert_eq!(
        help(&f, cx).as_deref(),
        Some("the first letter\nsecond line")
    );
    files.write("dict.txt", "alpha /// changed\r\n".as_bytes());
    // the word is kept for the list that is open ...
    assert_eq!(
        help(&f, cx).as_deref(),
        Some("the first letter\nsecond line")
    );
    // ... and looked up again with the new settings
    cx.update(|_, app| app.set_global(user_for(&files, ON)));
    assert_eq!(help(&f, cx).as_deref(), Some("changed"));
    // the type has the help off now
    cx.update(|_, app| app.set_global(user_for(&files, "")));
    assert_eq!(help(&f, cx), None);
}

#[gpui::test]
fn moving_the_caret_with_the_keyboard_shows_the_help_of_the_new_word(cx: &mut TestAppContext) {
    // `HandleCommand` hides the tip and starts the timer again; the pointer stays in the window
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.update(|_, app| set_caret_key_word(app, true));
    rest(cx, 120.0, 50.0, 400);
    assert_eq!(f.tip(cx).as_deref(), Some("the first letter\nsecond line"));
    cx.simulate_keystrokes("ctrl-end");
    assert_eq!(f.tip(cx), None);
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    assert_eq!(f.tip(cx).as_deref(), Some("second"));
}

#[gpui::test]
fn the_tip_uses_the_font_of_the_page_as_soon_as_it_is_set(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha", user);
    let text_font = f.view.read_with(cx, |v, _| v.font());
    let tip_font = |f: &Fixture, cx: &mut VisualTestContext| {
        f.view.read_with(cx, |v, app| v.key_help_font(app))
    };
    // no font of the page: the font of the text
    assert_eq!(tip_font(&f, cx), text_font);
    let mut font = ume_config::settings::FontSetting::default();
    font.log_font.face_name = "Sample Face".to_owned();
    font.point_size = 120;
    cx.update(|_, app| {
        app.set_global(crate::support::prefs::HelperPrefs {
            key_help_font: Some(font),
            ..Default::default()
        });
    });
    let (family, size) = tip_font(&f, cx);
    assert_eq!(family.as_ref(), "Sample Face");
    assert_eq!(size, crate::settings::view_settings::points_to_px(120));
}

#[gpui::test]
fn the_tip_comes_when_a_selection_by_the_mouse_ends(cx: &mut TestAppContext) {
    let (_files, user) = settings(ON);
    let (f, cx) = setup(cx, "alpha beta", user);
    cx.simulate_keystrokes("shift-right shift-right shift-right shift-right shift-right");
    // the button is down while the pointer moves: no tip
    let at = point(px(120.), px(50.));
    cx.simulate_event(gpui::MouseMoveEvent {
        position: at,
        pressed_button: Some(gpui::MouseButton::Left),
        modifiers: Modifiers::default(),
    });
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    assert_eq!(f.tip(cx), None);
    // the button goes up and the pointer rests
    cx.simulate_mouse_up(at, gpui::MouseButton::Left, Modifiers::default());
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    assert_eq!(f.tip(cx).as_deref(), Some("the first letter\nsecond line"));
}
