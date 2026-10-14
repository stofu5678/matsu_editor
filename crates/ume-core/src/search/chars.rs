//! Character rules of Sakura's search: case folding and word boundaries.
//!
//! Sakura works on UTF-16 text; this works on `char`s. Where the two differ the difference is
//! noted at the function.

use std::ops::Range;

/// Folds one char the way Sakura's case-insensitive search does.
///
/// Sakura lower-cases each UTF-16 unit with `skr_towlower` (`sakura_core/util/string_ex.cpp`),
/// which is `towlower` for the Unicode Basic Multilingual Plane: ASCII, Latin-1 and Latin
/// Extended, Greek, Cyrillic and full-width Latin (`Ａ` -> `ａ`) all fold. Width is not folded
/// (`Ａ` and `a` stay different). Chars outside the BMP are two UTF-16 units and do not fold
/// ("合成文字,サロゲートの「大文字小文字同一視」未対応",
/// `sakura_core/agent/CSearchAgent.cpp`, `CSearchStringPattern::SetPattern`). A char whose
/// lower case is several chars (`İ`) stays as it is, because `towlower` maps one unit to one.
pub fn fold_char(c: char) -> char {
    if c.is_ascii() {
        return c.to_ascii_lowercase();
    }
    if u32::from(c) > 0xFFFF {
        return c;
    }
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

/// Compares `text` with `folded` (already folded) char by char, folding `text` on the way.
pub(crate) fn eq_folded(text: &str, folded: &[char]) -> bool {
    let mut chars = text.chars();
    folded
        .iter()
        .all(|&f| chars.next().map(fold_char) == Some(f))
        && chars.next().is_none()
}

/// The kind of a char for word boundaries: `ECharKind` in `sakura_core/parse/CWordParse.h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Null,
    Tab,
    Ctrl,
    Space,
    Csym,
    Kata,
    Latin,
    Udef,
    Etc,
    ZenSpace,
    ZenNobasu,
    ZenDaku,
    ZenCsym,
    ZenKigo,
    Hira,
    ZenKata,
    Greek,
    ZenRos,
    ZenSkigo,
    ZenEtc,
}

/// `WCODE::IsZenkakuKigou` (`sakura_core/charset/charcode.h`): the table is copied as it is.
const ZENKAKU_KIGOU: &str = "　、。，．・：；？！´｀¨＾￣＿〃―‐／＼～∥｜…‥‘’“”（）〔〕［］｛｝〈〉《》「」『』【】＋－±×÷＝≠＜＞≦≧∞∴♂♀°′″℃￥＄￠￡％＃＆＊＠§☆★○●◎◇◆□■△▲▽▼※〒→←↑↓〓∈∋⊆⊇⊂⊃∪∩∧∨￢⇒⇔∀∃∠⊥⌒∂∇≡≒≪≫√∽∝∵∫∬Å‰♯♭♪†‡¶◯";

/// `gm_keyword_char` (`sakura_core/charset/charcode.cpp`) for ASCII.
fn ascii_kind(c: u8) -> Kind {
    match c {
        b'\t' => Kind::Tab,
        // CR and LF are kinds of their own in Sakura, but a line here never holds them.
        0..=0x1F | 0x7F => Kind::Ctrl,
        b' ' => Kind::Space,
        b'#' | b'$' | b'@' | b'\\' => Kind::Udef,
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_' => Kind::Csym,
        _ => Kind::Etc,
    }
}

/// `WCODE::IsHankaku` asks the font. We cannot, so this is the common case of a half-width
/// font: ASCII, the C1 controls, half-width kana, and the Latin and IPA range up to U+02FF.
fn is_hankaku(c: char) -> bool {
    let u = u32::from(c);
    u < 0x0300 || (0xFF61..=0xFF9F).contains(&u)
}

/// `CWordParse::WhatKindOfChar` for one char (`sakura_core/parse/CWordParse.cpp`).
/// `ivs` is true when the char is followed by an ideographic variation selector.
fn kind_of(c: char, ivs: bool) -> Kind {
    let u = u32::from(c);
    if ivs {
        return Kind::ZenEtc;
    }
    if u > 0xFFFF {
        // A surrogate pair: CJK extension planes are wide, the rest narrow.
        return if (0x20000..=0x2FFFF).contains(&u) {
            Kind::ZenEtc
        } else {
            Kind::Etc
        };
    }
    if u < 0x80 {
        return ascii_kind(u as u8);
    }
    if (0xFF61..=0xFF9F).contains(&u) {
        return Kind::Kata;
    }
    if (0x00C0..0x0180).contains(&u) && u != 0x00D7 && u != 0x00F7 {
        return Kind::Latin;
    }
    match c {
        '\u{3000}' => return Kind::ZenSpace,
        'ー' => return Kind::ZenNobasu,
        '゛' | '゜' => return Kind::ZenDaku,
        '＿' | '０'..='９' | 'Ａ'..='Ｚ' | 'ａ'..='ｚ' => return Kind::ZenCsym,
        _ => {}
    }
    if ZENKAKU_KIGOU.contains(c) {
        return Kind::ZenKigo;
    }
    match u {
        0x3041..=0x3096 | 0x309D..=0x309E => Kind::Hira,
        0x30A1..=0x30FA | 0x30FD..=0x30FE => Kind::ZenKata,
        0x0391..=0x03C9 => Kind::Greek,
        0x0400..=0x052F | 0x2DE0..=0x2DFF | 0xA640..=0xA69F => Kind::ZenRos,
        0x2500..=0x257F => Kind::ZenSkigo,
        _ if is_hankaku(c) => Kind::Etc,
        _ => Kind::ZenEtc,
    }
}

/// `CWordParse::WhatKindOfTwoChars`: the kind of two neighbours put together, or `Null` when
/// they are different words.
fn merge(pre: Kind, cur: Kind) -> Kind {
    use Kind::*;
    if pre == cur {
        return cur;
    }
    // The prolonged sound mark and the voicing marks stick to hiragana and katakana.
    if matches!(pre, ZenNobasu | ZenDaku) && matches!(cur, ZenKata | Hira) {
        return cur;
    }
    if matches!(cur, ZenNobasu | ZenDaku) && matches!(pre, ZenKata | Hira) {
        return pre;
    }
    if matches!(pre, ZenNobasu | ZenDaku) && matches!(cur, ZenNobasu | ZenDaku) {
        return cur;
    }
    let fix = |k: Kind| match k {
        Latin => Csym,
        Udef | Ctrl => Etc,
        k => k,
    };
    let (pre, cur) = (fix(pre), fix(cur));
    if pre == cur { cur } else { Null }
}

/// The ideographic variation selectors, U+E0100..=U+E01EF (`IsVariationSelector` in
/// `sakura_core/charset/codechecker.h`).
fn is_variation_selector(c: char) -> bool {
    (0xE0100..=0xE01EF).contains(&u32::from(c))
}

/// One line cut into Sakura's "chars": a char, or a char with its variation selector
/// (`CNativeW::GetSizeOfChar` counts that as one).
#[derive(Debug)]
pub(crate) struct Units {
    /// Byte offset of each unit, and one more for the end of the line.
    starts: Vec<usize>,
    kinds: Vec<Kind>,
    chars: Vec<char>,
}

impl Units {
    pub(crate) fn new(line: &str) -> Self {
        let cap = line.len() / 2 + 1;
        let mut starts = Vec::with_capacity(cap);
        let mut kinds = Vec::with_capacity(cap);
        let mut chars = Vec::with_capacity(cap);
        let mut it = line.char_indices().peekable();
        while let Some((i, c)) = it.next() {
            let ivs = u32::from(c) <= 0xFFFF
                && it
                    .peek()
                    .is_some_and(|&(_, next)| is_variation_selector(next));
            if ivs {
                it.next();
            }
            starts.push(i);
            kinds.push(kind_of(c, ivs));
            chars.push(c);
        }
        starts.push(line.len());
        Self {
            starts,
            kinds,
            chars,
        }
    }

    /// The number of units.
    pub(crate) fn len(&self) -> usize {
        self.kinds.len()
    }

    /// The byte offset of unit `i` (the line length for `len()`).
    pub(crate) fn byte(&self, i: usize) -> usize {
        self.starts[i]
    }

    /// The first unit that starts at or after byte `col`.
    pub(crate) fn unit_at_or_after(&self, col: usize) -> usize {
        self.starts[..self.len()].partition_point(|&s| s < col)
    }

    /// The first char of unit `i`.
    pub(crate) fn char_at(&self, i: usize) -> char {
        self.chars[i]
    }

    /// `WhatKindOfChar` past the end is `Null`.
    fn kind(&self, i: usize) -> Kind {
        self.kinds.get(i).copied().unwrap_or(Kind::Null)
    }

    /// The units `[from, to)` of the word that holds unit `idx`
    /// (`CWordParse::WhereCurrentWord_2`). Spaces form words of their own.
    pub(crate) fn current_word(&self, idx: usize) -> Option<Range<usize>> {
        if idx >= self.len() {
            return None;
        }
        let mut kind = self.kinds[idx];
        let mut from = idx;
        while from > 0 {
            let m = merge(self.kinds[from - 1], kind);
            if m == Kind::Null {
                break;
            }
            kind = m;
            from -= 1;
        }
        Some(from..self.word_end_from(idx))
    }

    /// The end of the word that holds unit `idx`, looking only to the right.
    pub(crate) fn word_end_from(&self, idx: usize) -> usize {
        let mut kind = self.kinds[idx];
        let mut to = idx + 1;
        while to < self.len() {
            let m = merge(self.kinds[to], kind);
            if m == Kind::Null {
                break;
            }
            kind = m;
            to += 1;
        }
        to
    }

    /// The start of the next word after unit `idx`, skipping spaces and tabs
    /// (`CWordParse::SearchNextWordPosition` with `bStopsBothEnds` false). Sakura returns the
    /// line end for the last word; this returns `None` there, which the caller treats the same.
    pub(crate) fn next_word(&self, idx: usize) -> Option<usize> {
        let mut kind = self.kind(idx);
        let mut i = idx;
        while i < self.len() {
            i += 1;
            let next = self.kind(i);
            if matches!(next, Kind::Tab | Kind::Space) {
                kind = next;
            } else {
                let m = merge(kind, next);
                if m == Kind::Null {
                    return (i < self.len()).then_some(i);
                }
                kind = m;
            }
        }
        None
    }

    /// The start of the word before unit `idx`, skipping spaces and tabs
    /// (`CWordParse::SearchPrevWordPosition` with `bStopsBothEnds` false).
    pub(crate) fn prev_word(&self, idx: usize) -> Option<usize> {
        if idx == 0 {
            return None;
        }
        let mut kind = self.kind(idx);
        let mut count = 0;
        let mut i = idx;
        while i > 0 {
            let next = self.kinds[i - 1];
            let mut m = merge(next, kind);
            if m == Kind::Null {
                if matches!(kind, Kind::Tab | Kind::Space) || count == 0 {
                    m = next;
                } else {
                    break;
                }
            }
            kind = m;
            i -= 1;
            count += 1;
        }
        Some(i)
    }
}

/// `WCODE::IsWordDelimiter`: space, tab, ideographic space.
pub(crate) fn is_word_delimiter(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{3000}')
}
