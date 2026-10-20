//! The font dialog of Windows (`ChooseFontW`, issue #150) driven with UI Automation: the face and
//! the size are changed, and OK returns that `FontSetting`; Cancel returns nothing.
//!
//! The dialog is the real one of the OS, so these tests need a desktop. Without one they print why
//! and pass.
#![cfg(windows)]

mod common;

use std::sync::Mutex;

use common::{Element, Running, Uia};
use ume_config::settings::FontSetting;
use ume_platform::font_dialog::{Error, Owner, Request, choose};

/// One dialog at a time on the desktop (the tests of this file run on several threads).
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// A face every Windows has; the dialog starts with it at 10 pt.
const START_FACE: &str = "Courier New";
/// `cmb1` of `dlgs.h`: the control id is the automation id of a control of a classic dialog.
const FACE_ID: &str = "1136";
const NEW_FACE: &str = "Arial";

type Answer = Result<Option<FontSetting>, Error>;

fn initial() -> FontSetting {
    let mut font = FontSetting::default();
    font.log_font.face_name = START_FACE.to_owned();
    // -13 pixels is 10 pt at 96 dpi; the dialog starts from the point size
    font.log_font.height = -13;
    font.point_size = 100;
    font
}

/// The face list and the size list (`cmb3`, 1138); OK is IDOK (1) and Cancel IDCANCEL (2).
struct Fields {
    window: Element,
    face: Element,
    size: Element,
}

fn find_fields(uia: &Uia, window: Element) -> Fields {
    let face = window.find(uia, "the face list", |e| {
        e.is_combo_box() && e.automation_id() == FACE_ID
    });
    let size = window.find(uia, "the size list", |e| {
        e.is_combo_box() && e.automation_id() == "1138"
    });
    Fields { window, face, size }
}

fn button(uia: &Uia, window: &Element, id: &str) -> Element {
    window.find(uia, &format!("the button with the id {id}"), |e| {
        e.is_button() && e.automation_id() == id
    })
}

/// Chooses `NEW_FACE`. The list items of this classic dialog do not always take the selection, so
/// the text of the combo box is set as well when it did not change.
fn choose_face(uia: &Uia, face: &Element) {
    face.choose(uia, NEW_FACE);
    if face.value() != NEW_FACE {
        face.set_value(NEW_FACE);
    }
    assert_eq!(face.value(), NEW_FACE);
}

fn with_dialog(test: impl FnOnce(&Uia, Fields, Running<Answer>)) {
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let Some(uia) = Uia::connect() else {
        eprintln!("SKIPPED: this session has no desktop that UI Automation can see");
        return;
    };
    let mut running = Running::start(choose(
        Owner::default(),
        Request {
            initial: initial(),
            fixed_pitch_only: false,
        },
    ));
    // ChooseFontW has no title to give (the title is "Font" or "フォント" by the language of
    // Windows): the dialog is the window of this process that has the face list.
    let window = running.wait_window_by(&uia, "the font dialog", |uia, w| {
        w.descendants(uia)
            .iter()
            .any(|e| e.is_combo_box() && e.automation_id() == FACE_ID)
    });
    let fields = find_fields(&uia, window);
    test(&uia, fields, running);
}

#[test]
fn ok_returns_the_face_and_the_size() {
    with_dialog(|uia, fields, running| {
        assert_eq!(fields.face.value(), START_FACE);
        choose_face(uia, &fields.face);
        fields.size.set_value("14");
        button(uia, &fields.window, "1").invoke();
        let font = running
            .finish()
            .expect("the dialog worked")
            .expect("the user chose");
        assert_eq!(font.log_font.face_name, NEW_FACE);
        assert_eq!(font.point_size, 140);
    });
}

#[test]
fn ok_without_changes_returns_the_initial_font() {
    with_dialog(|uia, fields, running| {
        button(uia, &fields.window, "1").invoke();
        let font = running
            .finish()
            .expect("the dialog worked")
            .expect("the user chose");
        assert_eq!(font.log_font.face_name, START_FACE);
        assert_eq!(font.point_size, 100);
    });
}

#[test]
fn cancel_returns_nothing() {
    with_dialog(|uia, fields, running| {
        choose_face(uia, &fields.face);
        button(uia, &fields.window, "2").invoke();
        assert!(matches!(running.finish(), Ok(None)));
    });
}
