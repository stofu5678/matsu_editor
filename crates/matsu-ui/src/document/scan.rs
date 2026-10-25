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
use matsu_core::buffer::{ScanJob, ScanState};

use super::SharedDocument;

/// The shortest time between two new documents of a running scan.
const MIN_SWAP_INTERVAL: Duration = Duration::from_millis(300);

static NEXT_SCAN_ID: AtomicU64 = AtomicU64::new(1);

/// The scan of the file behind a document.
pub(super) struct ScanWatch {
    job: ScanJob,
    id: u64,
    last_swap: Instant,
    /// How many bytes the document shows.
    shown: u64,
    /// Why the scan failed, once it has.
    failure: Option<String>,
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
        });
    }

    /// The number of the scan that is watched (a task of a view stops when it is not its own any
    /// more, or gone).
    pub(crate) fn scan_id(&self) -> Option<u64> {
        self.scan.as_ref().map(|s| s.id)
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
        self.scan = None;
        self.model.replace_scanned_document(doc);
        self.changed(cx);
    }

    /// The scan stopped: what was scanned stays on the screen, read-only. The file was probably
    /// changed by another program.
    fn scan_failed(&mut self, message: String, cx: &mut Context<Self>) {
        let Some(scan) = self.scan.as_mut() else {
            return;
        };
        scan.failure = Some(message);
        // The part that was scanned is shown up to the last block.
        if let Ok(doc) = scan.job.snapshot() {
            scan.shown = doc.raw_len() as u64;
            self.model.replace_scanned_document(doc);
        }
        self.changed(cx);
    }
}
