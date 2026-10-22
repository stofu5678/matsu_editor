//! Functions that only UME Editor has. They are added to the tables that come from Sakura's
//! source, so that the generated tables are complete and `committed_tables_match_sakura` still
//! checks them.
//!
//! Codes: the highest code of Sakura's commands in `Funccode_x.hsrc` is 32018 (the command
//! range is 30000-32767). UME-only commands use [`FIRST_CODE`] and up, far from Sakura's, so
//! that a Sakura function added later does not collide with them. Names start with `F_` like
//! Sakura's, and the command ID is the name without it (`copy_lf`).

use crate::parse::{Function, Kind, MenuItem, MenuKind, Tables};

/// The first code reserved for UME-only functions (up to 32767, the end of the command range).
pub const FIRST_CODE: u16 = 32700;

/// One UME-only command.
pub struct Extra {
    pub code: u16,
    pub name: &'static str,
    pub id: &'static str,
    /// The index into the function groups (5 is クリップボード系).
    pub group: usize,
    pub label: &'static str,
    /// The function after which this one is placed in the function table, the main menu and
    /// the macro table.
    pub after: &'static str,
    /// The access key in the main menu.
    pub access_key: char,
    /// The name a macro calls.
    pub macro_name: &'static str,
}

/// In order: the second one is placed after the first.
pub const EXTRAS: &[Extra] = &[
    Extra {
        code: 32700,
        name: "F_COPY_LF",
        id: "copy_lf",
        group: 5,
        label: "LF改行でコピー",
        after: "F_COPY_CRLF",
        access_key: 'N',
        macro_name: "CopyLF",
    },
    Extra {
        code: 32701,
        name: "F_COPY_CR",
        id: "copy_cr",
        group: 5,
        label: "CR改行でコピー",
        after: "F_COPY_LF",
        access_key: 'M',
        macro_name: "CopyCR",
    },
];

/// Adds the functions and the main menu items.
pub fn apply(tables: &mut Tables) -> Result<(), String> {
    for extra in EXTRAS {
        if extra.code < FIRST_CODE {
            return Err(format!(
                "{}: code {} is below {FIRST_CODE}",
                extra.name, extra.code
            ));
        }
        if tables.functions.iter().any(|f| f.code == extra.code) {
            return Err(format!("code {} is already used", extra.code));
        }
        let position = tables
            .functions
            .iter()
            .position(|f| f.name == extra.after)
            .ok_or_else(|| format!("{} is not a function", extra.after))?;
        let after_code = tables.functions[position].code;
        tables.functions.insert(
            position + 1,
            Function {
                code: extra.code,
                name: extra.name.to_string(),
                id: extra.id.to_string(),
                kind: Kind::Command,
                groups: vec![extra.group],
                label: Some(extra.label.to_string()),
            },
        );
        let menu = tables
            .menu
            .iter()
            .position(|i| i.kind == MenuKind::Leaf && i.code == after_code)
            .ok_or_else(|| format!("{} is not in the main menu", extra.after))?;
        let level = tables.menu[menu].level;
        tables.menu.insert(
            menu + 1,
            MenuItem {
                level,
                kind: MenuKind::Leaf,
                code: extra.code,
                access_key: Some(extra.access_key),
                label: Some(extra.label.to_string()),
            },
        );
    }
    Ok(())
}
