//! `F_PLSQL_COMPILE_ON_DATABASE_CHOICE` (接続を選んでデータベースで実行, the macro
//! `ExecDatabaseChoice`): chooses a connection of `databases.ini` from a list and runs the document
//! on it, the way F10 does on the default connection (ADR-0023).
//!
//! Sakura has nothing like it (its F10 finds the one SQL*Plus window); it is a MATSU-only function
//! (`xtask/src/matsu_extras.rs`). The file is only read: the default connection stays as it is. The
//! flow is in `crate::database::run`. The menu item is gray without a file, like F10's.

use crate::database::run::run_chosen;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct PlsqlCompileOnDatabaseChoice;

impl UiCommand for PlsqlCompileOnDatabaseChoice {
    const ID: &'static str = "plsql_compile_on_database_choice";
    const SAKURA: &'static str = "F_PLSQL_COMPILE_ON_DATABASE_CHOICE";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        start(window, cx, async move |view, cx| {
            run_chosen(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(PlsqlCompileOnDatabaseChoice);
