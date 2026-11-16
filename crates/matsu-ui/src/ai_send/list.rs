//! `F_AI_PROMPT_LIST` (ADR-0035, 1): the registered destinations as the items of a menu, each calling its function
//! `F_AI_PROMPT_n` (mode a), in the way that `F_ONELINER_LIST` is the registered one-liners
//! (`crate::oneliner::OneLinerList`).

use std::sync::Arc;

use matsu_config::settings::ai::{AiDestinations, F_AI_PROMPT_0, F_AI_PROMPT_LIST};

use crate::menu::{Item, Lists, MenuAction};

/// The name of the submenu that holds the destinations and the rest of the AI items (the right-click menu names it
/// from here; the main menu has it as a row of its own).
pub(crate) const SUBMENU_NAME: &str = "生成 AI";
/// The submenu of the destinations, inside [`SUBMENU_NAME`].
pub(crate) const PLAIN_SUBMENU_NAME: &str = "そのまま送る";

/// The items of `F_AI_PROMPT_LIST`: every destination that has a command, in the order of the slots, named by its
/// name (its command when it has none). Whether an item can be chosen (a document that cannot be edited, a run that
/// goes on) is for the states of the window to say ([`crate::menu_state::FuncStates`]), as for any item with a
/// function.
pub(crate) struct AiPromptList(pub Arc<AiDestinations>);

impl Lists for AiPromptList {
    fn items(&self, code: u16) -> Vec<Item> {
        if code != F_AI_PROMPT_LIST {
            return Vec::new();
        }
        self.0
            .slots
            .iter()
            .enumerate()
            .filter(|(_, destination)| destination.is_registered())
            .filter_map(|(index, destination)| {
                Some(Item {
                    label: destination.title().to_owned(),
                    action: MenuAction::Function(F_AI_PROMPT_0 + u16::try_from(index).ok()?),
                    shortcut: None,
                    enabled: true,
                })
            })
            .collect()
    }
}
