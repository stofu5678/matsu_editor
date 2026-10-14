//! Tests of the key map. Expectations come from Sakura's default table
//! (`sakura_core/func/CKeyBind.cpp`, `KeyDataInit`; see `crate::sakura::KEY_BINDINGS`).

use super::*;
use crate::sakura::KEY_BINDINGS;

fn chord(s: &str) -> Chord {
    Chord::parse(s).unwrap_or_else(|| panic!("bad chord {s:?}"))
}

fn windows() -> KeyMap {
    KeyMap::new(KeyMapOptions::for_platform(Platform::Other))
}

fn mac_with(f: impl FnOnce(&mut KeyMapOptions)) -> KeyMap {
    let mut options = KeyMapOptions::for_platform(Platform::MacOs);
    f(&mut options);
    KeyMap::new(options)
}

fn mac() -> KeyMap {
    mac_with(|_| {})
}

fn code(map: &KeyMap, s: &str) -> Resolved {
    map.resolve(chord(s))
}

fn id(lookup: Lookup) -> &'static str {
    match lookup {
        Lookup::Command(reg) => reg.id,
        other => panic!("not a command: {other:?}"),
    }
}

#[test]
fn spot_checks_of_sakura_s_table() {
    let map = windows();
    // CKeyBind.cpp: Ctrl+S file save, Ctrl+Z undo, Ctrl+Y redo, Ctrl+F search dialog.
    assert_eq!(code(&map, "Ctrl+S"), Resolved::Code(30103));
    assert_eq!(code(&map, "Ctrl+Z"), Resolved::Code(30210));
    assert_eq!(code(&map, "Ctrl+Y"), Resolved::Code(30211));
    assert_eq!(code(&map, "Ctrl+F"), Resolved::Code(30901));
    // F3 search next, Shift+F3 previous, Ctrl+F3 clear marks, Shift+Ctrl+F3 jump to start.
    assert_eq!(code(&map, "F3"), Resolved::Code(30902));
    assert_eq!(code(&map, "Shift+F3"), Resolved::Code(30903));
    assert_eq!(code(&map, "Ctrl+F3"), Resolved::Code(30905));
    assert_eq!(code(&map, "Shift+Ctrl+F3"), Resolved::Code(30909));
    // Alt rows: Alt+D sort descending, Alt+; insert date, Alt+1 custom menu 1.
    assert_eq!(code(&map, "Alt+D"), Resolved::Code(30283));
    assert_eq!(code(&map, "Alt+;"), Resolved::Code(30790));
    assert_eq!(code(&map, "Alt+1"), Resolved::Code(31501));
    assert_eq!(code(&map, "Ctrl+Alt+Shift+Up"), Resolved::Code(31350));
    // Plain keys.
    assert_eq!(code(&map, "Home"), Resolved::Code(30332));
    assert_eq!(code(&map, "Shift+Del"), Resolved::Code(30601));
    assert_eq!(code(&map, "Ctrl+Ins"), Resolved::Code(30602));
    assert_eq!(code(&map, "Ctrl+\\"), Resolved::Code(30620));
    assert_eq!(code(&map, "Ctrl+@(英語`)"), Resolved::Code(30610));
}

#[test]
fn spot_checks_of_the_mouse_rows() {
    let map = windows();
    let mouse = |m, mods| Chord::new(Key::Mouse(m), mods);
    // Double click selects the word under every modifier.
    for i in 0..8 {
        let c = mouse(MouseAction::DoubleClick, Modifiers::from_sakura_index(i));
        assert_eq!(map.resolve(c), Resolved::Code(30400));
    }
    // Ctrl+wheel changes the font size.
    assert_eq!(
        map.resolve(mouse(MouseAction::WheelUp, Modifiers::CTRL)),
        Resolved::Code(31122)
    );
    assert_eq!(
        map.resolve(mouse(MouseAction::WheelDown, Modifiers::CTRL)),
        Resolved::Code(31123)
    );
    assert_eq!(
        map.resolve(mouse(MouseAction::WheelUp, Modifiers::NONE)),
        Resolved::Code(30385)
    );
    assert_eq!(
        map.resolve(mouse(MouseAction::MiddleClick, Modifiers::NONE)),
        Resolved::Code(30384)
    );
    assert_eq!(
        map.resolve(mouse(MouseAction::MiddleClick, Modifiers::ALT)),
        Resolved::Unbound
    );
}

#[test]
fn every_cell_of_the_table_resolves_to_itself() {
    let map = windows();
    for b in KEY_BINDINGS {
        let key = Key::from_code(b.key_code).unwrap();
        for (i, &f) in b.functions.iter().enumerate() {
            let c = Chord::new(key, Modifiers::from_sakura_index(i));
            if f != 0 {
                assert_eq!(map.resolve(c), Resolved::Code(f), "{c}");
            }
        }
    }
}

#[test]
fn empty_cells_are_unbound_except_for_sakura_s_defaults() {
    let map = windows();
    assert_eq!(code(&map, "Ctrl+Shift+Q"), Resolved::Unbound);
    assert_eq!(code(&map, "F13"), Resolved::Unbound);
    // `GetDefFuncCode`: Ctrl+F4 closes the file, Alt+F4 the window, though the cells are empty.
    assert_eq!(code(&map, "Ctrl+F4"), Resolved::Code(30105));
    assert_eq!(code(&map, "Alt+F4"), Resolved::Code(31320));
    // The meta key (Windows/Super) is not a Sakura modifier.
    assert_eq!(code(&map, "Cmd+S"), Resolved::Unbound);
}

#[test]
fn lookup_finds_commands_and_reports_the_ones_not_implemented() {
    let map = windows();
    assert_eq!(id(map.lookup(chord("Ctrl+Z"))), "undo");
    assert_eq!(id(map.lookup(chord("Home"))), "go_line_top");
    assert_eq!(id(map.lookup(chord("Shift+Down"))), "cursor_down_sel");
}

#[test]
fn unimplemented_functions_stay_in_the_map() {
    let map = windows();
    match map.lookup(chord("Ctrl+S")) {
        Lookup::NotImplemented { code, function } => {
            assert_eq!(code, 30103);
            assert_eq!(function.unwrap().name, "F_FILESAVE");
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(map.lookup(chord("Ctrl+Shift+Q")), Lookup::Unbound));
    // A number that is no Sakura function (from a user's file) is still reported.
    let mut map = windows();
    map.apply_user_rows(&[UserRow {
        key: Key::Letter('Q'),
        functions: [0, 0, 29999, 0, 0, 0, 0, 0],
    }]);
    assert!(matches!(
        map.lookup(chord("Ctrl+Q")),
        Lookup::NotImplemented {
            code: 29999,
            function: None
        }
    ));
}

#[test]
fn macos_reads_ctrl_as_cmd_and_alt_as_option() {
    let map = mac();
    assert_eq!(code(&map, "Cmd+S"), Resolved::Code(30103));
    assert_eq!(code(&map, "Cmd+Z"), Resolved::Code(30210));
    assert_eq!(code(&map, "Cmd+Y"), Resolved::Code(30211));
    assert_eq!(code(&map, "Cmd+Shift+S"), Resolved::Code(30104));
    assert_eq!(code(&map, "Option+D"), Resolved::Code(30283));
    assert_eq!(code(&map, "Cmd+Option+L"), Resolved::Code(30800));
    assert_eq!(code(&map, "F3"), Resolved::Code(30902));
    // Ctrl alone is not Sakura's Ctrl.
    assert_eq!(code(&map, "Control+S"), Resolved::Unbound);
    assert_eq!(code(&map, "Control+Cmd+S"), Resolved::Unbound);
    // The mouse: Option+drag is Alt+drag; Cmd+wheel is Ctrl+wheel.
    let wheel = Chord::new(Key::Mouse(MouseAction::WheelUp), Modifiers::META);
    assert_eq!(map.resolve(wheel), Resolved::Code(31122));
}

#[test]
fn macos_can_keep_ctrl() {
    let map = mac_with(|o| o.ctrl_as_cmd = false);
    assert_eq!(code(&map, "Control+S"), Resolved::Code(30103));
    assert_eq!(code(&map, "Control+A"), Resolved::Code(30401));
    assert_eq!(code(&map, "Cmd+S"), Resolved::Unbound);
    assert_eq!(code(&map, "Option+D"), Resolved::Code(30283));
    // The standard Mac keys on Cmd still work.
    assert_eq!(code(&map, "Cmd+W"), Resolved::Code(31320));
    assert_eq!(code(&map, "Cmd+Left"), Resolved::Code(30332));
    // Control+K is Sakura's Ctrl+K here, not the Emacs key.
    assert_eq!(code(&map, "Control+K"), Resolved::Code(30241));
}

#[test]
fn macos_standard_keys_win_over_the_clashes() {
    let map = mac();
    // Sakura: Ctrl+W selects the word, Ctrl+Left moves by word, Ctrl+H centers the line.
    assert_eq!(code(&map, "Cmd+W"), Resolved::Code(31320));
    assert_eq!(id(map.lookup(chord("Cmd+Left"))), "go_line_top");
    assert_eq!(id(map.lookup(chord("Cmd+Shift+Right"))), "go_line_end_sel");
    assert_eq!(id(map.lookup(chord("Cmd+Up"))), "go_file_top");
    assert_eq!(id(map.lookup(chord("Cmd+Shift+Down"))), "go_file_end_sel");
    assert_eq!(code(&map, "Cmd+Shift+Z"), Resolved::Code(30211));
    assert_eq!(code(&map, "Cmd+G"), Resolved::Code(30902));
    assert_eq!(code(&map, "Cmd+Shift+F"), Resolved::Code(30910));
    assert_eq!(code(&map, "Cmd+,"), Resolved::Code(31112));
    assert_eq!(code(&map, "Option+Backspace"), Resolved::Code(30230));
    assert_eq!(code(&map, "Option+Del"), Resolved::Code(30231));
    // Hide and minimize are left to the OS.
    assert_eq!(code(&map, "Cmd+H"), Resolved::PassThrough);
    assert_eq!(code(&map, "Cmd+M"), Resolved::PassThrough);
    assert!(matches!(map.lookup(chord("Cmd+H")), Lookup::PassThrough));
    // Without the setting, Sakura's table is read as it is.
    let plain = mac_with(|o| o.standard_keys = false);
    assert_eq!(code(&plain, "Cmd+W"), Resolved::Code(30400));
    assert_eq!(code(&plain, "Cmd+Left"), Resolved::Code(30320));
    assert_eq!(code(&plain, "Cmd+H"), Resolved::Code(30360));
    assert_eq!(code(&plain, "Cmd+Shift+Z"), Resolved::Unbound);
}

#[test]
fn macos_control_key_is_emacs_or_ignored() {
    let map = mac();
    assert_eq!(id(map.lookup(chord("Control+A"))), "go_line_top");
    assert_eq!(id(map.lookup(chord("Control+E"))), "go_line_end");
    assert_eq!(id(map.lookup(chord("Control+B"))), "cursor_left");
    assert_eq!(id(map.lookup(chord("Control+D"))), "delete");
    assert_eq!(id(map.lookup(chord("Control+H"))), "delete_back");
    assert_eq!(code(&map, "Control+K"), Resolved::Code(30241));
    // Cmd+A is still select all.
    assert_eq!(code(&map, "Cmd+A"), Resolved::Code(30401));
    let ignore = mac_with(|o| o.control_key = ControlKey::Ignore);
    assert_eq!(code(&ignore, "Control+A"), Resolved::Unbound);
    assert_eq!(code(&ignore, "Cmd+A"), Resolved::Code(30401));
}

#[test]
fn macos_option_arrows_are_box_selection_unless_asked() {
    // ADR-0008 maps Sakura's Alt (box selection with Alt+drag, Alt+arrows) to Option.
    assert_eq!(code(&mac(), "Option+Left"), Resolved::Code(30513));
    let words = mac_with(|o| o.option_arrows_by_word = true);
    assert_eq!(code(&words, "Option+Left"), Resolved::Code(30320));
    assert_eq!(code(&words, "Option+Shift+Right"), Resolved::Code(30421));
}

#[test]
fn macos_option_characters_can_be_left_to_text_input() {
    // Option+D is Sakura's Alt+D (sort descending); on a JIS keyboard Option+¥ types a backslash
    // and the Alt column of that key is empty, so there is nothing to clash with.
    let map = mac();
    assert_eq!(code(&map, "Option+D"), Resolved::Code(30283));
    assert_eq!(code(&map, "Option+\\"), Resolved::Unbound);
    let text = mac_with(|o| o.option_text_input = true);
    assert_eq!(code(&text, "Option+D"), Resolved::PassThrough);
    assert_eq!(code(&text, "Option+Shift+1"), Resolved::PassThrough);
    // Keys that type nothing keep their bindings, and so do chords with Cmd.
    assert_eq!(code(&text, "Option+Left"), Resolved::Code(30513));
    assert_eq!(code(&text, "Cmd+Option+L"), Resolved::Code(30800));
}

#[test]
fn the_mac_rules_do_nothing_on_other_systems() {
    let map = KeyMap::new(KeyMapOptions {
        control_key: ControlKey::Emacs,
        ..KeyMapOptions::for_platform(Platform::Other)
    });
    assert_eq!(code(&map, "Ctrl+W"), Resolved::Code(30400));
    assert_eq!(code(&map, "Ctrl+A"), Resolved::Code(30401));
    assert_eq!(code(&map, "Ctrl+Left"), Resolved::Code(30320));
    assert_eq!(code(&map, "Alt+D"), Resolved::Code(30283));
    assert_eq!(map.overlay_len(), 0);
}

#[test]
fn every_name_in_the_mac_tables_is_a_sakura_function() {
    let all = mac_with(|o| o.option_arrows_by_word = true);
    let expected = super::options::STANDARD_KEYS.len()
        + super::options::OPTION_ARROWS.len()
        + super::options::EMACS_KEYS.len();
    assert_eq!(all.overlay_len(), expected);
}

#[test]
fn user_rows_are_used_as_written() {
    let mut map = mac();
    let (rows, warnings) = user::parse_entries([
        // Left: Cmd+Left (Ctrl+Left) goes to the end of the file.
        ("KeyBind[000]", "0025,0,0,F_GOFILEEND,0,0,0,0,0,←"),
        ("KeyBind[001]", "0041,0,0,0,0,0,0,0,0,A"),
    ]);
    assert!(warnings.is_empty());
    map.apply_user_rows(&rows);
    // The mac rule for Cmd+Left no longer applies to that key.
    assert_eq!(code(&map, "Cmd+Left"), Resolved::Code(30351));
    // Cmd+A was emptied by the user; Control+A is not Emacs's any more either.
    assert_eq!(code(&map, "Cmd+A"), Resolved::Unbound);
    assert_eq!(code(&map, "Control+A"), Resolved::Unbound);
    // Other keys keep the rules.
    assert_eq!(id(map.lookup(chord("Cmd+Right"))), "go_line_end");
    assert_eq!(map.rows().len(), KEY_BINDINGS.len());
}

#[test]
fn shortcuts_for_menus() {
    let win = windows();
    assert_eq!(win.shortcut_for(30103), Some(chord("Ctrl+S")));
    assert_eq!(win.shortcut_for(30902), Some(chord("F3")));
    assert_eq!(win.shortcut_for(30601), Some(chord("F7")));
    assert_eq!(win.shortcut_for(30400), Some(chord("Ctrl+W")));
    assert_eq!(win.shortcut_for(1), None);
    assert_eq!(
        win.shortcut_for(30103)
            .unwrap()
            .display(DisplayStyle::Sakura),
        "Ctrl+S"
    );
    // F_WINCLOSE has only the default of Alt+F4.
    assert_eq!(win.shortcut_for(31320), Some(chord("Alt+F4")));
    // Mouse-only functions are listed after keys.
    assert_eq!(
        win.shortcut_for(30384),
        Some(Chord::new(
            Key::Mouse(MouseAction::MiddleClick),
            Modifiers::NONE
        ))
    );

    let m = mac();
    assert_eq!(m.shortcut_for(30103), Some(chord("Cmd+S")));
    assert_eq!(
        m.shortcut_for(30103).unwrap().display(DisplayStyle::Mac),
        "⌘S"
    );
    // The Mac rules come first; the shadowed table cell (Cmd+W = select word) is not listed.
    assert_eq!(m.shortcut_for(31320), Some(chord("Cmd+W")));
    assert_eq!(m.shortcut_for(30332), Some(chord("Cmd+Left")));
    assert!(m.shortcuts_for(30332).contains(&chord("Home")));
    assert!(m.shortcuts_for(30332).contains(&chord("Control+A")));
    assert!(!m.shortcuts_for(30320).contains(&chord("Cmd+Left")));
    assert_eq!(m.shortcut_for(30902), Some(chord("Cmd+G")));
    assert!(m.shortcuts_for(30902).contains(&chord("F3")));
    // Every listed chord resolves back to the function.
    for f in [30103, 30210, 30902, 31320, 30400, 30332] {
        for c in m.shortcuts_for(f) {
            assert_eq!(m.resolve(c), Resolved::Code(f), "{c}");
        }
    }
}

#[test]
fn a_saved_default_table_keeps_the_mac_rules() {
    // Sakura writes the whole table; reading back a table that was never customized must not
    // switch the Mac rules off.
    let saved = user::format_entries(&mac().rows());
    let (rows, warnings) = user::parse_entries(saved.iter().map(|(k, v)| (k.as_str(), v.as_str())));
    assert!(warnings.is_empty());
    let mut map = mac();
    map.apply_user_rows(&rows);
    assert_eq!(code(&map, "Cmd+W"), Resolved::Code(31320));
    assert_eq!(code(&map, "Control+A"), Resolved::Code(30332));
}
