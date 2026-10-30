//! What the view does while the file behind its document is being scanned (ADR-0020, chapter 2):
//! it follows the scan, and holds back the jumps to places the scan has not reached.

use std::time::Duration;

use gpui::{App, Context};
use matsu_commands::command::{Arg, Registration};
use matsu_config::history::FileEntry;
use matsu_core::buffer::Pos;

use super::EditorView;
use crate::document::ScanTick;
use crate::history::HistoryOptions;

/// How often the task of the view looks at the scan.
const POLL: Duration = Duration::from_millis(100);

/// A jump that waits for the scan: the scan it waits for, the command and its arguments, and the
/// hold on the macro that asked for it (the next step of the macro waits until the jump has run).
pub(crate) type PendingScanJump = (
    u64,
    &'static Registration,
    Vec<Arg>,
    Option<crate::macros::wait::FlowGuard>,
);

/// What is restored after a file is opened when the scan has not reached it yet.
pub(crate) struct ScanRestore {
    /// The scan the restore waits for.
    id: u64,
    /// Where the caret was put when the file was opened: it is restored only if it is still there.
    at_load: Pos,
    /// The place of a reload.
    caret: Option<Pos>,
    /// The place and the bookmarks the history remembers; the history is written after them.
    history: Option<(FileEntry, HistoryOptions)>,
}

impl ScanRestore {
    /// The entry the history had for the file, to write again as it is.
    pub(crate) fn remembered(&self) -> Option<FileEntry> {
        self.history.as_ref().map(|(entry, _)| entry.clone())
    }
}

impl EditorView {
    /// Starts the task that follows the scan of the document's file, if it has one: it shows the
    /// scanned part as it grows and the complete document at the end. The task ends when the
    /// document is replaced or the window is gone.
    pub(crate) fn watch_scan(&mut self, cx: &mut Context<Self>) {
        // The task follows the document, not this view: another pane of the document goes on
        // when this one is closed. It ends when the document is gone.
        let document = self.document.downgrade();
        let Some(id) = self.document.read(cx).scan_id() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                let Ok(tick) = document.update(cx, |shared, cx| shared.scan_tick(id, cx)) else {
                    break;
                };
                match tick {
                    // the percentage of the status bar
                    ScanTick::Running => {
                        this.update(cx, |_, cx| cx.notify()).ok();
                    }
                    ScanTick::Gone => break,
                    ScanTick::Finished | ScanTick::Failed => {
                        this.update(cx, |view, cx| view.scan_ended(tick, cx)).ok();
                        break;
                    }
                }
            }
        })
        .detach();
    }

    /// The scan is done: the document can be edited now.
    fn scan_ended(&mut self, tick: ScanTick, cx: &mut Context<Self>) {
        self.notice = if tick == ScanTick::Finished {
            let lines = self.document.read(cx).doc().line_count();
            Some(format!("改行位置を調べ終えました ({lines} 行)"))
        } else {
            None
        };
        cx.notify();
    }

    /// When the document changes: a jump that waited for the scan runs when the scan is done (in
    /// every pane of the document that asked for one), and is dropped when the scan failed.
    pub(super) fn run_pending_scan_jump(&mut self, cx: &mut Context<Self>) {
        if self.pending_scan_jump.is_none() || self.document.read(cx).is_scanning() {
            return;
        }
        let complete = !self.document.read(cx).is_partial();
        // Only a jump that waited for the scan that ended: the document may have been replaced.
        if let Some((id, reg, args, hold)) = self.pending_scan_jump.take()
            && complete
            && self.document.read(cx).scan_finished(id)
        {
            self.replaying_scan_jump = true;
            self.run_registration(reg, &args, cx);
            self.replaying_scan_jump = false;
            // The macro goes on after the jump.
            drop(hold);
        }
    }

    /// While the scan runs, a jump to the end of the file or to a line it has not reached waits
    /// for the scan (ADR-0020, chapter 2: 「走査が終わるまで待つ」). Returns whether `reg` was
    /// held back.
    pub(crate) fn defer_for_scan(
        &mut self,
        reg: &'static Registration,
        args: &[Arg],
        cx: &mut Context<Self>,
    ) -> bool {
        let shared = self.document.read(cx);
        if !shared.is_scanning() {
            return false;
        }
        let beyond = match reg.sakura {
            "F_GOFILEEND" | "F_GOFILEEND_SEL" | "F_GOFILEEND_BOX" => true,
            "F_JUMP" => {
                let number = |arg: Option<&Arg>| match arg {
                    Some(Arg::Int(n)) => Some(*n),
                    Some(Arg::Text(text)) => text.trim().parse::<i64>().ok(),
                    _ => None,
                };
                let option = args.get(1).map_or(Some(1), |a| number(Some(a)));
                // Only the plain count of lines (option 1) is the number of a scanned line; the
                // others count rows or the valid lines of PL/SQL, which no prefix can place.
                // The last line that is scanned may be cut short: it counts as not reached.
                option != Some(1)
                    || number(args.first()).is_some_and(|n| {
                        usize::try_from(n).is_ok_and(|n| n >= shared.doc().line_count())
                    })
            }
            _ => false,
        };
        if !beyond {
            return false;
        }
        let Some(id) = shared.scan_id() else {
            return false;
        };
        // Recorded now, in its place among the commands; the run that follows does not record.
        self.record_call(reg.sakura, args);
        // A macro that asked waits for the jump (`macros/wait.rs`): its next step needs the text
        // that the jump moves to, and the edits are refused until the scan is done.
        let hold = crate::macros::wait::track(cx);
        self.pending_scan_jump = Some((id, reg, args.to_vec(), hold));
        self.notice = Some("改行位置を調べ終えてから移動します".to_owned());
        cx.notify();
        true
    }

    /// After a command: when it was refused because the document shows only a start of its file,
    /// the status bar says why (an edit beeps and does nothing otherwise).
    pub(super) fn say_why_refused(&mut self, cx: &App) {
        if std::mem::take(&mut self.refused_edit)
            && let Some(why) = self.document.read(cx).partial_reason("編集")
        {
            self.notice = Some(why);
        }
    }

    /// Holds back the restore of the caret (a reload) and of the history (`CMruListener`) while
    /// the document shows only the scanned start of its file: they would be cut to it, and the
    /// history would be written with the cut values. Returns whether they were held back; if not,
    /// the caller restores now.
    pub(crate) fn defer_restore(
        &mut self,
        caret: Option<Pos>,
        history: Option<(FileEntry, HistoryOptions)>,
        cx: &mut Context<Self>,
    ) -> bool {
        let shared = self.document.read(cx);
        let Some(id) = shared.scan_id().filter(|_| shared.is_scanning()) else {
            return false;
        };
        let held = self.pending_scan_restore.take().filter(|r| r.id == id);
        self.pending_scan_restore = Some(ScanRestore {
            id,
            at_load: self.cursor.caret(),
            caret: caret.or(held.as_ref().and_then(|r| r.caret)),
            history: history.or(held.and_then(|r| r.history)),
        });
        true
    }

    /// When the document changes: restores what waited for the scan, once it is done (and drops
    /// it when the scan failed). A caret that the user moved meanwhile stays where it is.
    pub(super) fn apply_scan_restore(&mut self, cx: &mut Context<Self>) {
        if self.pending_scan_restore.is_none() || self.document.read(cx).is_scanning() {
            return;
        }
        // A scan that failed leaves it where it is: the history keeps what it had.
        let finished = self
            .pending_scan_restore
            .as_ref()
            .is_some_and(|r| self.document.read(cx).scan_finished(r.id));
        if !finished {
            return;
        }
        let Some(restore) = self.pending_scan_restore.take() else {
            return;
        };
        let kept_place = self.cursor.caret() == restore.at_load;
        if kept_place
            && let Some(pos) = restore.caret
            && let Ok(pos) = self.document.read(cx).doc().clamp_pos(pos)
        {
            self.go_to(pos, cx);
        }
        if let Some((entry, mut options)) = restore.history {
            options.restore_cur_position &= kept_place;
            self.restore_from_history(&entry, options, cx);
        }
        self.record_history(cx);
    }
}
