//! Making the DIFF: reading the other side, computing the difference, marking the lines.
//!
//! Sakura: `Command_Diff` and `Command_Diff_Dialog` (`sakura_core/cmd/CViewCommander_Diff.cpp`) make
//! the two texts into files and `CEditView::ViewDiffInfo` (`sakura_core/view/CEditView_Diff.cpp`)
//! runs `diff.exe` on them. Here the texts are lines of strings and `ume_core::diff` does the
//! rest. The encoding of a file that is the other side is detected as when it is opened; a window
//! is read as it is on the screen, with its unsaved changes.
//!
//! The text and the difference are worked on in a task, and the marks are set when it is done,
//! if the document has not changed meanwhile (the marks would be on the wrong lines).

use std::path::Path;

use gpui::{Context, PromptLevel, Window};
use ume_core::buffer::{BufferError, Document};
use ume_core::diff::{DiffOptions, Hunk, diff_lines};
use ume_core::file::no_progress;
use ume_core::model::eol_text;

use super::{Target, find_document};
use crate::open::open_text_file;
use crate::settings::new_document_defaults;
use crate::view::EditorView;

/// `STR_ERR_DLGEDITVWDIFF1`: the file to compare with is not there.
pub(crate) const FILE_NOT_FOUND: &str =
    "差分コマンド実行は失敗しました。\n\n比較するファイルが見つかりません。";
/// `STR_ERR_DLGEDITVWDIFF4`.
pub(crate) const BINARY_FILE: &str = "DIFF差分を行おうとしたファイルはバイナリファイルです。";
/// `STR_ERR_DLGEDITVWDIFF5`.
pub(crate) const NO_DIFFERENCE: &str = "DIFF差分は見つかりませんでした。";
/// `STR_DIFF_FAILED`, with the reason after it (`STR_ERR_DLGEDITVWDIFF3`).
fn failed(reason: &str) -> String {
    format!("差分コマンド実行は失敗しました。\n\n{reason}")
}

/// The lines of a document with their line endings, which is what the comparison looks at
/// (`MakeDiffTmpFile` writes the lines with their endings).
pub(crate) fn document_lines(doc: &Document) -> Result<Vec<String>, BufferError> {
    doc.lines(0..doc.line_count())
        .map(|line| {
            line.map(|l| {
                let mut text = l.text;
                if let Some(eol) = l.eol {
                    text.push_str(eol_text(eol));
                }
                text
            })
        })
        .collect()
}

/// The lines of a file, decoded the way opening it would (`MakeDiffTmpFile2`).
fn file_lines(
    path: &Path,
    defaults: &crate::settings::NewDocumentDefaults,
) -> Result<Vec<String>, String> {
    let file = open_text_file(path, None, defaults, &mut no_progress).map_err(|e| e.to_string())?;
    document_lines(&file.document).map_err(|e| e.to_string())
}

/// `diff` reads a file that has a NUL as a binary file and does not compare it.
fn is_binary(lines: &[String]) -> bool {
    lines.iter().any(|l| l.contains('\0'))
}

/// What the task found.
enum Outcome {
    Binary,
    Hunks(Vec<Hunk>),
}

/// The difference of the two texts, with this window's text as the old or the new one.
fn compute(own: &[String], other: &[String], options: DiffOptions) -> Outcome {
    if is_binary(own) || is_binary(other) {
        return Outcome::Binary;
    }
    Outcome::Hunks(if options.own_is_old {
        diff_lines(own, other, options)
    } else {
        diff_lines(other, own, options)
    })
}

impl EditorView {
    /// A box with a warning and an OK button (Sakura's `WarningMessage`).
    pub(crate) fn warning_box(&self, message: &str, window: &mut Window, cx: &mut Context<Self>) {
        let answer = window.prompt(
            PromptLevel::Warning,
            "UME Editor",
            Some(message),
            &["OK"],
            cx,
        );
        crate::macros::keep_box(answer, window, cx);
    }

    /// `Command_Diff`: marks the lines of this window that differ from `target`. `flags` are the
    /// options of the macro `Diff` ([`DiffOptions`]).
    pub(crate) fn run_diff(
        &mut self,
        target: Target,
        flags: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let version = self.document.read(cx).version();
        // "今あるDIFF差分を消去する"; a reset or a newer comparison before this one is done
        // clears again, which makes this one void (see `finish_diff`)
        self.run_with(cx, |c| c.clear_diff_marks());
        let epoch = self.document.read(cx).model().diff_marks().epoch();
        let version = (version, epoch);
        match target {
            Target::Window(doc) => {
                let this = cx.entity_id();
                let other = find_document(cx, this, doc)
                    .map(|(_, view)| document_lines(view.read(cx).document().read(cx).doc()));
                match other {
                    Some(Ok(lines)) => {
                        let tracked = crate::macros::track(cx);
                        self.diff_with_lines(lines, flags, version, tracked, window, cx)
                    }
                    Some(Err(e)) => self.warning_box(&failed(&e.to_string()), window, cx),
                    None => self.warning_box(FILE_NOT_FOUND, window, cx),
                }
            }
            Target::File(path) => {
                // `IsFileExists( szDiffFile2, true )`
                if !path.is_file() {
                    self.warning_box(FILE_NOT_FOUND, window, cx);
                    return;
                }
                let mut tracked = crate::macros::track(cx);
                cx.spawn_in(window, async move |this, cx| {
                    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
                    let defaults = cx
                        .update(|_, app| new_document_defaults(app, name.as_deref()))
                        .unwrap_or_default();
                    let read = cx
                        .background_executor()
                        .spawn(async move { file_lines(&path, &defaults) })
                        .await;
                    this.update_in(cx, |this, window, cx| match read {
                        // the guard goes on to the comparison, so that the wait has no gap
                        Ok(lines) => {
                            this.diff_with_lines(lines, flags, version, tracked.take(), window, cx)
                        }
                        Err(reason) => this.warning_box(&failed(&reason), window, cx),
                    })
                    .ok();
                })
                .detach();
            }
        }
    }

    /// The rest of `ViewDiffInfo` once the other text is there: the old marks go, then the
    /// difference is computed and marked.
    fn diff_with_lines(
        &mut self,
        other: Vec<String>,
        flags: u32,
        version: (u64, u64),
        tracked: Option<crate::macros::FlowGuard>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let own = match document_lines(self.document.read(cx).doc()) {
            Ok(lines) => lines,
            Err(e) => {
                self.warning_box(&failed(&e.to_string()), window, cx);
                return;
            }
        };
        let options = DiffOptions::from_flags(flags);
        cx.spawn_in(window, async move |this, cx| {
            let _tracked = tracked;
            let outcome = cx
                .background_executor()
                .spawn(async move { compute(&own, &other, options) })
                .await;
            this.update_in(cx, |this, window, cx| {
                this.finish_diff(outcome, options, version, window, cx)
            })
            .ok();
        })
        .detach();
    }

    fn finish_diff(
        &mut self,
        outcome: Outcome,
        options: DiffOptions,
        (version, epoch): (u64, u64),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.document.read(cx).model().diff_marks().epoch() != epoch {
            return;
        }
        if self.document.read(cx).version() != version {
            self.notice = Some("DIFFの間に文書が変わったので、差分を表示しなかった".to_owned());
            cx.notify();
            return;
        }
        match outcome {
            Outcome::Binary => self.warning_box(BINARY_FILE, window, cx),
            Outcome::Hunks(hunks) => {
                let marked = self.run_with(cx, |c| c.set_diff_marks(&hunks, options.own_is_old));
                // Sakura's `ViewDiffInfo` asks `nFlgOpt & 0x0040` here, but has reused `nFlgOpt`
                // for the flags of `ExecCmd` (0x40 always set) by then, so Sakura shows the
                // message whenever nothing differs, whatever the check box 「DIFF差分がない
                // ときにメッセージを表示」 or the bit 0x40 of the macro `Diff` says. UME Editor
                // follows the check box and the bit on purpose (the user's decision, 2026-10-17).
                if !marked && options.notify_no_diff {
                    self.info_box(NO_DIFFERENCE, window, cx);
                }
            }
        }
    }
}
