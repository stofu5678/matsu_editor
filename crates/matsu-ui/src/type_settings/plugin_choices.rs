//! The smart indent choices that the plugins add to the list of the type settings (ADR-0026, chapter 1).
//!
//! Sakura adds the label of a plug to the list of its jack when the plugin is loaded
//! (`CJackManager::RegisterPlug`: `CPropTypesScreen::AddSIndentMethod` for `SmartIndent`) and stores
//! the function code of the plug as the value (`CPlug::GetSmartIndentType`). The lists are the
//! built-in choices, then the plugins' in the order the plugins were loaded.
//!
//! Here the choices are those of the plugins that can run now: plugins are switched on, the plugin is
//! in the table, supported, and the user agreed to its files as they are. They are made when the
//! sheet opens ([`forget_smart_indent_plugs`]) and kept while it is open: the page asks at every draw.
//! A value that no choice stands for (the plugin was switched off, removed or changed) is listed as
//! 「（使えないプラグイン N）」 by the page (`pages::screen::smart_indent_types`), and OK writes it back as
//! it is, so switching a plugin off for a while does not change the types that use it.

use std::cell::RefCell;

use gpui::App;
use matsu_commands::plugin_commands;
use matsu_config::plugins::consent::{Consent, ConsentFile};
use matsu_config::plugins::{DEFAULT_LANGUAGE, Jack, PluginSettings, PluginState};

/// The label of a choice whose plugin has to be resident and is not, so that the choice that does
/// nothing is not a surprise.
const NOT_RESIDENT: &str = "（常駐が必要: UseCache=1）";

thread_local! {
    /// The plugs made last. Making them reads the definitions and checks the files of the plugins.
    static PLUGS: RefCell<Option<Vec<(String, i32)>>> = const { RefCell::new(None) };
}

/// Forgets the plugs made last: the next ask makes them again.
pub(crate) fn forget_smart_indent_plugs() {
    PLUGS.with(|plugs| *plugs.borrow_mut() = None);
}

/// The plugs of the `SmartIndent` jack of the plugins that can run, with their function codes.
pub(crate) fn smart_indent_plugs(cx: &App) -> Vec<(String, i32)> {
    if let Some(list) = PLUGS.with(|plugs| plugs.borrow().clone()) {
        return list;
    }
    let list = make_smart_indent_plugs(cx);
    PLUGS.with(|plugs| *plugs.borrow_mut() = Some(list.clone()));
    list
}

fn make_smart_indent_plugs(cx: &App) -> Vec<(String, i32)> {
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
            consent.consent_for_dir_cached(&def.id, dir.as_deref()),
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
