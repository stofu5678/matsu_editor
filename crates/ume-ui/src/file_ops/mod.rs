//! Opening, saving and closing files in the window.
//!
//! The flows of Sakura's file functions (`sakura_core/cmd/CViewCommander_File.cpp`,
//! `sakura_core/doc/CDocFileOperation.cpp`) are written here as async functions over the view: they
//! ask the user (native dialogs and prompts), read and write through `ume_core::file`, and put the
//! result into the view. The UI commands in `ui_commands/file/` are thin: they read their
//! arguments and start a flow.
//!
//! * [`load`]: reading a file on a background task with progress, opening files into this window or
//!   new windows, reopening with an encoding.
//! * [`save`]: the save flow: the path, the questions Sakura asks before writing (mixed line
//!   endings, characters the encoding cannot write), and the write.
//! * [`save_as_dialog`]: the encoding, BOM and line ending asked after the path of a Save As.
//! * [`save_decoded`]: writing the bytes decoded from the selection (Base64, uu) to a chosen file.
//! * [`close`]: asking to save before closing, closing a file, a window, or all windows.
//! * [`external`]: handing the file to the OS (file manager, default program, a shell in the folder).
//! * [`header_source`]: the C/C++ header or source of the same name.
//! * [`lock`]: keeping the open file locked against other programs (排他制御).
//! * [`property`]: the text of the file property dialog.
//! * [`start_options`]: what Sakura's command-line options do to the documents a launch opens.
//! * [`windows`]: making editor windows, and finding the windows that are open.
//! * [`ask`]: the prompts and the texts of the questions and the errors.
//! * [`args`]: the arguments of the commands (Sakura's numbers for encodings and line endings).

pub(crate) mod args;
pub(crate) mod ask;
pub(crate) mod backup;
pub(crate) mod close;
pub(crate) mod drop_zone;
pub(crate) mod external;
pub(crate) mod header_source;
pub(crate) mod load;
pub(crate) mod lock;
pub(crate) mod prefs;
pub(crate) mod property;
pub(crate) mod save;
pub(crate) mod save_as_dialog;
pub(crate) mod save_decoded;
pub(crate) mod start_options;
pub(crate) mod tag_jump;
mod view_ext;
pub(crate) mod view_mode_request;
pub(crate) mod windows;

use gpui::{AsyncWindowContext, Context, WeakEntity, Window};

use crate::view::EditorView;

/// Starts a flow on the view without waiting for it. The flow ends when the user has answered all
/// questions; cancelling any of them leaves everything as it was.
pub(crate) fn start<F>(window: &mut Window, cx: &mut Context<EditorView>, flow: F)
where
    F: AsyncFnOnce(WeakEntity<EditorView>, &mut AsyncWindowContext) + 'static,
{
    // A macro that called the command waits until the flow ends (`macros::wait`).
    let tracked = crate::macros::track(cx);
    cx.spawn_in(window, async move |view, cx| {
        let _tracked = tracked;
        flow(view, cx).await;
    })
    .detach();
}

#[cfg(test)]
mod file_group_tests;
#[cfg(test)]
mod new_document_tests;
#[cfg(test)]
mod tag_jump_tests;
#[cfg(test)]
mod tests;
