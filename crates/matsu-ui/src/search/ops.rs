//! Find, replace and the other jumps of Sakura's 検索 menu, as methods of the view. The window
//! parts (the panel, the message box) are here and in [`super::panel`]; the rules are in
//! `matsu_core::search`, and the document is changed only through the command context, so that the
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
use matsu_commands::command::CommandError;
use matsu_commands::commands::cursor::jump::{DEFAULT_OPTION, jump_to_line};
use matsu_commands::context::CommandContext;
use matsu_core::buffer::{Document, Eol, Pos};
use matsu_core::model::eol_text;
use matsu_core::search::{
    AppliedEdit, Control, Direction, Found, ReplaceTarget, find, replace_all,
    replace_and_find_next, replacement_for, start_after,
};
use matsu_regex::{RegexReplacer, ReplaceAllMode};

use super::state::{
    REGEX_ERROR_TITLE, REPLACED_TO_END_MESSAGE, ReplaceSettings, SearchState, not_found_message,
    not_found_status, regex_error_message, replaced_count_message, search_origin, seed_text,
    wrapped_message,
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
    /// The regular expression (or the replacement for it) cannot be used; the box said why.
    InvalidPattern,
}

/// The title of the message boxes.
const BOX_TITLE: &str = "MATSU Editor";

impl EditorView {
    pub fn search_state(&self) -> &SearchState {
        &self.search
    }

    /// Shows a message box with an OK button (Sakura's `InfoMessage`).
    pub(crate) fn info_box(&self, message: &str, window: &mut Window, cx: &mut Context<Self>) {
        let answer = window.prompt(PromptLevel::Info, BOX_TITLE, Some(message), &["OK"], cx);
        crate::macros::keep_box(answer, window, cx);
    }

    /// Shows the box for a regular expression that does not compile (`CheckRegexpSyntax`,
    /// `sakura_core/extmodule/CBregexp.cpp`) and returns `true`, or returns `false` if the pattern
    /// is fine. A search with such a pattern does not run.
    pub(crate) fn report_pattern_error(&self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(error) = self.search.pattern_error() else {
            return false;
        };
        let message = regex_error_message(error);
        let answer = window.prompt(
            PromptLevel::Warning,
            REGEX_ERROR_TITLE,
            Some(&message),
            &["OK"],
            cx,
        );
        crate::macros::keep_box(answer, window, cx);
        true
    }

    /// The replacement for the regular expression of the search, if the option is on. A
    /// replacement that cannot be used (a trailing backslash) is reported like a bad pattern.
    fn regex_replacer(
        &self,
        settings: &ReplaceSettings,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<RegexReplacer>, ()> {
        let Some(regex) = self.search.regex() else {
            return Ok(None);
        };
        match RegexReplacer::new(regex.clone(), &settings.text, settings.target) {
            Ok(replacer) => Ok(Some(replacer)),
            Err(e) => {
                let message = regex_error_message(&e);
                let answer = window.prompt(
                    PromptLevel::Warning,
                    REGEX_ERROR_TITLE,
                    Some(&message),
                    &["OK"],
                    cx,
                );
                crate::macros::keep_box(answer, window, cx);
                Err(())
            }
        }
    }

    /// The text that the find and replace panels start with (see [`seed_text`]).
    pub(crate) fn search_seed(&self, cx: &gpui::App) -> Option<String> {
        let doc = self.document.read(cx).doc();
        let caret = self.cursor.caret();
        let selected = self.selected_first_line(cx);
        let line = doc
            .line(caret.line)
            .ok()
            .flatten()
            .map(|l| l.text)
            .unwrap_or_default();
        seed_text(selected.as_deref(), &line, caret.col)
    }

    /// The first string of the find, replace and Grep dialogs: [`search_seed`](Self::search_seed),
    /// but without a selection the word at the caret only when 「カーソル位置の文字列をデフォルトの検索文字列
    /// にする」 is on (`CEditView::GetCurrentTextForSearchDlg`, `m_bCaretTextForSearch`).
    pub(crate) fn dialog_seed(&self, cx: &gpui::App) -> Option<String> {
        if self.cursor.selection().is_none() && !super::prefs::prefs(cx).caret_text_for_search {
            return None;
        }
        self.search_seed(cx)
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
        if self.report_pattern_error(window, cx) {
            return SearchOutcome::InvalidPattern;
        }
        let Some(matcher) = self.search.matcher().cloned() else {
            return SearchOutcome::NoPattern;
        };
        self.search.set_mark(true);
        let selection = self.selection_range();
        let mut origin = search_origin(direction, self.cursor.caret(), selection.as_ref());
        let wrap = self.search.options().wrap;
        // The anchor of a selection that the lock keeps (not a box: `IsBoxSelecting`).
        let locked = selection
            .as_ref()
            .filter(|_| self.cursor.selecting_lock() && !self.cursor.is_box())
            .and(self.cursor.anchor());
        let found = {
            let doc = self.document.read(cx).doc();
            // An empty match is selected as nothing, so the caret stays on it; going on from
            // there would find it again. Sakura steps one character over it (`b0Match` in
            // `Command_SEARCH_NEXT`).
            if direction == Direction::Forward
                && selection.is_none()
                && self.search.empty_match() == Some(origin)
            {
                origin = start_after(doc, &(origin..origin)).unwrap_or(origin);
            }
            find(doc, &*matcher, origin, direction, wrap, &mut Control::new())
        };
        match found {
            Ok(Some(found)) => {
                self.search
                    .set_empty_match((found.start == found.end).then_some(found.start));
                match locked {
                    // With the selection lock on, the selection grows from its anchor to the
                    // match (`ChangeSelectAreaByCurrentCursor`), and the lock stays.
                    Some(anchor) => {
                        let to = match direction {
                            Direction::Forward => found.end,
                            Direction::Backward => found.start,
                        };
                        self.run_with(cx, |c| c.select_range(anchor, to)).ok();
                        self.cursor.set_selecting_lock(true);
                    }
                    None => self.select_found(&found, cx),
                }
                self.notice = found.wrapped.then(|| wrapped_message(direction).to_owned());
                cx.notify();
                SearchOutcome::Found
            }
            Ok(None) => {
                self.drop_selection_after_miss(direction, &selection, locked.is_some(), cx);
                self.not_found(
                    direction,
                    &not_found_message(direction, self.search.pattern()),
                    window,
                    cx,
                );
                SearchOutcome::NotFound
            }
            Err(e) => {
                self.drop_selection_after_miss(direction, &selection, locked.is_some(), cx);
                self.notice = Some(format!("検索できなかった: {e}"));
                cx.notify();
                SearchOutcome::NotFound
            }
        }
    }

    /// A search that finds nothing leaves the selection dropped, as Sakura's does: before it
    /// searches, `Command_SEARCH_NEXT` and `Command_SEARCH_PREV` put the caret at the end (the
    /// start for the search backward) of the selection and drop the selection
    /// (`DisableSelectArea`), unless the selection lock is on, where the selection stays. So
    /// after a search, "something is selected" means "found"; a macro tells a hit by
    /// `IsTextSelected` (`S_SearchNext` returns nothing).
    fn drop_selection_after_miss(
        &mut self,
        direction: Direction,
        selection: &Option<Range<Pos>>,
        locked: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(sel) = selection else { return };
        if locked {
            return;
        }
        let at = search_origin(direction, self.cursor.caret(), Some(sel));
        self.run_with(cx, |c| c.move_caret(at, false)).ok();
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
        if self.report_pattern_error(window, cx) {
            return SearchOutcome::InvalidPattern;
        }
        let Some(matcher) = self.search.matcher().cloned() else {
            return SearchOutcome::NoPattern;
        };
        let settings = self.search.replace.clone();
        let Ok(replacer) = self.regex_replacer(&settings, window, cx) else {
            return SearchOutcome::InvalidPattern;
        };
        self.search.set_mark(true);
        self.search.set_empty_match(None);
        // Sakura moves the caret to the start of the selection and looks from there, so a match
        // that is selected is the one that gets replaced.
        let from = self
            .selection_range()
            .map_or(self.cursor.caret(), |s| s.start);
        let options = self.search.options();
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
                    let step = match &replacer {
                        // The text of a regex replacement differs per match (`$1`), and
                        // already holds the kept match for the insert targets.
                        Some(r) => r.expand(doc, &found).and_then(|text| {
                            let step = r.replace_and_find_next(
                                &mut copy,
                                &found,
                                options.wrap,
                                &mut Control::new(),
                            )?;
                            Ok((step, text, regex_core_target(settings.target)))
                        }),
                        None => replace_and_find_next(
                            &mut copy,
                            &*matcher,
                            &found.range(),
                            &settings.text,
                            settings.target,
                            options.wrap,
                            &mut Control::new(),
                        )
                        .map(|step| (step, settings.text.clone(), settings.target)),
                    };
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
        let (step, text, target) = match step {
            Ok(done) => done,
            Err(e) => {
                self.notice = Some(format!("置換できなかった: {e}"));
                cx.notify();
                return SearchOutcome::NotFound;
            }
        };
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
                self.search
                    .set_empty_match((n.start == n.end).then_some(n.start));
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
        if self.report_pattern_error(window, cx) {
            return None;
        }
        let matcher = self.search.matcher().cloned()?;
        let settings = self.search.replace.clone();
        let replacer = self.regex_replacer(&settings, window, cx).ok()?;
        self.search.set_mark(true);
        self.search.set_empty_match(None);
        // 選択範囲内 means the selection if there is one, else the whole file
        // (`bSelectedArea=FALSE` in `Command_REPLACE_ALL`).
        let range = self.selection_range().filter(|_| settings.in_selection);
        let done = {
            let doc = self.document.read(cx).doc();
            let mut copy = doc.clone();
            match &replacer {
                Some(r) => {
                    let mode = if settings.consecutive_all {
                        ReplaceAllMode::Repeat
                    } else {
                        ReplaceAllMode::Batch
                    };
                    r.replace_all(&mut copy, mode, range.clone(), Control::new())
                }
                None => replace_all(
                    &mut copy,
                    &*matcher,
                    &settings.text,
                    settings.target,
                    range.clone(),
                    Control::new(),
                ),
            }
        };
        let result = match done {
            Ok(result) => result,
            Err(e) => {
                self.notice = Some(format!("置換できなかった: {e}"));
                cx.notify();
                return None;
            }
        };
        let count = result.count;
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
        // The box with the count is the dialog's (`CDlgReplace`, `STR_DLGREPLC_REPLACE`), not the
        // command's: a key or a macro replaces without it.
        self.notice = Some(replaced_count_message(count));
        cx.notify();
        Some(count)
    }

    /// `F_SEARCH_CLEARMARK`: with a selection, the selected text becomes the pattern and is
    /// marked; without one, the marks go (`Command_SEARCH_CLEARMARK`). The pattern stays, so
    /// 次を検索 still works.
    pub fn toggle_search_mark(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = self.search_seed_selection(cx) {
            let mut options = self.search.options();
            // Sakura: `m_sCurSearchOption.bRegularExp = false; bWordOnly = false;`
            options.regex = false;
            options.whole_word = false;
            self.search.set_options(options);
            self.search.set_pattern(&text);
            self.search.set_mark(true);
            // `AddToSearchKeyArr( cmemCurText )` in `Command_SEARCH_CLEARMARK`.
            self.remember_search_key(cx);
            self.show_pattern_in_panel(window, cx);
        } else {
            self.search.set_mark(false);
        }
        cx.notify();
    }

    /// The first line of the selection, if there is a selection (`CEditView::GetSelectedDataOne`):
    /// of a selection by lines the text up to the end of its first line; of a box selection the
    /// text of its top row between the columns of the box.
    pub(crate) fn selected_first_line(&self, cx: &gpui::App) -> Option<String> {
        let doc = self.document.read(cx).doc();
        if self.cursor.is_box() {
            let (row, from, to) =
                crate::hit::BoxArea::of_cursor(&self.layout, doc, &self.cursor)?.top_row();
            let from = self.layout.row_to_pos(doc, row, from).ok()?.pos;
            let to = self.layout.row_to_pos(doc, row, to).ok()?.pos.max(from);
            let capped = Pos::new(from.line, from.col.saturating_add(MAX_SEED_BYTES));
            return doc.text_between(from, to.min(capped)).ok();
        }
        let (a, b) = self.cursor.selection()?;
        first_line_of_selection(doc, a, b)
    }

    /// The first line of the selection, if there is a selection.
    fn search_seed_selection(&self, cx: &gpui::App) -> Option<String> {
        let text = self.selected_first_line(cx)?;
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

    /// The go-to-line dialog's jump: `F_JUMP` with the option the dialog gives, line breaks
    /// (Sakura's default, `m_bLineNumIsCRLF_ForJump`).
    pub fn go_to_line(&mut self, number: usize, cx: &mut Context<Self>) {
        let number = i64::try_from(number).unwrap_or(i64::MAX);
        self.run_with(cx, |c| jump_to_line(c, number, DEFAULT_OPTION))
            .ok();
    }
}

/// The target that the core functions see for a regex replacement: its text already holds the
/// kept match (`rep$&`, `$&rep`), see `RegexReplacer`.
fn regex_core_target(target: ReplaceTarget) -> ReplaceTarget {
    match target {
        ReplaceTarget::DeleteLine => ReplaceTarget::DeleteLine,
        _ => ReplaceTarget::Match,
    }
}

/// The most that a pattern taken from the selection can be: a selection of a whole large document
/// must not be read to use its first line.
const MAX_SEED_BYTES: usize = 8 * 1024;

/// The text of the selection `a..b` up to the end of its first line (and at most
/// [`MAX_SEED_BYTES`]), which is all that [`seed_text`] uses.
fn first_line_of_selection(doc: &Document, a: Pos, b: Pos) -> Option<String> {
    let line_end = Pos::new(a.line, doc.line_len(a.line).ok().flatten()?);
    let capped = Pos::new(a.line, a.col.saturating_add(MAX_SEED_BYTES));
    let end = b.min(line_end).min(capped);
    doc.text_between(a, end).ok()
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
        let (start, old_end) = old_range(c.doc(), e);
        let made = c.replace(start, old_end, &e.inserted)?;
        debug_assert_eq!((made.start, made.end), (e.start, e.end));
        old_ends.push(old_end);
    }
    Ok(old_ends)
}

/// Where `e` replaced text in the document as it was just before the edit.
///
/// `AppliedEdit::start` is a position after the edit. When the edit puts a CR and an LF side by
/// side, the buffer joins them into one CRLF and moves the start back before the CR (`a\rX\nb`
/// without `X`: the edit began at `(1, 0)` and `start` is `(0, 1)`). So when `start` is at the end
/// of a line that ends with a CR, the edit may have begun at the start of the next line; the
/// candidate that holds the removed text and gives the same edit again is the one.
fn old_range(doc: &Document, e: &AppliedEdit) -> (Pos, Pos) {
    let at_cr_end = doc
        .line(e.start.line)
        .ok()
        .flatten()
        .is_some_and(|l| l.eol == Some(Eol::Cr) && l.text.len() == e.start.col);
    let ext_eol = doc.ext_eol();
    let end_of = |at: Pos| {
        end_in_doc(doc, at, &e.removed).unwrap_or_else(|| end_after(at, &e.removed, ext_eol))
    };
    let direct = (e.start, end_of(e.start));
    if !at_cr_end {
        return direct;
    }
    let after_cr = Pos::new(e.start.line + 1, 0);
    let moved = (after_cr, end_of(after_cr));
    let fits = |&(start, end): &(Pos, Pos)| {
        doc.text_between(start, end).is_ok_and(|t| t == e.removed)
            && doc
                .clone()
                .replace(start, end, &e.inserted)
                .is_ok_and(|made| (made.start, made.end) == (e.start, e.end))
    };
    if fits(&direct) || !fits(&moved) {
        direct
    } else {
        moved
    }
}

/// Where `removed` ends when it lies in `doc` from `start`, found by walking the lines and endings
/// the document has (Sakura keeps the ending recorded in each line), not by reading the text again
/// under the setting now. `None` when the document does not hold that text there.
fn end_in_doc(doc: &Document, start: Pos, removed: &str) -> Option<Pos> {
    let (mut line, mut col, mut rest) = (start.line, start.col, removed);
    loop {
        let l = doc.line(line).ok().flatten()?;
        let tail = l.text.get(col..)?;
        if tail.len() >= rest.len() {
            return tail
                .starts_with(rest)
                .then(|| Pos::new(line, col + rest.len()));
        }
        rest = rest.strip_prefix(tail)?;
        rest = rest.strip_prefix(eol_text(l.eol?))?;
        (line, col) = (line + 1, 0);
        if rest.is_empty() {
            return Some(Pos::new(line, 0));
        }
    }
}

/// Where text that is `text` long ends when it starts at `start`, read as a new document.
/// `ext_eol` is the setting of the document the text was in: NEL, LS and PS end a line only then.
fn end_after(start: Pos, text: &str, ext_eol: bool) -> Pos {
    let Ok(scratch) = Document::from_utf8_with(text.as_bytes().to_vec(), ext_eol) else {
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
    use matsu_core::search::ReplaceTarget;

    /// Replaces `pattern` with `text` through the context, as the panel does, and returns the
    /// text. The expected text is what the core makes of the same replacement.
    fn replaced(text: &str, pattern: &str, with: &str, target: ReplaceTarget) -> (String, String) {
        use matsu_commands::layout::PlainLayout;
        use matsu_core::cursor::Cursor;
        use matsu_core::model::DocumentModel;
        use matsu_core::search::{Control, PlainMatcher, PlainOptions};

        let matcher = PlainMatcher::new(pattern, PlainOptions::default());
        let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
        let mut expected = doc.clone();
        let result = replace_all(&mut expected, &matcher, with, target, None, Control::new())
            .expect("the core replaces");
        let all = |d: &Document| {
            d.text_between(Pos::new(0, 0), Pos::new(d.line_count(), 0))
                .unwrap()
        };

        let mut model = DocumentModel::new(doc);
        let mut cursor = Cursor::default();
        let mut layout = PlainLayout::default();
        let mut cx = CommandContext::new(&mut model, &mut cursor, &mut layout, None);
        apply_edits(&mut cx, &result.edits).expect("the edits apply");
        (all(model.doc()), all(&expected))
    }

    #[test]
    fn a_cr_and_an_lf_that_meet_are_replayed_from_their_old_positions() {
        for (text, pattern, with) in [
            ("a\rX\nb", "X", ""),
            ("a\rX\nb", "X", "Y"),
            ("a\rb", "b", "\nc"),
            ("a\rX\nX\rX\nX", "X", ""),
            ("X\rX\nX\rX\nX", "X", ""),
            ("a\rX\nb\rX\nc", "X", "Q"),
            ("a\r\nX\r\nb", "X", ""),
            ("\rX\n", "X", ""),
            ("aX\r\nbX", "X", "\r"),
        ] {
            for target in [ReplaceTarget::Match, ReplaceTarget::InsertAfter] {
                let (got, want) = replaced(text, pattern, with, target);
                assert_eq!(got, want, "{text:?} {pattern:?} -> {with:?} ({target:?})");
            }
        }
        let (got, _) = replaced("a\rX\nb", "X", "", ReplaceTarget::Match);
        assert_eq!(got, "a\r\nb");
    }

    #[test]
    fn only_the_first_line_of_a_selection_is_read() {
        let doc = Document::from_utf8(b"abc def\r\nxyz".to_vec()).unwrap();
        let first = |a, b| first_line_of_selection(&doc, a, b);
        assert_eq!(
            first(Pos::new(0, 4), Pos::new(1, 2)).as_deref(),
            Some("def")
        );
        assert_eq!(
            first(Pos::new(0, 0), Pos::new(0, 3)).as_deref(),
            Some("abc")
        );
        let long = Document::from_utf8("a".repeat(MAX_SEED_BYTES * 2).into_bytes()).unwrap();
        let text = first_line_of_selection(&long, Pos::new(0, 0), Pos::new(0, MAX_SEED_BYTES * 2));
        assert_eq!(text.map(|t| t.len()), Some(MAX_SEED_BYTES));
    }

    #[test]
    fn the_old_end_of_an_edit_follows_the_endings_the_document_has() {
        // Read with `bEnableExtEol` off and turned on afterwards: the NEL is still a character.
        let doc = Document::from_utf8_with(b"a\xC2\x85b\nTAIL".to_vec(), false).unwrap();
        doc.set_ext_eol(true);
        let removed = "a\u{85}b";
        assert_eq!(
            end_in_doc(&doc, Pos::new(0, 0), removed),
            Some(Pos::new(0, 4))
        );
        // Reading the text again under the setting now would end it on the next line.
        assert_eq!(end_after(Pos::new(0, 0), removed, true), Pos::new(1, 1));
        let removed = "a\u{85}b\nTA";
        assert_eq!(
            end_in_doc(&doc, Pos::new(0, 0), removed),
            Some(Pos::new(1, 2))
        );
        // The other way round: a NEL that is an ending in the document.
        let doc = Document::from_utf8_with(b"a\xC2\x85b\nTAIL".to_vec(), true).unwrap();
        doc.set_ext_eol(false);
        assert_eq!(
            end_in_doc(&doc, Pos::new(0, 0), "a\u{85}b"),
            Some(Pos::new(1, 1))
        );
        // Text that is not there is not found.
        assert_eq!(end_in_doc(&doc, Pos::new(0, 0), "zz"), None);
    }

    #[test]
    fn end_after_follows_the_line_breaks_of_the_text() {
        let at = Pos::new(2, 3);
        assert_eq!(end_after(at, "", false), at);
        assert_eq!(end_after(at, "ab", false), Pos::new(2, 5));
        assert_eq!(end_after(at, "ab\r\n", false), Pos::new(3, 0));
        assert_eq!(end_after(at, "ab\ncd", false), Pos::new(3, 2));
        assert_eq!(end_after(at, "日本", false), Pos::new(2, 9));
        assert_eq!(end_after(at, "a\r\nb\r\n", false), Pos::new(4, 0));
        // NEL ends a line only when the document counts it.
        assert_eq!(end_after(at, "a\u{85}", false), Pos::new(2, 6));
        assert_eq!(end_after(at, "a\u{85}", true), Pos::new(3, 0));
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
