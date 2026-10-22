//! `F_CHGMOD_INS`: switches between insert mode and overwrite mode (the Insert key).
//!
//! Sakura: `CViewCommander::Command_CHGMOD_INS` in `sakura_core/cmd/CViewCommander_ModeChange.cpp`
//! flips `CEditView::SetInsMode`. The mode belongs to the document (`CDocEditor::m_bInsMode`, set
//! from the common setting `m_bIsINSMode`, which is on at the start), not to the application.
//! The caret shape and the status bar follow the mode; `matsu-ui` does that.

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;

#[derive(Default)]
pub struct ToggleOverwriteMode;

impl Command for ToggleOverwriteMode {
    const ID: &'static str = "toggle_overwrite_mode";
    const SAKURA: &'static str = "F_CHGMOD_INS";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        cx.refuse_in_view_mode()?;
        cx.set_overwrite(!cx.is_overwrite());
        Ok(())
    }
}

register_command!(ToggleOverwriteMode);

#[cfg(test)]
mod tests {
    use crate::command::Arg;
    use crate::testing::Editor;

    #[test]
    fn switches_the_mode_back_and_forth() {
        let mut e = Editor::new("ab|cd");
        assert!(!e.overwrite());
        e.run("toggle_overwrite_mode");
        assert!(e.overwrite());
        assert!(e.changes().overwrite);
        e.run("toggle_overwrite_mode");
        assert!(!e.overwrite());
        assert!(!e.changes().overwrite);
    }

    #[test]
    fn changes_nothing_else() {
        let mut e = Editor::new("a[bc|d");
        e.run("toggle_overwrite_mode").assert_marked("a[bc|d");
        assert!(!e.changes().text_changed());
        assert!(!e.changes().caret_moved);
        assert!(!e.changes().beep);
    }

    #[test]
    fn typing_follows_the_mode() {
        let mut e = Editor::new("ab|cd");
        e.run("toggle_overwrite_mode");
        e.run_with("input_char", &[Arg::Char('X')])
            .assert_marked("abX|d");
        e.run("toggle_overwrite_mode");
        e.run_with("input_char", &[Arg::Char('Y')])
            .assert_marked("abXY|d");
    }
}
