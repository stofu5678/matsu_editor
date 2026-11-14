//! `F_ONELINER_LIST` (ADR-0029): the registered one-liners as the items of a menu, each calling its function, in the
//! way that `F_USERMACRO_LIST` is the registered macros (`crate::macros::MacroList`).

use std::sync::Arc;

use matsu_config::settings::oneliner::{
    F_AI_LOG_SHOW, F_ONELINER_LIST, OneLiners, function_of_index,
};

use crate::menu::{Entry, Item, Lists, MenuAction};

/// The label of the item for the log (ADR-0030), with its access key where the menus show them (not on macOS).
pub(crate) const AI_LOG_LABEL: &str = if cfg!(target_os = "macos") {
    "AI ログを表示"
} else {
    "AI ログを表示(L)"
};

/// The items of `F_ONELINER_LIST`: every one-liner that has a command, in the order of the slots, named by its name
/// (its command when it has none). Whether an item can be chosen (a document that cannot be edited, a run that goes
/// on) is for the states of the window to say, as for any item with a function.
pub(crate) struct OneLinerList(pub Arc<OneLiners>);

impl Lists for OneLinerList {
    /// The registered one-liners, a separator, and the log: the log is not one of the one-liners.
    fn entries(&self, code: u16) -> Vec<Entry> {
        let mut items = self.items(code);
        let Some(log) = items.pop() else {
            return Vec::new();
        };
        let mut entries: Vec<Entry> = items.into_iter().map(Entry::Item).collect();
        if !entries.is_empty() {
            entries.push(Entry::Separator);
        }
        entries.push(Entry::Item(log));
        entries
    }

    fn items(&self, code: u16) -> Vec<Item> {
        if code != F_ONELINER_LIST {
            return Vec::new();
        }
        let mut items: Vec<Item> = self
            .0
            .slots
            .iter()
            .enumerate()
            .filter(|(_, one)| one.is_registered())
            .filter_map(|(index, one)| {
                Some(Item {
                    label: one.title().to_owned(),
                    action: MenuAction::Function(function_of_index(index)?),
                    shortcut: None,
                    enabled: true,
                })
            })
            .collect();
        // The way back to the log of what was sent to an AI (ADR-0030): always there, last, so that a window
        // without a tab bar can reach the log tab too, and no saved menu needs to change.
        items.push(Item {
            label: AI_LOG_LABEL.to_owned(),
            action: MenuAction::Function(F_AI_LOG_SHOW),
            shortcut: None,
            enabled: true,
        });
        items
    }
}
