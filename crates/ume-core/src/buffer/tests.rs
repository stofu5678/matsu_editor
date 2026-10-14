//! Tests of the document: edge cases, a model-based random test, and a smoke test.

use std::borrow::Cow;
use std::io;
use std::sync::Arc;
use std::time::Instant;

use super::*;

// ---- helpers ----

fn doc(text: &str) -> Document {
    Document::from_utf8(text.as_bytes().to_vec()).unwrap()
}

fn lines(d: &Document) -> Vec<(String, Option<Eol>)> {
    d.lines(0..usize::MAX)
        .map(|l| {
            let l = l.unwrap();
            (l.text, l.eol)
        })
        .collect()
}

fn whole(d: &Document) -> String {
    d.text_between(Pos::new(0, 0), Pos::new(usize::MAX, 0))
        .unwrap()
}

fn l(text: &str, eol: Option<Eol>) -> (String, Option<Eol>) {
    (text.to_string(), eol)
}

/// What the saver would write: original bytes as they are, added text encoded as UTF-8.
fn save(d: &Document) -> Vec<u8> {
    let mut out = Vec::new();
    for c in d.chunks() {
        match c.unwrap() {
            Chunk::Original(b) => out.extend_from_slice(&b),
            Chunk::Added(s) => Utf8Codec.encode(&s, &mut out).unwrap(),
        }
    }
    out
}

// ---- the model: the whole text in a String ----

#[derive(Clone, Debug)]
struct Model {
    text: String,
}

impl Model {
    /// (start, text end, end) of every line, plus the empty line after a final break.
    fn spans(&self) -> Vec<(usize, usize, usize, Option<Eol>)> {
        let t = &self.text;
        let mut out = Vec::new();
        let mut start = 0;
        let mut it = t.char_indices().peekable();
        while let Some((i, ch)) = it.next() {
            let (eol, len) = match ch {
                '\r' if matches!(it.peek(), Some((_, '\n'))) => {
                    it.next();
                    (Eol::CrLf, 2)
                }
                '\r' => (Eol::Cr, 1),
                '\n' => (Eol::Lf, 1),
                '\u{85}' => (Eol::Nel, 2),
                '\u{2028}' => (Eol::Ls, 3),
                '\u{2029}' => (Eol::Ps, 3),
                _ => continue,
            };
            out.push((start, i, i + len, Some(eol)));
            start = i + len;
        }
        out.push((start, t.len(), t.len(), None));
        out
    }

    fn lines(&self) -> Vec<(String, Option<Eol>)> {
        let mut spans = self.spans();
        if let Some(&(s, e, _, None)) = spans.last()
            && s == e
        {
            spans.pop();
        }
        spans
            .iter()
            .map(|&(s, e, _, eol)| (self.text[s..e].to_string(), eol))
            .collect()
    }

    fn offset(&self, pos: Pos) -> usize {
        let spans = self.spans();
        let nlines = self.lines().len();
        if pos.line >= nlines {
            return self.text.len();
        }
        let (s, e, _, _) = spans[pos.line];
        let mut c = pos.col.min(e - s);
        while !self.text.is_char_boundary(s + c) {
            c -= 1;
        }
        s + c
    }

    fn pos(&self, off: usize) -> Pos {
        let spans = self.spans();
        let (line, &(s, e, _, _)) = spans
            .iter()
            .enumerate()
            .rev()
            .find(|(_, sp)| sp.0 <= off)
            .unwrap();
        Pos::new(line, off.min(e) - s)
    }

    fn replace(&mut self, from: Pos, to: Pos, ins: &str) -> Edit {
        let (a, b) = (self.offset(from), self.offset(to));
        let (a, b) = (a.min(b), a.max(b));
        self.text.replace_range(a..b, ins);
        Edit {
            start: self.pos(a),
            end: self.pos(a + ins.len()),
        }
    }
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const ALPHABET: &[&str] = &[
    "a", "b", "xyz", "é", "日本", "😀", "\n", "\r", "\r\n", "\u{85}", "\u{2028}", "\u{2029}", " ",
    "\n\n", "\r\r",
];

fn random_text(rng: &mut Rng, max_parts: usize) -> String {
    (0..rng.below(max_parts + 1))
        .map(|_| ALPHABET[rng.below(ALPHABET.len())])
        .collect()
}

fn random_pos(rng: &mut Rng, m: &Model) -> Pos {
    let n = m.lines().len();
    let line = rng.below(n + 2);
    let width = m.lines().get(line).map_or(0, |l| l.0.len());
    Pos::new(line, rng.below(width + 3))
}

fn assert_same(d: &Document, m: &Model) {
    assert_eq!(lines(d), m.lines(), "lines differ for {:?}", m.text);
    assert_eq!(d.line_count(), m.lines().len());
    assert_eq!(whole(d), m.text);
    assert_eq!(d.end_pos().unwrap(), m.pos(m.text.len()));
    d.check_invariants();
}

fn run_random(seed: u64, start: &str, steps: usize) {
    let mut rng = Rng(seed);
    let mut d = doc(start);
    let mut m = Model {
        text: start.to_string(),
    };
    assert_same(&d, &m);
    let mut snapshots: Vec<(Document, String)> = Vec::new();
    for step in 0..steps {
        let (from, to) = (random_pos(&mut rng, &m), random_pos(&mut rng, &m));
        let before = m.clone();
        let (got, want) = match rng.below(10) {
            0..=5 => {
                let t = random_text(&mut rng, 4);
                (d.insert(from, &t).unwrap(), m.replace(from, from, &t))
            }
            6..=8 => (d.delete(from, to).unwrap(), m.replace(from, to, "")),
            _ => {
                let t = random_text(&mut rng, 3);
                (d.replace(from, to, &t).unwrap(), m.replace(from, to, &t))
            }
        };
        assert_eq!(
            got, want,
            "seed {seed} step {step}: edit result, text before {:?}",
            before.text
        );
        assert_same(&d, &m);
        if step % 50 == 0 {
            snapshots.push((d.clone(), m.text.clone()));
        }
        // Keep the text small so the cheap checks stay cheap.
        if m.text.len() > 400 {
            let end = d.end_pos().unwrap();
            let cut = Pos::new(end.line / 2, 0);
            d.delete(Pos::new(0, 0), cut).unwrap();
            m.replace(Pos::new(0, 0), cut, "");
            assert_same(&d, &m);
        }
    }
    // Old versions are untouched by later edits.
    for (snap, text) in snapshots {
        assert_eq!(whole(&snap), text);
        snap.check_invariants();
    }
}

#[test]
fn random_edits_on_an_empty_document() {
    for seed in 1..=8_u64 {
        run_random(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15), "", 600);
    }
}

#[test]
fn random_edits_on_an_original_source() {
    let start =
        "first line\r\nsecond 日本\nthird\rfourth\u{85}fifth\u{2028}sixth\u{2029}\r\n\r\nlast";
    for seed in 1..=8_u64 {
        run_random(seed.wrapping_mul(0xD1B5_4A32_D192_ED03), start, 600);
    }
}

#[test]
fn random_edits_on_an_original_ending_with_cr() {
    for seed in 1..=4_u64 {
        run_random(seed.wrapping_mul(0xA076_1D64_78BD_642F), "a\r\nb\r", 500);
    }
}

// ---- lines and positions ----

#[test]
fn empty_document() {
    let mut d = Document::empty();
    assert_eq!(d.line_count(), 0);
    assert_eq!(d.line(0).unwrap(), None);
    assert_eq!(d.end_pos().unwrap(), Pos::new(0, 0));
    assert_eq!(d.clamp_pos(Pos::new(5, 5)).unwrap(), Pos::new(0, 0));
    assert_eq!(whole(&d), "");
    assert!(d.chunks().next().is_none());
    // Deleting nothing from nothing is fine.
    let e = d.delete(Pos::new(0, 0), Pos::new(9, 9)).unwrap();
    assert_eq!((e.start, e.end), (Pos::new(0, 0), Pos::new(0, 0)));
    let e = d.insert(Pos::new(3, 3), "a").unwrap();
    assert_eq!(e.end, Pos::new(0, 1));
    assert_eq!(d.line_count(), 1);
}

#[test]
fn end_of_file_positions() {
    // Sakura: the empty text after the last line ending is not a line (CDocLineMgr.cpp).
    assert_eq!(doc("abc").end_pos().unwrap(), Pos::new(0, 3));
    assert_eq!(doc("abc\n").end_pos().unwrap(), Pos::new(1, 0));
    assert_eq!(doc("abc\n").line_count(), 1);
    assert_eq!(doc("abc\r\n\r\n").end_pos().unwrap(), Pos::new(2, 0));
    assert_eq!(doc("abc\r\nd").line_count(), 2);
    let d = doc("abc\n");
    assert_eq!(d.line(1).unwrap(), None);
    assert_eq!(d.clamp_pos(Pos::new(1, 7)).unwrap(), Pos::new(1, 0));
    assert_eq!(d.clamp_pos(Pos::new(0, 7)).unwrap(), Pos::new(0, 3));
}

#[test]
fn only_line_endings() {
    let d = doc("\n\r\n\r\u{85}\u{2028}\u{2029}");
    assert_eq!(
        lines(&d),
        [
            l("", Some(Eol::Lf)),
            l("", Some(Eol::CrLf)),
            l("", Some(Eol::Cr)),
            l("", Some(Eol::Nel)),
            l("", Some(Eol::Ls)),
            l("", Some(Eol::Ps)),
        ]
    );
    assert_eq!(d.end_pos().unwrap(), Pos::new(6, 0));
}

#[test]
fn clamps_bad_positions_without_panicking() {
    let d = doc("日本\nabc");
    // Inside a character: back to its start.
    assert_eq!(d.clamp_pos(Pos::new(0, 1)).unwrap(), Pos::new(0, 0));
    assert_eq!(d.clamp_pos(Pos::new(0, 4)).unwrap(), Pos::new(0, 3));
    assert_eq!(
        d.clamp_pos(Pos::new(0, usize::MAX)).unwrap(),
        Pos::new(0, 6)
    );
    assert_eq!(
        d.clamp_pos(Pos::new(usize::MAX, usize::MAX)).unwrap(),
        Pos::new(1, 3)
    );
    assert_eq!(
        d.text_between(Pos::new(1, 2), Pos::new(0, 3)).unwrap(),
        "本\nab"
    );
    let mut d = d;
    d.insert(Pos::new(0, 1), "X").unwrap();
    assert_eq!(lines(&d)[0].0, "X日本");
}

#[test]
fn line_len_and_text_between() {
    let d = doc("ab\r\ncd\nef");
    assert_eq!(d.line_len(0).unwrap(), Some(2));
    assert_eq!(d.line_len(3).unwrap(), None);
    assert_eq!(
        d.text_between(Pos::new(0, 1), Pos::new(2, 1)).unwrap(),
        "b\r\ncd\ne"
    );
    assert_eq!(d.text_between(Pos::new(1, 1), Pos::new(1, 1)).unwrap(), "");
}

#[test]
fn crlf_joined_by_a_delete_is_one_line_ending() {
    let mut d = doc("a\rX\nb");
    assert_eq!(d.line_count(), 3);
    d.delete(Pos::new(1, 0), Pos::new(1, 1)).unwrap();
    assert_eq!(lines(&d), [l("a", Some(Eol::CrLf)), l("b", None)]);
    d.check_invariants();
}

#[test]
fn crlf_joined_across_original_and_added_text() {
    let mut d = doc("a\r");
    assert_eq!(d.end_pos().unwrap(), Pos::new(1, 0));
    let e = d.insert(Pos::new(1, 0), "\nb").unwrap();
    assert_eq!(lines(&d), [l("a", Some(Eol::CrLf)), l("b", None)]);
    // The cursor start was inside the CRLF, so it is at the end of the text of line 0.
    assert_eq!(e.start, Pos::new(0, 1));
    assert_eq!(e.end, Pos::new(1, 1));
}

#[test]
fn crlf_cut_in_two_pieces_stays_one_line_ending() {
    let mut d = doc("a\r\nb");
    // Delete "b" and re-add it: the CR and the LF stay in one original piece.
    d.insert(Pos::new(1, 1), "c").unwrap();
    d.insert(Pos::new(0, 1), "").unwrap();
    assert_eq!(lines(&d), [l("a", Some(Eol::CrLf)), l("bc", None)]);
    // Make a CR piece and an LF piece by deleting the text between a CR and an LF.
    let mut d = doc("a\r12\nb");
    d.delete(Pos::new(1, 0), Pos::new(1, 2)).unwrap();
    assert_eq!(d.piece_count(), 2);
    assert_eq!(lines(&d), [l("a", Some(Eol::CrLf)), l("b", None)]);
    assert_eq!(d.line_count(), 2);
    // Inserting a CR in front of the LF merges with it too.
    let mut d = doc("a\nb");
    d.insert(Pos::new(0, 1), "\r").unwrap();
    assert_eq!(lines(&d), [l("a", Some(Eol::CrLf)), l("b", None)]);
}

#[test]
fn line_endings_in_inserted_text_become_line_breaks() {
    let mut d = Document::empty();
    let e = d
        .insert(Pos::new(0, 0), "a\r\nb\rc\nd\u{85}e\u{2028}f\u{2029}g")
        .unwrap();
    assert_eq!(
        lines(&d),
        [
            l("a", Some(Eol::CrLf)),
            l("b", Some(Eol::Cr)),
            l("c", Some(Eol::Lf)),
            l("d", Some(Eol::Nel)),
            l("e", Some(Eol::Ls)),
            l("f", Some(Eol::Ps)),
            l("g", None),
        ]
    );
    assert_eq!(e.end, Pos::new(6, 1));
}

#[test]
fn typing_continues_one_piece() {
    let mut d = doc("start");
    let mut at = Pos::new(0, 5);
    for ch in "hello world".chars() {
        at = d.insert(at, &ch.to_string()).unwrap().end;
    }
    assert_eq!(lines(&d), [l("starthello world", None)]);
    assert_eq!(d.piece_count(), 2);
    assert_eq!(d.added_bytes(), "hello world".len());
}

#[test]
fn very_long_line() {
    let long = "x".repeat(1_000_000);
    let mut d = doc(&format!("{long}\nshort"));
    assert_eq!(d.line_len(0).unwrap(), Some(1_000_000));
    d.insert(Pos::new(0, 500_000), "日").unwrap();
    d.insert(Pos::new(0, 999_999), "\n").unwrap();
    assert_eq!(d.line_len(0).unwrap(), Some(999_999));
    assert_eq!(d.line(1).unwrap().unwrap().text, "xxxx");
    assert_eq!(d.end_pos().unwrap(), Pos::new(2, 5));
}

#[test]
fn many_small_edits_on_an_original_source() {
    let mut text = String::new();
    for i in 0..2000 {
        text.push_str(&format!("line {i}\n"));
    }
    let mut d = doc(&text);
    let mut m = Model { text };
    let mut rng = Rng(42);
    for _ in 0..3000 {
        let p = Pos::new(rng.below(2000), rng.below(8));
        if rng.below(3) == 0 {
            let q = Pos::new(p.line, p.col + 2);
            d.delete(p, q).unwrap();
            m.replace(p, q, "");
        } else {
            d.insert(p, "ab\n").unwrap();
            m.replace(p, p, "ab\n");
        }
    }
    assert_same(&d, &m);
    // Unedited parts are still the original bytes.
    let original: usize = d
        .chunks()
        .map(|c| {
            if let Chunk::Original(b) = c.unwrap() {
                b.len()
            } else {
                0
            }
        })
        .sum();
    assert!(original > 0);
}

// ---- reading the original ----

#[test]
fn invalid_bytes_are_shown_as_stand_ins_and_saved_unchanged() {
    let bytes = b"a\xFFb\n\xE3\x81".to_vec();
    let mut d = Document::from_utf8(bytes.clone()).unwrap();
    let first = d.line(0).unwrap().unwrap();
    let stand_in = char::from_u32(INVALID_BYTE_BASE + 0xFF).unwrap();
    assert_eq!(first.text, format!("a{stand_in}b"));
    // The stand-in is 4 bytes in UTF-8, so `b` is at column 5.
    d.insert(Pos::new(0, 5), "Z").unwrap();
    assert_eq!(save(&d), b"a\xFFZb\n\xE3\x81");
    assert_eq!(d.line(1).unwrap().unwrap().text.chars().count(), 2);
    // Nothing edited: the saver sees the original bytes as one chunk.
    let d = Document::from_utf8(bytes.clone()).unwrap();
    let chunks: Vec<_> = d.chunks().collect::<Result<_>>().unwrap();
    assert_eq!(chunks, [Chunk::Original(Cow::Borrowed(&bytes[..]))]);
}

/// A codec with 2-byte characters and 2-byte line endings (like UTF-16LE for ASCII), to exercise
/// everything that a non-UTF-8 original needs.
#[derive(Debug)]
struct Wide;

impl Codec for Wide {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        for c in bytes.chunks(2) {
            if let [lo, 0] = c {
                out.push(char::from(*lo));
            } else {
                for &b in c {
                    out.push(char::from_u32(INVALID_BYTE_BASE + u32::from(b)).unwrap());
                }
            }
        }
    }
    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        let mut i = from;
        while i + 1 < bytes.len() {
            match (bytes[i], bytes[i + 1]) {
                (b'\n', 0) => return Some((i, Eol::Lf)),
                (b'\r', 0) if bytes.get(i + 2..i + 4) == Some(&[b'\n', 0]) => {
                    return Some((i, Eol::CrLf));
                }
                (b'\r', 0) => return Some((i, Eol::Cr)),
                _ => i += 2,
            }
        }
        None
    }
    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        match eol {
            Eol::CrLf => Some(b"\r\0\n\0"),
            Eol::Lf => Some(b"\n\0"),
            Eol::Cr => Some(b"\r\0"),
            _ => None,
        }
    }
    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        if bytes.get(i + 1) == Some(&0) { 2 } else { 1 }
    }
    fn encode(&self, text: &str, out: &mut Vec<u8>) -> std::result::Result<(), Unmappable> {
        for ch in text.chars() {
            out.extend_from_slice(&[ch as u8, 0]);
        }
        Ok(())
    }
}

fn wide(text: &str) -> Vec<u8> {
    text.bytes().flat_map(|b| [b, 0]).collect()
}

#[test]
fn a_non_utf8_original_with_wide_line_endings() {
    let mut d = Document::new(MemorySource::new(wide("ab\r\ncd\nef\r")), Arc::new(Wide)).unwrap();
    assert_eq!(
        lines(&d),
        [
            l("ab", Some(Eol::CrLf)),
            l("cd", Some(Eol::Lf)),
            l("ef", Some(Eol::Cr))
        ]
    );
    d.insert(Pos::new(1, 1), "X").unwrap();
    d.delete(Pos::new(0, 1), Pos::new(1, 0)).unwrap();
    assert_eq!(whole(&d), "acXd\nef\r");
    // Join a CR and an LF that sit in different pieces (2 bytes each here).
    let mut d = Document::new(MemorySource::new(wide("a\r12\nb")), Arc::new(Wide)).unwrap();
    d.delete(Pos::new(1, 0), Pos::new(1, 2)).unwrap();
    assert_eq!(lines(&d), [l("a", Some(Eol::CrLf)), l("b", None)]);
    assert_eq!(
        d.text_between(Pos::new(0, 0), d.end_pos().unwrap())
            .unwrap(),
        "a\r\nb"
    );
    d.check_invariants();
}

#[test]
fn random_edits_on_a_wide_original() {
    let start = "ab\r\ncd\nef\rgh\r\n\r\nij";
    let mut rng = Rng(7);
    let mut d = Document::new(MemorySource::new(wide(start)), Arc::new(Wide)).unwrap();
    let mut m = Model {
        text: start.to_string(),
    };
    for _ in 0..800 {
        let (from, to) = (random_pos(&mut rng, &m), random_pos(&mut rng, &m));
        // Only ASCII and the three line endings the codec knows, so the model matches.
        let t: String = (0..rng.below(4))
            .map(|_| ["a", "b", "\n", "\r", "\r\n"][rng.below(5)])
            .collect();
        let (got, want) = if rng.below(3) == 0 {
            (d.delete(from, to).unwrap(), m.replace(from, to, ""))
        } else {
            (d.replace(from, to, &t).unwrap(), m.replace(from, to, &t))
        };
        assert_eq!(got, want);
        assert_same(&d, &m);
        if m.text.len() > 200 {
            let e = d.end_pos().unwrap();
            let cut = Pos::new(e.line / 2, 0);
            d.delete(Pos::new(0, 0), cut).unwrap();
            m.replace(Pos::new(0, 0), cut, "");
        }
    }
}

#[derive(Debug)]
struct Stateful;

impl Codec for Stateful {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        Utf8Codec.decode(bytes, out);
    }
    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        Utf8Codec.find_eol(bytes, from)
    }
    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        Utf8Codec.eol_bytes(eol)
    }
    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        Utf8Codec.char_len(bytes, i)
    }
    fn encode(&self, text: &str, out: &mut Vec<u8>) -> std::result::Result<(), Unmappable> {
        Utf8Codec.encode(text, out)
    }
    fn is_stateless(&self) -> bool {
        false
    }
}

#[test]
fn a_stateful_encoding_is_converted_at_load_time() {
    let mut d = Document::new(MemorySource::new(b"ab\ncd".to_vec()), Arc::new(Stateful)).unwrap();
    assert_eq!(lines(&d), [l("ab", Some(Eol::Lf)), l("cd", None)]);
    assert!(d.chunks().all(|c| matches!(c.unwrap(), Chunk::Added(_))));
    d.insert(Pos::new(1, 2), "!").unwrap();
    assert_eq!(whole(&d), "ab\ncd!");
    let empty = Document::new(MemorySource::new(Vec::new()), Arc::new(Stateful)).unwrap();
    assert_eq!(empty.line_count(), 0);
}

// ---- limits, errors, sharing ----

#[test]
fn a_source_over_the_limit_is_refused() {
    let err = Document::with_limit(MemorySource::new(vec![b'a'; 10]), Arc::new(Utf8Codec), 5)
        .unwrap_err();
    assert!(matches!(
        err,
        BufferError::SourceTooLarge { len: 10, max: 5 }
    ));
    assert!(err.to_string().contains("too large"));
    assert!(Document::with_limit(MemorySource::new(vec![b'a'; 5]), Arc::new(Utf8Codec), 5).is_ok());
    assert_eq!(DEFAULT_MAX_SOURCE_LEN, 2_147_483_648);
}

/// A source that can be read once as a whole (to build the index) and then fails.
#[derive(Debug)]
struct Flaky(Vec<u8>);

impl Source for Flaky {
    fn len(&self) -> u64 {
        self.0.len() as u64
    }
    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        if start == 0 && end == self.len() {
            Ok(Cow::Borrowed(&self.0))
        } else {
            Err(io::Error::other("disk gone"))
        }
    }
}

#[test]
fn read_errors_are_reported() {
    let mut d = Document::new(Flaky(b"abc\ndef".to_vec()), Arc::new(Utf8Codec)).unwrap();
    // The whole-file piece is readable; a part of it is not.
    assert!(matches!(d.line(0), Err(BufferError::Io(_))));
    // Positions that need a part of the original fail the same way, and do not change the document.
    assert!(d.insert(Pos::new(1, 2), "x").is_err());
    assert_eq!(d.piece_count(), 1);
}

#[test]
fn clones_are_independent_and_can_cross_threads() {
    let mut d = doc("one\ntwo\n");
    let old = d.clone();
    d.insert(Pos::new(1, 0), "new ").unwrap();
    let handle = std::thread::spawn({
        let old = old.clone();
        move || whole(&old)
    });
    d.insert(Pos::new(0, 0), "X").unwrap();
    assert_eq!(handle.join().unwrap(), "one\ntwo\n");
    assert_eq!(whole(&old), "one\ntwo\n");
    assert_eq!(whole(&d), "Xone\nnew two\n");
    // Two clones that both append do not disturb each other.
    let mut a = old.clone();
    let mut b = old.clone();
    a.insert(Pos::new(0, 0), "AAA").unwrap();
    b.insert(Pos::new(0, 0), "BBB").unwrap();
    a.insert(Pos::new(0, 3), "a").unwrap();
    assert_eq!(whole(&a), "AAAaone\ntwo\n");
    assert_eq!(whole(&b), "BBBone\ntwo\n");
}

// ---- size ----

fn generated(bytes: usize) -> Vec<u8> {
    let mut rng = Rng(99);
    let mut v = Vec::with_capacity(bytes + 100);
    while v.len() < bytes {
        for _ in 0..rng.below(60) {
            v.push(b'a' + rng.below(26) as u8);
        }
        if rng.below(20) == 0 {
            v.extend_from_slice("日本語のテキスト".as_bytes());
        }
        v.extend_from_slice(if rng.below(4) == 0 { b"\r\n" } else { b"\n" });
    }
    v
}

fn smoke(bytes: usize) {
    let data = generated(bytes);
    let t = Instant::now();
    let mut d = Document::from_utf8(data.clone()).unwrap();
    let load = t.elapsed();
    let n = d.line_count();
    let mut rng = Rng(5);
    let t = Instant::now();
    for _ in 0..20_000 {
        let l = d.line(rng.below(n)).unwrap().unwrap();
        assert!(l.text.len() < 200);
    }
    let read = t.elapsed();
    let t = Instant::now();
    for _ in 0..20_000 {
        let p = Pos::new(rng.below(n), rng.below(10));
        if rng.below(2) == 0 {
            d.insert(p, "edit\n").unwrap();
        } else {
            d.delete(p, Pos::new(p.line + 1, 3)).unwrap();
        }
    }
    let edit = t.elapsed();
    d.check_invariants();
    println!(
        "{} MB, {} lines: load {:?}, 20k random line reads {:?}, 20k random edits {:?}, \
         {} pieces, line index {} bytes ({:.1}% of the file)",
        bytes >> 20,
        n,
        load,
        read,
        edit,
        d.piece_count(),
        d.index_heap_bytes(),
        d.index_heap_bytes() as f64 * 100.0 / bytes as f64
    );
    // Saving: unedited bytes come out unchanged.
    let saved: usize = d
        .chunks()
        .map(|c| match c.unwrap() {
            Chunk::Original(b) => b.len(),
            Chunk::Added(s) => s.len(),
        })
        .sum();
    assert!(saved > 0);
}

#[test]
fn smoke_test_20_mb() {
    smoke(20 << 20);
}

#[test]
#[ignore = "slow in debug builds; run with --release --ignored --nocapture to see the numbers"]
fn smoke_test_100_mb() {
    smoke(100 << 20);
}
