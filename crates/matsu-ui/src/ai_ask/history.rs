//! What the window of mode b remembers while MATSU Editor runs (ADR-0035, 4 and decided 4 (b)): the instructions that
//! were sent, up to [`MAX_HISTORY`], newest first without duplicates, and the destination that was used last.
//!
//! This is memory only. Nothing here reads or writes a file, and the types are not serializable: an instruction can
//! hold what is in a document or what is personal, so it is not kept in `matsu.ini`, in a session file or in a log
//! file (ADR-0030, decided 5, the same view as the log tab). It is gone when MATSU Editor ends.

use std::collections::VecDeque;

use gpui::{App, Global};
use matsu_core::ai_send::normalized_instruction;

/// How many instructions are kept (ADR-0035, decided 4).
pub(crate) const MAX_HISTORY: usize = 20;

/// The instructions that were sent, the newest first.
#[derive(Debug, Default)]
pub(crate) struct History {
    items: VecDeque<String>,
}

impl History {
    /// Puts `instruction` at the front, as it is sent (trimmed, in LF). The same instruction as an earlier one moves to
    /// the front instead of being added again, and the oldest is dropped beyond [`MAX_HISTORY`]. An empty instruction
    /// is not kept.
    pub(crate) fn remember(&mut self, instruction: &str) {
        let Ok(text) = normalized_instruction(instruction) else {
            return;
        };
        self.items.retain(|old| *old != text);
        self.items.push_front(text);
        self.items.truncate(MAX_HISTORY);
    }

    /// The instructions, newest first.
    pub(crate) fn items(&self) -> Vec<String> {
        self.items.iter().cloned().collect()
    }
}

/// The memory of the application: one for every window, as the settings are.
#[derive(Debug, Default)]
struct Memory {
    history: History,
    /// The number of the destination used last (a slot of `[AI]`).
    last_destination: Option<usize>,
}

impl Global for Memory {}

/// The instructions so far, the newest first.
pub(crate) fn items(cx: &App) -> Vec<String> {
    cx.try_global::<Memory>()
        .map(|memory| memory.history.items())
        .unwrap_or_default()
}

/// The destination that was used last in this run, if any.
pub(crate) fn last_destination(cx: &App) -> Option<usize> {
    cx.try_global::<Memory>()
        .and_then(|memory| memory.last_destination)
}

/// Remembers that `instruction` was sent to destination `index`.
pub(crate) fn remember(cx: &mut App, instruction: &str, index: usize) {
    if !cx.has_global::<Memory>() {
        cx.set_global(Memory::default());
    }
    let memory = cx.global_mut::<Memory>();
    memory.history.remember(instruction);
    memory.last_destination = Some(index);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_newest_is_first() {
        let mut history = History::default();
        history.remember("one");
        history.remember("two");
        history.remember("three");
        assert_eq!(history.items(), ["three", "two", "one"]);
    }

    #[test]
    fn the_same_instruction_moves_to_the_front_and_is_not_added_twice() {
        let mut history = History::default();
        history.remember("one");
        history.remember("two");
        // the same after trimming and with CR LF
        history.remember("  one \r\n");
        assert_eq!(history.items(), ["one", "two"]);
    }

    #[test]
    fn a_multi_line_instruction_is_kept_in_lf() {
        let mut history = History::default();
        history.remember("Translate.\r\nKeep the tone.");
        assert_eq!(history.items(), ["Translate.\nKeep the tone."]);
    }

    #[test]
    fn an_empty_instruction_is_not_kept() {
        let mut history = History::default();
        history.remember("   \n");
        assert!(history.items().is_empty());
    }

    #[test]
    fn at_most_twenty_are_kept_and_the_oldest_goes_first() {
        let mut history = History::default();
        for n in 0..MAX_HISTORY + 5 {
            history.remember(&format!("instruction {n}"));
        }
        let items = history.items();
        assert_eq!(items.len(), MAX_HISTORY);
        assert_eq!(items[0], format!("instruction {}", MAX_HISTORY + 4));
        assert_eq!(items[MAX_HISTORY - 1], "instruction 5");
    }
}
