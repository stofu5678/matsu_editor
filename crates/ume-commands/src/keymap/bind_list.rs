// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/func/CKeyBind.cpp (CKeyBind::CreateKeyBindList)
//   sakura_core/cmd/CViewCommander_Clipboard.cpp (Command_CREATEKEYBINDLIST)
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2001, jepro, genta
// Copyright (C) 2002, YAZAKI, aroka
// Copyright (C) 2007, ryoji
// Copyright (C) 2008, nasukoji
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (the `CNativeW`
// buffer, the string resources and the `KEYDATA` table) was rewritten in Rust on the rows of
// `UserRow` and on the tables of `crate::sakura::macro_functions`, and the names of the
// functions come from the caller (the UI has the macros and the custom menus that name some).
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The text of 「キー割り当て一覧をコピー」 (`F_CREATEKEYBINDLIST`): one line for each key that
//! has a function, with the function's name, its macro name and number, and whether a key macro
//! can record it.

use super::user::UserRow;
use crate::macro_names;
use crate::sakura::macro_functions::MACRO_COMMANDS;

/// The first line (`STR_ERR_DLGKEYBIND1`, `sakura_core/sakura_rc.rc`).
const HEADER: &str = "キー\t機能名\t関数名\t機能番号\tキーマクロ記録可/不可";
/// The name of a function that has none (`STR_ERR_DLGKEYBIND2`).
const NO_NAME: &str = "---名前が定義されていない-----";

/// Whether the key macro records the function (`CSMacroMgr::CanFuncIsKeyMacro`): a function is
/// recorded if its row in the table says so.
fn can_record(code: u16) -> bool {
    MACRO_COMMANDS.iter().any(|f| f.code == code && f.key_macro)
}

/// The macro name of function `code` (`CSMacroMgr::GetFuncInfoByID`, cut at a `(`), or empty.
fn macro_name(code: u16) -> &'static str {
    macro_names::by_code(code).map_or("", |f| f.name.split('(').next().unwrap_or(""))
}

/// The list (`CKeyBind::CreateKeyBindList` with `bGetDefFuncCode` off, which
/// `Command_CREATEKEYBINDLIST` passes): the header, a line of `-----`, then for each of the 8
/// combinations of Shift, Ctrl and Alt in turn, each key of `rows` that has a function with
/// that combination. The lines end with CR LF.
///
/// `name_of` gives the name of a function (`CFuncLookup::Funccode2Name`) or `None` when it has
/// none. A cell that is empty in the table is left out; the default of the key does not fill it
/// in.
pub fn create_key_bind_list(rows: &[UserRow], name_of: &dyn Fn(u16) -> Option<String>) -> String {
    const CR: &str = "\r\n";
    let mut out = String::new();
    out.push_str(HEADER);
    out.push_str(CR);
    out.push_str("-----\t-----\t-----\t-----\t-----");
    out.push_str(CR);
    for index in 0..8 {
        for row in rows {
            let code = row.functions[index];
            if code == 0 {
                continue;
            }
            // `_SHIFT`, `_CTRL`, `_ALT` are the bits 1, 2 and 4 of the index.
            for (bit, prefix) in [(1, "Shift+"), (2, "Ctrl+"), (4, "Alt+")] {
                if index & bit != 0 {
                    out.push_str(prefix);
                }
            }
            out.push_str(row.key.name());
            out.push('\t');
            out.push_str(&name_of(code).unwrap_or_else(|| NO_NAME.to_owned()));
            out.push('\t');
            out.push_str(macro_name(code));
            out.push('\t');
            out.push_str(&code.to_string());
            out.push('\t');
            out.push_str(if can_record(code) { "○" } else { "×" });
            out.push_str(CR);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{Key, KeyMap, KeyMapOptions, Platform};

    /// F_COPY, F_FILESAVE and F_UNDO of `Funccode_x.hsrc`.
    const COPY: u16 = 30602;
    const FILE_SAVE: u16 = 30103;

    fn name(code: u16) -> Option<String> {
        match code {
            COPY => Some("コピー".to_owned()),
            FILE_SAVE => Some("上書き保存".to_owned()),
            _ => None,
        }
    }

    fn row(key: Key, cells: &[(usize, u16)]) -> UserRow {
        let mut functions = [0; 8];
        for &(i, code) in cells {
            functions[i] = code;
        }
        UserRow { key, functions }
    }

    #[test]
    fn the_list_has_the_header_and_a_line_for_each_function_of_a_key() {
        let rows = [row(Key::letter('C').unwrap(), &[(2, COPY)])];
        let list = create_key_bind_list(&rows, &name);
        let lines: Vec<&str> = list.split("\r\n").collect();
        assert_eq!(
            lines[0],
            "キー\t機能名\t関数名\t機能番号\tキーマクロ記録可/不可"
        );
        assert_eq!(lines[1], "-----\t-----\t-----\t-----\t-----");
        assert_eq!(lines[2], format!("Ctrl+C\tコピー\tCopy\t{COPY}\t○"));
        // every line ends with CR LF, so the last piece is empty
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[3], "");
    }

    #[test]
    fn the_modifiers_go_in_the_order_shift_ctrl_alt_and_the_combinations_in_turn() {
        // the Alt cell comes after the Ctrl cell of another key, whatever the order of the rows
        let rows = [
            row(Key::letter('A').unwrap(), &[(4, COPY), (7, FILE_SAVE)]),
            row(Key::letter('B').unwrap(), &[(0, COPY), (2, COPY)]),
        ];
        let list = create_key_bind_list(&rows, &name);
        let keys: Vec<&str> = list
            .split("\r\n")
            .skip(2)
            .filter(|l| !l.is_empty())
            .map(|l| l.split('\t').next().unwrap())
            .collect();
        assert_eq!(keys, ["B", "Ctrl+B", "Alt+A", "Shift+Ctrl+Alt+A"]);
    }

    #[test]
    fn a_function_without_a_name_gets_the_text_of_sakura() {
        // F_UNDO has no name in `name` above
        let rows = [row(Key::letter('Z').unwrap(), &[(2, 30210)])];
        let list = create_key_bind_list(&rows, &name);
        let line = list.split("\r\n").nth(2).unwrap();
        assert!(
            line.starts_with("Ctrl+Z\t---名前が定義されていない-----\t"),
            "{line}"
        );
    }

    #[test]
    fn the_key_macro_column_follows_the_table() {
        // F_FILESAVE is not recorded by a key macro (`CanFuncIsKeyMacro`), F_COPY is
        assert!(can_record(COPY));
        assert!(!can_record(FILE_SAVE));
        let rows = [row(Key::letter('S').unwrap(), &[(2, FILE_SAVE)])];
        let list = create_key_bind_list(&rows, &name);
        let line = list.split("\r\n").nth(2).unwrap();
        assert_eq!(
            line,
            format!("Ctrl+S\t上書き保存\tFileSave\t{FILE_SAVE}\t×")
        );
    }

    #[test]
    fn the_default_table_lists_ctrl_c_and_leaves_empty_cells_out() {
        let map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
        let list = create_key_bind_list(&map.rows(), &name);
        assert!(list.contains(&format!("Ctrl+C\tコピー\tCopy\t{COPY}\t○\r\n")));
        // a row of the table has a line only for a cell with a function
        let functions: usize = map
            .rows()
            .iter()
            .map(|r| r.functions.iter().filter(|&&c| c != 0).count())
            .sum();
        assert_eq!(list.split("\r\n").count(), functions + 3);
    }
}
