//! Checks of the generated tables against what Sakura's source says.
//!
//! The counts below were taken from Sakura's files without the generator (commit 48db6c7c25e2):
//! - functions: 474 `F_` entries in `sakura_core/Funccode_x.hsrc`, minus 13 that only mark a range
//!   or are -1 (`F_INVALID`, `*_FIRST`, `*_LAST`, `F_CUSTMENU_BASE`, `F_USERMACRO_0`), plus 63
//!   `#define F_` lines in `src/main/resources/sakura_rc.h` (menu labels) = 524.
//! - key bindings: 100 rows in `KeyDataInit` of `sakura_core/func/CKeyBind.cpp`.
//! - main menu: 337 lines in `src/main/resources/MainMenu.ini`, minus the first (a flag) = 336.
//! - groups: 15 `STR_ERR_DLGFUNCLKUP04`..`18` in `sakura_core/func/Funccode.cpp`.

use std::collections::HashSet;

use super::modifier::{CTRL, SHIFT};
use super::{
    FUNCTION_GROUPS, FUNCTIONS, Function, FunctionKind, KEY_BINDINGS, KEY_PARENTHESES, KeyBinding,
    MAIN_MENU, MenuKind,
};

fn function(name: &str) -> &'static Function {
    FUNCTIONS
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("no function {name}"))
}

fn key(name: &str) -> &'static KeyBinding {
    KEY_BINDINGS
        .iter()
        .find(|k| k.key_name == name)
        .unwrap_or_else(|| panic!("no key {name}"))
}

#[test]
fn counts_match_sakura() {
    assert_eq!(FUNCTIONS.len(), 524);
    assert_eq!(KEY_BINDINGS.len(), 100);
    assert_eq!(MAIN_MENU.len(), 336);
    assert_eq!(FUNCTION_GROUPS.len(), 15);
}

#[test]
fn kinds_match_sakura_ranges() {
    let count = |kind| FUNCTIONS.iter().filter(|f| f.kind == kind).count();
    assert_eq!(count(FunctionKind::Reserved), 9);
    assert_eq!(count(FunctionKind::MenuSpecial), 6);
    assert_eq!(count(FunctionKind::MenuLabel), 63);
    assert_eq!(count(FunctionKind::MacroFunction), 59);
    assert_eq!(count(FunctionKind::Internal), 3);
    assert_eq!(count(FunctionKind::MacroOnly), 23);
    assert_eq!(count(FunctionKind::Command), 524 - 9 - 6 - 63 - 59 - 3 - 23);
}

#[test]
fn codes_names_and_ids_are_unique() {
    let mut names = HashSet::new();
    let mut ids = HashSet::new();
    let mut codes = HashSet::new();
    for f in FUNCTIONS {
        assert!(names.insert(f.name), "duplicate name {}", f.name);
        assert!(ids.insert(f.id), "duplicate ID {}", f.id);
        assert!(f.name.starts_with("F_"), "{}", f.name);
        // Only the placeholders (F_0, F_DISABLE, ...) share a code.
        if f.kind != FunctionKind::Reserved {
            assert!(
                codes.insert(f.code),
                "duplicate code {} ({})",
                f.code,
                f.name
            );
        }
    }
}

#[test]
fn ids_follow_the_convention() {
    // ADR-0013 uses `delete_line` for F_DELETE_LINE.
    assert_eq!(function("F_DELETE_LINE").id, "delete_line");
    assert_eq!(function("F_WordDeleteToStart").id, "word_delete_to_start");
    assert_eq!(function("F_OPEN_HfromtoC").id, "open_hfromto_c");
    for f in FUNCTIONS {
        assert!(
            f.id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "{}: {}",
            f.name,
            f.id
        );
    }
}

#[test]
fn function_spot_checks() {
    // Values from sakura_core/Funccode_x.hsrc.
    assert_eq!(function("F_FILESAVE").code, 30103);
    assert_eq!(function("F_FILESAVE").kind, FunctionKind::Command);
    assert_eq!(function("F_UNDO").code, 30210);
    assert_eq!(function("F_SEARCH_NEXT").kind, FunctionKind::Command);
    assert_eq!(function("F_CHGMOD_EOL").kind, FunctionKind::MacroOnly);
    assert_eq!(function("F_GETFILENAME").code, 40001);
    assert_eq!(function("F_GETFILENAME").kind, FunctionKind::MacroFunction);
    assert_eq!(function("F_ISEARCH_ADD_CHAR").code, 0xC001);
    assert_eq!(function("F_ISEARCH_ADD_CHAR").kind, FunctionKind::Internal);
    assert_eq!(function("F_SEPARATOR").code, 1);
    assert_eq!(function("F_FILE_TOPMENU").kind, FunctionKind::MenuLabel);
    // Labels from sakura_core/sakura_rc.rc.
    assert_eq!(function("F_FILESAVE").label, Some("上書き保存"));
    assert_eq!(function("F_FILE_TOPMENU").label, Some("ファイル"));
    assert_eq!(
        function("F_SAVEKEYMACRO_APPE").label,
        Some("キーマクロの記録終了&&保存")
    );
    // Groups from sakura_core/func/Funccode.cpp (the file-operation list is the first group).
    assert_eq!(function("F_FILESAVE").groups, &[0]);
    assert_eq!(FUNCTION_GROUPS[0], "ファイル操作系");
    // F_JUMP_SRCHSTARTPOS is listed in both the cursor-move and the search lists.
    assert_eq!(function("F_JUMP_SRCHSTARTPOS").groups.len(), 2);
    assert!(function("F_CHGMOD_EOL").groups.is_empty());
}

#[test]
fn key_binding_spot_checks() {
    // Rows of KeyDataInit in sakura_core/func/CKeyBind.cpp.
    let code = |name: &str| function(name).code;
    assert_eq!(key("S").key_code, u16::from(b'S'));
    assert_eq!(key("S").functions[CTRL], code("F_FILESAVE"));
    assert_eq!(
        key("S").functions[CTRL | SHIFT],
        code("F_FILESAVEAS_DIALOG")
    );
    assert_eq!(key("Z").functions[CTRL], code("F_UNDO"));
    assert_eq!(key("Y").functions[CTRL], code("F_REDO"));
    assert_eq!(key("F3").key_code, 0x72);
    assert_eq!(key("F3").functions[0], code("F_SEARCH_NEXT"));
    assert_eq!(key("F3").functions[SHIFT], code("F_SEARCH_PREV"));
    assert_eq!(key("F").functions[CTRL], code("F_SEARCH_DIALOG"));
    assert_eq!(key("E").functions[CTRL | SHIFT], code("F_DELETE_LINE"));
    assert_eq!(key("Del").functions[0], code("F_DELETE"));
    // The #define alias _SQL_RUN of CKeyBind.cpp.
    assert_eq!(key("F10").functions[0], code("F_PLSQL_COMPILE_ON_SQLPLUS"));
    // Mouse rows use Sakura's own codes (VKEX_* in CKeyBind.h).
    assert_eq!(key("ダブルクリック").key_code, 0x100);
    assert_eq!(key("ダブルクリック").functions[0], code("F_SELECTWORD"));
    assert_eq!(key("\\").key_code, 0xdc);
    // Names that come from the string table.
    assert_eq!(key("アプリキー").key_code, 0x5d);
}

#[test]
fn every_key_binding_refers_to_a_known_command() {
    let codes: HashSet<u16> = FUNCTIONS.iter().map(|f| f.code).collect();
    let mut key_codes_seen = HashSet::new();
    for k in KEY_BINDINGS {
        for &code in &k.functions {
            assert!(codes.contains(&code), "{}: unknown code {code}", k.key_name);
            let f = FUNCTIONS.iter().find(|f| f.code == code).unwrap();
            assert!(
                matches!(f.kind, FunctionKind::Reserved | FunctionKind::Command),
                "{}: {} is not a command",
                k.key_name,
                f.name
            );
        }
        // Sakura allows duplicate key codes ("the first one wins"); only the two IME variants
        // of `_` do it today, and they are different keys.
        key_codes_seen.insert(k.key_code);
    }
    assert!(key_codes_seen.len() >= KEY_BINDINGS.len() - 2);
}

#[test]
fn main_menu_refers_to_known_codes_and_is_a_tree() {
    let by_code = |code: u16| FUNCTIONS.iter().find(|f| f.code == code);
    const { assert!(KEY_PARENTHESES) };
    assert_eq!(MAIN_MENU[0].level, 0);
    let mut previous_level = 0;
    for (i, item) in MAIN_MENU.iter().enumerate() {
        let f =
            by_code(item.code).unwrap_or_else(|| panic!("item {i}: unknown code {}", item.code));
        assert!(item.level <= previous_level + 1, "item {i} skips a level");
        match item.kind {
            MenuKind::Leaf => assert_eq!(f.kind, FunctionKind::Command, "item {i}: {}", f.name),
            MenuKind::Special => assert_eq!(f.kind, FunctionKind::MenuSpecial, "item {i}"),
            MenuKind::Separator => {
                assert_eq!(item.code, 1);
                assert_eq!(item.label, None);
            }
            MenuKind::Node => {
                // A node has at least one child.
                let next = MAIN_MENU
                    .get(i + 1)
                    .unwrap_or_else(|| panic!("item {i}: empty node"));
                assert_eq!(next.level, item.level + 1, "item {i}: empty node");
            }
        }
        if item.kind != MenuKind::Separator {
            assert!(item.label.is_some(), "item {i}: {} has no label", f.name);
        }
        previous_level = item.level;
    }
}

#[test]
fn main_menu_spot_checks() {
    // Rows of src/main/resources/MainMenu.ini.
    let top: Vec<_> = MAIN_MENU.iter().filter(|m| m.level == 0).collect();
    assert_eq!(top.len(), 8);
    assert_eq!(top[0].label, Some("ファイル"));
    assert_eq!(top[0].access_key, Some('F'));
    assert_eq!(top[1].label, Some("編集"));
    assert_eq!(top[7].label, Some("ヘルプ"));
    // File > Save ("上書き保存(S)") is the fourth item of the File menu.
    let save = &MAIN_MENU[4];
    assert_eq!(
        (save.level, save.code, save.access_key),
        (1, 30103, Some('S'))
    );
    assert_eq!(save.label, Some("上書き保存"));
}
