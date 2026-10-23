//! The save dialog of Windows with the fields of 名前を付けて保存 (`IFileDialogCustomize`, issue #162),
//! driven with UI Automation: the groups 文字コードセット and 改行コード, the BOM check box that
//! follows the encoding (`Follow`, Sakura's `OnItemSelected`), and what the API returns after 保存
//! and after Cancel.
//!
//! The dialog is the real one of the OS, so these tests need a desktop, and they send keys to the
//! window in front. They run only on CI (`CI` is set) or with `MATSU_UIA_TESTS=1`; otherwise, and
//! without a desktop, they print why and pass.
#![cfg(windows)]

mod common;

use std::path::Path;
use std::sync::Mutex;

use common::{Element, Running, Uia};
use matsu_platform::dialog::{
    Chosen, Error, Extra, ExtraValue, Follow, Kind, Owner, Request, show_with_extras,
};

/// One dialog at a time on the desktop (the tests of this file run on several threads).
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

const ENCODINGS: [&str; 3] = ["UTF-8", "Shift_JIS", "UTF-16LE"];
const EOLS: [&str; 4] = ["変換なし", "CRLF", "LF", "CR"];
const PRESET_NAME: &str = "preset.txt";

type Answer = Result<Option<Chosen>, Error>;

fn request(dir: &Path, title: &str) -> Request {
    Request {
        kind: Kind::SaveFile,
        start: Some(dir.to_path_buf()),
        name: Some(PRESET_NAME.to_owned()),
        title: title.to_owned(),
        filters: Vec::new(),
        selected_filter: 0,
        default_extension: None,
        extras: vec![
            Extra::Combo {
                label: "文字コードセット(&C):".to_owned(),
                items: ENCODINGS.iter().map(|s| (*s).to_owned()).collect(),
                selected: 0,
            },
            Extra::Check {
                label: "&BOM".to_owned(),
                checked: true,
                enabled: true,
            },
            Extra::Combo {
                label: "改行コード(&E):".to_owned(),
                items: EOLS.iter().map(|s| (*s).to_owned()).collect(),
                selected: 1,
            },
        ],
        // Shift_JIS cannot have a BOM; the other two can. The application's rule is the same shape.
        follow: Some(Follow::new(0, 1, |item| match item {
            1 => (false, false),
            _ => (true, true),
        })),
    }
}

/// The controls of the dialog that the tests use.
struct Fields {
    window: Element,
    encoding: Element,
    bom: Element,
    eol: Element,
    name: Element,
}

fn find_fields(uia: &Uia, window: Element) -> Fields {
    let encoding = window.find(uia, "the encoding list", |e| {
        e.is_combo_box() && e.value() == ENCODINGS[0]
    });
    let eol = window.find(uia, "the line ending list", |e| {
        e.is_combo_box() && e.value() == EOLS[1]
    });
    let bom = window.find(uia, "the BOM check box", |e| {
        e.is_check_box() && e.name().replace('&', "").trim() == "BOM"
    });
    let name = window.find(uia, "the file name", |e| {
        // id 1001 (the edit of the file name; the address band has the id too, but is no edit).
        // Explorer shows the name without the extension it knows.
        e.is_edit() && e.automation_id() == "1001"
    });
    Fields {
        window,
        encoding,
        bom,
        eol,
        name,
    }
}

/// 保存 (IDOK, id "1") or Cancel (IDCANCEL, id "2"), found by the id of the control: the label
/// depends on the language of Windows.
fn button(uia: &Uia, window: &Element, id: &str) -> Element {
    window.find(uia, &format!("the button with the id {id}"), |e| {
        e.is_button() && e.automation_id() == id
    })
}

/// Shows the dialog the way the application does and hands the test its controls and the
/// answer. Skips (prints why) without a desktop.
fn with_dialog(title: &str, test: impl FnOnce(&Uia, Fields, Running<Answer>)) {
    let _guard = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let Some(uia) = Uia::connect() else {
        return;
    };
    let dir = tempfile::tempdir().expect("a folder to save into");
    let mut running = Running::start(show_with_extras(
        Owner::default(),
        request(dir.path(), title),
    ));
    let window = running.wait_window(&uia, title);
    let fields = find_fields(&uia, window);
    test(&uia, fields, running);
}

#[test]
fn the_dialog_shows_the_groups_and_the_items() {
    with_dialog("MATSU UIA save: groups", |uia, fields, running| {
        let names: Vec<String> = fields
            .window
            .descendants(uia)
            .iter()
            .map(Element::name)
            .collect();
        for label in ["文字コードセット", "改行コード"] {
            assert!(
                names.iter().any(|t| t.contains(label)),
                "no group {label}; the controls:\n{}",
                fields.window.dump(uia)
            );
        }
        // choosing an item proves that the list has it
        for (combo, items) in [(&fields.encoding, &ENCODINGS[..]), (&fields.eol, &EOLS[..])] {
            for item in items {
                combo.choose(uia, item);
                assert_eq!(combo.value(), *item);
            }
        }
        button(uia, &fields.window, "2").invoke();
        assert!(matches!(running.finish(), Ok(None)));
    });
}

#[test]
fn an_encoding_without_a_bom_turns_the_check_box_off_and_disables_it() {
    with_dialog("MATSU UIA save: follow", |uia, fields, running| {
        assert!(fields.bom.is_checked() && fields.bom.is_enabled());
        fields.encoding.choose(uia, "Shift_JIS");
        assert!(!fields.bom.is_checked(), "the BOM stays on for Shift_JIS");
        assert!(
            !fields.bom.is_enabled(),
            "the BOM stays enabled for Shift_JIS"
        );
        // and back: the rule gives the check box back
        fields.encoding.choose(uia, "UTF-8");
        assert!(fields.bom.is_checked() && fields.bom.is_enabled());
        button(uia, &fields.window, "2").invoke();
        assert!(matches!(running.finish(), Ok(None)));
    });
}

#[test]
fn save_returns_the_typed_name_and_the_chosen_values() {
    with_dialog("MATSU UIA save: save", |uia, fields, running| {
        fields.encoding.choose(uia, "UTF-16LE");
        fields.eol.choose(uia, "LF");
        fields.bom.toggle();
        assert!(!fields.bom.is_checked());
        let typed = fields.name.type_text("typed.txt");
        button(uia, &fields.window, "1").invoke();
        let chosen = running
            .finish()
            .expect("the dialog worked")
            .expect("the user saved");
        assert_eq!(chosen.paths.len(), 1);
        let name = chosen.paths[0].file_name().expect("a name");
        if typed {
            assert_eq!(name, "typed.txt");
        } else {
            // The keys did not reach the dialog (Windows ARM64 on GitHub's runners): the name
            // stays the one offered, and the rest of the answer is still checked.
            eprintln!("NOTE: the typed name could not be checked here; got {name:?}");
            assert_eq!(name, PRESET_NAME);
        }
        assert_eq!(
            chosen.extras,
            vec![
                ExtraValue::Combo(2),
                ExtraValue::Check(false),
                ExtraValue::Combo(2)
            ]
        );
    });
}

#[test]
fn cancel_returns_nothing() {
    with_dialog("MATSU UIA save: cancel", |uia, fields, running| {
        // a change before Cancel must not leak into the answer
        fields.eol.choose(uia, "CR");
        button(uia, &fields.window, "2").invoke();
        assert!(matches!(running.finish(), Ok(None)));
    });
}
