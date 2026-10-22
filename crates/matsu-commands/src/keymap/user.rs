//! The user's key bindings, in the form of Sakura's `KeyBind` entries.
//!
//! Sakura keeps the key table in `[KeyBind]` of `sakura.ini` and in `.key` files as lines
//! `KeyBind[NNN]=KKKK,F0,F1,F2,F3,F4,F5,F6,F7,Name`: the key code in hex, the function of each
//! of the 8 modifier combinations, then the key's name (`ShareData_IO_KeyBind` in
//! `sakura_core/env/CShareData_IO.cpp`). This module reads and writes those entries as
//! (name, value) pairs, so the settings crate can hand over the section without this crate
//! depending on it. Reading and writing whole `.key` files (header and `[Info]` section,
//! `CImpExpKeybind` in `sakura_core/typeprop/CImpExpManager.cpp`) is left to that crate.

use std::fmt;

use super::key::Key;
use crate::macro_names;
use crate::sakura::{FUNCTIONS, FunctionKind};

/// One row of the key table: a key and the function codes of its 8 modifier combinations
/// (index: Shift 1, Ctrl 2, Alt 4; 0 means none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UserRow {
    pub key: Key,
    pub functions: [u16; 8],
}

/// An entry that was read with a problem. The entry is skipped, or the function is read as
/// "none" (Sakura does the latter silently).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    /// The entry's name, for example `KeyBind[012]`.
    pub entry: String,
    pub problem: Problem,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Fewer than the key code, 8 functions and the name.
    TooFewFields,
    /// The key code is not hexadecimal.
    BadKeyCode(String),
    /// The key code is not a key of Sakura's table.
    UnknownKey(u16),
    /// A function that is neither a number nor a known name (plugin commands included).
    UnknownFunction(String),
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.entry)?;
        match &self.problem {
            Problem::TooFewFields => write!(f, "too few fields"),
            Problem::BadKeyCode(s) => write!(f, "bad key code {s:?}"),
            Problem::UnknownKey(c) => write!(f, "unknown key code {c:#06x}"),
            Problem::UnknownFunction(s) => write!(f, "unknown function {s:?}"),
        }
    }
}

fn strip_f_prefix(name: &str) -> &str {
    match name.get(..2) {
        Some(head) if head.eq_ignore_ascii_case("F_") => &name[2..],
        _ => name,
    }
}

/// Reads a function: a number, or a name (`F_FILESAVE`, `FileSave`, or our ID `file_save`; case
/// does not matter). Sakura's `GetFunctionStrToFunctionCode` takes anything that starts with a
/// digit as a number and the rest as macro names (`CSMacroMgr::GetFuncInfoByName`, here
/// [`macro_names::by_name`]: `SearchNext` for `F_SEARCH_NEXT`, with or without `S_`). The names that equal
/// a function constant without `F_` and our own IDs are read too, after the macro names.
pub fn parse_function(token: &str) -> Option<u16> {
    let token = token.trim();
    if token.starts_with(|c: char| c.is_ascii_digit()) {
        return token.parse().ok();
    }
    if let Some(row) = macro_names::by_name(token) {
        return Some(row.code);
    }
    let bare = strip_f_prefix(token);
    FUNCTIONS
        .iter()
        .filter(|f| f.kind != FunctionKind::Reserved)
        .find(|f| {
            strip_f_prefix(f.name).eq_ignore_ascii_case(bare) || f.id.eq_ignore_ascii_case(token)
        })
        .map(|f| f.code)
}

/// Reads `KeyBind[...]` entries, given as (name, value) pairs. Other names are ignored. Later
/// rows of the same key win. Problems are collected, not fatal.
pub fn parse_entries<'a>(
    entries: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> (Vec<UserRow>, Vec<Warning>) {
    let mut rows: Vec<UserRow> = Vec::new();
    let mut warnings = Vec::new();
    for (name, value) in entries {
        if !name.starts_with("KeyBind[") {
            continue;
        }
        let mut warn = |problem| {
            warnings.push(Warning {
                entry: name.to_owned(),
                problem,
            });
        };
        // The key name at the end may itself contain a comma (the "," key), so split at most
        // 10 times: code, 8 functions, rest.
        let fields: Vec<&str> = value.splitn(10, ',').collect();
        if fields.len() < 10 {
            warn(Problem::TooFewFields);
            continue;
        }
        let Ok(code) = u16::from_str_radix(fields[0].trim(), 16) else {
            warn(Problem::BadKeyCode(fields[0].to_owned()));
            continue;
        };
        let Some(key) = Key::from_code(code) else {
            warn(Problem::UnknownKey(code));
            continue;
        };
        let mut functions = [0u16; 8];
        for (cell, token) in functions.iter_mut().zip(&fields[1..9]) {
            match parse_function(token) {
                Some(code) => *cell = code,
                None => warn(Problem::UnknownFunction((*token).to_owned())),
            }
        }
        match rows.iter_mut().find(|r| r.key == key) {
            Some(existing) => existing.functions = functions,
            None => rows.push(UserRow { key, functions }),
        }
    }
    (rows, warnings)
}

/// Writes rows as `KeyBind[NNN]` entries with numeric function codes, like `sakura.ini`.
pub fn format_entries(rows: &[UserRow]) -> Vec<(String, String)> {
    format_entries_with(rows, &|_| None)
}

/// Writes rows as the entries of a `.key` file: the functions by their macro names where they have
/// one, as numbers otherwise (`ShareData_IO_KeyBind` with `bOutCmdName`, `CSMacroMgr::GetFuncInfoByID`).
pub fn format_entries_named(rows: &[UserRow]) -> Vec<(String, String)> {
    format_entries_with(rows, &|code| {
        // `GetFuncInfoByID` finds nothing for 0 or less.
        (code != 0)
            .then(|| macro_names::by_code(code).map(|row| row.name.to_owned()))
            .flatten()
    })
}

fn format_entries_with(
    rows: &[UserRow],
    name_of: &dyn Fn(u16) -> Option<String>,
) -> Vec<(String, String)> {
    rows.iter()
        .enumerate()
        .map(|(i, row)| {
            let mut value = format!("{:04x}", row.key.code());
            for f in row.functions {
                match name_of(f) {
                    Some(name) => value.push_str(&format!(",{name}")),
                    None => value.push_str(&format!(",{f}")),
                }
            }
            value.push(',');
            value.push_str(row.key.name());
            (format!("KeyBind[{i:03}]"), value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{Chord, KeyMap, KeyMapOptions, Modifiers, Platform, Resolved};

    #[test]
    fn functions_are_read_by_number_or_name() {
        assert_eq!(parse_function("30103"), Some(30103));
        assert_eq!(parse_function("0"), Some(0));
        assert_eq!(parse_function("F_FILESAVE"), Some(30103));
        assert_eq!(parse_function("FileSave"), Some(30103));
        assert_eq!(parse_function("filesave"), Some(30103));
        assert_eq!(parse_function("f_filesave"), Some(30103));
        assert_eq!(parse_function("word_cut"), Some(30232));
        assert_eq!(parse_function("Plugin/Command"), None);
        assert_eq!(parse_function("NoSuchThing"), None);
    }

    #[test]
    fn sakuras_macro_names_are_read() {
        // CSMacroMgr.cpp: {F_SEARCH_NEXT, L"SearchNext", ...}, {F_FILEOPEN, L"FileOpen", ...}
        assert_eq!(parse_function("SearchNext"), Some(30902));
        assert_eq!(parse_function("S_SearchNext"), Some(30902));
        // The macro table is case sensitive; the names of the function constants are the fallback.
        assert_eq!(parse_function("search_next"), Some(30902));
        assert_eq!(parse_function("searchnext"), None);
    }

    #[test]
    fn the_default_table_round_trips_with_names() {
        let map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
        let rows = map.rows();
        let entries = format_entries_named(&rows);
        // F3 searches forward: ShareData_IO_KeyBind writes the macro name where there is one.
        assert!(
            entries
                .iter()
                .any(|(_, v)| v.starts_with("0072,SearchNext,")),
            "{entries:?}"
        );
        assert!(
            entries[11].1.starts_with("0070,ExtHtmlHelp,CommandList,"),
            "{}",
            entries[11].1
        );
        // A function with no macro name, and 0, are numbers.
        let (back, warnings) = parse_entries(entries.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(back, rows);
        assert!(entries.iter().any(|(_, v)| v.contains(",0,")));
    }

    #[test]
    fn the_default_table_round_trips_through_entries() {
        let map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
        let rows = map.rows();
        let entries = format_entries(&rows);
        // Sakura writes e.g. `KeyBind[011]=0070,31451,31445,31450,31455,31440,31441,0,0,F1`.
        assert_eq!(entries[11].0, "KeyBind[011]");
        assert_eq!(
            entries[11].1,
            "0070,31451,31445,31450,31455,31440,31441,0,0,F1"
        );
        let (back, warnings) = parse_entries(entries.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(back, rows);
    }

    #[test]
    fn a_key_name_with_a_comma_is_kept() {
        let (rows, warnings) = parse_entries([("KeyBind[000]", "00bc,0,0,30103,0,0,0,0,0,,")]);
        assert!(warnings.is_empty());
        assert_eq!(rows[0].key, Key::Comma);
        assert_eq!(rows[0].functions[2], 30103);
    }

    #[test]
    fn bad_entries_are_reported_and_skipped() {
        let (rows, warnings) = parse_entries([
            ("Other", "x"),
            ("KeyBind[000]", "0041,1,2"),
            ("KeyBind[001]", "zz,0,0,0,0,0,0,0,0,A"),
            ("KeyBind[002]", "0003,0,0,0,0,0,0,0,0,Break"),
            ("KeyBind[003]", "0041,0,0,Bogus,0,0,0,0,0,A"),
        ]);
        let problems: Vec<_> = warnings.iter().map(|w| w.problem.clone()).collect();
        assert_eq!(
            problems,
            [
                Problem::TooFewFields,
                Problem::BadKeyCode("zz".into()),
                Problem::UnknownKey(3),
                Problem::UnknownFunction("Bogus".into()),
            ]
        );
        // The row with the unknown function is kept, with that cell empty.
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].functions, [0; 8]);
        assert!(warnings[0].to_string().starts_with("KeyBind[000]"));
    }

    #[test]
    fn a_user_row_replaces_the_whole_row() {
        let mut map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
        let ctrl_s = Chord::new(Key::Letter('S'), Modifiers::CTRL);
        let ctrl_shift_s = Chord::new(Key::Letter('S'), Modifiers::CTRL.with_shift());
        assert_eq!(map.resolve(ctrl_s), Resolved::Code(30103));
        let (rows, _) = parse_entries([("KeyBind[000]", "0053,0,0,F_FILEOPEN,0,0,0,0,0,S")]);
        map.apply_user_rows(&rows);
        assert_eq!(map.resolve(ctrl_s), Resolved::Code(30102));
        // Ctrl+Shift+S was in the row and is gone now.
        assert_eq!(map.resolve(ctrl_shift_s), Resolved::Unbound);
        assert!(map.shortcuts_for(30102).contains(&ctrl_s));
        assert!(map.shortcuts_for(30103).is_empty());
    }
}
