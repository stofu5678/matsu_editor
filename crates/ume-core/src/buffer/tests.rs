//! Tests of the document: edge cases, a model-based random test, and a smoke test.

use std::borrow::Cow;
use std::io;
use std::sync::Arc;
use std::time::Instant;

use super::storage::LineIndex;
use super::*;
use crate::encoding::Encoding;

const UTF8: EncodingCodec = EncodingCodec(Encoding::Utf8);

// ---- helpers ----

/// A document where NEL, LS and PS end a line (`m_bEnableExtEol` on).
fn doc(text: &str) -> Document {
    Document::from_utf8_with(text.as_bytes().to_vec(), true).unwrap()
}

/// A document with Sakura's default: NEL, LS and PS are ordinary characters.
fn doc_plain(text: &str) -> Document {
    Document::from_utf8_with(text.as_bytes().to_vec(), false).unwrap()
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
            Chunk::Added(s) => UTF8.encode(&s, &mut out).unwrap(),
        }
    }
    out
}

// ---- the model: the whole text in a String ----

#[derive(Clone, Debug)]
struct Model {
    text: String,
    /// NEL, LS and PS end a line.
    ext: bool,
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
                '\u{85}' if self.ext => (Eol::Nel, 2),
                '\u{2028}' if self.ext => (Eol::Ls, 3),
                '\u{2029}' if self.ext => (Eol::Ps, 3),
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
    run_random_with(seed, start, steps, true);
}

fn run_random_with(seed: u64, start: &str, steps: usize, ext: bool) {
    let mut rng = Rng(seed);
    let mut d = if ext { doc(start) } else { doc_plain(start) };
    let mut m = Model {
        text: start.to_string(),
        ext,
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
fn random_edits_without_the_extended_line_endings() {
    let start = "first\r\nsecond 日本\u{85}third\u{2028}x\u{2029}\n\rlast";
    for seed in 1..=8_u64 {
        run_random_with(seed.wrapping_mul(0xC2B2_AE3D_27D4_EB4F), start, 600, false);
        run_random_with(seed.wrapping_mul(0x1656_67B1_9E37_79F9), "", 600, false);
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
    let mut d = Document::from_utf8_with(Vec::new(), true).unwrap();
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
    let mut m = Model { text, ext: true };
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
        ext: true,
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
        UTF8.decode(bytes, out);
    }
    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        UTF8.find_eol(bytes, from)
    }
    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        UTF8.eol_bytes(eol)
    }
    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        UTF8.char_len(bytes, i)
    }
    fn encode(&self, text: &str, out: &mut Vec<u8>) -> std::result::Result<(), Unmappable> {
        UTF8.encode(text, out)
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
    let err =
        Document::with_limit(MemorySource::new(vec![b'a'; 10]), Arc::new(UTF8), 5).unwrap_err();
    assert!(matches!(
        err,
        BufferError::SourceTooLarge { len: 10, max: 5 }
    ));
    assert!(err.to_string().contains("too large"));
    assert!(Document::with_limit(MemorySource::new(vec![b'a'; 5]), Arc::new(UTF8), 5).is_ok());
}

// ---- offsets above 4 GiB ----

#[cfg(target_pointer_width = "64")]
const GIB: u64 = 1 << 30;
/// A "unit" of the synthetic source below: 8 bytes of text and an LF.
#[cfg(target_pointer_width = "64")]
const UNIT: u64 = 9;

/// A source of 10 GiB that makes its bytes on demand, so no test allocates gigabytes. Outside the
/// dense regions it is one run of `x` without line endings, so the line index has few entries.
/// In a dense region, unit `n` is `<n mod 10>xxxxxxx\n`.
#[cfg(target_pointer_width = "64")]
#[derive(Debug)]
struct HugeSource {
    /// `(start, number of units)`.
    regions: Vec<(u64, u64)>,
}

#[cfg(target_pointer_width = "64")]
impl HugeSource {
    fn byte(&self, pos: u64) -> u8 {
        for &(start, units) in &self.regions {
            if (start..start + units * UNIT).contains(&pos) {
                let (n, k) = ((pos - start) / UNIT, (pos - start) % UNIT);
                return match k {
                    0 => b'0' + (n % 10) as u8,
                    8 => b'\n',
                    _ => b'x',
                };
            }
        }
        b'x'
    }
}

#[cfg(target_pointer_width = "64")]
impl Source for HugeSource {
    fn len(&self) -> u64 {
        10 * GIB
    }

    fn read(&self, start: u64, end: u64) -> io::Result<Cow<'_, [u8]>> {
        assert!(end - start < 1 << 20, "a test read a huge range");
        Ok(Cow::Owned((start..end).map(|p| self.byte(p)).collect()))
    }
}

/// A document over [`HugeSource`]. Region 0 has units 0..100 and the 4 GiB mark falls on the start
/// of unit 25; region 1 has units 0..100 and the 8 GiB mark falls inside unit 0. Line `i` for
/// `1 <= i <= 99` is unit `i` of region 0, and line `100 + m` for `1 <= m <= 99` is unit `m` of
/// region 1. Lines 0, 100 and 200 are the long runs and are never read.
#[cfg(target_pointer_width = "64")]
fn huge_doc() -> Document {
    let source = HugeSource {
        regions: vec![(4 * GIB - 25 * UNIT, 100), (8 * GIB - 7, 100)],
    };
    let mut index = LineIndex::default();
    for &(start, units) in &source.regions {
        let bytes = source.read(start, start + units * UNIT).unwrap();
        index.scan(&UTF8, &bytes, start as usize, true).unwrap();
    }
    Document::with_prebuilt_index(source, Arc::new(UTF8), index)
}

#[cfg(target_pointer_width = "64")]
fn unit_text(unit: usize) -> String {
    format!("{}xxxxxxx", unit % 10)
}

#[cfg(target_pointer_width = "64")]
#[test]
fn lines_above_4_gib_are_found_and_edited() {
    let mut d = huge_doc();
    assert_eq!(d.line_count(), 201);
    for i in [1, 24, 25, 26, 99, 101, 102, 150, 199] {
        let line = d.line(i).unwrap().unwrap();
        assert_eq!(line.text, unit_text(i % 100), "line {i}");
        assert_eq!(line.eol, Some(Eol::Lf), "line {i}");
    }
    // Edit at the 4 GiB mark and read back.
    d.insert(Pos::new(25, 3), "ABC\nDEF").unwrap();
    assert_eq!(d.line(25).unwrap().unwrap().text, "5xxABC");
    assert_eq!(d.line(26).unwrap().unwrap().text, "DEFxxxxx");
    assert_eq!(d.line(27).unwrap().unwrap().text, unit_text(26));
    assert_eq!(d.line_count(), 202);
    d.delete(Pos::new(25, 3), Pos::new(26, 3)).unwrap();
    assert_eq!(d.line(25).unwrap().unwrap().text, unit_text(25));
    assert_eq!(d.line_count(), 201);
    // Delete across the 8 GiB mark's region.
    d.delete(Pos::new(150, 0), Pos::new(152, 0)).unwrap();
    assert_eq!(d.line_count(), 199);
    assert_eq!(d.line(150).unwrap().unwrap().text, unit_text(52));
    d.check_invariants();
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
    let mut d = Document::new(Flaky(b"abc\ndef".to_vec()), Arc::new(UTF8)).unwrap();
    // The whole-file piece is readable; a part of it is not.
    assert!(matches!(d.line(0), Err(BufferError::Io(_))));
    // Positions that need a part of the original fail the same way, and do not change the document.
    assert!(d.insert(Pos::new(1, 2), "x").is_err());
    assert_eq!(d.piece_count(), 1);
    // The positions after the edit need a read that fails: the edit is not applied.
    let mut d = Document::new(Flaky(b"abc".to_vec()), Arc::new(UTF8)).unwrap();
    assert!(d.insert(Pos::new(0, 1), "x").is_err());
    assert_eq!(d.piece_count(), 1);
    assert_eq!(d.line_count(), 1);
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

// ---- steps that decode to more than one char ----

/// `char_len` can return 2 for a pair that decodes to two stand-ins (the NEC-selected IBM
/// extension 0xED40 in Shift_JIS, a lone surrogate in UTF-16; see `encoding/mod.rs`). A column
/// between those two chars cannot be a raw offset, so it is rounded down to the start of the pair.
#[test]
fn a_column_between_the_chars_of_one_step_rounds_down() {
    let bytes = [b'a', 0xED, 0x40, b'b', b'\n', b'c'];
    let mut text = String::new();
    EncodingCodec(Encoding::ShiftJis).decode(&bytes[..4], &mut text);
    let pair = stand_in_pair(&text);
    // The test is only meaningful if the pair really is two chars of 4 bytes each.
    assert_eq!(pair.chars().count(), 2, "{text:?}");
    assert_eq!(text.len(), 1 + 4 + 4 + 1);

    let d = Document::new(
        MemorySource::new(bytes.to_vec()),
        Arc::new(EncodingCodec(Encoding::ShiftJis)),
    )
    .unwrap();
    assert_eq!(d.line_len(0).unwrap(), Some(10));
    // Columns 1 (before the pair), 5 (between), 9 (after).
    assert_eq!(d.clamp_pos(Pos::new(0, 1)).unwrap(), Pos::new(0, 1));
    assert_eq!(d.clamp_pos(Pos::new(0, 5)).unwrap(), Pos::new(0, 1));
    assert_eq!(d.clamp_pos(Pos::new(0, 3)).unwrap(), Pos::new(0, 1));
    assert_eq!(d.clamp_pos(Pos::new(0, 9)).unwrap(), Pos::new(0, 9));
    assert_eq!(d.text_between(Pos::new(0, 0), Pos::new(0, 5)).unwrap(), "a");
    assert_eq!(
        d.text_between(Pos::new(0, 1), Pos::new(0, 9)).unwrap(),
        pair
    );

    // An edit at the middle column lands before the pair and keeps every byte.
    let mut d = d;
    let edit = d.insert(Pos::new(0, 5), "X").unwrap();
    assert_eq!(edit.start, Pos::new(0, 1));
    assert_eq!(edit.end, Pos::new(0, 2));
    assert_eq!(save_with(&d, Encoding::ShiftJis), b"aX\xED\x40b\nc");
    // Deleting up to the middle column deletes nothing of the pair.
    let mut d2 = d.clone();
    d2.delete(Pos::new(0, 0), Pos::new(0, 6)).unwrap();
    assert_eq!(save_with(&d2, Encoding::ShiftJis), b"\xED\x40b\nc");
    d.check_invariants();
}

fn stand_in_pair(text: &str) -> String {
    text.chars()
        .filter(|&c| c as u32 >= INVALID_BYTE_BASE)
        .collect()
}

/// A lone surrogate in UTF-16 LE is one step of 2 bytes that decodes to 2 stand-ins.
#[test]
fn a_column_inside_a_lone_surrogate_rounds_down() {
    let bytes = [b'a', 0, 0x00, 0xD8, b'b', 0];
    let mut d = Document::new(
        MemorySource::new(bytes.to_vec()),
        Arc::new(EncodingCodec(Encoding::Utf16Le)),
    )
    .unwrap();
    assert_eq!(d.line_len(0).unwrap(), Some(1 + 8 + 1));
    assert_eq!(d.clamp_pos(Pos::new(0, 5)).unwrap(), Pos::new(0, 1));
    assert_eq!(d.clamp_pos(Pos::new(0, 9)).unwrap(), Pos::new(0, 9));
    d.insert(Pos::new(0, 5), "X").unwrap();
    // The lone surrogate is still one unit in the original bytes.
    let originals: Vec<u8> = d
        .chunks()
        .filter_map(|c| match c.unwrap() {
            Chunk::Original(b) => Some(b.into_owned()),
            Chunk::Added(_) => None,
        })
        .flatten()
        .collect();
    assert_eq!(originals, bytes);
    assert_eq!(d.piece_count(), 3);
}

/// What a saver writes for a document read as `encoding`.
fn save_with(d: &Document, encoding: Encoding) -> Vec<u8> {
    let mut out = Vec::new();
    for c in d.chunks() {
        match c.unwrap() {
            Chunk::Original(b) => out.extend_from_slice(&b),
            Chunk::Added(s) => encoding.codec().encode(&s, &mut out).unwrap(),
        }
    }
    out
}

// ---- NEL, LS and PS (`m_bEnableExtEol`) ----

#[test]
fn nel_ls_and_ps_are_characters_by_default() {
    // Sakura: `bEnableExtEol` is off by default (`CShareData::InitShareData`).
    let d = Document::from_utf8("a\u{85}b\u{2028}c\u{2029}d\ne".as_bytes().to_vec()).unwrap();
    assert!(!d.ext_eol());
    assert_eq!(
        lines(&d),
        [l("a\u{85}b\u{2028}c\u{2029}d", Some(Eol::Lf)), l("e", None)]
    );
    assert_eq!(d.line_count(), 2);
    assert_eq!(
        d.line_len(0).unwrap(),
        Some("a\u{85}b\u{2028}c\u{2029}d".len())
    );
    // Typed or pasted text is the same: no line break.
    let mut d = Document::empty();
    d.insert(Pos::new(0, 0), "x\u{85}y\u{2028}z\u{2029}w")
        .unwrap();
    assert_eq!(d.line_count(), 1);
    assert_eq!(whole(&d), "x\u{85}y\u{2028}z\u{2029}w");
    assert_eq!(
        d.end_pos().unwrap(),
        Pos::new(0, "x\u{85}y\u{2028}z\u{2029}w".len())
    );
}

#[test]
fn nel_ls_and_ps_end_a_line_when_the_setting_is_on() {
    let d = doc("a\u{85}b\u{2028}c\u{2029}d\ne");
    assert_eq!(
        lines(&d),
        [
            l("a", Some(Eol::Nel)),
            l("b", Some(Eol::Ls)),
            l("c", Some(Eol::Ps)),
            l("d", Some(Eol::Lf)),
            l("e", None),
        ]
    );
}

#[test]
fn a_mixed_file_splits_only_at_the_endings_that_count() {
    let text = "1\r\n2\u{85}3\r4\u{2029}5\n6";
    let on = doc(text);
    let off = doc_plain(text);
    assert_eq!(on.line_count(), 6);
    assert_eq!(
        lines(&off),
        [
            l("1", Some(Eol::CrLf)),
            l("2\u{85}3", Some(Eol::Cr)),
            l("4\u{2029}5", Some(Eol::Lf)),
            l("6", None),
        ]
    );
}

#[test]
fn changing_the_setting_reaches_only_the_text_added_afterwards() {
    // `CDocLine::SetDocLineString` reads the setting where a line is made: existing lines are not
    // split or joined again when it changes.
    let mut d = doc_plain("a\u{85}b");
    d.set_ext_eol(true);
    assert_eq!(d.line_count(), 1);
    d.insert(Pos::new(0, 0), "x\u{2028}y").unwrap();
    assert_eq!(lines(&d), [l("x", Some(Eol::Ls)), l("ya\u{85}b", None)]);
    let mut e = doc("a\u{85}b");
    e.set_ext_eol(false);
    assert_eq!(e.line_count(), 2);
    e.insert(Pos::new(1, 0), "p\u{85}q").unwrap();
    assert_eq!(lines(&e), [l("a", Some(Eol::Nel)), l("p\u{85}qb", None)]);
    // An older version (undo) keeps what it had.
    let before = d.clone();
    d.insert(Pos::new(0, 0), "\u{85}").unwrap();
    assert_eq!(lines(&before).len(), 2);
}

#[test]
fn a_stateful_codec_converted_at_load_follows_the_setting() {
    let bytes = b"a\xC2\x85b".to_vec();
    let off = Document::new(MemorySource::new(bytes.clone()), Arc::new(Stateful)).unwrap();
    assert_eq!(lines(&off), [l("a\u{85}b", None)]);
    let on = Document::with_ext_eol(MemorySource::new(bytes), Arc::new(Stateful), true).unwrap();
    assert_eq!(lines(&on), [l("a", Some(Eol::Nel)), l("b", None)]);
}

#[test]
fn find_eol_skips_the_extended_endings_when_off() {
    let bytes = b"a\xC2\x85b\xE2\x80\xA8c\r\nd";
    let utf8: &dyn Codec = UTF8.0.codec();
    assert_eq!(codec::find_eol(utf8, bytes, 0, true), Some((1, Eol::Nel)));
    assert_eq!(codec::find_eol(utf8, bytes, 0, false), Some((8, Eol::CrLf)));
    assert_eq!(codec::find_eol(utf8, b"a\xC2\x85b", 0, false), None);
}
