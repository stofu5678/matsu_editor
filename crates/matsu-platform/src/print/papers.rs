//! The layouts of a job for the papers a print panel asks about, kept so that the panel does not
//! wait for a layout every time it asks.
//!
//! AppKit's print panel asks the print view for its pages (`knowsPageRange:`) on the main thread
//! whenever the paper, the direction or the preview changes, and the layout of a big document
//! takes about a second (`timing_of_the_layout_of_a_100_mb_text` in `matsu-print`). Sakura's
//! preview lays the document out for the paper in `CPrintPreview::OnChangePrintSetting` once per
//! change of the setting; this keeps what was laid out for a paper, and
//!
//! * starts with the layout the preview already made for the paper of the setting, which is the
//!   paper the panel opens with in the usual case, so that the panel opens without a layout;
//! * lays the document out ahead on a worker thread for the paper the user is most likely to ask
//!   for next ([`PaperLayouts::lay_out_ahead`]: the same paper turned), so that turning it does
//!   not wait either.
//!
//! No OS is needed, so it is tested on every one; `mac` uses it.

use std::sync::Arc;
use std::thread::JoinHandle;

use matsu_print::{Layout, LayoutControl, Paper, PrintError};

use super::PrintJob;

/// A document of more lines than this is laid out ahead on a worker. Below it the layout takes
/// under 30 ms (the same bound as the preview's `BACKGROUND_LINES`), which nobody sees.
pub const AHEAD_LINES: usize = 20_000;

/// How many papers are kept. The panel offers a handful; the oldest is dropped.
const KEPT: usize = 4;

enum State {
    Ready(Box<Layout>),
    /// A worker lays the document out for the paper.
    Running {
        control: Arc<LayoutControl>,
        worker: JoinHandle<Result<Layout, PrintError>>,
    },
}

struct Entry {
    paper: Paper,
    state: State,
}

/// The layouts of one job, by paper. The owner is the print view; dropping it stops the workers.
pub struct PaperLayouts {
    job: PrintJob,
    /// The oldest first.
    entries: Vec<Entry>,
}

impl PaperLayouts {
    /// `known` is a layout of the job that was made already (the preview's), if there is one.
    pub fn new(job: PrintJob, known: Option<Layout>) -> Self {
        let mut this = Self {
            job,
            entries: Vec::new(),
        };
        if let Some(layout) = known {
            this.keep(layout.paper(), State::Ready(Box::new(layout)));
        }
        this
    }

    /// The layout for `paper`: the one kept, the one a worker is making (it waits for it), or a
    /// new one made here, which is kept.
    pub fn layout(&mut self, paper: Paper) -> Result<Layout, PrintError> {
        if let Some(index) = self.entries.iter().position(|e| e.paper == paper) {
            let entry = self.entries.remove(index);
            let layout = match entry.state {
                State::Ready(layout) => Ok(*layout),
                State::Running { worker, .. } => match worker.join() {
                    Ok(layout) => layout,
                    // The worker panicked: make it here, where the caller sees the panic.
                    Err(_) => self.job.layout(paper),
                },
            };
            if let Ok(layout) = &layout {
                // As the newest, so that the paper in use is the last to be dropped.
                self.keep(paper, State::Ready(Box::new(layout.clone())));
            }
            return layout;
        }
        let layout = self.job.layout(paper)?;
        self.keep(paper, State::Ready(Box::new(layout.clone())));
        Ok(layout)
    }

    /// Starts laying the document out for `paper` on a worker thread, when the document is big
    /// enough to matter ([`AHEAD_LINES`]) and nothing is kept or running for the paper.
    pub fn lay_out_ahead(&mut self, paper: Paper) {
        if self.job.source.line_count() <= AHEAD_LINES || self.has(paper) {
            return;
        }
        let control = Arc::new(LayoutControl::default());
        let job = self.job.clone();
        let worker_control = control.clone();
        let spawned = std::thread::Builder::new()
            .name("matsu-print-layout".into())
            .spawn(move || job.layout_controlled(paper, &worker_control));
        // No thread: the layout is made when the panel asks.
        if let Ok(worker) = spawned {
            self.keep(paper, State::Running { control, worker });
        }
    }

    /// Whether a layout for `paper` is kept or being made.
    pub fn has(&self, paper: Paper) -> bool {
        self.entries.iter().any(|e| e.paper == paper)
    }

    fn keep(&mut self, paper: Paper, state: State) {
        self.entries.retain(|e| e.paper != paper);
        self.entries.push(Entry { paper, state });
        if self.entries.len() > KEPT {
            self.entries.remove(0);
        }
    }
}

impl Drop for PaperLayouts {
    fn drop(&mut self) {
        // The workers stop at their next look; nothing waits for them.
        for entry in &self.entries {
            if let State::Running { control, .. } = &entry.state {
                control.cancel();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use matsu_print::{HeaderContext, LineSource, TypeOptions};

    use super::*;
    use crate::print::default_setting;

    /// Lines that count how many times they are read.
    struct Counting {
        lines: usize,
        reads: Arc<AtomicUsize>,
    }

    impl LineSource for Counting {
        fn line_count(&self) -> usize {
            self.lines
        }

        fn read_line(&self, _index: usize, buf: &mut String) -> bool {
            self.reads.fetch_add(1, Ordering::Relaxed);
            buf.clear();
            buf.push_str("text of a line");
            true
        }
    }

    fn job(lines: usize) -> (PrintJob, Arc<AtomicUsize>) {
        let reads = Arc::new(AtomicUsize::new(0));
        let job = PrintJob {
            source: Arc::new(Counting {
                lines,
                reads: reads.clone(),
            }),
            setting: default_setting(),
            options: TypeOptions::default(),
            header: HeaderContext::default(),
            title: "t".into(),
            styler: None,
        };
        (job, reads)
    }

    #[test]
    fn a_layout_that_is_known_is_not_made_again() {
        let (job, reads) = job(100);
        let known = job.layout(Paper::A4).unwrap();
        let pages = known.page_count();
        let before = reads.load(Ordering::Relaxed);
        let mut layouts = PaperLayouts::new(job, Some(known));
        assert_eq!(layouts.layout(Paper::A4).unwrap().page_count(), pages);
        assert_eq!(reads.load(Ordering::Relaxed), before);
    }

    #[test]
    fn a_paper_is_laid_out_once_however_often_the_panel_asks() {
        let (job, reads) = job(100);
        let mut layouts = PaperLayouts::new(job, None);
        let first = layouts.layout(Paper::A4).unwrap();
        let after_first = reads.load(Ordering::Relaxed);
        assert!(after_first > 0);
        for _ in 0..3 {
            assert_eq!(layouts.layout(Paper::A4).unwrap().paper(), first.paper());
        }
        assert_eq!(reads.load(Ordering::Relaxed), after_first);
    }

    #[test]
    fn another_paper_has_its_own_layout() {
        let (job, _) = job(500);
        let mut layouts = PaperLayouts::new(job, None);
        let portrait = layouts.layout(Paper::A4).unwrap();
        let landscape = layouts.layout(Paper::A4.turned(matsu_print::Orientation::Landscape));
        let landscape = landscape.unwrap();
        assert_eq!(landscape.paper().width, portrait.paper().height);
        // a landscape page holds fewer rows, so there are more pages
        assert!(landscape.page_count() > portrait.page_count());
        // both are kept
        assert!(layouts.has(Paper::A4));
        assert!(layouts.has(landscape.paper()));
    }

    #[test]
    fn a_small_document_is_not_laid_out_ahead() {
        let (job, reads) = job(AHEAD_LINES);
        let mut layouts = PaperLayouts::new(job, None);
        layouts.lay_out_ahead(Paper::A4);
        assert!(!layouts.has(Paper::A4));
        assert_eq!(reads.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn a_big_document_is_laid_out_ahead_and_the_panel_gets_that_layout() {
        let (job, reads) = job(AHEAD_LINES + 1);
        let direct = job.layout(Paper::A4).unwrap();
        let mut layouts = PaperLayouts::new(job, None);
        layouts.lay_out_ahead(Paper::A4);
        assert!(layouts.has(Paper::A4));
        // asking for it waits for the worker and takes its layout
        let got = layouts.layout(Paper::A4).unwrap();
        assert_eq!(got.page_count(), direct.page_count());
        // once it is there, it is not made again, and not started again
        let after = reads.load(Ordering::Relaxed);
        layouts.lay_out_ahead(Paper::A4);
        assert_eq!(layouts.layout(Paper::A4).unwrap().paper(), got.paper());
        assert_eq!(reads.load(Ordering::Relaxed), after);
    }

    #[test]
    fn only_the_newest_papers_are_kept() {
        let (job, _) = job(10);
        let mut layouts = PaperLayouts::new(job, None);
        for width in 0..KEPT as i32 + 2 {
            let paper = Paper {
                width: 2100 + width * 10,
                height: 2970,
            };
            layouts.layout(paper).unwrap();
        }
        assert_eq!(layouts.entries.len(), KEPT);
        assert!(!layouts.has(Paper {
            width: 2100,
            height: 2970
        }));
    }

    #[test]
    fn a_paper_with_no_room_for_text_is_an_error_and_not_kept() {
        let (job, _) = job(10);
        let mut layouts = PaperLayouts::new(job, None);
        let tiny = Paper {
            width: 10,
            height: 10,
        };
        assert!(layouts.layout(tiny).is_err());
        assert!(!layouts.has(tiny));
    }
}
