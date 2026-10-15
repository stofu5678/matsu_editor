//! What the file flows need from the view, kept out of `view/mod.rs`.

use std::path::PathBuf;

use gpui::{Context, ExternalPaths, Window};
use ume_core::buffer::{Document, Pos};
use ume_core::cursor::Cursor;
use ume_core::encoding::Encoding;
use ume_core::file::{FileError, SaveOptions, SaveReport, no_progress};
use ume_layout::{DocumentLayout, LayoutConfig};

use crate::document::SharedDocument;
use crate::layout_adapter::LAYOUT_BUDGET;
use crate::view::EditorView;

use super::{load, start};

impl EditorView {
    /// Whether a file can be opened into this window instead of a new one: it is untitled and
    /// unmodified (`CEditDoc::IsAcceptLoad`, `sakura_core/doc/CEditDoc.cpp`).
    pub(crate) fn accepts_load(&self, cx: &gpui::App) -> bool {
        let shared = self.document.read(cx);
        shared.path().is_none() && !shared.model().is_modified()
    }

    /// Replaces the document of the view with `new`: a new layout, the caret at `restore` (or at
    /// the top) and the view at the top. The old document and its undo history are gone.
    pub(crate) fn load_document(
        &mut self,
        new: SharedDocument,
        restore: Option<Pos>,
        cx: &mut Context<Self>,
    ) {
        let version = self.document.update(cx, |shared, cx| {
            *shared = new;
            shared.changed(cx)
        });
        let doc = self.document.read(cx).doc();
        let mut layout = DocumentLayout::new(doc, LayoutConfig::default()).unwrap_or_else(|_| {
            DocumentLayout::new(&Document::empty(), LayoutConfig::default())
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

    /// Writes the document to `path` and marks the undo history as saved.
    pub(crate) fn save_now(
        &mut self,
        path: PathBuf,
        options: &SaveOptions,
        cx: &mut Context<Self>,
    ) -> Result<SaveReport, FileError> {
        let cursor = &self.cursor;
        self.document.update(cx, |shared, cx| {
            let report = shared.save_as(path, options, &mut no_progress)?;
            shared.parts_mut().0.mark_saved(cursor);
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
            let handle = cx.window_handle();
            cx.update(|_, app| super::windows::finish_pending(app, handle))
                .ok();
        });
    }

    /// Files dropped on the window open like the files of the open dialog.
    pub(crate) fn on_external_drop(
        &mut self,
        paths: &ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let paths: Vec<PathBuf> = paths.paths().to_vec();
        start(window, cx, async move |view, cx| {
            load::open_paths(&view, cx, paths, None).await;
        });
    }
}
