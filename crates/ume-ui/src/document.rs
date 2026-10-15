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
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;

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
}

impl EventEmitter<DocumentEvent> for SharedDocument {}

impl SharedDocument {
    pub fn new(model: DocumentModel, path: Option<PathBuf>) -> Self {
        Self {
            model,
            path,
            version: 0,
        }
    }

    pub fn empty() -> Self {
        Self::new(DocumentModel::empty(), None)
    }

    /// Opens `path` ([`crate::open::open_document`]).
    pub fn open(path: &Path) -> Result<Self, crate::open::OpenError> {
        let doc = crate::open::open_document(path)?;
        Ok(Self::new(DocumentModel::new(doc), Some(path.to_owned())))
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
