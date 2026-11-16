//! Alt+letter in the Grep dialogs (`IsDialogMessage`, `IDD_GREP` and `IDD_GREP_REPLACE` in
//! `sakura_core/sakura_rc.rc`): the letters of the labels act while the focus is in a box, where the
//! letter is not typed. The letters are Sakura's, so no two controls of one dialog share one.

use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::Document;
use matsu_core::model::DocumentModel;

use super::format::{LineType, OutputStyle};
use super::panel::GrepPanel;
use crate::document::SharedDocument;
use crate::view::EditorView;

fn setup(cx: &mut TestAppContext) -> (Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(Vec::new()).unwrap();
    let shared = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.executor().allow_parking();
    (view, vcx)
}

fn open(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
    function: &str,
) -> Entity<GrepPanel> {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    for _ in 0..50 {
        cx.run_until_parked();
        cx.executor().advance_clock(Duration::from_millis(60));
    }
    view.read_with(cx, |v, _| v.grep.panel.clone())
        .expect("the dialog is open")
}

fn key_text(cx: &mut VisualTestContext, panel: &Entity<GrepPanel>) -> String {
    panel.read_with(cx, |p, app| p.key.read(app).value().to_string())
}

fn set_key(cx: &mut VisualTestContext, panel: &Entity<GrepPanel>, text: &str) {
    panel.update_in(cx, |p, window, cx| {
        p.key
            .update(cx, |state, cx| state.set_value(text.to_owned(), window, cx));
        p.focus_key(window, cx);
    });
}

fn press(cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    cx.run_until_parked();
}

#[gpui::test]
fn the_letters_of_the_check_boxes_switch_them_from_the_box_and_type_nothing(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_DIALOG");
    set_key(cx, &panel, "abc");
    let state = |cx: &mut VisualTestContext| {
        panel.read_with(cx, |p, _| {
            let s = &p.settings;
            (
                s.sub_folder,
                s.options.match_case,
                s.options.whole_word,
                s.options.regex,
                s.except_file_regex,
                s.file_only,
            )
        })
    };
    let start = state(cx);
    // サブフォルダー(S), 大小(C), 単語(W), 正規表現(E) greys 単語 out, 除外の正規表現(Q), 最初のみ(7)
    for letter in ["s", "c", "w"] {
        press(cx, &format!("alt-{letter}"));
    }
    assert_eq!(
        state(cx),
        (!start.0, !start.1, !start.2, start.3, start.4, start.5)
    );
    press(cx, "alt-e");
    // greyed out: the box does not change
    press(cx, "alt-w");
    assert_eq!(
        state(cx),
        (!start.0, !start.1, !start.2, !start.3, start.4, start.5)
    );
    press(cx, "alt-q");
    press(cx, "alt-7");
    assert_eq!(
        state(cx),
        (!start.0, !start.1, !start.2, !start.3, !start.4, !start.5)
    );
    // Shift does not matter
    press(cx, "alt-shift-c");
    assert_eq!(state(cx).1, start.1);
    // nothing went into the box
    assert_eq!(key_text(cx, &panel), "abc");
    assert!(view.read_with(cx, |v, _| v.grep_panel_open()));
}

#[gpui::test]
fn the_digits_choose_the_output_and_the_form(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_DIALOG");
    let state = |cx: &mut VisualTestContext| {
        panel.read_with(cx, |p, _| {
            (
                p.settings.line_type,
                p.settings.style,
                p.settings.base_folder,
                p.settings.separate_folder,
            )
        })
    };
    press(cx, "alt-3");
    assert_eq!(state(cx).0, LineType::NoHitLine);
    press(cx, "alt-2");
    assert_eq!(state(cx).0, LineType::MatchPart);
    press(cx, "alt-1");
    assert_eq!(state(cx).0, LineType::Line);
    press(cx, "alt-5");
    assert_eq!(state(cx).1, OutputStyle::PerFile);
    press(cx, "alt-6");
    assert_eq!(state(cx).1, OutputStyle::ResultOnly);
    // 結果のみ has no folders to show: the two check boxes are greyed out
    let (_, _, base, separate) = state(cx);
    press(cx, "alt-9");
    press(cx, "alt-8");
    assert_eq!((state(cx).2, state(cx).3), (base, separate));
    press(cx, "alt-4");
    assert_eq!(state(cx).1, OutputStyle::Normal);
    press(cx, "alt-9");
    press(cx, "alt-8");
    assert_eq!((state(cx).2, state(cx).3), (!base, !separate));
}

#[gpui::test]
fn the_letters_of_the_labels_move_the_focus_to_their_boxes(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_DIALOG");
    let texts = |cx: &mut VisualTestContext| -> [String; 5] {
        panel.read_with(cx, |p, app| {
            [
                &p.key,
                &p.file,
                &p.folder,
                &p.exclude_file,
                &p.exclude_folder,
            ]
            .map(|state| state.read(app).value().to_string())
        })
    };
    // the typing replaces the selected text of the box that took the focus
    for (letter, at) in [("i", 1), ("l", 2), ("j", 3), ("k", 4), ("n", 0)] {
        press(cx, &format!("alt-{letter}"));
        cx.simulate_input("Q");
        assert_eq!(texts(cx)[at], "Q", "alt-{letter}");
    }
    // each letter went to its own box only, and none was typed
    assert_eq!(texts(cx), ["Q", "Q", "Q", "Q", "Q"].map(String::from));
}

#[gpui::test]
fn the_check_box_for_the_text_of_the_window_greys_out_the_boxes_of_the_files(
    cx: &mut TestAppContext,
) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_DIALOG");
    let this_text =
        |cx: &mut VisualTestContext| panel.read_with(cx, |p, _| p.settings.from_this_text);
    assert!(!this_text(cx));
    press(cx, "alt-m");
    assert!(this_text(cx));
    // the sub folders are not searched then: its letter does nothing
    let sub = panel.read_with(cx, |p, _| p.settings.sub_folder);
    press(cx, "alt-s");
    assert_eq!(panel.read_with(cx, |p, _| p.settings.sub_folder), sub);
    press(cx, "alt-m");
    assert!(!this_text(cx));
    press(cx, "alt-s");
    assert_eq!(panel.read_with(cx, |p, _| p.settings.sub_folder), !sub);
}

#[gpui::test]
fn the_charset_goes_to_the_next_with_its_letter(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_DIALOG");
    let before = panel.read_with(cx, |p, _| p.settings.charset);
    press(cx, "alt-a");
    let after = panel.read_with(cx, |p, _| p.settings.charset);
    assert_eq!(after, super::panel::next_charset(before));
}

#[gpui::test]
fn alt_x_cancels_and_alt_f_starts_the_search_from_a_box(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_DIALOG");
    // 検索(F) with a folder that is not there is the box of the mistake, not a search
    panel.update_in(cx, |p, window, cx| {
        p.folder
            .update(cx, |state, cx| state.set_value("", window, cx));
        p.file
            .update(cx, |state, cx| state.set_value("*.*", window, cx));
    });
    set_key(cx, &panel, "abc");
    press(cx, "alt-f");
    assert!(cx.has_pending_prompt());
    assert_eq!(key_text(cx, &panel), "abc");
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    press(cx, "alt-x");
    assert!(!view.read_with(cx, |v, _| v.grep_panel_open()));
}

#[gpui::test]
fn the_grep_replace_dialog_has_its_own_letters(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx);
    let panel = open(&view, cx, "F_GREP_REPLACE_DLG");
    // 置換後(Z): the box of the replacement takes the focus
    press(cx, "alt-z");
    cx.simulate_input("new");
    assert_eq!(
        panel.read_with(cx, |p, app| p.replacement.read(app).value().to_string()),
        "new"
    );
    // クリップボードの内容で置換(V) greys the box of the replacement out
    press(cx, "alt-v");
    assert!(panel.read_with(cx, |p, _| p.settings.paste));
    press(cx, "alt-z");
    cx.simulate_input("zzz");
    assert_eq!(
        panel.read_with(cx, |p, app| p.replacement.read(app).value().to_string()),
        "new"
    );
    // バックアップ作成(B)
    let backup = panel.read_with(cx, |p, _| p.settings.backup);
    press(cx, "alt-b");
    assert_eq!(panel.read_with(cx, |p, _| p.settings.backup), !backup);
    // 置換前(N), and 編集中のテキストから検索(M) is greyed out for a replacement
    press(cx, "alt-m");
    assert!(!panel.read_with(cx, |p, _| p.settings.from_this_text));
    // 置換(R) with no condition is the box of the mistake
    press(cx, "alt-r");
    assert!(cx.has_pending_prompt());
}
