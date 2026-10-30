//! The scan that makes a [`BlockIndex`]: step by step ([`BlockScanner`]), with a callback
//! ([`scan_blocks`]) or on its own thread ([`ScanJob`]).

use std::fmt;
use std::io;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;

use super::long::LongAnchor;
use super::{BlockIndex, DEFAULT_INDEX_BLOCK_SIZE, EolLens, walk_eols};
use crate::buffer::codec::Codec;
use crate::buffer::{BufferError, Document, Source};

/// How many blocks one read of the scan covers by default (4 MiB with the default block size).
const DEFAULT_WINDOW_BLOCKS: usize = 64;

/// The largest block the scan cuts. A line ending table keeps offsets in 32 bits.
const MAX_BLOCK_SIZE: usize = 1 << 30;

/// A line of this many bytes is a long line: the scan records the decoded length of every block
/// in it, so that a place in it can be found without decoding the line from its start
/// (ADR-0020, 9.10).
pub const DEFAULT_LONG_LINE: usize = 1024 * 1024;

/// How far past the edge of a block the scan looks: the longest character is 4 bytes, a UTF-16
/// high surrogate looks 2 bytes further and a UTF-32 CRLF is 8 bytes.
const LOOKAHEAD: usize = 16;

/// What the scan is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanOptions {
    /// Where blocks are cut, in bytes (at least 1; a block is a little longer when a character or
    /// a line ending crosses the edge). Small values make tests cross the edges with small files.
    pub block_size: usize,
    /// How many blocks one read covers. Progress and cancel are checked between reads.
    pub window_blocks: usize,
    /// Whether NEL, LS and PS end a line (`bEnableExtEol`; the same setting the document gets).
    pub ext_eol: bool,
    /// How long a [`ScanJob`] waits after each read. `None` goes as fast as it can. For tests of
    /// what is shown while a scan runs, and for a scan that must leave the disk to others.
    pub step_delay: Option<std::time::Duration>,
    /// A line that has this many bytes without a line ending is a long line (see
    /// [`DEFAULT_LONG_LINE`]). 0 means there are none.
    pub long_line: usize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            block_size: DEFAULT_INDEX_BLOCK_SIZE,
            window_blocks: DEFAULT_WINDOW_BLOCKS,
            ext_eol: false,
            step_delay: None,
            long_line: DEFAULT_LONG_LINE,
        }
    }
}

/// Why a scan did not finish.
#[derive(Debug)]
pub enum ScanError {
    /// The caller cancelled it.
    Cancelled,
    /// The encoding depends on earlier bytes (ISO-2022-JP, UTF-7): no block edge can be known from
    /// the middle. The caller reads the file whole instead (ADR-0020, chapter 3, plan A).
    Stateful,
    /// Reading the source failed. [`is_changed_outside`](crate::file::is_changed_outside) tells a
    /// file that was changed by another program from other errors.
    Io(io::Error),
    /// Building the document failed.
    Buffer(BufferError),
}

impl fmt::Display for ScanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => f.write_str("the scan was cancelled"),
            Self::Stateful => {
                f.write_str("the encoding keeps a state between characters and cannot be scanned")
            }
            Self::Io(e) => write!(f, "cannot read the file: {e}"),
            Self::Buffer(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for ScanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Buffer(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for ScanError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<std::collections::TryReserveError> for ScanError {
    fn from(e: std::collections::TryReserveError) -> Self {
        Self::Io(io::Error::new(io::ErrorKind::OutOfMemory, e))
    }
}

impl From<BufferError> for ScanError {
    fn from(e: BufferError) -> Self {
        Self::Buffer(e)
    }
}

/// How far a scan is, for a display such as 「改行位置を調べています 35%」.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanProgress {
    /// The bytes scanned from the start of the file.
    pub scanned: u64,
    /// The length of the file.
    pub total: u64,
    /// The line endings found so far (the line count of the scanned part is this, or one more).
    pub breaks: u64,
    pub state: ScanState,
}

impl ScanProgress {
    /// The number of line endings the whole file has, estimated from the share scanned (the
    /// exact count once the scan is done). The scanned part has the line endings found so far;
    /// the rest is taken to have as many per byte.
    pub fn estimated_breaks(&self) -> u64 {
        if self.scanned == 0 || self.scanned >= self.total {
            return self.breaks;
        }
        (u128::from(self.breaks) * u128::from(self.total) / u128::from(self.scanned)) as u64
    }

    /// The share scanned, 0 to 100 (100 for an empty file).
    pub fn percent(&self) -> u8 {
        if self.total == 0 {
            return 100;
        }
        (u128::from(self.scanned.min(self.total)) * 100 / u128::from(self.total)) as u8
    }
}

/// What a scan is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanState {
    Running,
    /// The whole file is scanned.
    Done,
    Cancelled,
    /// A read failed, or the file was changed outside.
    Failed,
}

impl ScanState {
    fn to_u8(self) -> u8 {
        self as u8
    }

    fn from_u8(n: u8) -> Self {
        match n {
            0 => Self::Running,
            1 => Self::Done,
            2 => Self::Cancelled,
            _ => Self::Failed,
        }
    }
}

/// The scan, one read at a time. [`scan_blocks`] and [`ScanJob`] drive it; use it directly to
/// control the pace.
pub struct BlockScanner {
    index: Arc<Mutex<BlockIndex>>,
    source: Arc<dyn Source>,
    codec: Arc<dyn Codec>,
    lens: EolLens,
    block_size: usize,
    window: u64,
    ext_eol: bool,
    long_line: u64,
    /// The bytes since the last line ending, at `pos`.
    since_break: u64,
    /// While a long line goes on: where it started, and how long the part before the next block
    /// is in the decoded text.
    long: Option<(u64, u64)>,
    /// The blocks that are in the index (the number of the next block).
    blocks: usize,
    pos: u64,
    len: u64,
    breaks: u64,
}

/// What a block holds, as far as breaks are concerned.
struct BlockBreaks {
    /// Where the block ends, in the bytes of the window.
    end: usize,
    /// The line endings that start in it.
    count: u64,
    /// The start of the first and the end of the last, in the bytes of the window.
    first_start: Option<usize>,
    last_end: Option<usize>,
}

impl BlockScanner {
    /// Prepares a scan of `source` read through `codec`. Nothing is read yet.
    ///
    /// # Errors
    ///
    /// [`ScanError::Stateful`] for an encoding that depends on earlier bytes.
    pub fn new(
        source: Arc<dyn Source>,
        codec: Arc<dyn Codec>,
        options: ScanOptions,
    ) -> Result<Self, ScanError> {
        if !codec.is_stateless() {
            return Err(ScanError::Stateful);
        }
        let block_size = options.block_size.clamp(1, MAX_BLOCK_SIZE);
        let len = source.len();
        let index = BlockIndex::new(
            source.clone(),
            codec.clone(),
            options.ext_eol,
            block_size,
            options.long_line,
        );
        Ok(Self {
            lens: index.lens,
            index: Arc::new(Mutex::new(index)),
            source,
            codec,
            block_size,
            window: (block_size as u64).saturating_mul(options.window_blocks.max(1) as u64),
            ext_eol: options.ext_eol,
            long_line: options.long_line as u64,
            since_break: 0,
            long: None,
            blocks: 0,
            pos: 0,
            len,
            breaks: 0,
        })
    }

    /// The index being made. A snapshot is `shared().lock().clone()`: it covers what is scanned
    /// so far.
    pub fn shared(&self) -> Arc<Mutex<BlockIndex>> {
        self.index.clone()
    }

    pub fn is_done(&self) -> bool {
        self.pos >= self.len
    }

    pub fn progress(&self) -> ScanProgress {
        ScanProgress {
            scanned: self.pos,
            total: self.len,
            breaks: self.breaks,
            state: if self.is_done() {
                ScanState::Done
            } else {
                ScanState::Running
            },
        }
    }

    /// Reads the next window of the file and adds its blocks to the index. Does nothing when the
    /// scan is done. On an error nothing of the window is added.
    pub fn step(&mut self) -> Result<(), ScanError> {
        if self.is_done() {
            return Ok(());
        }
        let at_eof;
        // (a handle of its own: `bytes` borrows it while the blocks are noted)
        let source = Arc::clone(&self.source);
        let bytes = {
            let want = self.window.saturating_add(LOOKAHEAD as u64);
            let end = self.pos.saturating_add(want).min(self.len);
            at_eof = end == self.len;
            let bytes = source.read(self.pos, end)?;
            if bytes.len() as u64 != end - self.pos {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
            }
            bytes
        };
        let target = self.block_size;
        // The lookahead is a multiple of 4 so that a cut stays aligned for UTF-16 and UTF-32.
        let reach = target.next_multiple_of(4) + LOOKAHEAD;
        let mut blocks: Vec<(u64, u64)> = Vec::new();
        let mut anchors: Vec<LongAnchor> = Vec::new();
        let mut bs = 0;
        while bs < bytes.len() {
            let remaining = bytes.len() - bs;
            if !at_eof && bs + target + LOOKAHEAD > bytes.len() {
                // The rest is read again with the next window, so that its edge is cut with the
                // bytes after it in view.
                break;
            }
            blocks.try_reserve(1)?;
            let found = if at_eof && remaining <= target {
                let mut found = BlockBreaks {
                    end: bytes.len(),
                    count: 0,
                    first_start: None,
                    last_end: None,
                };
                walk_eols(
                    &*self.codec,
                    &self.lens,
                    &bytes,
                    bs,
                    usize::MAX,
                    self.ext_eol,
                    |off, len, _| {
                        found.count += 1;
                        found.first_start.get_or_insert(off);
                        found.last_end = Some(off + len);
                    },
                );
                found
            } else {
                self.cut_block(&bytes, bs, reach)
            };
            self.note_long_line(&bytes, bs, &found, &blocks, &mut anchors)?;
            let block_len = (found.end - bs) as u64;
            self.since_break = match found.last_end {
                Some(last_end) => (found.end - last_end) as u64,
                None => self.since_break + block_len,
            };
            blocks.push((self.pos + bs as u64, found.count));
            bs = found.end;
        }
        debug_assert!(bs > 0, "a window always yields a block");
        let mut index = lock(&self.index);
        index.try_reserve(blocks.len())?;
        index.long_anchors.try_reserve(anchors.len())?;
        index.extend(&blocks, self.pos + bs as u64);
        index.long_anchors.extend(anchors);
        self.blocks += blocks.len();
        self.breaks = index.breaks;
        self.pos += bs as u64;
        Ok(())
    }

    /// The decoded length of `bytes`.
    fn decoded_len(&self, bytes: &[u8]) -> u64 {
        let mut text = String::with_capacity(bytes.len());
        self.codec.decode(bytes, &mut text);
        text.len() as u64
    }

    /// A block that is in a long line gets an anchor: where the line started and how long the
    /// decoded text of the line is before the block. The line is known to be long when it has gone
    /// on for `long_line` bytes; its start is read again then, once (about `long_line` bytes), and
    /// the blocks of the start get their anchors. `blocks` are the blocks of this window that were
    /// cut already.
    fn note_long_line(
        &mut self,
        bytes: &[u8],
        bs: usize,
        found: &BlockBreaks,
        blocks: &[(u64, u64)],
        anchors: &mut Vec<LongAnchor>,
    ) -> Result<(), ScanError> {
        let block = self.blocks + blocks.len();
        // The block starts with a line ending: the line before it ended at the edge.
        if found.first_start == Some(bs) {
            self.long = None;
            return Ok(());
        }
        let at = self.pos + bs as u64;
        if self.long.is_none() && self.long_line > 0 && self.since_break >= self.long_line {
            let line_start = at - self.since_break;
            let head = self.source.read(line_start, at)?;
            if head.len() as u64 != at - line_start {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
            }
            // the blocks that start in the line so far: in the index, and in this window
            let mut starts: Vec<(usize, u64)> = {
                let index = lock(&self.index);
                let first = index.starts.partition_point(|&s| s < line_start);
                (first..index.starts.len())
                    .map(|i| (i, index.starts[i]))
                    .collect()
            };
            starts.extend(
                blocks
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.0 >= line_start)
                    .map(|(j, b)| (self.blocks + j, b.0)),
            );
            anchors.try_reserve(starts.len() + 1)?;
            let (mut done, mut dec) = (0usize, 0u64);
            for (number, start) in starts {
                let upto = (start - line_start) as usize;
                dec += self.decoded_len(&head[done..upto]);
                done = upto;
                anchors.push(LongAnchor {
                    block: number,
                    line_start,
                    dec_before: dec,
                });
            }
            let dec = dec + self.decoded_len(&head[done..]);
            self.long = Some((line_start, dec));
        }
        if let Some((line_start, dec)) = self.long {
            anchors.try_reserve(1)?;
            anchors.push(LongAnchor {
                block,
                line_start,
                dec_before: dec,
            });
            let part = &bytes[bs..found.first_start.unwrap_or(found.end)];
            let dec = dec + self.decoded_len(part);
            // A line ending in this block ends the line.
            self.long = found.first_start.is_none().then_some((line_start, dec));
        }
        Ok(())
    }

    /// Cuts the block that starts at `bs` of `bytes`: returns where it ends and which line
    /// endings start in it. The block is at least `block_size` long (or reaches the end of
    /// `bytes`), ends at a character boundary and does not cut a line ending.
    fn cut_block(&self, bytes: &[u8], bs: usize, reach: usize) -> BlockBreaks {
        let until = bs + self.block_size;
        // Only these bytes are searched for line endings: the search of a block must not run to
        // the end of the window (a file without line endings would cost a window per block).
        let ahead = &bytes[..(bs + reach).min(bytes.len())];
        let mut count = 0;
        let mut first_start = None;
        let mut last_end = None;
        let next = walk_eols(
            &*self.codec,
            &self.lens,
            ahead,
            bs,
            until,
            self.ext_eol,
            |off, len, _| {
                count += 1;
                first_start.get_or_insert(off);
                last_end = Some(off + len);
            },
        );
        let mut end = match last_end {
            // A line ending crosses the edge: the block ends with it.
            Some(e) if e > until => e,
            _ => {
                // Walk the characters from a boundary we know: the end of the last line ending,
                // or the start of the block.
                let from = last_end.unwrap_or(bs);
                self.codec.boundary_at_or_after(bytes, from, until)
            }
        };
        // A character that a decoder would take across the next line ending is cut before it:
        // line endings are found first (the same as the dense index).
        if let Some(next) = next {
            end = end.min(next);
        }
        BlockBreaks {
            end: end.min(bytes.len()),
            count,
            first_start,
            last_end,
        }
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Scans `source` to the end and returns the index. `progress` is called after every read with how
/// far the scan is; return `ControlFlow::Break(())` to cancel ([`ScanError::Cancelled`]).
pub fn scan_blocks(
    source: Arc<dyn Source>,
    codec: Arc<dyn Codec>,
    options: ScanOptions,
    mut progress: impl FnMut(ScanProgress) -> ControlFlow<()>,
) -> Result<BlockIndex, ScanError> {
    let mut scanner = BlockScanner::new(source, codec, options)?;
    while !scanner.is_done() {
        scanner.step()?;
        if progress(scanner.progress()).is_break() {
            return Err(ScanError::Cancelled);
        }
    }
    let index = lock(&scanner.index).clone();
    Ok(index)
}

/// What a failed scan reports while it is running or after it, for the status display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanFailure {
    pub message: String,
    /// The file was changed by another program ([`is_changed_outside`](crate::file::is_changed_outside)).
    pub changed_outside: bool,
}

#[derive(Debug)]
struct JobShared {
    cancel: AtomicBool,
    scanned: AtomicU64,
    breaks: AtomicU64,
    state: AtomicU8,
    failure: Mutex<Option<ScanFailure>>,
}

/// A scan on its own thread (ADR-0020, chapter 2). The document is read-only while it runs:
/// [`ScanJob::snapshot`] gives a read-only document over the part scanned so far (make a new one
/// every few seconds), and [`ScanJob::finish`] the complete, editable document.
pub struct ScanJob {
    shared: Arc<JobShared>,
    index: Arc<Mutex<BlockIndex>>,
    total: u64,
    thread: Option<JoinHandle<Result<(), ScanError>>>,
}

impl fmt::Debug for ScanJob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScanJob")
            .field("progress", &self.progress())
            .finish_non_exhaustive()
    }
}

impl ScanJob {
    /// Starts scanning `source` on a new thread.
    ///
    /// # Errors
    ///
    /// [`ScanError::Stateful`] for an encoding that depends on earlier bytes, or the error of
    /// starting the thread.
    pub fn spawn(
        source: Arc<dyn Source>,
        codec: Arc<dyn Codec>,
        options: ScanOptions,
    ) -> Result<Self, ScanError> {
        let mut scanner = BlockScanner::new(source, codec, options)?;
        let shared = Arc::new(JobShared {
            cancel: AtomicBool::new(false),
            scanned: AtomicU64::new(0),
            breaks: AtomicU64::new(0),
            state: AtomicU8::new(ScanState::Running.to_u8()),
            failure: Mutex::new(None),
        });
        let index = scanner.shared();
        let total = scanner.progress().total;
        if scanner.is_done() {
            shared
                .state
                .store(ScanState::Done.to_u8(), Ordering::Release);
        }
        let worker = shared.clone();
        let delay = options.step_delay;
        let thread = std::thread::Builder::new()
            .name("matsu-block-scan".into())
            .spawn(move || {
                let result = run(&mut scanner, &worker, delay);
                let state = match &result {
                    Ok(()) => ScanState::Done,
                    Err(ScanError::Cancelled) => ScanState::Cancelled,
                    Err(e) => {
                        let changed_outside =
                            matches!(e, ScanError::Io(io) if crate::file::is_changed_outside(io));
                        *lock(&worker.failure) = Some(ScanFailure {
                            message: e.to_string(),
                            changed_outside,
                        });
                        ScanState::Failed
                    }
                };
                worker.state.store(state.to_u8(), Ordering::Release);
                result
            })?;
        Ok(Self {
            shared,
            index,
            total,
            thread: Some(thread),
        })
    }

    /// How far the scan is. Cheap; call it as often as the display needs.
    pub fn progress(&self) -> ScanProgress {
        ScanProgress {
            scanned: self.shared.scanned.load(Ordering::Acquire),
            total: self.total,
            breaks: self.shared.breaks.load(Ordering::Acquire),
            state: ScanState::from_u8(self.shared.state.load(Ordering::Acquire)),
        }
    }

    /// Whether the thread has stopped: the scan is done, cancelled or failed.
    pub fn is_finished(&self) -> bool {
        self.progress().state != ScanState::Running
    }

    /// Asks the scan to stop at the next read. The thread ends by itself.
    pub fn cancel(&self) {
        self.shared.cancel.store(true, Ordering::Release);
    }

    /// Why the scan failed, once it has.
    pub fn failure(&self) -> Option<ScanFailure> {
        lock(&self.shared.failure).clone()
    }

    /// The index of the part scanned so far (all of it when the scan is done).
    pub fn index_snapshot(&self) -> BlockIndex {
        lock(&self.index).clone()
    }

    /// A document over the part scanned so far. It is read-only until the scan is done
    /// ([`Document::is_read_only`]).
    pub fn snapshot(&self) -> Result<Document, ScanError> {
        Ok(Document::from_block_index(self.index_snapshot())?)
    }

    /// Waits for the scan to end and returns the complete document, which can be edited.
    pub fn finish(mut self) -> Result<Document, ScanError> {
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .unwrap_or_else(|_| Err(io::Error::other("the scan thread panicked").into()))?;
        }
        Ok(Document::from_block_index(self.index_snapshot())?)
    }
}

impl Drop for ScanJob {
    /// Stops the scan; the thread is not waited for.
    fn drop(&mut self) {
        self.cancel();
    }
}

fn run(
    scanner: &mut BlockScanner,
    shared: &JobShared,
    delay: Option<std::time::Duration>,
) -> Result<(), ScanError> {
    while !scanner.is_done() {
        if shared.cancel.load(Ordering::Acquire) {
            return Err(ScanError::Cancelled);
        }
        scanner.step()?;
        if let Some(delay) = delay {
            std::thread::sleep(delay);
        }
        let progress = scanner.progress();
        shared.breaks.store(progress.breaks, Ordering::Release);
        shared.scanned.store(progress.scanned, Ordering::Release);
    }
    Ok(())
}
