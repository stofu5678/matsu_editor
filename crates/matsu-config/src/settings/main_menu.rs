// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CommonSetting.h (CMainMenu, CommonSetting_MainMenu), sakura_core/env/CShareData_IO.cpp
//   (ShareData_IO_MainMenu, IO_MainMenu, GetFunctionStrToFunctionCode), sakura_core/prop/CPropComMainMenu.cpp
//   (SetData, GetData, GetDataTree, SupplementAmpersand, RemoveAmpersand),
//   sakura_core/typeprop/CImpExpManager.cpp (CImpExpMainMenu)
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2010, Uchi
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The main menu (メインメニュー): the section `[MainMenu]`.
//!
//! Sakura keeps the menu as a flat list of rows in depth-first order (`m_cMainMenuTbl`): each row has a level,
//! a type (node, function, separator or special list), a function code, an access key and, for a node whose
//! name was edited, the name. [`MainMenu::read_over`] reads them the way `CShareData_IO::IO_MainMenu` does and
//! [`MainMenu::from_profile`] adds the migration of `ShareData_IO_MainMenu` (the items that newer versions
//! added to a menu an older version wrote). [`MainMenu::write_to_profile`] writes them back;
//! [`MainMenu::import_profile`] and [`MainMenu::export_profile`] are the `.ini` files of the common settings
//! (`CImpExpMainMenu`). [`MainMenu::edit_rows`] and [`MainMenu::from_edited`] are what the property page
//! does when it opens (`SetData`) and when OK is pressed (`GetData`).
//!
//! The default menu (Sakura's `MainMenu.ini`) is a table of the application, not of this crate: the readers
//! take it as an argument.
//!
//! Differences from Sakura: a row whose type is not 0 to 3 is read as a function (Sakura's menus ignore it);
//! a negative level is 0; a line that is missing is a blank row, where Sakura reuses the text of the line
//! before it; `nMainMenuNum` of 0 or less leaves the default.

use super::custom_menu::parse_function;
use crate::profile::{Profile, ProfileError};
use crate::value::parse_long;

/// The section of the file.
pub const SECTION: &str = "MainMenu";
/// `MAX_MAINMENU` (`sakura_core/config/maxdata.h`): the rows.
pub const MAX_MAINMENU: usize = 500;
/// `MAX_MAINMENU_TOP`: the menus of the menu bar.
pub const MAX_MAINMENU_TOP: usize = 20;
/// `MAX_MAIN_MENU_NAME_LEN`, in UTF-16 units.
pub const MAX_MAIN_MENU_NAME_LEN: usize = 40;
/// `WSTR_MAINMENU_HEAD_V1` (`CImpExpManager.cpp`): the version an exported file names in `[Info]`.
pub const MNU_VERSION: &str = "SakuraEditorMainMenu Ver1";
const INFO_SECTION: &str = "Info";
/// `nCurrentVer` of `ShareData_IO_MainMenu`.
pub const CURRENT_VERSION: i32 = 2;

/// `F_NODE`: a node whose name is the name of the row.
pub const F_NODE: u16 = 0;
/// `F_SEPARATOR`.
pub const F_SEPARATOR: u16 = 1;
/// `F_SPECIAL_FIRST` (`F_WINDOW_LIST`) and `F_SPECIAL_LAST` (`F_PLUGIN_LIST`): the lists filled in at run time.
pub const F_SPECIAL_FIRST: u16 = 29001;
pub const F_SPECIAL_LAST: u16 = 29006;
/// `F_OPTION`: 共通設定, which the menu must keep.
pub const F_OPTION: u16 = 31112;

/// `EMainMenuType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// `T_NODE`: a submenu.
    Node,
    /// `T_LEAF`: a function.
    Leaf,
    /// `T_SEPARATOR`.
    Separator,
    /// `T_SPECIAL`: a list filled in at run time.
    Special,
}

impl RowKind {
    /// The number the file holds.
    pub fn number(self) -> i32 {
        match self {
            RowKind::Node => 0,
            RowKind::Leaf => 1,
            RowKind::Separator => 2,
            RowKind::Special => 3,
        }
    }

    /// The type of a number in the file; anything else is read as a function.
    pub fn from_number(number: i32) -> Self {
        match number {
            0 => RowKind::Node,
            2 => RowKind::Separator,
            3 => RowKind::Special,
            _ => RowKind::Leaf,
        }
    }
}

/// One row of the menu (`CMainMenu`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuRow {
    /// 0 is a menu of the menu bar.
    pub level: usize,
    pub kind: RowKind,
    /// The Sakura function code. A node with a code other than [`F_NODE`] takes its name from the function's
    /// label when its own name is empty.
    pub function: u16,
    /// The access key (`m_sKey`); `None` for none.
    pub key: Option<char>,
    /// The name as the file holds it, where `&&` is a literal `&` (`m_sName`). Only a node uses it.
    pub name: String,
}

impl MenuRow {
    /// The row `IO_MainMenu` starts from when it reads a line.
    fn blank() -> Self {
        Self {
            level: 0,
            kind: RowKind::Node,
            function: F_NODE,
            key: None,
            name: String::new(),
        }
    }

    /// A function row with no access key.
    pub fn leaf(level: usize, function: u16) -> Self {
        Self {
            level,
            kind: RowKind::Leaf,
            function,
            ..Self::blank()
        }
    }

    /// A separator.
    pub fn separator(level: usize) -> Self {
        Self {
            level,
            kind: RowKind::Separator,
            function: F_SEPARATOR,
            ..Self::blank()
        }
    }
}

/// `CommonSetting_MainMenu`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MainMenu {
    pub rows: Vec<MenuRow>,
    /// `m_bMainMenuKeyParentheses`: the access key is shown in parentheses after the name (`File(F)`).
    pub key_parentheses: bool,
}

/// Cuts `name` to `MAX_MAIN_MENU_NAME_LEN` UTF-16 units (the size of the buffer).
fn cut_name(name: &str) -> String {
    let mut units = 0;
    name.chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= MAX_MAIN_MENU_NAME_LEN
        })
        .collect()
}

/// The part of `text` before the first comma, and the part after it.
fn split_comma(text: &str) -> (&str, Option<&str>) {
    match text.split_once(',') {
        Some((head, rest)) => (head, Some(rest)),
        None => (text, None),
    }
}

/// One line `MM[nnn]` (`IO_MainMenu` in reading mode): `level,type,function,key,name`. A line that stops early
/// leaves the rest as a blank row has it.
fn parse_row(line: &str, by_name: &dyn Fn(&str) -> Option<u16>) -> MenuRow {
    let mut row = MenuRow::blank();
    let (level, rest) = split_comma(line);
    row.level = parse_long(level).map_or(0, |n| usize::try_from(n).unwrap_or(0));
    let Some(rest) = rest else { return row };
    let (kind, rest) = split_comma(rest);
    row.kind = RowKind::from_number(parse_long(kind).unwrap_or(0));
    let Some(rest) = rest else { return row };
    let (function, rest) = split_comma(rest);
    row.function = parse_function(function, by_name);
    let Some(rest) = rest else { return row };
    // The access key: a `,` is "no key" unless another `,` follows (then it is the key `,`).
    let after_key = match rest.chars().next() {
        None => return row,
        Some(',') if rest[1..].starts_with(',') => {
            row.key = Some(',');
            &rest[1..]
        }
        Some(',') => rest,
        Some(key) => {
            row.key = Some(key);
            &rest[key.len_utf8()..]
        }
    };
    // The character after the key (the comma) is skipped, and the rest is the name.
    let mut after = after_key.chars();
    if after.next().is_some() {
        row.name = cut_name(after.as_str());
    }
    row
}

/// The line of a row: `level,type,function,key,name` (the name only for a node that has its own).
fn row_text(row: &MenuRow, name_of: Option<&dyn Fn(u16) -> Option<String>>) -> String {
    let function = name_of
        .filter(|_| row.function > F_SEPARATOR)
        .and_then(|name_of| name_of(row.function))
        .unwrap_or_else(|| row.function.to_string());
    let name = if row.function == F_NODE {
        row.name.as_str()
    } else {
        ""
    };
    let key = row.key.map(String::from).unwrap_or_default();
    format!(
        "{},{},{},{},{}",
        row.level,
        row.kind.number(),
        function,
        key,
        name
    )
}

fn row_key(index: usize) -> String {
    format!("MM[{index:03}]")
}

/// One item that a newer version added (`SMainMenuAddItemInfo`).
struct Addition {
    version: i32,
    function: u16,
    after: u16,
    separator_before: bool,
}

impl Addition {
    const fn new(version: i32, function: u16, after: u16, separator_before: bool) -> Self {
        Self {
            version,
            function,
            after,
            separator_before,
        }
    }
}

/// `addInfos` of `ShareData_IO_MainMenu`. The access keys it names are not used: the rows are added with none.
const ADDITIONS: [Addition; 21] = [
    Addition::new(1, 30110, 30101, false), // F_FILENEW_NEWWINDOW after F_FILENEW
    Addition::new(1, 31010, 31431, false), // F_CHG_CHARSET after F_TOGGLE_KEY_SEARCH
    Addition::new(1, 31010, 30185, false), // F_CHG_CHARSET after F_VIEWMODE
    Addition::new(1, 30122, 30113, false), // F_FILE_REOPEN_LATIN1 after F_FILE_REOPEN_EUC
    Addition::new(1, 30122, 30119, false), // F_FILE_REOPEN_LATIN1 after F_FILE_REOPEN
    Addition::new(1, 30613, 30612, false), // F_COPY_COLOR_HTML after F_COPYLINESWITHLINENUMBER
    Addition::new(1, 30614, 30613, false), // F_COPY_COLOR_HTML_LINENUMBER after F_COPY_COLOR_HTML
    Addition::new(1, 30912, 30910, false), // F_GREP_REPLACE_DLG after F_GREP_DIALOG
    Addition::new(1, 30990, 30930, false), // F_FILETREE after F_OUTLINE
    Addition::new(1, 30990, 30931, false), // F_FILETREE after F_OUTLINE_TOGGLE
    Addition::new(1, 31104, 31102, false), // F_SHOWMINIMAP after F_SHOWSTATUSBAR
    Addition::new(1, 31104, 31103, false), // F_SHOWMINIMAP after F_SHOWTAB
    Addition::new(1, 31104, 31101, false), // F_SHOWMINIMAP after F_SHOWFUNCKEY
    Addition::new(1, 31104, 31100, false), // F_SHOWMINIMAP after F_SHOWTOOLBAR
    Addition::new(1, 30988, 30372, true),  // F_FUNCLIST_NEXT after F_JUMPHIST_SET, with a separator
    Addition::new(1, 30989, 30988, false), // F_FUNCLIST_PREV after F_FUNCLIST_NEXT
    Addition::new(1, 30393, 30989, false), // F_MODIFYLINE_NEXT after F_FUNCLIST_PREV
    Addition::new(1, 30394, 30393, false), // F_MODIFYLINE_PREV after F_MODIFYLINE_NEXT
    Addition::new(1, 30484, 30451, true), // F_MODIFYLINE_NEXT_SEL after F_GOFILEEND_SEL, with a separator
    Addition::new(1, 30485, 30484, false), // F_MODIFYLINE_PREV_SEL after F_MODIFYLINE_NEXT_SEL
    Addition::new(2, 31337, 31370, false), // F_DLGWINLIST after F_WIN_OUTPUT
];

impl MainMenu {
    /// What the file gives over `default` (Sakura's menu): the section as `IO_MainMenu` reads it, and the
    /// items that newer versions added, as `ShareData_IO_MainMenu` puts them in. Without a menu in the file the
    /// result is `default`. `by_name` gives the code of a macro name.
    pub fn from_profile(
        profile: &Profile,
        by_name: &dyn Fn(&str) -> Option<u16>,
        default: &MainMenu,
    ) -> Self {
        let mut menu = default.clone();
        menu.read_over(profile, by_name);
        // The version of the menu in the file: a file that has the menu but no version is version 0.
        let version = profile.get_i32(SECTION, "nMainMenuVer").unwrap_or_else(|| {
            if profile.get_i32(SECTION, "nMainMenuNum").is_some() {
                0
            } else {
                CURRENT_VERSION
            }
        });
        if version < CURRENT_VERSION {
            menu.add_newer_items(version);
        }
        menu
    }

    /// The menu as the file holds it, without the items of newer versions; `None` if the file has no menu.
    pub fn read_raw(profile: &Profile, by_name: &dyn Fn(&str) -> Option<u16>) -> Option<Self> {
        let mut menu = Self::default();
        menu.read_over(profile, by_name).then_some(menu)
    }

    /// `IO_MainMenu` in reading mode: replaces the rows (and the access key setting, if the file has it) with
    /// the file's. Returns `false`, and changes nothing, when the file has no menu (`nMainMenuNum` is missing or
    /// not positive).
    pub fn read_over(&mut self, profile: &Profile, by_name: &dyn Fn(&str) -> Option<u16>) -> bool {
        let count = profile
            .get_i32(SECTION, "nMainMenuNum")
            .and_then(|n| usize::try_from(n).ok())
            .filter(|&n| n > 0);
        let Some(count) = count else { return false };
        let count = count.min(MAX_MAINMENU);
        if let Some(on) = profile.get_bool(SECTION, "bKeyParentheses") {
            self.key_parentheses = on;
        }
        self.rows = (0..count)
            .map(|i| match profile.get(SECTION, &row_key(i)) {
                Some(line) => parse_row(line, by_name),
                None => MenuRow::blank(),
            })
            .collect();
        true
    }

    /// `ShareData_IO_MainMenu`: puts the items that versions after `version` added into the menu, each after
    /// the item it follows (the first row of that function deeper than the menu bar), unless the menu already
    /// has the function anywhere.
    fn add_newer_items(&mut self, version: i32) {
        for add in ADDITIONS.iter().filter(|a| a.version > version) {
            let extra = usize::from(add.separator_before);
            if self.rows.iter().any(|r| r.function == add.function)
                || self.rows.len() + extra >= MAX_MAINMENU
            {
                continue;
            }
            let Some(at) = self
                .rows
                .iter()
                .position(|r| r.function == add.after && r.level > 0)
            else {
                continue;
            };
            let level = self.rows[at].level;
            let mut added = Vec::new();
            if add.separator_before {
                added.push(MenuRow::separator(level));
            }
            added.push(MenuRow::leaf(level, add.function));
            self.rows.splice(at + 1..at + 1, added);
        }
    }

    /// `IO_MainMenu` in writing mode: `nMainMenuNum`, `bKeyParentheses` and a line for each row, over what
    /// `profile` has; lines past the last row are removed, and the rest of the file stays. With `version`,
    /// `nMainMenuVer` is written too. A function is written as its code, or as the name `name_of` gives for it
    /// (`bOutCmdName`, which the exported files use).
    ///
    /// `previous` is what [`MainMenu::read_raw`] gave before: a row equal to the row it has at its place keeps
    /// the text the file has. That text may be one that reading turns into something else (a plug-in command, a
    /// macro name MATSU does not know), which writing the row would lose.
    pub fn write_to_profile(
        &self,
        profile: &mut Profile,
        name_of: Option<&dyn Fn(u16) -> Option<String>>,
        previous: Option<&MainMenu>,
        version: Option<i32>,
    ) -> Result<(), ProfileError> {
        if let Some(version) = version {
            profile.set_i32(SECTION, "nMainMenuVer", version)?;
        }
        profile.set_i32(SECTION, "nMainMenuNum", self.rows.len() as i32)?;
        profile.set_i32(SECTION, "bKeyParentheses", i32::from(self.key_parentheses))?;
        for (i, row) in self.rows.iter().enumerate() {
            let key = row_key(i);
            let kept = previous.and_then(|p| p.rows.get(i)) == Some(row)
                && profile.get(SECTION, &key).is_some();
            if !kept {
                profile.set(SECTION, &key, &row_text(row, name_of))?;
            }
        }
        let stale: Vec<String> = profile
            .entries(SECTION)
            .filter_map(|(key, _)| {
                let index = key.strip_prefix("MM[")?.strip_suffix(']')?;
                (index.parse::<usize>().ok()? >= self.rows.len()).then(|| key.to_owned())
            })
            .collect();
        for key in stale {
            profile.remove(SECTION, &key);
        }
        Ok(())
    }

    /// `CImpExpMainMenu::Export`: the contents of the file, a profile with the version in `[Info]`.
    pub fn export_profile(
        &self,
        name_of: &dyn Fn(u16) -> Option<String>,
    ) -> Result<Profile, ProfileError> {
        let mut profile = Profile::new();
        profile.set(INFO_SECTION, "MENU_VERSION", MNU_VERSION)?;
        self.write_to_profile(&mut profile, Some(name_of), None, None)?;
        Ok(profile)
    }

    /// `CImpExpMainMenu::Import`: reads an exported file over this menu. Returns `false`, and changes nothing,
    /// when the file is not a main menu file of this version. A file with no menu in it is accepted and leaves
    /// the menu as it is (`IO_MainMenu` returns early).
    pub fn import_profile(
        &mut self,
        profile: &Profile,
        by_name: &dyn Fn(&str) -> Option<u16>,
    ) -> bool {
        if profile.get(INFO_SECTION, "MENU_VERSION") != Some(MNU_VERSION) {
            return false;
        }
        self.read_over(profile, by_name);
        true
    }

    /// `CPropMainMenu::SetData`: the rows as the page shows them, in a tree whose levels are the depth. A row
    /// deeper than one level below the row before it becomes its child (a function row cannot have children, so
    /// it becomes a sibling). The name of a node whose name is its own has its `&&` turned into `&`
    /// (`RemoveAmpersand`); the other rows have no name (a node that takes its name from its function shows the
    /// function's label).
    pub fn edit_rows(&self) -> Vec<MenuRow> {
        let mut out: Vec<MenuRow> = Vec::with_capacity(self.rows.len());
        let mut depth = 0;
        for row in &self.rows {
            let level = match out.last() {
                None => 0,
                Some(previous) if row.level > depth && previous.kind == RowKind::Node => depth + 1,
                Some(_) => row.level.min(depth),
            };
            depth = level;
            let name = if row.kind == RowKind::Node && row.function == F_NODE {
                remove_ampersand(&row.name)
            } else {
                String::new()
            };
            out.push(MenuRow {
                level,
                name,
                ..row.clone()
            });
        }
        out
    }

    /// `CPropMainMenu::GetData` and `GetDataTree`: the menu the page's rows make. The type of a row follows its
    /// function; only the first [`MAX_MAINMENU_TOP`] menus of the menu bar and [`MAX_MAINMENU`] rows are kept
    /// (the rows past the limit are dropped); a function row cannot have children, so the rows under it are
    /// dropped. When there is no 共通設定 ([`F_OPTION`]) in the menu, it is added: at the end of a new menu
    /// named `auto_add`, or, if the menu bar is full, at the end of the menu.
    pub fn from_edited(rows: &[MenuRow], key_parentheses: bool) -> Self {
        let mut out: Vec<MenuRow> = Vec::new();
        let mut tops = 0;
        let mut option = false;
        let mut full = false;
        // The kind of each open ancestor: only a node can have children, and a row that was left out has no
        // children either.
        let mut open: Vec<RowKind> = Vec::new();
        for row in rows {
            open.truncate(row.level);
            if open.len() != row.level || open.last().is_some_and(|kind| *kind != RowKind::Node) {
                continue;
            }
            if out.len() >= MAX_MAINMENU {
                full = true;
                break;
            }
            if row.level == 0 {
                if tops >= MAX_MAINMENU_TOP {
                    continue;
                }
                tops += 1;
            }
            let (kind, name) = match row.function {
                F_NODE => (RowKind::Node, cut_name(&supplement_ampersand(&row.name))),
                F_SEPARATOR => (RowKind::Separator, String::new()),
                _ if row.kind == RowKind::Node => (RowKind::Node, String::new()),
                F_SPECIAL_FIRST..=F_SPECIAL_LAST => (RowKind::Special, String::new()),
                _ => {
                    option |= row.function == F_OPTION;
                    (RowKind::Leaf, String::new())
                }
            };
            open.push(kind);
            out.push(MenuRow {
                level: row.level,
                kind,
                function: row.function,
                key: row.key,
                name,
            });
        }
        if !full && !option {
            if tops < MAX_MAINMENU_TOP && out.len() + 1 < MAX_MAINMENU {
                out.push(MenuRow {
                    name: "auto_add".to_owned(),
                    ..MenuRow::blank()
                });
            }
            if out.len() < MAX_MAINMENU {
                out.push(MenuRow::leaf(1, F_OPTION));
            }
        }
        Self {
            rows: out,
            key_parentheses,
        }
    }
}

/// `SupplementAmpersand`: a `&` that is not part of `&&` becomes `&&`.
pub fn supplement_ampersand(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '&' {
            if chars.peek() == Some(&'&') {
                chars.next();
            }
            out.push('&');
        }
    }
    out
}

/// `RemoveAmpersand`: `&&` becomes `&`.
pub fn remove_ampersand(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '&' && chars.peek() == Some(&'&') {
            chars.next();
        }
    }
    out
}

#[cfg(test)]
mod tests;
