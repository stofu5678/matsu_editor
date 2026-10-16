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
//! # The macro waits when the file goes into this window
//!
//! Sakura reads such a file before `FileOpen` returns, so the calls after it work on the new
//! text. Here too: the file is read at once (on the UI thread, as Sakura does), and a failure is
//! shown and the macro goes on. A file for a new window, and the open dialog (empty name), are
//! asynchronous: the macro does not wait for them (Sakura waits for the dialog, and starts a new
//! process for the new window, which a macro does not wait for either).
//!
//! The default name (the folder, or the file, that the dialog starts with) is not used: GPUI's
//! open dialog cannot be told where to start.

use std::ops::ControlFlow;
use std::path::Path;

use gpui::{Context, Window};
use ume_commands::command::{Arg, CommandError, Result};
use ume_core::encoding::Encoding;

use super::int_arg;
use crate::document::SharedDocument;
use crate::file_ops::args::path_arg;
use crate::file_ops::ask::open_error_message;
use crate::file_ops::load::{choose_files, open_paths};
use crate::file_ops::{start, view_mode_request, windows};
use crate::open::open_text_file;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct FileOpen2;

impl UiCommand for FileOpen2 {
    const ID: &'static str = "fileopen2";
    const SAKURA: &'static str = "F_FILEOPEN2";

    fn run(
        &self,
        view: &mut EditorView,
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
        if !matches!(args.get(3), None | Some(Arg::Text(_))) {
            return Err(CommandError::BadArgs(
                "FileOpen needs a text as argument 4".to_owned(),
            ));
        }
        if let Some(path) = &path
            && goes_into_this_window(view, window, path, cx)
        {
            read_here(view, path, encoding, view_mode, window, cx);
            return Ok(());
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
                        if windows::find_open(app, path).is_none() {
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

/// Whether `path` is read into this window: it is not open anywhere, and this window is
/// untitled, unmodified and not busy reading another file (`CLoadAgent::OnCheckLoad`).
fn goes_into_this_window(
    view: &EditorView,
    window: &Window,
    path: &Path,
    cx: &mut Context<EditorView>,
) -> bool {
    let handle = window.window_handle();
    windows::find_open(cx, path).is_none()
        && !windows::has_pending(cx, handle)
        && view.accepts_load(cx)
}

/// Reads the file now and makes it the document of this window. A failure is shown.
fn read_here(
    view: &mut EditorView,
    path: &Path,
    encoding: Option<Encoding>,
    view_mode: bool,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let defaults = crate::settings::new_document_defaults(cx, name.as_deref());
    match open_text_file(path, encoding, &defaults, &mut |_, _| {
        ControlFlow::Continue(())
    }) {
        Ok(file) => {
            let mut shared = SharedDocument::from_text_file(file);
            if view_mode {
                shared.keep_view_mode(true);
            }
            view.load_document(shared, None, cx);
        }
        Err(error) => {
            let (message, detail) = open_error_message(path, &error);
            view.macro_failed(format!("{message}\n{detail}"), window, cx);
        }
    }
}

register_ui_command!(FileOpen2);
