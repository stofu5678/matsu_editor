//! The arguments that a macro gives to `SearchNext`, `SearchPrev`, `Replace` and `ReplaceAll`.
//!
//! Sakura: `CMacro::HandleCommand` (`sakura_core/macro/CMacro.cpp`, the cases `F_SEARCH_NEXT`,
//! `F_SEARCH_PREV` and `F_REPLACE`, `F_REPLACE_ALL`). The macro sets the search string, the
//! options and the replacement of the window the way the dialog does, and then runs the command
//! with no dialog. A command that is run without arguments (a key, a menu item) is not touched
//! and uses what the window remembers.
//!
//! The number that follows the strings is a sum of these (`CMacro.cpp`):
//!
//! | value | meaning |
//! |-------|---------|
//! | 0x001 | whole word |
//! | 0x002 | match case |
//! | 0x004 | regular expression |
//! | 0x008 | tell the user when nothing is found |
//! | 0x010 | close the find dialog after the search |
//! | 0x020 | search again from the top (the end) |
//! | 0x040 | (replace) paste from the clipboard |
//! | 0x080 | (replace all) in the selection |
//! | 0x100, 0x200 | (replace) insert before / after the match instead of replacing it |
//! | 0x400 | (replace all) repeat the replacement |
//! | 0x800 | do not put the strings in the history |
//! | 0x1000 | put the settings back after the call |
//!
//! A macro has no one to answer a box, so a match that is not found is not told unless 0x008 is
//! given. The flags 0x008, 0x010 and 0x020 are set by every call, 0 when not given.

use gpui::{Context, Window};
use matsu_commands::command::{Arg, CommandError};
use matsu_core::search::ReplaceTarget;

use super::state::{ReplaceSettings, SearchOptions, build_matcher, regex_error_message};
use crate::view::EditorView;

/// Which command the arguments are for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MacroSearch {
    /// `F_SEARCH_NEXT` and `F_SEARCH_PREV`: (string, flags).
    Find,
    /// `F_REPLACE`: (string, replacement, flags).
    Replace,
    /// `F_REPLACE_ALL`: (string, replacement, flags).
    ReplaceAll,
}

/// What 0x1000 puts back (`backupFlags`, `backupLocalFlags`, `backupStr` in `CMacro.cpp`).
struct Saved {
    pattern: String,
    options: SearchOptions,
    replace: ReplaceSettings,
    mark: bool,
}

/// `_MAX_PATH`: a longer string is not put in the history.
const MAX_HISTORY_LEN: usize = 260;

fn text(args: &[Arg], i: usize) -> Result<Option<&str>, CommandError> {
    match args.get(i) {
        None => Ok(None),
        Some(Arg::Text(t)) => Ok(Some(t)),
        Some(other) => Err(CommandError::BadArgs(format!(
            "argument {} must be a string, not {other:?}",
            i + 1
        ))),
    }
}

/// `_wtoi` of an argument that is left out: 0.
fn flags(args: &[Arg], i: usize) -> Result<i64, CommandError> {
    match args.get(i) {
        None => Ok(0),
        Some(Arg::Int(n)) => Ok(*n),
        Some(other) => Err(CommandError::BadArgs(format!(
            "argument {} must be a number, not {other:?}",
            i + 1
        ))),
    }
}

impl EditorView {
    /// Runs `run` (the search or replace command) after the macro's arguments are applied, and
    /// puts the settings back after it when the macro asked for that (0x1000). Gives `None`
    /// when the command did not run: the regular expression does not compile (the box said
    /// so, as `CheckRegexpSyntax` does), which Sakura's macro does not count as a failure.
    ///
    /// Without arguments, `run` is called as it is.
    pub(crate) fn with_search_args<R>(
        &mut self,
        args: &[Arg],
        kind: MacroSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
        run: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
    ) -> Result<Option<R>, CommandError> {
        if args.is_empty() {
            return Ok(Some(run(self, window, cx)));
        }
        let pattern = text(args, 0)?.unwrap_or("");
        let replacement =
            match kind {
                MacroSearch::Find => None,
                // `STR_ERR_DLGMACRO09`, `STR_ERR_DLGMACRO10`: the macro stops.
                MacroSearch::Replace | MacroSearch::ReplaceAll => {
                    if pattern.is_empty() {
                        return Err(CommandError::BadArgs(
                            "the string to replace is needed".to_owned(),
                        ));
                    }
                    Some(text(args, 1)?.ok_or_else(|| {
                        CommandError::BadArgs("the replacement is needed".to_owned())
                    })?)
                }
            };
        let flags = flags(args, if replacement.is_some() { 2 } else { 1 })?;
        let has = |bit: i64| flags & bit != 0;
        if replacement.is_some() && has(0x40) {
            // There is no clipboard replacement in the replace panel yet.
            return Err(CommandError::Unavailable(
                "replacing with the clipboard (0x40) from a macro",
            ));
        }

        // The options of the search itself: set with the string, and only when there is one.
        let mut options = self.search.options();
        let new_search = !pattern.is_empty();
        if new_search {
            options.whole_word = has(0x01);
            options.match_case = has(0x02);
            options.regex = has(0x04);
            if options.regex
                && let Err(error) = build_matcher(pattern, &options)
            {
                // `CheckRegexpSyntax( ..., true )` shows the box; the macro goes on.
                let answer = crate::boxes::prompt(
                    window,
                    gpui::PromptLevel::Warning,
                    super::state::REGEX_ERROR_TITLE,
                    Some(&regex_error_message(&error)),
                    &["OK"],
                    cx,
                );
                crate::macros::keep_box(answer, window, cx);
                return Ok(None);
            }
        }

        let saved = has(0x1000).then(|| Saved {
            pattern: self.search.pattern().to_owned(),
            options: self.search.options(),
            replace: self.search.replace.clone(),
            mark: self.search.mark(),
        });
        // The history is left alone when asked, and always when the settings are put back.
        let history = !has(0x800) && saved.is_none();

        options.notify_not_found = has(0x08);
        options.auto_close = has(0x10);
        options.wrap = has(0x20);
        self.search.set_options(options);
        if new_search {
            self.search.set_pattern(pattern);
            if history && pattern.chars().count() < MAX_HISTORY_LEN {
                self.remember_search_key(cx);
            }
        }
        if let Some(replacement) = replacement {
            self.search.replace.text = replacement.to_owned();
            self.search.replace.target = match (flags >> 8) & 3 {
                0 => ReplaceTarget::Match,
                1 => ReplaceTarget::InsertBefore,
                2 => ReplaceTarget::InsertAfter,
                _ => ReplaceTarget::DeleteLine,
            };
            self.search.replace.consecutive_all = has(0x400);
            // A single replacement never takes the selection as its area.
            self.search.replace.in_selection = kind == MacroSearch::ReplaceAll && has(0x80);
            if history && replacement.chars().count() < MAX_HISTORY_LEN {
                self.remember_replace_key(cx);
            }
        }

        let result = run(self, window, cx);

        if let Some(saved) = saved {
            self.search.set_options(saved.options);
            self.search.set_pattern(&saved.pattern);
            self.search.replace = saved.replace;
            self.search.set_mark(saved.mark);
            cx.notify();
        }
        Ok(Some(result))
    }
}
