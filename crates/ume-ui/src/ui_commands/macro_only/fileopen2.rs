//! `F_FILEOPEN2` (the macro `FileOpen(file, code, viewMode, defaultName)`): opens a file.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) stops the macro when the file
//! name is missing; the code defaults to 99 (`CODE_AUTODETECT`), the view mode and the default
//! name to none. `CViewCommander::Command_FILEOPEN` (`sakura_core/cmd/CViewCommander_File.cpp`)
//! turns a number that is not an encoding into auto detection, and shows the open dialog when the
//! file name is empty. The file opens as `F_FILEOPEN` does (the first in this window when it is
//! untitled and unmodified, else in a new window; a file that is open already is brought to the
//! front), and in view mode when the third argument is not 0.
//!
//! The default name (the folder, or the file, that the dialog starts with) is not used: GPUI's
//! open dialog cannot be told where to start.

use gpui::{Context, Window};
use ume_commands::command::{Arg, CommandError, Result};
use ume_core::encoding::Encoding;

use super::int_arg;
use crate::file_ops::args::path_arg;
use crate::file_ops::load::{choose_files, open_paths};
use crate::file_ops::{start, view_mode_request};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct FileOpen2;

impl UiCommand for FileOpen2 {
    const ID: &'static str = "fileopen2";
    const SAKURA: &'static str = "F_FILEOPEN2";

    fn run(
        &self,
        _view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if args.is_empty() {
            return Err(CommandError::BadArgs(
                "FileOpen needs the file name as argument 1".to_owned(),
            ));
        }
        let path = path_arg(args, 0)?;
        // `wtoi_def(Argument[1], CODE_AUTODETECT)`; `IsValidCodeType` else auto detection.
        let encoding = i32::try_from(int_arg("FileOpen", args, 1, 99)?)
            .ok()
            .and_then(Encoding::from_sakura_code);
        let view_mode = int_arg("FileOpen", args, 2, 0)? != 0;
        // the default name: see the module documentation
        if let Some(Arg::Text(_)) | None = args.get(3) {
        } else {
            return Err(CommandError::BadArgs(
                "FileOpen needs a text as argument 4".to_owned(),
            ));
        }
        start(window, cx, async move |view, cx| {
            let paths = match path {
                Some(path) => vec![path],
                None => match choose_files(cx).await {
                    Some(paths) => paths,
                    None => return,
                },
            };
            if view_mode {
                // A file that is open already is not read again, so it gets no request (it would
                // wait for the next time the file is opened).
                cx.update(|_, app| {
                    for path in &paths {
                        if crate::file_ops::windows::find_open(app, path).is_none() {
                            view_mode_request::request(app, path);
                        }
                    }
                })
                .ok();
            }
            open_paths(&view, cx, paths, encoding).await;
        });
        Ok(())
    }
}

register_ui_command!(FileOpen2);
