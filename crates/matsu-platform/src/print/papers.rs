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
    /// Shared, so that the panel's view and this cache hold one layout (it is only read).
    Ready(Arc<Layout>),
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
            this.keep(layout.paper(), State::Ready(Arc::new(layout)));
        }
        this
    }

    /// The layout for `paper`: the one kept (shared, not copied), the one a worker is making (it
    /// waits for it), or a new one made here, which is kept.
    pub fn layout(&mut self, paper: Paper) -> Result<Arc<Layout>, PrintError> {
        let Some(index) = self.entries.iter().position(|e| e.paper == paper) else {
            let layout = Arc::new(self.job.layout(paper)?);
            self.keep(paper, State::Ready(layout.clone()));
            return Ok(layout);
        };
        if let State::Ready(layout) = &self.entries[index].state {
            return Ok(layout.clone());
        }
        // A worker is making it: wait for it, and keep what it made.
        let State::Running { worker, .. } = self.entries.remove(index).state else {
            unreachable!("the entry was not ready");
        };
        let layout = match worker.join() {
            Ok(layout) => layout,
            // The worker panicked: make it here, where the caller sees the panic.
            Err(_) => self.job.layout(paper),
        }
        .map(Arc::new);
        if let Ok(layout) = &layout {
            self.keep(paper, State::Ready(layout.clone()));
        }
        layout
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
        if let Some(index) = self.entries.iter().position(|e| e.paper == paper) {
            cancel(&self.entries.remove(index).state);
        }
        self.entries.push(Entry { paper, state });
        if self.entries.len() > KEPT {
            // The oldest goes; a worker still making it must not go on reading the document
            // with nobody to take its answer.
            cancel(&self.entries.remove(0).state);
        }
    }
}

/// Tells a worker that is still making the layout of `state` to stop (at its next look; nothing
/// waits for it).
fn cancel(state: &State) {
    if let State::Running { control, .. } = state {
        control.cancel();
    }
}

impl Drop for PaperLayouts {
    fn drop(&mut self) {
        for entry in &self.entries {
            cancel(&entry.state);
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
    fn a_kept_layout_is_shared_not_copied() {
        let (job, _) = job(100);
        let mut layouts = PaperLayouts::new(job, None);
        let first = layouts.layout(Paper::A4).unwrap();
        let second = layouts.layout(Paper::A4).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    /// A worker that runs until it is cancelled, as an entry of the cache.
    fn running_entry(paper: Paper) -> (Entry, Arc<LayoutControl>) {
        let control = Arc::new(LayoutControl::default());
        let seen = control.clone();
        let worker = std::thread::spawn(move || {
            // Bounded, so that a failure of the test cannot hang it.
            for _ in 0..1000 {
                if seen.is_cancelled() {
                    return Err(PrintError::Cancelled);
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(PrintError::NoPrintableArea)
        });
        let entry = Entry {
            paper,
            state: State::Running {
                control: control.clone(),
                worker,
            },
        };
        (entry, control)
    }

    #[test]
    fn a_worker_that_is_dropped_from_the_cache_is_cancelled() {
        let (job, _) = job(10);
        // evicted as the oldest
        let mut layouts = PaperLayouts::new(job.clone(), None);
        let (entry, control) = running_entry(Paper {
            width: 1,
            height: 1,
        });
        layouts.entries.push(entry);
        for width in 0..KEPT as i32 {
            layouts
                .layout(Paper {
                    width: 2100 + width,
                    height: 2970,
                })
                .unwrap();
        }
        assert!(control.is_cancelled());
        // replaced by a layout for the same paper
        let mut layouts = PaperLayouts::new(job.clone(), None);
        let (entry, control) = running_entry(Paper::A4);
        layouts.entries.push(entry);
        layouts.keep(
            Paper::A4,
            State::Ready(Arc::new(job.layout(Paper::A4).unwrap())),
        );
        assert!(control.is_cancelled());
        // the cache dropped
        let mut layouts = PaperLayouts::new(job, None);
        let (entry, control) = running_entry(Paper::A4);
        layouts.entries.push(entry);
        drop(layouts);
        assert!(control.is_cancelled());
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
