//! Searching a document: next, previous, all, with cancellation and progress.
//!
//! The behavior follows `CViewCommander::Command_SEARCH_NEXT` and `Command_SEARCH_PREV`
//! (`sakura_core/cmd/CViewCommander_Search.cpp`) and `CSearchAgent::SearchWord`
//! (`sakura_core/agent/CSearchAgent.cpp`).

use std::collections::VecDeque;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;

use crate::buffer::{BufferError, Document, Pos};

use super::matcher::{Match, Matcher, MatcherError, ceil_boundary};

/// Why a search stopped without an answer.
#[derive(Debug)]
pub enum SearchError {
    /// The search was cancelled through its [`CancelToken`].
    Cancelled,
    /// The document could not be read.
    Buffer(BufferError),
    /// The matcher gave up.
    Matcher(MatcherError),
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("the search was cancelled"),
            Self::Buffer(e) => e.fmt(f),
            Self::Matcher(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for SearchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Cancelled => None,
            Self::Buffer(e) => Some(e),
            Self::Matcher(e) => Some(e),
        }
    }
}

impl From<BufferError> for SearchError {
    fn from(e: BufferError) -> Self {
        Self::Buffer(e)
    }
}

impl From<MatcherError> for SearchError {
    fn from(e: MatcherError) -> Self {
        Self::Matcher(e)
    }
}

/// A flag that stops a running search. Clones share the flag; the search checks it between lines.
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// How far a search has come, in lines of the document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub lines_done: usize,
    pub lines_total: usize,
}

/// The lines between two progress reports, unless set otherwise.
const DEFAULT_PROGRESS_INTERVAL: usize = 10_000;

/// Cancellation and progress of one search. `Control::default()` has neither.
pub struct Control<'a> {
    cancel: Option<CancelToken>,
    on_progress: Option<Box<dyn FnMut(Progress) + Send + 'a>>,
    interval: usize,
    done: usize,
    total: usize,
}

impl Default for Control<'_> {
    fn default() -> Self {
        Self {
            cancel: None,
            on_progress: None,
            interval: DEFAULT_PROGRESS_INTERVAL,
            done: 0,
            total: 0,
        }
    }
}

impl fmt::Debug for Control<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Control")
            .field("cancel", &self.cancel)
            .field("done", &self.done)
            .field("total", &self.total)
            .finish_non_exhaustive()
    }
}

impl<'a> Control<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stops the search with [`SearchError::Cancelled`] when `token` is cancelled.
    pub fn with_cancel(mut self, token: CancelToken) -> Self {
        self.cancel = Some(token);
        self
    }

    /// Calls `f` every `interval` lines (10,000 unless changed) and when the search ends.
    pub fn with_progress(mut self, f: impl FnMut(Progress) + Send + 'a) -> Self {
        self.on_progress = Some(Box::new(f));
        self
    }

    /// Changes the number of lines between two progress reports (at least 1).
    pub fn with_interval(mut self, lines: usize) -> Self {
        self.interval = lines.max(1);
        self
    }

    fn begin(&mut self, total: usize) {
        self.done = 0;
        self.total = total;
    }

    /// Called before each line is read.
    fn step(&mut self) -> Result<(), SearchError> {
        if self.cancel.as_ref().is_some_and(CancelToken::is_cancelled) {
            return Err(SearchError::Cancelled);
        }
        self.done += 1;
        if self.done.is_multiple_of(self.interval) {
            self.report();
        }
        Ok(())
    }

    fn report(&mut self) {
        let progress = Progress {
            lines_done: self.done,
            lines_total: self.total,
        };
        if let Some(f) = self.on_progress.as_mut() {
            f(progress);
        }
    }
}

/// Which way to search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

/// A match in the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub start: Pos,
    pub end: Pos,
    /// Capture groups 1, 2, ...: byte ranges into the text of line `start.line`.
    pub captures: Vec<Option<Range<usize>>>,
    /// The search went past the end (or the start) of the document and began again, which
    /// Sakura reports with "▼先頭から再検索しました" / "▲末尾から再検索しました"
    /// (`STR_ERR_SRNEXT1` / `STR_ERR_SRPREV1` in `sakura_core/sakura_rc.rc`).
    pub wrapped: bool,
}

impl Found {
    fn new(line: usize, m: Match, wrapped: bool) -> Self {
        Self {
            start: Pos::new(line, m.start),
            end: Pos::new(line, m.end),
            captures: m.captures,
            wrapped,
        }
    }

    pub fn range(&self) -> Range<Pos> {
        self.start..self.end
    }
}

/// Finds the match after (or before) `from`.
///
/// * Forward: the first match that begins at or after `from`. A line past the end of the
///   document has no match.
/// * Backward: the last match that begins before `from`; matches in earlier lines may begin
///   anywhere. A position past the end means the end of the document.
/// * `wrap` is Sakura's option 先頭（末尾）から再検索 (`m_bSearchAll`, `CommonSetting.h`). When
///   nothing is found, the search goes on from the start (end) of the document, up to the line
///   it began in, and `Found::wrapped` tells so. Without it the search stops at the end (start).
///   Sakura turns wrapping off while it replaces all (`bReplaceAll`); see
///   [`super::replace_all`].
///
/// The matches are the same as Sakura's, except that lines are searched without their line
/// ending (Sakura matches over the line ending too, which a pattern typed in the dialog cannot
/// contain), and that a backward whole-word search from the end of the document also looks at the
/// last line (Sakura skips it).
pub fn find(
    doc: &Document,
    matcher: &dyn Matcher,
    from: Pos,
    direction: Direction,
    wrap: bool,
    ctl: &mut Control<'_>,
) -> Result<Option<Found>, SearchError> {
    let count = doc.line_count();
    if count == 0 {
        return Ok(None);
    }
    let result = match direction {
        Direction::Forward => {
            let first = from.line.min(count);
            let total = (count - first) + if wrap { (from.line + 1).min(count) } else { 0 };
            ctl.begin(total);
            match forward(doc, matcher, from.line, from.col, count, false, ctl)? {
                Some(found) => Some(found),
                None if wrap => forward(doc, matcher, 0, 0, (from.line + 1).min(count), true, ctl)?,
                None => None,
            }
        }
        Direction::Backward => {
            let (line, limit) = if from.line >= count {
                (count - 1, None)
            } else {
                (from.line, Some(from.col))
            };
            let total = line + 1 + if wrap { count - line } else { 0 };
            ctl.begin(total);
            match backward(doc, matcher, line, limit, 0, false, ctl)? {
                Some(found) => Some(found),
                None if wrap => backward(doc, matcher, count - 1, None, line, true, ctl)?,
                None => None,
            }
        }
    };
    ctl.report();
    Ok(result)
}

/// The first match in lines `line..end_line`, beginning at `col` in the first of them.
fn forward(
    doc: &Document,
    matcher: &dyn Matcher,
    line: usize,
    col: usize,
    end_line: usize,
    wrapped: bool,
    ctl: &mut Control<'_>,
) -> Result<Option<Found>, SearchError> {
    for i in line..end_line {
        ctl.step()?;
        let Some(text) = doc.line(i)?.map(|l| l.text) else {
            break;
        };
        let start = if i == line { col } else { 0 };
        let Some(start) = ceil_boundary(&text, start) else {
            continue;
        };
        if let Some(m) = matcher.find_at(&text, start)? {
            return Ok(Some(Found::new(i, m, wrapped)));
        }
    }
    Ok(None)
}

/// The last match before `limit` in line `line` (any, for `None`), else the last match of each
/// line above it down to `lowest`.
fn backward(
    doc: &Document,
    matcher: &dyn Matcher,
    line: usize,
    limit: Option<usize>,
    lowest: usize,
    wrapped: bool,
    ctl: &mut Control<'_>,
) -> Result<Option<Found>, SearchError> {
    for i in (lowest..=line).rev() {
        ctl.step()?;
        let Some(text) = doc.line(i)?.map(|l| l.text) else {
            continue;
        };
        // A limit inside a char counts the same as the end of that char, and one past the end
        // of the line is no limit, because every match begins before it.
        let limit = if i == line {
            limit.and_then(|col| ceil_boundary(&text, col))
        } else {
            None
        };
        if let Some(m) = matcher.find_last_before(&text, limit)? {
            return Ok(Some(Found::new(i, m, wrapped)));
        }
    }
    Ok(None)
}

/// Where "find next" starts when `current` is the match that is selected now: its end, or one
/// char later if it is empty (Sakura: `b0Match` in `Command_SEARCH_NEXT`, the guard against
/// finding the same empty match forever). "Find previous" starts at `current.start`.
pub fn start_after(doc: &Document, current: &Range<Pos>) -> Result<Pos, BufferError> {
    let end = doc.clamp_pos(current.end)?;
    if current.start != current.end {
        return Ok(end);
    }
    let Some(line) = doc.line(end.line)? else {
        return Ok(end);
    };
    Ok(match line.text[end.col..].chars().next() {
        Some(c) => Pos::new(end.line, end.col + c.len_utf8()),
        None => Pos::new(end.line + 1, 0),
    })
}

/// All matches, one after the other without overlap, as an iterator. Made by [`find_all`].
///
/// Every line is read once. When an item is an error (cancelled, unreadable), the iterator ends
/// after it.
pub struct FindAll<'a> {
    doc: &'a Document,
    matcher: &'a dyn Matcher,
    ctl: Control<'a>,
    line: usize,
    end_line: usize,
    first_line: usize,
    first_col: usize,
    /// Matches on this line must end at or before this column.
    limit: Option<(usize, usize)>,
    pending: VecDeque<Found>,
    done: bool,
}

/// Finds every match in `range` (the whole document for `None`), as an iterator.
///
/// A match must lie inside the range. Sakura's replace-in-selection stops at the first match
/// that ends after the selection (`Command_REPLACE_ALL`), and a range that ends at column 0 does
/// not take matches on that last line; this does the same. Sakura does not check the start, so
/// a whole-word match that begins before the selection could be replaced; this leaves it out.
pub fn find_all<'a>(
    doc: &'a Document,
    matcher: &'a dyn Matcher,
    range: Option<Range<Pos>>,
    mut ctl: Control<'a>,
) -> FindAll<'a> {
    let count = doc.line_count();
    let (start, end_line, limit) = match range {
        None => (Pos::new(0, 0), count, None),
        Some(r) if r.end.col == 0 => (r.start, r.end.line.min(count), None),
        Some(r) => (
            r.start,
            (r.end.line + 1).min(count),
            Some((r.end.line, r.end.col)),
        ),
    };
    ctl.begin(end_line.saturating_sub(start.line));
    FindAll {
        doc,
        matcher,
        ctl,
        line: start.line,
        end_line,
        first_line: start.line,
        first_col: start.col,
        limit,
        pending: VecDeque::new(),
        done: false,
    }
}

impl FindAll<'_> {
    /// Reads the next line and queues its matches.
    fn load_line(&mut self) -> Result<(), SearchError> {
        self.ctl.step()?;
        let i = self.line;
        self.line += 1;
        let Some(text) = self.doc.line(i)?.map(|l| l.text) else {
            self.line = self.end_line;
            return Ok(());
        };
        let start = if i == self.first_line {
            self.first_col
        } else {
            0
        };
        let Some(start) = ceil_boundary(&text, start) else {
            return Ok(());
        };
        for m in self.matcher.find_all_from(&text, start)? {
            // A whole-word match can begin before `start`, when `start` is inside a word;
            // that text is outside the range.
            if i == self.first_line && m.start < start {
                continue;
            }
            if let Some((line, col)) = self.limit
                && i == line
                && m.end > col
            {
                self.line = self.end_line;
                break;
            }
            self.pending.push_back(Found::new(i, m, false));
        }
        Ok(())
    }
}

impl Iterator for FindAll<'_> {
    type Item = Result<Found, SearchError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(found) = self.pending.pop_front() {
                return Some(Ok(found));
            }
            if self.done {
                return None;
            }
            if self.line >= self.end_line {
                self.done = true;
                self.ctl.report();
                return None;
            }
            if let Err(e) = self.load_line() {
                self.done = true;
                return Some(Err(e));
            }
        }
    }
}

/// A search running on its own thread. Give it a clone of the [`Document`] (cloning is cheap and
/// the thread does not see later edits), watch [`progress`](Self::progress), and
/// [`cancel`](Self::cancel) it when the user types again.
#[derive(Debug)]
pub struct SearchJob<T> {
    cancel: CancelToken,
    progress: Arc<(AtomicUsize, AtomicUsize)>,
    handle: JoinHandle<Result<T, SearchError>>,
}

impl<T: Send + 'static> SearchJob<T> {
    /// Runs `work` on a new thread. `work` should pass the [`Control`] it gets on to the search
    /// functions, for example `SearchJob::spawn(move |ctl| find(&doc, &*matcher, from, dir, wrap, ctl))`.
    pub fn spawn(
        work: impl FnOnce(&mut Control<'static>) -> Result<T, SearchError> + Send + 'static,
    ) -> Self {
        let cancel = CancelToken::new();
        let progress = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
        let shared = Arc::clone(&progress);
        let token = cancel.clone();
        let handle = std::thread::spawn(move || {
            let mut ctl = Control::new()
                .with_cancel(token)
                .with_progress(move |p: Progress| {
                    shared.0.store(p.lines_done, Ordering::Relaxed);
                    shared.1.store(p.lines_total, Ordering::Relaxed);
                });
            work(&mut ctl)
        });
        Self {
            cancel,
            progress,
            handle,
        }
    }
}

impl<T> SearchJob<T> {
    /// Asks the search to stop. [`join`](Self::join) then gives [`SearchError::Cancelled`],
    /// unless the search had already finished.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// The last progress the search reported.
    pub fn progress(&self) -> Progress {
        Progress {
            lines_done: self.progress.0.load(Ordering::Relaxed),
            lines_total: self.progress.1.load(Ordering::Relaxed),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    /// Waits for the result. A panic in the search is passed on to the caller.
    pub fn join(self) -> Result<T, SearchError> {
        match self.handle.join() {
            Ok(result) => result,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }
}
