//! What the file flows need from the view, kept out of `view/mod.rs`.

use std::path::PathBuf;

use gpui::{Context, ExternalPaths, Window};
use matsu_core::buffer::{Document, Pos};
use matsu_core::cursor::Cursor;
use matsu_core::encoding::Encoding;
#[cfg(test)]
use matsu_core::file::no_progress;
use matsu_core::file::{FileError, SaveOptions, SaveReport, Written};
use matsu_layout::DocumentLayout;

use crate::document::SharedDocument;
use crate::layout_adapter::LAYOUT_BUDGET;
use crate::view::{EditorView, initial_layout_config};

use super::{load, start};

impl EditorView {
    /// Whether a file can be opened into this window instead of a new one: it is untitled and
    /// unmodified (`CEditDoc::IsAcceptLoad`, `sakura_core/doc/CEditDoc.cpp`).
    pub(crate) fn accepts_load(&self, cx: &gpui::App) -> bool {
        let shared = self.document.read(cx);
        // A Grep window is not one to load into (`CEditDoc::IsAcceptLoad`).
        shared.path().is_none() && !shared.model().is_modified() && !self.is_grep_window()
    }

    /// Replaces the document of the view with `new`: a new layout, the caret at `restore` (or at
    /// the top) and the view at the top. The old document and its undo history are gone.
    pub(crate) fn load_document(
        &mut self,
        new: SharedDocument,
        restore: Option<Pos>,
        cx: &mut Context<Self>,
    ) {
        self.load_document_of_type(new, restore, None, cx);
    }

    /// [`load_document`](Self::load_document) for a file that the history remembers with the
    /// document type of id `remembered` (`m_nTypeId` of the history, `CMruListener::OnBeforeLoad`):
    /// that type is taken instead of the one the name gives, so that the type is chosen once,
    /// before the indentation of the file is detected with the settings of that type. An id
    /// that is no type (or none) leaves the type of the name.
    pub(crate) fn load_document_of_type(
        &mut self,
        new: SharedDocument,
        restore: Option<Pos>,
        remembered: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        // Reopening the file keeps the mode (`CDocFileOperation::ReloadCurrentFile` loads with
        // the current one); any other new document starts without it.
        let mut new = new;
        // The document goes: what the user threw away leaves no copy (and a cancelled flow never gets here).
        crate::unsaved::apply_close_view(self, cx);
        if restore.is_some() {
            new.keep_view_mode(self.document.read(cx).view_mode());
            new.keep_watch_mode(self.document.read(cx).disk());
        }
        let version = self
            .document
            .update(cx, |shared, cx| shared.replace_with(new, cx));
        self.forget_scan_window(cx);
        // `CDocLocker::OnAfterLoad`: the old document let go of its file with `replace_with`.
        let share_mode = super::prefs::prefs(cx).file_share_mode;
        self.document
            .update(cx, |shared, _| shared.lock_after_load(share_mode));
        let doc = self.document.read(cx).doc();
        let mut layout = DocumentLayout::new(doc, initial_layout_config()).unwrap_or_else(|_| {
            DocumentLayout::new(&Document::empty(), initial_layout_config())
                .expect("the layout of an empty document")
        });
        let _ = layout.layout_for(doc, LAYOUT_BUDGET);
        self.layout = layout;
        // The change was made by this view: its layout is up to date.
        self.layout_version = version;
        self.cursor = Cursor::default();
        if let Some(pos) = restore {
            self.cursor.move_to(pos, false);
            let doc = self.document.read(cx).doc();
            let _ = self.cursor.clamp(doc);
        }
        self.marked = None;
        self.composing = false;
        self.scroll_x = 0.0;
        self.scroll_y = 0.0;
        self.notice = None;
        self.pending_scan_jump = None;
        self.pending_scan_restore = None;
        // A file opened by ranges is scanned behind its document. The place to go back to
        // (a reload) may be past what is scanned yet.
        self.watch_scan(cx);
        self.defer_restore(restore, None, cx);
        // The new document is not the result of a Grep.
        self.grep.forget_result();
        // The new document may be of another type (matsu.ini): then it has that type's font, wrap
        // and colors. Reloading the same type (reopen) keeps the temporary settings (zoom, wrap);
        // only the layout, which `initial_layout_config()` above reset, is brought back.
        // The type the user chose (`F_CHANGETYPE`) lasts until a file is loaded (`SetDocumentType`
        // with `force`).
        // (`replace_with` above made a document that is not locked.)
        let name = self.document.read(cx).file_name();
        let wanted = crate::doc_types::with_user(cx, |user| {
            let index = remembered
                .and_then(|id| user.document_types().iter().position(|t| t.id == id))
                .unwrap_or_else(|| user.type_index_for(name.as_deref()));
            (index != self.prefs.settings.type_index)
                .then(|| (index, user.view_settings_of_type(index)))
        });
        let type_changed = wanted.is_some();
        if let Some((index, settings)) = wanted {
            self.apply_settings(settings, cx);
            self.document
                .update(cx, |shared, cx| shared.announce_type(index, cx));
        }
        // `CDocEditor::OnAfterLoad`; the layouts follow if the tab width changed. This view has
        // the values before a macro goes on. The other panes take the new type when this update
        // ends, which resets their tab width: they get this view's values then.
        self.apply_loaded_indentation(true, cx);
        if type_changed {
            let this = cx.entity();
            cx.defer(move |cx| {
                this.update(cx, |view, cx| view.copy_indent_to_other_panes(cx));
            });
        }
        self.relayout_in(cx);
        self.ensure_caret_visible(cx);
        cx.notify();
    }

    /// Moves the caret and shows it (the "go to the place" answer of the save question).
    pub(crate) fn go_to(&mut self, pos: Pos, cx: &mut Context<Self>) {
        self.cursor.move_to(pos, false);
        let doc = self.document.read(cx).doc();
        let _ = self.cursor.clamp(doc);
        self.ensure_caret_visible(cx);
        cx.notify();
    }

    /// Writes the document to `path` on this thread and marks the undo history as saved. The save
    /// of the window goes through `finish_save_now`, which has the write done elsewhere.
    #[cfg(test)]
    pub(crate) fn save_now(
        &mut self,
        path: PathBuf,
        options: &SaveOptions,
        cx: &mut Context<Self>,
    ) -> Result<SaveReport, FileError> {
        self.after_write(options, cx, |shared| {
            shared.save_as(path, options, &mut no_progress)
        })
    }

    /// [`save_now`](Self::save_now) for a file that [`SaveSnapshot::write`] wrote on another thread
    /// (`file_ops::save_job`); `written` is what it said, or why it failed.
    pub(crate) fn finish_save_now(
        &mut self,
        path: PathBuf,
        options: &SaveOptions,
        written: Result<Written, FileError>,
        cx: &mut Context<Self>,
    ) -> Result<SaveReport, FileError> {
        self.after_write(options, cx, |shared| {
            Ok(shared.finish_save_as(path, options, written?))
        })
    }

    /// What follows a write in the window: the lock, the line endings, the undo history. `write`
    /// does the document's part of it.
    fn after_write(
        &mut self,
        options: &SaveOptions,
        cx: &mut Context<Self>,
        write: impl FnOnce(&mut SharedDocument) -> Result<SaveReport, FileError>,
    ) -> Result<SaveReport, FileError> {
        let cursor = &mut self.cursor;
        let share_mode = super::prefs::prefs(cx).file_share_mode;
        self.document.update(cx, |shared, cx| {
            // `CDocLocker::OnBeforeSave`: the save replaces the file, which a lock would refuse
            // (Windows) or lose (the lock belongs to the old file).
            shared.unlock_file();
            let saved = write(shared);
            // `CDocLocker::OnAfterSave`: lock the file that is there, also after a failed save
            // (the old file is as it was).
            shared.lock_file(share_mode);
            let report = saved?;
            // The file was written: the check against the file as it was does not apply any more.
            shared.clear_restored_check();
            // `CSaveAgent::OnBeforeSave` -> `CDocVisitor::SetAllEol`: a save that names a line
            // ending converts the lines of the document, an undo step of its own.
            let mut converted = true;
            if report.reloaded {
                // The document is the file that was written, read again: the endings are
                // converted already, and the cursor was in the document that is gone.
                let _ = cursor.clamp(shared.doc());
                shared.changed(cx);
            } else if let Some(eol) = options.eol {
                match shared.unify_line_endings(eol, cursor) {
                    Some(true) => {
                        shared.changed(cx);
                    }
                    Some(false) => {}
                    None => {
                        // Partly converted: it stays modified, the file was written.
                        shared.changed(cx);
                        converted = false;
                    }
                }
            }
            if converted {
                shared.parts_mut().0.mark_saved(cursor);
                // Saved: the kept copy is not wanted any more, now and not at the next tick (ADR-0022).
                crate::unsaved::copy_saved(cx.entity_id(), cx);
            }
            cx.notify();
            Ok(report)
        })
    }

    /// Loads `path` into this window in the background, showing progress and errors here.
    pub(crate) fn load_path_here(
        &mut self,
        path: PathBuf,
        encoding: Option<Encoding>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        start(window, cx, async move |view, cx| {
            load::load_here(&view, cx, &path, encoding, None).await;
            cx.update(|_, app| super::windows::finish_pending(app, &view))
                .ok();
        });
    }

    /// Files dropped on the window (`CEditWnd::OnDropFiles`): see [`load::drop_files`].
    pub(crate) fn on_external_drop(
        &mut self,
        paths: &ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // `ActivateFrameWindow( GetHwnd() )` before anything else (2009.08.20 ryoji).
        window.activate_window();
        let paths: Vec<PathBuf> = paths.paths().to_vec();
        let settings = load::DropSettings::read(cx);
        start(window, cx, async move |view, cx| {
            load::drop_files(&view, cx, paths, settings).await;
        });
    }
}
