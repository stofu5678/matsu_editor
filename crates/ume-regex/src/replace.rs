//! Replacing with a regular expression: the document functions of `ume-core` with a replacement
//! that is worked out for each match.
//!
//! Sakura's replace commands for a regular expression (`CViewCommander::Command_REPLACE` and
//! `Command_REPLACE_ALL`, `sakura_core/cmd/CViewCommander_Search.cpp`) differ from the plain
//! ones in three ways, all of which are here:
//!
//! * The replacement is bregonig's ([`ReplacementTemplate`]), different for every match.
//! * 選択始点に挿入 / 選択終点に挿入 are done by replacing the match with `rep$&` / `$&rep`
//!   ("Nov. 9, 2005 かろと" in the source), so a `\U` in the replacement also changes `$&`.
//! * "すべて置換" has two modes (the check box 「すべて置換」は置換の繰返し,
//!   `m_bConsecutiveAll`, off by default): [`ReplaceAllMode::Batch`] is bregonig's `g` flag, one
//!   pass over each line that searches the line as it was before any replacement;
//!   [`ReplaceAllMode::Repeat`] replaces one match, searches again from the caret and so on.
//!   They differ when a replacement changes what a later match sees (look-behind, `\b`, `\G`).

use std::ops::Range;
use std::sync::Arc;

use ume_core::buffer::{BufferError, Document, Pos};
use ume_core::search::{
    AppliedEdit, Control, Direction, Found, Matcher, ReplaceAllResult, ReplaceStep, ReplaceTarget,
    Replacement, SearchError, apply_replacements, find, find_all, replacement_for, start_after,
};

use crate::matcher::RegexMatcher;
use crate::prepare::ends_with_escape;
use crate::replacement::{ReplacementError, ReplacementProblem, ReplacementTemplate};

/// How "replace all" goes through a line (`CDlgReplace::m_bConsecutiveAll`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplaceAllMode {
    /// Every match is found in the line as it was; Sakura's default (一括置換, bregonig's `g`).
    #[default]
    Batch,
    /// One match is replaced, and the search goes on from there in the changed text
    /// (「すべて置換」は置換の繰返し).
    Repeat,
}

/// A regular expression, a replacement for it and what to do with the match.
#[derive(Clone, Debug)]
pub struct RegexReplacer {
    matcher: Arc<RegexMatcher>,
    template: ReplacementTemplate,
    target: ReplaceTarget,
}

impl RegexReplacer {
    /// Compiles `replacement` against `matcher`. For [`ReplaceTarget::DeleteLine`] the
    /// replacement is not used (Sakura deletes the line without a regex).
    pub fn new(
        matcher: Arc<RegexMatcher>,
        replacement: &str,
        target: ReplaceTarget,
    ) -> Result<Self, ReplacementError> {
        // `$&` is added after the text below, which must not hide a trailing backslash.
        if target == ReplaceTarget::InsertBefore && ends_with_escape(replacement) {
            return Err(ReplacementError::UnmatchedSeparator);
        }
        let source = match target {
            ReplaceTarget::InsertBefore => format!("{replacement}$&"),
            ReplaceTarget::InsertAfter => format!("$&{replacement}"),
            ReplaceTarget::Match => replacement.to_owned(),
            // Sakura deletes the line without a replacement, so the text is not even read.
            ReplaceTarget::DeleteLine => String::new(),
        };
        let template =
            ReplacementTemplate::parse(&source, matcher.names(), matcher.capture_count())?;
        Ok(Self {
            matcher,
            template,
            target,
        })
    }

    pub fn matcher(&self) -> &RegexMatcher {
        &self.matcher
    }

    pub fn template(&self) -> &ReplacementTemplate {
        &self.template
    }

    pub fn target(&self) -> ReplaceTarget {
        self.target
    }

    /// References in the replacement that give nothing (see [`ReplacementTemplate::problems`]).
    pub fn problems(&self) -> &[ReplacementProblem] {
        self.template.problems()
    }

    /// The target the core functions see: the regex replacement already holds the kept match.
    fn core_target(&self) -> ReplaceTarget {
        match self.target {
            ReplaceTarget::DeleteLine => ReplaceTarget::DeleteLine,
            _ => ReplaceTarget::Match,
        }
    }

    /// The text that replaces `found`.
    pub fn expand(&self, doc: &Document, found: &Found) -> Result<String, BufferError> {
        let line = doc
            .line(found.start.line)?
            .map(|l| l.text)
            .unwrap_or_default();
        Ok(self.expand_in(&line, found))
    }

    fn expand_in(&self, line: &str, found: &Found) -> String {
        let mut groups = Vec::with_capacity(found.captures.len() + 1);
        groups.push(Some(found.start.col..found.end.col));
        groups.extend(found.captures.iter().cloned());
        self.template.expand(line, &groups)
    }

    /// The edit that replaces `found`.
    fn edit_for(
        &self,
        doc: &Document,
        found: &Found,
        line: &str,
    ) -> Result<Replacement, BufferError> {
        let text = if self.target == ReplaceTarget::DeleteLine {
            String::new()
        } else {
            self.expand_in(line, found)
        };
        replacement_for(doc, &found.range(), &text, self.core_target())
    }

    /// Replaces `found` once.
    pub fn replace_one(
        &self,
        doc: &mut Document,
        found: &Found,
    ) -> Result<AppliedEdit, BufferError> {
        let line = doc
            .line(found.start.line)?
            .map(|l| l.text)
            .unwrap_or_default();
        let edit = self.edit_for(doc, found, &line)?;
        Ok(apply_replacements(doc, std::slice::from_ref(&edit))?.remove(0))
    }

    /// What Sakura's 置換 button does: replace `found`, then find the next match from there
    /// (with `wrap`, the option 先頭（末尾）から再検索). See [`replace_and_find_next`](ume_core::search::replace_and_find_next).
    pub fn replace_and_find_next(
        &self,
        doc: &mut Document,
        found: &Found,
        wrap: bool,
        ctl: &mut Control<'_>,
    ) -> Result<ReplaceStep, BufferError> {
        let edit = self.replace_one(doc, found)?;
        let from = self.resume_after(doc, found, &edit)?;
        let next = find(doc, &*self.matcher, from, Direction::Forward, wrap, ctl);
        Ok(ReplaceStep { edit, next })
    }

    /// Line deletion of an empty last line removes nothing (no line ending to remove). Other
    /// targets are never skipped: an empty match replaced with nothing still counts.
    fn is_no_op(&self, edit: &Replacement) -> bool {
        self.target == ReplaceTarget::DeleteLine && edit.start == edit.end && edit.text.is_empty()
    }

    /// The first match from `from` in the lines up to the end of a selection that ends at `end`
    /// (a selection that ends at column 0 leaves that line out).
    fn find_before(
        &self,
        doc: &Document,
        from: Pos,
        end: Pos,
        ctl: &mut Control<'_>,
        polled: &mut usize,
    ) -> Result<Option<Found>, SearchError> {
        let Some(last) = (if end.col == 0 {
            end.line.checked_sub(1)
        } else {
            Some(end.line)
        }) else {
            return Ok(None);
        };
        for i in from.line..=last {
            // Once in 256 lines scanned over the whole replacement: the probe reads the last
            // line of the document, so it must not run for every replacement.
            *polled += 1;
            if (*polled - 1).is_multiple_of(256) {
                poll_cancel(doc, &*self.matcher, ctl)?;
            }
            let Some(line) = doc.line(i)? else { break };
            let mut start = if i == from.line { from.col } else { 0 };
            if start > line.text.len() {
                continue;
            }
            while !line.text.is_char_boundary(start) {
                start += 1;
            }
            if let Some(m) = self.matcher.find_at(&line.text, start)? {
                return Ok(Some(Found {
                    start: Pos::new(i, m.start),
                    end: Pos::new(i, m.end),
                    captures: m.captures,
                    wrapped: false,
                }));
            }
        }
        Ok(None)
    }

    /// Where the search goes on: after the new text, and one char further after an empty match.
    fn resume_after(
        &self,
        doc: &Document,
        found: &Found,
        edit: &AppliedEdit,
    ) -> Result<Pos, BufferError> {
        if found.start == found.end && self.target != ReplaceTarget::DeleteLine {
            start_after(doc, &(edit.resume..edit.resume))
        } else {
            Ok(edit.resume)
        }
    }

    /// Replaces every match from the start of the document, or of `range` (replace in the
    /// selection), to its end, without wrapping. The caller records the edits as one undo step.
    /// The rules of [`ume_core::search::replace_all`] apply (a match must lie in the range, the
    /// inserted text is not searched again, a line is deleted once). If the search is cancelled
    /// or fails, the document is not changed.
    pub fn replace_all(
        &self,
        doc: &mut Document,
        mode: ReplaceAllMode,
        range: Option<Range<Pos>>,
        ctl: Control<'_>,
    ) -> Result<ReplaceAllResult, SearchError> {
        let mut work = doc.clone();
        let edits = match mode {
            ReplaceAllMode::Batch => self.replace_batch(&mut work, range, ctl)?,
            ReplaceAllMode::Repeat => self.replace_repeat(&mut work, range, ctl)?,
        };
        *doc = work;
        let caret = edits.last().map(|e| e.resume);
        Ok(ReplaceAllResult { edits, caret })
    }

    fn replace_batch(
        &self,
        work: &mut Document,
        range: Option<Range<Pos>>,
        ctl: Control<'_>,
    ) -> Result<Vec<AppliedEdit>, SearchError> {
        let mut plan = Vec::new();
        let mut deleted_through: Option<usize> = None;
        // Matches come line by line, so the last line read is enough.
        let mut cached: Option<(usize, String)> = None;
        for found in find_all(work, &*self.matcher, range, ctl) {
            let found = found?;
            let line_no = found.start.line;
            if self.target == ReplaceTarget::DeleteLine {
                if deleted_through.is_some_and(|l| line_no <= l) {
                    continue;
                }
                deleted_through = Some(line_no);
            }
            if cached.as_ref().is_none_or(|(n, _)| *n != line_no) {
                let text = work.line(line_no)?.map(|l| l.text).unwrap_or_default();
                cached = Some((line_no, text));
            }
            let line = cached.as_ref().map_or("", |(_, t)| t.as_str());
            let edit = self.edit_for(work, &found, line)?;
            if self.is_no_op(&edit) {
                continue;
            }
            plan.push(edit);
        }
        Ok(apply_replacements(work, &plan)?)
    }

    fn replace_repeat(
        &self,
        work: &mut Document,
        range: Option<Range<Pos>>,
        mut ctl: Control<'_>,
    ) -> Result<Vec<AppliedEdit>, SearchError> {
        let mut from = range.as_ref().map_or(Pos::new(0, 0), |r| r.start);
        // Where the range ends now; it moves as the text before it changes.
        let mut limit = range.map(|r| r.end);
        let mut edits = Vec::new();
        let mut polled = 0usize;
        loop {
            // A selection is searched only up to its end, not through the rest of the document
            // (that scan cannot be cancelled, but a selection is small). Without one the
            // search is the cancellable `find`.
            let next = match limit {
                Some(l) => self.find_before(work, from, l, &mut ctl, &mut polled)?,
                None => find(
                    work,
                    &*self.matcher,
                    from,
                    Direction::Forward,
                    false,
                    &mut ctl,
                )?,
            };
            let Some(found) = next else { break };
            if let Some(l) = limit {
                // The same rule as `find_all`: a range that ends at column 0 leaves that line
                // out, and a match must end inside the range.
                let beyond = if l.col == 0 {
                    found.start.line >= l.line
                } else {
                    found.start.line > l.line || (found.end.line == l.line && found.end.col > l.col)
                };
                if beyond {
                    break;
                }
            }
            let line = work
                .line(found.start.line)?
                .map(|l| l.text)
                .unwrap_or_default();
            let edit = self.edit_for(work, &found, &line)?;
            if self.is_no_op(&edit) {
                // Deleting the last line when it is already empty removes nothing; going on
                // from the same place would find the same match forever.
                from = Pos::new(found.start.line + 1, 0);
                continue;
            }
            let applied = apply_replacements(work, std::slice::from_ref(&edit))?.remove(0);
            limit = limit.map(|l| shift_pos(edit.end, applied.end, l));
            from = self.resume_after(work, &found, &applied)?;
            edits.push(applied);
        }
        Ok(edits)
    }
}

/// Where position `p` of the old document is after an edit, if `p` was at or after the old end
/// of the edit (`old_end`) and the edit now ends at `new_end`; before it, the end of the edit.
fn shift_pos(old_end: Pos, new_end: Pos, p: Pos) -> Pos {
    if p < old_end {
        new_end
    } else if p.line == old_end.line {
        Pos::new(new_end.line, new_end.col + p.col - old_end.col)
    } else {
        Pos::new(new_end.line + p.line - old_end.line, p.col)
    }
}

/// `Control` only checks for cancellation inside `find`, so ask `find` to look at one line past
/// its end: it reads that line, checks the token first, and finds nothing.
fn poll_cancel(
    doc: &Document,
    matcher: &dyn Matcher,
    ctl: &mut Control<'_>,
) -> Result<(), SearchError> {
    let count = doc.line_count();
    if count > 0 {
        find(
            doc,
            matcher,
            Pos::new(count - 1, usize::MAX),
            Direction::Forward,
            false,
            ctl,
        )?;
    }
    Ok(())
}
