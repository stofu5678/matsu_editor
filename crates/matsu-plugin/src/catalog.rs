//! Every function a macro can call, in one list (ADR-0007: the API is defined in one place and
//! generated for every language).
//!
//! The list is [`crate::api::FUNCS`] (the hand-written functions, with parameter names and a
//! description) followed by every other row of Sakura's macro tables
//! (`matsu_commands::sakura::macro_functions`, generated from `sakura_core/macro/CSMacroMgr.cpp`):
//! `m_MacroFuncInfoCommandArr` (commands, no result) and `m_MacroFuncInfoArr` (functions with a
//! result). A name keeps the spelling of the table, and an old spelling that shares a function
//! code with another name (`ReadOnly` and `ViewMode`) is an entry of its own.
//!
//! The code generator (`codegen.rs`) and the bundled engines (`matsu-script`: Lua, QuickJS) read
//! this list, so that a new row of the tables or a new `FUNCS` entry reaches every language
//! without another change. What a call does is decided by the bridge (`crate::bridge`): a
//! command runs through the registry, as a key macro does, and a name without a command answers
//! [`crate::bridge::NOT_IMPLEMENTED`].

use std::sync::OnceLock;

use matsu_commands::sakura::macro_functions::{
    ArgType, MACRO_COMMANDS, MACRO_FUNCTIONS, MacroFunction,
};

use crate::api::{FUNCS, Param, Ty};

/// One callable name.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The name a macro calls, with the spelling of Sakura's table.
    pub name: &'static str,
    /// The parameters. A row of Sakura's table has no parameter names: they are `arg1`, `arg2`,
    /// ... and the macro may leave out any of the last ones (the command decides whether it can
    /// do without them, as in Sakura).
    pub params: Vec<Param>,
    pub result: Ty,
    pub doc: String,
    /// Sakura's name of the function (`F_DOWN`), when the name is in Sakura's tables.
    pub function: Option<&'static str>,
    /// Whether the name is a command (no result) in Sakura's tables, or a hand-written
    /// function of `FUNCS` that is not.
    pub is_command: bool,
}

impl Entry {
    /// Whether the name can be a method name in the generated JavaScript, Python, Ruby and PHP
    /// code. A few Sakura names start with a digit (`1PageUp`): a macro reaches them through the
    /// by-name fallback of the generated code (`Editor['1PageUp']()`, `getattr`, `send`) and the
    /// bundled engines bind every name as a table key.
    pub fn is_identifier(&self) -> bool {
        let mut chars = self.name.chars();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
}

/// The parameter names of a row without names. No row has more arguments than this (checked by a
/// test).
const ARG_NAMES: [&str; 8] = [
    "arg1", "arg2", "arg3", "arg4", "arg5", "arg6", "arg7", "arg8",
];

/// Every callable name, `FUNCS` first and then the table rows in table order (commands, then
/// functions with a result). Each name appears once.
pub fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(build)
}

/// The entry of a name.
pub fn find(name: &str) -> Option<&'static Entry> {
    entries().iter().find(|e| e.name == name)
}

fn build() -> Vec<Entry> {
    let mut list: Vec<Entry> = FUNCS
        .iter()
        .map(|func| {
            let row = matsu_commands::macro_names::by_name(func.name);
            Entry {
                name: func.name,
                params: func.params.to_vec(),
                result: func.result,
                doc: func.doc.to_string(),
                function: row.map(|r| r.function),
                is_command: row.is_some_and(|r| r.result.is_none()),
            }
        })
        .collect();
    // The functions that need the editor (`crate::functions`), with their own parameter names.
    for func in crate::functions::all() {
        let row = matsu_commands::macro_names::by_name(func.name);
        list.push(Entry {
            name: func.name,
            params: func.params.to_vec(),
            result: func.result,
            doc: func.doc.to_string(),
            function: row.map(|r| r.function),
            is_command: false,
        });
    }
    for row in MACRO_COMMANDS.iter().chain(MACRO_FUNCTIONS) {
        if list.iter().any(|e| e.name == row.name) {
            continue;
        }
        list.push(from_row(row));
    }
    list
}

fn ty(ty: ArgType) -> Ty {
    match ty {
        ArgType::Int => Ty::Int,
        ArgType::Str => Ty::Str,
    }
}

/// The first function code of the commands that only MATSU Editor has (Sakura's end at 32018).
const MATSU_ONLY_FIRST_CODE: u16 = 32700;

fn from_row(row: &'static MacroFunction) -> Entry {
    let params = row
        .args
        .iter()
        .enumerate()
        .map(|(i, arg)| Param {
            name: ARG_NAMES[i],
            ty: ty(*arg),
            optional: true,
        })
        .collect();
    let doc = if row.code >= MATSU_ONLY_FIRST_CODE {
        // The commands that only MATSU Editor has (`xtask/src/matsu_extras.rs`).
        format!(
            "MATSU Editor's own {} (not in Sakura). Runs the command that a key, the menu and a key \
             macro run.",
            row.function
        )
    } else if row.result.is_none() {
        format!(
            "Sakura's {} (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the \
             menu and a key macro run. The last arguments may be left out. A command that this \
             editor does not have yet answers the error -32001 (not implemented).",
            row.function
        )
    } else {
        format!(
            "Sakura's {} (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This \
             editor answers the error -32001 (not implemented) while it cannot answer it.",
            row.function
        )
    };
    Entry {
        name: row.name,
        params,
        result: row.result.map_or(Ty::Null, ty),
        doc,
        function: Some(row.function),
        is_command: row.result.is_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_of_the_tables_and_of_funcs_is_listed_once() {
        let list = entries();
        for row in MACRO_COMMANDS.iter().chain(MACRO_FUNCTIONS) {
            let found: Vec<_> = list.iter().filter(|e| e.name == row.name).collect();
            assert_eq!(found.len(), 1, "{}", row.name);
            assert_eq!(found[0].function, Some(row.function), "{}", row.name);
            assert_eq!(found[0].is_command, row.result.is_none(), "{}", row.name);
        }
        for func in FUNCS {
            assert!(find(func.name).is_some(), "{}", func.name);
        }
        let mut names: Vec<_> = list.iter().map(|e| e.name).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "a name is listed twice");
    }

    #[test]
    fn the_editor_functions_are_rows_of_the_table_with_its_types() {
        for func in crate::functions::all() {
            let row = matsu_commands::macro_names::by_name(func.name)
                .unwrap_or_else(|| panic!("{} is not a name of Sakura's tables", func.name));
            assert!(row.result.is_some(), "{}", func.name);
            assert_eq!(ty_of(row.result), func.result, "{}", func.name);
            assert!(func.params.len() <= row.args.len(), "{}", func.name);
            for (param, arg) in func.params.iter().zip(row.args) {
                assert_eq!(param.ty, ty(*arg), "{} {}", func.name, param.name);
            }
            // a parameter that the table has but the function leaves out would be refused
            assert_eq!(func.params.len(), row.args.len(), "{}", func.name);
            let found = find(func.name).unwrap();
            assert_eq!(found.params.len(), func.params.len(), "{}", func.name);
            assert!(!found.is_command, "{}", func.name);
        }
        // no name is defined by both lists
        for func in crate::functions::all() {
            assert!(!FUNCS.iter().any(|f| f.name == func.name), "{}", func.name);
        }
    }

    /// A parameter name becomes a parameter of generated JavaScript, Python, Ruby and PHP, and of
    /// the Lua and QuickJS code of the bundled engines: it must not be a keyword of any of them.
    #[test]
    fn parameter_names_are_not_keywords_of_any_language() {
        const RESERVED: &[&str] = &[
            // JavaScript
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "else",
            "enum",
            "export",
            "extends",
            "false",
            "finally",
            "for",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "new",
            "null",
            "return",
            "super",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "typeof",
            "var",
            "void",
            "while",
            "with",
            "yield",
            "let",
            "static",
            "await",
            "async",
            "of",
            "arguments",
            "eval",
            // Python
            "and",
            "as",
            "assert",
            "def",
            "del",
            "elif",
            "except",
            "from",
            "global",
            "is",
            "lambda",
            "nonlocal",
            "not",
            "or",
            "pass",
            "raise",
            "None",
            "True",
            "False",
            // Ruby
            "alias",
            "begin",
            "end",
            "ensure",
            "module",
            "next",
            "nil",
            "redo",
            "rescue",
            "retry",
            "self",
            "then",
            "undef",
            "unless",
            "until",
            "when",
            "defined",
            "__FILE__",
            // PHP
            "abstract",
            "array",
            "callable",
            "clone",
            "declare",
            "echo",
            "empty",
            "endif",
            "exit",
            "final",
            "foreach",
            "goto",
            "implements",
            "include",
            "interface",
            "isset",
            "list",
            "namespace",
            "print",
            "private",
            "protected",
            "public",
            "require",
            "unset",
            "use",
            "xor",
            "fn",
            "match",
            "readonly",
            // Lua
            "repeat",
            "local",
            "elseif",
        ];
        for entry in entries() {
            for param in &entry.params {
                assert!(
                    !RESERVED.contains(&param.name),
                    "{}: the parameter name \"{}\" is a keyword",
                    entry.name,
                    param.name
                );
            }
        }
    }

    fn ty_of(result: Option<ArgType>) -> Ty {
        result.map_or(Ty::Null, ty)
    }

    #[test]
    fn names_that_share_a_function_code_are_both_listed() {
        // Old spellings: ReadOnly and ViewMode are both F_VIEWMODE.
        assert_eq!(find("ViewMode").unwrap().function, Some("F_VIEWMODE"));
        assert_eq!(find("ReadOnly").unwrap().function, Some("F_VIEWMODE"));
        // The spelling of the table is kept.
        assert!(find("down").is_none());
        assert!(find("Down").is_some());
    }

    #[test]
    fn the_hand_written_functions_keep_their_parameters() {
        let jump = find("Jump").unwrap();
        assert_eq!(jump.params.len(), 2);
        assert_eq!(jump.params[0].name, "line");
        assert!(jump.params[1].optional);
        // A table row has numbered parameters, all of which may be left out.
        let open = find("FileOpen").unwrap();
        assert_eq!(open.params.len(), 4);
        assert_eq!(open.params[0].name, "arg1");
        assert_eq!(open.params[0].ty, Ty::Str);
        assert!(open.params.iter().all(|p| p.optional));
        assert_eq!(open.result, Ty::Null);
    }

    #[test]
    fn the_parameter_names_are_enough_and_the_names_are_plain() {
        for entry in entries() {
            assert!(entry.params.len() <= ARG_NAMES.len(), "{}", entry.name);
            assert!(
                entry
                    .name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{}",
                entry.name
            );
            // The by-name fallback of the generated code takes an upper case letter or a digit.
            assert!(
                entry
                    .name
                    .starts_with(|c: char| c.is_ascii_uppercase() || c.is_ascii_digit())
            );
        }
        let digits: Vec<_> = entries()
            .iter()
            .filter(|e| !e.is_identifier())
            .map(|e| e.name)
            .collect();
        assert!(digits.contains(&"1PageUp"), "{digits:?}");
        assert!(
            digits
                .iter()
                .all(|n| n.starts_with(|c: char| c.is_ascii_digit()))
        );
    }
}
