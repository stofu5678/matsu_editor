//! The clipboard that the clipboard commands use, behind a trait so that this crate does not
//! depend on GPUI (ADR-0013 section 4).
//!
//! `ume-ui` implements [`Clipboard`] over the OS clipboard. [`MemoryClipboard`] is the
//! implementation for tests and for macros that run without a window.
//!
//! The text is plain text. Besides the text, Sakura marks two kinds of copies so that a later
//! paste can treat them differently (`MySetClipboardData` in `sakura_core/view/CEditView.cpp`
//! and `CClipboard::SetText` in `sakura_core/_os/CClipboard.cpp`): a line copy
//! (`MSDEVLineSelect`) and a box copy (`MSDEVColumnSelect`). [`ClipKind`] is that mark.

/// How the text on the clipboard was copied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClipKind {
    /// Copied from a selection, or from outside the editor.
    #[default]
    Normal,
    /// Copied without a selection: the whole row. Sakura's `MSDEVLineSelect`. A paste puts it at
    /// the start of the row instead of at the caret.
    Line,
    /// Copied from a rectangular selection. Sakura's `MSDEVColumnSelect`. Pasting it as a box is
    /// not implemented yet; it is pasted as normal text.
    Box,
}

/// The content of the clipboard.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClipData {
    pub text: String,
    pub kind: ClipKind,
}

impl ClipData {
    pub fn new(text: impl Into<String>, kind: ClipKind) -> Self {
        Self {
            text: text.into(),
            kind,
        }
    }
}

/// A clipboard that holds text.
pub trait Clipboard {
    /// The text on the clipboard, or `None` if there is no text (empty, or an image).
    fn get(&mut self) -> Option<ClipData>;

    /// Replaces the content of the clipboard. Returns `false` if the clipboard could not be
    /// written (Sakura: `MySetClipboardData` fails and the command beeps).
    fn set(&mut self, data: ClipData) -> bool;
}

/// A clipboard in memory, for tests.
#[derive(Clone, Debug, Default)]
pub struct MemoryClipboard {
    data: Option<ClipData>,
}

impl MemoryClipboard {
    /// A clipboard that holds `text` as a normal copy.
    pub fn with_text(text: &str) -> Self {
        Self {
            data: Some(ClipData::new(text, ClipKind::Normal)),
        }
    }

    /// The content, without the trait.
    pub fn data(&self) -> Option<&ClipData> {
        self.data.as_ref()
    }
}

impl Clipboard for MemoryClipboard {
    fn get(&mut self) -> Option<ClipData> {
        self.data.clone()
    }

    fn set(&mut self, data: ClipData) -> bool {
        self.data = Some(data);
        true
    }
}
