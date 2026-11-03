//! The choices that the plugins add to the lists of the type settings (ADR-0026, chapter 1).
//!
//! Sakura adds the label of a plug to the list of its jack when the plugin is loaded
//! (`CJackManager::RegisterPlug`: `CPropTypesScreen::AddSIndentMethod` for `SmartIndent`) and stores
//! the function code of the plug as the value (`CPlug::GetSmartIndentType`). The lists are the
//! built-in choices, then the plugins' in the order the plugins were loaded.
//!
//! Here the choices are those of the plugins that can run now: plugins are switched on, the plugin is
//! in the table, supported, and the user agreed to its files as they are. They are made when the
//! sheet opens. A value that no choice stands for (the plugin was switched off, removed or changed)
//! is kept as it is: nothing is chosen in the list and OK writes the same value back, so switching a
//! plugin off for a while does not change the types that use it.

use std::collections::HashMap;

use gpui::App;
use matsu_commands::plugin_commands;
use matsu_config::plugins::consent::{Consent, ConsentFile};
use matsu_config::plugins::{DEFAULT_LANGUAGE, Jack, PluginSettings, PluginState};

use super::ids::SMART_INDENT;

/// The label of a choice whose plugin has to be resident and is not, so that the choice that does
/// nothing is not a surprise.
const NOT_RESIDENT: &str = "（常駐が必要: UseCache=1）";

/// The choices of the plugins for the controls of the type settings, by control id. `values` are the
/// values of the sheet: when the smart indent is a plug that cannot run (its plugin is off, gone or
/// changed) it is listed as `（使えないプラグイン N）`, so that the page shows it and OK keeps it.
pub(crate) fn plugin_choices(
    cx: &App,
    values: &crate::common_settings::model::Values,
) -> HashMap<&'static str, Vec<(String, i32)>> {
    plugin_choices_for(cx, values.int(SMART_INDENT))
}

/// [`plugin_choices`] for the smart indent `now`.
pub(crate) fn plugin_choices_for(cx: &App, now: i32) -> HashMap<&'static str, Vec<(String, i32)>> {
    let mut choices = HashMap::new();
    let mut smart_indent = smart_indent_choices(cx);
    // 0 and 1 are the choices of the page itself (なし, C/C++).
    if now > 1 && !smart_indent.iter().any(|(_, value)| *value == now) {
        smart_indent.push((format!("（使えないプラグイン {now}）"), now));
    }
    if !smart_indent.is_empty() {
        choices.insert(SMART_INDENT, smart_indent);
    }
    choices
}

/// The plugs of the `SmartIndent` jack of the plugins that can run, with their function codes.
fn smart_indent_choices(cx: &App) -> Vec<(String, i32)> {
    let Some(dirs) = crate::plugin_commands::search_dirs(cx) else {
        return Vec::new();
    };
    let Some(settings) =
        crate::doc_types::with_user(cx, |user| user.profile().map(PluginSettings::from_profile))
    else {
        return Vec::new();
    };
    if !settings.enabled {
        return Vec::new();
    }
    let consent = ConsentFile::load(&dirs.consent_file());
    let mut choices = Vec::new();
    for (slot, rec) in settings.table.iter().enumerate() {
        if !rec.is_used() || rec.state == PluginState::Deleted || rec.name.is_empty() {
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
        for plug in def.plugs.iter().filter(|p| p.jack == Jack::SmartIndent) {
            let Some(code) = plugin_commands::plug_code(slot, plug.index) else {
                continue;
            };
            let label = if def.resident {
                plug.label.clone()
            } else {
                format!("{}{NOT_RESIDENT}", plug.label)
            };
            choices.push((label, i32::from(code)));
        }
    }
    choices
}
