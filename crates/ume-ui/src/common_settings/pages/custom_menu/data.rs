//! The 25 menus as one list of numbers (the value of the [`Kind::Numbers`](super::super::super::model::Kind)
//! control that holds them), and how that list is read from and written to `[CustMenu]`.
//!
//! A menu is `popup, name length, the UTF-16 units of the name, item count`, then `function, key` for each
//! item (the key is the character's code, 0 for none).

use ume_commands::macro_names;
use ume_config::settings::CustomMenus;
use ume_config::settings::custom_menu::{
    CustomMenu, CustomMenuItem, MAX_CUSTOM_MENU, MAX_CUSTOM_MENU_ITEMS,
};
use ume_config::{Profile, ProfileError};

/// `GetFunctionStrToFunctionCode`: the code of a macro name.
pub(super) fn code_of_name(name: &str) -> Option<u16> {
    macro_names::by_name(name).map(|f| f.code)
}

/// `GetFuncInfoByID`: the macro name a `.mnu` file writes for a function.
pub(super) fn name_of_code(code: u16) -> Option<String> {
    if code == 0 {
        return None;
    }
    macro_names::by_code(code).map(|f| f.name.to_owned())
}

pub(super) fn encode(menus: &[CustomMenu]) -> Vec<i32> {
    let mut out = Vec::new();
    for menu in menus.iter().take(MAX_CUSTOM_MENU) {
        let units: Vec<u16> = menu.name.encode_utf16().collect();
        out.push(i32::from(menu.popup));
        out.push(units.len() as i32);
        out.extend(units.into_iter().map(i32::from));
        out.push(menu.items.len() as i32);
        for item in &menu.items {
            out.push(i32::from(item.function));
            out.push(item.key.map_or(0, |c| c as i32));
        }
    }
    out
}

/// The menus of a list made by [`encode`]. A list that is cut short gives empty menus for the rest.
pub(super) fn decode(data: &[i32]) -> Vec<CustomMenu> {
    let mut at = 0;
    let mut next = || {
        let value = data.get(at).copied();
        at += 1;
        value
    };
    let mut menus = Vec::new();
    while menus.len() < MAX_CUSTOM_MENU {
        let Some(popup) = next() else { break };
        let units: Vec<u16> = (0..next().unwrap_or(0).max(0))
            .map_while(|_| next().and_then(|u| u16::try_from(u).ok()))
            .collect();
        let count = usize::try_from(next().unwrap_or(0))
            .unwrap_or(0)
            .min(MAX_CUSTOM_MENU_ITEMS);
        let items = (0..count)
            .map_while(|_| {
                let function = u16::try_from(next()?).ok()?;
                let key = u32::try_from(next()?).ok().and_then(char::from_u32);
                Some(CustomMenuItem {
                    function,
                    key: key.filter(|&c| c != '\0'),
                })
            })
            .collect();
        menus.push(CustomMenu {
            name: String::from_utf16_lossy(&units),
            items,
            popup: popup != 0,
        });
    }
    menus
}

/// The menus as a [`CustomMenus`], with the limits applied.
pub(super) fn menus_of(data: &[i32]) -> CustomMenus {
    CustomMenus::from_menus(decode(data))
}

/// The list of numbers for `menus`.
pub(super) fn data_of(menus: &CustomMenus) -> Vec<i32> {
    encode(menus.menus())
}

/// What the file gives (`IO_CustMenu` in reading mode, over Sakura's defaults).
pub(super) fn load(profile: &Profile) -> Vec<i32> {
    data_of(&CustomMenus::from_profile(profile, &code_of_name))
}

/// `IO_CustMenu` in writing mode. Nothing is written when the menus are the ones the file already gives, so
/// that the file keeps its bytes (and Sakura's defaults are not written out).
pub(super) fn store(data: &[i32], profile: &mut Profile) -> Result<bool, ProfileError> {
    let menus = menus_of(data);
    if CustomMenus::from_profile(profile, &code_of_name) == menus {
        return Ok(false);
    }
    menus.write_to_profile(profile, None)?;
    Ok(true)
}
