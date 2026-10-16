//! `F_BRACKETPAIR`: moves the caret to the bracket that pairs with the one at the caret.
//!
//! Sakura: `CViewCommander::Command_BRACKETPAIR` in `sakura_core/cmd/CViewCommander_Search.cpp`,
//! which calls `CEditView::SearchBracket`, `SearchBracketForward` and `SearchBracketBackward`
//! in `sakura_core/view/CEditView_Paint_Bracket.cpp`, with the table `g_aKakkos` (see
//! [`crate::text::bracket`]).
//!
//! * If the char at the caret is a bracket, that is the one; otherwise the char before the caret,
//!   if it is one. Without a bracket nothing happens (no beep).
//! * An opening bracket searches to the end of the document, a closing one to the start. Only
//!   brackets of the same pair count, and pairs nest. The caret goes in front of the bracket
//!   that is found.
//! * Sakura passes `mode = 3`: the search is not limited to the lines that are on the screen
//!   (bit 0), and the char before the caret is checked too (bit 1). The limit by the screen is
//!   used only when Sakura draws the highlight of a pair, so it is not needed here.
//!
//! Sakura moves with `MoveCursorSelecting(..., m_bSelectingLock)`: while the selection lock is on
//! (after `F_BEGIN_SEL` or `F_BEGIN_BOX`) the move extends the selection.

use ume_core::buffer::{Document, Pos};

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;
use crate::text::bracket::{Side, pair_of};

#[derive(Default)]
pub struct BracketPair;

impl Command for BracketPair {
    const ID: &'static str = "bracket_pair";
    const SAKURA: &'static str = "F_BRACKETPAIR";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        if let Some(target) = find_pair(cx.doc(), cx.caret())? {
            let extend = cx.extends(false);
            cx.move_caret(target, extend)?;
        }
        Ok(())
    }
}

register_command!(BracketPair);

/// The position of the bracket that pairs with the one at (or before) `caret`.
fn find_pair(doc: &Document, caret: Pos) -> Result<Option<Pos>> {
    let Some(line) = doc.line(caret.line)? else {
        return Ok(None);
    };
    let at = line.text.get(caret.col..).and_then(|s| s.chars().next());
    let before = line
        .text
        .get(..caret.col)
        .and_then(|s| s.chars().next_back());
    // the char at the caret first, then the one before it
    let (bracket_pos, c) = match (at, before) {
        (Some(c), _) if pair_of(c).is_some() => (caret, c),
        (_, Some(c)) if pair_of(c).is_some() => (Pos::new(caret.line, caret.col - c.len_utf8()), c),
        _ => return Ok(None),
    };
    let Some(((open, close), side)) = pair_of(c) else {
        return Ok(None);
    };
    match side {
        Side::Open => search_forward(doc, bracket_pos, open, close),
        Side::Close => search_backward(doc, bracket_pos, open, close),
    }
}

/// From the opening bracket at `from` to the end of the document.
fn search_forward(doc: &Document, from: Pos, open: char, close: char) -> Result<Option<Pos>> {
    let mut level = 0_usize;
    for line_no in from.line..doc.line_count() {
        let Some(line) = doc.line(line_no)? else {
            break;
        };
        let start = if line_no == from.line { from.col } else { 0 };
        for (i, c) in line.text.get(start..).unwrap_or_default().char_indices() {
            if c == open {
                level += 1;
            } else if c == close {
                level -= 1;
                if level == 0 {
                    return Ok(Some(Pos::new(line_no, start + i)));
                }
            }
        }
    }
    Ok(None)
}

/// From the closing bracket at `from` to the start of the document.
fn search_backward(doc: &Document, from: Pos, open: char, close: char) -> Result<Option<Pos>> {
    let mut level = 1_usize;
    for line_no in (0..=from.line).rev() {
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
    use ume_core::buffer::Pos;
    use ume_core::cursor::Cursor;

    use crate::testing::{Editor, check};

    #[test]
    fn from_an_opening_bracket_to_the_closing_one() {
        check("bracket_pair", "|(ab)", "(ab|)");
        // `[` cannot be written in a marked text
        let mut e = Editor::from_parts("x[ab]y", Cursor::new(Pos::new(0, 1)));
        e.run("bracket_pair");
        assert_eq!(e.cursor().caret(), Pos::new(0, 4));
        e.run("bracket_pair");
        assert_eq!(e.cursor().caret(), Pos::new(0, 1));
        check("bracket_pair", "|{a}", "{a|}");
        check("bracket_pair", "|<a>", "<a|>");
    }

    #[test]
    fn extends_the_box_while_it_is_locked() {
        let mut e = Editor::new("|(ab)");
        e.run("begin_box")
            .run("bracket_pair")
            .assert_box((0, 0), (0, 3));
    }

    #[test]
    fn from_a_closing_bracket_to_the_opening_one() {
        check("bracket_pair", "(ab|)", "|(ab)");
        check("bracket_pair", "(a(b)c|)", "|(a(b)c)");
    }

    #[test]
    fn the_char_before_the_caret_counts_when_the_one_at_the_caret_is_no_bracket() {
        check("bracket_pair", "(ab)|", "|(ab)");
        check("bracket_pair", "(|ab", "(|ab");
        check("bracket_pair", "(ab)|c", "|(ab)c");
        // the one at the caret comes first
        check("bracket_pair", "(a)|(b)", "(a)(b|)");
    }

    #[test]
    fn pairs_nest_and_only_the_same_pair_counts() {
        check("bracket_pair", "|(a(b)c)", "(a(b)c|)");
        check("bracket_pair", "(a(b|)c)", "(a|(b)c)");
        check("bracket_pair", "|(a}b{c)", "(a}b{c|)");
        check("bracket_pair", "|(a)b)", "(a|)b)");
    }

    #[test]
    fn searches_over_lines() {
        check("bracket_pair", "|{\n  {x}\n}\n", "{\n  {x}\n|}\n");
        check("bracket_pair", "{\r\n  {x}\r\n|}", "|{\r\n  {x}\r\n}");
        check("bracket_pair", "a(\n\nb|)", "a|(\n\nb)");
    }

    #[test]
    fn full_width_brackets() {
        check("bracket_pair", "|「あ」", "「あ|」");
        check("bracket_pair", "「あ」|", "|「あ」");
        check("bracket_pair", "|【「あ」】", "【「あ」|】");
        check("bracket_pair", "|（あ）", "（あ|）");
        check("bracket_pair", "|“a”", "“a|”");
        // half-width and full-width brackets are different pairs
        check("bracket_pair", "|(あ）", "|(あ）");
        check("bracket_pair", "|\u{FF62}a\u{FF63}", "\u{FF62}a|\u{FF63}");
    }

    #[test]
    fn does_nothing_without_a_pair() {
        for text in ["|(ab", "a|b)c", "a|bc", "|", "ab\n|"] {
            let mut e = Editor::new(text);
            e.run("bracket_pair").assert_marked(text);
            assert!(!e.changes().beep, "{text:?}");
        }
    }

    #[test]
    fn drops_the_selection() {
        check("bracket_pair", "[a|(b)", "a(b|)");
    }
}
