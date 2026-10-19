//! The key table the page edits, and how it is read from and written to `[KeyBind]`.
//!
//! The page holds the table as a [`Value::Numbers`](crate::common_settings::model::Value) list:
//! nine numbers for each key (the key code, then the function of each of the 8 modifier
//! combinations). [`Bindings`] is the same data with the operations of the page
//! (`CPropKeybind::DispatchEvent`).

use ume_commands::keymap::user::{format_entries, parse_entries};
use ume_commands::keymap::{Key, KeyMap, KeyMapOptions, MouseAction, Platform, UserRow};
use ume_config::{Profile, ProfileError};

/// The section of the file (`ShareData_IO_KeyBind`).
const SECTION: &str = "KeyBind";
/// The numbers of one key in the list of the page: its code and 8 functions.
const WIDTH: usize = 9;
/// `_SHIFT`, `_CTRL` and `_ALT` (`CKeyBind.h`): the bits of the index of a modifier combination.
pub(super) const SHIFT: usize = 1;
pub(super) const CTRL: usize = 2;
pub(super) const ALT: usize = 4;

/// The table as Sakura has it before the file is read, with the rows of the file over it
/// (`CShareData::InitKeyAssign`, then `ShareData_IO_KeyBind`). It is Sakura's table on every OS:
/// the macOS rules of UME Editor apply on top when the key map is made, not in the page.
pub(super) fn load(profile: &Profile) -> Vec<i32> {
    let (user, _) = parse_entries(profile.entries(SECTION));
    let mut map = KeyMap::new(KeyMapOptions::for_platform(Platform::Other));
    map.apply_user_rows(&user);
    Bindings::from_rows(&map.rows()).to_numbers()
}

/// `ShareData_IO_KeyBind` writing: every key of the table as `KeyBind[NNN]`, with the function
/// codes as numbers. A row that is the same as the one in the file keeps its text there (the
/// names of functions that UME Editor does not know, such as plugin commands, stay in the file).
/// Returns whether the file changed; nothing is written when the table is the one the file gives.
pub(super) fn store(list: &[i32], profile: &mut Profile) -> Result<bool, ProfileError> {
    if load(profile) == list {
        return Ok(false);
    }
    let rows = Bindings::from_numbers(list).rows;
    // The text of each key in the file (a later row of the same key wins, as when it is read).
    let mut old: Vec<(Key, String)> = Vec::new();
    for (name, value) in profile.entries(SECTION) {
        if let Some(row) = parse_entries([(name, value)]).0.first() {
            old.retain(|(key, _)| *key != row.key);
            old.push((row.key, value.to_owned()));
        }
    }
    let mut changed = false;
    for ((name, formatted), row) in format_entries(&rows).into_iter().zip(&rows) {
        let value = old
            .iter()
            .find(|(key, _)| *key == row.key)
            .map_or(formatted, |(_, text)| merge(&name, text, row));
        if profile.get(SECTION, &name) != Some(value.as_str()) {
            profile.set(SECTION, &name, &value)?;
            changed = true;
        }
    }
    // Entries past the end of the table go, gaps or not: a later row of a key would win over the
    // one written above when the file is read.
    let leftovers: Vec<String> = profile
        .entries(SECTION)
        .filter(|(name, _)| entry_number(name).is_some_and(|n| n >= rows.len()))
        .map(|(name, _)| name.to_owned())
        .collect();
    for name in leftovers {
        changed |= profile.remove(SECTION, &name);
    }
    Ok(changed)
}

/// The text of `row` where the file has `old` for its key: every cell whose function did not
/// change keeps the token of the file (a plugin command or a name UME Editor does not know reads
/// as 0 and would be lost otherwise), and so do the key code and the key name. Only the cells that
/// changed are written as numbers.
fn merge(name: &str, old: &str, row: &UserRow) -> String {
    let tokens: Vec<&str> = old.splitn(10, ',').collect();
    let parsed = parse_entries([(name, old)]).0;
    let (Some(before), 10) = (parsed.first(), tokens.len()) else {
        return format_entries(std::slice::from_ref(row)).remove(0).1;
    };
    let mut text = tokens[0].to_owned();
    for (cell, token) in tokens[1..9].iter().enumerate() {
        text.push(',');
        if before.functions[cell] == row.functions[cell] {
            text.push_str(token);
        } else {
            text.push_str(&row.functions[cell].to_string());
        }
    }
    text.push(',');
    text.push_str(tokens[9]);
    text
}

/// The number in `KeyBind[NNN]`.
fn entry_number(name: &str) -> Option<usize> {
    name.strip_prefix("KeyBind[")?
        .strip_suffix(']')?
        .parse()
        .ok()
}

/// What a key cannot do in UME Editor, if anything: the reason the page shows. The assignment is
/// still written to the file (Sakura reads it), but nothing in UME Editor looks it up.
///
/// * The mouse actions that the window handles itself (the double click selects a word, ...) are
///   not looked up in the key table; only the wheel (`crate::view::paging`) and the right click
///   (`crate::custom_menu::mouse`) are.
/// * `_(PC-98)` is a key of the PC-98 keyboard, which no event of the window stands for.
pub(super) fn why_unusable(key: Key) -> Option<&'static str> {
    match key {
        Key::Mouse(
            MouseAction::DoubleClick
            | MouseAction::MiddleClick
            | MouseAction::LeftSideClick
            | MouseAction::RightSideClick
            | MouseAction::TripleClick
            | MouseAction::QuadrupleClick,
        ) => Some("UME Editor は、この操作をキー割り当てから引きません。"),
        Key::UnderscorePc98 => Some("PC-98 のキーボードにあるキーで、UME Editor では押せません。"),
        _ => None,
    }
}

/// The table, with the operations of the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Bindings {
    pub rows: Vec<UserRow>,
}

/// A key and modifiers that a function is assigned to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Assigned {
    /// What the list shows: `Shift+Ctrl+Alt+` and the name of the key.
    pub text: String,
    /// The index of the key in the table.
    pub key: usize,
    /// The index of the modifiers.
    pub mods: usize,
}

impl Bindings {
    pub(super) fn from_rows(rows: &[UserRow]) -> Self {
        Self {
            rows: rows.to_vec(),
        }
    }

    /// Reads the list of the page. A part that is not a whole key, and a key that is not in the
    /// table, are left out.
    pub(super) fn from_numbers(list: &[i32]) -> Self {
        let rows = list
            .as_chunks::<WIDTH>()
            .0
            .iter()
            .filter_map(|chunk| {
                let key = Key::from_code(u16::try_from(chunk[0]).ok()?)?;
                let mut functions = [0u16; 8];
                for (cell, &n) in functions.iter_mut().zip(&chunk[1..]) {
                    *cell = u16::try_from(n).unwrap_or(0);
                }
                Some(UserRow { key, functions })
            })
            .collect();
        Self { rows }
    }

    pub(super) fn to_numbers(&self) -> Vec<i32> {
        self.rows
            .iter()
            .flat_map(|row| {
                std::iter::once(i32::from(row.key.code()))
                    .chain(row.functions.iter().map(|&f| i32::from(f)))
            })
            .collect()
    }

    /// The function of `mods` of the key at `key` (0: none).
    pub(super) fn function(&self, key: usize, mods: usize) -> u16 {
        self.rows
            .get(key)
            .and_then(|row| row.functions.get(mods))
            .copied()
            .unwrap_or(0)
    }

    /// 「割当」 and 「解除」: the function of `mods` of the key at `key`; 0 releases it
    /// (`F_DEFAULT`). Returns whether there was such a key.
    pub(super) fn set_function(&mut self, key: usize, mods: usize, code: u16) -> bool {
        match self
            .rows
            .get_mut(key)
            .and_then(|row| row.functions.get_mut(mods))
        {
            Some(cell) => {
                *cell = code;
                true
            }
            None => false,
        }
    }

    /// The keys function `code` is assigned to (`CKeyBind::GetKeyStrList` without the default
    /// functions): for each modifier combination in turn, the keys of the table in its order; the
    /// list box sorts them (`LBS_SORT`).
    pub(super) fn assigned_to(&self, code: u16) -> Vec<Assigned> {
        if code == 0 {
            return Vec::new();
        }
        let mut found = Vec::new();
        for mods in 0..8 {
            for (key, row) in self.rows.iter().enumerate() {
                if row.functions[mods] == code {
                    found.push(Assigned {
                        text: format!("{}{}", modifier_prefix(mods), row.key.name()),
                        key,
                        mods,
                    });
                }
            }
        }
        found.sort_by_key(|a| a.text.to_lowercase());
        found
    }
}

/// `Shift+`, `Ctrl+` and `Alt+` for the modifiers, in this order (`ChangeKeyList`).
pub(super) fn modifier_prefix(mods: usize) -> String {
    let mut text = String::new();
    for (bit, name) in [(SHIFT, "Shift+"), (CTRL, "Ctrl+"), (ALT, "Alt+")] {
        if mods & bit != 0 {
            text.push_str(name);
        }
    }
    text
}
