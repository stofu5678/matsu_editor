//! `F_WCHAR`: types one char, or a line break when the char is CR or LF (Enter).
//!
//! Sakura: `CViewCommander::Command_WCHAR` in `sakura_core/cmd/CViewCommander_Edit.cpp`, called
//! from `CViewCommander::HandleCommand`. The Enter key has no binding in Sakura's key table;
//! the window gets the char `\r` and runs `F_WCHAR` with it.
//!
//! * A selection is replaced by the char.
//! * A line break char becomes the line ending of the document (`GetNewLineCode`), and the
//!   indentation of the line before the caret is copied after it (auto indent,
//!   `m_bAutoIndent`, on by default in `sakura_core/types/CType.cpp`).
//!
//! Not done yet: overwrite mode (`IsInsMode`), typing over a rectangular selection
//! (`Command_INDENT`), the other indent chars and smart indent of the type settings, trimming
//! the previous line (`m_bRTrimPrevLine`), and the extended line endings (`m_bEnableExtEol`).
//!
//! Argument: one char, as `Arg::Char`, or a code point as `Arg::Int`.

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::{CommandContext, is_indent_char};
use crate::register_command;
use ume_core::model::eol_text;

#[derive(Default)]
pub struct InputChar;

impl Command for InputChar {
    const ID: &'static str = "input_char";
    const SAKURA: &'static str = "F_WCHAR";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        let c = char_arg(args)?;
        if c == '\r' || c == '\n' {
            cx.delete_selection()?;
            let text = format!("{}{}", eol_text(cx.new_line()), indent_before_caret(cx)?);
            cx.insert_text(&text)?;
        } else {
            cx.insert_text(c.encode_utf8(&mut [0; 4]))?;
        }
        Ok(())
    }
}

register_command!(InputChar);

fn char_arg(args: &[Arg]) -> Result<char> {
    let bad = || CommandError::BadArgs("input_char takes one char".to_owned());
    match args {
        [Arg::Char(c)] => Ok(*c),
        [Arg::Int(n)] => u32::try_from(*n)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(bad),
        _ => Err(bad()),
    }
}

/// The indentation of the line, up to the caret (Sakura copies the indent chars that are before
/// the caret, so Enter in the middle of the indentation does not copy all of it).
fn indent_before_caret(cx: &CommandContext<'_>) -> Result<String> {
    let caret = cx.caret();
    let line = cx
        .doc()
        .line(caret.line)?
        .map(|l| l.text)
        .unwrap_or_default();
    let before = line.get(..caret.col).unwrap_or(&line);
    Ok(before.chars().take_while(|&c| is_indent_char(c)).collect())
}

#[cfg(test)]
mod tests {
    use ume_core::buffer::Eol;

    use crate::command::{Arg, CommandError};
    use crate::testing::{Editor, check_with};

    const ID: &str = "input_char";

    fn typed(c: char, before: &str, after: &str) {
        check_with(ID, &[Arg::Char(c)], before, after);
    }

    #[test]
    fn types_a_char_at_the_caret() {
        typed('X', "ab|cd", "abX|cd");
        typed('X', "|", "X|");
        typed('日', "a|", "a日|");
        typed('😀', "|a", "😀|a");
        typed('X', "ab|\r\ncd", "abX|\r\ncd");
        typed('X', "ab\r\n|", "ab\r\nX|");
    }

    #[test]
    fn a_code_point_works_as_well() {
        check_with(ID, &[Arg::Int(0x65E5)], "|", "日|");
    }

    #[test]
    fn replaces_the_selection() {
        typed('X', "a[bc|d", "aX|d");
        typed('X', "a|bc[d", "aX|d");
    }

    #[test]
    fn enter_inserts_the_line_ending_of_the_document() {
        let enter = |text: &str| {
            let mut e = Editor::new(text);
            e.run_with(ID, &[Arg::Char('\r')]);
            e.marked()
        };
        // the first line ending of the document decides
        assert_eq!(enter("ab|cd\r\nef"), "ab\r\n|cd\r\nef");
        assert_eq!(enter("ab|cd\nef"), "ab\n|cd\nef");
        assert_eq!(enter("ab|cd\ref"), "ab\r|cd\ref");
        assert_eq!(enter("ab\r\n|"), "ab\r\n\r\n|");
        // a document without a line ending gets CRLF, as a new document of Sakura on Windows
        assert_eq!(enter("ab|cd"), "ab\r\n|cd");
        // LF has the same effect as CR
        let mut e = Editor::new("ab|cd").with_new_line(Eol::Lf);
        e.run_with(ID, &[Arg::Char('\n')]).assert_marked("ab\n|cd");
        e.run_with(ID, &[Arg::Char('\r')])
            .assert_marked("ab\n\n|cd");
    }

    #[test]
    fn enter_replaces_the_selection() {
        let mut e = Editor::new("a[bc|d").with_new_line(Eol::Lf);
        e.run_with(ID, &[Arg::Char('\r')]).assert_marked("a\n|d");
    }

    #[test]
    fn enter_copies_the_indentation_before_the_caret() {
        let enter = |text: &str| {
            let mut e = Editor::new(text).with_new_line(Eol::Lf);
            e.run_with(ID, &[Arg::Char('\r')]);
            e.marked()
        };
        assert_eq!(enter("  \tab|cd"), "  \tab\n  \t|cd");
        // an ideographic space is indentation (type setting m_bAutoIndent_ZENSPACE is on)
        assert_eq!(enter("\u{3000}a|b"), "\u{3000}a\n\u{3000}|b");
        // in the middle of the indentation only the part before the caret is copied
        assert_eq!(enter(" | x"), " \n | x");
        // the second line of the document
        assert_eq!(enter("x\n  y|"), "x\n  y\n  |");
        // a caret at the start of an indented line: nothing to copy
        assert_eq!(enter("|  x"), "\n|  x");
    }

    #[test]
    fn is_one_undo_step() {
        let mut e = Editor::new("ab|cd").with_new_line(Eol::Lf);
        e.run_with(ID, &[Arg::Char('\r')]);
        e.run("undo").assert_marked("ab|cd");
        e.run("redo").assert_marked("ab\n|cd");
        // each typed char is a step of its own, as in Sakura
        let mut e = Editor::new("|");
        e.run_with(ID, &[Arg::Char('a')])
            .run_with(ID, &[Arg::Char('b')]);
        e.run("undo").assert_marked("a|");
        e.run("undo").assert_marked("|");
    }

    #[test]
    fn reports_the_change() {
        let mut e = Editor::new("ab|cd");
        e.run_with(ID, &[Arg::Char('X')]);
        assert!(e.changes().text_changed());
        assert!(e.changes().caret_moved);
        assert!(!e.changes().beep);
    }

    #[test]
    fn rejects_bad_arguments() {
        for args in [
            &[][..],
            &[Arg::Text("ab".into())],
            &[Arg::Int(-1)],
            &[Arg::Int(0xD800)],
        ] {
            let err = Editor::new("|").try_run(ID, args).unwrap_err();
            assert!(matches!(err, CommandError::BadArgs(_)), "{args:?}");
        }
    }
}
