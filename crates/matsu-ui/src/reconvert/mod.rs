//! Reconversion (再変換, `F_RECONVERT`): what the IME is given to convert again, and the flow.
//!
//! Sakura: `CEditView::SetReconvertStruct` (`sakura_core/view/CEditView_Ime.cpp`) picks the text
//! around the caret and the range to convert; `CEditView::SetSelectionFromReonvert` selects the
//! range the IME settled on; `CViewCommander::Command_Reconvert`
//! (`sakura_core/cmd/CViewCommander_Edit_advanced.cpp`) runs the steps. The calls into the OS are
//! in `matsu_platform::reconvert`; this module chooses the text and the range, and selects the
//! result. [`plan`] is a function of plain values and is tested without a window.
//!
//! Sakura's `IMR_RECONVERTSTRING` (the IME asking for the structure itself, by a message of the
//! window) is not handled: GPUI does not pass that message on. Reconversion started from
//! the menu, a key or a macro (`F_RECONVERT`) does not need it.

use std::rc::Rc;

use gpui::{AsyncWindowContext, Context, WeakEntity, Window};
use matsu_core::buffer::{Document, Pos};
use matsu_platform::reconvert::{self, Error, Reconvert, Request};

use crate::view::EditorView;

#[cfg(test)]
mod tests;

/// How far from the start of the range the text handed over reaches, in UTF-16 units
/// (`nReconvMaxLen`, "$$マジックナンバー注意").
const REACH: usize = 200;

/// The bytes on each side of a range that are read of a very long line: the text handed over
/// reaches [`REACH`] units from the range, and a unit is at most 3 bytes.
pub(crate) const MARGIN: usize = 3 * REACH + 16;

/// What is handed to the IME and where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    /// The line (logical, from 0) the text is from. Reconversion stays in one line.
    pub line: usize,
    /// Where `request.text` starts in the window of the line (see [`LineWindow`]), in UTF-16 units
    /// (`nReconvIndex`). For a line that is not very long the window is the line.
    pub index: usize,
    /// The columns (bytes) that the window was asked for, to read the same window again.
    pub asked: (usize, usize),
    pub request: Request,
}

/// The part of a line that the plan works on: the line itself, or, for a very long line (ADR-0020,
/// 9.10) that is not decoded as a whole, the part around the range.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LineWindow {
    pub text: String,
    /// The column (bytes) of the line where `text` starts.
    pub base: usize,
    /// `text` reaches the end of the line.
    pub to_end: bool,
}

impl LineWindow {
    /// A window that is the whole line.
    pub(crate) fn whole(text: String) -> Self {
        Self {
            text,
            base: 0,
            to_end: true,
        }
    }

    /// The UTF-16 units of the window before the column `col` of the line (all of them when it is
    /// past the end or inside a char).
    fn units_before(&self, col: usize) -> usize {
        units_before(&self.text, col.saturating_sub(self.base))
    }
}

/// `SetReconvertStruct` for a reconversion: the text around the caret and the range to convert.
///
/// * `caret` and `selection` (start, end) are positions of the document (`col` is a byte offset
///   in the line, as everywhere in the core). `window` gives the window of a line for a range of
///   its columns `(line, from, to)`, `None` past the last line; it has to reach [`MARGIN`] bytes
///   on each side of the range, or the end of the line. A line that is not very long can be given
///   whole.
/// * Nothing selected: the caret alone, the IME takes the word before it. A selection in one
///   line: that range. A selection of several lines is cut to one line, the one the caret is in
///   (the next when the caret is at the end of the first line of the selection, the previous
///   when it is at the start of the last).
/// * The text is the line from at most 200 units before the range to at most 200 after it, cut at
///   the edge of a surrogate pair.
///
/// Returns `None` when Sakura asks the IME for nothing: the line is missing or empty.
pub(crate) fn plan(
    caret: Pos,
    selection: Option<(Pos, Pos)>,
    line_count: usize,
    window: &dyn Fn(usize, usize, usize) -> Option<LineWindow>,
) -> Option<Plan> {
    // the range as columns (bytes) of a line: `usize::MAX` is the end of the line
    let (y, from_col, to_col) = match selection {
        Some((start, end)) if start != end && start.line == end.line => {
            (start.line, start.col, end.col)
        }
        Some((start, end)) if start != end => {
            // limited to one logical line: the line of the caret
            // where the caret is in its line (a line that is not there is empty: both are true)
            let (at_end, at_start) = match window(caret.line, caret.col, caret.col) {
                Some(w) => {
                    let before = w.units_before(caret.col);
                    (
                        w.to_end && before == units(&w.text),
                        w.base == 0 && before == 0,
                    )
                }
                None => (true, true),
            };
            let mut target = caret.line;
            if start.line == caret.line && at_end {
                // the caret is at the end of the upper line: the next line
                target = (caret.line + 1).min(line_count);
            } else if end.line == caret.line && at_start {
                // the caret is at the start of the lower line: the previous line
                target = caret.line.checked_sub(1)?;
            }
            if target == start.line {
                (target, start.col, usize::MAX)
            } else if target == end.line {
                (target, 0, end.col)
            } else {
                (target, 0, usize::MAX)
            }
        }
        _ => (caret.line, caret.col, caret.col),
    };
    // a range of many units is cut to what is handed over anyway
    let to_col = to_col.min(from_col.saturating_add(MARGIN));
    let win = window(y, from_col, to_col)?;
    let units: Vec<u16> = win.text.encode_utf16().collect();
    let len = units.len();
    if len == 0 {
        return None;
    }
    let (mut from, mut to) = (win.units_before(from_col), win.units_before(to_col));
    // a caret past the end of the line (free cursor) hands over the end of the line
    if len < from {
        from = len;
        to = len;
    }
    let mut selected = to.saturating_sub(from).min(len - from);

    // `nReconvIndex`: the text starts at most 200 units before the range
    let mut index = 0;
    while from - index > REACH {
        index = next_boundary(&units, index);
    }
    // and ends at most 200 units after it, at the edge of a char
    let mut reconv_len = len - index;
    if reconv_len + index - from > REACH {
        let mut p = from;
        while p <= from + REACH {
            p = next_boundary(&units, p);
        }
        reconv_len = p.min(len) - index;
    }
    selected = selected.min(index + reconv_len - from);
    Some(Plan {
        line: y,
        index,
        asked: (from_col, to_col),
        request: Request {
            text: units[index..index + reconv_len].to_vec(),
            start: from - index,
            len: selected,
        },
    })
}

/// The window of line `y` for the columns `from..to`: the line, or, for a very long line, the part
/// from [`MARGIN`] bytes before `from` to [`MARGIN`] bytes after `to`, so that the line is not
/// decoded as a whole. `None` if there is no such line.
fn window_of(doc: &Document, y: usize, from: usize, to: usize) -> Option<LineWindow> {
    if !doc.is_long_line(y) {
        return doc
            .line(y)
            .ok()
            .flatten()
            .map(|l| LineWindow::whole(l.text));
    }
    let len = doc.line_len(y).ok().flatten()?;
    let slice = doc
        .line_slice(y, from.saturating_sub(MARGIN), to.saturating_add(MARGIN))
        .ok()
        .flatten()?;
    Some(LineWindow {
        to_end: slice.start_col + slice.text.len() >= len,
        base: slice.start_col,
        text: slice.text,
    })
}

/// The number of UTF-16 units of `text`.
fn units(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// The number of UTF-16 units of `text` before the byte offset `col` (all of them when it is
/// past the end or inside a char).
fn units_before(text: &str, col: usize) -> usize {
    text.get(..col)
        .unwrap_or(text)
        .chars()
        .map(char::len_utf16)
        .sum()
}

/// The byte offset of the char that starts at (or, inside a pair, contains) the unit `unit`.
fn byte_at(text: &str, unit: usize) -> usize {
    let mut seen = 0;
    for (byte, c) in text.char_indices() {
        if seen + c.len_utf16() > unit {
            return byte;
        }
        seen += c.len_utf16();
    }
    text.len()
}

/// The start of the char after the one at `i` (`CharNextW`): a surrogate pair is one char.
fn next_boundary(units: &[u16], i: usize) -> usize {
    let pair = (0xD800..0xDC00).contains(&units[i])
        && units
            .get(i + 1)
            .is_some_and(|low| (0xDC00..0xE000).contains(low));
    i + if pair { 2 } else { 1 }
}

/// The IME side of the flow: the calls into the OS, which a test replaces.
pub(crate) trait Ime {
    fn query(&self, reconvert: &mut Reconvert) -> Result<(), Error>;
    fn set(&self, reconvert: &Reconvert) -> Result<(), Error>;
}

/// The IME of the OS for one window.
pub(crate) struct OsIme(pub reconvert::RawWindowHandle);

impl Ime for OsIme {
    fn query(&self, reconvert: &mut Reconvert) -> Result<(), Error> {
        reconvert::query(self.0, reconvert)
    }

    fn set(&self, reconvert: &Reconvert) -> Result<(), Error> {
        reconvert::set(self.0, reconvert)
    }
}

impl EditorView {
    /// `Command_Reconvert`: asks the IME to convert the text at the caret again. Nothing happens
    /// where Sakura asks for nothing (a box selection, view mode, an empty line).
    pub(crate) fn start_reconvert(
        &mut self,
        ime: Rc<dyn Ime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // `SetReconvertStruct`: not with a box selected, not in view mode
        if self.cursor.is_box() || self.document.read(cx).view_mode() {
            return;
        }
        let doc = self.document.read(cx).doc();
        let windows = |y: usize, from: usize, to: usize| window_of(doc, y, from, to);
        let Some(plan) = plan(
            self.cursor.caret(),
            self.cursor.selection(),
            doc.line_count(),
            &windows,
        ) else {
            return;
        };
        // The IME answers `SCS_SETRECONVERTSTRING` with the messages of a composition, which the
        // window handles while the application is borrowed by this command. Run the calls after it.
        crate::file_ops::start(window, cx, async move |view, cx| {
            run(&view, cx, &*ime, &plan);
        });
    }

    /// `SetSelectionFromReonvert`: selects the range of the line, with the caret at its start.
    fn select_reconverted(&mut self, anchor: Pos, caret: Pos, cx: &mut Context<Self>) {
        if anchor == caret {
            self.cursor.move_to(caret, false);
        } else {
            self.cursor.select(anchor, caret);
        }
        self.ensure_caret_visible(cx);
        cx.notify();
    }
}

/// The steps of `Command_Reconvert` after the structure is made: the IME widens the range, the
/// range is selected, the conversion starts. The IME not taking reconversion is said on the
/// status bar.
fn run(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext, ime: &dyn Ime, plan: &Plan) {
    let mut reconvert = Reconvert::new(&plan.request);
    let say = |cx: &mut AsyncWindowContext, error: Error| {
        view.update(cx, |view, cx| {
            view.notice = Some(format!("再変換できない: {error}"));
            cx.notify();
        })
        .ok();
    };
    if let Err(error) = ime.query(&mut reconvert) {
        say(cx, error);
        return;
    }
    // A range the IME did not settle on (another version) selects nothing; the conversion still
    // starts, as in Sakura.
    if let Some((start, len)) = reconvert.range() {
        view.update(cx, |view, cx| {
            let doc = view.document().read(cx).doc();
            // the same window as the plan was made from
            let Some(win) = window_of(doc, plan.line, plan.asked.0, plan.asked.1) else {
                return;
            };
            let at =
                |unit: usize| Pos::new(plan.line, win.base + byte_at(&win.text, plan.index + unit));
            view.select_reconverted(at(start + len), at(start), cx);
        })
        .ok();
    }
    if let Err(error) = ime.set(&reconvert) {
        say(cx, error);
    }
}
