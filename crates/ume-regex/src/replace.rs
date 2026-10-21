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
//!   `m_bConsecutiveAll`, off by default): [`ReplaceAllMode::Batch`] is the `g` option of bregonig.html (5.2, "g: global
//!   replacement"; Sakura passes it unless the check box is on), one
//!   pass over the rest of each line that searches the line as it was before any replacement (that look-behind
//!   sees the original text is not in bregonig.html; bregonig.dll 4.20 does so);
//!   [`ReplaceAllMode::Repeat`] replaces one match, searches again from the caret and so on.
//!   They differ when a replacement changes what a later match sees (look-behind, `\b`, `\G`).
//!
//! # The line ending
//!
//! A line is matched together with its own line ending (see [`RegexMatcher`]), so a match can
//! cover the line break. Sakura replaces exactly the matched text: a match that covers only
//! the `\r` of a CR LF leaves the `\n` (`CViewCommander::Command_REPLACE`, 「\rを置換するとその後ろの
//! \nが消えてしまう問題の対応」), and the part of the line ending before or after the match is put
//! back ([`Found::kept_before`], [`Found::kept_after`]). A replacement that removes the line
//! ending joins the line with the next one, and the search goes on from the end of the
//! replacement, in the joined line (「一括置換」: `Command_REPLACE_ALL`, the caret is after the
//! inserted text when `Command_SEARCH_NEXT` runs again).

use std::ops::Range;
use std::sync::Arc;

use ume_core::buffer::{BufferError, Document, Pos};
use ume_core::search::{
    AppliedEdit, Control, Direction, Found, Match, Matcher, MatcherError, ReplaceAllResult,
    ReplaceStep, ReplaceTarget, Replacement, SearchError, apply_replacements, find, find_in_line,
    replacement_for, start_after,
};

use crate::matcher::RegexMatcher;
use crate::prepare::ends_with_escape;
use crate::replacement::{ReplacementError, ReplacementProblem, ReplacementTemplate};

/// How "replace all" goes through a line (`CDlgReplace::m_bConsecutiveAll`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReplaceAllMode {
    /// Every match is found in the line as it was; Sakura's default (一括置換, the `g` option of bregonig.html 5.2).
    #[default]
    Batch,
    /// One match is replaced, and the search goes on from there in the changed text
    /// (「すべて置換」は置換の繰返し).
    Repeat,
}

/// The matcher of [`ReplaceAllMode::Batch`]: it scans a line the way the `g` option of
/// bregonig.dll does (measured on bregonig.dll 4.20). That differs from Sakura's own loop
/// (`CSearchAgent::SearchWord`, which `RegexMatcher` follows) in one place: `\G` stays at the end
/// of the last match while the scan steps one character over an empty match, so after an empty
/// match `\G` cannot match any more (`\G` replaced by `-` in `abc` gives `-abc`). It runs over
/// the whole subject, the line ending included, and also finds the empty match at the very end
/// after the line ending (`x*` replaced by `-` in `ab` plus LF gives `-a-b-` LF `-`, as the real
/// Sakura does), where the loop of `CSearchAgent::SearchWord` stops at the end of the text.
#[derive(Debug)]
struct GlobalScan<'a>(&'a RegexMatcher);

impl Matcher for GlobalScan<'_> {
    fn find_at(&self, line: &str, start: usize) -> Result<Option<Match>, MatcherError> {
        self.0.find_at(line, start)
    }

    fn sees_line_ending(&self) -> bool {
        true
    }

    fn find_all_from(
        &self,
        line: &str,
        _body: usize,
        start: usize,
    ) -> Result<Vec<Match>, MatcherError> {
        let mut all = Vec::new();
        let (mut pos, mut gpos) = (start, start);
        while let Some(m) = self.0.find_from(line, pos, gpos)? {
            let end = m.end;
            let empty = m.is_empty();
            all.push(m);
            gpos = end;
            pos = if empty {
                match line[end..].chars().next() {
                    Some(c) => end + c.len_utf8(),
                    None => break,
                }
            } else {
                end
            };
        }
        Ok(all)
    }
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

    /// The text that replaces `found.range()`: the template, with the part of the line ending
    /// before and after the match put back (see the module docs).
    fn expand_in(&self, line: &str, found: &Found) -> String {
        let text = self.expand_bare(&found.subject(line), found);
        format!("{}{text}{}", found.kept_before(), found.kept_after())
    }

    /// The template for one match, in `subject` (the line text and its line ending).
    fn expand_bare(&self, subject: &str, found: &Found) -> String {
        let mut groups = Vec::with_capacity(found.captures.len() + 1);
        groups.push(Some(found.raw.clone()));
        groups.extend(found.captures.iter().cloned());
        self.template.expand(subject, &groups)
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
            if i >= doc.line_count() {
                break;
            }
            let start = if i == from.line { from.col } else { 0 };
            if let Some(found) = find_in_line(doc, &*self.matcher, i, start, false)? {
                return Ok(Some(found));
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
        let (edits, caret) = match mode {
            ReplaceAllMode::Batch => self.replace_batch(&mut work, range, ctl)?,
            ReplaceAllMode::Repeat => {
                let edits = self.replace_repeat(&mut work, range, ctl)?;
                let caret = edits.last().map(|e| e.resume);
                (edits, caret)
            }
        };
        *doc = work;
        Ok(ReplaceAllResult { edits, caret })
    }

    /// 一括置換 (`Command_REPLACE_ALL` without 「すべて置換は置換の繰返し」): Sakura finds the first
    /// match from the caret, then lets bregonig replace every match in the rest of that line
    /// (the line ending included) in one go, and searches again from the caret, which is then at
    /// the end of the text it inserted. So the next search sees the text as the replacements
    /// left it, a joined line included. The edits are one per match, as the count Sakura shows
    /// is the number of matches. Returns the edits and the caret after the last one.
    fn replace_batch(
        &self,
        work: &mut Document,
        range: Option<Range<Pos>>,
        mut ctl: Control<'_>,
    ) -> Result<(Vec<AppliedEdit>, Option<Pos>), SearchError> {
        let mut from = range.as_ref().map_or(Pos::new(0, 0), |r| r.start);
        // Where the range ends now; it moves as the text before it changes.
        let mut limit = range.map(|r| r.end);
        let mut edits = Vec::new();
        let mut caret = None;
        let mut polled = 0usize;
        let scan = GlobalScan(&self.matcher);
        while let Some(found) = self.next_found(work, from, limit, &mut ctl, &mut polled)? {
            if limit.is_some_and(|l| beyond(&found, l)) {
                break;
            }
            let line_no = found.start.line;
            let Some(line) = work.line(line_no)? else {
                break;
            };
            let body = line.text.len();
            let eol = line.eol.map_or("", ume_core::encoding::Eol::as_str);
            // A selection that ends inside the text of this line shortens what is replaced to the
            // end of the selection (`nLen` in `Command_REPLACE_ALL`, 「選択範囲置換の場合は行内の
            // 選択範囲末尾まで置換範囲を縮める」).
            let (body, eol, cut) = match limit {
                Some(l) if l.line == line_no && body > l.col => {
                    let mut cut = l.col;
                    while !line.text.is_char_boundary(cut) {
                        cut += 1;
                    }
                    (cut, "", true)
                }
                _ => (body, eol, false),
            };
            let subject = format!("{}{eol}", &line.text[..body]);
            let matches = if self.target == ReplaceTarget::DeleteLine {
                // Sakura deletes the line of the first match, whatever else matches in it.
                vec![Match::new(found.raw.start, found.raw.end)]
            } else {
                // From where the search began: the first match is the first one from there
                // (Sakura starts at the match, the same except for \K).
                let at = if from.line == line_no { from.col } else { 0 };
                scan.find_all_from(&subject, body, at)?
            };
            let mut plan = Vec::with_capacity(matches.len());
            // Matches that reach into the line ending share one range of the document (the
            // whole line ending is replaced), so they are put together in the subject's own
            // coordinates and make one edit.
            let mut tail: Vec<Found> = Vec::new();
            for m in matches {
                let m_found = Found::in_subject(line_no, m, body, eol, false);
                if self.target != ReplaceTarget::DeleteLine && m_found.raw.end > body {
                    tail.push(m_found);
                    continue;
                }
                let edit = self.edit_for(work, &m_found, &line.text)?;
                if !self.is_no_op(&edit) {
                    plan.push(edit);
                }
            }
            if let Some(first) = tail.first() {
                let start = first.start;
                let mut text = String::new();
                let mut at = start.col;
                for f in &tail {
                    text.push_str(&subject[at..f.raw.start.max(at)]);
                    text.push_str(&self.expand_bare(&subject, f));
                    at = f.raw.end.max(at);
                }
                text.push_str(&subject[at..]);
                let end = Pos::new(line_no + 1, 0);
                plan.push(replacement_for(
                    work,
                    &(start..end),
                    &text,
                    ReplaceTarget::Match,
                )?);
            }
            // The end of the text that Sakura replaces in one go: the end of the line, with its
            // line ending.
            let line_end = if eol.is_empty() {
                Pos::new(line_no, body)
            } else {
                Pos::new(line_no + 1, 0)
            };
            let Some(last) = plan.last() else {
                // Deleting the last line when it is already empty removes nothing.
                from = Pos::new(line_no + 1, 0);
                continue;
            };
            let last_end = last.end;
            let applied = apply_replacements(work, &plan)?;
            let new_end = applied.last().map_or(last_end, |e| e.end);
            limit = limit.map(|l| shift_pos(last_end, new_end, l));
            from = shift_pos(last_end, new_end, line_end);
            caret = Some(from);
            edits.extend(applied);
            // The selection ends in this line: the shortened subject held every match that is
            // left in the selection, and the text that was inserted is not searched again.
            if cut {
                break;
            }
        }
        Ok((edits, caret))
    }

    /// The next match from `from`, in the lines up to the end of the selection `limit` if there is
    /// one, without wrapping.
    fn next_found(
        &self,
        work: &Document,
        from: Pos,
        limit: Option<Pos>,
        ctl: &mut Control<'_>,
        polled: &mut usize,
    ) -> Result<Option<Found>, SearchError> {
        // A selection is searched only up to its end, not through the rest of the document
        // (that scan cannot be cancelled, but a selection is small). Without one the search is
        // the cancellable `find`.
        match limit {
            Some(l) => self.find_before(work, from, l, ctl, polled),
            None => find(work, &*self.matcher, from, Direction::Forward, false, ctl),
        }
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
        while let Some(found) = self.next_found(work, from, limit, &mut ctl, &mut polled)? {
            if limit.is_some_and(|l| beyond(&found, l)) {
                break;
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

/// Whether `found` is outside a range that ends at `end`: the same rule as `find_all`. A range
/// that ends at column 0 leaves that line out, and a match must end inside the range (a match
/// over a line ending ends on the next line).
fn beyond(found: &Found, end: Pos) -> bool {
    if end.col == 0 {
        found.end.line >= end.line
    } else {
        found.end > end
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
