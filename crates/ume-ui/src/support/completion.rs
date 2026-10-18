//! The input completion list (入力補完, `F_HOKAN`).
//!
//! Sakura: `CViewCommander::Command_HOKAN` (`sakura_core/cmd/CViewCommander_Support.cpp`),
//! `CEditView::ShowHokanMgr`, `PreprocessCommand_hokan`, `PostprocessCommand_hokan`
//! (`sakura_core/view/CEditView_CmdHokan.cpp`), `CHokanMgr` (`sakura_core/env/CHokanMgr.cpp`) and
//! the help `help/sakura/res/HLP000104.html`. The words are found by [`ume_commands::text::complete`].
//!
//! What happens:
//!
//! * `F_HOKAN` takes the word before the caret. With no such word it only says
//!   "補完対象がありません" in the status bar. With no candidate the list is not shown; with one
//!   candidate that one is inserted at once; otherwise the list is shown below the start of the
//!   word, with the first candidate chosen.
//! * While the list is shown, a key goes to it first (`CHokanMgr::KeyProc`): Up, Down, Page Up,
//!   Page Down, Home and End choose a candidate; Enter and Right insert it (Tab too, once the
//!   setting `bHokanKey_TAB` is on); Esc closes the list. Any other key goes on to the editor.
//! * Typing a char or Backspace keeps the list and narrows it to the new word before the caret;
//!   it closes when there is no word or no candidate left. Any other command, a click, or a move of
//!   the caret closes it (`PreprocessCommand_hokan`).
//! * Choosing a candidate deletes the word before the caret and inserts the candidate (the key is
//!   part of the result: "Ap" and "apple" make "apple"), as one undo step.
//!
//! The candidates come from where the document type says (the type settings 支援: the word file,
//! the words of the document, the highlighted keywords) and are compared as it says (英大文字小文字を
//! 同一視する).
//!
//! Not done: the plugins as a source of candidates; the help text (the keyword help dictionary)
//! beside the list; the keys of the settings `bHokanKey_*` are Sakura's defaults (Enter and Right
//! on, Tab off), fixed.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, ParentElement, Styled, anchored, deferred, div, point, px, rgb,
};
use ume_commands::context::Changes;
use ume_commands::registry::Registry;
use ume_commands::text::complete;
use ume_config::settings::DocumentTypes;
use ume_core::buffer::Pos;

use crate::view::EditorView;

/// How many candidates the list shows without scrolling.
const VISIBLE_ROWS: usize = 10;

/// The function codes after which the list stays (`PreprocessCommand_hokan`): the keys that edit
/// the word, and `F_HOKAN` itself.
const KEEPING: [&str; 4] = ["F_HOKAN", "F_WCHAR", "F_IME_CHAR", "F_DELETE_BACK"];

/// The keys that insert the chosen candidate (`CommonSetting_Helper::m_bHokanKey_*`, with the
/// defaults of `CShareData.cpp`): Enter and Right on, Tab off.
const KEY_ENTER: bool = true;
const KEY_TAB: bool = false;
const KEY_RIGHT: bool = true;

/// The message of the status bar when there is no word before the caret
/// (`STR_SUPPORT_*` in `sakura_core/sakura_rc.rc`).
pub(crate) const NOT_COMPLETABLE: &str = "補完対象がありません";

/// The list that is shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Completion {
    /// The word before the caret that the list was made for.
    pub key: String,
    pub items: Vec<String>,
    pub selected: usize,
    /// The first row shown.
    pub top: usize,
}

impl Completion {
    pub(crate) fn new(key: String, items: Vec<String>) -> Self {
        Self {
            key,
            items,
            selected: 0,
            top: 0,
        }
    }

    /// Chooses row `to` (clamped), scrolling it into view.
    pub(crate) fn select(&mut self, to: usize) {
        self.selected = to.min(self.items.len().saturating_sub(1));
        if self.selected < self.top {
            self.top = self.selected;
        } else if self.selected >= self.top + VISIBLE_ROWS {
            self.top = self.selected + 1 - VISIBLE_ROWS;
        }
    }

    /// Moves the choice by `delta` rows, stopping at the ends (a list box does not wrap).
    pub(crate) fn move_by(&mut self, delta: isize) {
        self.select(self.selected.saturating_add_signed(delta));
    }

    pub(crate) fn chosen(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }
}

/// The key of a keystroke that the list uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListKey {
    Move(isize),
    First,
    Last,
    Insert,
    Close,
}

fn list_key(key: &str) -> Option<ListKey> {
    Some(match key {
        "up" => ListKey::Move(-1),
        "down" => ListKey::Move(1),
        "pageup" => ListKey::Move(-(VISIBLE_ROWS as isize)),
        "pagedown" => ListKey::Move(VISIBLE_ROWS as isize),
        "home" => ListKey::First,
        "end" => ListKey::Last,
        "enter" if KEY_ENTER => ListKey::Insert,
        "tab" if KEY_TAB => ListKey::Insert,
        "right" if KEY_RIGHT => ListKey::Insert,
        "escape" => ListKey::Close,
        _ => return None,
    })
}

/// The word file that was read last, with what tells that it has not changed.
#[derive(Default)]
struct WordFileCache {
    path: PathBuf,
    stamp: Option<(u64, SystemTime)>,
    lines: Arc<Vec<String>>,
}

impl WordFileCache {
    /// The lines of the file at `path`: the kept ones if the file is the same, else the file read
    /// again. A file that cannot be read has no lines.
    fn lines(&mut self, path: &Path) -> Arc<Vec<String>> {
        let stamp = std::fs::metadata(path)
            .ok()
            .and_then(|meta| Some((meta.len(), meta.modified().ok()?)));
        if self.path != path || self.stamp != stamp || stamp.is_none() {
            self.path = path.to_owned();
            self.stamp = stamp;
            self.lines = Arc::new(
                std::fs::read(path)
                    .map(|bytes| complete::word_file_lines(&bytes))
                    .unwrap_or_default(),
            );
        }
        Arc::clone(&self.lines)
    }
}

thread_local! {
    /// The UI runs on one thread; so does the cache.
    static WORD_FILE_CACHE: RefCell<WordFileCache> = RefCell::new(WordFileCache::default());
}

/// Where the candidates of the input completion come from, and how they are compared: the
/// settings of the document type (`CHokanMgr::Search`: `m_szHokanFile`, `m_bHokanLoHiCase`,
/// `m_bUseHokanByFile`, `m_bUseHokanByKeyword`).
struct Sources {
    ignore_case: bool,
    by_file: bool,
    by_keyword: bool,
    /// The word file as the type says it (empty: none).
    word_file: String,
    /// The folder of the settings file, where a relative path of the word file starts
    /// (`CTextInputStream_AbsIni`).
    ini_dir: Option<PathBuf>,
    types: Arc<DocumentTypes>,
    type_index: usize,
}

impl Sources {
    fn of(type_index: usize, cx: &gpui::App) -> Self {
        let (types, ini_dir) = crate::doc_types::with_user(cx, |user| {
            (
                user.shared_types(),
                crate::settings::history_file(user)
                    .and_then(|path| path.parent().map(Path::to_owned)),
            )
        });
        // A type that is not there completes as the defaults of Sakura do.
        let t = types.get(type_index).cloned().unwrap_or_default();
        Self {
            ignore_case: t.hokan_lo_hi_case,
            by_file: t.use_hokan_by_file,
            by_keyword: t.use_hokan_by_keyword,
            word_file: t.hokan_file,
            ini_dir,
            types,
            type_index,
        }
    }

    /// The lines of the word file, or none when the type has no word file or it cannot be read
    /// (`CDicMgr::HokanSearch` returns at once).
    ///
    /// The list is made again for every key typed while it is open, so the lines are kept until
    /// the file changes (its size or time), instead of reading and decoding it each time as
    /// Sakura does.
    fn word_file_lines(&self) -> Arc<Vec<String>> {
        if self.word_file.is_empty() {
            return Arc::default();
        }
        let path = Path::new(&self.word_file);
        let path = match (&self.ini_dir, path.is_relative()) {
            (Some(dir), true) => dir.join(path),
            _ => path.to_owned(),
        };
        WORD_FILE_CACHE.with(|cache| cache.borrow_mut().lines(&path))
    }

    /// The words of the keyword sets of the type, in the order of the slots
    /// (`CHokanMgr::HokanSearchByKeyword`).
    fn keywords(&self) -> impl Iterator<Item = &str> {
        self.types
            .keyword_sets_of(self.type_index)
            .into_iter()
            .flat_map(|(_, set)| set.words().iter().map(String::as_str))
    }
}

impl EditorView {
    /// The word before the caret and the candidates for it, or `None` if there is no word.
    ///
    /// The order is Sakura's (`CHokanMgr::Search`): the word file, the words of the document, the
    /// highlighted keywords.
    fn completion_for_caret(&self, cx: &gpui::App) -> Option<Completion> {
        let doc = self.document.read(cx).doc();
        let caret = self.cursor.caret();
        let line = doc.line(caret.line).ok().flatten()?;
        let before = line.text.get(..caret.col)?;
        let chars: Vec<char> = line.text.chars().collect();
        let idx = before.chars().count();
        let key = complete::left_word(&chars, idx)?;
        let sources = Sources::of(self.type_index(), cx);
        let mut items =
            complete::word_file_candidates(&sources.word_file_lines(), &key, sources.ignore_case);
        if sources.by_file {
            let in_document = complete::candidates(
                |i| doc.line(i).ok().flatten().map(|l| l.text),
                &key,
                sources.ignore_case,
                (caret.line, idx),
            );
            items = complete::merge_candidates(items, in_document, Some(complete::MAX_CANDIDATES));
        }
        if sources.by_keyword {
            let keywords =
                complete::keyword_candidates(sources.keywords(), &key, sources.ignore_case);
            items = complete::merge_candidates(items, keywords, None);
        }
        Some(Completion::new(key, items))
    }

    /// `F_HOKAN` (`Command_HOKAN` and `ShowHokanMgr` with `bAutoDecided`).
    pub(crate) fn start_completion(&mut self, cx: &mut Context<Self>) {
        self.support.completion = None;
        let Some(found) = self.completion_for_caret(cx) else {
            self.notice = Some(NOT_COMPLETABLE.to_owned());
            cx.notify();
            return;
        };
        match found.items.len() {
            // no candidate: nothing is shown (Sakura beeps only if the list was open)
            0 => {}
            // one candidate: no list, the word is completed
            1 => self.insert_completion(&found.items[0], cx),
            _ => self.support.completion = Some(found),
        }
        cx.notify();
    }

    /// Replaces the word before the caret with `word`, as one undo step (`CHokanMgr::DoHokan`:
    /// `F_WordDeleteToStart`, then `F_INSTEXT_W`).
    fn insert_completion(&mut self, word: &str, cx: &mut Context<Self>) {
        // A click can come while an IME composition is open: end it, as any command does.
        self.interrupt_composition(cx);
        // What a key macro records (`HandleCommand` records both commands in Sakura). The text
        // is recorded as typed chars, with the known difference of `record_typed_text` (`InsText`
        // has no command yet): in overwrite mode the replay overwrites where the insertion does not.
        self.record_call("F_WordDeleteToStart", &[]);
        self.record_typed_text(word);
        let delete = Registry::global().get("word_delete_to_start");
        self.run_with(cx, |command_cx| {
            command_cx.begin_block();
            if let Some(delete) = delete {
                // a failure (a box selection) leaves the word, and the candidate is inserted
                delete.run(command_cx, &[]).ok();
            }
            command_cx.insert_text(word).ok();
            command_cx.end_block();
        });
    }

    /// Before any command runs: closes the list unless the command keeps it
    /// (`PreprocessCommand_hokan`). `sakura` is the name of the function.
    pub(crate) fn completion_before_command(&mut self, sakura: &str) {
        if !KEEPING.contains(&sakura) {
            self.support.completion = None;
        }
    }

    /// After a command: the list follows the word before the caret when the text changed
    /// (`PostprocessCommand_hokan`: the search is made again, without the automatic completion of
    /// a single candidate), and closes when the caret moved without an edit.
    pub(crate) fn completion_after_command(&mut self, changes: &Changes, cx: &gpui::App) {
        if self.support.completion.is_none() {
            return;
        }
        if changes.text_changed() {
            self.support.completion = self
                .completion_for_caret(cx)
                .filter(|found| !found.items.is_empty());
        } else if changes.caret_moved || changes.selection_changed {
            self.support.completion = None;
        }
    }

    /// A key while the list is shown (`CHokanMgr::KeyProc`). Returns `true` if the list took it.
    pub(crate) fn completion_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) -> bool {
        let Some(list) = self.support.completion.as_mut() else {
            return false;
        };
        let Some(key) = list_key(&event.keystroke.key) else {
            return false;
        };
        match key {
            ListKey::Move(delta) => list.move_by(delta),
            ListKey::First => list.select(0),
            ListKey::Last => list.select(usize::MAX),
            ListKey::Close => self.support.completion = None,
            ListKey::Insert => {
                if let Some(list) = self.support.completion.take()
                    && let Some(word) = list.chosen()
                {
                    self.insert_completion(word, cx);
                }
            }
        }
        cx.notify();
        true
    }

    /// A click on row `row`: chooses it; a double click inserts it.
    fn completion_click(&mut self, row: usize, clicks: usize, cx: &mut Context<Self>) {
        let Some(list) = self.support.completion.as_mut() else {
            return;
        };
        list.select(row);
        if clicks >= 2
            && let Some(list) = self.support.completion.take()
            && let Some(word) = list.chosen()
        {
            self.insert_completion(word, cx);
        }
        cx.notify();
    }

    /// The list, below the start of the word before the caret.
    pub(crate) fn render_completion(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let list = self.support.completion.as_ref()?;
        let caret = self.cursor.caret();
        let start = Pos::new(caret.line, caret.col.saturating_sub(list.key.len()));
        let at = self.point_of(start, cx)?;
        // `ShowHokanMgr`: below the line, "4" pixels apart (`nY = poWin.y + height + 4`)
        let below = point(at.x, at.y + px(self.metrics.line_height + 4.0));
        let (family, size) = self.font();

        let rows = list
            .items
            .iter()
            .enumerate()
            .skip(list.top)
            .take(VISIBLE_ROWS)
            .map(|(i, word)| {
                let row = div()
                    .id(("hokan-row", i))
                    .px_2()
                    .whitespace_nowrap()
                    .child(word.clone())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            this.completion_click(i, event.click_count, cx);
                            cx.stop_propagation();
                        }),
                    );
                if i == list.selected {
                    row.bg(rgb(0x2f6fd0)).text_color(rgb(0xffffff))
                } else {
                    row
                }
            });
        let popup = div()
            .id("hokan-list")
            .occlude()
            .min_w(px(120.))
            .max_w(px(480.))
            .py_1()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x222222))
            .border_1()
            .border_color(rgb(0x888888))
            .font_family(family)
            .text_size(px(size))
            .children(rows);
        Some(
            deferred(
                anchored()
                    .position(below)
                    .snap_to_window_with_margin(px(8.))
                    .child(popup),
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(n: usize) -> Completion {
        Completion::new("a".into(), (0..n).map(|i| format!("a{i}")).collect())
    }

    #[test]
    fn the_first_candidate_is_chosen() {
        assert_eq!(list(3).chosen(), Some("a0"));
    }

    #[test]
    fn the_choice_stops_at_the_ends() {
        let mut l = list(3);
        l.move_by(-1);
        assert_eq!(l.selected, 0);
        l.move_by(5);
        assert_eq!(l.selected, 2);
        l.select(usize::MAX);
        assert_eq!(l.selected, 2);
    }

    #[test]
    fn the_list_scrolls_to_keep_the_choice_in_view() {
        let mut l = list(30);
        l.select(VISIBLE_ROWS);
        assert_eq!((l.selected, l.top), (VISIBLE_ROWS, 1));
        l.select(29);
        assert_eq!(l.top, 29 + 1 - VISIBLE_ROWS);
        l.select(0);
        assert_eq!(l.top, 0);
    }

    #[test]
    fn the_keys_of_the_list_follow_ckhokanmgr_keyproc() {
        assert_eq!(list_key("down"), Some(ListKey::Move(1)));
        assert_eq!(list_key("up"), Some(ListKey::Move(-1)));
        assert_eq!(list_key("enter"), Some(ListKey::Insert));
        assert_eq!(list_key("right"), Some(ListKey::Insert));
        assert_eq!(list_key("escape"), Some(ListKey::Close));
        // Tab is off in Sakura's defaults, and Left goes on to the editor
        assert_eq!(list_key("tab"), None);
        assert_eq!(list_key("left"), None);
        assert_eq!(list_key("a"), None);
    }
}
