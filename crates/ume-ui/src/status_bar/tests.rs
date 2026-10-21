use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use gpui_component::Root;
use ume_core::buffer::{Document, Eol, Pos};
use ume_core::encoding::Encoding;
use ume_core::model::DocumentModel;

use super::*;
use crate::document::SharedDocument;
use crate::search::PanelMode;

// ---- the pure parts ----

#[test]
fn the_double_clicks_run_the_functions_of_sakura() {
    // CEditWnd::DispatchEvent, NM_DBLCLK
    assert_eq!(double_click_function(Part::Position), Some("F_JUMP_DIALOG"));
    assert_eq!(double_click_function(Part::Charset), Some("F_CHG_CHARSET"));
    assert_eq!(double_click_function(Part::Rec), Some("F_RECKEYMACRO"));
    assert_eq!(double_click_function(Part::Mode), Some("F_CHGMOD_INS"));
    // the code box is not a function
    assert_eq!(double_click_function(Part::CharCode), None);
    for part in [Part::Message, Part::Eol, Part::Zoom] {
        assert_eq!(double_click_function(part), None, "{part:?}");
    }
    for name in [
        "F_JUMP_DIALOG",
        "F_CHG_CHARSET",
        "F_RECKEYMACRO",
        "F_CHGMOD_INS",
    ] {
        assert!(code_of(name).is_some(), "{name}");
    }
}

#[test]
fn the_line_ending_menu_has_crlf_lf_and_cr() {
    // The first three are always there; NEL, LS and PS come after them when `bEnableExtEol` is on
    // (tested with the check box, in the tests of the common settings).
    let labels: Vec<_> = eol_menu_entries()
        .into_iter()
        .take(3)
        .map(|e| match e {
            Entry::Item(item) => (item.label, item.action),
            other => panic!("{other:?}"),
        })
        .collect();
    let function = |name| MenuAction::Function(code_of(name).unwrap());
    assert_eq!(
        labels,
        [
            (
                "入力改行コード指定(CRLF)".to_owned(),
                function("F_CHGMOD_EOL_CRLF")
            ),
            (
                "入力改行コード指定(LF)".to_owned(),
                function("F_CHGMOD_EOL_LF")
            ),
            (
                "入力改行コード指定(CR)".to_owned(),
                function("F_CHGMOD_EOL_CR")
            ),
        ]
    );
}

#[test]
fn the_code_box_lists_the_codes_in_each_encoding() {
    // STR_ERR_DLGEDITWND13
    let message = code_box_message("あい").unwrap();
    assert_eq!(
        message,
        "文字:\t\tあ (U+3042)\n\nSJIS:\t\t82A0\nJIS:\t\t2422\nEUC:\t\tA4A2\nLatin1:\t\t-\nUTF-16:\t\tU+3042\nUTF-8:\t\tE38182\nCESU-8:\t\tE38182"
    );
}

#[test]
fn the_code_box_for_ascii_and_a_surrogate_pair() {
    let message = code_box_message("a").unwrap();
    assert!(
        message.starts_with("文字:\t\ta (U+0061)\n\nSJIS:\t\t61\n"),
        "{message}"
    );
    assert!(message.contains("\nLatin1:\t\t61\n"), "{message}");
    // the code point shows a pair as one number, the UTF-16 line as its two units
    let message = code_box_message("😀").unwrap();
    assert!(message.starts_with("文字:\t\t😀 (U+1F600)\n"), "{message}");
    assert!(message.contains("\nSJIS:\t\t-\n"), "{message}");
    assert!(message.contains("\nUTF-8:\t\tF09F9880\n"), "{message}");
    assert!(message.ends_with("\nCESU-8:\t\tEDA0BDEDB880"), "{message}");
}

#[test]
fn the_code_box_needs_a_character() {
    assert_eq!(code_box_message(""), None);
}

// ---- the window ----

struct Fixture {
    document: Entity<SharedDocument>,
    workspace: Entity<Workspace>,
}

fn setup<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let shared = document.clone();
    let made: std::rc::Rc<std::cell::RefCell<Option<Entity<Workspace>>>> = Default::default();
    let slot = made.clone();
    let (_root, vcx) = cx.add_window_view(move |window, cx| {
        let workspace = cx.new(|cx| Workspace::new(shared, window, cx));
        *slot.borrow_mut() = Some(workspace.clone());
        Root::new(workspace, window, cx)
    });
    vcx.update(|window, _| window.activate_window());
    vcx.run_until_parked();
    let workspace = made.borrow().clone().unwrap();
    (
        Fixture {
            document,
            workspace,
        },
        vcx,
    )
}

impl Fixture {
    fn editor(&self, cx: &mut VisualTestContext) -> Entity<EditorView> {
        self.workspace.read_with(cx, |w, _| w.editor().clone())
    }

    fn double_click(&self, cx: &mut VisualTestContext, part: Part) {
        self.workspace
            .update_in(cx, |w, window, cx| w.status_double_click(part, window, cx));
        cx.run_until_parked();
    }

    fn right_click(&self, cx: &mut VisualTestContext, part: Part) {
        let at = gpui::point(gpui::px(300.), gpui::px(500.));
        self.workspace.update_in(cx, |w, window, cx| {
            w.status_right_click(part, at, window, cx)
        });
        cx.run_until_parked();
    }

    fn status(&self, cx: &mut VisualTestContext) -> StatusInfo {
        let editor = self.editor(cx);
        editor.read_with(cx, |v, app| v.status(app))
    }

    fn put_caret(&self, cx: &mut VisualTestContext, at: Pos) {
        let editor = self.editor(cx);
        editor.update(cx, |v, cx| {
            v.run_with(cx, |c| c.move_caret(at, false)).unwrap();
        });
    }
}

#[gpui::test]
fn the_window_with_the_parts_builds_and_paints(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "あいう\r\nabc");
    cx.run_until_parked();
    let status = f.status(cx);
    assert_eq!(status.text(Part::Position), "    1 行    1 桁");
    assert_eq!(status.text(Part::Eol), "CRLF");
    assert_eq!(status.text(Part::CharCode), "U+3042");
    assert_eq!(status.text(Part::Charset), "UTF-8");
    assert_eq!(status.text(Part::Mode), "挿入");
    assert_eq!(status.text(Part::Zoom), " 100 %");
}

#[gpui::test]
fn the_parts_follow_the_caret_the_selection_and_the_modes(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "あいう\nabc");
    f.put_caret(cx, Pos::new(1, 1));
    let status = f.status(cx);
    // Japanese text: a character is two columns
    assert_eq!(status.text(Part::Position), "    2 行    2 桁");
    assert_eq!(status.text(Part::CharCode), "U+0062");
    // the end of the first line shows the name of its line ending
    f.put_caret(cx, Pos::new(0, 9));
    assert_eq!(f.status(cx).text(Part::Position), "    1 行    7 桁");
    assert_eq!(f.status(cx).text(Part::CharCode), "LF");
    // a selection goes to the message
    let editor = f.editor(cx);
    editor.update(cx, |v, cx| {
        v.run_with(cx, |c| c.select_range(Pos::new(0, 0), Pos::new(0, 6)))
            .unwrap();
    });
    assert_eq!(
        f.status(cx).text(Part::Message),
        "2 chars (1 lines) selected."
    );
    // overwrite
    f.document.update(cx, |s, cx_| s.set_overwrite(true, cx_));
    assert_eq!(f.status(cx).text(Part::Mode), "上書");
}

#[gpui::test]
fn rec_is_lit_while_this_window_records(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a");
    assert!(!f.status(cx).recording);
    assert_eq!(f.status(cx).text(Part::Rec), "REC");
    f.double_click(cx, Part::Rec);
    assert!(f.status(cx).recording, "F_RECKEYMACRO started");
    f.double_click(cx, Part::Rec);
    assert!(!f.status(cx).recording, "and the second click stopped it");
}

#[gpui::test]
fn the_zoom_part_shows_the_font_zoom(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a");
    let editor = f.editor(cx);
    editor.update(cx, |v, cx| v.zoom_font(1, cx));
    let status = f.status(cx);
    assert!(status.zoom > 1.0, "{}", status.zoom);
    let percent = (status.zoom * 100.0).round();
    assert_eq!(status.text(Part::Zoom), format!("{percent:>4} %"));
}

#[gpui::test]
fn a_double_click_on_the_position_opens_the_jump_dialog(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb");
    let editor = f.editor(cx);
    assert_eq!(
        editor.read_with(cx, |v, app| v.search_panel_mode(app)),
        None
    );
    f.double_click(cx, Part::Position);
    assert_eq!(
        editor.read_with(cx, |v, app| v.search_panel_mode(app)),
        Some(PanelMode::Jump)
    );
}

#[gpui::test]
fn a_double_click_on_the_mode_toggles_overwrite(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    assert_eq!(f.status(cx).text(Part::Mode), "挿入");
    f.double_click(cx, Part::Mode);
    assert_eq!(f.status(cx).text(Part::Mode), "上書");
    f.double_click(cx, Part::Mode);
    assert_eq!(f.status(cx).text(Part::Mode), "挿入");
}

#[gpui::test]
fn a_double_click_on_the_encoding_opens_the_charset_dialog(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "あ");
    let editor = f.editor(cx);
    let open = |cx: &mut VisualTestContext| editor.read_with(cx, |v, _| v.save_as_dialog.is_some());
    assert!(!open(cx));
    f.double_click(cx, Part::Charset);
    assert!(open(cx));
    // Shift_JIS, and OK: the next save writes it
    cx.simulate_keystrokes("home enter");
    cx.run_until_parked();
    assert!(!open(cx));
    assert_eq!(f.status(cx).text(Part::Charset), "SJIS");
}

#[gpui::test]
fn a_double_click_on_the_code_shows_the_code_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "あ\n");
    assert!(!cx.has_pending_prompt());
    f.double_click(cx, Part::CharCode);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    // at the end of a line (the line ending is not a character) and of the file: nothing
    f.put_caret(cx, Pos::new(0, 3));
    f.double_click(cx, Part::CharCode);
    assert!(!cx.has_pending_prompt());
    f.put_caret(cx, Pos::new(1, 0));
    f.double_click(cx, Part::CharCode);
    assert!(!cx.has_pending_prompt());
}

#[gpui::test]
fn the_parts_without_a_click_do_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    let before = f.status(cx);
    for part in [Part::Message, Part::Eol, Part::Zoom] {
        f.double_click(cx, part);
    }
    assert_eq!(f.status(cx), before);
    let editor = f.editor(cx);
    assert!(!editor.read_with(cx, |v, _| v.custom_menu.is_open()));
}

#[gpui::test]
fn the_right_click_on_the_line_ending_opens_the_menu_that_sets_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\r\nb");
    let editor = f.editor(cx);
    let open = |cx: &mut VisualTestContext| editor.read_with(cx, |v, _| v.custom_menu.is_open());
    // another part has no menu
    f.right_click(cx, Part::Mode);
    assert!(!open(cx));
    f.right_click(cx, Part::Eol);
    assert!(open(cx));
    let at = editor.read_with(cx, |v, _| v.custom_menu.open_at().unwrap());
    assert_eq!(at, gpui::point(gpui::px(300.), gpui::px(496.)));
    // the first item is CRLF, the second LF: choose LF with the keys
    cx.simulate_keystrokes("down down enter");
    cx.run_until_parked();
    assert!(!open(cx));
    assert_eq!(f.document.read_with(cx, |s, _| s.new_line()), Eol::Lf);
    assert_eq!(f.status(cx).text(Part::Eol), "LF");
}

#[gpui::test]
fn the_menu_sets_cr_and_crlf_too(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a\nb");
    f.right_click(cx, Part::Eol);
    cx.simulate_keystrokes("down down down enter");
    cx.run_until_parked();
    assert_eq!(f.status(cx).text(Part::Eol), "CR");
    f.right_click(cx, Part::Eol);
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    assert_eq!(f.status(cx).text(Part::Eol), "CRLF");
    // the encoding is not touched
    assert_eq!(
        f.status(cx).text(Part::Charset),
        Encoding::Utf8.combo_name()
    );
}

// ---- the mouse, on the painted parts ----

fn center_of(cx: &mut VisualTestContext, part: Part) -> gpui::Point<gpui::Pixels> {
    let name: &'static str = Box::leak(selector(part).into_boxed_str());
    cx.debug_bounds(name)
        .unwrap_or_else(|| panic!("{part:?} is painted"))
        .center()
}

/// Presses and releases the left button the way the OS counts a double click.
fn double_click_at(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>) {
    for count in [1, 2] {
        cx.simulate_event(gpui::MouseDownEvent {
            position: at,
            modifiers: gpui::Modifiers::default(),
            button: gpui::MouseButton::Left,
            click_count: count,
            first_mouse: false,
        });
        cx.simulate_event(gpui::MouseUpEvent {
            position: at,
            modifiers: gpui::Modifiers::default(),
            button: gpui::MouseButton::Left,
            click_count: count,
        });
    }
    cx.run_until_parked();
}

fn right_click_at(cx: &mut VisualTestContext, at: gpui::Point<gpui::Pixels>) {
    cx.simulate_mouse_down(at, gpui::MouseButton::Right, gpui::Modifiers::default());
    cx.simulate_mouse_up(at, gpui::MouseButton::Right, gpui::Modifiers::default());
    cx.run_until_parked();
}

#[gpui::test]
fn every_part_is_painted_from_left_to_right(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "あ");
    let xs: Vec<_> = Part::ALL
        .iter()
        .map(|&p| f32::from(center_of(cx, p).x))
        .collect();
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
}

#[gpui::test]
fn the_mouse_double_click_runs_the_function_and_one_click_does_not(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    let at = center_of(cx, Part::Mode);
    cx.simulate_click(at, gpui::Modifiers::default());
    cx.run_until_parked();
    assert_eq!(f.status(cx).text(Part::Mode), "挿入", "one click");
    double_click_at(cx, at);
    assert_eq!(f.status(cx).text(Part::Mode), "上書");
    let at = center_of(cx, Part::Rec);
    double_click_at(cx, at);
    assert!(f.status(cx).recording);
}

#[gpui::test]
fn the_mouse_right_click_opens_the_menu_only_on_the_line_ending(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "a");
    let editor = f.editor(cx);
    let open = |cx: &mut VisualTestContext| editor.read_with(cx, |v, _| v.custom_menu.is_open());
    let at = center_of(cx, Part::Mode);
    right_click_at(cx, at);
    assert!(!open(cx));
    let at = center_of(cx, Part::Eol);
    right_click_at(cx, at);
    assert!(open(cx));
}
