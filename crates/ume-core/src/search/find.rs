//! Searching a document: next, previous, all, with cancellation and progress.
//!
//! The behavior follows `CViewCommander::Command_SEARCH_NEXT` and `Command_SEARCH_PREV`
//! (`sakura_core/cmd/CViewCommander_Search.cpp`) and `CSearchAgent::SearchWord`
//! (`sakura_core/agent/CSearchAgent.cpp`).
//!
//! A matcher that [sees the line ending](Matcher::sees_line_ending) (the regular expressions) gets
//! each line followed by its own line ending, as Sakura's regex search does, and a match may
//! cover the line break: see [`Found`].

use std::collections::VecDeque;
use std::fmt;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::JoinHandle;

use crate::buffer::{BufferError, Document, Line, Pos};

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

    pub(crate) fn begin(&mut self, total: usize) {
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

    /// Called when the search has got to line `done` (a search that goes by lines of its own).
    pub(crate) fn tick(&mut self, done: usize) -> Result<(), SearchError> {
        if self.cancel.as_ref().is_some_and(CancelToken::is_cancelled) {
            return Err(SearchError::Cancelled);
        }
        let before = self.done;
        self.done = done;
        if done / self.interval > before / self.interval {
            self.report();
        }
        Ok(())
    }

    pub(crate) fn report(&mut self) {
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

/// A line as a matcher sees it: the text, followed by the line ending if the matcher wants it.
#[derive(Clone, Debug)]
pub(crate) struct Subject {
    /// The text and, for a matcher that sees the line ending, the line ending.
    pub text: String,
    /// The length of the text without the line ending.
    pub body: usize,
    /// The line ending that follows the text in `text` ("" if the matcher does not see it or the
    /// line has none).
    pub eol: &'static str,
    /// Whether the matcher sees the line ending (a regular expression).
    sees_eol: bool,
}

impl Subject {
    pub(crate) fn new(line: Line, with_eol: bool) -> Self {
        let body = line.text.len();
        let eol = match line.eol {
            Some(eol) if with_eol => eol.as_str(),
            _ => "",
        };
        let mut text = line.text;
        text.push_str(eol);
        Self {
            text,
            body,
            eol,
            sees_eol: with_eol,
        }
    }

    /// Whether a search may start at byte `col`: not past the end of the text, and not at the end
    /// of the whole subject (`CBregexp::Match` gives no match for `nStart >= len`). For a line
    /// without a line ending, whose subject is the text alone, that rules out its end. For a
    /// matcher that does not see the line ending (plain text, which cannot match at the end
    /// anyway) this changes nothing.
    pub(crate) fn can_start_at(&self, col: usize) -> bool {
        col <= self.body && (!self.sees_eol || col < self.text.len())
    }
}

/// A match in the document.
///
/// For a matcher that sees the line ending, the match may cover it (`\r\n`, `\s`, `[^a]`). Then
/// `end` is the start of the next line, and `start` is at most the end of the text of the line:
/// a match that begins inside the line ending begins at its start (Sakura: "改行文字内でマッチ
/// した場合、改行文字の始めからマッチしたことにする", `CSearchAgent::SearchWord`). [`raw`](Self::raw)
/// is the match as the matcher reported it, in the line text followed by its line ending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub start: Pos,
    pub end: Pos,
    /// Capture groups 1, 2, ...: byte ranges into the text of line `start.line`, followed by its
    /// line ending for a matcher that sees it.
    pub captures: Vec<Option<Range<usize>>>,
    /// The search went past the end (or the start) of the document and began again, which
    /// Sakura reports with "▼先頭から再検索しました" / "▲末尾から再検索しました"
    /// (`STR_ERR_SRNEXT1` / `STR_ERR_SRPREV1` in `sakura_core/sakura_rc.rc`).
    pub wrapped: bool,
    /// The match as the matcher reported it: bytes of the line text followed by its line ending
    /// (the text alone for a matcher that does not see the line ending).
    pub raw: Range<usize>,
    /// The line ending that follows the text in the bytes `raw` and `captures` refer to; "" if
    /// the matcher does not see it.
    eol: &'static str,
    /// The length of the text of the line, without the line ending.
    body: usize,
}

impl Found {
    pub(crate) fn new(line: usize, m: Match, wrapped: bool, subject: &Subject) -> Self {
        Self::in_subject(line, m, subject.body, subject.eol, wrapped)
    }

    /// A match `m` that a matcher found in a subject: the text of line `line` (`body` bytes),
    /// followed by `eol`, its line ending, if the matcher sees it.
    pub fn in_subject(
        line: usize,
        m: Match,
        body: usize,
        eol: &'static str,
        wrapped: bool,
    ) -> Self {
        let end = if m.end > body {
            Pos::new(line + 1, 0)
        } else {
            Pos::new(line, m.end)
        };
        Self {
            start: Pos::new(line, m.start.min(body)),
            end,
            raw: m.start..m.end,
            captures: m.captures,
            wrapped,
            eol,
            body,
        }
    }

    pub fn range(&self) -> Range<Pos> {
        self.start..self.end
    }

    /// Whether the matcher found an empty match. (A match at the very end of a line with its
    /// line ending is empty, but `start..end` is not: it spans the line ending.)
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// The line text followed by its line ending, as the matcher saw it. `text` is the text of
    /// line `start.line`.
    pub fn subject(&self, text: &str) -> String {
        format!("{text}{}", self.eol)
    }

    /// The part of the line ending that comes before the match, when the match begins inside it.
    /// A replacement of `start..end` has to put it back.
    pub fn kept_before(&self) -> &str {
        let n = self.raw.start.saturating_sub(self.body).min(self.eol.len());
        &self.eol[..n]
    }

    /// The part of the line ending that comes after the match, when the match ends inside it
    /// (`\r` of a CR LF). A replacement of `start..end` has to put it back.
    pub fn kept_after(&self) -> &str {
        let from = self.raw.end.saturating_sub(self.body);
        if self.raw.end > self.body && from < self.eol.len() {
            &self.eol[from..]
        } else {
            ""
        }
    }
}

/// The first match in line `line` that a search starting at byte `col` finds, or `None`. A
/// search starts at most at the end of the text of the line (`nIdxPos <=
/// pDocLine->GetLengthWithoutEOL()` in `CSearchAgent::SearchWord`), and never at the end of
/// the whole subject: `CBregexp::Match` finds nothing when `nStart >= len` (「nStart>=lenのような
/// 場合に、マッチ扱いになり無限置換等の不具合になっていた」), so on a line without a line
/// ending the end of the text is no place to start from.
pub fn find_in_line(
    doc: &Document,
    matcher: &dyn Matcher,
    line: usize,
    col: usize,
    wrapped: bool,
) -> Result<Option<Found>, SearchError> {
    let Some(l) = doc.line(line)? else {
        return Ok(None);
    };
    Ok(match_line(matcher, line, l, col, wrapped)?)
}

fn match_line(
    matcher: &dyn Matcher,
    line: usize,
    l: Line,
    col: usize,
    wrapped: bool,
) -> Result<Option<Found>, MatcherError> {
    let subject = Subject::new(l, matcher.sees_line_ending());
    if !subject.can_start_at(col) {
        return Ok(None);
    }
    let Some(start) = ceil_boundary(&subject.text, col) else {
        return Ok(None);
    };
    // A search never finds a match that begins at the very end of the subject (the real Sakura
    // does not replace `$` at the end of a last line without a line ending, though the global
    // replacement of `x*` does reach the end).
    Ok(matcher
        .find_at(&subject.text, start)?
        .filter(|m| !(subject.sees_eol && m.start >= subject.text.len()))
        .map(|m| Found::new(line, m, wrapped, &subject)))
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
/// The matches are the same as Sakura's, except that a backward whole-word search from the end of
/// the document also looks at the last line (Sakura skips it). A regular expression is matched
/// against each line together with its line ending, as Sakura does (see [`Found`]).
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
        let Some(l) = doc.line(i)? else {
            break;
        };
        let start = if i == line { col } else { 0 };
        if let Some(found) = match_line(matcher, i, l, start, wrapped)? {
            return Ok(Some(found));
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
        let Some(l) = doc.line(i)? else {
            continue;
        };
        let subject = Subject::new(l, matcher.sees_line_ending());
        // A limit inside a char counts the same as the end of that char, and one past the end
        // of the text of the line is no limit, because every match begins before it. (A match
        // that begins inside the line ending is before no caret of the line; Sakura compares the
        // start of the match with the caret, `nHitPos >= nHitTo`.)
        let limit = if i == line {
            limit
                .filter(|col| *col <= subject.body)
                .and_then(|col| ceil_boundary(&subject.text, col))
        } else {
            None
        };
        if let Some(m) = matcher.find_last_before(&subject.text, subject.body, limit)? {
            return Ok(Some(Found::new(i, m, wrapped, &subject)));
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
    /// A match that ends on this line or a later one is outside the range: the range ends at the
    /// start of this line (column 0).
    stop_line: Option<usize>,
    pending: VecDeque<Found>,
    done: bool,
}

/// Finds every match in `range` (the whole document for `None`), as an iterator.
///
/// A match must lie inside the range. Sakura's replace-in-selection stops at the first match
/// that ends after the selection (`Command_REPLACE_ALL`), and a range that ends at column 0 does
/// not take matches on that last line, nor one that ends there (a match over the line ending of
/// the line above); this does the same. Sakura does not check the start, so
/// a whole-word match that begins before the selection could be replaced; this leaves it out.
pub fn find_all<'a>(
    doc: &'a Document,
    matcher: &'a dyn Matcher,
    range: Option<Range<Pos>>,
    mut ctl: Control<'a>,
) -> FindAll<'a> {
    let count = doc.line_count();
    let (start, end_line, limit, stop_line) = match range {
        None => (Pos::new(0, 0), count, None, None),
        Some(r) if r.end.col == 0 => (r.start, r.end.line.min(count), None, Some(r.end.line)),
        Some(r) => (
            r.start,
            (r.end.line + 1).min(count),
            Some((r.end.line, r.end.col)),
            None,
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
        stop_line,
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
        let Some(l) = self.doc.line(i)? else {
            self.line = self.end_line;
            return Ok(());
        };
        let subject = Subject::new(l, self.matcher.sees_line_ending());
        let start = if i == self.first_line {
            self.first_col
        } else {
            0
        };
        if !subject.can_start_at(start) {
            return Ok(());
        }
        let Some(start) = ceil_boundary(&subject.text, start) else {
            return Ok(());
        };
        for m in self
            .matcher
            .find_all_from(&subject.text, subject.body, start)?
        {
            // A whole-word match can begin before `start`, when `start` is inside a word;
            // that text is outside the range.
            if i == self.first_line && m.start < start {
                continue;
            }
            let found = Found::new(i, m, false, &subject);
            // A match that covers the line ending ends on the next line: Sakura stops at a
            // match that ends at the line where the selection ends at column 0
            // (`ptColLineP.x == 0` in `Command_REPLACE_ALL`).
            if self.stop_line.is_some_and(|l| found.end.line >= l) {
                self.line = self.end_line;
                break;
            }
            if let Some((line, col)) = self.limit
                && i == line
                && found.end > Pos::new(line, col)
            {
                self.line = self.end_line;
                break;
            }
            self.pending.push_back(found);
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
