//! The commands of the plugins in the editor (ADR-0026, the jack `Command`): which commands there are,
//! the list `F_PLUGIN_LIST` of the main menu, and running one from a menu, a key or a toolbar button.
//!
//! The function codes and the names are `matsu_commands::plugin_commands`, a table the settings files
//! are read and written with. This module fills it from the settings: a command is there while its plugin
//! is registered, plugins are switched on (`EnablePlugin`) and the user enabled the plugin with its
//! files as they are. `plugin.def` gives `[Command]`; a plugin can add more while it runs
//! (`Plugin.AddCommand`), and those stay until the plugin is switched off or removed.
//!
//! Running goes through `macros::plugin_call` (the resident host and its limits).

use gpui::{App, Context};
use matsu_commands::plugin_commands::{self, CommandInfo};
use matsu_config::plugins::consent::{Consent, ConsentFile};
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginSettings, PluginState, SearchDirs};

use crate::menu::{Entry, Item, Lists, MenuAction};
use crate::settings::UserSettings;
use crate::view::EditorView;

/// `F_PLUGIN_LIST`, the row of the main menu that stands for the commands of the plugins.
pub(crate) const F_PLUGIN_LIST: u16 = 29006;

/// The folders of the plugins for the settings file and the executable of this application.
pub(crate) fn search_dirs(cx: &App) -> Option<SearchDirs> {
    crate::doc_types::with_user(cx, dirs_of)
}

fn dirs_of(user: &UserSettings) -> Option<SearchDirs> {
    let settings = crate::settings::history_file(user)?;
    Some(SearchDirs::new(
        &settings,
        crate::support::help::exe_dir().as_deref(),
    ))
}

/// Makes the table of the commands from the settings now (see [`refresh_for`]). Returns whether the
/// commands changed.
pub(crate) fn refresh(cx: &App) -> bool {
    crate::doc_types::with_user(cx, refresh_for)
}

/// Makes the table of the commands from `user`. Call it when the application starts, before the key
/// map is made, and after the settings are written (the plugin table, the switch and what the user
/// enabled may have changed). Returns whether the commands changed.
pub(crate) fn refresh_for(user: &UserSettings) -> bool {
    let before = plugin_commands::commands();
    let profile = user.profile().cloned().unwrap_or_default();
    let settings = PluginSettings::from_profile(&profile);
    plugin_commands::set_slots(
        settings
            .table
            .iter()
            .map(|rec| {
                if rec.is_used() && rec.state != PluginState::Deleted {
                    rec.id.clone()
                } else {
                    String::new()
                }
            })
            .collect(),
    );
    let mut commands = Vec::new();
    if let Some(dirs) = dirs_of(user).filter(|_| settings.enabled) {
        let consent = ConsentFile::load(&dirs.consent_file());
        for (slot, rec) in settings.table.iter().enumerate() {
            if !rec.is_used() || rec.id.is_empty() {
                continue;
            }
            let Some(def) = dirs.load_def(&rec.name, DEFAULT_LANGUAGE) else {
                continue;
            };
            if def.id != rec.id || def.unsupported().is_some() {
                continue;
            }
            let dir = dirs.plugin_dir(&rec.name);
            if !matches!(
                consent.consent_for_dir(&def.id, dir.as_deref()),
                Consent::On
            ) {
                continue;
            }
            let plugin_name = if def.name.is_empty() {
                rec.name.clone()
            } else {
                def.name.clone()
            };
            for item in &def.commands {
                let Some(code) = plugin_commands::function_code(slot, item.index) else {
                    continue;
                };
                commands.push(CommandInfo {
                    code,
                    slot,
                    plug: item.index,
                    plugin_id: rec.id.clone(),
                    plugin_name: plugin_name.clone(),
                    label: item.label.clone(),
                    handler: item.handler.clone(),
                    icon: item.icon.clone(),
                    added: false,
                });
            }
        }
    }
    // The commands a running plugin added stay while their plugin is still there in the same slot.
    let added: Vec<CommandInfo> = plugin_commands::commands()
        .into_iter()
        .filter(|old| {
            old.added
                && commands
                    .iter()
                    .any(|c| c.slot == old.slot && c.plugin_id == old.plugin_id)
        })
        .collect();
    commands.extend(added);
    plugin_commands::set_commands(commands);
    plugin_commands::commands() != before
}

/// `F_PLUGIN_LIST` (`CEditWnd::InitMenu_Special`): a submenu for each plugin that has commands, with its
/// commands in it.
pub(crate) struct PluginList;

impl Lists for PluginList {
    fn items(&self, _: u16) -> Vec<Item> {
        Vec::new()
    }

    fn entries(&self, code: u16) -> Vec<Entry> {
        if code != F_PLUGIN_LIST {
            return Vec::new();
        }
        let mut out: Vec<Entry> = Vec::new();
        let mut slot = None;
        for command in plugin_commands::commands() {
            let item = Entry::Item(Item {
                label: command.label.clone(),
                action: MenuAction::Function(command.code),
                shortcut: None,
                enabled: true,
            });
            match (slot == Some(command.slot), out.last_mut()) {
                (true, Some(Entry::Submenu { entries, .. })) => entries.push(item),
                _ => {
                    slot = Some(command.slot);
                    out.push(Entry::Submenu {
                        label: command.plugin_name.clone(),
                        entries: vec![item],
                    });
                }
            }
        }
        out
    }
}

impl EditorView {
    /// Runs the plugin command with the function code `code` on this window, the way a menu, a key
    /// or a toolbar button does. The call starts after the command that asked for it is over (the
    /// call needs this view); a reason it cannot run goes to the status bar. Returns whether the call
    /// was started.
    pub(crate) fn run_plugin_command(&mut self, code: u16, cx: &mut Context<Self>) -> bool {
        let editor = cx.entity();
        cx.defer(move |cx| {
            let Some(dirs) = search_dirs(cx) else {
                return;
            };
            match crate::macros::plugin_call::prepare_function(cx, &dirs, code) {
                Ok(prepared) => crate::macros::plugin_call::run(editor, prepared, cx),
                Err(message) => {
                    editor.update(cx, |view, cx| {
                        view.notice = Some(message);
                        cx.notify();
                    });
                }
            }
        });
        true
    }
}

#[cfg(test)]
mod tests;
