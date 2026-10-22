//! `F_DOWN_SEL`: moves the caret one row down and extends the selection.
//!
//! Sakura: `CViewCommander::Command_DOWN` in `sakura_core/cmd/CViewCommander_Cursor.cpp`, called from
//! `CViewCommander::HandleCommand` in `sakura_core/cmd/CViewCommander.cpp`.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct CursorDownSel;

impl Command for CursorDownSel {
    const ID: &'static str = "cursor_down_sel";
    const SAKURA: &'static str = "F_DOWN_SEL";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.move_vertical(1, true)
    }
}

register_command!(CursorDownSel);

#[cfg(test)]
mod tests {
    use crate::testing::check;

    #[test]
    fn moves() {
        check("cursor_down_sel", "ab|cd\nefgh", "ab[cd\nef|gh");
        check("cursor_down_sel", "ab|cd", "ab|cd");
    }
}
