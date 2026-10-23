// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/view/CEditView_Paint_Bracket.cpp (SearchBracket, SearchBracketForward,
//     SearchBracketBackward, g_aKakkos)
//
// Original copyright notices of that file:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2022, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Sakura's table of bracket pairs, and the search for the bracket that pairs with another.
//!
//! Source: `g_aKakkos` in `sakura_core/view/CEditView_Paint_Bracket.cpp`. No char appears twice,
//! so the order of the table does not matter.
//!
//! [`search`] is `CEditView::SearchBracket`. The command `F_BRACKETPAIR` searches the whole
//! document (`mode = 3`); the highlight of the pair searches only the rows on the screen
//! (`mode = 2`, [`Reach`]). The search does not know comments or strings: a bracket in a string
//! counts like any other, as in Sakura.

use matsu_core::buffer::{Document, Pos, Result};

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

/// How far the search may go (`mode` without bit 0 in `SearchBracket`). The lines are logical
/// lines, counted from the line of the bracket the search starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reach {
    /// How many lines after that line may be searched (Sakura: the rows from the bracket to the
    /// bottom of the view).
    pub forward: usize,
    /// How many lines before it (the rows from the top of the view to the bracket).
    pub backward: usize,
    /// A line longer than this many bytes ends the search without a result, so that a huge line
    /// is not read at every paint.
    pub max_line_bytes: usize,
}

/// The bracket a search starts from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Start {
    /// Where the bracket is: at the caret, else right before it.
    pub pos: Pos,
    pub ch: char,
    /// It is the char before the caret (bit 2 of `mode`).
    pub before_caret: bool,
}

/// The result of [`search`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    /// The bracket the search started from.
    pub start: Start,
    /// The bracket that pairs with it.
    pub other: Pos,
}

/// The bracket that `SearchBracket` starts from: the char at `caret` if it is a bracket, else the
/// char right before the caret if that is one.
pub fn start_of(doc: &Document, caret: Pos) -> Result<Option<Start>> {
    let Some(line) = doc.line(caret.line)? else {
        return Ok(None);
    };
    let at = line.text.get(caret.col..).and_then(|s| s.chars().next());
    let before = line
        .text
        .get(..caret.col)
        .and_then(|s| s.chars().next_back());
    Ok(match (at, before) {
        (Some(ch), _) if pair_of(ch).is_some() => Some(Start {
            pos: caret,
            ch,
            before_caret: false,
        }),
        (_, Some(ch)) if pair_of(ch).is_some() => Some(Start {
            pos: Pos::new(caret.line, caret.col - ch.len_utf8()),
            ch,
            before_caret: true,
        }),
        _ => None,
    })
}

/// `CEditView::SearchBracket`: finds the bracket that pairs with the one at `caret`, or else with
/// the one right before it. `None` if neither char is a bracket or its pair is not found (within
/// `reach`, if given).
pub fn search(doc: &Document, caret: Pos, reach: Option<Reach>) -> Result<Option<Pair>> {
    let Some(start) = start_of(doc, caret)? else {
        return Ok(None);
    };
    Ok(pair_of_start(doc, start, reach)?.map(|other| Pair { start, other }))
}

/// `SearchBracketForward` or `SearchBracketBackward` from `start`.
pub fn pair_of_start(doc: &Document, start: Start, reach: Option<Reach>) -> Result<Option<Pos>> {
    let Some(((open, close), side)) = pair_of(start.ch) else {
        return Ok(None);
    };
    match side {
        Side::Open => search_forward(doc, start.pos, open, close, reach),
        Side::Close => search_backward(doc, start.pos, open, close, reach),
    }
}

/// Whether the line may be read under `reach` (`false` for a line that is too long).
fn readable(doc: &Document, line: usize, reach: Option<Reach>) -> Result<bool> {
    let Some(reach) = reach else {
        return Ok(true);
    };
    Ok(doc
        .line_len(line)?
        .is_none_or(|len| len <= reach.max_line_bytes))
}

/// `SearchBracketForward`: from the opening bracket at `from` to the end of the document.
fn search_forward(
    doc: &Document,
    from: Pos,
    open: char,
    close: char,
    reach: Option<Reach>,
) -> Result<Option<Pos>> {
    let mut level = 0_usize;
    for (n, line_no) in (from.line..doc.line_count()).enumerate() {
        if reach.is_some_and(|r| n > r.forward) || !readable(doc, line_no, reach)? {
            break;
        }
        let Some(line) = doc.line(line_no)? else {
            break;
        };
        let start = if line_no == from.line { from.col } else { 0 };
        for (i, c) in line.text.get(start..).unwrap_or_default().char_indices() {
            if c == open {
                level += 1;
            } else if c == close {
                level = level.saturating_sub(1);
                if level == 0 {
                    return Ok(Some(Pos::new(line_no, start + i)));
                }
            }
        }
    }
    Ok(None)
}

/// `SearchBracketBackward`: from the closing bracket at `from` to the start of the document.
fn search_backward(
    doc: &Document,
    from: Pos,
    open: char,
    close: char,
    reach: Option<Reach>,
) -> Result<Option<Pos>> {
    let mut level = 1_usize;
    for (n, line_no) in (0..=from.line).rev().enumerate() {
        if reach.is_some_and(|r| n > r.backward) || !readable(doc, line_no, reach)? {
            break;
        }
        let Some(line) = doc.line(line_no)? else {
            continue;
        };
        let end = if line_no == from.line {
            from.col
        } else {
            line.text.len()
        };
        for (i, c) in line.text[..end].char_indices().rev() {
            if c == close {
                level += 1;
            } else if c == open {
                level -= 1;
                if level == 0 {
                    return Ok(Some(Pos::new(line_no, i)));
                }
            }
        }
    }
    Ok(None)
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

    fn doc(text: &str) -> Document {
        Document::from_utf8(text.as_bytes().to_vec()).unwrap()
    }

    fn found(text: &str, caret: (usize, usize), reach: Option<Reach>) -> Option<Pair> {
        search(&doc(text), Pos::new(caret.0, caret.1), reach).unwrap()
    }

    fn far() -> Reach {
        Reach {
            forward: usize::MAX,
            backward: usize::MAX,
            max_line_bytes: usize::MAX,
        }
    }

    #[test]
    fn the_pair_comes_with_where_the_search_started() {
        let at = found("(ab)", (0, 0), None).unwrap();
        assert_eq!(
            (at.start.pos, at.start.before_caret, at.other),
            (Pos::new(0, 0), false, Pos::new(0, 3))
        );
        assert_eq!(at.start.ch, '(');
        // the char before the caret, when the one at it is no bracket
        let before = found("(ab)c", (0, 4), None).unwrap();
        assert_eq!(
            (before.start.pos, before.start.before_caret, before.other),
            (Pos::new(0, 3), true, Pos::new(0, 0))
        );
        assert_eq!(found("(ab", (0, 0), None), None);
        assert_eq!(found("ab", (5, 0), None), None);
    }

    #[test]
    fn brackets_in_strings_and_comments_count() {
        // Sakura does not look at the colors
        let t = "f(\"a)\")";
        assert_eq!(found(t, (0, 1), None).unwrap().other, Pos::new(0, 4));
    }

    #[test]
    fn the_search_stays_within_the_reach() {
        let text = "{\na\nb\nc\n}\n";
        // the closing brace is 4 lines below
        let reach = |forward| Reach { forward, ..far() };
        assert_eq!(found(text, (0, 0), Some(reach(3))), None);
        assert_eq!(
            found(text, (0, 0), Some(reach(4))).unwrap().other,
            Pos::new(4, 0)
        );
        // backwards
        let back = |backward| Reach { backward, ..far() };
        assert_eq!(found(text, (4, 0), Some(back(3))), None);
        assert_eq!(
            found(text, (4, 0), Some(back(4))).unwrap().other,
            Pos::new(0, 0)
        );
        // a search without a reach goes on
        assert!(found(text, (0, 0), None).is_some());
        // a reach of no lines still reads the line of the bracket
        assert_eq!(
            found("(a)", (0, 0), Some(reach(0))).unwrap().other,
            Pos::new(0, 2)
        );
    }

    #[test]
    fn a_line_that_is_too_long_ends_the_search() {
        let reach = Reach {
            max_line_bytes: 8,
            ..far()
        };
        let long = format!("(\n{}\n)", "x".repeat(20));
        assert_eq!(found(&long, (0, 0), Some(reach)), None);
        assert!(found(&long, (0, 0), None).is_some());
        let short = "(\nxx\n)";
        assert!(found(short, (0, 0), Some(reach)).is_some());
    }

    #[test]
    fn nested_pairs_and_unbalanced_text() {
        assert_eq!(
            found("((a)", (0, 0), None),
            None,
            "the first is never closed"
        );
        assert_eq!(found("(a))", (0, 0), None).unwrap().other, Pos::new(0, 2));
        assert_eq!(found("((a))", (0, 1), None).unwrap().other, Pos::new(0, 3));
    }

    #[test]
    fn a_big_document_is_searched_by_its_reach_only() {
        let text = "(\n".to_string() + &"x\n".repeat(200_000) + ")";
        let reach = Reach {
            forward: 60,
            ..far()
        };
        let started = std::time::Instant::now();
        assert_eq!(found(&text, (0, 0), Some(reach)), None);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }
}
