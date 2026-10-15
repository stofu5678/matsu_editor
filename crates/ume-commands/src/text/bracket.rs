//! Sakura's table of bracket pairs.
//!
//! Source: `g_aKakkos` in `sakura_core/view/CEditView_Paint_Bracket.cpp`. No char appears twice,
//! so the order of the table does not matter.

/// The pairs, as (opening, closing): half-width first, then full-width.
pub const PAIRS: [(char, char); 18] = [
    ('(', ')'),
    ('[', ']'),
    ('{', '}'),
    ('<', '>'),
    ('\u{FF62}', '\u{FF63}'), // half-width corner brackets
    ('【', '】'),
    ('『', '』'),
    ('「', '」'),
    ('＜', '＞'),
    ('≪', '≫'),
    ('《', '》'),
    ('（', '）'),
    ('〈', '〉'),
    ('｛', '｝'),
    ('〔', '〕'),
    ('［', '］'),
    ('“', '”'),
    ('〝', '〟'),
];

/// Which side of a pair a char is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Open,
    Close,
}

/// The pair that `c` belongs to, and which side of it `c` is.
pub fn pair_of(c: char) -> Option<((char, char), Side)> {
    PAIRS.iter().find_map(|&(open, close)| {
        if c == open {
            Some(((open, close), Side::Open))
        } else if c == close {
            Some(((open, close), Side::Close))
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_both_sides() {
        assert_eq!(pair_of('('), Some((('(', ')'), Side::Open)));
        assert_eq!(pair_of('」'), Some((('「', '」'), Side::Close)));
        assert_eq!(
            pair_of('\u{FF63}'),
            Some((('\u{FF62}', '\u{FF63}'), Side::Close))
        );
        assert_eq!(pair_of('a'), None);
    }

    #[test]
    fn no_char_is_in_two_pairs() {
        for (i, a) in PAIRS.iter().enumerate() {
            assert_ne!(a.0, a.1);
            for b in &PAIRS[i + 1..] {
                assert!(a.0 != b.0 && a.0 != b.1 && a.1 != b.0 && a.1 != b.1);
            }
        }
    }
}
