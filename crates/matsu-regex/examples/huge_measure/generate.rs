//! The test files of ADR-0020, section 8.1: S (UTF-8, LF), C (CRLF), T (a line ending per byte),
//! L (one line), J (Shift_JIS), W (UTF-16 LE), E (EUC-JP). They are written whole, in buffers of
//! about 1 MiB: nothing is sparse, because the content (and so the scan, the decoding and the
//! search) must be real.

use std::io::{self, Write};

use matsu_core::encoding::Encoding;

/// A line that exists once, at about nine tenths of the file, for the "first hit" of a search.
pub const NEEDLE: &str = "MATSU-NEEDLE";
/// A plain text that is in the line numbers, but only a few times in a file of a few GB.
pub const COMMON_PLAIN: &str = "7777777";
/// The regular expression that finds the same places as [`COMMON_PLAIN`].
pub const COMMON_REGEX: &str = "7{7}";

const CHUNK: usize = 1024 * 1024;
const GIB: u64 = 1024 * 1024 * 1024;
const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz.,";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    S,
    C,
    T,
    L,
    J,
    W,
    E,
}

impl Kind {
    pub const ALL: [Kind; 7] = [
        Kind::S,
        Kind::C,
        Kind::T,
        Kind::L,
        Kind::J,
        Kind::W,
        Kind::E,
    ];

    /// `s`, `S10` and `c` are all accepted: the digits (the size of ADR-0020) are ignored.
    pub fn parse(text: &str) -> Option<Self> {
        let letter = text.trim().trim_end_matches(|c: char| c.is_ascii_digit());
        Self::ALL
            .into_iter()
            .find(|k| k.letter().eq_ignore_ascii_case(letter))
    }

    pub fn letter(self) -> &'static str {
        match self {
            Kind::S => "S",
            Kind::C => "C",
            Kind::T => "T",
            Kind::L => "L",
            Kind::J => "J",
            Kind::W => "W",
            Kind::E => "E",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Kind::S => "UTF-8, 40 bytes per line, LF",
            Kind::C => "UTF-8, 41 bytes per line, CRLF",
            Kind::T => "UTF-8, a line is only an LF (1 byte)",
            Kind::L => "UTF-8, one line without a line ending",
            Kind::J => "Shift_JIS, Japanese (full width), LF",
            Kind::W => "UTF-16 LE with BOM, Japanese and one surrogate pair per line, LF",
            Kind::E => "EUC-JP, Japanese (full width), LF",
        }
    }

    pub fn encoding(self) -> Encoding {
        match self {
            Kind::J => Encoding::ShiftJis,
            Kind::W => Encoding::Utf16Le,
            Kind::E => Encoding::EucJp,
            _ => Encoding::Utf8,
        }
    }

    /// The size the kind is made in: ADR-0020 makes L and E 1 GB, whatever the others are.
    pub fn size_for(self, requested: u64) -> u64 {
        match self {
            Kind::L | Kind::E => requested.min(GIB),
            _ => requested,
        }
    }

    /// Whether the file is one line (searching it decodes the whole line, so it is left out).
    pub fn is_one_line(self) -> bool {
        self == Kind::L
    }
}

/// The 10 digits of a number (it wraps after 10^10).
fn digits(mut n: u64) -> [u8; 10] {
    let mut d = [b'0'; 10];
    for b in d.iter_mut().rev() {
        *b = b'0' + (n % 10) as u8;
        n /= 10;
    }
    d
}

/// The `i`-th kana of a line, as `(lead, trail)` in the table of Shift_JIS (`euc` false) or EUC-JP:
/// 83 hiragana, then 63 katakana.
fn kana(line_no: u64, i: u64, euc: bool) -> [u8; 2] {
    let k = (line_no.wrapping_mul(7).wrapping_add(i * 3) % 146) as u8;
    match (euc, k < 83) {
        (false, true) => [0x82, 0x9F + k],
        (false, false) => [0x83, 0x40 + (k - 83)],
        (true, true) => [0xA4, 0xA1 + k],
        (true, false) => [0xA5, 0xA1 + (k - 83)],
    }
}

/// Makes the bytes of a file, chunk after chunk.
pub struct Generator {
    kind: Kind,
    size: u64,
    written: u64,
    line_no: u64,
    needle_at: u64,
    needle_done: bool,
    started: bool,
}

impl Generator {
    /// A generator of about `size` bytes (whole lines: the file is a little longer; L is exactly
    /// `size`).
    pub fn new(kind: Kind, size: u64) -> Self {
        Self {
            kind,
            size,
            written: 0,
            line_no: 0,
            needle_at: size / 10 * 9,
            needle_done: kind.is_one_line(),
            started: false,
        }
    }

    /// The bytes of a line that has `text` (ASCII) in the encoding of the kind, with its ending.
    pub fn ascii_line(kind: Kind, text: &str) -> Vec<u8> {
        let mut out = Vec::new();
        for b in text.bytes() {
            out.push(b);
            if kind == Kind::W {
                out.push(0);
            }
        }
        push_eol(kind, &mut out);
        out
    }

    /// Replaces `buf` with the next chunk. `false` when the file is complete (and `buf` is empty).
    pub fn next_chunk(&mut self, buf: &mut Vec<u8>) -> bool {
        buf.clear();
        if self.written >= self.size {
            return false;
        }
        if !self.started {
            self.started = true;
            if self.kind == Kind::W {
                buf.extend_from_slice(&[0xFF, 0xFE]);
            }
        }
        if self.kind == Kind::L {
            let want = (self.size - self.written).min(CHUNK as u64);
            for i in 0..want {
                buf.push(ALPHABET[((self.written + i) % ALPHABET.len() as u64) as usize]);
            }
        } else {
            while buf.len() < CHUNK && self.written + (buf.len() as u64) < self.size {
                if !self.needle_done && self.written + buf.len() as u64 >= self.needle_at {
                    self.needle_done = true;
                    buf.extend_from_slice(&Self::ascii_line(self.kind, NEEDLE));
                    continue;
                }
                self.push_line(buf);
            }
        }
        self.written += buf.len() as u64;
        true
    }

    fn push_line(&mut self, buf: &mut Vec<u8>) {
        let n = self.line_no;
        self.line_no += 1;
        match self.kind {
            Kind::S | Kind::C => {
                buf.extend_from_slice(&digits(n));
                buf.push(b' ');
                buf.extend_from_slice(ALPHABET);
            }
            Kind::T => {}
            Kind::J | Kind::E => {
                buf.extend_from_slice(&digits(n));
                buf.push(b' ');
                for i in 0..20 {
                    buf.extend_from_slice(&kana(n, i, self.kind == Kind::E));
                }
            }
            Kind::W => {
                for d in digits(n) {
                    buf.extend_from_slice(&[d, 0]);
                }
                buf.extend_from_slice(&[b' ', 0]);
                for i in 0..20u64 {
                    let ch = 0x3042 + (n.wrapping_mul(7).wrapping_add(i * 3) % 83) as u16;
                    buf.extend_from_slice(&ch.to_le_bytes());
                }
                // U+20BB7, a surrogate pair (D842 DFB7).
                buf.extend_from_slice(&[0x42, 0xD8, 0xB7, 0xDF]);
            }
            Kind::L => unreachable!("L is made in next_chunk"),
        }
        push_eol(self.kind, buf);
    }
}

fn push_eol(kind: Kind, buf: &mut Vec<u8>) {
    match kind {
        Kind::C => buf.extend_from_slice(b"\r\n"),
        Kind::W => buf.extend_from_slice(&[b'\n', 0]),
        Kind::L => {}
        _ => buf.push(b'\n'),
    }
}

/// Writes the whole file to `out` and returns its length.
pub fn write_file(kind: Kind, size: u64, out: &mut impl Write) -> io::Result<u64> {
    let mut generator = Generator::new(kind, size);
    let mut buf = Vec::with_capacity(CHUNK + 256);
    let mut total = 0u64;
    while generator.next_chunk(&mut buf) {
        out.write_all(&buf)?;
        total += buf.len() as u64;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use matsu_core::buffer::{Document, EncodingCodec, Source};
    use std::borrow::Cow;
    use std::sync::Arc;

    /// The bytes of a file after its BOM, as a source.
    #[derive(Debug)]
    struct Bytes(Vec<u8>, usize);

    impl Source for Bytes {
        fn len(&self) -> u64 {
            (self.0.len() - self.1) as u64
        }

        fn read(&self, start: u64, end: u64) -> std::io::Result<Cow<'_, [u8]>> {
            Ok(Cow::Borrowed(
                &self.0[self.1 + start as usize..self.1 + end as usize],
            ))
        }
    }

    fn make(kind: Kind, size: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        let n = write_file(kind, size, &mut bytes).unwrap();
        assert_eq!(n, bytes.len() as u64);
        bytes
    }

    /// Reads the bytes back through a document, as the editor does: every line must decode.
    fn lines_of(kind: Kind, bytes: Vec<u8>) -> Vec<String> {
        let bom = if kind == Kind::W { 2 } else { 0 };
        let doc =
            Document::new(Bytes(bytes, bom), Arc::new(EncodingCodec(kind.encoding()))).unwrap();
        (0..doc.line_count())
            .map(|i| doc.line(i).unwrap().unwrap())
            .map(|l| format!("{}|{:?}", l.text, l.eol))
            .collect()
    }

    #[test]
    fn kinds_parse_with_or_without_the_size() {
        assert_eq!(Kind::parse("S10"), Some(Kind::S));
        assert_eq!(Kind::parse("l1"), Some(Kind::L));
        assert_eq!(Kind::parse(" e "), Some(Kind::E));
        assert_eq!(Kind::parse("x"), None);
        assert_eq!(Kind::parse(""), None);
    }

    #[test]
    fn l_and_e_are_at_most_one_gib() {
        let gib = GIB;
        assert_eq!(Kind::L.size_for(10 * gib), gib);
        assert_eq!(Kind::E.size_for(10 * gib), gib);
        assert_eq!(Kind::S.size_for(10 * gib), 10 * gib);
        assert_eq!(Kind::L.size_for(5), 5);
    }

    #[test]
    fn s_has_lines_of_40_bytes_with_lf() {
        let bytes = make(Kind::S, 4000);
        assert!(bytes.len() >= 4000 && bytes.len() < 4000 + 40 + 40);
        let needle = Generator::ascii_line(Kind::S, NEEDLE);
        let text = std::str::from_utf8(&bytes).unwrap();
        let mut lines = 0;
        for line in text.split_inclusive('\n') {
            assert!(line.ends_with('\n') && !line.ends_with("\r\n"));
            if line.as_bytes() != needle {
                assert_eq!(line.len(), 40, "{line:?}");
            }
            lines += 1;
        }
        assert!(lines >= 100);
        assert!(text.starts_with("0000000000 abcdefghijklmnopqrstuvwxyz.,\n"));
    }

    #[test]
    fn c_ends_every_line_with_crlf() {
        let bytes = make(Kind::C, 4100);
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.ends_with("\r\n"));
        assert_eq!(text.matches('\n').count(), text.matches("\r\n").count());
        assert_eq!(text.split_inclusive("\r\n").next().unwrap().len(), 41);
    }

    #[test]
    fn t_is_line_endings_and_one_needle() {
        let bytes = make(Kind::T, 1000);
        let needle = Generator::ascii_line(Kind::T, NEEDLE);
        // The needle line counts toward the size.
        assert_eq!(bytes.len(), 1000);
        assert_eq!(
            bytes.iter().filter(|&&b| b == b'\n').count(),
            bytes.len() - NEEDLE.len()
        );
        assert!(needle.ends_with(b"\n"));
        assert_eq!(count(&bytes, NEEDLE.as_bytes()), 1);
    }

    fn count(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|w| *w == needle)
            .count()
    }

    #[test]
    fn l_is_exactly_the_size_and_has_no_line_ending() {
        // More than one chunk, so that the letters must go on across the edge.
        let size = CHUNK as u64 + 100;
        let bytes = make(Kind::L, size);
        assert_eq!(bytes.len() as u64, size);
        assert!(!bytes.contains(&b'\n') && !bytes.contains(&b'\r'));
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.starts_with("abcdefghijklmnopqrstuvwxyz.,abc"));
        let at = CHUNK - 3;
        assert_eq!(text.as_bytes()[at], ALPHABET[at % 28]);
        assert_eq!(count(&bytes, NEEDLE.as_bytes()), 0);
    }

    #[test]
    fn j_and_e_decode_as_japanese_and_whole_lines() {
        for kind in [Kind::J, Kind::E] {
            let bytes = make(kind, 3000);
            assert!(bytes.ends_with(b"\n"));
            let lines = lines_of(kind, bytes);
            assert!(lines.len() > 30);
            for l in &lines {
                if l.starts_with(NEEDLE) {
                    continue;
                }
                let text = l.split('|').next().unwrap();
                assert_eq!(text.chars().count(), 11 + 20, "{kind:?} {l}");
                assert!(
                    text.chars()
                        .skip(11)
                        .all(|c| ('\u{3041}'..='\u{30FF}').contains(&c)),
                    "{kind:?} {l}"
                );
            }
        }
    }

    #[test]
    fn w_has_a_bom_surrogate_pairs_and_lf() {
        let bytes = make(Kind::W, 3000);
        assert_eq!(&bytes[..2], &[0xFF, 0xFE]);
        assert_eq!(bytes.len() % 2, 0);
        let lines = lines_of(Kind::W, bytes);
        let line = &lines[0];
        assert!(line.starts_with("0000000000 "), "{line}");
        assert!(line.contains('\u{20BB7}'), "{line}");
        assert!(line.ends_with("|Some(Lf)"), "{line}");
        assert_eq!(lines.iter().filter(|l| l.starts_with(NEEDLE)).count(), 1);
    }

    #[test]
    fn the_needle_is_once_near_nine_tenths() {
        for kind in [Kind::S, Kind::C, Kind::T, Kind::J, Kind::E, Kind::W] {
            let bytes = make(kind, 100_000);
            let needle = Generator::ascii_line(kind, NEEDLE);
            let at = bytes
                .windows(needle.len())
                .position(|w| w == needle.as_slice())
                .unwrap_or_else(|| panic!("{kind:?}: no needle"));
            let share = at as f64 / bytes.len() as f64;
            assert!((0.89..0.92).contains(&share), "{kind:?}: {share}");
        }
    }

    #[test]
    fn the_common_text_is_in_the_numbers() {
        // 10^7 lines would be too many for a test: the numbers are checked directly.
        assert_eq!(&digits(1_777_777_712), b"1777777712");
        assert!(
            std::str::from_utf8(&digits(1_777_777_712))
                .unwrap()
                .contains(COMMON_PLAIN)
        );
        assert_eq!(&digits(5), b"0000000005");
    }
}
