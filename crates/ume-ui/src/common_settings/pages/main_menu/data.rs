//! The menu as one list of numbers (the value of the [`Kind::Numbers`](super::super::super::model::Kind)
//! control that holds it), and how that list is read from and written to `[MainMenu]`.
//!
//! The list is `key in parentheses`, then for each row `level, type, function, key, flags, name length`
//! and the UTF-16 units of the name (the key is the character's code, 0 for none; the flags say that the
//! row is marked as a duplicate key and that its node is collapsed).

use ume_commands::macro_names;
use ume_config::settings::main_menu::CURRENT_VERSION;
use ume_config::settings::{MainMenu, MenuRow, RowKind};
use ume_config::{Profile, ProfileError};

use super::tree::PageRow;

const FLAG_DUPLICATE: i32 = 1;
const FLAG_COLLAPSED: i32 = 2;

/// `GetFunctionStrToFunctionCode`: the code of a macro name.
pub(super) fn code_of_name(name: &str) -> Option<u16> {
    macro_names::by_name(name).map(|f| f.code)
}

/// `GetFuncInfoByID`: the macro name an exported file writes for a function.
pub(super) fn name_of_code(code: u16) -> Option<String> {
    if code == 0 {
        return None;
    }
    macro_names::by_code(code).map(|f| f.name.to_owned())
}

/// The state of the page: the rows of the tree and the check box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct State {
    pub rows: Vec<PageRow>,
    pub key_parentheses: bool,
}

impl State {
    /// The page for `menu` (`SetData`): every node is collapsed, as the tree view starts.
    pub(super) fn of_menu(menu: &MainMenu) -> Self {
        Self {
            rows: menu
                .edit_rows()
                .into_iter()
                .map(|row| PageRow {
                    collapsed: row.kind == RowKind::Node,
                    row,
                    duplicate: false,
                })
                .collect(),
            key_parentheses: menu.key_parentheses,
        }
    }

    /// The menu OK writes (`GetData`).
    pub(super) fn menu(&self) -> MainMenu {
        let rows: Vec<MenuRow> = self.rows.iter().map(|r| r.row.clone()).collect();
        MainMenu::from_edited(&rows, self.key_parentheses)
    }

    pub(super) fn encode(&self) -> Vec<i32> {
        let mut out = vec![i32::from(self.key_parentheses)];
        for page_row in &self.rows {
            let row = &page_row.row;
            let units: Vec<u16> = row.name.encode_utf16().collect();
            out.push(row.level as i32);
            out.push(row.kind.number());
            out.push(i32::from(row.function));
            out.push(row.key.map_or(0, |c| c as i32));
            out.push(
                if page_row.duplicate {
                    FLAG_DUPLICATE
                } else {
                    0
                } | if page_row.collapsed {
                    FLAG_COLLAPSED
                } else {
                    0
                },
            );
            out.push(units.len() as i32);
            out.extend(units.into_iter().map(i32::from));
        }
        out
    }

    /// The state of a list made by [`State::encode`]. A list that is cut short ends at the last whole row.
    pub(super) fn decode(data: &[i32]) -> Self {
        let mut at = 1;
        let mut next = || {
            let value = data.get(at).copied();
            at += 1;
            value
        };
        let mut rows = Vec::new();
        'rows: loop {
            let mut fields = [0; 6];
            for field in &mut fields {
                let Some(value) = next() else { break 'rows };
                *field = value;
            }
            let units: Vec<u16> = (0..fields[5].max(0))
                .map_while(|_| next().and_then(|u| u16::try_from(u).ok()))
                .collect();
            rows.push(PageRow {
                row: MenuRow {
                    level: usize::try_from(fields[0]).unwrap_or(0),
                    kind: RowKind::from_number(fields[1]),
                    function: u16::try_from(fields[2]).unwrap_or(0),
                    key: u32::try_from(fields[3])
                        .ok()
                        .and_then(char::from_u32)
                        .filter(|&c| c != '\0'),
                    name: String::from_utf16_lossy(&units),
                },
                duplicate: fields[4] & FLAG_DUPLICATE != 0,
                collapsed: fields[4] & FLAG_COLLAPSED != 0,
            });
        }
        Self {
            rows,
            key_parentheses: data.first().is_some_and(|&on| on != 0),
        }
    }
}

/// What the file gives (`IO_MainMenu` in reading mode, over Sakura's default menu).
pub(super) fn load(profile: &Profile) -> Vec<i32> {
    State::of_menu(&MainMenu::from_profile(
        profile,
        &code_of_name,
        crate::menu::default_main_menu(),
    ))
    .encode()
}

/// `IO_MainMenu` in writing mode. Nothing is written when the menu is the one the file already gives, so
/// that the file keeps its bytes (and Sakura's default menu is not written out).
pub(super) fn store(data: &[i32], profile: &mut Profile) -> Result<bool, ProfileError> {
    let menu = State::decode(data).menu();
    let current = MainMenu::from_profile(profile, &code_of_name, crate::menu::default_main_menu());
    if current == menu {
        return Ok(false);
    }
    // Rows that did not change keep the file's text (a macro name stays a name).
    let previous = MainMenu::read_raw(profile, &code_of_name);
    menu.write_to_profile(profile, None, previous.as_ref(), Some(CURRENT_VERSION))?;
    Ok(true)
}
