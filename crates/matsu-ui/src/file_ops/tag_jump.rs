//! Tag jump: to the file and place named on the caret's line, and back (`F_TAGJUMP`,
//! `F_TAGJUMP_CLOSE`, `F_TAGJUMPBACK`).
//!
//! Sakura: `Command_TAGJUMP`, `Command_TAGJUMPBACK` (`sakura_core/cmd/CViewCommander_TagJump.cpp`),
//! `CEditView::TagJumpSub` (`sakura_core/view/CEditView_Command.cpp`) and `CTagJumpManager`
//! (`sakura_core/env/CTagJumpManager.cpp`). What the line names is found by
//! [`matsu_core::tag_jump`].
//!
//! Before a jump the document (its window and tab) and the caret are pushed on a stack that all windows share (Sakura keeps
//! it in the shared memory, 100 entries; when it is full the oldest entry is dropped). The jump
//! brings the window to the front if the file is open and moves its caret to the place, else opens
//! the file in a new window with the encoding detected. Without a line number the caret stays.
//! Jumping back pops the stack and returns to that window and place.
//!
//! When the line names nothing, Sakura looks for the word under the caret in the tags file
//! (`Command_TagJumpByTagsFile`); that is [`crate::tags`], which comes back here to jump.

use std::time::Duration;

use gpui::{AsyncWindowContext, Context, Global, WeakEntity};
use matsu_core::buffer::Pos;
use matsu_core::tag_jump::{TagTarget, find_target};

use super::close::confirm_and_close_window;
use super::load::load_here;
use super::windows;
use crate::document::SharedDocument;
use crate::view::EditorView;

/// The code of a function of Sakura.
fn function_code(name: &str) -> u16 {
    matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == name)
        .map_or(0, |f| f.code)
}

/// `MAX_TAGJUMPNUM`.
const MAX_ENTRIES: usize = 100;

/// `STR_ERR_TAGJMP1`.
const CANNOT_JUMP: &str = "タグジャンプできません";
/// `STR_ERR_TAGJMPBK1`.
const CANNOT_JUMP_BACK: &str = "タグジャンプバックできません";

/// Where the tag jumps came from, the newest last.
#[derive(Default)]
struct TagJumpStack {
    entries: Vec<(WeakEntity<EditorView>, Pos)>,
}

impl Global for TagJumpStack {}

impl TagJumpStack {
    /// `CTagJumpManager::PushTagJump`: a full stack drops its oldest entry.
    fn push(&mut self, view: WeakEntity<EditorView>, pos: Pos) {
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push((view, pos));
    }

    /// `CTagJumpManager::PopTagJump`.
    fn pop(&mut self) -> Option<(WeakEntity<EditorView>, Pos)> {
        self.entries.pop()
    }
}

/// Pushes the place of the caret of `view` for a later `F_TAGJUMPBACK`.
pub(crate) fn push_current(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let Ok(pos) = view.read_with(cx, |view, _| view.cursor().caret()) else {
        return;
    };
    let view = view.clone();
    cx.update(|_, app| app.default_global::<TagJumpStack>().push(view, pos))
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
        let col = column.map_or(0, |column| doc.col_from_utf16_col(index, column - 1));
        self.go_to(Pos::new(index, col), cx);
    }
}

impl EditorView {
    /// Enter in the result of a Grep when 「エンターキーでタグジャンプ」 is on
    /// (`CEditView::TranslateCommand_grep`: the line break of `F_WCHAR` becomes `F_TAGJUMP`, and
    /// Ctrl closes the window the jump started from). Returns whether the key was taken.
    pub(crate) fn grep_enter_jump(
        &mut self,
        close: bool,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.is_grep_window() || !crate::search::prefs::prefs(cx).grep_enter_jumps {
            return false;
        }
        // Through the command (`F_TAGJUMP`, `F_TAGJUMP_CLOSE`), so that a key macro that is
        // being recorded keeps the jump.
        let name = if close {
            "F_TAGJUMP_CLOSE"
        } else {
            "F_TAGJUMP"
        };
        self.run_function(function_code(name), &[], window, cx);
        true
    }

    /// A double click in the result of a Grep when 「ダブルクリックでタグジャンプ」 is on
    /// (`CEditView::OnLBUTTONDBLCLK`). Sakura jumps by the line, then by the tags file, and does
    /// the ordinary double click when both fail. The tags file is searched in the background here,
    /// so the double click can only fall back when the line itself names no place: returns whether
    /// the line did and the jump started.
    pub(crate) fn grep_double_click_jump(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.is_grep_window() || !crate::search::prefs::prefs(cx).grep_double_click_jumps {
            return false;
        }
        if target_of(self, cx).is_none() {
            return false;
        }
        self.run_function(function_code("F_TAGJUMP"), &[], window, cx);
        true
    }
}

impl EditorView {
    /// A click on a line of the output panel that names a place of an SQL file (ADR-0023): the
    /// same jump as a tag jump (`F_TAGJUMPBACK` comes back), or a move of the caret in this
    /// window when the SQL has no file.
    pub(crate) fn jump_to_place(
        &mut self,
        place: crate::macros::Place,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        match place {
            crate::macros::Place::Here { line, column } => {
                self.go_to_tag(Some(line), column, cx);
            }
            crate::macros::Place::File(target) => {
                // only a regular file (a pipe or a device would stop the window; a path that
                // starts like a network path never gets here, `is_safe_jump_path`)
                if !is_plain_local_file(&target.path, cfg!(windows)) {
                    self.notice = Some(CANNOT_JUMP.to_owned());
                    cx.notify();
                    return;
                }
                super::start(window, cx, async move |view, cx| {
                    jump_to_target(&view, cx, &target, false).await;
                });
            }
        }
    }
}

/// Whether `path` is a regular file that a jump from the output of a client may open. `is_file` follows
/// links, and a link can point to a network path (`\\\\host\\share`, which Windows supports), where the
/// check itself would send the user's credentials. So with `refuse_links` (Windows) no part of the path
/// may be a link: each part is looked at without following it, from the root down.
pub(crate) fn is_plain_local_file(path: &std::path::Path, refuse_links: bool) -> bool {
    if refuse_links {
        let mut so_far = std::path::PathBuf::new();
        for part in path.components() {
            so_far.push(part);
            // the drive (`C:`, `\\?\C:`) and the root are not links, and a drive alone is not a path to
            // ask for
            if matches!(
                part,
                std::path::Component::Prefix(_) | std::path::Component::RootDir
            ) {
                continue;
            }
            match std::fs::symlink_metadata(&so_far) {
                Ok(meta) if meta.file_type().is_symlink() => return false,
                Ok(_) => {}
                Err(_) => return false,
            }
        }
    }
    path.is_file()
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
        // "Also after the Grep form failed, the tags file is searched" (2011.11.29 Moca).
        if crate::tags::jump_by_tags_file(view, cx, close).await {
            return;
        }
        say_cannot_jump(view, cx);
        return;
    };
    jump_to_target(view, cx, &target, close).await;
}

/// `TagJumpSub`: pushes the place for `F_TAGJUMPBACK`, jumps to `target`, and closes the window
/// that the jump started from if `close`. Says so if the file cannot be opened.
pub(crate) async fn jump_to_target(
    view: &WeakEntity<EditorView>,
    cx: &mut AsyncWindowContext,
    target: &TagTarget,
    close: bool,
) -> bool {
    // Saved before the jump: a jump inside this window moves the caret that is saved.
    push_current(view, cx);
    if !jump_to(target, cx).await {
        // The entry stays, as in Sakura (`PushTagJump` comes before the file is opened).
        say_cannot_jump(view, cx);
        return false;
    }
    if close {
        confirm_and_close_window(view, cx).await;
    }
    true
}

/// `STR_ERR_TAGJMP1` in the status bar.
pub(crate) fn say_cannot_jump(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    view.update(cx, |view, cx| {
        view.notice = Some(CANNOT_JUMP.to_owned());
        cx.notify();
    })
    .ok();
}

/// Brings the window of the file to the front and moves its caret, or opens the file in a new
/// window. `false` if the file could not be opened.
async fn jump_to(target: &TagTarget, cx: &mut AsyncWindowContext) -> bool {
    let (line, column) = (target.line, target.column);
    // `:HWND:[id]`: a document with no file, named by the Grep of its text. It is gone when no
    // window has it (`IsSakuraMainWindow`).
    if let Some(id) = target.window {
        let found = cx
            .update(|_, app| windows::find_document_by_id(app, id))
            .ok()
            .flatten();
        let Some((handle, editor)) = found else {
            return false;
        };
        return cx
            .update(|_, app| {
                editor.update(app, |view, cx| view.go_to_tag(line, column, cx));
                crate::tabs::flows::reveal(app, handle, editor.clone());
            })
            .is_ok();
    }
    // A window that is still reading its file would drop the caret when the file arrives: wait,
    // and look again (the window may have closed, or the file may not have come).
    let timer = cx.background_executor().clone();
    let open = loop {
        let found = cx
            .update(|_, app| {
                windows::find_open(app, &target.path).map(|(handle, editor)| {
                    let pending = windows::is_pending(app, &editor.downgrade());
                    (handle, editor, pending)
                })
            })
            .ok()
            .flatten();
        match found {
            Some((_, _, true)) => timer.timer(Duration::from_millis(50)).await,
            Some((handle, editor, false)) => break Some((handle, editor)),
            None => break None,
        }
    };
    if let Some((handle, editor)) = open {
        // The place is set before the window comes to the front, as in Sakura.
        let moved = cx.update(|_, app| {
            editor.update(app, |view, cx| view.go_to_tag(line, column, cx));
            // the tab of the file may not be the one that is shown
            crate::tabs::flows::reveal(app, handle, editor.clone());
        });
        return moved.is_ok();
    }
    let own = cx.window_handle();
    // A new tab of this window in tab mode, else a new window (`OpenNewEditor`).
    let Ok(Ok((handle, editor))) = cx.update(|window, app| {
        crate::tabs::flows::open_document(
            app,
            Some(window),
            own,
            false,
            SharedDocument::empty(),
            None,
        )
    }) else {
        return false;
    };
    let editor = editor.downgrade();
    let path = target.path.clone();
    let Ok(mut window_cx) = handle.update(cx, |_, window, app| window.to_async(app)) else {
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
            super::close::close_tab_or_window(&editor, &mut window_cx);
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
    // The window or the tab may be gone (`IsSakuraMainWindow`). The stack remembers the document,
    // not the window: the tab may have moved to another window since.
    let target = entry.and_then(|(view, pos)| {
        let (handle, editor) = cx
            .update(|_, app| windows::find_tab_of(app, &view))
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
    cx.update(|_, app| {
        editor.update(app, |view, cx| view.go_to(pos, cx));
        crate::tabs::flows::reveal(app, handle, editor.clone());
    })
    .ok();
}
