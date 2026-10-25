//! What the view does while the file behind its document is being scanned (ADR-0020, chapter 2):
//! it follows the scan, and holds back the jumps to places the scan has not reached.

use std::time::Duration;

use gpui::{App, Context};
use matsu_commands::command::{Arg, Registration};

use super::EditorView;
use crate::document::ScanTick;

/// How often the task of the view looks at the scan.
const POLL: Duration = Duration::from_millis(100);

impl EditorView {
    /// Starts the task that follows the scan of the document's file, if it has one: it shows the
    /// scanned part as it grows and the complete document at the end. The task ends when the
    /// document is replaced or the window is gone.
    pub(crate) fn watch_scan(&mut self, cx: &mut Context<Self>) {
        let document = self.document.clone();
        let Some(id) = document.read(cx).scan_id() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(POLL).await;
                let tick = document.update(cx, |shared, cx| shared.scan_tick(id, cx));
                match tick {
                    ScanTick::Running => {
                        // the percentage of the status bar
                        if this.update(cx, |_, cx| cx.notify()).is_err() {
                            break;
                        }
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

    /// The scan is done: the document can be edited now. A jump that waited for it runs.
    fn scan_ended(&mut self, tick: ScanTick, cx: &mut Context<Self>) {
        let waiting = self.pending_scan_jump.take();
        if tick == ScanTick::Finished {
            let lines = self.document.read(cx).doc().line_count();
            self.notice = Some(format!("改行位置を調べ終えました ({lines} 行)"));
            cx.notify();
            if let Some((reg, args)) = waiting {
                self.run_registration(reg, &args, cx);
            }
        } else {
            self.notice = None;
            cx.notify();
        }
    }

    /// While the scan runs, a jump to the end of the file or to a line it has not reached waits
    /// for the scan (ADR-0020, chapter 2: 「走査が終わるまで待つ」). Returns whether `reg` was
    /// held back.
    pub(super) fn defer_for_scan(
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
                let line = match args.first() {
                    Some(Arg::Int(n)) => Some(*n),
                    Some(Arg::Text(text)) => text.trim().parse::<i64>().ok(),
                    _ => None,
                };
                // The last line that is scanned may be cut short: it counts as not reached.
                line.is_some_and(|n| {
                    usize::try_from(n).is_ok_and(|n| n >= shared.doc().line_count())
                })
            }
            _ => false,
        };
        if !beyond {
            return false;
        }
        self.pending_scan_jump = Some((reg, args.to_vec()));
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
}
