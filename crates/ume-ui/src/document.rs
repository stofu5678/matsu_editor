//! `Entity<SharedDocument>`: the document layer of ADR-0013 section 3.
//!
//! `ume_core::model::DocumentModel` is a plain struct that knows nothing of GPUI. GPUI's
//! `EventEmitter` cannot be implemented for it here (neither the trait nor the type is ours), so
//! the entity holds it in this small wrapper, together with what the views of one document share:
//! the path and a version number that counts the changes.
//!
//! The version solves a problem that section 3 does not mention. A view that runs a command
//! brings its own layout up to date at once (section 4). The `Changed` event that the same change
//! emits reaches that view only after the outermost update, and it must not lay the text out a
//! second time. So the view remembers the version its layout matches and ignores the events that
//! carry it; the other views of the document see a newer version and lay out again.

use std::path::{Path, PathBuf};

use gpui::{Context, EventEmitter};
use ume_core::buffer::{Document, Eol};
use ume_core::encoding::Encoding;
use ume_core::file::{Progress, SaveOptions, SaveReport, TextFile, UnmappableReport};
use ume_core::model::DocumentModel;

use crate::settings::NewDocumentDefaults;

/// What the document tells its views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentEvent {
    /// The text changed. `version` is the number [`SharedDocument::version`] has now.
    Changed { version: u64 },
}

/// The document of the editor with its undo history, shared by the views that show it.
pub struct SharedDocument {
    model: DocumentModel,
    path: Option<PathBuf>,
    version: u64,
    /// Typing overwrites instead of inserting. Sakura keeps the mode per document
    /// (`CDocEditor::m_bInsMode`) and starts in insert mode.
    overwrite: bool,
    /// View mode (`F_VIEWMODE`): every edit is refused. Sakura keeps it per window
    /// (`CAppMode::m_bViewMode`), clears it for a new document and keeps it when the file is
    /// reopened.
    view_mode: bool,
    /// What saving needs besides the text: the encoding, the BOM and the line ending. Its
    /// `document` is a placeholder; the text is `model`'s (see [`with_file`](Self::with_file)).
    file: TextFile,
}

impl EventEmitter<DocumentEvent> for SharedDocument {}

impl SharedDocument {
    pub fn new(model: DocumentModel, path: Option<PathBuf>) -> Self {
        let file = TextFile::new_untitled(Encoding::Utf8, false, model.new_line());
        Self {
            model,
            path,
            version: 0,
            overwrite: false,
            view_mode: false,
            file,
        }
    }

    /// A document from an opened file ([`crate::open::open_text_file`]): the text goes into the
    /// model, and the encoding, BOM and line ending stay for the status bar and for saving.
    pub fn from_text_file(mut file: TextFile) -> Self {
        let doc = std::mem::take(&mut file.document);
        let mut model = DocumentModel::new(doc);
        model.set_new_line(file.new_line);
        Self {
            model,
            path: file.path.clone(),
            version: 0,
            overwrite: false,
            view_mode: false,
            file,
        }
    }

    /// Replaces the whole document (open, close, reopen) and tells the views. The version keeps
    /// counting up across the replacement: a read that started before cannot mistake the new
    /// document for the one it saw, and a view cannot skip the re-layout for a reused number.
    pub fn replace_with(&mut self, new: SharedDocument, cx: &mut Context<Self>) -> u64 {
        let version = self.version;
        *self = new;
        self.version = version;
        self.changed(cx)
    }

    /// The encoding, BOM and line ending of the file (its `document` is empty: use [`doc`](Self::doc)).
    pub fn file(&self) -> &TextFile {
        &self.file
    }

    /// Gives an untitled, empty document the encoding, BOM and line ending of a new document
    /// ([`NewDocumentDefaults`]): what the status bar shows and what saving writes.
    pub fn set_new_document_defaults(&mut self, defaults: &NewDocumentDefaults) {
        self.set_format(defaults.encoding, defaults.bom);
        self.file.new_line = defaults.eol;
        self.model.set_new_line(defaults.eol);
    }

    /// Sets what a plain save writes (reopening or `F_FILE_REOPEN_*` of a file that is not there).
    pub fn set_format(&mut self, encoding: Encoding, bom: bool) {
        self.file.encoding = encoding;
        self.file.bom = bom;
    }

    /// Runs `f` on the file with the current text in it. The text is a cheap copy (the buffer
    /// shares its pieces), and the placeholder goes back afterwards.
    fn with_file<R>(&mut self, f: impl FnOnce(&mut TextFile) -> R) -> R {
        self.file.document = self.model.doc().clone();
        self.file.path = self.path.clone();
        let result = f(&mut self.file);
        self.file.document = Document::empty();
        result
    }

    /// Finds the characters that `options` cannot write ([`TextFile::check_save`]).
    pub fn check_save(
        &mut self,
        options: &SaveOptions,
        progress: Progress<'_>,
    ) -> ume_core::file::Result<UnmappableReport> {
        self.with_file(|file| file.check_save(options, progress))
    }

    /// Whether the line endings are mixed ([`TextFile::has_mixed_line_endings`]).
    pub fn has_mixed_line_endings(&mut self) -> ume_core::file::Result<bool> {
        self.with_file(|file| file.has_mixed_line_endings())
    }

    /// Writes the document to `path` ([`TextFile::save_as`]). On success the document takes the
    /// path, the encoding, the BOM and the line ending of `options`. The caller marks the
    /// undo history as saved, because the cursor is the view's.
    pub fn save_as(
        &mut self,
        path: PathBuf,
        options: &SaveOptions,
        progress: Progress<'_>,
    ) -> ume_core::file::Result<SaveReport> {
        let report = self.with_file(|file| file.save_as(path, options, progress))?;
        self.path = self.file.path.clone();
        if let Some(eol) = options.eol {
            self.model.set_new_line(eol);
        }
        Ok(report)
    }

    /// Writes the document to `path` for `F_PUTFILE`: like [`save_as`](Self::save_as), but the
    /// document keeps its own path, encoding, BOM and line ending (Sakura: "現在開いているファイル
    /// には影響しない").
    pub fn put_file(
        &mut self,
        path: PathBuf,
        options: &SaveOptions,
        progress: Progress<'_>,
    ) -> ume_core::file::Result<SaveReport> {
        let (encoding, bom, new_line, eol_on_save) = (
            self.file.encoding,
            self.file.bom,
            self.file.new_line,
            self.file.eol_on_save,
        );
        let result = self.with_file(|file| file.save_as(path, options, progress));
        self.file.path = self.path.clone();
        self.file.encoding = encoding;
        self.file.bom = bom;
        self.file.new_line = new_line;
        self.file.eol_on_save = eol_on_save;
        result
    }

    /// The line ending that Enter inserts, as the status bar names it.
    pub fn new_line(&self) -> Eol {
        self.model.new_line()
    }

    pub fn empty() -> Self {
        Self::new(DocumentModel::empty(), None)
    }

    pub fn model(&self) -> &DocumentModel {
        &self.model
    }

    pub fn doc(&self) -> &Document {
        self.model.doc()
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The file name for the title of the window.
    pub fn file_name(&self) -> Option<String> {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
    }

    /// Whether typing overwrites (overwrite mode) instead of inserting.
    pub fn overwrite(&self) -> bool {
        self.overwrite
    }

    /// Sets the typing mode. The views repaint (the caret and the status bar show it).
    pub fn set_overwrite(&mut self, overwrite: bool, cx: &mut Context<Self>) {
        if self.overwrite != overwrite {
            self.overwrite = overwrite;
            cx.notify();
        }
    }

    /// For a document that replaces this one (reopen): starts in the same mode.
    pub(crate) fn keep_view_mode(&mut self, view_mode: bool) {
        self.view_mode = view_mode;
    }

    /// Whether view mode is on.
    pub fn view_mode(&self) -> bool {
        self.view_mode
    }

    /// Turns view mode on or off. The views repaint (the title shows it).
    pub fn set_view_mode(&mut self, view_mode: bool, cx: &mut Context<Self>) {
        if self.view_mode != view_mode {
            self.view_mode = view_mode;
            cx.notify();
        }
    }

    /// Counts the changes of the text. A view compares it with the version its layout matches.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// The model and the path, for a [`CommandContext`](ume_commands::context::CommandContext).
    /// Whoever edits through it calls [`changed`](Self::changed) afterwards.
    pub fn parts_mut(&mut self) -> (&mut DocumentModel, Option<&Path>) {
        (&mut self.model, self.path.as_deref())
    }

    /// Tells the views that the text changed, and returns the new version.
    pub fn changed(&mut self, cx: &mut Context<Self>) -> u64 {
        self.version += 1;
        cx.emit(DocumentEvent::Changed {
            version: self.version,
        });
        cx.notify();
        self.version
    }
}
