//! Temporary: replaced by `crate::encoding` when the encoding PR lands.
//!
//! This is a private copy of the codec contract shared with the encoding PR
//! (names and signatures must stay exactly as they are), with a UTF-8-only
//! implementation. The buffer only talks to the [`Codec`] trait.

/// A line ending. Sakura: `EEolType` in sakura_core/basis/CEol.h.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Eol {
    CrLf,
    Lf,
    Cr,
    /// U+0085
    Nel,
    /// U+2028
    Ls,
    /// U+2029
    Ps,
}

/// The first char of the 256 stand-ins for bytes that are invalid in their encoding.
/// Byte `b` decodes to `char::from_u32(INVALID_BYTE_BASE + b as u32)`, and encoding that char gives
/// `b` back. Sakura maps them to U+DC00..=U+DCFF (`CCodeBase::BinToText`), which a Rust `String`
/// cannot hold, so we use the last 256 code points of Supplementary Private Use Area-B.
pub const INVALID_BYTE_BASE: u32 = 0x10FF00;

/// Reads and writes the bytes of one encoding. The buffer keeps the file's original bytes and asks
/// the codec to decode only the parts it shows or searches (ADR-0005).
pub trait Codec: Send + Sync + std::fmt::Debug {
    /// Decodes `bytes`, which start at a character boundary, and appends the text to `out`.
    /// Each invalid byte becomes one stand-in char (see INVALID_BYTE_BASE). A truncated character
    /// at the end of `bytes` is treated as invalid bytes.
    fn decode(&self, bytes: &[u8], out: &mut String);

    /// Finds the first line ending that starts at or after `from` (a character boundary). Returns
    /// the offset of its first byte and its kind. CR followed by LF is one `CrLf`. A CR as the last
    /// unit of `bytes` is reported as `Cr`; the caller joins it with an LF at the start of the next
    /// chunk. Which kinds an encoding recognizes follows Sakura (`sakura_core/basis/CEol.cpp`).
    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)>;

    /// The bytes of a line ending in this encoding, or None if the encoding cannot write it.
    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]>;

    /// The length in bytes of the character that starts at `bytes[i]` (1 for an invalid byte).
    fn char_len(&self, bytes: &[u8], i: usize) -> usize;

    /// Encodes `text` and appends the bytes to `out`. Stand-ins become their byte again.
    /// Characters this encoding cannot represent are written as Sakura writes them and reported in
    /// the error (their byte offsets in `text`), so the caller can warn and still save.
    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable>;

    /// False for encodings whose meaning depends on earlier bytes (ISO-2022-JP, UTF-7). The buffer
    /// cannot decode a range of those from the middle, so it converts the whole file at load time.
    fn is_stateless(&self) -> bool {
        true
    }
}

/// Characters that could not be encoded: byte offsets into the encoded `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unmappable {
    pub offsets: Vec<usize>,
}

/// UTF-8 with all six line ending kinds. The add buffer of the document is always scanned with
/// this codec, because typed text is UTF-8 whatever the file's encoding is.
#[derive(Clone, Copy, Debug, Default)]
pub struct Utf8Codec;

impl Codec for Utf8Codec {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        for chunk in bytes.utf8_chunks() {
            out.push_str(chunk.valid());
            for &b in chunk.invalid() {
                out.push(
                    char::from_u32(INVALID_BYTE_BASE + u32::from(b))
                        .unwrap_or(char::REPLACEMENT_CHARACTER),
                );
            }
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        let mut i = from;
        while i < bytes.len() {
            // Only these four bytes can start a line ending in UTF-8.
            let step = bytes[i..]
                .iter()
                .position(|&b| matches!(b, b'\n' | b'\r' | 0xC2 | 0xE2))?;
            i += step;
            match bytes[i] {
                b'\n' => return Some((i, Eol::Lf)),
                b'\r' if bytes.get(i + 1) == Some(&b'\n') => return Some((i, Eol::CrLf)),
                b'\r' => return Some((i, Eol::Cr)),
                0xC2 if bytes.get(i + 1) == Some(&0x85) => return Some((i, Eol::Nel)),
                0xE2 if bytes.get(i + 1) == Some(&0x80) => match bytes.get(i + 2) {
                    Some(0xA8) => return Some((i, Eol::Ls)),
                    Some(0xA9) => return Some((i, Eol::Ps)),
                    _ => {}
                },
                _ => {}
            }
            i += 1;
        }
        None
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        Some(match eol {
            Eol::CrLf => b"\r\n",
            Eol::Lf => b"\n",
            Eol::Cr => b"\r",
            Eol::Nel => "\u{85}".as_bytes(),
            Eol::Ls => "\u{2028}".as_bytes(),
            Eol::Ps => "\u{2029}".as_bytes(),
        })
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        let want = match bytes.get(i) {
            Some(0xC2..=0xDF) => 2,
            Some(0xE0..=0xEF) => 3,
            Some(0xF0..=0xF4) => 4,
            _ => return 1,
        };
        match bytes.get(i..i + want) {
            Some(s) if std::str::from_utf8(s).is_ok() => want,
            _ => 1,
        }
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        let mut utf8 = [0u8; 4];
        for ch in text.chars() {
            let c = u32::from(ch);
            if c >= INVALID_BYTE_BASE {
                // The range ends at U+10FFFF, so the difference fits in a byte.
                out.push((c - INVALID_BYTE_BASE) as u8);
            } else {
                out.extend_from_slice(ch.encode_utf8(&mut utf8).as_bytes());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_eol_kinds() {
        let c = Utf8Codec;
        assert_eq!(c.find_eol(b"ab\r\ncd", 0), Some((2, Eol::CrLf)));
        assert_eq!(c.find_eol(b"ab\rcd", 0), Some((2, Eol::Cr)));
        assert_eq!(c.find_eol(b"ab\r", 0), Some((2, Eol::Cr)));
        assert_eq!(c.find_eol(b"ab\ncd", 3), None);
        assert_eq!(c.find_eol("a\u{85}".as_bytes(), 0), Some((1, Eol::Nel)));
        assert_eq!(c.find_eol("a\u{2028}".as_bytes(), 0), Some((1, Eol::Ls)));
        assert_eq!(c.find_eol("a\u{2029}".as_bytes(), 0), Some((1, Eol::Ps)));
        // U+2027 and U+00C0 share lead bytes with line endings but are not line endings.
        assert_eq!(c.find_eol("\u{2027}\u{C0}".as_bytes(), 0), None);
        assert_eq!(c.find_eol(b"abc", 99), None);
    }

    #[test]
    fn invalid_bytes_round_trip() {
        let c = Utf8Codec;
        let bytes = [b'a', 0xFF, 0xE3, 0x81, b'b'];
        let mut s = String::new();
        c.decode(&bytes, &mut s);
        assert_eq!(s.chars().count(), 5);
        let mut back = Vec::new();
        c.encode(&s, &mut back).unwrap();
        assert_eq!(back, bytes);
    }

    #[test]
    fn char_len_matches_decode() {
        let c = Utf8Codec;
        let bytes = "aé日😀".as_bytes();
        let mut i = 0;
        let mut lens = Vec::new();
        while i < bytes.len() {
            let l = c.char_len(bytes, i);
            lens.push(l);
            i += l;
        }
        assert_eq!(lens, [1, 2, 3, 4]);
        assert_eq!(c.char_len(&[0xE3, 0x81], 0), 1);
        assert_eq!(c.char_len(&[], 0), 1);
    }
}
