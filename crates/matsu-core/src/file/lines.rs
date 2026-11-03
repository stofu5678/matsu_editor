//! Reading a file line by line in order, without an index (ADR-0020, chapter 5, "Grep").
//!
//! [`LineReader`] reads a [`Source`] in chunks, cuts the lines at their endings and decodes one
//! line at a time. Nothing but the chunk being cut (and the line that is not finished yet) is in
//! memory, so a file of any size is read with a small, steady amount of memory. The line number
//! is counted by the caller, as it reads. [`open_line_stream`] opens a file this way.
//!
//! A line ending, a character or a UTF-16 surrogate pair may sit across the edge of two chunks.
//! The reader never cuts a line before it has seen a few bytes past the ending (so that a CR at
//! the edge is joined with the LF that follows it) and looks for the next ending only from a
//! character boundary (so that the second byte of a Shift_JIS character is not taken for an
//! ending).
//!
//! The lines are the lines of [`Document::line`](crate::buffer::Document::line): the text without
//! its ending, and the ending. The text after the last ending is a line when it is not empty.

use std::io;
use std::path::Path;
use std::sync::Arc;

use super::source::OffsetSource;
use super::{FileError, OpenOptions, RangedFile, Result};
use crate::buffer::{Codec, EncodingCodec, Line, Source, find_eol};
use crate::encoding::{DETECT_SAMPLE_LEN, Encoding, detect};

/// The bytes one read of a [`LineReader`] asks the source for, by default.
pub const DEFAULT_STREAM_CHUNK: usize = 1024 * 1024;

/// How far past an ending the reader looks before it cuts there: a CR at the edge of the bytes
/// read so far may be the first half of CR LF (8 bytes in UTF-32).
const LOOKAHEAD: usize = 16;

/// Reads the lines of a source, one after another. See the module documentation.
pub struct LineReader {
    source: Arc<dyn Source>,
    codec: Arc<dyn Codec>,
    ext_eol: bool,
    chunk: usize,
    /// The bytes read and not yet given out. The lines in `buf[start..]` are the next ones.
    buf: Vec<u8>,
    start: usize,
    /// Where the search for the next ending goes on: a character boundary of `buf`.
    scan_from: usize,
    /// The offset in the source of the end of `buf`.
    pos: u64,
    peak: usize,
}

impl std::fmt::Debug for LineReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LineReader")
            .field("pos", &self.pos)
            .field("buffered", &(self.buf.len() - self.start))
            .finish_non_exhaustive()
    }
}

impl LineReader {
    /// A reader of `source` through `codec`, starting at its first byte (the source has no BOM
    /// in front). `ext_eol` is the setting `bEnableExtEol` (NEL, LS and PS end a line). Each read
    /// of the source asks for `chunk` bytes (rounded up to a multiple of 4, at least 4).
    ///
    /// # Panics
    ///
    /// When the codec keeps a state between characters (ISO-2022-JP, UTF-7): such a file cannot
    /// be cut anywhere, and is read whole.
    pub fn new(
        source: Arc<dyn Source>,
        codec: Arc<dyn Codec>,
        ext_eol: bool,
        chunk: usize,
    ) -> Self {
        assert!(codec.is_stateless(), "a stateful encoding is read whole");
        Self {
            source,
            codec,
            ext_eol,
            chunk: chunk.max(1).next_multiple_of(4),
            buf: Vec::new(),
            start: 0,
            scan_from: 0,
            pos: 0,
            peak: 0,
        }
    }

    /// The number of bytes of the source that have been read from it so far.
    pub fn bytes_read(&self) -> u64 {
        self.pos
    }

    /// The length of the source.
    pub fn total(&self) -> u64 {
        self.source.len()
    }

    /// The most bytes this reader has held at once (the chunk being cut and an unfinished line).
    /// For tests, and for a caller that wants to show how much memory a long line costs.
    pub fn peak_buffered(&self) -> usize {
        self.peak
    }

    /// The next line, or `None` at the end.
    ///
    /// # Errors
    ///
    /// When the source cannot be read. The reader should not be used after that.
    pub fn next_line(&mut self) -> io::Result<Option<Line>> {
        self.next_line_until(&mut || false)
    }

    /// Like [`next_line`](Self::next_line), asking `stop` before each read of the source: a line
    /// of any length needs many reads, and a caller that can be stopped must be able to stop
    /// inside one. When `stop` says so, the error has the kind [`io::ErrorKind::Interrupted`]
    /// and nothing is lost: the next call goes on with the same line.
    pub fn next_line_until(&mut self, stop: &mut dyn FnMut() -> bool) -> io::Result<Option<Line>> {
        loop {
            let at_eof = self.pos >= self.source.len();
            if let Some((off, kind)) =
                find_eol(&*self.codec, &self.buf, self.scan_from, self.ext_eol)
            {
                let eol_len = self.codec.eol_bytes(kind).map_or(1, <[u8]>::len);
                if at_eof || self.buf.len() - off >= LOOKAHEAD.max(eol_len) {
                    let mut text = String::new();
                    self.codec.decode(&self.buf[self.start..off], &mut text);
                    self.start = (off + eol_len).min(self.buf.len());
                    self.scan_from = self.start;
                    return Ok(Some(Line {
                        text,
                        eol: Some(kind),
                    }));
                }
                // The ending is too near the edge to be sure of it: read on, and look again from
                // its start.
                self.scan_from = off;
            } else if at_eof {
                if self.start >= self.buf.len() {
                    return Ok(None);
                }
                let mut text = String::new();
                self.codec.decode(&self.buf[self.start..], &mut text);
                self.start = self.buf.len();
                self.scan_from = self.start;
                return Ok(Some(Line { text, eol: None }));
            } else {
                // No ending in what is held: the next search starts a little before the end (an
                // ending may begin there), at a character boundary.
                let target = self.buf.len().saturating_sub(LOOKAHEAD);
                if target > self.scan_from {
                    self.scan_from =
                        self.codec
                            .boundary_at_or_after(&self.buf, self.scan_from, target);
                }
            }
            if stop() {
                return Err(io::Error::from(io::ErrorKind::Interrupted));
            }
            self.fill()?;
        }
    }

    /// Reads the next chunk of the source after the bytes held, dropping the lines already given
    /// out.
    fn fill(&mut self) -> io::Result<()> {
        if self.start > 0 {
            self.buf.drain(..self.start);
            self.scan_from -= self.start;
            self.start = 0;
        }
        let end = self
            .pos
            .saturating_add(self.chunk as u64)
            .min(self.source.len());
        let bytes = self.source.read(self.pos, end)?;
        if bytes.len() as u64 != end - self.pos {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
        self.buf
            .try_reserve(bytes.len())
            .map_err(|e| io::Error::new(io::ErrorKind::OutOfMemory, e))?;
        self.buf.extend_from_slice(&bytes);
        self.pos = end;
        self.peak = self.peak.max(self.buf.len());
        Ok(())
    }
}

/// A file opened to be read line by line ([`open_line_stream`]).
#[derive(Debug)]
pub struct LineStream {
    /// The encoding the lines are decoded in.
    pub encoding: Encoding,
    /// The file starts with the BOM of the encoding (it is not part of the first line).
    pub bom: bool,
    reader: LineReader,
}

impl LineStream {
    /// The next line, or `None` at the end. See [`LineReader::next_line`].
    pub fn next_line(&mut self) -> io::Result<Option<Line>> {
        self.reader.next_line()
    }

    /// The reader behind the stream.
    pub fn reader(&self) -> &LineReader {
        &self.reader
    }

    /// The reader behind the stream, to read it with [`LineReader::next_line_until`].
    pub fn into_reader(self) -> LineReader {
        self.reader
    }
}

/// Opens `path` to read its lines in order, without reading it whole (ADR-0020, chapter 5). The
/// encoding is `options.encoding`, or detected from the start of the file as
/// [`open_file_with`](super::open_file_with) does.
///
/// `None` when the file should be opened whole instead: it is smaller than
/// [`OpenOptions::huge_file_len`] (`None` there means every file is), it is empty, or its
/// encoding keeps a state between characters (ISO-2022-JP, UTF-7), which cannot be cut anywhere.
///
/// # Errors
///
/// When the file cannot be opened or read.
pub fn open_line_stream(
    path: impl AsRef<Path>,
    options: &OpenOptions,
    chunk: usize,
) -> Result<Option<LineStream>> {
    let path = path.as_ref();
    if !std::fs::metadata(path)?.is_file() {
        return Err(FileError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not a regular file",
        )));
    }
    let ranged = RangedFile::open(path)?;
    let len = ranged.len();
    let Some(from) = options.huge_file_len else {
        return Ok(None);
    };
    if len < from || len == 0 {
        return Ok(None);
    }
    let head = ranged
        .read(0, len.min(DETECT_SAMPLE_LEN as u64))?
        .into_owned();
    let (encoding, bom) = match options.encoding {
        Some(encoding) => (encoding, encoding.bom_len(&head) > 0),
        None => {
            let d = detect(&head, &options.detect);
            (d.encoding, d.bom)
        }
    };
    if !encoding.codec().is_stateless() {
        return Ok(None);
    }
    let bom_len = if bom { encoding.bom().len() } else { 0 };
    let source = Arc::new(OffsetSource::new(ranged, bom_len as u64));
    Ok(Some(LineStream {
        encoding,
        bom,
        reader: LineReader::new(
            source,
            Arc::new(EncodingCodec(encoding)),
            options.ext_eol,
            chunk,
        ),
    }))
}

#[cfg(test)]
mod tests;
