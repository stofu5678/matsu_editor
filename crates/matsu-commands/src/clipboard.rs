//! The clipboard that the clipboard commands use, behind a trait so that this crate does not
//! depend on GPUI (ADR-0013 section 4).
//!
//! `matsu-ui` implements [`Clipboard`] over the OS clipboard. [`MemoryClipboard`] is the
//! implementation for tests and for macros that run without a window.
//!
//! The text is plain text. Besides the text, Sakura marks two kinds of copies so that a later
//! paste can treat them differently (`MySetClipboardData` in `sakura_core/view/CEditView.cpp`
//! and `CClipboard::SetText` in `sakura_core/_os/CClipboard.cpp`): a line copy
//! (`MSDEVLineSelect`) and a box copy (`MSDEVColumnSelect`). [`ClipKind`] is that mark.

/// The most bytes of the document that one copy or cut takes (ADR-0020, chapter 5 and 9-8): the
/// selection of a huge file is a range, but the clipboard and the string that carries it into it
/// are not, so a bigger selection is refused with [`too_big_message`] and nothing is deleted. The
/// macro functions `GetSelectedString` and `GetLineStr` have the same limit. The bytes are those
/// the document holds, in the file's own encoding.
pub const MAX_COPY_BYTES: usize = 256 * 1024 * 1024;

thread_local! {
    static COPY_LIMIT: std::cell::Cell<usize> = const { std::cell::Cell::new(MAX_COPY_BYTES) };
}

/// The limit that is in force: [`MAX_COPY_BYTES`], or a smaller one that a test asked for on this
/// thread (`with_copy_limit`, for tests).
pub fn copy_limit() -> usize {
    COPY_LIMIT.with(std::cell::Cell::get)
}

/// Runs `f` with `limit` bytes as the limit of a copy on this thread, so that a test does not need
/// a selection of 256 MiB. The old limit comes back when `f` ends, also by a panic.
#[cfg(any(test, feature = "test-util"))]
#[doc(hidden)]
pub fn with_copy_limit<R>(limit: usize, f: impl FnOnce() -> R) -> R {
    struct Restore(usize);
    impl Drop for Restore {
        fn drop(&mut self) {
            COPY_LIMIT.with(|l| l.set(self.0));
        }
    }
    let _restore = Restore(COPY_LIMIT.with(|l| l.replace(limit)));
    f()
}

/// `bytes` in the megabytes that the messages about big texts show (MiB, rounded up, at least 1).
pub fn mib(bytes: usize) -> usize {
    bytes.div_ceil(1024 * 1024).max(1)
}

/// What the editor says when a copy or a cut of `bytes` bytes is refused (Sakura has no limit and
/// no text for it, ADR-0018 type F; the ADR asks for a refusal that tells the way out).
pub fn too_big_message(bytes: usize, cut: bool) -> String {
    format!(
        "選択範囲が大きすぎる（{} MB。上限は {} MB）ので、{}できません。{}範囲を小さくしてやり直してください。",
        mib(bytes),
        mib(copy_limit()),
        if cut { "切り取り" } else { "コピー" },
        if cut {
            "文字は削除していません。"
        } else {
            ""
        },
    )
}

/// What the editor says when a command that needs the text of a range, to sort it or to convert
/// it, is refused because the range has `bytes` bytes or more (the limit is the one of a copy,
/// ADR-0020; Sakura has no limit and no text for it, ADR-0018 type F). `what` is the name of the
/// work, for example `並べ替え`.
pub fn too_big_to_work_on_message(bytes: usize, what: &str) -> String {
    format!(
        "対象の範囲が大きすぎる（{} MB。上限は {} MB）ので、{what}できません。文字は変更していません。範囲を小さくしてやり直してください。",
        mib(bytes),
        mib(copy_limit()),
    )
}

/// How the text on the clipboard was copied.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClipKind {
    /// Copied from a selection, or from outside the editor.
    #[default]
    Normal,
    /// Copied without a selection: the whole row. Sakura's `MSDEVLineSelect`. A paste puts it at
    /// the start of the row instead of at the caret.
    Line,
    /// Copied from a rectangular selection. Sakura's `MSDEVColumnSelect`. `paste` pastes it as a
    /// box (`bAutoColumnPaste`).
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

    /// Empties the clipboard, so that [`get`](Self::get) finds no text (Sakura: `CClipboard::Empty`,
    /// `EmptyClipboard`).
    fn clear(&mut self);
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

    fn clear(&mut self) {
        self.data = None;
    }
}
