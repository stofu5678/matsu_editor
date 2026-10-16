//! Sakura's incremental search (インクリメンタルサーチ): a mode of the view in which typed
//! characters build the pattern and every change searches again from where the mode began.
//!
//! Sakura: `sakura_core/view/CEditView_Cmdisrch.cpp` (`ISearchEnter`, `ISearchExit`,
//! `ISearchExec`, `ISearchBack`, `ISearchSetStatusMsg`, `TranslateCommand_isearch`), the state
//! members in `CEditView.h`, and the help `HLP000304.html` to `HLP000307.html`.
//!
//! The mode is [`ISearch`], held by the view; the operations are methods of the view. While the
//! mode is on, the functions that Sakura translates ([`translate`]) are taken over by the mode,
//! and any other function ends it first.
//!
//! Not like Sakura: the migemo search is not here (it needs a dictionary and a library); an IME
//! composition is not shown while the mode is on (the text that the IME commits is added to the
//! pattern, as Sakura's `F_IME_CHAR` does); the pointer does not change into the incremental
//! search icon.

use gpui::Context;
use ume_commands::command::Arg;
use ume_core::buffer::Pos;
use ume_core::search::{Control, Direction, find, start_after};

use crate::view::EditorView;

/// Sakura's `ESearchMode` without the migemo search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ISearchMode {
    /// `SEARCH_NORMAL`: the pattern is plain text.
    Normal,
    /// `SEARCH_REGEXP`: the pattern is a regular expression.
    Regexp,
}

/// The most history entries (`m_sISearchHistory[256]`).
const HISTORY_SIZE: usize = 256;
/// How many of the oldest entries go when the history is full (`i - 100` in `ISearchExec`).
const HISTORY_DROP: usize = 100;

/// `STR_EDITVWISRCH_NOMATCH`.
const NO_MATCH: &str = " (見つかりません)";

/// One step of the history: what was selected (or where the caret was) after a search, and
/// whether the step searched for the next match without changing the pattern
/// (`m_sISearchHistory`, `m_bISearchFlagHistory`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Step {
    range: std::ops::Range<Pos>,
    next: bool,
}

/// The state of the mode while it is on.
#[derive(Clone, Debug)]
pub struct ISearch {
    mode: ISearchMode,
    direction: Direction,
    /// The last search found nothing when asked for the next match, so the next one begins at
    /// the other end of the document (`m_bISearchWrap`).
    wrap: bool,
    /// The next typed character starts a new pattern (`m_bISearchFirst`).
    first: bool,
    /// The pattern being typed (`m_strCurSearchKey`).
    key: String,
    /// Entry 0 is where the mode began; the steps after it are the searches.
    history: Vec<Step>,
    /// The status bar text (`ISearchSetStatusMsg`, plus the no match note).
    message: String,
}

impl ISearch {
    pub fn mode(&self) -> ISearchMode {
        self.mode
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// The pattern typed so far.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The number of searches made (`m_nISearchHistoryCount`).
    pub fn count(&self) -> usize {
        self.history.len() - 1
    }

    /// The text of the status bar.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The status bar text without the no match note (`ISearchSetStatusMsg`).
    fn status(&self) -> String {
        let mut text = String::from(match self.mode {
            ISearchMode::Normal => "I-Search",
            ISearchMode::Regexp => "[RegExp] I-Search",
        });
        text.push_str(if self.direction == Direction::Backward {
            " Backward: "
        } else {
            ": "
        });
        if self.count() > 0 {
            text.push_str(&self.key);
        }
        text
    }
}

/// What the mode does with a function while it is on (`TranslateCommand_isearch`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Translation {
    /// The function enters or repeats the mode: it runs as it is.
    Enter,
    /// `F_ISEARCH_ADD_CHAR`; `None` takes the char from the arguments.
    AddChar(Option<char>),
    /// `F_ISEARCH_ADD_STR`.
    AddStr,
    /// `F_ISEARCH_DEL_BACK`.
    DelBack,
    /// Any other function ends the mode, then runs.
    Exit,
}

/// Sakura's table in `TranslateCommand_isearch`. Tab and Space are chars, not indentation, and
/// so are their unindent twins.
pub fn translate(function: &str) -> Translation {
    match function {
        "F_ISEARCH_NEXT"
        | "F_ISEARCH_PREV"
        | "F_ISEARCH_REGEXP_NEXT"
        | "F_ISEARCH_REGEXP_PREV"
        | "F_ISEARCH_MIGEMO_NEXT"
        | "F_ISEARCH_MIGEMO_PREV" => Translation::Enter,
        "F_WCHAR" | "F_IME_CHAR" => Translation::AddChar(None),
        "F_INSTEXT_W" => Translation::AddStr,
        "F_INDENT_TAB" | "F_UNINDENT_TAB" => Translation::AddChar(Some('\t')),
        "F_INDENT_SPACE" | "F_UNINDENT_SPACE" => Translation::AddChar(Some(' ')),
        "F_DELETE_BACK" => Translation::DelBack,
        _ => Translation::Exit,
    }
}

impl EditorView {
    /// The mode, while it is on.
    pub fn isearch(&self) -> Option<&ISearch> {
        self.isearch.as_ref()
    }

    /// Lets the mode take over the function `function` (a Sakura name such as `F_WCHAR`) if it is
    /// on. Returns `true` when the mode handled the function, and `false` when it is off or when
    /// the function goes on to run (after ending the mode, or because it enters the mode).
    pub(crate) fn isearch_intercept(
        &mut self,
        function: &str,
        args: &[Arg],
        cx: &mut Context<Self>,
    ) -> bool {
        if self.isearch.is_none() {
            return false;
        }
        match translate(function) {
            Translation::Enter => false,
            Translation::AddChar(fixed) => {
                let c = fixed.or_else(|| match args.first() {
                    Some(Arg::Char(c)) => Some(*c),
                    Some(Arg::Int(n)) => u32::try_from(*n).ok().and_then(char::from_u32),
                    _ => None,
                });
                if let Some(c) = c {
                    self.isearch_add_char(c, cx);
                }
                true
            }
            Translation::AddStr => {
                if let Some(Arg::Text(text)) = args.first() {
                    self.isearch_add_str(text, cx);
                }
                true
            }
            Translation::DelBack => {
                self.isearch_back(cx);
                true
            }
            Translation::Exit => {
                self.isearch_exit(cx);
                false
            }
        }
    }

    /// `ISearchEnter`: starts the mode, or, when it is on in the same mode, searches for the next
    /// match in `direction`.
    pub fn isearch_enter(
        &mut self,
        mode: ISearchMode,
        direction: Direction,
        cx: &mut Context<Self>,
    ) {
        if let Some(a) = &mut self.isearch
            && a.mode == mode
        {
            // The command again: the next match, the other way if the direction changed.
            a.direction = direction;
            a.first = false;
            self.isearch_exec(true, cx);
            return;
        }
        // Only the mode begins. The selection goes (`DisableSelectArea`).
        let caret = self.cursor.caret();
        if self.cursor.has_selection() {
            self.run_with(cx, |c| c.move_caret(caret, false)).ok();
        }
        // The options of the search panel, with the incremental search's own case and word
        // rules: it ignores case, and the plain search is not by word.
        let mut options = self.search.options();
        options.regex = mode == ISearchMode::Regexp;
        options.match_case = false;
        if mode == ISearchMode::Normal {
            options.whole_word = false;
        }
        self.search.set_options(options);
        let key = match self.isearch.take() {
            // switching the mode keeps the pattern
            Some(a) => a.key,
            None => {
                // 検索開始位置 is where the first mode began, not where a switch happened
                self.search.set_start_pos(Some(caret));
                self.search.pattern().to_owned()
            }
        };
        self.search.set_mark(false);
        let mut a = ISearch {
            mode,
            direction,
            wrap: false,
            first: true,
            key,
            history: vec![Step {
                range: caret..caret,
                next: false,
            }],
            message: String::new(),
        };
        a.message = a.status();
        self.isearch = Some(a);
        cx.notify();
    }

    /// `ISearchExit`: the pattern and the options stay as the current search (F3 goes on with
    /// them). A pattern that was never searched for is dropped.
    pub fn isearch_exit(&mut self, cx: &mut Context<Self>) {
        let Some(a) = self.isearch.take() else { return };
        self.search
            .set_pattern(if a.count() == 0 { "" } else { &a.key });
        // `ISearchExit` puts the key in the history and keeps the window's sequence up to date.
        self.remember_search_key(cx);
        cx.notify();
    }

    /// `ISearchExec(DWORD)`: adds one character to the pattern and searches. A line break ends
    /// the mode.
    pub fn isearch_add_char(&mut self, c: char, cx: &mut Context<Self>) {
        if matches!(c, '\r' | '\n') {
            self.isearch_exit(cx);
            return;
        }
        let Some(a) = &mut self.isearch else { return };
        if a.first {
            a.first = false;
            a.key.clear();
        }
        a.key.push(c);
        self.isearch_exec(false, cx);
    }

    /// `ISearchExec(LPCWSTR)`: adds the characters one by one.
    pub fn isearch_add_str(&mut self, text: &str, cx: &mut Context<Self>) {
        for c in text.chars() {
            if self.isearch.is_none() {
                break;
            }
            self.isearch_add_char(c, cx);
        }
    }

    /// `ISearchExec(bool)`: searches for the pattern. `next` goes on to the next match; else the
    /// search starts at the current one, so that a longer pattern keeps the match it extends.
    fn isearch_exec(&mut self, next: bool, cx: &mut Context<Self>) {
        let Some(a) = &self.isearch else { return };
        if a.key.is_empty() {
            return;
        }
        let (direction, wrapping, key) = (a.direction, a.wrap && next, a.key.clone());
        // `ISearchWordMake`: the options were set when the mode began.
        self.search.set_pattern(&key);
        self.search.set_mark(true);
        let selection = self.cursor.selection();
        let caret = self.cursor.caret();
        // Where the search begins.
        let mut origin = if wrapping {
            // the other end of the document
            match direction {
                Direction::Forward => Pos::new(0, 0),
                Direction::Backward => Pos::new(self.document.read(cx).doc().line_count(), 0),
            }
        } else if let Some((from, to)) = selection {
            match (direction, next) {
                (Direction::Forward, false) | (Direction::Backward, true) => from,
                (Direction::Backward, false) | (Direction::Forward, true) => to,
            }
        } else {
            caret
        };
        let found = match self.search.matcher().cloned() {
            Some(matcher) => {
                let doc = self.document.read(cx).doc();
                if !wrapping
                    && direction == Direction::Forward
                    && selection.is_none()
                    && self.search.empty_match() == Some(origin)
                {
                    // an empty match stays selected as nothing: step over it
                    origin = start_after(doc, &(origin..origin)).unwrap_or(origin);
                }
                find(
                    doc,
                    &*matcher,
                    origin,
                    direction,
                    false,
                    &mut Control::new(),
                )
                .ok()
                .flatten()
            }
            // a regular expression that does not compile finds nothing (no box while typing)
            None => None,
        };
        let matched = found.is_some();
        let step = match found {
            Some(found) => {
                self.search
                    .set_empty_match((found.start == found.end).then_some(found.start));
                self.run_with(cx, |c| c.select_range(found.end, found.start))
                    .ok();
                Step {
                    range: found.start..found.end,
                    next,
                }
            }
            None => Step {
                range: selection.map_or(caret..caret, |(from, to)| from..to),
                next,
            },
        };
        let Some(a) = &mut self.isearch else { return };
        if a.count() + 1 >= HISTORY_SIZE {
            a.history.drain(..HISTORY_DROP);
        }
        a.history.push(step);
        a.wrap = !matched && (next || a.wrap);
        a.message = a.status();
        if !matched {
            a.message.push_str(NO_MATCH);
        }
        cx.notify();
    }

    /// `ISearchBack`: Backspace goes back one step of the history: the pattern loses its last
    /// character if the step added one, and the selection returns to where it was.
    pub fn isearch_back(&mut self, cx: &mut Context<Self>) {
        let Some(a) = &mut self.isearch else { return };
        if a.count() == 0 {
            return;
        }
        let mut mark = true;
        if a.count() == 1 {
            mark = false;
            a.first = true;
        } else if !a.history[a.count()].next {
            a.key.pop();
            if a.key.is_empty() {
                mark = false;
            }
        }
        a.history.pop();
        let range = a.history[a.count()].range.clone();
        let at_start = a.count() == 0;
        a.message = a.status();
        let key = a.key.clone();
        self.search.set_pattern(&key);
        self.search.set_mark(mark);
        self.run_with(cx, |c| {
            if at_start || range.start == range.end {
                c.move_caret(range.start, false)
            } else {
                c.select_range(range.end, range.start)
            }
        })
        .ok();
        cx.notify();
    }
}
