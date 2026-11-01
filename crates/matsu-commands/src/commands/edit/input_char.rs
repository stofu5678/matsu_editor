// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/cmd/CViewCommander_Edit.cpp (CViewCommander::Command_WCHAR)
//   sakura_core/view/CEditView_Command_New.cpp (CEditView::RTrimPrevLine)
//   sakura_core/charset/charcode.h (WCODE::IsIndentChar, WCODE::IsBlank)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000, genta
//   Copyright (C) 2001, genta, asa-o, hor
//   Copyright (C) 2002, YAZAKI, hor, genta. aroka, MIK, minfu, KK, かろと
//   Copyright (C) 2003, MIK, Moca, genta, zenryaku, ryoji, naoh, KEITA, じゅうじ
//   Copyright (C) 2004, genta, Moca
//   Copyright (C) 2005, genta, D.S.Koba, ryoji
//   Copyright (C) 2006, genta, Moca, fon
//   Copyright (C) 2007, ryoji, maru, kobake
//   Copyright (C) 2008, ryoji, nasukoji
//   Copyright (C) 2009, ryoji
//   Copyright (C) 2010, ryoji
//   Copyright (C) 2018-2026, Sakura Editor Organization
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
//! * With `m_bNotOverWriteCRLF` off, the line ending is deleted at the end of a line, so the next
//!   line joins. With `m_bOverWriteFixMode` on, a char of another width leaves the columns of the
//!   rest of the line in place: spaces fill what a narrower char leaves, and a wider one takes
//!   the next char too (see `CommandContext::type_text`). With a box selected, `m_bOverWriteBoxDelete`
//!   deletes the box before typing.
//!
//! * The type settings of the "画面" page: `m_bAutoIndent` (off: nothing is copied),
//!   `m_bAutoIndent_ZENSPACE` (whether an ideographic space is indentation), `m_szIndentChars`
//!   (chars that are indentation besides the blanks), `m_eSmartIndent` (C/C++ only, see
//!   `CommandContext::smart_indent_cpp`) and `m_bRTrimPrevLine` (the blanks at the end of the line
//!   that the line break leaves are deleted). The smart indent also reacts to `: { } ( )`. With a
//!   box selected the char goes no further than `Command_INDENT` (no smart indent).
//!
//! Not done yet: the extended line endings (`m_bEnableExtEol`) and the smart indent of plug-ins.
//!
//! Argument: one char, as `Arg::Char`, or a code point as `Arg::Int`.

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::input_options::SmartIndent;
use crate::register_command;
use matsu_core::buffer::Pos;
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
    type_char_as(cx, c, true)
}

/// `Command_WCHAR( c, bConvertEOL )`: with `convert_eol` off (`F_CTRL_CODE`) a CR or LF is
/// inserted as it is, not as the line ending of the document.
pub(super) fn type_char_as(cx: &mut CommandContext<'_>, c: char, convert_eol: bool) -> Result {
    let is_break = c == '\r' || c == '\n';
    if is_break {
        // `Command_WCHAR` deletes the selection first, a box as a box.
        if cx.cursor().is_box() {
            cx.delete_selection()?;
        }
        // One edit replaces the selection: deleting it first could join the CR and the LF
        // around it into a CRLF and move the place to insert.
        let line_break = if convert_eol {
            eol_text(cx.new_line()).to_owned()
        } else {
            c.to_string()
        };
        let text = format!("{line_break}{}", indent_before_insertion(cx)?);
        cx.insert_text(&text)?;
    } else {
        // A box selection takes the char as `Command_INDENT` does and goes no further.
        let boxed = cx.cursor().is_box() && cx.cursor().has_selection();
        cx.type_text(c.encode_utf8(&mut [0; 4]))?;
        if boxed {
            return Ok(());
        }
    }
    if cx.input_options().smart_indent == SmartIndent::Cpp {
        cx.smart_indent_cpp(c)?;
    }
    if is_break && cx.input_options().rtrim_prev_line {
        rtrim_prev_line(cx)?;
    }
    Ok(())
}

/// `CEditView::RTrimPrevLine` (`sakura_core/view/CEditView_Command_New.cpp`): deletes the blanks at
/// the end of the line before the caret's. The caret stays where it was.
fn rtrim_prev_line(cx: &mut CommandContext<'_>) -> Result {
    let caret = cx.caret();
    let Some(prev) = caret.line.checked_sub(1) else {
        return Ok(());
    };
    let text = cx.doc().line(prev)?.map(|l| l.text).unwrap_or_default();
    // `WCODE::IsBlank`: a space, a tab or an ideographic space.
    let kept = text.trim_end_matches([' ', '\t', '\u{3000}']).len();
    if kept < text.len() {
        // The rows after the caret's stay as they are; a bare CR before a line of blanks and
        // the LF behind it join into one CRLF and the document has a line less.
        let lines_after = cx.doc().line_count() - caret.line;
        cx.replace(Pos::new(prev, kept), Pos::new(prev, text.len()), "")?;
        let line = cx.doc().line_count().saturating_sub(lines_after);
        cx.move_caret(Pos::new(line, caret.col), false)?;
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
///
/// A char is indentation if the type says so: a space or a tab, an ideographic space with
/// `m_bAutoIndent_ZENSPACE`, or a char of `m_szIndentChars`; with `m_bAutoIndent` off there is
/// none (`Command_WCHAR`).
pub(super) fn indent_before_insertion(cx: &CommandContext<'_>) -> Result<String> {
    let options = cx.input_options();
    if !options.auto_indent {
        return Ok(String::new());
    }
    let at = cx.selection().map_or(cx.caret(), |(start, _)| start);
    let line = cx.doc().line(at.line)?.map(|l| l.text).unwrap_or_default();
    let before = line.get(..at.col).unwrap_or(&line);
    Ok(before
        .chars()
        .take_while(|&c| {
            // `wcsstr( m_szIndentChars, ch )`, but never for NUL
            let listed = c != '\0' && options.indent_chars.contains(c);
            listed || matches!(c, ' ' | '\t') || (options.auto_indent_zenspace && c == '\u{3000}')
        })
        .collect())
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

    use crate::edit_options::{EditOptions, with_options};

    #[test]
    fn overwrite_mode_deletes_the_line_ending_without_the_option() {
        // m_bNotOverWriteCRLF off: Command_DELETE at the end of the text joins the next line
        let options = EditOptions {
            not_overwrite_crlf: false,
            ..Default::default()
        };
        with_options(options, || {
            overtyped('X', "ab|\r\ncd", "abX|cd");
            overtyped('X', "ab|\ncd", "abX|cd");
            // the end of the document has no line ending to delete
            overtyped('X', "ab|", "abX|");
            overtyped('X', "ab\n|", "ab\nX|");
            // inside the line it is the same
            overtyped('X', "a|bc", "aX|c");
        });
    }

    #[test]
    fn overwrite_fix_mode_keeps_the_columns() {
        let options = EditOptions {
            overwrite_fix_mode: true,
            ..Default::default()
        };
        with_options(options, || {
            // a narrow char over a full-width one: a space fills the second column
            overtyped('X', "a|日b", "aX| b");
            // a full-width char over a narrow one takes the next narrow char too
            overtyped('日', "a|bcd", "a日|d");
            // ... and a full-width one gives a space back when the two take more than it needs
            overtyped('日', "a|b日d", "a日| d");
            // the same width: the plain replacement
            overtyped('X', "a|bc", "aX|c");
            overtyped('日', "|あい", "日|い");
            // a tab is replaced as any char, without spaces
            overtyped('X', "a|\tb", "aX|b");
            overtyped('日', "a|\tb", "a日|b");
            // a wide char over a narrow one before a tab or the end of the text
            overtyped('日', "a|b\tc", "a日|\tc");
            overtyped('日', "a|b", "a日|");
            // the end of the text: inserted
            overtyped('日', "ab|", "ab日|");
            // one edit, one undo step
            let mut e = Editor::new("a|日b");
            e.set_overwrite(true);
            e.run_with(ID, &[Arg::Char('X')]);
            assert_eq!(e.changes().edits.len(), 1);
            e.run("undo").assert_marked("a|日b");
        });
    }

    /// Types `text` at once, as the IME commits it.
    fn overtyped_text(text: &str, before: &str, after: &str) {
        let mut e = Editor::new(before);
        e.set_overwrite(true);
        e.with_context(|cx| cx.type_text(text).map(|_| ())).unwrap();
        e.assert_marked(after);
    }

    #[test]
    fn several_chars_overwrite_one_by_one() {
        // DelCharForOverwrite runs for each char: the spaces of the fix mode are chars at the caret
        let options = EditOptions {
            overwrite_fix_mode: true,
            ..Default::default()
        };
        with_options(options, || {
            overtyped_text("日本", "a|bcdef", "a日本|f");
            overtyped_text("XY", "a|日b", "aXY|b");
            overtyped_text("X日", "a|bcd", "aX日|");
            overtyped_text("日X", "a|bcd", "a日X|");
            // a line break is inserted
            overtyped_text("X\nY", "a|bcd", "aX\nY|d");
        });
        // without the fix mode a char replaces a char
        overtyped_text("日本", "a|bcdef", "a日本|def");
        overtyped_text("XY", "a|日b", "aXY|");
        // nothing to overwrite for no chars, also at the end of a line with the option on
        let options = EditOptions {
            not_overwrite_crlf: false,
            ..Default::default()
        };
        with_options(options, || {
            overtyped_text("", "ab|\ncd", "ab|\ncd");
            // the first char deletes the line ending, the next one overwrites the next line
            overtyped_text("XY", "ab|\ncd", "abXY|d");
            overtyped_text("X", "ab|\n\ncd", "abX|\ncd");
        });
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

    // ---- the indent options of the type ----

    use crate::input_options::{InputOptions, SmartIndent};

    /// Types `c` over `before` with `options`; the line ending is LF.
    fn with(options: &InputOptions, c: char, before: &str, after: &str) {
        let mut e = Editor::new(before)
            .with_new_line(Eol::Lf)
            .with_input_options(options.clone());
        e.run_with(ID, &[Arg::Char(c)]).assert_marked(after);
    }

    fn cpp() -> InputOptions {
        InputOptions {
            smart_indent: SmartIndent::Cpp,
            ..Default::default()
        }
    }

    #[test]
    fn auto_indent_off_copies_nothing() {
        let off = InputOptions {
            auto_indent: false,
            ..Default::default()
        };
        with(&off, '\r', "  \tab|cd", "  \tab\n|cd");
        with(&off, '\r', "ab|", "ab\n|");
        with(&InputOptions::default(), '\r', "  ab|", "  ab\n  |");
    }

    #[test]
    fn the_ideographic_space_is_indentation_with_the_option() {
        let no_zen = InputOptions {
            auto_indent_zenspace: false,
            ..Default::default()
        };
        with(&no_zen, '\r', "\u{3000}a|b", "\u{3000}a\n|b");
        with(&no_zen, '\r', " \u{3000}a|b", " \u{3000}a\n |b");
        let defaults = InputOptions::default();
        with(&defaults, '\r', " \u{3000}a|b", " \u{3000}a\n \u{3000}|b");
    }

    #[test]
    fn the_other_indent_chars_are_indentation() {
        let quote = InputOptions {
            indent_chars: ">".to_owned(),
            ..Default::default()
        };
        with(&quote, '\r', "> > text|", "> > text\n> > |");
        with(&InputOptions::default(), '\r', "> > text|", "> > text\n|");
        let several = InputOptions {
            indent_chars: "*-".to_owned(),
            ..Default::default()
        };
        with(&several, '\r', "-* a|", "-* a\n-* |");
        // a char that is not listed ends the indentation
        with(&several, '\r', "-x* a|", "-x* a\n-|");
        // only the chars before the caret count
        with(&quote, '\r', ">|> x", ">\n>|> x");
    }

    #[test]
    fn rtrim_prev_line_deletes_the_blanks_the_line_break_leaves() {
        let rtrim = InputOptions {
            rtrim_prev_line: true,
            ..Default::default()
        };
        with(&rtrim, '\r', "ab \t\u{3000}|cd", "ab\n|cd");
        // an indented line that holds nothing else is emptied: the new line has the indentation
        with(&rtrim, '\r', "x\n  |y", "x\n\n  |y");
        // nothing to delete, and the first line
        with(&rtrim, '\r', "ab|cd", "ab\n|cd");
        with(&rtrim, '\r', "|ab", "\n|ab");
        // a bare CR and the LF of the emptied line join; the caret stays before the text
        with(&rtrim, '\n', "a\r  |b\nnext", "a\r\n  |b\nnext");
        // other chars are not line breaks
        with(&rtrim, 'x', "ab |cd", "ab x|cd");
        // off by default
        with(&InputOptions::default(), '\r', "ab |cd", "ab \n|cd");
        // one undo step
        let mut e = Editor::new("ab  |cd")
            .with_new_line(Eol::Lf)
            .with_input_options(rtrim);
        e.run_with(ID, &[Arg::Char('\r')]).assert_marked("ab\n|cd");
        e.run("undo").assert_marked("ab  |cd");
    }

    #[test]
    fn smart_indent_none_adds_no_level() {
        with(&InputOptions::default(), '\r', "if (x) {|", "if (x) {\n|");
        with(&InputOptions::default(), '}', "{\n\t|", "{\n\t}|");
    }

    #[test]
    fn smart_indent_cpp_indents_after_an_opening_bracket() {
        with(&cpp(), '\r', "if (x) {|", "if (x) {\n\t|");
        with(&cpp(), '\r', "\tif (x) {|", "\tif (x) {\n\t\t|");
        with(&cpp(), '\r', "f(|", "f(\n\t|");
        // no bracket open: the indentation that auto indent copied stays
        with(&cpp(), '\r', "\tfoo();|", "\tfoo();\n\t|");
        with(&cpp(), '\r', "foo();|", "foo();\n|");
        // the bracket was closed
        with(&cpp(), '\r', "{ foo(); }|", "{ foo(); }\n|");
        // the nearest open bracket decides
        let nested = "void f() {\n\tif (x) {\n\t\ty();|";
        with(
            &cpp(),
            '\r',
            nested,
            "void f() {\n\tif (x) {\n\t\ty();\n\t\t|",
        );
        with(
            &cpp(),
            '\r',
            "void f() {\n\tg(a,|",
            "void f() {\n\tg(a,\n\t\t|",
        );
    }

    #[test]
    fn smart_indent_cpp_reacts_to_cr_only() {
        // `SmartIndent_CPP` returns unless the char is `WCODE::CR` or one of `:{}()`: an LF
        // (`F_CTRL_CODE` 10) gets the copied indentation and no extra level.
        with(&cpp(), '\n', "if (x) {|", "if (x) {\n|");
        with(&cpp(), '\n', "\tif (x) {|", "\tif (x) {\n\t|");
    }

    #[test]
    fn smart_indent_cpp_puts_a_closing_bracket_behind_a_line_break_with_its_opener() {
        with(&cpp(), '\r', "\tvoid f() {|}", "\tvoid f() {\n\t|}");
        with(&cpp(), '\r', "void f() {|}", "void f() {\n|}");
        // blanks behind the caret go with the replaced indentation
        with(&cpp(), '\r', "if (x) {| }", "if (x) {\n|}");
    }

    #[test]
    fn smart_indent_cpp_aligns_a_typed_closing_bracket() {
        with(
            &cpp(),
            '}',
            "if (x) {\n\tfoo();\n\t|",
            "if (x) {\n\tfoo();\n}|",
        );
        with(&cpp(), ')', "f(a,\n\t\t|", "f(a,\n)|");
        // the brackets in between count
        let deep = "void f() {\n\tif (x) {\n\t\ty;\n\t\t|";
        with(&cpp(), '}', deep, "void f() {\n\tif (x) {\n\t\ty;\n\t}|");
        let closed = "void f() {\n\tif (x) {\n\t\ty;\n\t}\n\t|";
        with(
            &cpp(),
            '}',
            closed,
            "void f() {\n\tif (x) {\n\t\ty;\n\t}\n}|",
        );
        // no opener: the indentation goes
        with(&cpp(), '}', "\t\t|", "}|");
        // text before the bracket: left alone
        with(&cpp(), '}', "if (x) {\n\tfoo(); |", "if (x) {\n\tfoo(); }|");
    }

    #[test]
    fn smart_indent_cpp_moves_a_typed_opening_bracket_to_the_level() {
        let before = "void f() {\n\tif (x)\n\t\t|";
        with(&cpp(), '{', before, "void f() {\n\tif (x)\n\t{|");
        // a bracket that follows text stays where it is
        with(&cpp(), '{', "if (x) |", "if (x) {|");
        // a preprocessor line
        with(&cpp(), '(', "#define X|", "#define X(|");
        // inside a string or a comment
        with(&cpp(), '(', "\"a|\"", "\"a(|\"");
        with(&cpp(), '(', "foo(); // a|\nx", "foo(); // a(|\nx");
        // ... also on the last line, and in a literal that starts at column 0
        with(&cpp(), '(', "{\n\t// a|", "{\n\t// a(|");
        with(&cpp(), '(', "{\n\"abc|", "{\n\"abc(|");
        // a string right behind the char does not hold it
        with(&cpp(), '(', "{\n\t\t|\"a\"", "{\n\t(|\"a\"");
        // a bracket in a char literal does not count
        let literal = "void f() {\n\tchar c = '{';\n\t|";
        with(&cpp(), '}', literal, "void f() {\n\tchar c = '{';\n}|");
    }

    #[test]
    fn smart_indent_cpp_aligns_labels() {
        with(&cpp(), ':', "class A {\n\t\tpublic|", "class A {\npublic:|");
        let case = "switch (x) {\n\t\tcase 1|";
        with(&cpp(), ':', case, "switch (x) {\ncase 1:|");
        let default = "switch (x) {\n\t\tdefault|";
        with(&cpp(), ':', default, "switch (x) {\ndefault:|");
        // not a label
        with(&cpp(), ':', "\ta|", "\ta:|");
        with(&cpp(), ':', "\t|", "\t:|");
    }

    #[test]
    fn smart_indent_cpp_uses_spaces_when_the_tab_key_does() {
        let soft = |before: &str, after: &str| {
            let mut e = Editor::new(before)
                .with_new_line(Eol::Lf)
                .with_tab_width(4)
                .with_ins_space(true)
                .with_input_options(cpp());
            e.run_with(ID, &[Arg::Char('\r')]).assert_marked(after);
        };
        soft("if (x) {|", "if (x) {\n    |");
        // up to the next tab stop, counting a tab as the tab stop does
        soft("  if (x) {|", "  if (x) {\n    |");
        soft("\tif (x) {|", "\tif (x) {\n\t    |");
        soft(" \tif (x) {|", " \tif (x) {\n \t    |");
    }

    #[test]
    fn smart_indent_cpp_is_one_undo_step_with_the_typed_char() {
        let mut e = Editor::new("if (x) {|")
            .with_new_line(Eol::Lf)
            .with_input_options(cpp());
        e.run_with(ID, &[Arg::Char('\r')])
            .assert_marked("if (x) {\n\t|");
        e.run("undo").assert_marked("if (x) {|");
        let mut e = Editor::new("{\n\t|")
            .with_new_line(Eol::Lf)
            .with_input_options(cpp());
        e.run_with(ID, &[Arg::Char('}')]).assert_marked("{\n}|");
        e.run("undo").assert_marked("{\n\t|");
    }

    #[test]
    fn ctrl_code_runs_the_smart_indent_too() {
        // `F_CTRL_CODE` is `Command_WCHAR( c, false )`
        let mut e = Editor::new("{\n\t|")
            .with_new_line(Eol::Lf)
            .with_input_options(cpp());
        e.run_with("ctrl_code", &[Arg::Int(i64::from(b'}'))])
            .assert_marked("{\n}|");
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
