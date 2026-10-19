//! The keyword help (キーワードヘルプ): the meaning of the word under the caret or of the
//! selection, from the dictionaries of the document type, shown in a tip when the mouse rests, and
//! in the right-click menu.
//!
//! Sakura:
//!
//! * `CEditView::KeyWordHelpSearchDict` and `KeySearchCore` (`sakura_core/view/CEditView_Search.cpp`)
//!   pick the key and look it up ([`lookup`]): the selection up to the end of its first line (at
//!   most 101 characters), else the word at the caret when 「キャレット位置の単語を辞書検索」
//!   (`m_bUseCaretKeyWord`, `F_TOGGLE_KEY_SEARCH`) is on. A key that was searched and not found is
//!   not searched again until it changes (or the settings of the type do).
//! * `CEditView::OnTimer` (`sakura_core/view/CEditView.cpp`) shows the tip when the mouse has not
//!   moved for 300 ms in the window and no menu is open; the tip hangs from the pointer, one row
//!   lower (`CTipWnd::Show`). A move of the pointer hides it.
//! * `CEditView::CreatePopUpMenu_R` and `AddKeyHelpMenu` add the items 「キーワードの説明を
//!   クリップボードにコピー」 and 「キーワード辞書を開く」 to the right-click menu, at the top or at
//!   the bottom as the type says (`nKeyHelpRMenuShowType`); `Command_MENU_RBUTTON`
//!   (`sakura_core/cmd/CViewCommander_CustMenu.cpp`) runs them.
//! * The font of the tip is `khlf` of `[Common]` (the 支援 page).
//!
//! * `CHokanMgr::ShowTip` and `CEditView::ShowKeywordHelp` show the meaning of the candidate that
//!   is chosen in the list of the input completion beside the list ([`EditorView::completion_help`]).
//!   Sakura puts it on the left when there is no room on the right; here the window moves it back
//!   into the window.

pub(crate) mod lookup;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use gpui::{
    AnyElement, App, Context, Global, InteractiveElement as _, IntoElement as _, MenuItem,
    ParentElement as _, Pixels, Point, SharedString, Styled as _, Window, actions, anchored,
    deferred, div, point, px, rgb,
};
use ume_commands::text::complete;
use ume_config::settings::DocumentTypes;
use ume_core::tag_jump::TagTarget;

use crate::view::EditorView;

actions!(
    ume,
    [
        /// The right-click menu: 「キーワードの説明をクリップボードにコピー」 (`IDM_COPYDICINFO`)
        CopyKeyHelp,
        /// The right-click menu: 「キーワード辞書を開く」 (`IDM_JUMPDICT`)
        OpenKeyHelpDict,
    ]
);

/// How long the mouse must rest before the tip is looked for (`300 < GetTickCount() - m_dwTipTimer`).
const REST: Duration = Duration::from_millis(300);

/// `STR_MENU_KEYWORDINFO` and its access key.
const COPY_LABEL: &str = "キーワードの説明をクリップボードにコピー";
/// `STR_MENU_OPENKEYWORDDIC` and its access key.
const OPEN_LABEL: &str = "キーワード辞書を開く";

/// `EKeyHelpRMenuType`
const RMENU_NONE: i32 = 0;
const RMENU_TOP: i32 = 1;
const RMENU_BOTTOM: i32 = 2;

/// The colors of a tip window: `COLOR_INFOBK` and `COLOR_INFOTEXT` of a standard Windows.
const TIP_BACK: u32 = 0xFFFFE1;
const TIP_TEXT: u32 = 0x000000;
const TIP_BORDER: u32 = 0x000000;

/// `CommonSetting_Search::m_bUseCaretKeyWord`: the word at the caret is looked up when nothing is
/// selected. Sakura keeps it in the shared memory only (the file does not have it), so it is for
/// the session and starts off.
#[derive(Clone, Copy, Default)]
struct CaretKeyWord(bool);

impl Global for CaretKeyWord {}

/// Whether the word at the caret is looked up (`m_bUseCaretKeyWord`).
pub(crate) fn caret_key_word(cx: &App) -> bool {
    cx.try_global::<CaretKeyWord>().is_some_and(|on| on.0)
}

/// Sets `m_bUseCaretKeyWord` (`Command_ToggleKeySearch`).
pub(crate) fn set_caret_key_word(cx: &mut App, on: bool) {
    cx.set_global(CaretKeyWord(on));
}

/// What tells that a file has not changed: its size and time.
type Stamp = Option<(u64, SystemTime)>;

/// The dictionary files that were read, with what tells that a file has not changed.
#[derive(Default)]
struct DictCache {
    files: HashMap<PathBuf, (Stamp, Arc<Vec<String>>)>,
}

/// More files than a type can have (`MAX_KEYHELP_FILE`) for the types that are used in turn.
const CACHE_LIMIT: usize = 64;

thread_local! {
    /// The UI runs on one thread; so does the cache.
    static DICT_CACHE: RefCell<DictCache> = RefCell::new(DictCache::default());
}

/// The file of a dictionary: a relative path starts at the folder of the settings file, else at
/// the folder of the program (`CTextInputStream_AbsIni`).
pub(crate) fn dict_path(path: &str, ini_dir: Option<&Path>) -> PathBuf {
    use crate::support::help::{exe_dir, inidir_or_exedir};
    inidir_or_exedir(path, ini_dir, exe_dir().as_deref())
}

/// The lines of a dictionary, or `None` when the file cannot be read (`CDicMgr::Search` returns at
/// once). A dictionary can be large and the lookup runs when the mouse rests, so the lines are
/// kept until the file changes (its size or time), instead of reading and decoding it each time.
pub(crate) fn dict_lines(path: &str, ini_dir: Option<&Path>) -> Option<Arc<Vec<String>>> {
    if path.is_empty() {
        return None;
    }
    let full = dict_path(path, ini_dir);
    let meta = std::fs::metadata(&full).ok()?;
    let stamp = meta.modified().ok().map(|time| (meta.len(), time));
    DICT_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((kept, lines)) = cache.files.get(&full)
            && stamp.is_some()
            && *kept == stamp
        {
            return Some(Arc::clone(lines));
        }
        let bytes = std::fs::read(&full).ok()?;
        let lines = Arc::new(complete::word_file_lines(&bytes));
        if cache.files.len() >= CACHE_LIMIT {
            cache.files.clear();
        }
        cache.files.insert(full, (stamp, Arc::clone(&lines)));
        Some(lines)
    })
}

/// The folder of the settings file, where a relative path starts.
fn ini_dir(cx: &App) -> Option<PathBuf> {
    crate::doc_types::with_user(cx, |user| {
        crate::settings::history_file(user).and_then(|path| path.parent().map(Path::to_owned))
    })
}

/// Which settings a lookup was made with: the types of the application as they were read (a new
/// value each time the type settings or the file change; it is kept, so that no other one can take
/// its place) and the type of the document. A result is used again only while this is the same.
#[derive(Clone)]
struct Setup {
    types: Arc<DocumentTypes>,
    index: usize,
}

impl PartialEq for Setup {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.types, &other.types) && self.index == other.index
    }
}

/// The tip that is shown.
pub(crate) struct Tip {
    pub text: String,
    /// The top left corner, in the window.
    pub at: Point<Pixels>,
}

/// What the view remembers of the keyword help (`CTipWnd`'s `m_cKey`, `m_KeyWasHit`, `m_cInfo`,
/// `m_nSearchDict`, `m_nSearchLine`).
#[derive(Default)]
pub(crate) struct KeyHelpState {
    /// The key searched last, whether it was found, and the settings it was searched with.
    searched: Option<(Setup, String, bool)>,
    /// The result of the last search that found something.
    found: Option<lookup::Found>,
    /// The tip, while it is shown.
    pub tip: Option<Tip>,
    /// Counts the moves of the pointer: a timer that finds a later count has been overtaken.
    moves: u64,
    /// Where the pointer was last.
    pointer: Option<Point<Pixels>>,
    /// The meaning shown beside the list of the input completion for the word that was chosen
    /// last, and that word (a search runs only when the choice changes).
    beside_list: RefCell<Option<(Setup, String, Option<String>)>>,
}

impl KeyHelpState {
    /// Forgets the meaning kept for the list of the input completion.
    pub(crate) fn forget_beside_list(&self) {
        self.beside_list.borrow_mut().take();
    }

    /// The text of the last result, with line breaks as the clipboard has them (`cInfo.Replace(
    /// L"\n", L"\r\n" )`).
    pub(crate) fn clipboard_text(&self) -> Option<String> {
        self.found.as_ref().map(|f| f.text.replace('\n', "\r\n"))
    }
}

impl EditorView {
    /// The settings a lookup is made with now ([`Setup`]).
    fn key_help_setup(&self, cx: &App) -> Setup {
        Setup {
            types: crate::doc_types::with_user(cx, |user| user.shared_types()),
            index: self.type_index(),
        }
    }

    /// Whether the keyword help of this view's type is on.
    fn key_help_enabled(&self, cx: &App) -> bool {
        let index = self.type_index();
        crate::doc_types::with_user(cx, |user| {
            user.shared_types()
                .get(index)
                .is_some_and(|t| t.use_key_word_help)
        })
    }

    /// Hides the tip. Returns whether one was shown.
    pub(crate) fn hide_key_help(&mut self, cx: &mut Context<Self>) -> bool {
        // The timers that are out are overtaken: a click, a key or the wheel ends the wait.
        self.support.key_help.moves += 1;
        let shown = self.support.key_help.tip.take().is_some();
        if shown {
            cx.notify();
        }
        shown
    }

    /// The pointer moved over the text (`CEditView::OnMOUSEMOVE`): a tip that is shown goes, and
    /// a timer starts to look for a new one when the pointer rests.
    pub(crate) fn key_help_pointer_moved(
        &mut self,
        position: Point<Pixels>,
        dragging: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = &mut self.support.key_help;
        if state.pointer == Some(position) {
            return;
        }
        state.pointer = Some(position);
        // A selection by the mouse shows no tip: the timers that are out are overtaken.
        if dragging {
            state.moves += 1;
            self.hide_key_help(cx);
            return;
        }
        self.key_help_restart(window, cx);
    }

    /// Hides the tip and starts the wait for a new one (`CViewCommander::HandleCommand` and
    /// `OnMOUSEMOVE`: `m_cTipWnd.Hide(); m_dwTipTimer = GetTickCount()`): after [`REST`] the word
    /// is looked up again. A key calls this too, so that moving the caret to another word with
    /// the keyboard shows the help of that word.
    pub(crate) fn key_help_restart(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.hide_key_help(cx);
        let moves = self.support.key_help.moves;
        if !self.key_help_enabled(cx) {
            return;
        }
        let position = self
            .support
            .key_help
            .pointer
            .unwrap_or_else(|| window.mouse_position());
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(REST).await;
            this.update_in(cx, |this, window, cx| {
                this.key_help_rest(moves, position, window, cx)
            })
            .ok();
        })
        .detach();
    }

    /// The pointer has rested for [`REST`] (`KeyWordHelpSearchDict( LID_SKH_ONTIMER )`): shows
    /// the tip if the key is in a dictionary.
    fn key_help_rest(
        &mut self,
        moves: u64,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = &self.support.key_help;
        if state.moves != moves || state.tip.is_some() {
            return;
        }
        // Not while the mouse selects or drags the scroll bar (the button is down).
        if self.drag != crate::view::Drag::None {
            return;
        }
        // The caret has the focus, and no menu is open (`m_bInMenuLoop`).
        if !self.focus_handle.is_focused(window) || self.custom_menu_is_open() {
            return;
        }
        // The pointer is in the window (`PtInRect`).
        let size = window.viewport_size();
        let mouse = window.mouse_position();
        if mouse.x < px(0.) || mouse.y < px(0.) || mouse.x > size.width || mouse.y > size.height {
            return;
        }
        if !self.key_help_lookup(cx) {
            return;
        }
        let Some(text) = self
            .support
            .key_help
            .found
            .as_ref()
            .map(|found| found.text.clone())
        else {
            return;
        };
        // `Show( po.x, po.y + GetTextMetrics().GetHankakuHeight() )`: one row below the pointer.
        let at = point(position.x, position.y + px(self.metrics.line_height));
        self.support.key_help.tip = Some(Tip { text, at });
        cx.notify();
    }

    /// `KeyWordHelpSearchDict`, from the key on: looks the key up in the dictionaries of the type.
    /// Returns whether the tip text (and [`KeyHelpState::found`]) is there.
    fn key_help_lookup(&mut self, cx: &App) -> bool {
        let t = self.type_settings(cx);
        if !t.use_key_word_help {
            return false;
        }
        // The selection (its first line), else the word at the caret when that is on.
        let key = match self.selected_first_line(cx) {
            Some(selected) => selected
                .chars()
                .take(lookup::STRNCMP_MAX + 1)
                .collect::<String>(),
            None if caret_key_word(cx) => {
                let caret = self.cursor.caret();
                let line = self
                    .document
                    .read(cx)
                    .doc()
                    .line(caret.line)
                    .ok()
                    .flatten()
                    .map(|l| l.text)
                    .unwrap_or_default();
                let (from, to) = crate::word::word_range(&line, caret.col);
                match line.get(from..to) {
                    Some(word) if !word.is_empty() => word.to_owned(),
                    _ => return false,
                }
            }
            None => return false,
        };
        if key.is_empty() {
            return false;
        }
        let setup = self.key_help_setup(cx);
        let state = &mut self.support.key_help;
        // Already searched and not found (with these settings).
        if matches!(&state.searched, Some((s, k, false)) if *s == setup && *k == key) {
            return false;
        }
        let ini = ini_dir(cx);
        let options = lookup::Options {
            all_search: t.use_key_help_all_search,
            key_disp: t.use_key_help_key_disp,
            prefix: t.use_key_help_prefix,
            dicts: &t.key_help,
        };
        let found = lookup::key_search(&options, &key, |path| dict_lines(path, ini.as_deref()));
        state.searched = Some((setup, key, found.is_some()));
        let hit = found.is_some();
        state.found = found;
        hit
    }

    /// The items for the right-click menu, before and after the items of the menu
    /// (`AddKeyHelpMenu`): the separators included. Nothing when the type does not show them here
    /// or the word is not in a dictionary. `by_mouse` is whether the menu opens at the pointer
    /// (else the pointer must be in the window).
    pub(crate) fn key_help_menu_items(
        &mut self,
        by_mouse: bool,
        window: &Window,
        cx: &App,
    ) -> (Vec<MenuItem>, Vec<MenuItem>) {
        let none = (Vec::new(), Vec::new());
        let show = self.type_settings(cx).key_help_rmenu_show_type;
        if show == RMENU_NONE || !matches!(show, RMENU_TOP | RMENU_BOTTOM) {
            return none;
        }
        if !by_mouse {
            let bounds = gpui::Bounds::from_corners(point(px(0.), px(0.)), {
                let size = window.viewport_size();
                point(size.width, size.height)
            });
            if !bounds.contains(&window.mouse_position()) {
                return none;
            }
        }
        if !self.key_help_lookup(cx) {
            return none;
        }
        let items = vec![
            MenuItem::action(crate::menu::menu_label(COPY_LABEL, Some('K')), CopyKeyHelp),
            MenuItem::action(
                crate::menu::menu_label(OPEN_LABEL, Some('L')),
                OpenKeyHelpDict,
            ),
        ];
        if show == RMENU_TOP {
            let mut top = items;
            top.push(MenuItem::separator());
            (top, Vec::new())
        } else {
            let mut bottom = vec![MenuItem::separator()];
            bottom.extend(items);
            (Vec::new(), bottom)
        }
    }

    /// 「キーワードの説明をクリップボードにコピー」 (`IDM_COPYDICINFO`).
    pub(crate) fn copy_key_help(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = self.support.key_help.clipboard_text() {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
        }
    }

    /// 「キーワード辞書を開く」 (`IDM_JUMPDICT`): opens the first dictionary that had the word, at
    /// the line (`TagJumpSub` with the path relative to the settings file).
    pub(crate) fn open_key_help_dict(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let t = self.type_settings(cx);
        // `if(m_pTypeData->m_bUseKeyWordHelp)`
        if !t.use_key_word_help {
            return;
        }
        let Some(found) = self.support.key_help.found.clone() else {
            return;
        };
        let Some(dict) = t.key_help.get(found.dict) else {
            return;
        };
        let target = TagTarget {
            path: dict_path(&dict.path, ini_dir(cx).as_deref()),
            line: Some(found.line),
            column: None,
        };
        crate::file_ops::start(window, cx, async move |view, cx| {
            crate::file_ops::tag_jump::jump_to_target(&view, cx, &target, false).await;
        });
    }

    /// The font of the tip: `khlf` and `khps` of `[Common]` (the 支援 page), else the font of the
    /// text.
    fn key_help_font(&self, cx: &App) -> (SharedString, f32) {
        let (family, size) = self.font();
        let font = crate::support::prefs::current(cx).key_help_font;
        let Some(font) = font else {
            return (family, size);
        };
        let family = crate::settings::view_settings::font_family_for(
            &font.log_font.face_name,
            ume_config::location::Os::current(),
        )
        .map_or(family, SharedString::from);
        let size = crate::settings::view_settings::points_to_px(
            font.point_size_for_dpi(crate::settings::view_settings::DPI),
        );
        (family, size)
    }

    /// The meaning of the candidate `word` of the input completion, for the tip beside the list
    /// (`CEditView::ShowKeywordHelp`): the same lookup as for the tip of the mouse, with the
    /// candidate as the key. `None` when the help is off or the word is in no dictionary.
    pub(crate) fn completion_help(&self, word: &str, cx: &App) -> Option<String> {
        if !self.key_help_enabled(cx) {
            return None;
        }
        let setup = self.key_help_setup(cx);
        if let Some((kept, key, text)) = &*self.support.key_help.beside_list.borrow()
            && *kept == setup
            && key == word
        {
            return text.clone();
        }
        let t = self.type_settings(cx);
        let ini = ini_dir(cx);
        let options = lookup::Options {
            all_search: t.use_key_help_all_search,
            key_disp: t.use_key_help_key_disp,
            prefix: t.use_key_help_prefix,
            dicts: &t.key_help,
        };
        let text = lookup::key_search(&options, word, |path| dict_lines(path, ini.as_deref()))
            .map(|found| found.text);
        *self.support.key_help.beside_list.borrow_mut() =
            Some((setup, word.to_owned(), text.clone()));
        text
    }

    /// The window of a tip with `text`, in the font of the keyword help.
    pub(crate) fn key_help_window(&self, text: &str, cx: &App) -> gpui::Stateful<gpui::Div> {
        let (family, size) = self.key_help_font(cx);
        let rows = text.split('\n').map(|row| {
            // An empty row is as high as a row of text (`szDummy`).
            div().child(if row.is_empty() {
                SharedString::from(" ")
            } else {
                SharedString::from(row.to_owned())
            })
        });
        div()
            .id("key-help-tip")
            .max_w(px(640.))
            .px_1()
            .py_1()
            .bg(rgb(TIP_BACK))
            .text_color(rgb(TIP_TEXT))
            .border_1()
            .border_color(rgb(TIP_BORDER))
            .font_family(family)
            .text_size(px(size))
            .children(rows)
    }

    /// The tip, over the text.
    pub(crate) fn render_key_help(&self, cx: &App) -> Option<AnyElement> {
        let tip = self.support.key_help.tip.as_ref()?;
        let popup = self.key_help_window(&tip.text, cx);
        Some(
            deferred(
                anchored()
                    .position(tip.at)
                    .snap_to_window_with_margin(px(8.))
                    .child(popup),
            )
            .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests;
