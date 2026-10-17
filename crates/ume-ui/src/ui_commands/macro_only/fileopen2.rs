//! `F_FILEOPEN2` (the macro `FileOpen(file, code, viewMode, defaultName)`): opens a file.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`) stops the macro when the file
//! name is missing; the code defaults to 99 (`CODE_AUTODETECT`), the view mode and the default
//! name to none. `CViewCommander::Command_FILEOPEN` (`sakura_core/cmd/CViewCommander_File.cpp`)
//! turns a number that is not an encoding into auto detection, and shows the open dialog when the
//! file name is empty. The file opens as `F_FILEOPEN` does (the first in this window when it is
//! untitled and unmodified, else in a new window, or a new tab in tab mode; a file that is open already is brought to
//! the front), and in view mode when the third argument is not 0.
//!
//! # The macro waits in all three cases
//!
//! Sakura reads a file that goes into this window before `FileOpen` returns, so the calls after
//! it work on the new text. Here too: such a file is read at once (on the UI thread, as Sakura
//! does), and a failure is shown and the macro goes on. A file for a new window, and the open
//! dialog (empty name), run as flows (`file_ops::start`) and the macro waits for them
//! (`macros/wait.rs`): until the dialog is closed and the file is read, and, for a new window,
//! until that window has finished loading. Sakura waits for the dialog, but starts a new process
//! for the new window and does not wait for it; waiting for it is the user's decision
//! (2026-10-17), so that the calls after `FileOpen` can rely on the file being loaded.
//!
//! # The default name
//!
//! The fourth argument is the folder, or the file, that the open dialog starts with. A folder
//! (the text ends with a separator) is where the dialog starts; for a file the dialog starts in
//! its folder, and offers the name when the folder exists (`Command_FILEOPEN`). Without it the
//! dialog starts where 「ファイルダイアログの初期位置」 says.

use std::ops::ControlFlow;
use std::path::Path;

use gpui::{Context, Window};
use ume_commands::command::{Arg, CommandError, Result};
use ume_config::dialog_dir::split_default_name;
use ume_core::encoding::Encoding;

use super::int_arg;
use crate::document::SharedDocument;
use crate::file_ops::args::path_arg;
use crate::file_ops::ask::open_error_message;
use crate::file_ops::load::{choose_files, open_paths};
use crate::file_ops::{start, view_mode_request, windows};
use crate::history::view::{encoding_of as history_encoding, remembered};
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
        let default_name = match args.get(3) {
            None => None,
            Some(Arg::Text(text)) => Some(text.as_str()),
            Some(_) => {
                return Err(CommandError::BadArgs(
                    "FileOpen needs a text as argument 4".to_owned(),
                ));
            }
        };
        let given = default_name
            .filter(|name| !name.is_empty())
            .map(|name| split_default_name(name, &|dir| dir.is_dir()));
        if let Some(path) = &path
            && goes_into_this_window(view, path, cx)
            && read_here(view, path, encoding, view_mode, window, cx)
        {
            return Ok(());
        }
        start(window, cx, async move |view, cx| {
            let paths = match path {
                Some(path) => vec![path],
                None => match choose_files(&view, cx, given).await {
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
fn goes_into_this_window(view: &EditorView, path: &Path, cx: &mut Context<EditorView>) -> bool {
    // this view is being updated: it is not read (it is untitled anyway, see `accepts_load`)
    let me = cx.entity_id();
    windows::find_open_except(cx, path, Some(me)).is_none()
        && !windows::is_pending_document(cx, view.document.entity_id())
        && view.accepts_load(cx)
}

/// Reads the file now and makes it the document of this window, the way `load::load_here` does:
/// a file the history knows is opened in the encoding of last time, and its caret, view and
/// bookmarks are restored; it goes to the front of the history. A failure is shown.
///
/// Returns false, having done nothing, when the user has to be asked which encoding to use
/// (`bQueryIfCodeChange`): the asynchronous flow asks.
fn read_here(
    view: &mut EditorView,
    path: &Path,
    encoding: Option<Encoding>,
    view_mode: bool,
    window: &mut Window,
    cx: &mut Context<EditorView>,
) -> bool {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let defaults = crate::settings::new_document_defaults(cx, name.as_deref());
    let read = |encoding| {
        open_text_file(path, encoding, &defaults, &mut |_, _| {
            ControlFlow::Continue(())
        })
    };
    let previous = remembered(cx, path);
    let mut result = read(encoding);
    if encoding.is_none()
        && let (Ok(file), Some(old)) = (&result, previous.as_ref().and_then(history_encoding))
        && old != file.encoding
    {
        // `CMruListener::OnBeforeLoad`: the encoding of last time, unless the user is asked.
        if path.exists() && crate::history::options(cx).query_if_code_change {
            return false;
        }
        // if the file cannot be read in that encoding, the detected one stays
        if let Ok(again) = read(Some(old)) {
            result = Ok(again);
        }
    }
    match result {
        Ok(file) => {
            let mut shared = SharedDocument::from_text_file(file);
            if view_mode {
                shared.keep_view_mode(true);
            }
            view.load_document(shared, None, cx);
            if let Some(entry) = &previous {
                let options = crate::history::options(cx);
                view.restore_from_history(entry, options, cx);
            }
            view.record_history(cx);
        }
        Err(error) => {
            let (message, detail) = open_error_message(path, &error);
            view.macro_failed(format!("{message}\n{detail}"), window, cx);
        }
    }
    true
}

register_ui_command!(FileOpen2);
