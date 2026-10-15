//! Find, replace and the other jumps of Sakura's 検索 menu, as methods of the view. The window
//! parts (the panel, the message box) are here and in [`super::panel`]; the rules are in
//! `ume_core::search`, and the document is changed only through the command context, so that the
//! layout, the undo history and the caret follow as for any command (ADR-0013 section 4).
//!
//! Sakura: `CViewCommander::Command_SEARCH_NEXT`, `Command_SEARCH_PREV`, `Command_REPLACE`,
//! `Command_REPLACE_ALL`, `Command_SEARCH_CLEARMARK` (`sakura_core/cmd/CViewCommander_Search.cpp`),
//! `Command_JUMP_SRCHSTARTPOS` and `Command_JUMP` (`CViewCommander_Bookmark.cpp`).
//!
//! Not like Sakura: the search runs on the UI thread, without the progress and cancel dialog of
//! "replace all" (`SearchJob` in the core is there for a later change); a replace that fails
//! halfway leaves the edits made so far in its one undo step.

use std::ops::Range;

use gpui::{Context, PromptLevel, Window};
use ume_commands::command::CommandError;
use ume_commands::context::CommandContext;
use ume_core::buffer::{Document, Pos};
use ume_core::search::{
    AppliedEdit, Control, Direction, Found, find, replace_all, replace_and_find_next,
    replacement_for,
};

use super::state::{
    REPLACED_TO_END_MESSAGE, SearchState, not_found_message, not_found_status,
    replaced_count_message, search_origin, seed_text, wrapped_message,
};
use crate::view::EditorView;

/// How a search ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchOutcome {
    /// A match was found and is selected.
    Found,
    /// There is no match (after wrapping around, if that was asked for).
    NotFound,
    /// There is no pattern to look for.
    NoPattern,
}

/// The title of the message boxes.
const BOX_TITLE: &str = "UME Editor";

impl EditorView {
    pub fn search_state(&self) -> &SearchState {
        &self.search
    }

    /// Shows a message box with an OK button (Sakura's `InfoMessage`).
    pub(crate) fn info_box(&self, message: &str, window: &mut Window, cx: &mut Context<Self>) {
        let answer = window.prompt(PromptLevel::Info, BOX_TITLE, Some(message), &["OK"], cx);
        drop(answer);
    }

    /// The text that the find and replace panels start with (see [`seed_text`]).
    pub(crate) fn search_seed(&self, cx: &gpui::App) -> Option<String> {
        let doc = self.document.read(cx).doc();
        let caret = self.cursor.caret();
        let selected = self
            .cursor
            .selection()
            .and_then(|(a, b)| doc.text_between(a, b).ok());
        let line = doc
            .line(caret.line)
            .ok()
            .flatten()
            .map(|l| l.text)
            .unwrap_or_default();
        seed_text(selected.as_deref(), &line, caret.col)
    }

    /// The selected match range, ordered.
    fn selection_range(&self) -> Option<Range<Pos>> {
        self.cursor.selection().map(|(a, b)| a..b)
    }

    /// Finds the next (or previous) match from the caret and selects it, with the caret at its
    /// start, as Sakura does. Reports a wrap-around in the status bar and a failure in the status
    /// bar and, if the option is on, in a message box.
    pub fn search(
        &mut self,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SearchOutcome {
        let Some(matcher) = self.search.matcher().cloned() else {
            return SearchOutcome::NoPattern;
        };
        self.search.set_mark(true);
        let origin = search_origin(
            direction,
            self.cursor.caret(),
            self.selection_range().as_ref(),
        );
        let wrap = self.search.options().wrap;
        let found = {
            let doc = self.document.read(cx).doc();
            find(doc, &*matcher, origin, direction, wrap, &mut Control::new())
        };
        match found {
            Ok(Some(found)) => {
                self.select_found(&found, cx);
                self.notice = found.wrapped.then(|| wrapped_message(direction).to_owned());
                cx.notify();
                SearchOutcome::Found
            }
            Ok(None) => {
                self.not_found(
                    direction,
                    &not_found_message(direction, self.search.pattern()),
                    window,
                    cx,
                );
                SearchOutcome::NotFound
            }
            Err(e) => {
                self.notice = Some(format!("検索できなかった: {e}"));
                cx.notify();
                SearchOutcome::NotFound
            }
        }
    }

    /// Nothing was found: the status bar says so, and the box does too if the option is on
    /// (`AlertNotFound` in `sakura_core/cmd/CViewCommander.h`).
    fn not_found(
        &mut self,
        direction: Direction,
        message: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.notice = Some(not_found_status(direction).to_owned());
        cx.notify();
        if self.search.options().notify_not_found {
            self.info_box(message, window, cx);
        }
    }

    /// Selects `found`, with the caret at its start.
    fn select_found(&mut self, found: &Found, cx: &mut Context<Self>) {
        self.run_with(cx, |c| c.select_range(found.end, found.start))
            .ok();
        // `run_with` clears the notice when the caret moves; the callers set theirs after this.
    }

    /// `F_REPLACE`: the 置換 button. Finds the match at the caret (or the selection), replaces it
    /// and selects the next one. When there is no next match, the document ends with the
    /// replacement and the box says "最後まで置換しました。".
    pub fn replace_next(&mut self, window: &mut Window, cx: &mut Context<Self>) -> SearchOutcome {
        let Some(matcher) = self.search.matcher().cloned() else {
            return SearchOutcome::NoPattern;
        };
        self.search.set_mark(true);
        // Sakura moves the caret to the start of the selection and looks from there, so a match
        // that is selected is the one that gets replaced.
        let from = self
            .selection_range()
            .map_or(self.cursor.caret(), |s| s.start);
        let options = self.search.options();
        let settings = self.search.replace.clone();
        let planned = {
            let doc = self.document.read(cx).doc();
            find(
                doc,
                &*matcher,
                from,
                Direction::Forward,
                options.wrap,
                &mut Control::new(),
            )
            .map(|found| {
                // Work out the replacement and what follows it on a copy; then make the same
                // edit in the document through the command context.
                found.map(|found| {
                    let mut copy = doc.clone();
                    let step = replace_and_find_next(
                        &mut copy,
                        &*matcher,
                        &found.range(),
                        &settings.text,
                        settings.target,
                        options.wrap,
                        &mut Control::new(),
                    );
                    (found, step)
                })
            })
        };
        let (found, step) = match planned {
            Ok(Some(planned)) => planned,
            Ok(None) => {
                let msg = not_found_message(Direction::Forward, self.search.pattern());
                self.not_found(Direction::Forward, &msg, window, cx);
                return SearchOutcome::NotFound;
            }
            Err(e) => {
                self.notice = Some(format!("検索できなかった: {e}"));
                cx.notify();
                return SearchOutcome::NotFound;
            }
        };
        let step = match step {
            Ok(step) => step,
            Err(e) => {
                self.notice = Some(format!("置換できなかった: {e}"));
                cx.notify();
                return SearchOutcome::NotFound;
            }
        };
        let (target, text) = (settings.target, settings.text);
        let range = found.range();
        let next = step.next.ok().flatten();
        let resume = step.edit.resume;
        let made = self.run_with(cx, |c| -> Result<(), CommandError> {
            c.begin_block();
            let result = (|| {
                let r = replacement_for(c.doc(), &range, &text, target)?;
                c.replace(r.start, r.end, &r.text)?;
                match &next {
                    Some(n) => c.select_range(n.end, n.start)?,
                    None => c.move_caret(resume, false)?,
                }
                Ok(())
            })();
            c.end_block();
            result
        });
        if let Err(e) = made {
            self.notice = Some(format!("置換できなかった: {e}"));
            cx.notify();
            return SearchOutcome::NotFound;
        }
        match next {
            Some(n) => {
                self.notice = n
                    .wrapped
                    .then(|| wrapped_message(Direction::Forward).to_owned());
                cx.notify();
            }
            None => self.not_found(Direction::Forward, REPLACED_TO_END_MESSAGE, window, cx),
        }
        SearchOutcome::Found
    }

    /// `F_REPLACE_ALL`: the すべて置換 button, as one undo step. Returns the number of
    /// replacements, or `None` when there is no pattern or it failed.
    pub fn replace_all(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<usize> {
        let matcher = self.search.matcher().cloned()?;
        self.search.set_mark(true);
        let settings = self.search.replace.clone();
        // 選択範囲内 means the selection if there is one, else the whole file
        // (`bSelectedArea=FALSE` in `Command_REPLACE_ALL`).
        let range = self.selection_range().filter(|_| settings.in_selection);
        let done = {
            let doc = self.document.read(cx).doc();
            let mut copy = doc.clone();
            replace_all(
                &mut copy,
                &*matcher,
                &settings.text,
                settings.target,
                range.clone(),
                Control::new(),
            )
        };
        let result = match done {
            Ok(result) => result,
            Err(e) => {
                self.notice = Some(format!("置換できなかった: {e}"));
                cx.notify();
                return None;
            }
        };
        let count = result.edits.len();
        let made = self.run_with(cx, |c| -> Result<(), CommandError> {
            c.begin_block();
            let result_of = (|| {
                let ends = apply_edits(c, &result.edits)?;
                match (&range, result.caret) {
                    (Some(range), _) => {
                        let start = shift_start(range.start, &result.edits, &ends);
                        let end = shift_through(range.end, &result.edits, &ends);
                        c.select_range(start, end)?;
                    }
                    (None, Some(caret)) => c.move_caret(caret, false)?,
                    (None, None) => {}
                }
                Ok(())
            })();
            c.end_block();
            result_of
        });
        if let Err(e) = made {
            self.notice = Some(format!("置換できなかった: {e}"));
            cx.notify();
            return None;
        }
        let message = replaced_count_message(count);
        self.notice = Some(message.clone());
        cx.notify();
        self.info_box(&message, window, cx);
        Some(count)
    }

    /// `F_SEARCH_CLEARMARK`: with a selection, the selected text becomes the pattern and is
    /// marked; without one, the marks go (`Command_SEARCH_CLEARMARK`). The pattern stays, so
    /// 次を検索 still works.
    pub fn toggle_search_mark(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.search_seed_selection(cx) {
            let mut options = self.search.options();
            // `m_sCurSearchOption.bRegularExp = false; bWordOnly = false;`
            options.whole_word = false;
            self.search.set_options(options);
            self.search.set_pattern(&text);
            self.search.set_mark(true);
        } else {
            self.search.set_mark(false);
        }
        cx.notify();
    }

    /// The first line of the selection, if there is a selection.
    fn search_seed_selection(&self, cx: &gpui::App) -> Option<String> {
        let (a, b) = self.cursor.selection()?;
        let text = self.document.read(cx).doc().text_between(a, b).ok()?;
        seed_text(Some(&text), "", 0)
    }

    /// `F_JUMP_SRCHSTARTPOS`: back to where the last search began.
    pub fn jump_to_search_start(&mut self, cx: &mut Context<Self>) {
        match self.search.start_pos() {
            Some(pos) => {
                let extend = false;
                self.run_with(cx, |c| c.move_caret(pos, extend)).ok();
            }
            None => {
                self.notice = Some("検索開始位置がまだない".to_owned());
                cx.notify();
            }
        }
    }

    /// `F_JUMP`: goes to the start of line `number` (counted from 1 by line breaks, which is
    /// Sakura's default, `m_bLineNumIsCRLF_ForJump`), clamped to the document.
    pub fn go_to_line(&mut self, number: usize, cx: &mut Context<Self>) {
        let lines = self.document.read(cx).doc().line_count();
        if lines == 0 {
            return;
        }
        let line = number.clamp(1, lines) - 1;
        self.run_with(cx, |c| c.move_caret(Pos::new(line, 0), false))
            .ok();
    }
}

/// Makes `edits` in the document, in order, and returns where each one's replaced text ended
/// before it was made (the old end, for [`shift_through`]). The edits come from running the same
/// replacement on a copy of the document, so their positions fit.
fn apply_edits(
    c: &mut CommandContext<'_>,
    edits: &[AppliedEdit],
) -> Result<Vec<Pos>, CommandError> {
    let mut old_ends = Vec::with_capacity(edits.len());
    for e in edits {
        let old_end = end_after(e.start, &e.removed);
        let made = c.replace(e.start, old_end, &e.inserted)?;
        debug_assert_eq!((made.start, made.end), (e.start, e.end));
        old_ends.push(old_end);
    }
    Ok(old_ends)
}

/// Where text that is `text` long ends when it starts at `start`.
fn end_after(start: Pos, text: &str) -> Pos {
    let Ok(scratch) = Document::from_utf8(text.as_bytes().to_vec()) else {
        return start;
    };
    let count = scratch.line_count();
    let Some(last) = count
        .checked_sub(1)
        .and_then(|i| scratch.line(i).ok().flatten())
    else {
        return start;
    };
    if last.eol.is_some() {
        Pos::new(start.line + count, 0)
    } else if count == 1 {
        Pos::new(start.line, start.col + last.text.len())
    } else {
        Pos::new(start.line + count - 1, last.text.len())
    }
}

/// Moves `p` (a position before all the edits) through the edits, each of which replaced text up
/// to `old_ends[i]` and now ends at `edits[i].end`.
fn shift_through(p: Pos, edits: &[AppliedEdit], old_ends: &[Pos]) -> Pos {
    let mut p = p;
    for (e, &old_end) in edits.iter().zip(old_ends) {
        p = if p < old_end {
            e.end
        } else if p.line == old_end.line {
            Pos::new(e.end.line, e.end.col + p.col - old_end.col)
        } else {
            Pos::new(e.end.line + p.line - old_end.line, p.col)
        };
    }
    p
}

/// Like [`shift_through`], for the start of a range: an edit that began at or before it and
/// replaced text up to or past it leaves the start at the edit's start (a deleted line takes the
/// start to the start of that line), not at the end of the new text.
fn shift_start(p: Pos, edits: &[AppliedEdit], old_ends: &[Pos]) -> Pos {
    let mut p = p;
    for (e, &old_end) in edits.iter().zip(old_ends) {
        if p <= e.start {
            break; // this edit and the later ones are after the start
        }
        p = if p < old_end {
            e.start
        } else {
            shift_through(p, std::slice::from_ref(e), &[old_end])
        };
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_after_follows_the_line_breaks_of_the_text() {
        let at = Pos::new(2, 3);
        assert_eq!(end_after(at, ""), at);
        assert_eq!(end_after(at, "ab"), Pos::new(2, 5));
        assert_eq!(end_after(at, "ab\r\n"), Pos::new(3, 0));
        assert_eq!(end_after(at, "ab\ncd"), Pos::new(3, 2));
        assert_eq!(end_after(at, "日本"), Pos::new(2, 9));
        assert_eq!(end_after(at, "a\r\nb\r\n"), Pos::new(4, 0));
    }

    #[test]
    fn the_start_of_a_range_stays_before_an_edit_that_covers_it() {
        // the line "xxa" is deleted from (0, 0) to (1, 0); the range started inside it
        let delete = AppliedEdit {
            start: Pos::new(0, 0),
            end: Pos::new(0, 0),
            removed: "xxa\n".into(),
            inserted: String::new(),
            resume: Pos::new(0, 0),
        };
        let moved = shift_start(
            Pos::new(0, 2),
            std::slice::from_ref(&delete),
            &[Pos::new(1, 0)],
        );
        assert_eq!(moved, Pos::new(0, 0));
        // an edit that starts exactly at the start does not move it
        let moved = shift_start(
            Pos::new(0, 0),
            std::slice::from_ref(&delete),
            &[Pos::new(1, 0)],
        );
        assert_eq!(moved, Pos::new(0, 0));
    }

    #[test]
    fn a_position_moves_with_the_edits_before_it() {
        // "ab" -> "xyz" at 0..2: the position (0, 5) is at (0, 6) afterwards
        let edit = AppliedEdit {
            start: Pos::new(0, 0),
            end: Pos::new(0, 3),
            removed: "ab".into(),
            inserted: "xyz".into(),
            resume: Pos::new(0, 3),
        };
        let moved = shift_through(
            Pos::new(0, 5),
            std::slice::from_ref(&edit),
            &[Pos::new(0, 2)],
        );
        assert_eq!(moved, Pos::new(0, 6));
        // a position inside the replaced text goes to the end of the new text
        let moved = shift_through(
            Pos::new(0, 1),
            std::slice::from_ref(&edit),
            &[Pos::new(0, 2)],
        );
        assert_eq!(moved, Pos::new(0, 3));
    }
}
