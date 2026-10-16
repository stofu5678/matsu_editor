//! `F_FILESAVEAS`: saves under the given name, without a dialog.
//!
//! Sakura: `Command_FILESAVEAS` (`sakura_core/cmd/CViewCommander_File.cpp`) takes the file name
//! and the line ending (argument 3). It ignores the encoding number that the macro help lists, so
//! this does too.

use gpui::{Context, Window};
use ume_commands::command::CommandError;
use ume_commands::command::{Arg, Result};

use crate::file_ops::args::{eol_arg, path_arg};
use crate::file_ops::save::{SaveRequest, save};
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct FileSaveAs;

impl UiCommand for FileSaveAs {
    const ID: &'static str = "file_save_as";
    const SAKURA: &'static str = "F_FILESAVEAS";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let path = path_arg(args, 0)?
            .ok_or_else(|| CommandError::BadArgs("a file name is needed".to_owned()))?;
        let eol = eol_arg(args, 2)?;
        let _ = view;
        start(window, cx, async move |view, cx| {
            let request = SaveRequest {
                path: Some(path),
                eol,
                ..SaveRequest::default()
            };
            save(&view, cx, request).await;
        });
        Ok(())
    }
}

register_ui_command!(FileSaveAs);
