use gpui::{TestAppContext, VisualTestContext};
use matsu_commands::keymap::{Chord, Key, KeyMap, KeyMapOptions, Modifiers, MouseAction, Platform};
use matsu_commands::keymap::{Resolved, UserRow};
use matsu_config::Profile;

use super::bindings::{Bindings, load, modifier_prefix, store, why_unusable};
use super::*;
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::tests::{
    RestoreEditOptions, cancel, code_of, ok, open_window, setup, with_dialog,
};
use crate::common_settings::{self, Page};
use crate::file_ops::windows::editors;

const FILE_SAVE: u16 = 30103;

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn table(profile: &Profile) -> Bindings {
    Bindings::from_numbers(&load(profile))
}

fn index_of(table: &Bindings, key: Key) -> usize {
    table.rows.iter().position(|r| r.key == key).unwrap()
}

fn ctrl(key: Key) -> Chord {
    Chord::new(key, Modifiers::CTRL)
}

// ---- the table ----

#[test]
fn a_file_without_rows_gives_sakuras_table() {
    let t = table(&Profile::new());
    // Sakura's table: the mouse first, then the keys; Ctrl+S saves.
    assert!(matches!(t.rows[0].key, Key::Mouse(_)));
    let s = index_of(&t, Key::Letter('S'));
    assert_eq!(t.function(s, CTRL), FILE_SAVE);
    assert_eq!(t.function(s, 0), 0);
    assert_eq!(t.rows.len(), 100);
}

#[test]
fn the_rows_of_the_file_replace_the_rows_of_their_keys() {
    let t = table(&profile(
        "[KeyBind]\r\nKeyBind[000]=0053,0,0,30102,0,0,0,0,0,S\r\n",
    ));
    let s = index_of(&t, Key::Letter('S'));
    assert_eq!(t.function(s, CTRL), 30102);
    // The other keys stay Sakura's.
    let a = index_of(&t, Key::Letter('A'));
    assert_eq!(t.function(a, CTRL), code_of("F_SELECTALL"));
}

#[test]
fn the_list_of_numbers_round_trips() {
    let t = table(&Profile::new());
    let numbers = t.to_numbers();
    assert_eq!(numbers.len(), t.rows.len() * 9);
    assert_eq!(Bindings::from_numbers(&numbers), t);
    // Half a key, and a key that is not in Sakura's table, are left out.
    let mut broken = numbers[..18].to_vec();
    broken.extend([0x03, 0, 0, 0, 0, 0, 0, 0, 0]);
    broken.extend([1, 2, 3]);
    assert_eq!(Bindings::from_numbers(&broken).rows.len(), 2);
}

#[test]
fn assigning_and_releasing_use_the_checked_modifiers() {
    let mut t = table(&Profile::new());
    let j = index_of(&t, Key::Letter('J'));
    let mods = SHIFT | CTRL;
    assert!(t.set_function(j, mods, FILE_SAVE));
    assert_eq!(t.function(j, mods), FILE_SAVE);
    // Only that combination changed.
    assert_eq!(
        t.function(j, CTRL),
        table(&Profile::new()).function(j, CTRL)
    );
    assert!(t.set_function(j, mods, 0));
    assert_eq!(t.function(j, mods), 0);
    assert!(!t.set_function(9999, 0, 1));
}

#[test]
fn the_keys_of_a_function_are_listed_in_the_order_of_the_list_box() {
    let mut t = table(&Profile::new());
    let w = index_of(&t, Key::Letter('W'));
    let f = index_of(&t, Key::Function(2));
    t.set_function(w, SHIFT | CTRL | ALT, FILE_SAVE);
    t.set_function(f, 0, FILE_SAVE);
    let texts: Vec<_> = t
        .assigned_to(FILE_SAVE)
        .into_iter()
        .map(|a| a.text)
        .collect();
    // `LBS_SORT`: sorted; the modifiers are written as Shift+Ctrl+Alt+.
    assert_eq!(texts, ["Ctrl+S", "F2", "Shift+Ctrl+Alt+W"]);
    let found = &t.assigned_to(FILE_SAVE)[2];
    assert_eq!((found.key, found.mods), (w, SHIFT | CTRL | ALT));
    // Function 0 (none) has no keys.
    assert!(t.assigned_to(0).is_empty());
    assert_eq!(modifier_prefix(CTRL | ALT), "Ctrl+Alt+");
    assert_eq!(modifier_prefix(0), "");
}

#[test]
fn writing_the_table_keeps_the_rest_of_the_file() {
    let mut p = profile("[Other]\r\nx=1\r\n[Common]\r\nbA=1\r\n");
    let mut t = table(&p);
    let j = index_of(&t, Key::Letter('J'));
    t.set_function(j, CTRL, FILE_SAVE);
    assert!(store(&t.to_numbers(), &mut p).unwrap());
    // Sakura writes the whole table.
    assert_eq!(p.entries("KeyBind").count(), 100);
    assert_eq!(p.get("Other", "x"), Some("1"));
    assert_eq!(p.get("Common", "bA"), Some("1"));
    assert_eq!(table(&p), t);
    // The same table again writes nothing.
    assert!(!store(&t.to_numbers(), &mut p).unwrap());
    // The row of `J` is in the format of Sakura: hex code, 8 numbers, the name.
    let row = p
        .entries("KeyBind")
        .find(|(_, v)| v.starts_with("004a,"))
        .map(|(_, v)| v.to_owned())
        .unwrap();
    assert!(row.ends_with(",J"), "{row}");
    assert_eq!(row.split(',').count(), 10);
    // A file that has the table of Sakura gets nothing when the table is not changed.
    let mut fresh = Profile::new();
    assert!(!store(&table(&fresh).to_numbers(), &mut fresh).unwrap());
    assert_eq!(fresh.entries("KeyBind").count(), 0);
}

#[test]
fn a_row_that_did_not_change_keeps_its_text_and_shorter_tables_remove_the_leftovers() {
    // Row 000 names its functions; row 001 is a plugin command that MATSU Editor does not know.
    let mut p = profile(
        "[KeyBind]\r\nKeyBind[000]=0053,0,0,F_FILESAVE,0,0,0,0,0,S\r\n\
         KeyBind[001]=0051,0,0,Plugin/Cmd,0,0,0,0,0,Q\r\nKeyBind[200]=0041,0,0,0,0,0,0,0,0,A\r\n",
    );
    let mut t = table(&p);
    let j = index_of(&t, Key::Letter('J'));
    t.set_function(j, CTRL, FILE_SAVE);
    assert!(store(&t.to_numbers(), &mut p).unwrap());
    let texts: Vec<_> = p.entries("KeyBind").map(|(_, v)| v.to_owned()).collect();
    assert!(texts.contains(&"0053,0,0,F_FILESAVE,0,0,0,0,0,S".to_owned()));
    assert!(texts.contains(&"0051,0,0,Plugin/Cmd,0,0,0,0,0,Q".to_owned()));
    // An entry far past the table goes too: it would win over the row of its key.
    assert_eq!(p.get("KeyBind", "KeyBind[200]"), None);
    // The rows after the end of the table go.
    let mut p = profile("[KeyBind]\r\nKeyBind[100]=0041,0,0,0,0,0,0,0,0,A\r\n");
    let mut t = table(&p);
    t.set_function(0, 0, FILE_SAVE);
    assert!(store(&t.to_numbers(), &mut p).unwrap());
    assert_eq!(p.get("KeyBind", "KeyBind[100]"), None);
    assert!(p.get("KeyBind", "KeyBind[099]").is_some());
}

#[test]
fn the_keys_that_matsu_editor_cannot_use_say_why() {
    for action in [
        MouseAction::DoubleClick,
        MouseAction::MiddleClick,
        MouseAction::LeftSideClick,
        MouseAction::RightSideClick,
        MouseAction::TripleClick,
        MouseAction::QuadrupleClick,
    ] {
        assert!(why_unusable(Key::Mouse(action)).is_some(), "{action:?}");
    }
    assert!(why_unusable(Key::UnderscorePc98).is_some());
    // The wheel and the right click are looked up in the key map.
    for action in [
        MouseAction::WheelUp,
        MouseAction::WheelDown,
        MouseAction::WheelLeft,
        MouseAction::WheelRight,
        MouseAction::RightClick,
    ] {
        assert_eq!(why_unusable(Key::Mouse(action)), None, "{action:?}");
    }
    assert_eq!(why_unusable(Key::Letter('A')), None);
    assert_eq!(why_unusable(Key::Underscore), None);
}

#[test]
fn the_page_edits_sakuras_table_whatever_the_os() {
    // On macOS the rules of MATSU Editor are put over the rows of the file, not edited here.
    let map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
    let rows: Vec<UserRow> = map.rows();
    assert_eq!(table(&Profile::new()), Bindings::from_rows(&rows));
}

#[test]
fn the_page_has_every_control_of_sakura() {
    let list = controls(&items().unwrap());
    for id in [
        KIND,
        FUNC_SEL,
        KEY_SEL,
        SHIFT_ID,
        CTRL_ID,
        ALT_ID,
        "keybind_assign",
        "keybind_release",
        "keybind_import",
        "keybind_export",
    ] {
        assert!(list.iter().any(|c| c.id == id), "{id}");
    }
    assert!(
        list.iter()
            .any(|c| matches!(c.kind, Kind::Numbers { .. }) && c.id == ROWS)
    );
    // Only the table is written to the file.
    assert!(
        list.iter()
            .filter(|c| c.id != ROWS)
            .all(|c| c.key.is_empty())
    );
}

// ---- the dialog ----

fn press(cx: &mut TestAppContext, id: &'static str) {
    with_dialog(cx, |d, window, dcx| {
        let control = d
            .page_controls()
            .into_iter()
            .find(|c| c.id == id)
            .expect("the button");
        assert!(d.is_usable(&control), "{id} is usable");
        let Kind::Button { run } = control.kind else {
            panic!("{id} is not a button");
        };
        run(d, window, dcx);
    });
}

fn is_usable(cx: &mut TestAppContext, id: &'static str) -> bool {
    with_dialog(cx, |d, _, _| {
        let control = d.page_controls().into_iter().find(|c| c.id == id).unwrap();
        d.is_usable(&control)
    })
}

fn values<R>(cx: &mut TestAppContext, f: impl FnOnce(&Values) -> R) -> R {
    with_dialog(cx, |d, _, _| f(d.values()))
}

/// Chooses the key and the modifiers.
fn pick_key(cx: &mut TestAppContext, key: Key, mods: usize) {
    let at = values(cx, |v| index_of(&bindings_of(v), key));
    with_dialog(cx, |d, _, dcx| {
        d.select_key(at, dcx);
        d.set_bool(SHIFT_ID, mods & SHIFT != 0, dcx);
        d.set_bool(CTRL_ID, mods & CTRL != 0, dcx);
        d.set_bool(ALT_ID, mods & ALT != 0, dcx);
    });
}

/// Chooses the function in its kind, as a click in the lists does.
fn pick_function(cx: &mut TestAppContext, code: u16) {
    let (kind, at) = (0..16)
        .find_map(|kind| {
            codes_of_kind(kind)
                .iter()
                .position(|&c| c == code)
                .map(|at| (kind, at))
        })
        .unwrap();
    with_dialog(cx, |d, _, dcx| {
        d.set_int(KIND, kind, dcx);
        d.select_function(at, dcx);
    });
}

fn shown(cx: &mut TestAppContext) -> Vec<String> {
    values(cx, |v| {
        let code = u16::try_from(v.int(SHOWN_FUNC)).unwrap();
        bindings_of(v)
            .assigned_to(code)
            .into_iter()
            .map(|a| a.text)
            .collect()
    })
}

fn open_page(
    cx: &mut TestAppContext,
    text: &str,
) -> (crate::common_settings::tests::Ini, gpui::AnyWindowHandle) {
    let (ini, first) = setup(cx, text);
    cx.update(|app| common_settings::open(app, Some(Page::Keys), Some(first)));
    cx.run_until_parked();
    (ini, first)
}

#[gpui::test]
fn the_buttons_follow_the_selections_as_in_sakura(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (_ini, _first) = open_page(cx, "");
    // The first key is selected at the start, as `WM_INITDIALOG` does; no function is.
    assert_eq!(values(cx, |v| v.int(KEY_SEL)), 0);
    assert!(!is_usable(cx, "keybind_assign"));
    assert!(is_usable(cx, "keybind_release"));
    pick_function(cx, FILE_SAVE);
    assert!(is_usable(cx, "keybind_assign"));
    // A kind with no function selected again: 割当 cannot be used.
    with_dialog(cx, |d, _, dcx| d.set_int(KIND, 1, dcx));
    assert!(!is_usable(cx, "keybind_assign"));
    assert!(is_usable(cx, "keybind_import"));
    assert!(is_usable(cx, "keybind_export"));
    cancel(cx);
}

#[gpui::test]
fn both_lookups_follow_assign_and_release(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (_ini, _first) = open_page(cx, "");
    let names = |cx: &mut TestAppContext, code: u16| {
        cx.update(|app| crate::func_name::FuncNames::new().name(app, code))
    };
    // Function to keys: Ctrl+S is the key of 上書き保存.
    pick_function(cx, FILE_SAVE);
    assert_eq!(shown(cx), ["Ctrl+S"]);
    // Key to function: the function of Ctrl+S, and 未割当 for a key with none.
    pick_key(cx, Key::Letter('S'), CTRL);
    let function_at = |cx: &mut TestAppContext| {
        values(cx, |v| {
            let t = bindings_of(v);
            t.function(key_of(v).unwrap(), mods_of(v))
        })
    };
    assert_eq!(function_at(cx), FILE_SAVE);
    assert_eq!(names(cx, FILE_SAVE), "上書き保存");
    pick_key(cx, Key::Letter('J'), SHIFT | CTRL);
    assert_eq!(function_at(cx), 0);
    // 割当: Shift+Ctrl+J gets 上書き保存, and the list of keys is made again.
    press(cx, "keybind_assign");
    assert_eq!(function_at(cx), FILE_SAVE);
    assert_eq!(shown(cx), ["Ctrl+S", "Shift+Ctrl+J"]);
    // Choosing an entry of the list of keys goes to that key and its modifiers.
    with_dialog(cx, |d, _, dcx| {
        let found = bindings_of(d.values()).assigned_to(FILE_SAVE)[0].clone();
        d.select_assigned(0, &found, dcx);
    });
    assert_eq!(
        values(cx, |v| (mods_of(v), key_of(v))),
        (
            CTRL,
            values(cx, |v| Some(index_of(&bindings_of(v), Key::Letter('S'))))
        )
    );
    // 解除 with the function still selected: the keys are listed again.
    press(cx, "keybind_release");
    assert_eq!(shown(cx), ["Shift+Ctrl+J"]);
    assert_eq!(function_at(cx), 0);
    // 解除 with no function selected empties the list (the selection message goes with `-1`).
    with_dialog(cx, |d, _, dcx| d.set_int(KIND, 1, dcx));
    pick_key(cx, Key::Letter('J'), SHIFT | CTRL);
    press(cx, "keybind_release");
    assert!(shown(cx).is_empty());
    cancel(cx);
}

#[gpui::test]
fn the_kind_empties_the_function_list_but_not_the_keys(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (_ini, _first) = open_page(cx, "");
    pick_function(cx, FILE_SAVE);
    assert_eq!(shown(cx), ["Ctrl+S"]);
    with_dialog(cx, |d, _, dcx| d.set_int(KIND, 2, dcx));
    assert_eq!(values(cx, function_of), None);
    assert_eq!(shown(cx), ["Ctrl+S"]);
    cancel(cx);
}

#[gpui::test]
fn the_names_of_the_custom_menus_are_the_names_set_for_them(cx: &mut TestAppContext) {
    let (_ini, _first) = setup(cx, "[CustMenu]\r\nszCMN[01]=My menu\r\n");
    let name = |cx: &mut TestAppContext, code: u16| {
        cx.update(|app| crate::func_name::FuncNames::new().name(app, code))
    };
    assert_eq!(name(cx, 31580), "右クリックメニュー");
    assert_eq!(name(cx, 31501), "My menu");
    assert_eq!(name(cx, 31502), "メニュー2");
    assert_eq!(name(cx, 31524), "タブメニュー");
    assert_eq!(name(cx, 31600), "マクロ 0 (未登録)");
    assert_eq!(name(cx, 64999), "-- 不明 --");
}

#[gpui::test]
fn ok_writes_the_rows_and_every_window_uses_the_new_key_at_once(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (ini, first) = open_page(cx, "[Other]\r\nx=1\r\n");
    let second = open_window(cx);
    // Move 上書き保存 from Ctrl+S to Shift+Ctrl+J.
    pick_function(cx, FILE_SAVE);
    pick_key(cx, Key::Letter('S'), CTRL);
    press(cx, "keybind_release");
    pick_key(cx, Key::Letter('J'), SHIFT | CTRL);
    press(cx, "keybind_assign");
    // Nothing is written before OK.
    assert_eq!(ini.profile().entries("KeyBind").count(), 0);
    let old = ctrl(Key::Letter('S'));
    let new = Chord::new(Key::Letter('J'), Modifiers::CTRL.with_shift());
    let resolve = |cx: &mut TestAppContext, chord: Chord| -> Vec<Resolved> {
        cx.update(editors)
            .into_iter()
            .map(|(_, view)| view.read_with(cx, |v, _| v.keymap.resolve(chord)))
            .collect()
    };
    assert_eq!(resolve(cx, old), [Resolved::Code(FILE_SAVE); 2]);
    ok(cx);

    let profile = ini.profile();
    assert_eq!(profile.get("Other", "x"), Some("1"));
    assert_eq!(profile.entries("KeyBind").count(), 100);
    assert_eq!(
        profile
            .get("KeyBind", "KeyBind[000]")
            .map(|v| v.split(',').count()),
        Some(10)
    );
    // Both windows, the one open before OK and the one made before it too.
    assert_eq!(resolve(cx, new), [Resolved::Code(FILE_SAVE); 2]);
    assert_eq!(resolve(cx, old), [Resolved::Unbound; 2]);
    for handle in [first, second] {
        let _ = VisualTestContext::from_window(handle, cx);
    }
    // The menus read this key map.
    cx.update(|app| {
        crate::app::with_key_map(app, |map| {
            assert_eq!(map.shortcut_for(FILE_SAVE), Some(new));
        });
    });
    // A window opened after OK uses the new key too (it reads the file).
    let third = open_window(cx);
    assert!(cx.update(editors).iter().any(|(h, _)| *h == third));
    assert_eq!(resolve(cx, new), [Resolved::Code(FILE_SAVE); 3]);
    // The file read again gives the table that was edited.
    let user = UserSettings::load_from(std::slice::from_ref(&ini.path));
    let map = user.key_map(Platform::Other);
    assert_eq!(map.resolve(new), Resolved::Code(FILE_SAVE));
    assert_eq!(map.resolve(old), Resolved::Unbound);
}

#[gpui::test]
fn ok_without_a_change_in_the_table_leaves_the_file_and_the_windows_alone(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (ini, _first) = open_page(
        cx,
        "[KeyBind]\r\nKeyBind[000]=0053,0,0,F_FILESAVE,0,0,0,0,0,S\r\n",
    );
    let rows_before = ini.profile().entries("KeyBind").count();
    // Selecting around changes no row.
    pick_function(cx, FILE_SAVE);
    pick_key(cx, Key::Letter('A'), ALT);
    ok(cx);
    // The table was not written: the one row the file had is all it has.
    assert_eq!(rows_before, 1);
    assert_eq!(ini.profile().entries("KeyBind").count(), 1);
    assert_eq!(
        ini.profile().get("KeyBind", "KeyBind[000]"),
        Some("0053,0,0,F_FILESAVE,0,0,0,0,0,S")
    );
}

#[gpui::test]
fn cancel_changes_nothing(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (ini, _first) = open_page(cx, "");
    let bytes = ini.bytes();
    let resolve = |cx: &mut TestAppContext| -> Vec<Resolved> {
        cx.update(editors)
            .into_iter()
            .map(|(_, view)| view.read_with(cx, |v, _| v.keymap.resolve(ctrl(Key::Letter('J')))))
            .collect()
    };
    let before = resolve(cx);
    pick_function(cx, FILE_SAVE);
    pick_key(cx, Key::Letter('J'), CTRL);
    press(cx, "keybind_assign");
    cancel(cx);
    assert_eq!(ini.bytes(), bytes);
    assert_eq!(resolve(cx), before);
    assert_ne!(before, [Resolved::Code(FILE_SAVE)]);
}

#[gpui::test]
fn the_page_says_when_a_key_cannot_be_used(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (_ini, first) = open_page(cx, "");
    let note = |cx: &mut TestAppContext| {
        let dialog_window = cx
            .update(|app| common_settings::dialog_window(app))
            .unwrap();
        let cx = &mut VisualTestContext::from_window(dialog_window, cx);
        cx.run_until_parked();
        cx.debug_bounds("keybind-unusable").is_some()
    };
    pick_key(cx, Key::Letter('A'), 0);
    assert!(!note(cx));
    pick_key(cx, Key::Mouse(MouseAction::TripleClick), 0);
    assert!(note(cx));
    pick_key(cx, Key::Mouse(MouseAction::WheelUp), 0);
    assert!(!note(cx));
    // The assignment is still allowed (Sakura reads the file).
    pick_function(cx, FILE_SAVE);
    pick_key(cx, Key::UnderscorePc98, 0);
    assert!(note(cx));
    assert!(is_usable(cx, "keybind_assign"));
    cancel(cx);
    let _ = first;
}

#[test]
fn a_cell_that_did_not_change_keeps_its_token_in_a_row_that_did() {
    // Ctrl+Q is a plugin command, Shift+Q a name MATSU Editor does not know; Alt+Q is assigned.
    let mut p = profile(
        "[KeyBind]\r\nKeyBind[000]=0051,0,0,0,0,0,0,0,0,Q\r\n\
         KeyBind[001]=0051,0,Nope,Plugin/Cmd,0,0,0,0,0,Q\r\n",
    );
    let mut t = table(&p);
    let q = index_of(&t, Key::Letter('Q'));
    assert!(t.set_function(q, ALT, FILE_SAVE));
    assert!(store(&t.to_numbers(), &mut p).unwrap());
    let row = p
        .entries("KeyBind")
        .map(|(_, v)| v.to_owned())
        .find(|v| v.starts_with("0051,"))
        .unwrap();
    assert_eq!(row, format!("0051,0,Nope,Plugin/Cmd,0,{FILE_SAVE},0,0,0,Q"));
    assert_eq!(table(&p).function(q, ALT), FILE_SAVE);
}

#[test]
fn an_entry_the_file_has_twice_does_not_win_over_the_table_written() {
    let mut p = profile(
        "[KeyBind]\r\nKeyBind[000]=0051,0,0,0,0,0,0,0,0,Q\r\nKeyBind[000]=0051,0,0,1,0,0,0,0,0,Q\r\n",
    );
    let mut t = table(&p);
    let q = index_of(&t, Key::Letter('Q'));
    t.set_function(q, CTRL, FILE_SAVE);
    assert!(store(&t.to_numbers(), &mut p).unwrap());
    assert_eq!(table(&p), t);
}

// ---- the .key files ----

fn key_file_text(path: &std::path::Path) -> String {
    String::from_utf8_lossy(&std::fs::read(path).unwrap()).into_owned()
}

#[gpui::test]
fn a_key_file_has_macro_names_and_numbers_and_reads_back(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (ini, _first) = open_page(cx, "");
    let file = ini.dir.0.join("keys.key");
    assert!(with_dialog(cx, |d, window, dcx| d.export_key_file(&file, window, dcx)));
    let text = key_file_text(&file);
    assert!(text.contains("[Info]"), "{text}");
    assert!(
        text.contains("KEYBIND_VERSION=SakuraKeyBind_Ver4"),
        "{text}"
    );
    assert!(text.contains("KEYBIND_COUNT=100"), "{text}");
    // Ctrl+S is F_FILESAVE, which has the macro name FileSave; Alt+Shift+S (31142) has none.
    let s_row = text
        .lines()
        .find(|l| l.contains("=0053,"))
        .expect("the row of S");
    assert!(s_row.contains(",FileSave,"), "{s_row}");
    assert!(s_row.contains(",31142,"), "{s_row}");
    // Take Ctrl+S away, then read the file: the key is Ctrl+S again.
    pick_key(cx, Key::Letter('S'), CTRL);
    press(cx, "keybind_release");
    let s = values(cx, |v| index_of(&bindings_of(v), Key::Letter('S')));
    assert_eq!(values(cx, |v| bindings_of(v).function(s, CTRL)), 0);
    assert!(with_dialog(cx, |d, window, dcx| d.import_key_file(&file, window, dcx)));
    assert_eq!(values(cx, |v| bindings_of(v).function(s, CTRL)), FILE_SAVE);
    cancel(cx);
}

#[gpui::test]
fn a_key_file_with_numbers_or_sakuras_names_is_read(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (ini, _first) = open_page(cx, "");
    let file = ini.dir.0.join("mixed.key");
    // Ctrl+S: a name; Ctrl+J: a number; Ctrl+Q: S_ with a name; the count is the number of rows.
    std::fs::write(
        &file,
        "[Info]\r\nKEYBIND_VERSION=SakuraKeyBind_Ver3\r\nKEYBIND_COUNT=3\r\n[KeyBind]\r\n\
         KeyBind[000]=0053,0,0,SearchNext,0,0,0,0,0,S\r\n\
         KeyBind[001]=004a,0,0,30102,0,0,0,0,0,J\r\n\
         KeyBind[002]=0051,0,0,S_SearchPrev,0,0,0,0,0,Q\r\n",
    )
    .unwrap();
    assert!(with_dialog(cx, |d, window, dcx| d.import_key_file(&file, window, dcx)));
    let function = |cx: &mut TestAppContext, key: Key| {
        values(cx, |v| {
            let t = bindings_of(v);
            t.function(index_of(&t, key), CTRL)
        })
    };
    assert_eq!(function(cx, Key::Letter('S')), 30902);
    assert_eq!(function(cx, Key::Letter('J')), 30102);
    assert_eq!(function(cx, Key::Letter('Q')), 30903);
    // A key the file does not have stays.
    assert_eq!(function(cx, Key::Letter('A')), code_of("F_SELECTALL"));
    cancel(cx);
}

#[gpui::test]
fn a_file_that_is_not_a_key_file_changes_nothing(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (ini, _first) = open_page(cx, "");
    let before = values(cx, |v| v.numbers(ROWS).to_vec());
    let wrong_version = ini.dir.0.join("v.key");
    std::fs::write(
        &wrong_version,
        "[Info]\r\nKEYBIND_VERSION=Other\r\n[KeyBind]\r\nKeyBind[000]=0053,0,0,30102,0,0,0,0,0,S\r\n",
    )
    .unwrap();
    let too_many = ini.dir.0.join("c.key");
    std::fs::write(
        &too_many,
        "[Info]\r\nKEYBIND_VERSION=SakuraKeyBind_Ver4\r\nKEYBIND_COUNT=257\r\n[KeyBind]\r\nKeyBind[000]=0053,0,0,30102,0,0,0,0,0,S\r\n",
    )
    .unwrap();
    let missing = ini.dir.0.join("missing.key");
    for file in [&wrong_version, &too_many, &missing] {
        assert!(!with_dialog(cx, |d, window, dcx| d.import_key_file(file, window, dcx)));
    }
    assert_eq!(values(cx, |v| v.numbers(ROWS).to_vec()), before);
    cancel(cx);
}

#[test]
fn reading_a_key_file_overwrites_whole_rows_and_skips_unknown_keys() {
    let base = table(&Profile::new());
    let file = profile(
        "[Info]\r\nKEYBIND_VERSION=SakuraKeyBind_Ver4\r\n[KeyBind]\r\n\
         KeyBind[000]=0053,0,0,0,0,0,0,0,0,S\r\n\
         KeyBind[001]=7777,0,0,30102,0,0,0,0,0,?\r\n",
    );
    let read = transfer::read_key_file(&file, &base).unwrap();
    let s = index_of(&read, Key::Letter('S'));
    // The row of S in the file has no functions: Ctrl+S is released, not kept.
    assert_eq!(read.function(s, CTRL), 0);
    assert_eq!(read.rows.len(), base.rows.len());
}

#[test]
fn the_exported_file_reads_back_to_the_same_table() {
    let base = table(&Profile::new());
    let file = transfer::key_file(&base);
    let bytes = file.to_sakura_bytes(Some("SakuraKeyBind_Ver4"));
    let again = Profile::from_bytes(&bytes);
    let mut other = base.clone();
    for row in &mut other.rows {
        row.functions = [0; 8];
    }
    assert_eq!(transfer::read_key_file(&again, &other).unwrap(), base);
}

#[gpui::test]
fn the_dialogs_list_the_key_files_and_remember_the_folder(cx: &mut TestAppContext) {
    let _restore = RestoreEditOptions::new();
    let (ini, _first) = open_page(cx, "");
    crate::file_dialog::tests::asked();
    press(cx, "keybind_import");
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    let names: Vec<_> = request.filters.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        ["ユーザー指定", "テキストファイル", "すべてのファイル"]
    );
    assert_eq!(request.filters[0].patterns, ["*.key"]);
    // Nothing was chosen before: the folder of the settings file.
    assert_eq!(request.start.as_deref(), Some(ini.dir.0.as_path()));
    // Export to a name in another folder; the folder is the import folder from then on.
    let other = tempfile::tempdir().unwrap();
    let target = other.path().join("my.key");
    press(cx, "keybind_export");
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    assert_eq!(request.filters[0].patterns, ["*.key"]);
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert!(target.exists());
    let remembered = ini
        .profile()
        .get("Folders", "szIMPORTFOLDER")
        .map(str::to_owned);
    let want = format!("{}{}", other.path().display(), std::path::MAIN_SEPARATOR);
    assert_eq!(remembered.as_deref(), Some(want.as_str()));
    press(cx, "keybind_import");
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    assert_eq!(request.start.as_deref(), Some(other.path()));
    cancel(cx);
}

#[test]
fn a_plugin_command_assigned_in_the_page_is_written_by_its_name() {
    use matsu_commands::plugin_commands as pc;
    pc::set_slots(vec!["keys.test".to_owned()]);
    pc::add_command(pc::CommandInfo {
        code: 20001,
        slot: 0,
        plug: 1,
        plugin_id: "keys.test".to_owned(),
        plugin_name: "Keys test".to_owned(),
        label: "Do it".to_owned(),
        handler: "do.js".to_owned(),
        icon: String::new(),
        added: false,
    });
    // A binding in the file by name is read as the command, and kept as the name when nothing changes.
    let text = "[KeyBind]\r\nKeyBind[000]=0041,0,0,keys.test/01,0,0,0,0,0,A\r\n";
    let mut p = profile(text);
    let mut t = table(&p);
    let a = index_of(&t, Key::Letter('A'));
    assert_eq!(t.function(a, super::bindings::CTRL), 20001);
    assert!(!store(&t.to_numbers(), &mut p).unwrap());
    // The user assigns the command to Shift+Ctrl+A too: the changed cell is written by name.
    let both = super::bindings::CTRL | super::bindings::SHIFT;
    assert!(t.set_function(a, both, 20001));
    assert!(store(&t.to_numbers(), &mut p).unwrap());
    let row = p
        .entries("KeyBind")
        .map(|(_, v)| v.to_owned())
        .find(|v| v.starts_with("0041,"))
        .unwrap();
    assert_eq!(row, "0041,0,0,keys.test/01,keys.test/01,0,0,0,0,A");
    pc::set_commands(Vec::new());
    pc::set_slots(Vec::new());
}
