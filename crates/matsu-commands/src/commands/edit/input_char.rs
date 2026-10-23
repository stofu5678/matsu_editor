//! `F_WCHAR`: types one char, or a line break when the char is CR or LF (Enter).
//!
//! Sakura: `CViewCommander::Command_WCHAR` in `sakura_core/cmd/CViewCommander_Edit.cpp`, called
//! from `CViewCommander::HandleCommand`. The Enter key has no binding in Sakura's key table;
//! the window gets the char `\r` and runs `F_WCHAR` with it.
//!
//! * A selection is replaced by the char.
//! * In overwrite mode (no selection), the char replaces the char at the caret: a tab, a
//!   full-width char or a char of a surrogate pair is one char. At the end of the text of a line
//!   it is inserted, because a line ending is not overwritten (`m_bNotOverWriteCRLF` is on by
//!   default); the same at the end of the document. `DelCharForOverwrite` in the same file.
//! * A line break char becomes the line ending of the document (`GetNewLineCode`), and the
//!   indentation of the line before the caret is copied after it (auto indent,
//!   `m_bAutoIndent`, on by default in `sakura_core/types/CType.cpp`).
//!
//! * With a box selected, the char goes before the box in every row and the box moves right
//!   (`Command_INDENT`, see `CommandContext::insert_text`); Enter deletes the box first.
//!
//! Not done yet: the option to pad with spaces to the width of the replaced char
//! (`m_bOverWriteFixMode`, off by default), the option to overwrite line endings, the other
//! indent chars and smart indent of the type settings, trimming the previous line
//! (`m_bRTrimPrevLine`), and the extended line endings (`m_bEnableExtEol`).
//!
//! Argument: one char, as `Arg::Char`, or a code point as `Arg::Int`.

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::{CommandContext, is_indent_char};
use crate::register_command;
use matsu_core::model::eol_text;

#[derive(Default)]
pub struct InputChar;

impl Command for InputChar {
    const ID: &'static str = "input_char";
    const SAKURA: &'static str = "F_WCHAR";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        let c = char_arg(args)?;
        type_char(cx, c)
    }
}

/// Types `c` as `Command_WCHAR` does (`F_IME_CHAR` uses it too, see [`super::ime_char`]).
pub(super) fn type_char(cx: &mut CommandContext<'_>, c: char) -> Result {
    if c == '\r' || c == '\n' {
        // `Command_WCHAR` deletes the selection first, a box as a box.
        if cx.cursor().is_box() {
            cx.delete_selection()?;
        }
        // One edit replaces the selection: deleting it first could join the CR and the LF
        // around it into a CRLF and move the place to insert.
        let text = format!(
            "{}{}",
            eol_text(cx.new_line()),
            indent_before_insertion(cx)?
        );
        cx.insert_text(&text)?;
    } else {
        cx.type_text(c.encode_utf8(&mut [0; 4]))?;
    }
    Ok(())
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

/// The indentation of the line, up to where the text is inserted: the caret, or the start of the
/// selection (Sakura copies the indent chars that are before that place, so Enter in the middle
/// of the indentation does not copy all of it).
pub(super) fn indent_before_insertion(cx: &CommandContext<'_>) -> Result<String> {
    let at = cx.selection().map_or(cx.caret(), |(start, _)| start);
    let line = cx.doc().line(at.line)?.map(|l| l.text).unwrap_or_default();
    let before = line.get(..at.col).unwrap_or(&line);
    Ok(before.chars().take_while(|&c| is_indent_char(c)).collect())
}

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Eol;

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

    fn overtyped(c: char, before: &str, after: &str) {
        let mut e = Editor::new(before);
        e.set_overwrite(true);
        e.run_with(ID, &[Arg::Char(c)]).assert_marked(after);
    }

    #[test]
    fn overwrite_mode_replaces_the_char_at_the_caret() {
        // Sakura: DelCharForOverwrite, then the insertion (CViewCommander_Edit.cpp)
        overtyped('X', "ab|cd", "abX|d");
        overtyped('X', "|abc", "X|bc");
        overtyped('X', "ab|c\ndef", "abX|\ndef");
        // a full-width char, a char of two UTF-16 units, and a tab are one char each
        overtyped('X', "a|日b", "aX|b");
        overtyped('X', "a|😀b", "aX|b");
        overtyped('X', "a|\tb", "aX|b");
        // the typed char may be wider than the replaced one: the next char stays
        overtyped('日', "a|bc", "a日|c");
    }

    #[test]
    fn overwrite_mode_inserts_at_the_end_of_a_line_and_of_the_document() {
        // m_bNotOverWriteCRLF is on by default: the line ending stays
        overtyped('X', "ab|\r\ncd", "abX|\r\ncd");
        overtyped('X', "ab|\ncd", "abX|\ncd");
        overtyped('X', "ab\n|", "ab\nX|");
        overtyped('X', "|", "X|");
    }

    #[test]
    fn overwrite_mode_replaces_a_selection_and_enter_inserts() {
        overtyped('X', "a[bc|de", "aX|de");
        let mut e = Editor::new("ab|cd").with_new_line(Eol::Lf);
        e.set_overwrite(true);
        e.run_with(ID, &[Arg::Char('\r')]).assert_marked("ab\n|cd");
    }

    #[test]
    fn overwrite_mode_is_one_edit_and_one_undo_step() {
        let mut e = Editor::new("ab|cd");
        e.set_overwrite(true);
        e.run_with(ID, &[Arg::Char('X')]);
        assert_eq!(e.changes().edits.len(), 1);
        e.run("undo").assert_marked("ab|cd");
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
        // a document without a line ending gets the line ending of the OS (CRLF on Windows, as
        // in Sakura; LF elsewhere)
        let os = if cfg!(windows) { "\r\n" } else { "\n" };
        assert_eq!(enter("ab|cd"), format!("ab{os}|cd"));
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
    fn enter_over_a_selection_is_one_edit() {
        // deleting the selection first would join the CR and the LF around it
        let mut e = Editor::new("z\na\r[b|\nc").with_new_line(Eol::Lf);
        e.run_with(ID, &[Arg::Char('\r')]);
        e.assert_marked("z\na\r\n|\nc");
        assert_eq!(e.changes().edits.len(), 1);
        // the indentation is the one before the selection
        let mut e = Editor::new("  a[b|c").with_new_line(Eol::Lf);
        e.run_with(ID, &[Arg::Char('\r')])
            .assert_marked("  a\n  |c");
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
