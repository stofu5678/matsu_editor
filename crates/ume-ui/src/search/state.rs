//! What the find and replace dialogs of Sakura remember, without any window: the pattern and the
//! options, how a matcher is made from them, the text the dialog starts with, and the messages.
//!
//! Sakura keeps the current key and options in the view (`CEditView::m_strCurSearchKey`,
//! `m_sCurSearchOption`) and the dialog settings in the shared data
//! (`DllShareData::m_Common.m_sSearch`, defaults in `sakura_core/env/CShareData.cpp`). One window
//! has one [`SearchState`] here, in its [`EditorView`](crate::view::EditorView).

use std::ops::Range;
use std::sync::Arc;

use ume_core::buffer::Pos;
use ume_core::search::{Direction, Matcher, PlainMatcher, PlainOptions, ReplaceTarget};
use ume_regex::{PatternError, RegexMatcher, RegexSearchOptions};

use crate::word::word_range;

/// The options of the find dialog (`sakura_core/dlg/CDlgFind.cpp`, `GetData`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchOptions {
    /// 英大文字と小文字を区別する (`bLoHiCase`).
    pub match_case: bool,
    /// 単語単位で探す (`bWordOnly`).
    pub whole_word: bool,
    /// 見つからないときにメッセージを表示 (`m_bNOTIFYNOTFOUND`, on by default).
    pub notify_not_found: bool,
    /// 検索ダイアログを自動的に閉じる (`m_bAutoCloseDlgFind`, on by default).
    pub auto_close: bool,
    /// 先頭（末尾）から再検索 (`m_bSearchAll`, off by default).
    pub wrap: bool,
    /// 正規表現 (`bRegularExp`). The pattern is bregonig's syntax ([`ume_regex`]). 単語単位 is
    /// then ignored, as in Sakura, whose dialog greys the box out (`CDlgFind::OnBnClicked`).
    pub regex: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        // `CShareData.cpp`: sSearch.m_bNOTIFYNOTFOUND = TRUE, m_bAutoCloseDlgFind = TRUE,
        // m_bSearchAll = FALSE; the search option itself starts all off.
        Self {
            match_case: false,
            whole_word: false,
            notify_not_found: true,
            auto_close: true,
            wrap: false,
            regex: false,
        }
    }
}

/// The settings of the replace dialog (`sakura_core/dlg/CDlgReplace.cpp`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplaceSettings {
    /// 置換後 (`m_strText2`).
    pub text: String,
    /// 置換対象 (`m_nReplaceTarget`): 選択文字 / 選択始点に挿入 / 選択終点に挿入 / 行削除.
    pub target: ReplaceTarget,
    /// 選択範囲内 (`m_bSelectedArea`) instead of the whole file.
    pub in_selection: bool,
    /// 「すべて置換」は置換の繰返し (`m_bConsecutiveAll`, off by default). Only a regular
    /// expression has it (the box is disabled without 正規表現).
    pub consecutive_all: bool,
}

/// What the window remembers about searching.
#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pattern: String,
    options: SearchOptions,
    pub replace: ReplaceSettings,
    /// Whether the matches of the pattern are drawn (`m_bCurSrchKeyMark`).
    mark: bool,
    /// 検索開始位置 for `F_JUMP_SRCHSTARTPOS` (`m_ptSrchStartPos_PHY`).
    start_pos: Option<Pos>,
    matcher: Option<Arc<dyn Matcher>>,
    /// The same matcher as [`regex`](Self::regex) when the option is on, for the replacement.
    regex: Option<Arc<RegexMatcher>>,
    /// The same matcher when the search is a plain one.
    plain: Option<Arc<PlainMatcher>>,
    /// Why a regular expression pattern cannot be used (it then has no matcher).
    pattern_error: Option<PatternError>,
    /// Where the last search selected an empty match, so that the next one steps over it.
    empty_match: Option<Pos>,
    /// `m_nCurSearchKeySequence`: how much of the shared search history this window has seen.
    /// `None` is Sakura's -1 (nothing yet).
    key_sequence: Option<u64>,
    /// The same for the replace strings (`CDlgReplace::m_nReplaceKeySequence`).
    replace_sequence: Option<u64>,
}

/// A matcher made by [`build_matcher`].
#[derive(Clone, Debug)]
pub struct BuiltMatcher {
    pub matcher: Arc<dyn Matcher>,
    /// Set for a regular expression, whose replacement needs more than [`Matcher`] gives.
    pub regex: Option<Arc<RegexMatcher>>,
    /// Set for a plain search, whose whole-word mode numbers the words of the pattern.
    pub plain: Option<Arc<PlainMatcher>>,
}

/// The matcher for a pattern and options, or `None` for an empty pattern. This is the one place
/// that picks the matching engine: the plain search of `ume_core`, or the regular expression of
/// `ume_regex` (bregonig's syntax) when the option is on. A pattern that does not compile is an
/// error (Sakura says so with `CheckRegexpSyntax`, `sakura_core/extmodule/CBregexp.cpp`).
pub fn build_matcher(
    pattern: &str,
    options: &SearchOptions,
) -> Result<Option<BuiltMatcher>, PatternError> {
    if pattern.is_empty() {
        return Ok(None);
    }
    if options.regex {
        // 単語単位 is left out: `CSearchAgent::SearchWord` takes the regex branch first.
        let regex = Arc::new(RegexMatcher::new(
            pattern,
            RegexSearchOptions {
                match_case: options.match_case,
            },
        )?);
        return Ok(Some(BuiltMatcher {
            matcher: regex.clone(),
            regex: Some(regex),
            plain: None,
        }));
    }
    let plain = Arc::new(PlainMatcher::new(
        pattern,
        PlainOptions {
            match_case: options.match_case,
            whole_word: options.whole_word,
        },
    ));
    Ok(Some(BuiltMatcher {
        matcher: plain.clone(),
        regex: None,
        plain: Some(plain),
    }))
}

/// `STR_REGEX_COMPILE_ERR_PREAMBLE`: the box for a pattern that does not compile (the title is
/// [`REGEX_ERROR_TITLE`]).
pub fn regex_error_message(error: &impl std::fmt::Display) -> String {
    format!("正規表現の構文に誤りがあります。\n詳細：{error}")
}

/// `STR_BREGONIG_TITLE`.
pub const REGEX_ERROR_TITLE: &str = "正規表現エラー";

impl SearchState {
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn options(&self) -> SearchOptions {
        self.options
    }

    /// The matcher of the current pattern, if there is a pattern.
    pub fn matcher(&self) -> Option<&Arc<dyn Matcher>> {
        self.matcher.as_ref()
    }

    /// The regular expression matcher, when the option is on and the pattern compiles.
    pub fn regex(&self) -> Option<&Arc<RegexMatcher>> {
        self.regex.as_ref()
    }

    /// The plain-text matcher, when the search is not by regular expression.
    pub fn plain(&self) -> Option<&Arc<PlainMatcher>> {
        self.plain.as_ref()
    }

    /// Why the regular expression cannot be used, if it cannot.
    pub fn pattern_error(&self) -> Option<&PatternError> {
        self.pattern_error.as_ref()
    }

    /// Where the last search selected an empty match.
    pub fn empty_match(&self) -> Option<Pos> {
        self.empty_match
    }

    pub fn set_empty_match(&mut self, pos: Option<Pos>) {
        self.empty_match = pos;
    }

    /// How much of the shared search history this window has seen (see [`super::history`]).
    pub fn key_sequence(&self) -> Option<u64> {
        self.key_sequence
    }

    pub fn set_key_sequence(&mut self, sequence: Option<u64>) {
        self.key_sequence = sequence;
    }

    pub fn replace_sequence(&self) -> Option<u64> {
        self.replace_sequence
    }

    pub fn set_replace_sequence(&mut self, sequence: Option<u64>) {
        self.replace_sequence = sequence;
    }

    pub fn set_pattern(&mut self, pattern: &str) {
        if self.pattern != pattern {
            self.pattern = pattern.to_owned();
            self.rebuild();
        }
    }

    pub fn set_options(&mut self, options: SearchOptions) {
        if self.options != options {
            self.options = options;
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        self.empty_match = None;
        match build_matcher(&self.pattern, &self.options) {
            Ok(built) => {
                self.matcher = built.as_ref().map(|b| b.matcher.clone());
                self.regex = built.as_ref().and_then(|b| b.regex.clone());
                self.plain = built.and_then(|b| b.plain);
                self.pattern_error = None;
            }
            Err(e) => {
                self.matcher = None;
                self.regex = None;
                self.plain = None;
                self.pattern_error = Some(e);
            }
        }
    }

    /// Whether the matches are drawn.
    pub fn mark(&self) -> bool {
        self.mark && self.matcher.is_some()
    }

    pub fn set_mark(&mut self, mark: bool) {
        self.mark = mark;
    }

    pub fn start_pos(&self) -> Option<Pos> {
        self.start_pos
    }

    pub fn set_start_pos(&mut self, pos: Option<Pos>) {
        self.start_pos = pos;
    }
}

/// The text that the find and replace dialogs start with: the selection up to its first line
/// break, else the word at the caret; `None` when there is neither (the dialog then keeps the
/// pattern it had).
///
/// Sakura: `CEditView::GetCurrentTextForSearchDlg` and `GetCurrentTextForSearch`
/// (`sakura_core/view/CEditView_Search.cpp`). The word is found with this crate's cheap word rule
/// ([`word_range`]), not Sakura's `WhereCurrentWord`.
pub fn seed_text(selected: Option<&str>, line: &str, caret_col: usize) -> Option<String> {
    let text = match selected {
        Some(selected) => selected,
        None => {
            let (a, b) = word_range(line, caret_col);
            line.get(a..b).unwrap_or("")
        }
    };
    let first = text
        .split(['\r', '\n', '\u{85}', '\u{2028}', '\u{2029}'])
        .next()?;
    (!first.is_empty()).then(|| first.to_owned())
}

/// Where a search in `direction` goes on from. A found match is selected with the caret at its
/// start (as in Sakura), so "next" goes on from the end of the selection and "previous" from its
/// start; without a selection both go on from the caret.
pub fn search_origin(direction: Direction, caret: Pos, selection: Option<&Range<Pos>>) -> Pos {
    match (selection, direction) {
        (Some(s), Direction::Forward) => s.end,
        (Some(s), Direction::Backward) => s.start,
        (None, _) => caret,
    }
}

/// The status bar text for a search that wrapped around (`STR_ERR_SRNEXT1`, `STR_ERR_SRPREV1` in
/// `sakura_core/sakura_rc.rc`).
pub fn wrapped_message(direction: Direction) -> &'static str {
    match direction {
        Direction::Forward => "▼先頭から再検索しました",
        Direction::Backward => "▲末尾から再検索しました",
    }
}

/// The status bar text when nothing was found (`STR_ERR_SRNEXT2`, `STR_ERR_SRPREV2`).
pub fn not_found_status(direction: Direction) -> &'static str {
    match direction {
        Direction::Forward => "▽見つかりませんでした",
        Direction::Backward => "△見つかりませんでした",
    }
}

/// The message of the box shown when nothing was found (`STR_ERR_SRNEXT3`, `STR_ERR_SRPREV3`). A
/// long key is cut at 260 characters (`_MAX_PATH`) and "..." is added, as Sakura does.
pub fn not_found_message(direction: Direction, pattern: &str) -> String {
    let mut key: String = pattern.chars().take(260).collect();
    if key.chars().count() < pattern.chars().count() {
        key.push_str("...");
    }
    match direction {
        Direction::Forward => format!("前方(↓) に文字列 '{key}' が１つも見つかりません。"),
        Direction::Backward => format!("後方(↑) に文字列 '{key}' が１つも見つかりません。"),
    }
}

/// `STR_DLGFIND1`: the find button with no text.
pub const NO_PATTERN_MESSAGE: &str = "検索条件を指定してください。";
/// `STR_ERR_CEDITVIEW_CMD11`: the box after "replace" ran through the last match.
pub const REPLACED_TO_END_MESSAGE: &str = "最後まで置換しました。";

/// `STR_DLGREPLC_REPLACE`: the box after "replace all".
pub fn replaced_count_message(count: usize) -> String {
    format!("{count}箇所を置換しました。")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_sakuras() {
        let o = SearchOptions::default();
        assert!(!o.match_case && !o.whole_word && !o.wrap);
        assert!(o.notify_not_found && o.auto_close);
    }

    #[test]
    fn an_empty_pattern_has_no_matcher() {
        let mut s = SearchState::default();
        assert!(s.matcher().is_none() && !s.mark());
        s.set_pattern("a");
        s.set_mark(true);
        assert!(s.matcher().is_some() && s.mark());
        s.set_pattern("");
        assert!(s.matcher().is_none() && !s.mark());
    }

    #[test]
    fn options_change_the_matcher() {
        let mut s = SearchState::default();
        s.set_pattern("Ab");
        let hit = |s: &SearchState| s.matcher().unwrap().find_at("xab", 0).unwrap().is_some();
        assert!(hit(&s));
        s.set_options(SearchOptions {
            match_case: true,
            ..s.options()
        });
        assert!(!hit(&s));
    }

    #[test]
    fn the_regular_expression_option_picks_the_engine() {
        let mut s = SearchState::default();
        s.set_options(SearchOptions {
            regex: true,
            ..s.options()
        });
        s.set_pattern("a+");
        assert!(s.regex().is_some() && s.pattern_error().is_none());
        let m = s.matcher().unwrap().find_at("baab", 0).unwrap().unwrap();
        assert_eq!((m.start, m.end), (1, 3));
        // a pattern that does not compile has no matcher, and says why
        s.set_pattern("(");
        assert!(s.matcher().is_none() && s.regex().is_none() && s.pattern_error().is_some());
        // the plain search takes the same text literally
        s.set_options(SearchOptions {
            regex: false,
            ..s.options()
        });
        assert!(s.pattern_error().is_none() && s.regex().is_none());
        assert!(s.matcher().unwrap().find_at("a(b", 0).unwrap().is_some());
    }

    #[test]
    fn the_dialog_starts_with_the_selection_or_the_word() {
        assert_eq!(seed_text(Some("ab cd"), "x", 0).as_deref(), Some("ab cd"));
        // only the first line of a selection
        assert_eq!(seed_text(Some("ab\r\ncd"), "x", 0).as_deref(), Some("ab"));
        assert_eq!(seed_text(None, "foo bar", 5).as_deref(), Some("bar"));
        assert_eq!(seed_text(None, "", 0), None);
    }

    #[test]
    fn the_search_goes_on_from_the_selection() {
        let sel = Pos::new(1, 2)..Pos::new(1, 5);
        let caret = Pos::new(1, 2);
        assert_eq!(
            search_origin(Direction::Forward, caret, Some(&sel)),
            Pos::new(1, 5)
        );
        assert_eq!(
            search_origin(Direction::Backward, caret, Some(&sel)),
            Pos::new(1, 2)
        );
        assert_eq!(search_origin(Direction::Forward, caret, None), caret);
    }

    #[test]
    fn the_messages_are_sakuras() {
        assert_eq!(
            not_found_message(Direction::Forward, "abc"),
            "前方(↓) に文字列 'abc' が１つも見つかりません。"
        );
        let long = "a".repeat(300);
        let m = not_found_message(Direction::Backward, &long);
        assert!(m.contains(&format!("{}...", "a".repeat(260))));
        assert_eq!(replaced_count_message(3), "3箇所を置換しました。");
    }
}
