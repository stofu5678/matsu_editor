//! Tag jump: to the file and place named on the caret's line, and back (`F_TAGJUMP`,
//! `F_TAGJUMP_CLOSE`, `F_TAGJUMPBACK`).
//!
//! Sakura: `Command_TAGJUMP`, `Command_TAGJUMPBACK` (`sakura_core/cmd/CViewCommander_TagJump.cpp`),
//! `CEditView::TagJumpSub` (`sakura_core/view/CEditView_Command.cpp`) and `CTagJumpManager`
//! (`sakura_core/env/CTagJumpManager.cpp`). What the line names is found by
//! [`ume_core::tag_jump`].
//!
//! Before a jump the window and the caret are pushed on a stack that all windows share (Sakura keeps
//! it in the shared memory, 100 entries; when it is full the oldest entry is dropped). The jump
//! brings the window to the front if the file is open and moves its caret to the place, else opens
//! the file in a new window with the encoding detected. Without a line number the caret stays.
//! Jumping back pops the stack and returns to that window and place.
//!
//! Not done: when the line names nothing, Sakura tries the tags file (`ctags`) with the word under
//! the caret (`Command_TagJumpByTagsFile`); this program has no tags file support yet, so it only
//! says that it cannot jump.

use std::time::Duration;

use gpui::{AnyWindowHandle, AsyncWindowContext, Context, Global, WeakEntity};
use ume_core::buffer::{Pos, col_from_utf16};
use ume_core::tag_jump::{TagTarget, find_target};

use super::close::confirm_and_close_window;
use super::load::load_here;
use super::windows::{self, open_editor_window};
use crate::document::SharedDocument;
use crate::view::EditorView;

/// `MAX_TAGJUMPNUM`.
const MAX_ENTRIES: usize = 100;

/// `STR_ERR_TAGJMP1`.
const CANNOT_JUMP: &str = "タグジャンプできません";
/// `STR_ERR_TAGJMPBK1`.
const CANNOT_JUMP_BACK: &str = "タグジャンプバックできません";

/// Where the tag jumps came from, the newest last.
#[derive(Default)]
struct TagJumpStack {
    entries: Vec<(AnyWindowHandle, Pos)>,
}

impl Global for TagJumpStack {}

impl TagJumpStack {
    /// `CTagJumpManager::PushTagJump`: a full stack drops its oldest entry.
    fn push(&mut self, window: AnyWindowHandle, pos: Pos) {
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push((window, pos));
    }

    /// `CTagJumpManager::PopTagJump`.
    fn pop(&mut self) -> Option<(AnyWindowHandle, Pos)> {
        self.entries.pop()
    }
}

/// Pushes the place of the caret of `view` for a later `F_TAGJUMPBACK`.
pub(crate) fn push_current(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let Ok(pos) = view.read_with(cx, |view, _| view.cursor().caret()) else {
        return;
    };
    let handle = cx.window_handle();
    cx.update(|_, app| app.default_global::<TagJumpStack>().push(handle, pos))
        .ok();
}

impl EditorView {
    /// Moves the caret to the line and column of a tag (both from 1; the column in UTF-16 code
    /// units, Sakura's). A line beyond the end goes to the last line. Without a line number the
    /// caret stays (`CEditView::TagJumpSub`: "a negative value is invalid").
    pub(crate) fn go_to_tag(
        &mut self,
        line: Option<usize>,
        column: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        let Some(line) = line else {
            return;
        };
        let doc = self.document().read(cx).doc();
        let Some(index) = doc
            .line_count()
            .checked_sub(1)
            .map(|last| (line - 1).min(last))
        else {
            return;
        };
        let col = match (column, doc.line(index)) {
            (Some(column), Ok(Some(text))) => col_from_utf16(&text.text, column - 1),
            _ => 0,
        };
        self.go_to(Pos::new(index, col), cx);
    }
}

/// The target of the tag jump on the caret line of `view`, and the place of the caret.
fn target_of(view: &EditorView, cx: &gpui::App) -> Option<TagTarget> {
    let shared = view.document().read(cx);
    let doc = shared.doc();
    let base = shared.path().and_then(|path| path.parent());
    find_target(
        &|i| doc.line(i).ok().flatten().map(|line| line.text),
        view.cursor().caret().line,
        base,
        &|path| path.is_file(),
    )
}

/// `F_TAGJUMP` and `F_TAGJUMP_CLOSE` (`close`): jumps to what the caret line names. With `close`
/// the window that the jump started from is closed afterwards (asking about unsaved changes),
/// also when the jump went to a place in the same window (`TagJumpSub` calls
/// `Command_WINCLOSE` whatever the target was).
pub(crate) async fn tag_jump(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    close: bool,
) {
    let Ok(Some(target)) = view.read_with(cx, target_of) else {
        view.update(cx, |view, cx| {
            view.notice = Some(CANNOT_JUMP.to_owned());
            cx.notify();
        })
        .ok();
        return;
    };
    // Saved before the jump: a jump inside this window moves the caret that is saved.
    push_current(view, cx);
    if !jump_to(&target, cx).await {
        // The entry stays, as in Sakura (`PushTagJump` comes before the file is opened).
        view.update(cx, |view, cx| {
            view.notice = Some(CANNOT_JUMP.to_owned());
            cx.notify();
        })
        .ok();
        return;
    }
    if close {
        confirm_and_close_window(view, cx).await;
    }
}

/// Brings the window of the file to the front and moves its caret, or opens the file in a new
/// window. `false` if the file could not be opened.
async fn jump_to(target: &TagTarget, cx: &mut AsyncWindowContext) -> bool {
    let (line, column) = (target.line, target.column);
    let open = cx
        .update(|_, app| windows::find_open(app, &target.path))
        .ok()
        .flatten();
    if let Some((handle, editor)) = open {
        // A window that is still reading its file would drop the caret when the file arrives.
        let timer = cx.background_executor().clone();
        while cx
            .update(|_, app| windows::has_pending(app, handle))
            .unwrap_or(false)
        {
            timer.timer(Duration::from_millis(50)).await;
        }
        // The place is set before the window comes to the front, as in Sakura.
        let own = cx.window_handle() == handle;
        let moved = cx.update(|_, app| {
            editor.update(app, |view, cx| view.go_to_tag(line, column, cx));
            if !own {
                windows::activate(app, handle);
            }
        });
        return moved.is_ok();
    }
    let Ok(handle) = cx.update(|_, app| open_editor_window(app, SharedDocument::empty(), None))
    else {
        return false;
    };
    let Ok(handle) = handle else {
        return false;
    };
    let path = target.path.clone();
    let Ok(mut window_cx) = handle.update(cx, |_, window, app| window.to_async(app)) else {
        return false;
    };
    let editor = window_cx
        .update(|_, app| {
            windows::editors(app)
                .into_iter()
                .find(|(h, _)| *h == handle)
                .map(|(_, view)| view.downgrade())
        })
        .ok()
        .flatten();
    let Some(editor) = editor else {
        return false;
    };
    // The file is read in the new window, which shows the progress and the errors.
    let loaded = load_here(&editor, &mut window_cx, &path, None, None).await;
    if !loaded {
        // The window was opened for this jump and has no file; one in which the user has typed
        // meanwhile (which is why a load is dropped) stays.
        let modified = editor
            .read_with(&window_cx, |view, app| {
                view.document().read(app).model().is_modified()
            })
            .unwrap_or(true);
        if !modified {
            window_cx.update(|window, _| window.remove_window()).ok();
        }
        return false;
    }
    editor
        .update(&mut window_cx, |view, cx| view.go_to_tag(line, column, cx))
        .is_ok()
}

/// `F_TAGJUMPBACK`: back to the window and the place of the last jump.
pub(crate) fn tag_jump_back(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let entry = cx
        .update(|_, app| app.default_global::<TagJumpStack>().pop())
        .ok()
        .flatten();
    // The window may be gone (`IsSakuraMainWindow`).
    let target = entry.and_then(|(handle, pos)| {
        let editor = cx
            .update(|_, app| {
                windows::editors(app)
                    .into_iter()
                    .find(|(h, _)| *h == handle)
                    .map(|(_, view)| view)
            })
            .ok()
            .flatten()?;
        Some((handle, pos, editor))
    });
    let Some((handle, pos, editor)) = target else {
        view.update(cx, |view, cx| {
            view.notice = Some(CANNOT_JUMP_BACK.to_owned());
            cx.notify();
        })
        .ok();
        return;
    };
    let own = cx.window_handle() == handle;
    cx.update(|_, app| {
        editor.update(app, |view, cx| view.go_to(pos, cx));
        if !own {
            windows::activate(app, handle);
        }
    })
    .ok();
}
