//! What the view does with the history.
//!
//! Sakura: `CMruListener` (`sakura_core/recent/CMruListener.cpp`).
//!
//! * After a file is loaded ([`EditorView::restore_from_history`], `OnAfterLoad`): the caret and the
//!   position of the view go to where they were, and the bookmarks come back, if the options say so.
//! * When a file is loaded and when it is closed ([`EditorView::record_history`], `OnAfterLoad` and
//!   `OnBeforeClose`): the file goes to the front of the history with the caret, the position of the
//!   view, the encoding and the bookmarks it has then.
//! * Before a file is loaded ([`encoding_of`], `OnBeforeLoad`): see `file_ops::load`.

use std::path::{Path, PathBuf};

use gpui::{App, Context, Window};
use ume_config::history::FileEntry;
use ume_config::history::marks::{decode_marks, encode_marks};
use ume_core::buffer::{Pos, col_from_utf16, col_to_utf16};
use ume_core::encoding::Encoding;

use super::HistoryOptions;
use crate::file_ops::load::open_paths;
use crate::file_ops::start;
use crate::file_ops::windows::editors;
use crate::view::EditorView;

/// The text of a path in the history: the absolute path, as the OS writes it. A path that cannot
/// be made absolute stays as it is.
pub(crate) fn path_text(path: &Path) -> String {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    path.to_string_lossy().into_owned()
}

/// What the history remembers of `path`.
pub(crate) fn remembered(cx: &App, path: &Path) -> Option<FileEntry> {
    super::current(cx)?.file(&path_text(path)).cloned()
}

/// The encoding of an entry, if it has one that can be used (`IsValidCodeOrCPType`).
pub(crate) fn encoding_of(entry: &FileEntry) -> Option<Encoding> {
    Encoding::from_sakura_code(entry.encoding_code)
}

/// The question of `bQueryIfCodeChange` (`STR_ERR_DLGEDITDOC6`): the file is about to be opened in
/// `new`, and last time it was opened in `old`. The first button, yes, takes `old`.
pub(crate) fn code_change_message(path: &Path, new: Encoding, old: Encoding) -> String {
    let (new, old) = (new.combo_name(), old.combo_name());
    format!(
        "{}\n\nこのファイルを文字コード {new} で開こうとしていますが、前回は別の文字コード {old} で開かれています。\n前回と同じ文字コードを使いますか？\n\n・[はい(Y)]  ＝{old}\n・[いいえ(N)]＝{new}\n",
        path.display()
    )
}

impl EditorView {
    /// The entry of the history for the file of this view as it is now (`CEditDoc::GetEditInfo`,
    /// with the bookmarks of `_HoldBookmarks_And_AddToMRU`). Nothing for a document without a file
    /// and for a Grep window.
    pub(crate) fn history_entry(&self, cx: &App) -> Option<FileEntry> {
        if self.is_grep_window() {
            return None;
        }
        let shared = self.document().read(cx);
        let path = path_text(shared.path()?);
        let doc = shared.doc();
        let caret = self.cursor().caret();
        let x = doc
            .line(caret.line)
            .ok()
            .flatten()
            .map_or(caret.col, |line| col_to_utf16(&line.text, caret.col));
        let mut entry = FileEntry::new(path);
        entry.encoding_code = shared.file().encoding.sakura_code();
        entry.caret_x = x as i32;
        entry.caret_y = caret.line as i32;
        entry.view_top = self.top_row() as i32;
        entry.view_left = (self.scroll_x / self.metrics.cell_width).round().max(0.0) as i32;
        entry.marks = encode_marks(shared.model().bookmarks().iter());
        // The id of the document type is not known here: what the history had stays.
        if let Some(old) = super::current(cx).and_then(|h| h.file(&entry.path).cloned()) {
            entry.type_id = old.type_id;
        }
        Some(entry)
    }

    /// Puts the file of this view at the front of the history.
    pub(crate) fn record_history(&self, cx: &mut Context<Self>) {
        if let Some(entry) = self.history_entry(cx) {
            super::add_file(cx, entry);
        }
    }

    /// Moves the caret and the view to where the history says they were, and sets the bookmarks
    /// (`CMruListener::OnAfterLoad`). A line past the end of the file puts the caret at the end of
    /// the file.
    pub(crate) fn restore_from_history(
        &mut self,
        entry: &FileEntry,
        options: HistoryOptions,
        cx: &mut Context<Self>,
    ) {
        if options.restore_cur_position && entry.caret_x >= 0 && entry.caret_y >= 0 {
            let doc = self.document().read(cx).doc();
            let line = entry.caret_y as usize;
            if line >= doc.line_count() {
                let end = doc.end_pos().unwrap_or_default();
                self.go_to(end, cx);
            } else {
                let text = doc.line(line).ok().flatten().map(|l| l.text);
                let col = text.map_or(0, |t| col_from_utf16(&t, entry.caret_x as usize));
                // The view first, then the caret, which scrolls only if it is not in sight.
                self.scroll_top_to(entry.view_top.max(0) as isize, cx);
                self.scroll_x = entry.view_left.max(0) as f32 * self.metrics.cell_width;
                self.go_to(Pos::new(line, col), cx);
            }
        }
        if options.restore_bookmarks {
            let count = self.document().read(cx).doc().line_count();
            self.document().clone().update(cx, |shared, _| {
                let (model, _) = shared.parts_mut();
                for line in decode_marks(&entry.marks)
                    .into_iter()
                    .filter(|l| *l < count)
                {
                    model.bookmarks_mut().set(line, true);
                }
            });
            cx.notify();
        }
    }

    /// `IDM_SELMRU + index`: opens the file of the history. The encoding the file had is used
    /// (`CControlTray::OnCommand`: the `EditInfo` is given to the new editor).
    pub(crate) fn open_recent_file(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = super::current(cx).and_then(|h| h.files.get(index).cloned()) else {
            return;
        };
        let path = PathBuf::from(&entry.path);
        let encoding = encoding_of(&entry);
        start(window, cx, async move |view, cx| {
            open_paths(&view, cx, vec![path], encoding).await;
        });
    }
}

/// Puts every open file into the history, as it is now. When the application quits.
pub(crate) fn record_all(cx: &mut App) {
    for (_, view) in editors(cx) {
        view.update(cx, |view, cx| view.record_history(cx));
    }
}
