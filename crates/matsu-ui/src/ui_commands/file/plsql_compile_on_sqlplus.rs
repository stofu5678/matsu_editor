//! `F_PLSQL_COMPILE_ON_SQLPLUS` (データベースで実行, F10, the macro `ExecSQLPLUS`): runs the document on
//! the default connection of `databases.ini` (ADR-0023).
//!
//! Sakura: `Command_PLSQL_COMPILE_ON_SQLPLUS` (`sakura_core/cmd/CViewCommander_File.cpp`) pastes
//! `@file` into a running Windows SQL*Plus. This program runs the command line of the connection
//! and shows the output in the output panel; the flow is in `crate::database::run`. The menu item
//! is gray without a file (`IsFuncEnable`, `crate::menu_state`).

use crate::database::run::run_document;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct PlsqlCompileOnSqlplus;

impl UiCommand for PlsqlCompileOnSqlplus {
    const ID: &'static str = "plsql_compile_on_sqlplus";
    const SAKURA: &'static str = "F_PLSQL_COMPILE_ON_SQLPLUS";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        start(window, cx, async move |view, cx| {
            run_document(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(PlsqlCompileOnSqlplus);
