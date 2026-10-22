// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CommonSetting.h (CommonSetting_CustomMenu), sakura_core/env/CShareData.cpp (InitPopupMenu),
//   sakura_core/env/CShareData_IO.cpp (IO_CustMenu, GetFunctionStrToFunctionCode),
//   sakura_core/env/CDataProfile.h (TryParse), sakura_core/func/CFuncLookup.cpp (Custmenu2Name)
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
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

//! The custom menus (カスタムメニュー): the section `[CustMenu]`.
//!
//! Sakura keeps 25 menus in the shared data (`m_Common.m_sCustomMenu`): number 0 is the right-click menu,
//! 1 to 23 are the custom menus a key or a toolbar button opens, and 24 is the tab menu. Each has a name, a
//! flag for "shown as a submenu in other menus", and up to 48 items: a function and an access key, or a
//! separator. [`CustomMenus::from_profile`] reads them the way `CShareData_IO::IO_CustMenu` does, over the
//! defaults of `CShareData::InitPopupMenu`; [`CustomMenus::write_to_profile`] writes them back, and
//! [`CustomMenus::import_profile`] and [`CustomMenus::export_profile`] are the `.mnu` files of the
//! common settings (`CImpExpCustMenu`).

use crate::profile::{Profile, ProfileError};
use crate::value::{parse_bool, parse_long};

/// The section of the file.
pub const SECTION: &str = "CustMenu";
/// `MAX_CUSTOM_MENU` (`sakura_core/env/CommonSetting.h`).
pub const MAX_CUSTOM_MENU: usize = 25;
/// `MAX_CUSTOM_MENU_NAME_LEN`, in UTF-16 units.
pub const MAX_CUSTOM_MENU_NAME_LEN: usize = 32;
/// `MAX_CUSTOM_MENU_ITEMS`.
pub const MAX_CUSTOM_MENU_ITEMS: usize = 48;
/// `CUSTMENU_INDEX_FOR_RBUTTONUP` (`sakura_core/_main/global.h`): the right-click menu.
pub const INDEX_FOR_RBUTTONUP: usize = 0;
/// `CUSTMENU_INDEX_FOR_TABWND`: the menu of the tab bar.
pub const INDEX_FOR_TABWND: usize = 24;
/// `WSTR_CUSTMENU_HEAD_V2` (`CImpExpManager.cpp`): the version a `.mnu` file names in `[Info]`.
pub const MNU_VERSION: &str = "SakuraEditorMenu_Ver2";
const MNU_INFO_SECTION: &str = "Info";

/// One row of a custom menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomMenuItem {
    /// The Sakura function code. `0` (`F_0`) is a separator; so is a name that no function has
    /// (`F_DEFAULT`, which is also 0).
    pub function: u16,
    /// The access key (`m_nCustMenuItemKeyArr`); `None` for `'\0'`.
    pub key: Option<char>,
}

impl CustomMenuItem {
    pub const SEPARATOR: Self = Self {
        function: 0,
        key: None,
    };

    pub fn is_separator(&self) -> bool {
        self.function == 0
    }
}

/// One custom menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomMenu {
    /// The name as set (`m_szCustMenuNameArr`); empty means the default name ([`CustomMenus::name`]).
    pub name: String,
    pub items: Vec<CustomMenuItem>,
    /// `m_bCustMenuPopupArr`: another menu that has this function shows it as a submenu.
    pub popup: bool,
}

impl CustomMenu {
    fn empty() -> Self {
        Self {
            name: String::new(),
            items: Vec::new(),
            popup: true,
        }
    }

    fn with_items(rows: &[(u16, char)]) -> Self {
        Self {
            items: rows
                .iter()
                .map(|&(function, key)| CustomMenuItem {
                    function,
                    key: (key != '\0').then_some(key),
                })
                .collect(),
            ..Self::empty()
        }
    }
}

/// `CShareData::InitPopupMenu`: `(function code, access key)`; code 0 is a separator. The comments name the
/// functions.
mod defaults {
    /// The two UME-only items of the default right-click menu, which follow `F_COPY_CRLF`.
    pub(super) const UME_COPIES: [(u16, char); 2] = [
        (32700, 'N'), // F_COPY_LF
        (32701, 'M'), // F_COPY_CR
    ];
    const AFTER_COPY_CRLF: u16 = 30603; // F_COPY_CRLF

    /// Sakura's right-click menu with `UME_COPIES` right after 「CRLF改行でコピー」. N and M are free in it.
    pub(super) fn right_click_with_ume_copies() -> Vec<(u16, char)> {
        let mut items = RIGHT_CLICK.to_vec();
        let at = items
            .iter()
            .position(|&(function, _)| function == AFTER_COPY_CRLF)
            .expect("Sakura's right-click menu has F_COPY_CRLF")
            + 1;
        items.splice(at..at, UME_COPIES);
        items
    }

    /// The right-click menu (custom menu 0), as Sakura has it.
    pub(super) const RIGHT_CLICK: &[(u16, char)] = &[
        (30210, 'U'),  // F_UNDO
        (30211, 'R'),  // F_REDO
        (0, '\0'),     // F_0
        (30601, 'T'),  // F_CUT
        (30602, 'C'),  // F_COPY
        (30604, 'P'),  // F_PASTE
        (30221, 'D'),  // F_DELETE
        (0, '\0'),     // F_0
        (30603, 'L'),  // F_COPY_CRLF
        (30608, 'H'),  // F_COPY_ADDCRLF
        (30605, 'X'),  // F_PASTEBOX
        (0, '\0'),     // F_0
        (30401, 'A'),  // F_SELECTALL
        (0, '\0'),     // F_0
        (30940, 'G'),  // F_TAGJUMP
        (30941, 'B'),  // F_TAGJUMPBACK
        (0, '\0'),     // F_0
        (30610, '@'),  // F_COPYLINES
        (30611, '.'),  // F_COPYLINESASPASSAGE
        (0, '\0'),     // F_0
        (30622, 'F'),  // F_COPYFNAME
        (30620, '\\'), // F_COPYPATH
        (30631, 'O'),  // F_COPYDIRPATH
        (0, '\0'),     // F_0
        (30196, 'E'),  // F_OPEN_FOLDER_IN_EXPLORER
        (30197, 'W'),  // F_OPEN_COMMAND_PROMPT
        (30198, 'w'),  // F_OPEN_COMMAND_PROMPT_AS_ADMIN
        (31456, 'P'),  // F_OPEN_POWERSHELL
        (31457, 'p'),  // F_OPEN_POWERSHELL_AS_ADMIN
        (30190, 'F'),  // F_PROPERTY_FILE
    ];

    /// Custom menu 1.
    pub(super) const MENU_1: &[(u16, char)] = &[
        (30102, 'O'), // F_FILEOPEN
        (30103, 'S'), // F_FILESAVE
        (31340, 'N'), // F_NEXTWINDOW
        (30800, 'L'), // F_TOLOWER
        (30801, 'U'), // F_TOUPPER
        (0, '\0'),    // F_0
        (31320, 'C'), // F_WINCLOSE
    ];

    /// The tab menu (custom menu 24).
    pub(super) const TAB: &[(u16, char)] = &[
        (30103, 'S'),  // F_FILESAVE
        (30104, 'A'),  // F_FILESAVEAS_DIALOG
        (30105, 'R'),  // F_FILECLOSE
        (30107, 'L'),  // F_FILECLOSE_OPEN
        (31320, 'C'),  // F_WINCLOSE
        (30119, 'W'),  // F_FILE_REOPEN
        (0, '\0'),     // F_0
        (30622, 'F'),  // F_COPYFNAME
        (30620, '\0'), // F_COPYPATH
        (30631, '\0'), // F_COPYDIRPATH
        (0, '\0'),     // F_0
        (30196, 'F'),  // F_OPEN_FOLDER_IN_EXPLORER
        (30197, 'W'),  // F_OPEN_COMMAND_PROMPT
        (30198, 'w'),  // F_OPEN_COMMAND_PROMPT_AS_ADMIN
        (31456, 'P'),  // F_OPEN_POWERSHELL
        (31457, 'p'),  // F_OPEN_POWERSHELL_AS_ADMIN
        (0, '\0'),     // F_0
        (31380, 'G'),  // F_GROUPCLOSE
        (31388, 'O'),  // F_TAB_CLOSEOTHER
        (31389, 'H'),  // F_TAB_CLOSELEFT
        (31390, 'M'),  // F_TAB_CLOSERIGHT
        (0, '\0'),     // F_0
        (31383, '0'),  // F_TAB_MOVERIGHT
        (31384, '1'),  // F_TAB_MOVELEFT
        (31385, 'E'),  // F_TAB_SEPARATE
        (31386, 'X'),  // F_TAB_JOINTNEXT
        (31387, 'V'),  // F_TAB_JOINTPREV
    ];
}

/// The 25 custom menus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomMenus {
    menus: Vec<CustomMenu>,
}

impl Default for CustomMenus {
    /// Sakura's defaults (`CShareData::Init` and `InitPopupMenu`): the right-click menu, custom menu 1 and
    /// the tab menu have items; the others are empty. The one difference is in the right-click menu: the
    /// UME-only copies with LF and with CR (`F_COPY_LF`, `F_COPY_CR`) follow 「CRLF改行でコピー」
    /// (`defaults::right_click_with_ume_copies`).
    fn default() -> Self {
        let mut menus = vec![CustomMenu::empty(); MAX_CUSTOM_MENU];
        menus[INDEX_FOR_RBUTTONUP] =
            CustomMenu::with_items(&defaults::right_click_with_ume_copies());
        menus[1] = CustomMenu::with_items(defaults::MENU_1);
        menus[INDEX_FOR_TABWND] = CustomMenu::with_items(defaults::TAB);
        Self { menus }
    }
}

/// `TryParse<KEYCODE>`: the first character, which must be ASCII; an empty value is "no key".
fn parse_key(value: &str) -> Option<Option<char>> {
    match value.chars().next() {
        None => Some(None),
        Some(c) if c.is_ascii() => Some((c != '\0').then_some(c)),
        Some(_) => None,
    }
}

/// `GetFunctionStrToFunctionCode` (`CShareData_IO.cpp`): a number of one or two digits is the code itself;
/// a name with a `/` is a plug-in command (not supported here: `F_DEFAULT`); anything else is a macro
/// name, which `by_name` turns into a code. A name that is unknown gives 0 (`F_DEFAULT`), a separator.
pub(super) fn parse_function(value: &str, by_name: &dyn Fn(&str) -> Option<u16>) -> u16 {
    let bytes = value.as_bytes();
    let is_digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
    if value.contains('/') {
        0
    } else if is_digit(0) && (bytes.len() == 1 || is_digit(1)) {
        // `_wtol`
        parse_long(value)
            .and_then(|n| u16::try_from(n).ok())
            .unwrap_or(0)
    } else {
        by_name(value).unwrap_or(0)
    }
}

/// Cuts `name` to `MAX_CUSTOM_MENU_NAME_LEN` UTF-16 units (the size of the buffer).
fn cut_name(name: &str) -> String {
    let mut units = 0;
    name.chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= MAX_CUSTOM_MENU_NAME_LEN
        })
        .collect()
}

impl CustomMenus {
    /// Reads `[CustMenu]` over the defaults (`CShareData_IO::IO_CustMenu` in reading mode). `by_name` gives the
    /// code of a macro name (`nCMIF` may hold a name such as `Undo` instead of a number); `ume-config` does not
    /// know the function table.
    ///
    /// An entry that is missing keeps its default. A function that is not a number or a known name is 0 (a
    /// separator), as `F_DEFAULT` is in Sakura. The number of items is cut to 0..=48. A key that is not ASCII
    /// is ignored.
    pub fn from_profile(profile: &Profile, by_name: &dyn Fn(&str) -> Option<u16>) -> Self {
        let mut all = Self::default();
        // A file that has the right-click menu (`nCMIN[00]`) was written by Sakura or by UME before the
        // copies were added to the default: items it does not list are Sakura's own, not shifted by the two
        // new ones. A file without it gets UME's default.
        if profile
            .get_i32(SECTION, &format!("nCMIN[{INDEX_FOR_RBUTTONUP:02}]"))
            .is_some()
        {
            all.menus[INDEX_FOR_RBUTTONUP] = CustomMenu::with_items(defaults::RIGHT_CLICK);
        }
        all.read_over(profile, by_name);
        all
    }

    /// `IO_CustMenu` in reading mode: what `profile` has replaces what `self` has; the rest stays.
    fn read_over(&mut self, profile: &Profile, by_name: &dyn Fn(&str) -> Option<u16>) {
        for (i, menu) in self.menus.iter_mut().enumerate() {
            if let Some(name) = profile.get_string(SECTION, &format!("szCMN[{i:02}]")) {
                menu.name = cut_name(&name);
            }
            if let Some(popup) = profile
                .get(SECTION, &format!("bCMPOP[{i:02}]"))
                .and_then(parse_bool)
            {
                menu.popup = popup;
            }
            let count =
                profile
                    .get_i32(SECTION, &format!("nCMIN[{i:02}]"))
                    .map_or(menu.items.len(), |n| {
                        usize::try_from(n.max(0))
                            .unwrap_or(0)
                            .min(MAX_CUSTOM_MENU_ITEMS)
                    });
            menu.items.resize(count, CustomMenuItem::SEPARATOR);
            for (j, item) in menu.items.iter_mut().enumerate() {
                if let Some(value) = profile.get(SECTION, &format!("nCMIF[{i:02}][{j:02}]")) {
                    item.function = parse_function(value, by_name);
                }
                if let Some(key) = profile
                    .get(SECTION, &format!("nCMIK[{i:02}][{j:02}]"))
                    .and_then(parse_key)
                {
                    item.key = key;
                }
            }
        }
    }

    /// The menus as the common settings edit them: `menus` are menus 0 and up; missing ones are empty menus with the
    /// default name, extra ones are dropped. A name is cut to its limit and a menu to its 48 items, as reading does.
    pub fn from_menus(menus: impl IntoIterator<Item = CustomMenu>) -> Self {
        let mut menus: Vec<CustomMenu> = menus.into_iter().take(MAX_CUSTOM_MENU).collect();
        menus.resize(MAX_CUSTOM_MENU, CustomMenu::empty());
        for menu in &mut menus {
            menu.name = cut_name(&menu.name);
            menu.items.truncate(MAX_CUSTOM_MENU_ITEMS);
        }
        Self { menus }
    }

    /// All 25 menus in order.
    pub fn menus(&self) -> &[CustomMenu] {
        &self.menus
    }

    /// `IO_CustMenu` in writing mode: every name, flag and item of the 25 menus, over what `profile` has. The
    /// rest of the file stays. An item is written as its code, or as the name `name_of` gives for it
    /// (`bOutCmdName`, which the `.mnu` files use); keys of items that are gone are removed.
    ///
    /// `previous` is what reading the file gave before: a menu equal to its previous one is not written, and an
    /// item equal to the previous item at its place keeps the text the file has. That text may be one that reading
    /// turns into a separator (a plug-in command, a macro name UME does not know), which writing the item
    /// would lose.
    pub fn write_to_profile(
        &self,
        profile: &mut Profile,
        name_of: Option<&dyn Fn(u16) -> Option<String>>,
        previous: Option<&CustomMenus>,
    ) -> Result<(), ProfileError> {
        for (i, menu) in self.menus.iter().enumerate() {
            let before = previous.and_then(|p| p.menus.get(i));
            if before == Some(menu) {
                continue;
            }
            // Reading takes UME's default for the right-click menu while the file has no `nCMIN[00]`, and
            // Sakura's once it has: writing the count must therefore write every item, or the items left out
            // would change their meaning on the next read.
            let before = if i == INDEX_FOR_RBUTTONUP
                && profile
                    .get_i32(SECTION, &format!("nCMIN[{i:02}]"))
                    .is_none()
            {
                None
            } else {
                before
            };
            profile.set(SECTION, &format!("szCMN[{i:02}]"), &menu.name)?;
            profile.set(
                SECTION,
                &format!("bCMPOP[{i:02}]"),
                if menu.popup { "1" } else { "0" },
            )?;
            profile.set(
                SECTION,
                &format!("nCMIN[{i:02}]"),
                &menu.items.len().to_string(),
            )?;
            for (j, item) in menu.items.iter().enumerate() {
                if before.and_then(|b| b.items.get(j)) == Some(item) {
                    continue;
                }
                let function = name_of
                    .and_then(|name_of| name_of(item.function))
                    .unwrap_or_else(|| item.function.to_string());
                profile.set(SECTION, &format!("nCMIF[{i:02}][{j:02}]"), &function)?;
                let key = item.key.map(String::from).unwrap_or_default();
                profile.set(SECTION, &format!("nCMIK[{i:02}][{j:02}]"), &key)?;
            }
            for j in menu.items.len()..MAX_CUSTOM_MENU_ITEMS {
                profile.remove(SECTION, &format!("nCMIF[{i:02}][{j:02}]"));
                profile.remove(SECTION, &format!("nCMIK[{i:02}][{j:02}]"));
            }
        }
        Ok(())
    }

    /// `CImpExpCustMenu::Export`: the contents of a `.mnu` file, a profile with the version in `[Info]`.
    pub fn export_profile(
        &self,
        name_of: &dyn Fn(u16) -> Option<String>,
    ) -> Result<Profile, ProfileError> {
        let mut profile = Profile::new();
        profile.set(MNU_INFO_SECTION, "MENU_VERSION", MNU_VERSION)?;
        profile.set(
            MNU_INFO_SECTION,
            "MAX_CUSTOM_MENU",
            &MAX_CUSTOM_MENU.to_string(),
        )?;
        self.write_to_profile(&mut profile, Some(name_of), None)?;
        Ok(profile)
    }

    /// `CImpExpCustMenu::Import`: reads a `.mnu` file over these menus (what the file does not have stays).
    /// Returns `false`, and changes nothing, when the file is not a menu file of this version.
    pub fn import_profile(
        &mut self,
        profile: &Profile,
        by_name: &dyn Fn(&str) -> Option<u16>,
    ) -> bool {
        if profile.get(MNU_INFO_SECTION, "MENU_VERSION") != Some(MNU_VERSION) {
            return false;
        }
        self.read_over(profile, by_name);
        true
    }

    /// The menu `index` (0 to 24), or `None` out of range.
    pub fn get(&self, index: usize) -> Option<&CustomMenu> {
        self.menus.get(index)
    }

    /// The name of menu `index` as the menus show it (`CFuncLookup::Custmenu2Name`): the name that is set,
    /// else the default of the string table (`STR_CUSTMENU_RIGHT_CLICK`, `STR_CUSTMENU_CUSTOM`,
    /// `STR_CUSTMENU_TAB`). `None` out of range.
    pub fn name(&self, index: usize) -> Option<String> {
        let menu = self.menus.get(index)?;
        if !menu.name.is_empty() {
            return Some(menu.name.clone());
        }
        Some(match index {
            INDEX_FOR_RBUTTONUP => "右クリックメニュー".to_owned(),
            INDEX_FOR_TABWND => "タブメニュー".to_owned(),
            _ => format!("メニュー{index}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(name: &str) -> Option<u16> {
        match name {
            "Undo" => Some(30210),
            "FileOpen" => Some(30102),
            _ => None,
        }
    }

    fn read(text: &str) -> CustomMenus {
        CustomMenus::from_profile(&Profile::from_bytes(text.as_bytes()), &names)
    }

    #[test]
    fn the_defaults_are_sakuras() {
        let menus = CustomMenus::default();
        let counts: Vec<_> = (0..MAX_CUSTOM_MENU)
            .map(|i| menus.get(i).unwrap().items.len())
            .collect();
        // InitPopupMenu: 30 items in the right-click menu, 7 in custom menu 1, 27 in the tab menu.
        // UME's right-click menu has the two copies on top of Sakura's 30 (see
        // `the_default_right_click_menu_is_sakuras_with_the_two_copies`).
        assert_eq!((counts[0], counts[1], counts[24]), (30 + 2, 7, 27));
        assert!(counts[2..24].iter().all(|&n| n == 0));
        let right = menus.get(0).unwrap();
        assert_eq!(
            right.items[0],
            CustomMenuItem {
                function: 30210,
                key: Some('U')
            }
        );
        assert!(right.items[2].is_separator());
        assert!(
            (0..MAX_CUSTOM_MENU).all(|i| menus.get(i).unwrap().popup),
            "bCMPOP defaults to true"
        );
        assert!(menus.get(25).is_none());
    }

    /// The one difference of UME's default menus from Sakura's: LF and CR copies after the CRLF copy.
    #[test]
    fn the_default_right_click_menu_is_sakuras_with_the_two_copies() {
        let item = |(function, key): (u16, char)| CustomMenuItem {
            function,
            key: (key != '\0').then_some(key),
        };
        let mut expected: Vec<_> = defaults::RIGHT_CLICK.iter().copied().map(item).collect();
        let crlf = expected.iter().position(|i| i.function == 30603).unwrap();
        assert_eq!(
            expected[crlf + 1].function,
            30608,
            "Sakura: ADDCRLF follows"
        );
        expected.insert(
            crlf + 1,
            CustomMenuItem {
                function: 32700,
                key: Some('N'),
            },
        );
        expected.insert(
            crlf + 2,
            CustomMenuItem {
                function: 32701,
                key: Some('M'),
            },
        );
        assert_eq!(CustomMenus::default().get(0).unwrap().items, expected);
        // Sakura's menu has no other N or M.
        let keys: Vec<_> = defaults::RIGHT_CLICK.iter().map(|&(_, k)| k).collect();
        assert!(!keys.contains(&'N') && !keys.contains(&'M'));
        // The other default menus are Sakura's.
        let menus = CustomMenus::default();
        assert_eq!(menus.get(1).unwrap().items.len(), defaults::MENU_1.len());
        assert_eq!(menus.get(24).unwrap().items.len(), defaults::TAB.len());
    }

    #[test]
    fn a_file_with_the_right_click_menu_keeps_sakuras_items_where_it_lists_none() {
        // 30 items counted, one listed: the rest are Sakura's own defaults, not shifted by the two new ones.
        let menus = read("[CustMenu]\nnCMIN[00]=30\nnCMIF[00][00]=30102\n");
        let sakura: Vec<_> = defaults::RIGHT_CLICK.iter().map(|&(f, _)| f).collect();
        let got: Vec<_> = menus
            .get(0)
            .unwrap()
            .items
            .iter()
            .map(|i| i.function)
            .collect();
        assert_eq!(got[0], 30102);
        assert_eq!(got[1..], sakura[1..]);
        // A file with other menus only gets UME's default.
        let menus = read("[CustMenu]\nnCMIN[02]=1\n");
        assert_eq!(menus.get(0), CustomMenus::default().get(0));
        assert_eq!(menus.get(0).unwrap().items.len(), 32);
    }

    #[test]
    fn changing_the_right_click_menu_in_a_file_without_it_writes_every_item() {
        // Rename the right-click menu only: the count is written, so the items must be written too.
        let mut profile = Profile::from_bytes(b"");
        let before = CustomMenus::from_profile(&profile, &names);
        let mut menus = before.menus().to_vec();
        menus[0].name = "Mine".to_owned();
        let after = CustomMenus::from_menus(menus);
        after
            .write_to_profile(&mut profile, None, Some(&before))
            .unwrap();
        let reread = CustomMenus::from_profile(&profile, &names);
        assert_eq!(reread, after);
        assert_eq!(reread.get(0).unwrap().items.len(), 32);
        // Writing again over the file that now has the menu writes no more than what changed.
        let mut third = reread.menus().to_vec();
        third[0].popup = false;
        let third = CustomMenus::from_menus(third);
        third
            .write_to_profile(&mut profile, None, Some(&reread))
            .unwrap();
        assert_eq!(CustomMenus::from_profile(&profile, &names), third);
    }

    #[test]
    fn default_names_follow_the_string_table() {
        let menus = CustomMenus::default();
        assert_eq!(menus.name(0).unwrap(), "右クリックメニュー");
        assert_eq!(menus.name(3).unwrap(), "メニュー3");
        assert_eq!(menus.name(24).unwrap(), "タブメニュー");
        assert!(menus.name(25).is_none());
    }

    #[test]
    fn no_section_means_the_defaults() {
        assert_eq!(read(""), CustomMenus::default());
    }

    #[test]
    fn a_section_replaces_what_it_names() {
        let menus = read(
            "[CustMenu]\nszCMN[02]=My menu\nbCMPOP[02]=0\nnCMIN[02]=3\nnCMIF[02][00]=30102\n\
             nCMIK[02][00]=O\nnCMIF[02][01]=Undo\nnCMIK[02][01]=\nnCMIF[02][02]=0\n",
        );
        let menu = menus.get(2).unwrap();
        assert_eq!(menu.name, "My menu");
        assert_eq!(menus.name(2).unwrap(), "My menu");
        assert!(!menu.popup);
        assert_eq!(
            menu.items,
            vec![
                CustomMenuItem {
                    function: 30102,
                    key: Some('O')
                },
                CustomMenuItem {
                    function: 30210,
                    key: None
                },
                CustomMenuItem::SEPARATOR,
            ]
        );
        // The others keep their defaults.
        assert_eq!(menus.get(0), CustomMenus::default().get(0));
    }

    #[test]
    fn a_shorter_count_cuts_the_default_items() {
        let menus = read("[CustMenu]\nnCMIN[00]=2\n");
        let right = menus.get(0).unwrap();
        assert_eq!(right.items.len(), 2);
        assert_eq!(right.items[1].function, 30211);
        let none = read("[CustMenu]\nnCMIN[00]=0\n");
        assert!(none.get(0).unwrap().items.is_empty());
        let negative = read("[CustMenu]\nnCMIN[00]=-5\n");
        assert!(negative.get(0).unwrap().items.is_empty());
    }

    #[test]
    fn the_count_is_limited_to_48() {
        let menus = read("[CustMenu]\nnCMIN[05]=500\nnCMIF[05][47]=30210\n");
        let menu = menus.get(5).unwrap();
        assert_eq!(menu.items.len(), MAX_CUSTOM_MENU_ITEMS);
        assert_eq!(menu.items[47].function, 30210);
    }

    #[test]
    fn a_function_is_a_number_a_known_name_or_a_separator() {
        assert_eq!(parse_function("30210", &names), 30210);
        assert_eq!(parse_function("7", &names), 7);
        assert_eq!(parse_function("Undo", &names), 30210);
        assert_eq!(parse_function("NoSuchMacro", &names), 0);
        assert_eq!(parse_function("vendor/plugin", &names), 0);
        assert_eq!(parse_function("", &names), 0);
        // Out of the range of a function code.
        assert_eq!(parse_function("99999999", &names), 0);
    }

    #[test]
    fn a_key_is_one_ascii_character() {
        assert_eq!(parse_key(""), Some(None));
        assert_eq!(parse_key("U"), Some(Some('U')));
        assert_eq!(parse_key("\\"), Some(Some('\\')));
        assert_eq!(parse_key("あ"), None);
        let menus = read("[CustMenu]\nnCMIK[00][00]=あ\nnCMIK[00][01]=\n");
        let right = menus.get(0).unwrap();
        assert_eq!(
            right.items[0].key,
            Some('U'),
            "a non-ASCII key keeps the default"
        );
        assert_eq!(right.items[1].key, None, "an empty key is none");
    }

    #[test]
    fn a_name_is_cut_to_32_units() {
        let long = "あ".repeat(40);
        let menus = read(&format!("[CustMenu]\nszCMN[03]={long}\n"));
        assert_eq!(menus.get(3).unwrap().name.chars().count(), 32);
        // An empty name is the default name.
        let menus = read("[CustMenu]\nszCMN[03]=\n");
        assert_eq!(menus.name(3).unwrap(), "メニュー3");
    }

    #[test]
    fn the_reading_leaves_the_file_as_it_is() {
        let text = "[CustMenu]\nszCMN[01]=Mine\n";
        let profile = Profile::from_bytes(text.as_bytes());
        let _ = CustomMenus::from_profile(&profile, &names);
        assert_eq!(profile.to_bytes(), text.as_bytes());
    }

    fn name_of(code: u16) -> Option<String> {
        (code == 30210).then(|| "Undo".to_owned())
    }

    #[test]
    fn written_menus_read_back_and_the_rest_of_the_file_stays() {
        let mut menus = CustomMenus::default().menus().to_vec();
        menus[2] = CustomMenu {
            name: "あ名前".to_owned(),
            items: vec![
                CustomMenuItem {
                    function: 30210,
                    key: Some('U'),
                },
                CustomMenuItem::SEPARATOR,
            ],
            popup: false,
        };
        let menus = CustomMenus::from_menus(menus);
        let mut profile = Profile::from_bytes(
            b"[Other]\nx=1\n[CustMenu]\nnCMIN[02]=5\nnCMIF[02][04]=7\nnCMIK[02][04]=Z\n",
        );
        menus.write_to_profile(&mut profile, None, None).unwrap();
        assert_eq!(profile.get("Other", "x"), Some("1"));
        assert_eq!(profile.get(SECTION, "nCMIN[02]"), Some("2"));
        assert_eq!(profile.get(SECTION, "nCMIF[02][00]"), Some("30210"));
        assert_eq!(profile.get(SECTION, "nCMIK[02][01]"), Some(""));
        assert_eq!(profile.get(SECTION, "nCMIF[02][04]"), None, "left over");
        assert_eq!(profile.get(SECTION, "bCMPOP[02]"), Some("0"));
        assert_eq!(CustomMenus::from_profile(&profile, &names), menus);
    }

    #[test]
    fn from_menus_pads_cuts_and_limits() {
        let long = CustomMenu {
            name: "あ".repeat(40),
            items: vec![CustomMenuItem::SEPARATOR; 60],
            popup: true,
        };
        let menus = CustomMenus::from_menus(vec![long]);
        assert_eq!(menus.menus().len(), MAX_CUSTOM_MENU);
        assert_eq!(menus.get(0).unwrap().name.chars().count(), 32);
        assert_eq!(menus.get(0).unwrap().items.len(), MAX_CUSTOM_MENU_ITEMS);
        assert!(menus.get(1).unwrap().items.is_empty());
        assert_eq!(menus.name(1).unwrap(), "メニュー1");
    }

    #[test]
    fn a_mnu_file_has_the_version_and_names_for_the_functions() {
        let menus = CustomMenus::default();
        let file = menus.export_profile(&name_of).unwrap();
        assert_eq!(file.get("Info", "MENU_VERSION"), Some(MNU_VERSION));
        assert_eq!(file.get("Info", "MAX_CUSTOM_MENU"), Some("25"));
        // The right-click menu starts with F_UNDO, which has a name here; F_REDO does not.
        assert_eq!(file.get(SECTION, "nCMIF[00][00]"), Some("Undo"));
        assert_eq!(file.get(SECTION, "nCMIF[00][01]"), Some("30211"));
        let mut other = CustomMenus::from_menus(vec![]);
        assert!(other.import_profile(&file, &names));
        assert_eq!(other.get(0).unwrap().items, menus.get(0).unwrap().items);
    }

    #[test]
    fn an_import_keeps_what_the_file_lacks_and_refuses_another_version() {
        let mut menus = CustomMenus::default();
        let file = Profile::from_bytes(
            b"[Info]\nMENU_VERSION=SakuraEditorMenu_Ver2\n[CustMenu]\nszCMN[01]=Imported\n",
        );
        assert!(menus.import_profile(&file, &names));
        assert_eq!(menus.name(1).unwrap(), "Imported");
        assert_eq!(menus.get(1).unwrap().items.len(), 7, "items stay");
        let before = menus.clone();
        let wrong = Profile::from_bytes(b"[Info]\nMENU_VERSION=Other\n[CustMenu]\nszCMN[01]=X\n");
        assert!(!menus.import_profile(&wrong, &names));
        assert!(!menus.import_profile(&Profile::from_bytes(b"[CustMenu]\nszCMN[01]=X\n"), &names));
        assert_eq!(menus, before);
    }
}
