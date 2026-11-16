//! `F_ONELINER_LIST` (ADR-0029): the registered one-liners as the items of a menu, each calling its function, in the
//! way that `F_USERMACRO_LIST` is the registered macros (`crate::macros::MacroList`).

use std::sync::Arc;

use matsu_config::settings::oneliner::{F_ONELINER_LIST, OneLiners, function_of_index};

use crate::menu::{Item, Lists, MenuAction};

/// The items of `F_ONELINER_LIST`: every one-liner that has a command, in the order of the slots, named by its name
/// (its command when it has none). Whether an item can be chosen (a document that cannot be edited, a run that goes
/// on) is for the states of the window to say, as for any item with a function; a slot that `[OneLiner]` still marks
/// for a generative AI is grayed here (ADR-0035, 6: it does not run as a one-liner until it has moved to `[AI]`).
///
/// The way to the log of what was sent to an AI is not here since ADR-0035: it is in the submenu 「生成 AI」.
pub(crate) struct OneLinerList(pub Arc<OneLiners>);

impl Lists for OneLinerList {
    fn items(&self, code: u16) -> Vec<Item> {
        if code != F_ONELINER_LIST {
            return Vec::new();
        }
        self.0
            .slots
            .iter()
            .enumerate()
            .filter(|(_, one)| one.is_registered())
            .filter_map(|(index, one)| {
                Some(Item {
                    label: one.title().to_owned(),
                    action: MenuAction::Function(function_of_index(index)?),
                    shortcut: None,
                    enabled: !one.is_ai_marked(),
                })
            })
            .collect()
    }
}
