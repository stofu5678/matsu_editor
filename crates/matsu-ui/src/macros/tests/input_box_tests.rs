//! `InputBox`, `CreateMenu` and `ChangeInsertSpace` called from a macro. The dialog and the menu
//! are answered with keys, the way a user does; the scripts run with Node.js (a test whose
//! interpreter is missing prints "SKIP" and passes).

use gpui::{TestAppContext, VisualTestContext};
use matsu_plugin::session::Language;

use super::{Fixture, available, results, setup};

impl Fixture {
    fn dialog_message(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, cx| {
            v.input_dialog
                .as_ref()
                .map(|dialog| dialog.read(cx).message().to_owned())
        })
    }

    fn menu_open(&self, cx: &mut VisualTestContext) -> bool {
        self.view.read_with(cx, |v, _| v.custom_menu.is_open())
    }
}

/// Runs `code` and types `typed` and `key` into the dialog each time one opens. Returns the
/// output and the messages the dialogs showed.
fn ask(cx: &mut TestAppContext, code: &str, typed: &[&str], key: &str) -> (String, Vec<String>) {
    cx.update(gpui_component::init);
    let (f, cx) = setup(cx, "");
    let mut shown = Vec::new();
    let mut count = 0;
    let output = f.script_answering(cx, code, |cx| {
        if let Some(message) = f.dialog_message(cx) {
            shown.push(message);
            // the default is selected when the dialog opens; End keeps it under what is typed
            cx.simulate_keystrokes("end");
            if let Some(text) = typed.get(count) {
                cx.simulate_input(text);
            }
            count += 1;
            cx.simulate_keystrokes(key);
        }
    });
    (output, shown)
}

#[gpui::test]
fn input_box_returns_the_default_with_what_was_typed(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (output, shown) = ask(
        cx,
        "console.log('r=' + Editor.InputBox('名前は？', '既定', 10));",
        &["の値"],
        "enter",
    );
    assert_eq!(results(&output), ["既定の値"], "{output}");
    assert_eq!(shown, ["名前は？"]);
}

#[gpui::test]
fn input_box_gives_an_empty_string_when_it_is_cancelled(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (output, _) = ask(
        cx,
        "console.log('r=[' + Editor.InputBox('m', 'default') + ']');",
        &["x"],
        "escape",
    );
    assert_eq!(results(&output), ["[]"], "{output}");
}

#[gpui::test]
fn input_box_cuts_the_text_at_the_maximum_length(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    // The default is cut to the length, and what is typed beyond it is not taken.
    let (output, _) = ask(
        cx,
        "console.log('r=' + Editor.InputBox('m', '', 3));\n\
         console.log('r=' + Editor.InputBox('m', 'あいうえお', 2));\n\
         console.log('r=' + Editor.InputBox('m', 'a', 4));\n",
        &["あいうえお", "", "𠮷𠮷"],
        "enter",
    );
    // 𠮷 is two UTF-16 code units: two of them make the 4.
    assert_eq!(results(&output), ["あいう", "あい", "a𠮷"], "{output}");
}

#[gpui::test]
fn input_box_takes_260_characters_when_no_length_is_given(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let long = "a".repeat(300);
    let (output, _) = ask(
        cx,
        "console.log('r=' + Editor.InputBox('m').length);\n\
         console.log('r=' + Editor.InputBox('m', '', 0).length);\n\
         console.log('r=' + Editor.InputBox('m', '', -5).length);\n",
        &[long.as_str(), long.as_str(), long.as_str()],
        "enter",
    );
    assert_eq!(results(&output), ["260", "260", "260"], "{output}");
}

/// Runs `code` and answers each menu that opens with `keys`. Returns the output and where the
/// menus opened.
fn pick(
    cx: &mut TestAppContext,
    code: &str,
    keys: &str,
) -> (
    String,
    Vec<gpui::Point<gpui::Pixels>>,
    gpui::Point<gpui::Pixels>,
) {
    cx.update(gpui_component::init);
    let (f, cx) = setup(cx, "ab\ncd");
    // The caret goes to the second line, second character.
    cx.simulate_keystrokes("down right");
    cx.run_until_parked();
    let mut at = Vec::new();
    let output = f.script_answering(cx, code, |cx| {
        if f.menu_open(cx) {
            at.extend(f.view.read_with(cx, |v, _| v.custom_menu.open_at()));
            cx.simulate_keystrokes(keys);
        }
    });
    let caret = f.view.read_with(cx, |v, app| {
        v.point_of(v.cursor().caret(), app)
            .expect("the window was painted")
    });
    (output, at, caret)
}

#[gpui::test]
fn create_menu_answers_the_number_of_the_item_chosen(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    // Separators and submenus take no number: え is 3, お is 4 and う is 5.
    let menu = "[C]あ,-,い,[S]サブ,え,[E]お,う";
    let code = format!("console.log('r=' + Editor.CreateMenu(0, '{menu}'));");
    for (keys, expected) in [
        ("down enter", "1"),
        ("down down enter", "2"),
        ("down down down down enter", "5"),
        // closed without a choice
        ("escape", "0"),
    ] {
        let (output, _, _) = pick(cx, &code, keys);
        assert_eq!(results(&output), [expected], "{keys}: {output}");
    }
}

#[gpui::test]
fn create_menu_opens_a_submenu_and_skips_disabled_items(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    // Down to the submenu, right opens it, and a key chooses in it: い and う are in the submenu
    // (nothing ends it), numbered after あ.
    let (output, _, _) = pick(
        cx,
        "console.log('r=' + Editor.CreateMenu(0, 'あ,[S]サブ,い,う'));",
        "down down right down enter",
    );
    assert_eq!(results(&output), ["3"], "{output}");
    // The disabled item cannot be chosen: the second key skips it. (The first key of GPUI's menu
    // selects the first item whatever it is; Enter on a disabled item gives 0, not its number.)
    let (output, _, _) = pick(
        cx,
        "console.log('r=' + Editor.CreateMenu(0, '[D]あ,い'));",
        "down down enter",
    );
    assert_eq!(results(&output), ["2"], "{output}");
    let (output, _, _) = pick(
        cx,
        "console.log('r=' + Editor.CreateMenu(0, '[D]あ,い'));",
        "down enter",
    );
    assert_eq!(results(&output), ["0"], "{output}");
}

#[gpui::test]
fn create_menu_without_an_item_answers_0_and_shows_nothing(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (output, at, _) = pick(
        cx,
        "console.log('r=' + Editor.CreateMenu(0, ''));\n\
         console.log('r=' + Editor.CreateMenu(1, '-,,[S]サブ'));\n",
        "escape",
    );
    assert_eq!(results(&output), ["0", "0"], "{output}");
    assert!(at.is_empty());
}

#[gpui::test]
fn create_menu_opens_at_the_caret_or_the_pointer(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (output, at, caret) = pick(
        cx,
        "console.log('r=' + Editor.CreateMenu(1, 'あ'));\n\
         console.log('r=' + Editor.CreateMenu(0, 'あ'));\n",
        "down enter",
    );
    assert_eq!(results(&output), ["1", "1"], "{output}");
    let [at_caret, at_pointer] = at.as_slice() else {
        panic!("{at:?}");
    };
    assert_eq!(*at_caret, caret);
    // The pointer has not moved in the test window.
    assert_ne!(*at_pointer, caret);
}

#[gpui::test]
fn change_insert_space_reads_and_sets_the_flag_of_the_window(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "");
    let output = f.script(
        cx,
        "console.log('r=' + [Editor.ChangeInsertSpace(1), Editor.ChangeInsertSpace(2), \
         Editor.ChangeInsertSpace(0), Editor.ChangeInsertSpace(5)].join(','));\n\
         Editor.ChangeInsertSpace(1);",
    );
    assert_eq!(results(&output), ["0,1,1,0"], "{output}");
    // The window keeps it for the next macro.
    let output = f.script(cx, "console.log('r=' + Editor.ChangeInsertSpace(2));");
    assert_eq!(results(&output), ["1"], "{output}");
}
