// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/plugin/CPlugin.h (`GetPluginFunctionCode`, `GetPluginId`, `GetPlugId`),
//   sakura_core/plugin/CJackManager.cpp (`GetCommandCode`, `GetCommandName`),
//   sakura_core/env/CShareData_IO.cpp (`GetPlugCmdInfoByName`, `GetPlugCmdInfoByFuncCode`),
//   sakura_core/config/maxdata.h
//
// Original copyright notices of those files:
//   Copyright (C) 2009, syat
//   Copyright (C) 2010, Uchi
//   Copyright (C) 2011, syat
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

//! The commands of the plugins (ADR-0026): their function codes, and the table that names them.
//!
//! Sakura gives the command number `plug` (1 to 49) of the plugin in slot `slot` (0 to 39 of the
//! plugin table) the function code
//! `slot % 20 * 100 + slot / 20 * 50 + plug + F_PLUGCOMMAND_FIRST` (`CPlug::GetPluginFunctionCode`),
//! so that the code of a command does not change while the plugin keeps its slot. The settings
//! files write such a function as `<plugin ID>/<plug, two digits>` (`GetPlugCmdInfoByFuncCode`) and
//! read that back (`GetPlugCmdInfoByName`).
//!
//! The table is global to the thread of the UI, because the places that read and write a function (the
//! key bindings, the menus, the toolbar) have no window to ask. The UI fills it from the settings
//! ([`set_slots`]) and from the plugins that are enabled ([`set_commands`]).

use std::cell::RefCell;

/// `F_PLUGCOMMAND_FIRST`.
pub const FIRST: u16 = 20000;
/// `F_PLUGCOMMAND_LAST`: the last code of a plugin command (the 49th command of the 40th plugin).
pub const LAST: u16 = 21999;
/// `MAX_PLUGIN`.
pub const MAX_PLUGIN: usize = 40;
/// `MAX_PLUG_CMD`: the commands of a plugin are numbered from 1 to one less than this.
pub const MAX_PLUG_CMD: usize = 50;

/// `CPlug::GetPluginFunctionCode`. `None` for a slot or a command number out of range.
pub fn function_code(slot: usize, plug: usize) -> Option<u16> {
    if slot >= MAX_PLUGIN || plug == 0 || plug >= MAX_PLUG_CMD {
        return None;
    }
    let code = slot % 20 * 100 + slot / 20 * 50 + plug + usize::from(FIRST);
    u16::try_from(code).ok()
}

/// `CPlug::GetPluginId` and `GetPlugId`: the slot and the command number of a function code.
/// `None` for a code that is not a plugin command. (Sakura stops below `F_PLUGCOMMAND_LAST`, which
/// leaves the 49th command of the 40th plugin, whose code is `F_PLUGCOMMAND_LAST`, with no plugin:
/// an off-by-one that is not meant, so the last code is included here.)
pub fn decode(code: u16) -> Option<(usize, usize)> {
    if !is_plugin_code(code) {
        return None;
    }
    let offset = usize::from(code - FIRST);
    let code = usize::from(code);
    let slot = offset / 100 + code % 100 / 50 * 20;
    let plug = code % 100 - code % 100 / 50 * 50;
    Some((slot, plug))
}

/// Whether `code` is in the range of the plugin commands.
pub fn is_plugin_code(code: u16) -> bool {
    (FIRST..=LAST).contains(&code)
}

/// A plugin command that is loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandInfo {
    pub code: u16,
    /// The slot of the plugin in the plugin table.
    pub slot: usize,
    /// The number of the command in the plugin (from 1).
    pub plug: usize,
    pub plugin_id: String,
    /// The name of the plugin (`Plugin.Name`, else the folder).
    pub plugin_name: String,
    /// The label of the command (`C[n].Label`, else the handler).
    pub label: String,
    /// The handler file, relative to the folder of the plugin.
    pub handler: String,
    /// `C[n].Icon`, as written.
    pub icon: String,
    /// Added by the plugin while it runs (`Plugin.AddCommand`), not written in `plugin.def`.
    pub added: bool,
}

#[derive(Default)]
struct Table {
    /// The ID of the plugin in each registered slot (`""`: nobody).
    slots: Vec<String>,
    /// The commands of the plugins that are enabled, in the order of their codes.
    commands: Vec<CommandInfo>,
}

thread_local! {
    // The UI thread is the one that reads and writes the settings files and runs the plugins, so the
    // table is of this thread (a test, which has a thread of its own, has a table of its own).
    static TABLE: RefCell<Table> = RefCell::new(Table::default());
}

fn read<T>(f: impl FnOnce(&Table) -> T) -> T {
    TABLE.with(|table| f(&table.borrow()))
}

fn write<T>(f: impl FnOnce(&mut Table) -> T) -> T {
    TABLE.with(|table| f(&mut table.borrow_mut()))
}

/// Sets the IDs of the registered plugins by slot (the plugin table of the settings).
pub fn set_slots(ids: Vec<String>) {
    write(|table| table.slots = ids);
}

/// Sets the commands of the plugins that are enabled. They are kept in the order of their codes.
pub fn set_commands(mut commands: Vec<CommandInfo>) {
    commands.sort_by_key(|c| c.code);
    write(|table| table.commands = commands);
}

/// Adds one command (`Plugin.AddCommand`), or replaces the one with the same code.
pub fn add_command(command: CommandInfo) {
    write(
        |table| match table.commands.iter_mut().find(|c| c.code == command.code) {
            Some(existing) => *existing = command,
            None => {
                table.commands.push(command);
                table.commands.sort_by_key(|c| c.code);
            }
        },
    );
}

/// The loaded commands, in the order of the list of the kind プラグイン.
pub fn commands() -> Vec<CommandInfo> {
    read(|table| table.commands.clone())
}

/// The loaded command with this function code.
pub fn info(code: u16) -> Option<CommandInfo> {
    read(|table| table.commands.iter().find(|c| c.code == code).cloned())
}

/// How many commands the plugin in `slot` has now.
pub fn count_in(slot: usize) -> usize {
    read(|table| table.commands.iter().filter(|c| c.slot == slot).count())
}

/// `GetPlugCmdInfoByFuncCode`: the name of a function code as the settings files write it,
/// `<plugin ID>/<two digits>`. The plugin has to be registered; it does not have to be enabled.
pub fn name_of(code: u16) -> Option<String> {
    let (slot, plug) = decode(code)?;
    read(|table| {
        let id = table.slots.get(slot).filter(|id| !id.is_empty())?;
        Some(format!("{id}/{plug:02}"))
    })
}

/// `GetPlugCmdInfoByName`: the function code of `<plugin ID>/<number>` for a command that is
/// loaded. `None` for anything else, also for a command of a plugin that is registered but not
/// enabled (the text stays as it is then).
pub fn by_name(text: &str) -> Option<u16> {
    let (id, number) = text.split_once('/')?;
    // `_wtoi`: the number at the start of the text, 0 when there is none.
    let digits: String = number.chars().take_while(char::is_ascii_digit).collect();
    let plug = digits.parse::<usize>().ok().filter(|&n| n > 0)?;
    read(|table| {
        let slot = table.slots.iter().position(|s| !s.is_empty() && s == id)?;
        let code = function_code(slot, plug)?;
        table
            .commands
            .iter()
            .any(|c| c.code == code)
            .then_some(code)
    })
}

/// The name of function `code` if it is a loaded plugin command (`CJackManager::GetCommandName`).
pub fn label(code: u16) -> Option<String> {
    read(|table| {
        table
            .commands
            .iter()
            .find(|c| c.code == code)
            .map(|c| c.label.clone())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_follow_the_table_of_sakura() {
        // CPlugin.h: 20 plugins fill 20000-20999 in steps of 100, the next 20 are 50 further on.
        assert_eq!(function_code(0, 1), Some(20001));
        assert_eq!(function_code(1, 1), Some(20101));
        assert_eq!(function_code(19, 49), Some(21949));
        assert_eq!(function_code(20, 1), Some(20051));
        assert_eq!(function_code(39, 49), Some(21999));
        assert_eq!(function_code(0, 0), None);
        assert_eq!(function_code(0, MAX_PLUG_CMD), None);
        assert_eq!(function_code(MAX_PLUGIN, 1), None);
    }

    #[test]
    fn a_code_is_decoded_to_the_slot_and_the_number_it_came_from() {
        for slot in 0..MAX_PLUGIN {
            for plug in 1..MAX_PLUG_CMD {
                let code = function_code(slot, plug).unwrap();
                assert_eq!(decode(code), Some((slot, plug)), "{slot} {plug}");
            }
        }
        assert_eq!(decode(19999), None);
        assert_eq!(decode(30103), None);
    }

    #[test]
    fn names_and_loaded_commands_are_found_by_each_other() {
        let mut ids = vec![String::new(); MAX_PLUGIN];
        ids[3] = "test.names".to_owned();
        set_slots(ids);
        let code = function_code(3, 2).unwrap();
        assert_eq!(name_of(code).as_deref(), Some("test.names/02"));
        // Registered, not loaded: the name is known, the text does not read.
        assert_eq!(by_name("test.names/02"), None);
        add_command(CommandInfo {
            code,
            slot: 3,
            plug: 2,
            plugin_id: "test.names".to_owned(),
            plugin_name: "Names".to_owned(),
            label: "Second".to_owned(),
            handler: "second.js".to_owned(),
            icon: String::new(),
            added: false,
        });
        assert_eq!(by_name("test.names/02"), Some(code));
        assert_eq!(by_name("test.names/2"), Some(code));
        assert_eq!(by_name("test.names/03"), None);
        assert_eq!(by_name("test.names/00"), None);
        assert_eq!(by_name("other/02"), None);
        assert_eq!(label(code).as_deref(), Some("Second"));
        assert_eq!(info(code).map(|c| c.plug), Some(2));
        assert_eq!(count_in(3), 1);
        set_commands(Vec::new());
        assert_eq!(by_name("test.names/02"), None);
        set_slots(Vec::new());
        assert_eq!(name_of(code), None);
    }
}
