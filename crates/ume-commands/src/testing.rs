//! Tests of commands with the caret and the selection written inside the text.
//!
//! # The notation
//!
//! A *marked text* is the text of a document with two kinds of marks in it:
//!
//! | mark | meaning |
//! |------|---------|
//! | `\|` | the caret. Exactly one in every marked text. |
//! | `[`  | the anchor of a selection (the end that does not move). At most one. |
//!
//! * `"ab\|cd"`: the caret is between `b` and `c`; nothing is selected.
//! * `"a[bc\|d"`: `bc` is selected, and the caret is at its end (the selection was made to the
//!   right). `"a\|bc[d"` is the same text selected from the other end (to the left).
//! * Line endings are the real characters: `"ab\r\n\|cd"` is CRLF, `"ab\n\|cd"` is LF. The
//!   marks cannot stand between the CR and the LF of one CRLF.
//! * The end of the file is just the end of the string: `"ab\|"` has the caret at the end of the
//!   last line, `"ab\n\|"` on the empty line after a final line ending, and `"\|"` is an empty
//!   document.
//! * The text itself cannot contain `|` or `[`. A test that needs them builds the editor with
//!   [`Editor::from_parts`].
//!
//! A test says "this command turns that into this":
//!
//! ```ignore
//! check("delete_back", "ab|cd", "a|cd");
//! check("cursor_left_sel", "ab|cd", "a|b[cd");
//! check("cursor_right", "a|bc[d", "abc|d");
//! ```
//!
//! For more than one step, or to look at [`Changes`], use [`Editor`]:
//!
//! ```ignore
//! let mut e = Editor::new("ab|cd");
//! e.run("delete").run("undo").assert_marked("ab|cd");
//! assert!(e.changes().text_changed());
//! ```
//!
//! Commands are run through the [registry](crate::registry) by ID, so every test also checks that
//! the command is registered. A failure prints both marked texts with the line endings escaped.
//! (The mark `|` is also what `{:?}` would show escaped; there is no ambiguity because the text
//! cannot contain it.)

use std::path::PathBuf;

use ume_core::buffer::{Document, Eol, Pos};
use ume_core::cursor::Cursor;
use ume_core::model::{DocumentModel, eol_text};

use crate::clipboard::{ClipData, ClipKind, MemoryClipboard};
use crate::command::{Arg, CommandError};
use crate::context::{Changes, CommandContext};
use crate::layout::PlainLayout;
use crate::registry::Registry;

/// A document and a cursor, as written in a marked text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Marked {
    pub text: String,
    pub caret: Pos,
    pub anchor: Option<Pos>,
}

const CARET: char = '|';
const ANCHOR: char = '[';

/// Reads a marked text. See the [module documentation](self) for the notation.
pub fn parse(marked: &str) -> Result<Marked, String> {
    let mut text = String::new();
    let (mut caret, mut anchor) = (None, None);
    for c in marked.chars() {
        let slot = match c {
            CARET => &mut caret,
            ANCHOR => &mut anchor,
            _ => {
                text.push(c);
                continue;
            }
        };
        if slot.replace(text.len()).is_some() {
            return Err(format!("{c:?} appears twice in {marked:?}"));
        }
    }
    let caret = caret.ok_or_else(|| format!("no caret mark {CARET:?} in {marked:?}"))?;
    let anchor = anchor.map(|off| pos_of(&text, off)).transpose()?;
    let caret = pos_of(&text, caret)?;
    Ok(Marked {
        text,
        caret,
        anchor,
    })
}

/// The position of the byte offset `off` in `text`.
fn pos_of(text: &str, off: usize) -> Result<Pos, String> {
    let (mut line, mut start, mut i) = (0, 0, 0);
    while let Some(c) = text[i..].chars().next() {
        let len = match c {
            '\r' if text[i + 1..].starts_with('\n') => 2,
            '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}' => c.len_utf8(),
            _ => {
                i += c.len_utf8();
                continue;
            }
        };
        if off > i && off < i + len {
            return Err(format!("a mark is inside a CRLF in {text:?}"));
        }
        if off <= i {
            break;
        }
        line += 1;
        start = i + len;
        i += len;
    }
    Ok(Pos::new(line, off - start))
}

/// Writes the document and the cursor as a marked text.
pub fn render(doc: &Document, cursor: &Cursor) -> String {
    let (text, offsets) = {
        let mut text = String::new();
        let mut line_starts = Vec::new();
        for line in doc.lines(0..doc.line_count()) {
            let line = line.expect("a line of the test document");
            line_starts.push(text.len());
            text.push_str(&line.text);
            if let Some(eol) = line.eol {
                text.push_str(eol_text(eol));
            }
        }
        // the empty line after a final line ending (or the only line of an empty document)
        line_starts.push(text.len());
        let offset = |p: Pos| line_starts[p.line.min(line_starts.len() - 1)] + p.col;
        let offsets = (offset(cursor.caret()), cursor.anchor().map(offset));
        (text, offsets)
    };
    let mut marks = vec![(offsets.0, CARET)];
    marks.extend(offsets.1.map(|a| (a, ANCHOR)));
    marks.sort_by_key(|&(off, _)| std::cmp::Reverse(off));
    let mut out = text;
    for (off, mark) in marks {
        out.insert(off.min(out.len()), mark);
    }
    out
}

/// The whole text of the document, for comparing.
fn text_of(doc: &Document) -> String {
    render(doc, &Cursor::default())
        .strip_prefix(CARET)
        .expect("the caret of a default cursor is first")
        .to_owned()
}

/// A document, a cursor and a layout to run commands on.
pub struct Editor {
    model: DocumentModel,
    cursor: Cursor,
    layout: PlainLayout,
    path: Option<PathBuf>,
    clipboard: MemoryClipboard,
    changes: Changes,
    overwrite: bool,
    ins_space: bool,
    view_mode: bool,
    /// The quote string that `F_SET_QUOTESTRING` set (the window keeps it between commands).
    quote: Option<String>,
}

impl Editor {
    /// An editor on a marked text (see the [module documentation](self)). Panics if the
    /// notation is wrong. The line ending of Enter is the first one in the text, or CRLF.
    pub fn new(marked: &str) -> Self {
        let m = parse(marked).unwrap_or_else(|e| panic!("bad marked text: {e}"));
        let mut cursor = Cursor::default();
        cursor.select(m.anchor.unwrap_or(m.caret), m.caret);
        Self::from_parts(&m.text, cursor)
    }

    /// An editor on `text` (which may contain `|` and `[`) with a given cursor.
    pub fn from_parts(text: &str, cursor: Cursor) -> Self {
        let doc =
            Document::from_utf8_with(text.as_bytes().to_vec(), true).expect("a test document");
        Self {
            model: DocumentModel::new(doc),
            cursor,
            layout: PlainLayout::default(),
            path: None,
            clipboard: MemoryClipboard::default(),
            changes: Changes::default(),
            overwrite: false,
            ins_space: false,
            view_mode: false,
            quote: None,
        }
    }

    /// Sets view mode: every edit is refused (`F_VIEWMODE`).
    pub fn set_view_mode(&mut self, view_mode: bool) -> &mut Self {
        self.view_mode = view_mode;
        self
    }

    /// Sets the typing mode: overwrite or insert (the default). Commands that switch the mode
    /// change it for the next ones.
    pub fn set_overwrite(&mut self, overwrite: bool) -> &mut Self {
        self.overwrite = overwrite;
        self
    }

    /// Whether the editor is in overwrite mode.
    pub fn overwrite(&self) -> bool {
        self.overwrite
    }

    /// Sets the tab width of the layout (4 by default).
    #[must_use]
    pub fn with_tab_width(mut self, tab_width: usize) -> Self {
        self.layout.tab_width = tab_width;
        self
    }

    /// Sets whether the Tab key inserts spaces (`bInsSpace`).
    #[must_use]
    pub fn with_ins_space(mut self, ins_space: bool) -> Self {
        self.ins_space = ins_space;
        self
    }

    /// Sets the line ending that Enter inserts.
    #[must_use]
    pub fn with_new_line(mut self, eol: Eol) -> Self {
        self.model.set_new_line(eol);
        self
    }

    /// Sets the path of the document (for the commands that copy it).
    #[must_use]
    pub fn with_path(mut self, path: &str) -> Self {
        self.path = Some(PathBuf::from(path));
        self
    }

    /// Puts `text` on the clipboard as `kind`.
    #[must_use]
    pub fn with_clipboard(mut self, text: &str, kind: ClipKind) -> Self {
        use crate::clipboard::Clipboard;
        self.clipboard.set(ClipData::new(text, kind));
        self
    }

    /// The content of the clipboard.
    pub fn clipboard(&self) -> Option<&ClipData> {
        self.clipboard.data()
    }

    /// Runs the command `id` with no arguments. Panics if the command fails.
    pub fn run(&mut self, id: &str) -> &mut Self {
        self.run_with(id, &[])
    }

    /// Runs the command `id` with arguments. Panics if the command fails.
    pub fn run_with(&mut self, id: &str, args: &[Arg]) -> &mut Self {
        self.try_run(id, args)
            .unwrap_or_else(|e| panic!("command {id} failed: {e}"));
        self
    }

    /// Runs the command `id` with arguments and returns its error, to test the errors.
    pub fn try_run(&mut self, id: &str, args: &[Arg]) -> Result<(), CommandError> {
        let mut cx = CommandContext::new(
            &mut self.model,
            &mut self.cursor,
            &mut self.layout,
            self.path.as_deref(),
        )
        .with_clipboard(&mut self.clipboard)
        .with_overwrite(self.overwrite)
        .with_ins_space(self.ins_space)
        .with_view_mode(self.view_mode);
        if let Some(quote) = &self.quote {
            cx = cx.with_quote_string(quote.as_str());
        }
        let result = Registry::global().run(id, &mut cx, args);
        self.changes = cx.finish();
        self.overwrite = self.changes.overwrite;
        if let Some(quote) = &self.changes.quote_string {
            self.quote = Some(quote.clone());
        }
        result
    }

    /// Runs `f` on a context, for the methods that no command calls on its own (the ones that
    /// `ume-ui` uses). What it changed is in [`changes`](Self::changes) afterwards.
    pub fn with_context<R>(&mut self, f: impl FnOnce(&mut CommandContext<'_>) -> R) -> R {
        let mut cx = CommandContext::new(
            &mut self.model,
            &mut self.cursor,
            &mut self.layout,
            self.path.as_deref(),
        )
        .with_clipboard(&mut self.clipboard)
        .with_overwrite(self.overwrite)
        .with_ins_space(self.ins_space)
        .with_view_mode(self.view_mode);
        let result = f(&mut cx);
        self.changes = cx.finish();
        self.overwrite = self.changes.overwrite;
        result
    }

    /// Records that the document is saved as it is now (the window does it after a save).
    pub fn mark_saved(&mut self) -> &mut Self {
        self.model.mark_saved(&self.cursor);
        self
    }

    /// What the last command changed.
    pub fn changes(&self) -> &Changes {
        &self.changes
    }

    pub fn model(&self) -> &DocumentModel {
        &self.model
    }

    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// The corners of the box selection as `(line, display column)`: the anchor and the caret.
    /// `None` if there is no box selection (a box that covers nothing has both corners equal).
    pub fn box_state(&self) -> Option<((usize, usize), (usize, usize))> {
        let c = &self.cursor;
        let caret = (c.caret().line, c.box_caret_col()?);
        let anchor = (c.anchor().unwrap_or(c.caret()).line, c.box_anchor_col()?);
        Some((anchor, caret))
    }

    /// Checks the box selection: the corners as `(line, display column)`.
    #[track_caller]
    pub fn assert_box(&self, anchor: (usize, usize), caret: (usize, usize)) -> &Self {
        assert_eq!(
            self.box_state(),
            Some((anchor, caret)),
            "the corners (anchor, caret) of the box in {:?}",
            self.marked()
        );
        self
    }

    /// The document and the cursor as a marked text.
    pub fn marked(&self) -> String {
        render(self.model.doc(), &self.cursor)
    }

    /// Checks the document and the cursor against a marked text.
    #[track_caller]
    pub fn assert_marked(&self, expected: &str) -> &Self {
        let want = parse(expected).unwrap_or_else(|e| panic!("bad marked text: {e}"));
        let got = Marked {
            text: text_of(self.model.doc()),
            caret: self.cursor.caret(),
            anchor: self.cursor.anchor(),
        };
        assert!(
            got == want,
            "\n  expected: {expected:?}\n    actual: {:?}\n",
            self.marked()
        );
        self
    }
}

/// Runs the command `id` on `before` and checks that the result is `after` (both marked texts).
#[track_caller]
pub fn check(id: &str, before: &str, after: &str) {
    Editor::new(before).run(id).assert_marked(after);
}

/// Runs the command `id` on `before` (a marked text) and checks the corners of the box
/// selection that results, as `(line, display column)`.
#[track_caller]
pub fn check_box(id: &str, before: &str, anchor: (usize, usize), caret: (usize, usize)) {
    Editor::new(before).run(id).assert_box(anchor, caret);
}

/// Like [`check`], with arguments.
#[track_caller]
pub fn check_with(id: &str, args: &[Arg], before: &str, after: &str) {
    Editor::new(before).run_with(id, args).assert_marked(after);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_the_marks() {
        let m = parse("ab|cd").unwrap();
        assert_eq!(
            (m.text.as_str(), m.caret, m.anchor),
            ("abcd", Pos::new(0, 2), None)
        );
        let m = parse("a[bc|d").unwrap();
        assert_eq!((m.caret, m.anchor), (Pos::new(0, 3), Some(Pos::new(0, 1))));
        let m = parse("a|bc[d").unwrap();
        assert_eq!((m.caret, m.anchor), (Pos::new(0, 1), Some(Pos::new(0, 3))));
    }

    #[test]
    fn parse_counts_lines_and_bytes() {
        let m = parse("a\r\nb\nc日|").unwrap();
        assert_eq!(m.text, "a\r\nb\nc日");
        assert_eq!(m.caret, Pos::new(2, 4));
        // the empty line after a final line ending, and the empty document
        assert_eq!(parse("ab\n|").unwrap().caret, Pos::new(1, 0));
        assert_eq!(parse("|").unwrap().caret, Pos::new(0, 0));
        // the end of a line is before its CRLF; the start of the next line is after it
        assert_eq!(parse("ab|\r\ncd").unwrap().caret, Pos::new(0, 2));
        assert_eq!(parse("ab\r\n|cd").unwrap().caret, Pos::new(1, 0));
    }

    #[test]
    fn parse_rejects_bad_notation() {
        assert!(parse("abc").is_err());
        assert!(parse("a||b").is_err());
        assert!(parse("[a[b|").is_err());
        assert!(parse("a\r|\nb|").is_err());
    }

    #[test]
    fn render_is_the_inverse_of_parse() {
        for marked in [
            "ab|cd",
            "a[bc|d",
            "a|bc[d",
            "a\r\n|b\r\nc",
            "a\nb|\n",
            "a\nb\n|",
            "|",
            "[ab\n\n|",
            "日本[語|\r\n",
            "x\ry\u{2028}|z",
        ] {
            let m = parse(marked).unwrap();
            let e = Editor::new(marked);
            assert_eq!(e.marked(), marked);
            assert_eq!(text_of(e.model.doc()), m.text);
        }
    }

    #[test]
    fn assert_marked_compares_text_caret_and_direction() {
        Editor::new("a[bc|d").assert_marked("a[bc|d");
        let result = std::panic::catch_unwind(|| {
            Editor::new("a[bc|d").assert_marked("a|bc[d");
        });
        assert!(result.is_err(), "the direction of a selection matters");
        let result = std::panic::catch_unwind(|| {
            Editor::new("ab|cd").assert_marked("ab|ce");
        });
        assert!(result.is_err());
    }
}
