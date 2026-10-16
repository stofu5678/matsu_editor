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
//! * [`close`]: asking to save before closing, closing a file, a window, or all windows.
//! * [`windows`]: making editor windows, and finding the windows that are open.
//! * [`ask`]: the prompts and the texts of the questions and the errors.
//! * [`args`]: the arguments of the commands (Sakura's numbers for encodings and line endings).

pub(crate) mod args;
pub(crate) mod ask;
pub(crate) mod close;
pub(crate) mod load;
pub(crate) mod save;
mod view_ext;
pub(crate) mod windows;

use gpui::{AsyncWindowContext, Context, WeakEntity, Window};

use crate::view::EditorView;

/// Starts a flow on the view without waiting for it. The flow ends when the user has answered all
/// questions; cancelling any of them leaves everything as it was.
pub(crate) fn start<F>(window: &mut Window, cx: &mut Context<EditorView>, flow: F)
where
    F: AsyncFnOnce(WeakEntity<EditorView>, &mut AsyncWindowContext) + 'static,
{
    cx.spawn_in(window, flow).detach();
}

#[cfg(test)]
mod tests;
