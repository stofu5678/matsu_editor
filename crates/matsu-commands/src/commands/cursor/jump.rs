// Ported in part from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/cmd/CViewCommander_Bookmark.cpp (Command_JUMP)
//   sakura_core/macro/CMacro.cpp (F_JUMP)
//
// sakura_core/cmd/CViewCommander_Bookmark.cpp:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, genta
//   Copyright (C) 2002, hor, YAZAKI, MIK
//   Copyright (C) 2006, genta
//   Copyright (C) 2018-2022, Sakura Editor Organization
// sakura_core/macro/CMacro.cpp:
//   Copyright (C) 2002, YAZAKI, genta, aroka, hor, MIK
//   Copyright (C) 2003, MIK, genta, Moca
//   Copyright (C) 2004, genta, Moca, zenryaku
//   Copyright (C) 2005, FILE, genta, maru, MIK
//   Copyright (C) 2006, genta, ryoji, かろと
//   Copyright (C) 2007, ryoji, genta, maru, nasukoji
//   Copyright (C) 2008, nasukoji
//   Copyright (C) 2009, ryoji
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (the layout manager,
// `MoveCursorSelecting` and UTF-16 buffers) was rewritten in Rust on the `Layout` of this crate,
// the dialog values that a macro does not give (the first line of the PL/SQL block and the
// number of the first valid line) are fixed at their defaults, and the tests are new.
//
// zlib License
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

//! `F_JUMP`: goes to a line (指定行へジャンプ). The macro is `Jump(line, option)`; the dialog
//! `F_JUMP_DIALOG` runs the same code.
//!
//! Sakura: `CViewCommander::Command_JUMP` (`sakura_core/cmd/CViewCommander_Bookmark.cpp`); the
//! arguments are read in `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`, "指定行へジャンプ").
//!
//! The option is the sum of
//!
//! * 0x01: the line number counts lines (up to line breaks); without it, it counts the rows of
//!   the layout (wrapped lines are several rows). The default of the macro is 1;
//! * 0x02: PL/SQL: the number counts only the valid lines of the source, from the start of the
//!   text. Blank lines are not valid, nor is a blank line inside a `/* */` comment; a `--` comment
//!   line is (the rest of the line is not read), a line with `/* */` is, and text in quotes goes
//!   on over line ends. (The dialog lets the user choose the line that the count starts from; a
//!   macro cannot, and it is the first line.) A comment that starts on the first line of the text
//!   is not counted: Sakura tests `nCommentBegin != 0` where it means "has begun".
//!
//! A line number below 1 is the first line and one past the end is the last. The caret goes to
//! the start of the line and the place is put in the jump history first. While the selection is
//! locked (`F_BEGIN_SEL`) the selection follows the caret, else it is cancelled.

use matsu_core::buffer::Pos;
use matsu_core::encoding::Eol;

use crate::command::{Arg, Command, CommandError, Result};
use crate::context::CommandContext;
use crate::register_command;

/// The option when the macro leaves it out.
pub const DEFAULT_OPTION: i64 = 0x01;

#[derive(Default)]
pub struct Jump;

impl Command for Jump {
    const ID: &'static str = "jump";
    const SAKURA: &'static str = "F_JUMP";

    fn run(&self, cx: &mut CommandContext<'_>, args: &[Arg]) -> Result {
        // `Argument[0] == nullptr` is an error box in Sakura (`STR_ERR_DLGMACRO05`).
        let line = number(args.first())
            .ok_or_else(|| CommandError::BadArgs("Jump needs a line number".to_owned()))?;
        let option = args
            .get(1)
            .map_or(Some(DEFAULT_OPTION), |a| number(Some(a)));
        let option = option.ok_or_else(|| {
            CommandError::BadArgs("the option of Jump is not a number".to_owned())
        })?;
        jump_to_line(cx, line, option)
    }
}

register_command!(Jump);

/// An argument as a number: an integer, or text that `_wtoi` reads.
fn number(arg: Option<&Arg>) -> Option<i64> {
    match arg? {
        Arg::Int(n) => Some(*n),
        Arg::Text(text) => Some(atoi(text)),
        Arg::Char(_) => None,
    }
}

/// `_wtoi`: leading white space, a sign and digits; anything else is 0.
fn atoi(text: &str) -> i64 {
    let t = text.trim_start();
    let (sign, digits) = match t.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, t.strip_prefix('+').unwrap_or(t)),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    sign * digits[..end].parse::<i64>().unwrap_or(0)
}

/// `Command_JUMP`: goes to line `number` as `option` says.
pub fn jump_to_line(cx: &mut CommandContext<'_>, number: i64, option: i64) -> Result {
    // no lines in the layout: `ErrorBeep`
    if cx.doc().line_count() == 0 {
        cx.beep();
        return Ok(());
    }
    let target = if option & 0x02 != 0 {
        plsql_line(cx, number)?
    } else if option & 0x01 != 0 {
        line_start(cx, number)?
    } else {
        row_start(cx, number)?
    };
    cx.add_to_jump_history();
    let extend = cx.extends(false);
    cx.move_caret(target, extend)
}

/// The start of line `number` (from 1) counted by line breaks. The row after a final line break
/// is a line, as Sakura's last (empty) line is.
fn line_start(cx: &CommandContext<'_>, number: i64) -> Result<Pos> {
    let lines = cx.doc().row_count();
    let line = number.max(1).saturating_sub(1);
    let line = usize::try_from(line).unwrap_or(usize::MAX).min(lines - 1);
    Ok(Pos::new(line, 0))
}

/// The start of row `number` (from 1) of the layout; past the last row is the last row.
fn row_start(cx: &CommandContext<'_>, number: i64) -> Result<Pos> {
    let row = usize::try_from(number.max(1) - 1).unwrap_or(usize::MAX);
    if let Some(pos) = cx.layout().row_start_at(cx.doc(), row)? {
        return Ok(pos);
    }
    let end = cx.doc().end_pos()?;
    Ok(cx.layout().row_start(cx.doc(), end)?)
}

/// The `Command_JUMP` branch for PL/SQL: the start of the line that holds the `number`th valid
/// line, counting from the first line of the text (the dialog's defaults).
fn plsql_line(cx: &CommandContext<'_>, number: i64) -> Result<Pos> {
    let wanted = number.max(1);
    let doc = cx.doc();
    let count = doc.line_count();
    // 0: normal, 8: in a comment, 20: in '...', 21: in "..."
    let mut mode = 0;
    let mut comment_begin = 0;
    let mut valid_lines = 0;
    let mut at = 0;
    while at < count {
        let Some(line) = doc.line(at)? else {
            break;
        };
        let mut text: Vec<char> = line.text.chars().collect();
        if let Some(eol) = line.eol {
            text.extend(eol_text(eol).chars());
        }
        let len = text.len();
        let begin = text
            .iter()
            .position(|&c| c != ' ' && c != '\t')
            .unwrap_or(len);
        let mut valid = false;
        let mut i = begin;
        while i < len {
            let c = text[i];
            let next = text.get(i + 1).copied();
            let escaped = i > 0 && text[i - 1] == '\\';
            match mode {
                20 | 21 => {
                    valid = true;
                    let quote = if mode == 20 { '\'' } else { '"' };
                    if c == quote && !escaped {
                        mode = 0;
                    }
                }
                8 => {
                    if c == '*' && next == Some('/') {
                        if comment_begin != 0 {
                            valid = true;
                        }
                        i += 1;
                        mode = 0;
                    }
                }
                _ => {
                    if c == '\t' || c == ' ' || is_line_delimiter(c) {
                        // skipped
                    } else if c == '-' && next == Some('-') {
                        valid = true;
                        break;
                    } else if c == '/' && next == Some('*') {
                        i += 1;
                        mode = 8;
                        comment_begin = at;
                    } else if c == '\'' {
                        mode = 20;
                    } else if c == '"' {
                        mode = 21;
                    } else {
                        valid = true;
                    }
                }
            }
            i += 1;
        }
        if mode == 8 {
            if comment_begin != 0 {
                valid = true;
            }
            // a line that is only a line break inside a comment
            if text.get(begin).is_some_and(|&c| is_line_delimiter(c)) {
                valid = false;
            }
        }
        if valid {
            valid_lines += 1;
            if valid_lines >= wanted {
                break;
            }
        }
        at += 1;
    }
    // Past the last line is the last line.
    Ok(Pos::new(at.min(count - 1), 0))
}

fn eol_text(eol: Eol) -> &'static str {
    eol.as_str()
}

/// `WCODE::IsLineDelimiter` without the extended line ends.
fn is_line_delimiter(c: char) -> bool {
    c == '\r' || c == '\n'
}

#[cfg(test)]
mod tests {
    use matsu_core::buffer::Document;
    use matsu_core::cursor::Cursor;
    use matsu_core::model::DocumentModel;

    use super::*;
    use crate::layout::Layout;
    use crate::registry::Registry;
    use crate::testing::{Editor, check_with};

    fn jump(before: &str, args: &[Arg], after: &str) {
        check_with("jump", args, before, after);
    }

    fn at(n: i64) -> [Arg; 1] {
        [Arg::Int(n)]
    }

    #[test]
    fn goes_to_the_start_of_the_line() {
        jump("ab|c\ndef\nghi", &at(2), "abc\n|def\nghi");
        jump("|abc\ndef\nghi", &at(3), "abc\ndef\n|ghi");
        // CRLF lines count as one
        jump("|a\r\nb\r\nc", &at(2), "a\r\n|b\r\nc");
        // Japanese text and a wide caret
        jump("日本語|\nテキスト", &at(2), "日本語\n|テキスト");
    }

    #[test]
    fn a_number_out_of_range_goes_to_the_first_or_the_last_line() {
        jump("a\nb|\nc", &at(0), "|a\nb\nc");
        jump("a\nb|\nc", &at(-5), "|a\nb\nc");
        jump("a|\nb\nc", &at(99), "a\nb\n|c");
        // the empty row after a final line break is a line (Sakura's last line is empty)
        jump("a|\nb\n", &at(99), "a\nb\n|");
        jump("a|\nb\n", &at(3), "a\nb\n|");
    }

    #[test]
    fn the_option_defaults_to_line_breaks_and_text_is_read_like_wtoi() {
        jump("|a\nb", &[Arg::Text(" 2x".into())], "a\n|b");
        jump("|a\nb", &[Arg::Text("zz".into())], "|a\nb");
        jump("|a\nb\nc", &[Arg::Int(3), Arg::Int(1)], "a\nb\n|c");
        jump(
            "|a\nb\nc",
            &[Arg::Int(3), Arg::Text("1".into())],
            "a\nb\n|c",
        );
    }

    #[test]
    fn a_missing_or_wrong_argument_is_an_error() {
        let mut e = Editor::new("a|");
        assert!(matches!(
            e.try_run("jump", &[]),
            Err(CommandError::BadArgs(_))
        ));
        assert!(matches!(
            e.try_run("jump", &[Arg::Char('x')]),
            Err(CommandError::BadArgs(_))
        ));
        assert!(matches!(
            e.try_run("jump", &[Arg::Int(1), Arg::Char('x')]),
            Err(CommandError::BadArgs(_))
        ));
    }

    #[test]
    fn an_empty_document_beeps() {
        let mut e = Editor::new("|");
        e.run_with("jump", &at(1));
        assert!(e.changes().beep);
    }

    #[test]
    fn a_selection_is_cancelled_unless_it_is_locked() {
        jump("a[bc\nd|ef\nghi", &at(3), "abc\ndef\n|ghi");
        let mut e = Editor::new("a|bc\ndef\nghi");
        e.run("begin_sel");
        e.run_with("jump", &at(3));
        e.assert_marked("a[bc\ndef\n|ghi");
    }

    #[test]
    fn the_place_is_put_in_the_jump_history() {
        let mut e = Editor::new("a\nb|c\nd");
        e.run_with("jump", &at(3));
        e.run("jumphist_prev");
        e.assert_marked("a\nb|c\nd");
    }

    /// The line that `Jump(number, 2)` goes to in `source`.
    fn plsql(source: &str, number: i64) -> usize {
        let mut model =
            DocumentModel::new(Document::from_utf8(source.as_bytes().to_vec()).unwrap());
        let mut cursor = Cursor::default();
        let mut layout = crate::layout::PlainLayout::default();
        let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
        jump_to_line(&mut cx, number, 2).unwrap();
        cursor.caret().line
    }

    #[test]
    fn plsql_counts_valid_lines_only() {
        let source = "-- header\n\nCREATE PROCEDURE p IS\n/* note\n\n   more */\nBEGIN\n  NULL; -- x\nEND;\n";
        // blank lines are skipped; a `--` line, and the lines of a comment, count
        assert_eq!(plsql(source, 1), 0);
        assert_eq!(plsql(source, 2), 2);
        assert_eq!(plsql(source, 3), 3);
        // the blank line inside the comment is not valid
        assert_eq!(plsql(source, 4), 5);
        assert_eq!(plsql(source, 5), 6);
        assert_eq!(plsql(source, 7), 8);
        // below 1 is the first valid line; past the last valid line is the last line
        assert_eq!(plsql(source, 0), 0);
        assert_eq!(plsql(source, 99), 8);
        // the leading blank lines are skipped
        assert_eq!(plsql("\n  \t\nx\n", 1), 2);
    }

    #[test]
    fn plsql_leaves_out_a_comment_that_begins_on_the_first_line() {
        // `nCommentBegin != 0` in Command_JUMP: a block comment at the very start is not valid
        assert_eq!(plsql("/* a\n b */\nx\n", 1), 2);
        assert_eq!(plsql("y\n/* a\n b */\nx\n", 2), 1);
    }

    #[test]
    fn plsql_text_in_quotes_goes_on_over_line_ends() {
        let src = "a\n'x\ny'\nb\n";
        // line 1 `a`; line 2 holds the quote that starts; line 3 is inside the quote (valid);
        // line 4 `b`
        jump(
            &format!("|{src}"),
            &[Arg::Int(4), Arg::Int(2)],
            "a\n'x\ny'\n|b\n",
        );
        jump(
            &format!("|{src}"),
            &[Arg::Int(3), Arg::Int(3)],
            "a\n'x\n|y'\nb\n",
        );
    }

    // ---- wrapped rows ----

    /// A layout that wraps every line after 3 ASCII chars.
    struct Wrap;

    impl Wrap {
        fn rows(doc: &Document) -> Vec<Pos> {
            let mut rows = Vec::new();
            for line in 0..doc.line_count() {
                let len = doc.line_len(line).unwrap().unwrap_or(0);
                let mut col = 0;
                loop {
                    rows.push(Pos::new(line, col));
                    col += 3;
                    if col >= len {
                        break;
                    }
                }
            }
            if doc.end_pos().unwrap().line >= doc.line_count() {
                rows.push(Pos::new(doc.line_count(), 0));
            }
            rows
        }
    }

    type R<T> = matsu_core::buffer::Result<T>;

    impl Layout for Wrap {
        fn edited(&mut self, _: &Document, _: &crate::context::TextChange) {}
        fn reset(&mut self, _: &Document) {}
        fn tab_width(&self) -> usize {
            4
        }
        fn display_col(&self, doc: &Document, pos: Pos) -> R<usize> {
            Ok(pos.col - self.row_start(doc, pos)?.col)
        }
        fn row_start(&self, doc: &Document, pos: Pos) -> R<Pos> {
            let pos = doc.clamp_pos(pos)?;
            Ok(Self::rows(doc)
                .into_iter()
                .rfind(|&r| r <= pos)
                .unwrap_or_default())
        }
        fn row_end(&self, doc: &Document, pos: Pos) -> R<Pos> {
            let start = self.row_start(doc, pos)?;
            let len = doc.line_len(start.line)?.unwrap_or(0);
            Ok(Pos::new(start.line, (start.col + 3).min(len)))
        }
        fn next_row_start(&self, doc: &Document, pos: Pos) -> R<Option<Pos>> {
            let start = self.row_start(doc, pos)?;
            Ok(Self::rows(doc).into_iter().find(|&r| r > start))
        }
        fn pos_at_col(&self, doc: &Document, row_pos: Pos, col: usize) -> R<Pos> {
            let start = self.row_start(doc, row_pos)?;
            Ok(Pos::new(start.line, start.col + col))
        }
        fn move_rows(&self, _: &Document, pos: Pos, _: isize, _: usize) -> R<Pos> {
            Ok(pos)
        }
    }

    fn wrapped(text: &str, number: i64, option: i64) -> Pos {
        let mut model = DocumentModel::new(Document::from_utf8(text.as_bytes().to_vec()).unwrap());
        let mut cursor = Cursor::default();
        let mut layout = Wrap;
        let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
        Registry::global()
            .run("jump", &mut cx, &[Arg::Int(number), Arg::Int(option)])
            .unwrap();
        cursor.caret()
    }

    #[test]
    fn without_the_line_break_bit_the_number_counts_rows_of_the_layout() {
        // "abcdefgh" is the rows "abc", "def", "gh"; "ij" is row 4
        let text = "abcdefgh\nij";
        assert_eq!(wrapped(text, 2, 0), Pos::new(0, 3));
        assert_eq!(wrapped(text, 3, 0), Pos::new(0, 6));
        assert_eq!(wrapped(text, 4, 0), Pos::new(1, 0));
        // below 1 and past the end
        assert_eq!(wrapped(text, 0, 0), Pos::new(0, 0));
        assert_eq!(wrapped(text, 99, 0), Pos::new(1, 0));
        // with the bit it counts lines, as the default does
        assert_eq!(wrapped(text, 2, 1), Pos::new(1, 0));
    }

    #[test]
    fn the_sakura_function_is_this_command() {
        assert_eq!(
            Registry::global().get("jump").map(|c| c.sakura),
            Some("F_JUMP")
        );
    }
}
