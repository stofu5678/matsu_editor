//! The Windows code page 20932 ("Japanese EUC": JIS X 0208 and JIS X 0212) as
//! `WideCharToMultiByte(20932, ...)` and `MultiByteToWideChar(20932, ...)` give it.
//!
//! Sakura converts a code page number that Windows accepts with the Windows API
//! (`CCodePage::UniToCP`, `CCodePage::CPToUni`, `sakura_core/charset/CCodePage.cpp`; the flag
//! `WC_SEPCHARS` is refused for this code page and `WideCharToMultiByte2` tries again with 0).
//! MATSU Editor does not call Windows, so the table is built here, once, from what it may use:
//! the JIS X 0208 and JIS X 0212 indexes of `encoding_rs` (reached through its EUC-JP decoder), the
//! CP932 table that `cp932.rs` already has, and the rules and the short list of differences below.
//! Every rule and every difference was measured on Windows (the coordinator ran
//! `WideCharToMultiByte` and `MultiByteToWideChar` over all of U+0000..=U+FFFF and every two-byte
//! sequence, 2026-11-27); no Windows table is copied.
//!
//! # What the code page is
//!
//! - ASCII as it is. The bytes `0x80..=0x9F` except `0x8E` are the C1 controls U+0080..=U+009F
//!   when read (they are not written: U+0080..=U+009F become `?`).
//! - `0x8E` and a half-width katakana byte `0xA1..=0xDF`: U+FF61..=U+FF9F.
//! - Two bytes, lead `0xA1..=0xFE`, and a trail in one of two ranges. **A trail with the top bit
//!   (`0xA1..=0xFE`) is the JIS X 0208 plane. A trail without it (`0x21..=0x7E`) is the JIS X 0212
//!   plane**; there is no `0x8F` prefix. The same lead byte means the same row in both planes.
//! - Every character has exactly one code (there are no duplicates), so writing is the inverse of
//!   reading.
//! - No best fit: what the table does not have is `?`, and U+10000 and above (two UTF-16 units)
//!   are `??`.
//!
//! # The planes
//!
//! JIS X 0208 plane, leads `0xA1..=0xF4`: the `encoding_rs` index (the same characters as CP932
//! has), with these differences, because Windows 20932 is JIS-style where CP932 is MS-style
//! ([`JIS_STYLE`]): `0xA1C1` is U+301C (not U+FF5E), `0xA1C2` U+2016 (not U+2225), `0xA1DD` U+2212
//! (not U+FF0D), `0xA1F1` U+00A2 (not U+FFE0), `0xA1F2` U+00A3 (not U+FFE1), `0xA2CC` U+00AC (not
//! U+FFE2). The MS-style characters are `?`. NEC row 13 (lead `0xAD`) is the CP932 one, with one
//! difference, `0xADE0` ([`NEC_ROW_13_DIFFERENCE`]).
//!
//! JIS X 0212 plane, leads `0xA1..=0xF2`: the `encoding_rs` index of JIS X 0212, without the
//! characters that NEC row 13 has (№ and the like: they have only their row 13 code), and without
//! U+FF5E (`0xA237`).
//!
//! IBM extensions, in the JIS X 0212 plane, rows `0xF3` and `0xF4`: the IBM extensions of CP932
//! (`0xFA40..=0xFC4B`, 388 cells) that have no place elsewhere. The cells are taken in CP932 order
//! and each one takes the next slot (`0xF3 0x21`, `0xF3 0x22`, ... 94 per row), with these rules
//! ([`place_ibm_extensions`]):
//!
//! - a character that is in the JIS X 0208 or JIS X 0212 plane takes no slot (the IBM kanji that
//!   JIS X 0212 has, and U+2235 and the like);
//! - a character that is in NEC row 13 (the Roman numerals Ⅰ to Ⅹ, ㈱, №, ℡) leaves its slot empty;
//! - U+FFE2 and U+FFE4 take no slot and are not in the code page;
//! - two differences: U+663B takes a slot and leaves it empty, and U+974D is in slot 160
//!   (`0xF4 0x63`) instead of the slot its position gives.
//!
//! Leads `0xF5..=0xFE`, in both planes, are the user-defined area: U+E000.. in order, the
//! JIS X 0208 plane first (`0xF5A1` is U+E000, `0xF9A1` is U+E178, `0xFEFE` is U+E3AB), then the
//! JIS X 0212 plane (`0xF521` is U+E3AC, `0xFE7E` is U+E757).
//!
//! # Reading what is not in the table
//!
//! A byte pair that has no character, a lone `0x8E`, `0xA0`, `0xFF` and a lead byte at the end are
//! invalid bytes: each invalid byte is one stand-in (`INVALID_BYTE_BASE`) and the next byte starts
//! a new character (so a trail byte can be ASCII). Windows would give its default character.
//!
//! # Contract
//!
//! `0x0A` and `0x0D` are never a trail byte (trail bytes start at `0x21`), so line endings are
//! found by scanning for the bytes. `char_len` and `decode` use the same rule, so decoding a range
//! gives the same text as decoding the whole.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use super::cp932;
use super::{Codec, Eol, Unmappable, ascii_eol_bytes, find_cr_lf, stand_in, stand_in_byte};

/// Lead bytes `0xA1..=0xFE`.
const LEADS: usize = 94;
/// Trail bytes of one lead: 94 of the JIS X 0212 plane, then 94 of the JIS X 0208 plane.
const TRAILS: usize = 188;

/// Where the JIS X 0208 plane is not the `encoding_rs` index: (lead, trail, the Windows
/// character). Windows 20932 is JIS-style where CP932 and the WHATWG index are MS-style.
const JIS_STYLE: [(u8, u8, char); 6] = [
    (0xA1, 0xC1, '\u{301C}'),
    (0xA1, 0xC2, '\u{2016}'),
    (0xA1, 0xDD, '\u{2212}'),
    (0xA1, 0xF1, '\u{00A2}'),
    (0xA1, 0xF2, '\u{00A3}'),
    (0xA2, 0xCC, '\u{00AC}'),
];

/// The one place in NEC row 13 where Windows 20932 differs from CP932.
const NEC_ROW_13_DIFFERENCE: (u8, u8, char) = (0xAD, 0xE0, '\u{301E}');

/// The IBM extension that takes a slot and leaves it empty.
const IBM_EMPTY_SLOT: char = '\u{663B}';
/// The IBM extension that is not in the slot its position gives, and its slot.
const IBM_MOVED: (char, usize) = ('\u{974D}', 160);
/// The IBM extensions that are not in the code page and take no slot.
const IBM_NO_SLOT: [char; 2] = ['\u{FFE2}', '\u{FFE4}'];

/// The code page 20932, both ways. Built once.
struct Table {
    /// `forward[(lead - 0xA1) * TRAILS + column]`, see [`column`].
    forward: Vec<Option<char>>,
    /// The two bytes of a character: the inverse of `forward`.
    reverse: HashMap<char, [u8; 2]>,
}

/// The position of a trail byte in the row of a lead: `0x21..=0x7E` (JIS X 0212 plane) are the
/// first 94, `0xA1..=0xFE` (JIS X 0208 plane) the next 94. None for any other byte.
fn column(trail: u8) -> Option<usize> {
    match trail {
        0x21..=0x7E => Some(usize::from(trail - 0x21)),
        0xA1..=0xFE => Some(94 + usize::from(trail - 0xA1)),
        _ => None,
    }
}

fn index(lead: u8, trail: u8) -> Option<usize> {
    if !(0xA1..=0xFE).contains(&lead) {
        return None;
    }
    Some(usize::from(lead - 0xA1) * TRAILS + column(trail)?)
}

fn table() -> &'static Table {
    static TABLE: OnceLock<Table> = OnceLock::new();
    TABLE.get_or_init(build_table)
}

fn build_table() -> Table {
    let mut forward = vec![None; LEADS * TRAILS];
    let mut put = |lead: u8, trail: u8, c: char| {
        let at = index(lead, trail).expect("a position inside the table");
        forward[at] = Some(c);
    };
    // The JIS X 0208 plane (rows 1 to 84), and the JIS X 0212 plane (rows 2 to 82, EUC 0xA1..).
    for lead in 0xA1..=0xF4u8 {
        for trail in 0xA1..=0xFEu8 {
            if let Some(c) = euc_char(&[lead, trail]) {
                put(lead, trail, c);
            }
        }
    }
    for lead in 0xA1..=0xF2u8 {
        for trail in 0xA1..=0xFEu8 {
            if let Some(c) = euc_char(&[0x8F, lead, trail]) {
                put(lead, trail & 0x7F, c);
            }
        }
    }
    for (lead, trail, c) in JIS_STYLE.into_iter().chain([NEC_ROW_13_DIFFERENCE]) {
        put(lead, trail, c);
    }
    settle_row_13(&mut forward);
    // U+FF5E is in the `encoding_rs` index of JIS X 0212 (`0xA237`), and Windows has no code for it.
    forward[index(0xA2, 0x37).expect("inside the table")] = None;
    place_ibm_extensions(&mut forward);
    // The user-defined area: the JIS X 0208 plane, then the JIS X 0212 plane.
    let mut private = 0xE000u32;
    for plane in [0xA1u8..=0xFE, 0x21..=0x7E] {
        for lead in 0xF5..=0xFEu8 {
            for trail in plane.clone() {
                let c = char::from_u32(private).expect("the private use area");
                forward[index(lead, trail).expect("inside the table")] = Some(c);
                private += 1;
            }
        }
    }
    let mut reverse = HashMap::new();
    for lead in 0xA1..=0xFEu8 {
        for trail in (0x21..=0x7Eu8).chain(0xA1..=0xFE) {
            if let Some(c) = forward[index(lead, trail).expect("inside the table")] {
                reverse.entry(c).or_insert([lead, trail]);
            }
        }
    }
    Table { forward, reverse }
}

/// The one character that `encoding_rs`'s EUC-JP decoder makes of `bytes`, if any.
fn euc_char(bytes: &[u8]) -> Option<char> {
    let text = encoding_rs::EUC_JP.decode_without_bom_handling_and_without_replacement(bytes)?;
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    }
}

/// Settles the characters that NEC row 13 (lead `0xAD`) shares with the other planes, so that a
/// character has one code: a character that JIS X 0208 has in another row (≒, ≡, ∫, √, ⊥, ∠, ∵,
/// ∩, ∪) is kept there and leaves row 13; a character that JIS X 0212 has (№ at `0xA271`, ™ and
/// the like) is kept in row 13 and leaves the JIS X 0212 plane.
fn settle_row_13(forward: &mut [Option<char>]) {
    let in_0208: HashSet<char> = (0xA1..=0xF4u8)
        .filter(|&lead| lead != 0xAD)
        .flat_map(|lead| (0xA1..=0xFEu8).map(move |trail| (lead, trail)))
        .filter_map(|(lead, trail)| forward[index(lead, trail).expect("inside the table")])
        .collect();
    for trail in 0xA1..=0xFEu8 {
        let at = index(0xAD, trail).expect("inside the table");
        if forward[at].is_some_and(|c| in_0208.contains(&c)) {
            forward[at] = None;
        }
    }
    let row_13: HashSet<char> = (0xA1..=0xFEu8)
        .filter_map(|trail| forward[index(0xAD, trail).expect("inside the table")])
        .collect();
    for lead in 0xA1..=0xF2u8 {
        for trail in 0x21..=0x7Eu8 {
            let at = index(lead, trail).expect("inside the table");
            if forward[at].is_some_and(|c| row_13.contains(&c)) {
                forward[at] = None;
            }
        }
    }
}

/// Puts the IBM extensions of CP932 into rows `0xF3` and `0xF4` of the JIS X 0212 plane, by the
/// rules in the module documentation. Called when the rest of rows below `0xF3` is filled.
fn place_ibm_extensions(forward: &mut [Option<char>]) {
    let mut elsewhere = HashSet::new();
    let mut nec_row_13 = HashSet::new();
    for lead in 0xA1..=0xF2u8 {
        for trail in (0x21..=0x7Eu8).chain(0xA1..=0xFE) {
            let Some(c) = forward[index(lead, trail).expect("inside the table")] else {
                continue;
            };
            if lead == 0xAD && trail >= 0xA1 {
                nec_row_13.insert(c);
            } else {
                elsewhere.insert(c);
            }
        }
    }
    let mut slot = 0;
    for lead in 0xFA..=0xFCu8 {
        for trail in (0x40..=0xFCu8).filter(|&t| t != 0x7F) {
            if lead == 0xFC && trail > 0x4B {
                break;
            }
            let Some(c) = cp932::raw_decode(lead, trail) else {
                continue;
            };
            if IBM_NO_SLOT.contains(&c) || elsewhere.contains(&c) {
                continue;
            }
            let this = slot;
            slot += 1;
            if nec_row_13.contains(&c) || c == IBM_EMPTY_SLOT {
                continue;
            }
            let at = if c == IBM_MOVED.0 { IBM_MOVED.1 } else { this };
            let (lead, trail) = (0xF3 + (at / 94) as u8, 0x21 + (at % 94) as u8);
            forward[index(lead, trail).expect("inside the table")] = Some(c);
        }
    }
}

/// What starts at `bytes[i]`.
enum Unit {
    /// One byte that is the character: ASCII and the C1 controls `0x80..=0x9F` but `0x8E`.
    Single(char),
    /// Two bytes that make a character.
    Pair(char),
    /// An invalid byte.
    Invalid,
}

fn unit(bytes: &[u8], i: usize) -> Unit {
    let Some(&lead) = bytes.get(i) else {
        return Unit::Invalid;
    };
    match lead {
        0x00..=0x7F | 0x80..=0x8D | 0x8F..=0x9F => Unit::Single(char::from(lead)),
        0x8E => match bytes.get(i + 1) {
            Some(&kana) if cp932::is_hankata(kana) => Unit::Pair(cp932::hankata_char(kana)),
            _ => Unit::Invalid,
        },
        0xA1..=0xFE => {
            let found = bytes
                .get(i + 1)
                .and_then(|&trail| index(lead, trail))
                .and_then(|at| table().forward[at]);
            match found {
                Some(c) => Unit::Pair(c),
                None => Unit::Invalid,
            }
        }
        _ => Unit::Invalid,
    }
}

/// The code page 20932: `CP20932`.
#[derive(Debug)]
pub(crate) struct Cp20932;

impl Codec for Cp20932 {
    fn decode(&self, bytes: &[u8], out: &mut String) {
        let mut i = 0;
        while i < bytes.len() {
            match unit(bytes, i) {
                Unit::Single(c) => {
                    out.push(c);
                    i += 1;
                }
                Unit::Pair(c) => {
                    out.push(c);
                    i += 2;
                }
                Unit::Invalid => {
                    out.push(stand_in(bytes[i]));
                    i += 1;
                }
            }
        }
    }

    fn find_eol(&self, bytes: &[u8], from: usize) -> Option<(usize, Eol)> {
        find_cr_lf(bytes, from)
    }

    fn eol_bytes(&self, eol: Eol) -> Option<&'static [u8]> {
        ascii_eol_bytes(eol)
    }

    fn char_len(&self, bytes: &[u8], i: usize) -> usize {
        match unit(bytes, i) {
            Unit::Pair(_) => 2,
            Unit::Single(_) | Unit::Invalid => 1,
        }
    }

    fn encode(&self, text: &str, out: &mut Vec<u8>) -> Result<(), Unmappable> {
        let mut offsets = Vec::new();
        for (offset, c) in text.char_indices() {
            if let Some(byte) = stand_in_byte(c) {
                out.push(byte);
            } else if c.is_ascii() {
                out.push(c as u8);
            } else if ('\u{FF61}'..='\u{FF9F}').contains(&c) {
                out.extend_from_slice(&[0x8E, (c as u32 - 0xFF61) as u8 + 0xA1]);
            } else if let Some(bytes) = table().reverse.get(&c) {
                out.extend_from_slice(bytes);
            } else {
                // `WideCharToMultiByte` writes `?` for each UTF-16 unit of a character that the
                // code page does not have: two for a character outside the BMP.
                out.extend(std::iter::repeat_n(b'?', c.len_utf16()));
                offsets.push(offset);
            }
        }
        if offsets.is_empty() {
            Ok(())
        } else {
            Err(Unmappable { offsets })
        }
    }
}
