//! Lookups in Sakura's macro tables ([`MACRO_COMMANDS`] and [`MACRO_FUNCTIONS`]): from a macro
//! name to a function code and back.
//!
//! Sakura: `CSMacroMgr::GetFuncInfoByName` and `CSMacroMgr::GetFuncInfoByID` in
//! `sakura_core/macro/CSMacroMgr.cpp`.

use crate::sakura::macro_functions::{MACRO_COMMANDS, MACRO_FUNCTIONS, MacroFunction};

/// The row for a macro name. A leading `S_` is ignored (old macros write `S_Down`), and the
/// name is case sensitive. The functions that return a value are searched before the commands,
/// as Sakura does; no name is in both tables.
pub fn by_name(name: &str) -> Option<&'static MacroFunction> {
    let name = name.strip_prefix("S_").unwrap_or(name);
    MACRO_FUNCTIONS
        .iter()
        .chain(MACRO_COMMANDS)
        .find(|f| f.name == name)
}

/// The row for a function code. When several names share the code, the first row of the table
/// wins (it is the name that a saved key macro uses). The commands are searched first.
pub fn by_code(code: u16) -> Option<&'static MacroFunction> {
    MACRO_COMMANDS
        .iter()
        .chain(MACRO_FUNCTIONS)
        .find(|f| f.code == code)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::sakura::FUNCTIONS;
    use crate::sakura::macro_functions::ArgType;

    #[test]
    fn the_tables_have_the_rows_of_sakura() {
        // Counted without the generator, in sakura_core/macro/CSMacroMgr.cpp (commit
        // 48db6c7c25e2): 323 rows in m_MacroFuncInfoCommandArr, 62 in m_MacroFuncInfoArr, and
        // 175 `case F_...:` lines that are not commented out in CanFuncIsKeyMacro. MATSU Editor adds
        // two commands of its own that a key macro records (`CopyLF`, `CopyCR`).
        assert_eq!(MACRO_COMMANDS.len(), 323 + 2);
        assert_eq!(MACRO_FUNCTIONS.len(), 62);
        let recorded: HashSet<_> = MACRO_COMMANDS
            .iter()
            .filter(|f| f.key_macro)
            .map(|f| f.function)
            .collect();
        assert_eq!(recorded.len(), 175 + 2);
        assert!(MACRO_FUNCTIONS.iter().all(|f| !f.key_macro));
    }

    #[test]
    fn names_are_unique_and_codes_agree_with_the_function_table() {
        let mut names = HashSet::new();
        for row in MACRO_COMMANDS.iter().chain(MACRO_FUNCTIONS) {
            assert!(names.insert(row.name), "duplicate macro name {}", row.name);
            let function = FUNCTIONS
                .iter()
                .find(|f| f.name == row.function)
                .unwrap_or_else(|| panic!("{} is not in FUNCTIONS", row.function));
            assert_eq!(function.code, row.code, "{}", row.name);
            assert!(row.args.len() <= 6, "{}", row.name);
        }
        // Functions return a value; commands do not.
        assert!(MACRO_COMMANDS.iter().all(|f| f.result.is_none()));
        assert!(MACRO_FUNCTIONS.iter().all(|f| f.result.is_some()));
    }

    #[test]
    fn rows_are_read_as_in_the_source() {
        // CSMacroMgr.cpp: {F_DOWN, L"Down", {VT_EMPTY, ...}, VT_EMPTY, nullptr}
        let down = by_name("Down").unwrap();
        assert_eq!((down.function, down.args.len()), ("F_DOWN", 0));
        // {F_WCHAR, L"Char", {VT_I4, VT_EMPTY, ...}, ...}
        let char_ = by_name("Char").unwrap();
        assert_eq!((char_.args, char_.key_macro), (&[ArgType::Int][..], true));
        // {F_FILEOPEN2, L"FileOpen", {VT_BSTR, VT_I4, VT_I4, VT_BSTR}, ...}
        assert_eq!(
            by_name("FileOpen").unwrap().args,
            [ArgType::Str, ArgType::Int, ArgType::Int, ArgType::Str]
        );
        // {F_GETLINESTR, L"GetLineStr", {VT_I4, ...}, VT_BSTR, nullptr}
        let get_line = by_name("GetLineStr").unwrap();
        assert_eq!(
            (get_line.args, get_line.result),
            (&[ArgType::Int][..], Some(ArgType::Str))
        );
        // {F_GREP, ..., {VT_BSTR, VT_BSTR, VT_BSTR, VT_I4}, VT_EMPTY, &s_MacroInfoEx_i}: five
        // arguments; GrepReplace has six (s_MacroInfoEx_ii).
        assert_eq!(by_name("Grep").unwrap().args.len(), 5);
        assert_eq!(by_name("GrepReplace").unwrap().args.len(), 6);
        // Not recorded by a key macro: the file commands are commented out in CanFuncIsKeyMacro.
        assert!(!by_name("FileSave").unwrap().key_macro);
    }

    #[test]
    fn lookup_by_name_and_by_code() {
        assert_eq!(by_name("S_Down").unwrap().name, "Down");
        assert_eq!(
            by_name("Down").unwrap().code,
            by_code(by_name("Down").unwrap().code).unwrap().code
        );
        assert!(by_name("down").is_none());
        assert!(by_name("NoSuchMacro").is_none());
        assert!(by_name("").is_none());
        // Two names for one code: the first row is the name for the code (PageUp before 1PageUp).
        assert_eq!(
            by_name("1PageUp").unwrap().code,
            by_name("PageUp").unwrap().code
        );
        assert_eq!(
            by_code(by_name("1PageUp").unwrap().code).unwrap().name,
            "PageUp"
        );
        assert_eq!(by_name("GetFilename").unwrap().result, Some(ArgType::Str));
        assert!(by_code(0).is_none());
    }
}
