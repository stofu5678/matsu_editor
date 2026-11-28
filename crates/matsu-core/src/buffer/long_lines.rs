//! Places in a very long line of a document that is read whole (ADR-0020, 9.10).
//!
//! A document read by ranges gets these places from its block index while it scans the file. A
//! document that is read whole has the line endings of the whole file in a [`LineIndex`] and
//! nothing else, so a place inside a line of a million bytes or more could only be found by
//! decoding the line from its start (#614: 0.1 s for a line of 200 MiB, at every call).
//!
//! [`FullIndex`] adds the same kind of markers as the block index has, built the first time that
//! someone asks and only for the long lines: for each part of [`MARK_BYTES`] bytes of a long line,
//! where it starts in the file and in the decoded text. A place is then found by looking at one
//! part. Typed and pasted text is UTF-8 in the add buffer, where a column is a byte of the file
//! already, so only the original data needs markers.

use std::io;
use std::sync::{Arc, OnceLock};

use super::block::DEFAULT_LONG_LINE;
use super::codec::Codec;
use super::storage::{Entry, EolIndex, LineIndex, Source};

/// The size of the parts of a long line that have a marker.
const MARK_BYTES: usize = 64 * 1024;

/// A range shorter than this is decoded as it is: it costs less than finding the long lines of the
/// file (which a document with none never needs), and it is the work of one row of a long line.
const MIN_RANGE_BYTES: usize = 8 * 1024;

/// A part of a long line: where it starts in the file and in the decoded text of the line.
#[derive(Clone, Copy, Debug)]
struct Mark {
    raw: usize,
    dec: usize,
    /// The decoded text of the part is its bytes (UTF-8 or ASCII text): a place in it is found by
    /// counting, without decoding it.
    identity: bool,
}

/// A line of the file that has [`DEFAULT_LONG_LINE`] bytes or more, from its start to the start of
/// its line ending.
#[derive(Debug)]
struct LongLine {
    start: usize,
    end: usize,
    /// The length of the decoded text of the whole line.
    dec_end: usize,
    /// At least one: the first starts at `start`.
    marks: Vec<Mark>,
}

/// The line index of a document that is read whole, with the places inside its very long lines.
#[derive(Debug)]
pub(crate) struct FullIndex {
    lines: LineIndex,
    source: Arc<dyn Source>,
    codec: Arc<dyn Codec>,
    /// The long lines of the file, found at the first question about one. `None` when reading the
    /// file failed: the callers then decode the bytes, which is slow and right.
    long: OnceLock<Option<Vec<LongLine>>>,
}

impl FullIndex {
    pub fn new(lines: LineIndex, source: Arc<dyn Source>, codec: Arc<dyn Codec>) -> Self {
        Self {
            lines,
            source,
            codec,
            long: OnceLock::new(),
        }
    }

    fn long_lines(&self) -> Option<&[LongLine]> {
        self.long
            .get_or_init(|| self.find_long_lines().ok())
            .as_deref()
    }

    fn find_long_lines(&self) -> io::Result<Vec<LongLine>> {
        let len = usize::try_from(self.source.len()).unwrap_or(usize::MAX);
        let mut found = Vec::new();
        if len < DEFAULT_LONG_LINE {
            return Ok(found);
        }
        let mut line_start = 0;
        for j in 0..EolIndex::len(&self.lines) {
            let Entry {
                start, len: eol, ..
            } = self.lines.entry(j)?;
            if start - line_start >= DEFAULT_LONG_LINE {
                found.push(self.mark_line(line_start, start)?);
            }
            line_start = start + eol;
        }
        if len - line_start >= DEFAULT_LONG_LINE {
            found.push(self.mark_line(line_start, len)?);
        }
        Ok(found)
    }

    /// The markers of the line `start..end`, read part by part (a file source is not read whole).
    fn mark_line(&self, start: usize, end: usize) -> io::Result<LongLine> {
        let mut marks = Vec::new();
        marks
            .try_reserve(end.saturating_sub(start) / MARK_BYTES + 1)
            .map_err(io::Error::other)?;
        let (mut raw, mut dec) = (start, 0);
        let mut text = String::new();
        while raw < end {
            // A few bytes more than a part, to find the edge of the character at its end.
            let look = (raw + MARK_BYTES + 8).min(end);
            let bytes = self.source.read(raw as u64, look as u64)?;
            let take = if raw + MARK_BYTES >= end {
                end - raw
            } else {
                self.codec
                    .boundary_at_or_after(&bytes, 0, MARK_BYTES)
                    .clamp(1, bytes.len())
            };
            text.clear();
            self.codec.decode(&bytes[..take], &mut text);
            marks.push(Mark {
                raw,
                dec,
                identity: text.as_bytes() == &bytes[..take],
            });
            dec += text.len();
            raw += take;
        }
        Ok(LongLine {
            start,
            end,
            dec_end: dec,
            marks,
        })
    }

    /// The long line that holds the bytes `start..end`, if there is one.
    fn line_of(&self, start: usize, end: usize) -> Option<&LongLine> {
        let lines = self.long_lines()?;
        let i = lines.partition_point(|l| l.start <= start).checked_sub(1)?;
        lines.get(i).filter(|l| end <= l.end)
    }

    /// The length of the decoded text of `line` from its start to the byte `x` of the file.
    fn dec_at(&self, line: &LongLine, x: usize) -> io::Result<usize> {
        let k = line.marks.partition_point(|m| m.raw <= x).saturating_sub(1);
        let mark = line.marks[k];
        if mark.identity {
            return Ok(mark.dec + (x - mark.raw));
        }
        let bytes = self.source.read(mark.raw as u64, x as u64)?;
        let mut text = String::with_capacity(bytes.len());
        self.codec.decode(&bytes, &mut text);
        Ok(mark.dec + text.len())
    }

    /// The byte of the file where `target` bytes of the decoded text of `line` have passed,
    /// rounded down to a character.
    fn raw_at(&self, line: &LongLine, target: usize) -> io::Result<usize> {
        if target >= line.dec_end {
            return Ok(line.end);
        }
        let k = line
            .marks
            .partition_point(|m| m.dec <= target)
            .saturating_sub(1);
        let mark = line.marks[k];
        let part_end = line.marks.get(k + 1).map_or(line.end, |next| next.raw);
        if mark.identity {
            // The text is valid UTF-8 and the same bytes: back to the start of the character.
            let mut at = mark.raw + (target - mark.dec);
            let from = at.saturating_sub(3).max(mark.raw);
            let bytes = self.source.read(from as u64, (at + 1) as u64)?;
            while at > from && bytes.get(at - from).is_some_and(|b| b & 0xC0 == 0x80) {
                at -= 1;
            }
            return Ok(at);
        }
        let bytes = self.source.read(mark.raw as u64, part_end as u64)?;
        let (mut at, mut dec) = (0, mark.dec);
        let mut text = String::new();
        while at < bytes.len() {
            let n = self.codec.char_len(&bytes, at).clamp(1, bytes.len() - at);
            text.clear();
            self.codec.decode(&bytes[at..at + n], &mut text);
            if dec + text.len() > target {
                return Ok(mark.raw + at);
            }
            dec += text.len();
            at += n;
        }
        Ok(part_end)
    }
}

impl EolIndex for FullIndex {
    fn len(&self) -> usize {
        EolIndex::len(&self.lines)
    }

    fn count_before(&self, pos: usize) -> io::Result<usize> {
        self.lines.count_before(pos)
    }

    fn entry(&self, j: usize) -> io::Result<Entry> {
        self.lines.entry(j)
    }

    fn heap_bytes(&self) -> usize {
        EolIndex::heap_bytes(&self.lines)
    }

    fn long_line_bytes(&self) -> Option<usize> {
        Some(DEFAULT_LONG_LINE)
    }

    fn decoded_len_long(&self, start: usize, end: usize) -> io::Result<Option<usize>> {
        if end < start || end - start < MIN_RANGE_BYTES {
            return Ok(None);
        }
        let Some(line) = self.line_of(start, end) else {
            return Ok(None);
        };
        let (a, b) = (self.dec_at(line, start)?, self.dec_at(line, end)?);
        Ok(b.checked_sub(a))
    }

    fn raw_at_decoded_long(
        &self,
        start: usize,
        end: usize,
        cols: usize,
    ) -> io::Result<Option<usize>> {
        if end < start || end - start < MIN_RANGE_BYTES {
            return Ok(None);
        }
        let Some(line) = self.line_of(start, end) else {
            return Ok(None);
        };
        let target = self.dec_at(line, start)?.saturating_add(cols);
        Ok(Some(self.raw_at(line, target)?.clamp(start, end)))
    }
}

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use super::*;
    use crate::buffer::{
        Document, EncodingCodec, MemorySource, Pos, ScanOptions, Source, scan_blocks,
    };
    use crate::encoding::Encoding;

    /// Text of about `len` bytes with characters of one, two and three bytes (as UTF-8), and
    /// half-width katakana, which take one byte in Shift_JIS and three in UTF-8.
    fn mixed(len: usize) -> String {
        let mut line = String::new();
        let mut i = 0;
        while line.len() < len {
            line.push_str(["abc", "é", "日本語", "ｱｲ", "x", "\t"][i % 6]);
            i += 1;
        }
        line
    }

    /// `text` in `encoding`, with the characters the encoding has (`é` is left out of Shift_JIS by
    /// the caller).
    fn encoded(encoding: Encoding, text: &str) -> Vec<u8> {
        let mut out = Vec::new();
        let _ = encoding.codec().encode(text, &mut out);
        out
    }

    /// The same bytes as a document read whole and as one read by ranges.
    fn both(encoding: Encoding, bytes: Vec<u8>) -> (Document, Document) {
        let codec = Arc::new(EncodingCodec(encoding));
        let whole = Document::new(MemorySource::new(bytes.clone()), codec.clone()).unwrap();
        let index = scan_blocks(
            Arc::new(MemorySource::new(bytes)),
            codec,
            ScanOptions::default(),
            |_| ControlFlow::Continue(()),
        )
        .unwrap();
        (whole, Document::from_block_index(index).unwrap())
    }

    /// Every question that a view asks about a place in line 1 answers the same in both.
    fn same_answers(whole: &Document, ranged: &Document) {
        assert!(whole.is_long_line(1) && ranged.is_long_line(1));
        assert!(!whole.is_long_line(0) && !whole.is_long_line(2));
        let len = ranged.line_len(1).unwrap().unwrap();
        assert_eq!(whole.line_len(1).unwrap(), Some(len));
        assert_eq!(
            whole.line_raw_len(1).unwrap(),
            ranged.line_raw_len(1).unwrap()
        );
        // the whole text, to know where the characters are
        let text = whole.line(1).unwrap().unwrap().text;
        assert_eq!(text.len(), len);
        let mut cols: Vec<usize> = (0..len).step_by(7919).collect();
        cols.extend([
            1,
            4_095,
            8_191,
            8_192,
            65_535,
            65_536,
            65_537,
            131_071,
            131_072,
            len / 2,
            len - 1,
            len,
        ]);
        for col in cols {
            let col = (0..=col).rev().find(|&c| text.is_char_boundary(c)).unwrap();
            let to = (col + 40).min(len);
            assert_eq!(
                whole.line_slice(1, col, to).unwrap(),
                ranged.line_slice(1, col, to).unwrap(),
                "col {col}"
            );
            let slice = whole.line_slice(1, col, to).unwrap().unwrap();
            assert_eq!(slice.start_col, col);
            assert!(text[col..].starts_with(&slice.text));
            assert_eq!(whole.utf16_col(1, col), ranged.utf16_col(1, col));
        }
    }

    #[test]
    fn a_very_long_line_of_a_whole_document_answers_as_one_read_by_ranges() {
        for encoding in [Encoding::Utf8, Encoding::ShiftJis, Encoding::Utf16Be] {
            let line = mixed(2_400_000);
            let line = if encoding == Encoding::ShiftJis {
                line.replace('é', "e")
            } else {
                line
            };
            let text = format!("first\n{line}\nlast\n");
            let (whole, ranged) = both(encoding, encoded(encoding, &text));
            same_answers(&whole, &ranged);
        }
    }

    #[test]
    fn invalid_bytes_in_a_very_long_line_count_as_the_stand_ins_the_decoder_makes() {
        // `0xFF` is not UTF-8: it decodes to a stand-in of four bytes, so the text is longer than
        // the file there, and the part that is its own bytes does not hold.
        let mut bytes = b"first\n".to_vec();
        for i in 0..1_300_000usize {
            bytes.push(if i % 5000 == 0 {
                0xFF
            } else {
                b'a' + (i % 26) as u8
            });
        }
        bytes.extend_from_slice(b"\nlast\n");
        let (whole, ranged) = both(Encoding::Utf8, bytes);
        same_answers(&whole, &ranged);
    }

    #[test]
    fn an_edit_in_a_very_long_line_keeps_the_places_right() {
        let text = format!("first\n{}\nlast\n", mixed(1_300_000));
        let (mut whole, _) = both(Encoding::Utf8, text.into_bytes());
        whole.insert(Pos::new(1, 600_000), "あいう").unwrap();
        whole
            .delete(Pos::new(1, 100_000), Pos::new(1, 100_030))
            .unwrap();
        assert!(whole.is_long_line(1));
        let all = whole.line(1).unwrap().unwrap().text;
        assert_eq!(whole.line_len(1).unwrap(), Some(all.len()));
        for col in [
            0, 99_999, 100_000, 300_000, 599_999, 600_000, 600_009, 1_000_000,
        ] {
            let col = (0..=col).rev().find(|&c| all.is_char_boundary(c)).unwrap();
            let slice = whole.line_slice(1, col, col + 30).unwrap().unwrap();
            assert_eq!(slice.start_col, col);
            assert!(all[col..].starts_with(&slice.text));
            assert!(!slice.text.is_empty());
        }
    }

    #[test]
    fn a_line_is_very_long_from_one_mebibyte_and_not_before() {
        let doc = |len: usize| {
            let text = format!("a\n{}\nb", "x".repeat(len));
            Document::new(
                MemorySource::new(text.into_bytes()),
                Arc::new(EncodingCodec(Encoding::Utf8)),
            )
            .unwrap()
        };
        assert!(!doc(DEFAULT_LONG_LINE - 1).is_long_line(1));
        assert!(doc(DEFAULT_LONG_LINE).is_long_line(1));
        assert!(!doc(10).is_long_line(1));
        // a line that is made of pasted text counts too
        let mut pasted = doc(10);
        pasted
            .insert(Pos::new(1, 0), &"y".repeat(DEFAULT_LONG_LINE))
            .unwrap();
        assert!(pasted.is_long_line(1));
        assert_eq!(pasted.line_len(1).unwrap(), Some(DEFAULT_LONG_LINE + 10));
    }

    /// ISO-2022-JP cannot be decoded from the middle, so the document converts it at load time
    /// and keeps the text as typed text: its long lines are long all the same.
    #[test]
    fn a_line_of_a_stateful_encoding_is_very_long_too() {
        let line = "日本語".repeat(130_000);
        let bytes = encoded(Encoding::Jis, &format!("first\n{line}\nlast\n"));
        let doc = Document::new(
            MemorySource::new(bytes),
            Arc::new(EncodingCodec(Encoding::Jis)),
        )
        .unwrap();
        assert!(doc.is_long_line(1));
        assert!(!doc.is_long_line(0));
        let slice = doc.line_slice(1, 900_000, 900_040).unwrap().unwrap();
        assert_eq!(slice.start_col, 900_000);
        assert!(line[900_000..].starts_with(&slice.text) && !slice.text.is_empty());
        assert_eq!(doc.line_len(1).unwrap(), Some(line.len()));
    }

    /// The source is read in parts: a line of 3 MB makes no read of the size of the line.
    #[test]
    fn the_markers_are_made_part_by_part() {
        use std::sync::atomic::{AtomicU64, Ordering};

        #[derive(Debug)]
        struct Widest(MemorySource, AtomicU64);
        impl Source for Widest {
            fn len(&self) -> u64 {
                self.0.len()
            }
            fn read(&self, start: u64, end: u64) -> io::Result<std::borrow::Cow<'_, [u8]>> {
                self.1.fetch_max(end - start, Ordering::Relaxed);
                self.0.read(start, end)
            }
        }
        let bytes = format!("a\n{}\nb", mixed(3_000_000)).into_bytes();
        let source = Arc::new(Widest(MemorySource::new(bytes.clone()), AtomicU64::new(0)));
        let mut lines = LineIndex::default();
        lines.scan(Encoding::Utf8.codec(), &bytes, 0, true).unwrap();
        let codec = Arc::new(EncodingCodec(Encoding::Utf8));
        let index = FullIndex::new(lines, source.clone(), codec);
        assert!(index.line_of(3, 2_900_000).is_some());
        assert!(source.1.load(Ordering::Relaxed) <= (MARK_BYTES + 8) as u64);
    }
}
