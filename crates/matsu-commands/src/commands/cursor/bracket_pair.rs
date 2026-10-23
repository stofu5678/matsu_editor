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

use crate::command::{Arg, Command, Result};
use crate::context::CommandContext;
use crate::register_command;
use crate::text::bracket::search;

#[derive(Default)]
pub struct BracketPair;

impl Command for BracketPair {
    const ID: &'static str = "bracket_pair";
    const SAKURA: &'static str = "F_BRACKETPAIR";

    fn run(&self, cx: &mut CommandContext<'_>, _args: &[Arg]) -> Result {
        // `mode = 3`: the whole document, and the char before the caret too.
        if let Some(pair) = search(cx.doc(), cx.caret(), None)? {
            let extend = cx.extends(false);
            cx.move_caret(pair.other, extend)?;
        }
        Ok(())
    }
}

register_command!(BracketPair);

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Pos;
    use matsu_core::cursor::Cursor;

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
