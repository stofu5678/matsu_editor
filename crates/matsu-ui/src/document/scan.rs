//! A file opened by ranges: the scan that runs behind its document (ADR-0020, chapter 2).
//!
//! While the scan runs the document shows the scanned start of the file and is read-only
//! ([`matsu_core::buffer::Document::is_read_only`]). [`SharedDocument::scan_tick`] is called now
//! and then by a task of the view: it puts a document over a larger start in place of the old one
//! (each time the scanned part has doubled, so that the layout, which starts again from the top
//! with a new document, is not reset every moment), and at the end the complete document, which
//! can be edited.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use gpui::Context;
use matsu_core::buffer::{Document, ScanJob, ScanState, WINDOW_BYTES};

use super::SharedDocument;

/// The shortest time between two new documents of a running scan.
const MIN_SWAP_INTERVAL: Duration = Duration::from_millis(300);

static NEXT_SCAN_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_FRAME_ID: AtomicU64 = AtomicU64::new(1);

/// A window on the file that is shown in place of the scanned start (ADR-0020, chapter 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScanWindowInfo {
    /// Where the window starts in the file, in bytes; it ends at `end`.
    pub start: u64,
    pub end: u64,
    /// The lines before the first line of the window.
    pub lines_before: u64,
    /// Whether `lines_before` is exact; if not, it is estimated from the part scanned.
    pub exact: bool,
}

/// How the line numbers of the document are counted: from `lines_before` lines before its first
/// line, which is exact or "about" (「約」).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LineBase {
    pub lines_before: u64,
    pub approx: bool,
}

/// The document the positions of the views are in: it changes when a window replaces the
/// document, a window is put elsewhere, or the scan is done. A view that sees another number
/// than its own moves its caret and its scroll along ([`FrameChange`]).
pub(super) struct ScanFrame {
    seq: u64,
    /// The byte of the file where the text of the document starts.
    start: u64,
    change: Option<FrameChange>,
}

impl Default for ScanFrame {
    fn default() -> Self {
        Self {
            seq: NEXT_FRAME_ID.fetch_add(1, Ordering::Relaxed),
            start: 0,
            change: None,
        }
    }
}

/// What the document was before the last change of its frame: positions of that document are
/// moved to the new one by the bytes of the file they stand for.
#[derive(Clone)]
pub(crate) struct FrameChange {
    pub from_seq: u64,
    pub from_start: u64,
    pub from_doc: Document,
}

/// The scan of the file behind a document.
pub(super) struct ScanWatch {
    job: ScanJob,
    id: u64,
    last_swap: Instant,
    /// How many bytes the document shows.
    shown: u64,
    /// Why the scan failed, once it has.
    failure: Option<String>,
    /// The window on the file that the document is, if the user moved to a place the scan had not
    /// reached.
    window: Option<ScanWindowInfo>,
    /// The bookmarks of the file while a window is shown: their lines are the file's, not the
    /// window's. They come back with the whole file (the marks set in a window are dropped).
    bookmarks: Option<matsu_core::bookmarks::Bookmarks>,
}

/// What [`SharedDocument::scan_tick`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScanTick {
    /// There is no scan (any more) for the caller's number.
    Gone,
    /// The scan goes on.
    Running,
    /// The scan is done: the document is complete and can be edited.
    Finished,
    /// The scan stopped with an error: the document stays what it was, read-only.
    Failed,
}

/// `1234567` as `1,234,567`.
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl SharedDocument {
    /// Starts to watch the scan `job` of the file the document was made from.
    pub(super) fn attach_scan(&mut self, job: ScanJob) {
        self.scan = Some(ScanWatch {
            shown: self.model.doc().raw_len() as u64,
            job,
            id: NEXT_SCAN_ID.fetch_add(1, Ordering::Relaxed),
            last_swap: Instant::now(),
            failure: None,
            window: None,
            bookmarks: None,
        });
    }

    /// The number of the scan that is watched (a task of a view stops when it is not its own any
    /// more, or gone).
    pub(crate) fn scan_id(&self) -> Option<u64> {
        self.scan.as_ref().map(|s| s.id)
    }

    /// The index of the running scan (a copy of what is scanned so far), whose source is the open
    /// file behind the document; `None` when there is no scan or it has failed.
    pub(crate) fn scan_index(&self) -> Option<matsu_core::buffer::BlockIndex> {
        self.scan
            .as_ref()
            .filter(|s| s.failure.is_none())
            .map(|s| s.job.index_snapshot())
    }

    /// Whether the file is read by ranges (ADR-0020): a huge file, whose long lines are cut, whose
    /// lines far down are not colored from the top, and that has no outline.
    pub(crate) fn is_ranged(&self) -> bool {
        self.ranged
    }

    /// Whether the scan of number `id` ended with the complete document.
    pub(crate) fn scan_finished(&self, id: u64) -> bool {
        self.scan_done == Some(id)
    }

    /// Whether the scan is running.
    pub(crate) fn is_scanning(&self) -> bool {
        self.scan.as_ref().is_some_and(|s| s.failure.is_none())
    }

    /// Whether the document shows only a start of its file: edits and saves are refused.
    pub(crate) fn is_partial(&self) -> bool {
        self.model.doc().is_read_only()
    }

    /// The text for the status bar about the scan, 「改行位置を調べています 35%」: how far it is
    /// and the number of lines the file will have (estimated from the part scanned), or why the
    /// scan stopped.
    pub(crate) fn scan_text(&self) -> Option<String> {
        let scan = self.scan.as_ref()?;
        if let Some(why) = &scan.failure {
            return Some(format!(
                "改行位置を調べられなかったので、読み取り専用のままです ({why})"
            ));
        }
        let p = scan.job.progress();
        // The last line has no line ending when the file does not end with one: one more line
        // than the endings, which the estimate cannot know; it is "about" anyway.
        Some(format!(
            "改行位置を調べています {}% (約 {} 行)",
            p.percent(),
            grouped(p.estimated_breaks())
        ))
    }

    /// Why `what` (「編集」 or 「保存」) is refused when the document shows only a start of its file,
    /// for the status bar and the message of a save.
    pub(crate) fn partial_reason(&self, what: &str) -> Option<String> {
        if !self.is_partial() {
            None
        } else if self.is_scanning() {
            Some(format!("改行位置を調べている間は{what}できません"))
        } else {
            Some(format!(
                "ファイルを最後まで読めなかったので{what}できません"
            ))
        }
    }

    /// Looks at the scan of number `id` and brings the document up to date: a document over the
    /// part scanned so far when it has grown a lot, the complete document when the scan is done.
    /// Tells the views when the document was replaced.
    pub(crate) fn scan_tick(&mut self, id: u64, cx: &mut Context<Self>) -> ScanTick {
        let Some(scan) = self.scan.as_mut().filter(|s| s.id == id) else {
            return ScanTick::Gone;
        };
        let id = scan.id;
        if scan.failure.is_some() {
            return ScanTick::Failed;
        }
        let progress = scan.job.progress();
        match progress.state {
            ScanState::Running => {
                // A window stays as it is: the document is the place the user moved to, not the
                // start of the file. Its line numbers become exact when the scan passes it.
                if let Some(window) = scan.window.as_mut() {
                    if !window.exact
                        && let Some(n) = scan.job.exact_lines_before(window.start)
                    {
                        window.lines_before = n;
                        window.exact = true;
                        cx.notify();
                    }
                    return ScanTick::Running;
                }
                let grown = progress.scanned >= scan.shown.saturating_mul(2);
                if grown && scan.last_swap.elapsed() >= MIN_SWAP_INTERVAL {
                    scan.last_swap = Instant::now();
                    if let Ok(doc) = scan.job.snapshot() {
                        // The worker may have scanned the rest since the state was read: the
                        // document covers the whole file and can be edited, so the scan is over
                        // now. A later poll must not put the same text over an edit.
                        if !doc.is_read_only() {
                            self.complete_scan(id, doc, cx);
                            return ScanTick::Finished;
                        }
                        scan.shown = doc.raw_len() as u64;
                        self.model.replace_scanned_document(doc);
                        self.changed(cx);
                    }
                }
                ScanTick::Running
            }
            ScanState::Done => match scan.job.snapshot() {
                // The scan is done, so this document covers the whole file.
                Ok(doc) => {
                    self.complete_scan(id, doc, cx);
                    ScanTick::Finished
                }
                Err(e) => {
                    self.scan_failed(e.to_string(), cx);
                    ScanTick::Failed
                }
            },
            ScanState::Cancelled | ScanState::Failed => {
                let message = scan
                    .job
                    .failure()
                    .map_or_else(|| "中断された".to_owned(), |f| f.message);
                self.scan_failed(message, cx);
                ScanTick::Failed
            }
        }
    }

    /// The scan is over: `doc` covers the whole file and takes the place of the document.
    fn complete_scan(
        &mut self,
        id: u64,
        doc: matsu_core::buffer::Document,
        cx: &mut Context<Self>,
    ) {
        self.scan_done = Some(id);
        let was_window = self.scan.as_ref().is_some_and(|s| s.window.is_some());
        let stashed = self.scan.as_mut().and_then(|s| s.bookmarks.take());
        self.scan = None;
        if was_window {
            // The views are in the window: they follow it to the same place of the whole file.
            self.switch_frame(doc, 0);
            if let Some(marks) = stashed {
                *self.model.bookmarks_mut() = marks;
            }
        } else {
            self.model.replace_scanned_document(doc);
        }
        self.forget_tsv_stops();
        self.changed(cx);
    }

    /// The scan stopped: what was scanned stays on the screen, read-only. The file was probably
    /// changed by another program.
    fn scan_failed(&mut self, message: String, cx: &mut Context<Self>) {
        let Some(scan) = self.scan.as_mut() else {
            return;
        };
        scan.failure = Some(message);
        // The part that was scanned is shown up to the last block (a window stays).
        if scan.window.is_some() {
            self.forget_tsv_stops();
            self.changed(cx);
            return;
        }
        if let Ok(doc) = scan.job.snapshot() {
            scan.shown = doc.raw_len() as u64;
            self.model.replace_scanned_document(doc);
        }
        self.forget_tsv_stops();
        self.changed(cx);
    }
}

/// Moving to the parts of the file that the scan has not reached (ADR-0020, chapter 2).
impl SharedDocument {
    /// The number of the document the positions of the views are in ([`ScanFrame`]).
    pub(crate) fn frame_seq(&self) -> u64 {
        self.frame.seq
    }

    /// The byte of the file where the text of the document starts: 0 unless it is a window.
    pub(crate) fn frame_start(&self) -> u64 {
        self.frame.start
    }

    /// What the document was before the last change of its frame.
    pub(crate) fn frame_change(&self) -> Option<&FrameChange> {
        self.frame.change.as_ref()
    }

    /// Puts `doc`, whose text starts at byte `start` of the file, in place of the document.
    fn switch_frame(&mut self, doc: Document, start: u64) {
        self.frame.change = Some(FrameChange {
            from_seq: self.frame.seq,
            from_start: self.frame.start,
            from_doc: self.model.doc().clone(),
        });
        self.frame.seq = NEXT_FRAME_ID.fetch_add(1, Ordering::Relaxed);
        self.frame.start = start;
        self.model.replace_scanned_document(doc);
    }

    /// The window on the file that the document is, while the scan runs.
    pub(crate) fn scan_window(&self) -> Option<ScanWindowInfo> {
        self.scan.as_ref().and_then(|s| s.window)
    }

    /// How the line numbers of the document are counted.
    pub(crate) fn line_base(&self) -> LineBase {
        self.scan_window()
            .map_or_else(LineBase::default, |w| LineBase {
                lines_before: w.lines_before,
                approx: !w.exact,
            })
    }

    /// The length of the file and the number of lines it will have (estimated from the part
    /// scanned), while the scan runs: what the scroll bar covers.
    pub(crate) fn scan_extent(&self) -> Option<(u64, u64)> {
        let scan = self.scan.as_ref().filter(|s| s.failure.is_none())?;
        let p = scan.job.progress();
        Some((p.total, p.estimated_breaks() + 1))
    }

    /// The byte of the file where line `line` (0-based) starts, and whether that is exact (the
    /// scan has passed the line) or estimated from the part scanned.
    pub(crate) fn scan_line_byte(&self, line: u64) -> Option<(u64, bool)> {
        let scan = self.scan.as_ref()?;
        Some(match scan.job.line_start_byte(line) {
            Some(byte) => (byte, true),
            None => (scan.job.progress().estimated_byte_of_line(line), false),
        })
    }

    /// Shows a window of the file around byte `byte` in place of the document, for a place the
    /// scan has not reached. Reads only that part of the file. Returns whether it did: not when
    /// the lines there are too long to cut a window out, or the file cannot be read (the caller
    /// waits for the scan then).
    pub(crate) fn open_window_at(&mut self, byte: u64, cx: &mut Context<Self>) -> bool {
        let Some(scan) = self.scan.as_ref().filter(|s| s.failure.is_none()) else {
            return false;
        };
        let Ok(Some(window)) = scan.job.open_window(byte, WINDOW_BYTES) else {
            return false;
        };
        let info = ScanWindowInfo {
            start: window.start,
            end: window.end,
            lines_before: window.lines_before,
            exact: window.exact,
        };
        self.switch_frame(window.document, window.start);
        // The bookmarks count lines of the file: the window's lines are others.
        let marks = std::mem::take(self.model.bookmarks_mut());
        if let Some(scan) = self.scan.as_mut() {
            if scan.bookmarks.is_none() {
                scan.bookmarks = Some(marks);
            }
            scan.window = Some(info);
        }
        self.forget_tsv_stops();
        self.changed(cx);
        true
    }
}
