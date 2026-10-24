//! `F_ACTIVATE_SQLPLUS` (SQL*Plusをアクティブ表示, Shift+F11, the macro `ActivateSQLPLUS`): shows the
//! output panel, where the database functions put their output (ADR-0023, type B of ADR-0018).
//!
//! Sakura: `Command_ACTIVATE_SQLPLUS` (`sakura_core/cmd/CViewCommander_File.cpp`) brings the
//! running SQL*Plus window to the front. The clients run here have no window of their own, so the
//! nearest thing is the panel that shows what they wrote.

use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct ActivateSqlplus;

impl UiCommand for ActivateSqlplus {
    const ID: &'static str = "activate_sqlplus";
    const SAKURA: &'static str = "F_ACTIVATE_SQLPLUS";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.macros.output.show();
        cx.notify();
        Ok(())
    }
}

register_ui_command!(ActivateSqlplus);
