//! `F_FILEOPEN`: opens files.
//!
//! Sakura: `Command_FILEOPEN` (`sakura_core/cmd/CViewCommander_File.cpp`). Arguments: the file
//! (without it the open dialog is shown), the encoding number (99 or none: detect). The first
//! file goes into this window when it is untitled and unmodified, else into a new window; the
//! other files get new windows (`CLoadAgent::OnCheckLoad`, `sakura_core/agent/CLoadAgent.cpp`).

use crate::file_ops::args::{encoding_arg, path_arg};
use crate::file_ops::load::{choose_files, open_paths};
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

#[derive(Default)]
pub struct FileOpen;

impl UiCommand for FileOpen {
    const ID: &'static str = "file_open";
    const SAKURA: &'static str = "F_FILEOPEN";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let path = path_arg(args, 0)?;
        let encoding = encoding_arg(args, 1)?;
        let _ = view;
        start(window, cx, async move |view, cx| {
            let paths = match path {
                Some(path) => vec![path],
                None => match choose_files(&view, cx, None).await {
                    Some(paths) => paths,
                    None => return,
                },
            };
            open_paths(&view, cx, paths, encoding).await;
        });
        Ok(())
    }
}

register_ui_command!(FileOpen);
