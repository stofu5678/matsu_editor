//! Sakura's DIFF差分表示 and ファイル内容比較 (検索 menu): `F_DIFF_DIALOG`, `F_DIFF`,
//! `F_DIFF_NEXT`, `F_DIFF_PREV`, `F_DIFF_RESET` and `F_COMPARE`.
//!
//! * The difference and the marks are `ume_core::diff` and the commands of `ume-commands` (the
//!   marks belong to the document and follow its edits like the bookmarks). This module is the
//!   part that needs windows and files.
//! * [`run`] is `CEditView::ViewDiffInfo` and `Command_Diff`: read the text of the other side (a
//!   file, detected and decoded as when it is opened, or another window with its unsaved changes),
//!   compute the difference on a background task and mark the lines. Sakura writes both texts to
//!   temporary files and runs `diff.exe`; there is no such program here, `ume_core::diff` does it.
//! * [`panel`] is the DIFF dialog (`CDlgDiff`) and [`compare`] the dialog of the comparison
//!   (`CDlgCompare`) and the comparison itself. Like the other dialogs they are panels in the
//!   editor window.
//! * [`jump`] is the move to the next and the previous marked line.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Diff.cpp`, `sakura_core/view/CEditView_Diff.cpp`,
//! `sakura_core/docplus/CDiffManager.cpp`, `sakura_core/dlg/CDlgDiff.cpp`,
//! `sakura_core/dlg/CDlgCompare.cpp`.

pub(crate) mod compare;
pub(crate) mod jump;
pub(crate) mod panel;
pub(crate) mod run;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use gpui::{AnyWindowHandle, App, Entity, EntityId, Global};
use ume_core::encoding::Encoding;

use crate::file_ops::windows::editors;
use crate::view::EditorView;
use crate::window_ops::list::label;

use self::compare::ComparePanel;
use self::panel::DiffPanel;

/// The dialogs of the view, while they are open.
#[derive(Default)]
pub(crate) struct DiffState {
    pub(crate) panel: Option<Entity<DiffPanel>>,
    pub(crate) compare: Option<Entity<ComparePanel>>,
}

/// What the dialogs keep from one use to the next (Sakura keeps them in the shared data,
/// `m_nDiffFlgOpt` and `m_sCompare.m_bCompareAndTileHorz`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Remembered {
    /// The flags of the macro `Diff` (`ume_core::diff::DiffOptions`).
    pub(crate) flags: u32,
    /// 比較後、左右に並べて表示.
    pub(crate) tile: bool,
}

impl Default for Remembered {
    fn default() -> Self {
        Self {
            // Sakura starts with `m_nDiffFlgOpt = 0` (`CShareData::InitShareData`), but its users
            // always saw the message "no difference" (see `finish_diff`), so the box starts
            // checked (0x0040) and keeps that. A value that is read from a settings file, 0
            // included, is used as it is. `m_bCompareAndTileHorz = TRUE`.
            flags: ume_core::diff::DiffOptions::NOTIFY_NO_DIFF,
            tile: true,
        }
    }
}

impl Global for Remembered {}

pub(crate) fn remembered(cx: &App) -> Remembered {
    cx.try_global::<Remembered>().copied().unwrap_or_default()
}

pub(crate) fn remember(cx: &mut App, value: Remembered) {
    cx.set_global(value);
}

/// The other side of a comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    /// A file (外部ファイルを指定).
    File(PathBuf),
    /// Another document that is open (他の編集中のファイル): a window, or a tab of one.
    Window(DocRef),
}

/// An open document: its window, and the document itself. A window with tabs has several, so the
/// window alone does not say which one is meant; the document stays the same when the user
/// changes panes or the tab moves to another window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DocRef {
    pub(crate) handle: AnyWindowHandle,
    pub(crate) document: EntityId,
}

/// The view that opened a dialog and its document, which are not candidates for the other side.
/// The view may be borrowed while the dialog opens, so it is skipped by its id, not read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Owner {
    pub(crate) view: EntityId,
    pub(crate) document: EntityId,
}

/// A document that can be the other side, as the lists of the dialogs show it.
#[derive(Clone)]
pub(crate) struct Candidate {
    pub(crate) doc: DocRef,
    /// The label of the window list: the number, the name, ` *` when modified and the encoding.
    pub(crate) label: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) encoding: Encoding,
}

/// The open documents but the document of `owner`, in the order of the window list (whose numbers the
/// labels use).
pub(crate) fn candidates(cx: &mut App, owner: Owner) -> Vec<Candidate> {
    editors(cx)
        .into_iter()
        .enumerate()
        .filter(|(_, (_, view))| {
            view.entity_id() != owner.view && view.read(cx).document().entity_id() != owner.document
        })
        .map(|(i, (handle, view))| {
            let document = view.read(cx).document();
            let doc = DocRef {
                handle,
                document: document.entity_id(),
            };
            let shared = document.read(cx);
            Candidate {
                doc,
                label: label(
                    i,
                    shared.path(),
                    shared.model().is_modified(),
                    shared.file().encoding,
                ),
                path: shared.path().map(Path::to_path_buf),
                encoding: shared.file().encoding,
            }
        })
        .collect()
}

/// The window and the view of the open document `doc`. `this` is the view that asks, which is
/// borrowed and so is not read.
pub(crate) fn find_document(
    cx: &mut App,
    this: EntityId,
    doc: DocRef,
) -> Option<(AnyWindowHandle, Entity<EditorView>)> {
    editors(cx).into_iter().find(|(_, view)| {
        view.entity_id() != this && view.read(cx).document().entity_id() == doc.document
    })
}

/// The length of the longest part that two names have in common, ignoring case
/// (`FileMatchScore` in `sakura_core/util/file.cpp`).
fn common_part(a: &str, b: &str) -> usize {
    let fold = |s: &str| -> Vec<char> { s.chars().flat_map(char::to_lowercase).collect() };
    let (a, b) = (fold(a), fold(b));
    // dynamic programming over the end of the common run
    let mut best = 0;
    let mut run = vec![0usize; b.len() + 1];
    for &x in &a {
        let mut previous = 0;
        for (j, &y) in b.iter().enumerate() {
            let diagonal = previous;
            previous = run[j + 1];
            run[j + 1] = if x == y { diagonal + 1 } else { 0 };
            best = best.max(run[j + 1]);
        }
    }
    best
}

/// How well the file names of two paths match: the common part of the names without the extension
/// plus the common part of the extensions (`FileMatchScoreSepExt`).
pub(crate) fn name_score(a: &Path, b: &Path) -> usize {
    // the extension has its dot, as `std::filesystem::path::extension` gives it
    let part = |p: &Path, ext: bool| -> String {
        if ext {
            p.extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default()
        } else {
            p.file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        }
    };
    common_part(&part(a, false), &part(b, false)) + common_part(&part(a, true), &part(b, true))
}

/// The window that the dialogs select first: the one whose file name matches that of this window
/// best, and among equal ones the first that has the encoding of this window
/// (`CDlgDiff::SetData`, `CDlgCompare::SetData`; the latter has no rule for the encoding).
pub(crate) fn best_candidate(
    own_path: Option<&Path>,
    own_encoding: Encoding,
    candidates: &[Candidate],
    by_encoding: bool,
) -> Option<usize> {
    let mut best: Option<(usize, usize, Encoding)> = None;
    for (i, c) in candidates.iter().enumerate() {
        let score = match (own_path, c.path.as_deref()) {
            (Some(a), Some(b)) => name_score(a, b),
            _ => 0,
        };
        let better = match best {
            None => true,
            Some((_, top, top_encoding)) => {
                score > top
                    || (by_encoding
                        && score == top
                        && top_encoding != own_encoding
                        && c.encoding == own_encoding)
            }
        };
        if better {
            best = Some((i, score, c.encoding));
        }
    }
    best.map(|(i, ..)| i)
}
