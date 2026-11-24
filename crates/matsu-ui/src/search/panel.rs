//! The find, replace and go-to-line panel: Sakura's 検索 / 置換 / 指定行へジャンプ dialogs
//! (`sakura_core/dlg/CDlgFind.cpp`, `CDlgReplace.cpp`, `CDlgJump.cpp`; the layout is `IDD_FIND`,
//! `IDD_REPLACE`, `IDD_JUMP` in `sakura_core/sakura_rc.rc`).
//!
//! Sakura's find and replace dialogs are modeless: they stay open while the text is edited. A
//! panel inside the editor window does the same on every OS without a second GPUI window (whose
//! focus, z-order and IME would differ per platform), the text can be edited next to it, and it
//! works on Linux where WSLg gives windows no decorations. It is drawn over the top right of the
//! editor, as an editor's find widget usually is.
//!
//! The panel keeps no search state of its own: the options, the pattern and the replace settings
//! live in the view's [`SearchState`](super::state::SearchState), so the menu, F3 and the panel
//! always agree.

use crate::a11y::Labelled as _;
use crate::access_keys::{Entry, Found, OnAccessKey as _, pick};
use crate::dialog_focus::KeepTabInside as _;
use crate::input_select::{focus_selecting, input_field};
use crate::support::help::OnHelpKey as _;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::{Disableable as _, Selectable as _, h_flex, v_flex};
use matsu_core::buffer::Pos;
use matsu_core::search::{Direction, ReplaceTarget};

use super::history;
use super::ops::SearchOutcome;
use super::state::{NO_PATTERN_MESSAGE, ReplaceSettings, SearchOptions};
use crate::saved_state::other::{JumpUnit, jump_unit, remember_jump_unit};
use crate::view::EditorView;

/// The labels with their access keys (Alt+letter, [`SearchPanel::use_access_key`]). They are Sakura's
/// (`IDD_FIND`, `IDD_REPLACE`, `IDD_JUMP` in `sakura_core/sakura_rc.rc`).
const WHOLE_WORD: &str = "単語単位で探す(W)";
const MATCH_CASE: &str = "英大文字と小文字を区別する(C)";
const REGEX: &str = "正規表現(E)";
const NOTIFY_NOT_FOUND: &str = "見つからないときにメッセージを表示(M)";
const WRAP: &str = "先頭（末尾）から再検索する(Z)";
const AUTO_CLOSE: &str = "検索ダイアログを自動的に閉じる(L)";
const AUTO_CLOSE_REPLACE: &str = "置換ダイアログを自動的に閉じる(L)";
const CONSECUTIVE: &str = "「すべて置換」は置換の繰返し(I)";
const TARGET_MATCH: &str = "選択文字(0)";
const TARGET_BEFORE: &str = "選択始点(1)挿入";
const TARGET_AFTER: &str = "選択終点(2)追加";
const TARGET_LINE: &str = "行削除(3)";
const AREA_SELECTION: &str = "選択範囲(S)";
const AREA_ALL: &str = "ファイル全体(O)";
const PREV: &str = "上検索(U)";
const NEXT: &str = "下検索(D)";
const MARK: &str = "該当行マーク(B)";
const REPLACE_ONE: &str = "置換(R)";
const REPLACE_ALL: &str = "すべて置換(A)";
const CLOSE: &str = "閉じる(X)";
const HELP: &str = crate::support::help::BUTTON_LABEL;
const BEFORE_LABEL: &str = "置換前(N)";
const AFTER_LABEL: &str = "置換後(P)";
const LINE_LABEL: &str = "行番号(N)";
/// `IDC_EDIT_LINENUM` is 50 dialog units wide, spin button included (`IDD_JUMP`, `sakura_rc.rc`
/// line 91: the edit spans x 40 to 90 and the spin sits at 82 to 91). The dialog font is 9 point
/// MS PGothic, whose half-width letter is 6 px at 96 dpi, so a dialog unit is 1.5 px (the width of
/// a letter / 4); the box is 75 px. The arrows (`spin::ARROW_WIDTH`) are inside it.
const LINE_BOX_WIDTH: f32 = 50. * 1.5;
// `IDC_RADIO_LINENUM_LAYOUT` and `IDC_RADIO_LINENUM_CRLF` (`IDD_JUMP`)
const UNIT_ROWS: &str = "折り返し単位の行番号(R)";
const UNIT_LINES: &str = "改行単位の行番号(W)";

/// A check box of the options.
#[derive(Clone, Copy, Debug)]
enum Opt {
    WholeWord,
    MatchCase,
    Regex,
    NotifyNotFound,
    Wrap,
    AutoClose,
    AutoCloseReplace,
}

impl Opt {
    fn switch(self, options: &mut SearchOptions) {
        let flag = match self {
            Opt::WholeWord => &mut options.whole_word,
            Opt::MatchCase => &mut options.match_case,
            Opt::Regex => &mut options.regex,
            Opt::NotifyNotFound => &mut options.notify_not_found,
            Opt::Wrap => &mut options.wrap,
            Opt::AutoClose => &mut options.auto_close,
            Opt::AutoCloseReplace => &mut options.auto_close_replace,
        };
        *flag = !*flag;
    }
}

/// What an access key does.
#[derive(Clone, Copy, Debug)]
enum Use {
    Option(Opt),
    Consecutive,
    Target(ReplaceTarget),
    /// 選択範囲 (true) or ファイル全体.
    Area(bool),
    Find(Direction),
    Mark,
    ReplaceOne,
    ReplaceAll,
    Close,
    /// ヘルプ(H): the page of the dialog.
    Help,
    /// The label in front of a box: the focus goes to the box.
    FocusSearch,
    FocusReplacement,
    FocusLine,
    /// The unit of the line number of the go-to-line panel.
    Unit(JumpUnit),
}

/// Which dialog the panel is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelMode {
    Find,
    Replace,
    Jump,
}

/// The names of a combo box and of its history button, from the visible labels of Sakura's
/// dialogs: 検索する文字列 (`IDD_FIND`), 置換前 and 置換後 (`IDD_REPLACE`).
fn combo_names(combo: Combo, mode: PanelMode) -> (&'static str, &'static str) {
    match (combo, mode) {
        (Combo::Search, PanelMode::Replace) => ("置換前", "置換前の履歴"),
        (Combo::Search, _) => ("検索する文字列", "検索する文字列の履歴"),
        (Combo::Replace, _) => ("置換後", "置換後の履歴"),
    }
}

/// The input that a history drop-down belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Combo {
    /// 検索する文字列 (`IDC_COMBO_TEXT`): the search strings.
    Search,
    /// 置換後の文字列 (`IDC_COMBO_TEXT2`): the replace strings.
    Replace,
}

/// A drop-down list that is open: Sakura's combo box shows `m_aSearchKeys` or `m_aReplaceKeys`
/// (`CDlgFind::OnCbnDropDown`, `CDlgReplace::OnCbnDropDown`).
#[derive(Clone, Copy, Debug)]
struct DropDown {
    combo: Combo,
    /// The entry that Enter takes.
    highlight: usize,
}

pub struct SearchPanel {
    editor: WeakEntity<EditorView>,
    /// The panel as a whole: Tab stays inside it.
    pub(crate) scope: FocusHandle,
    dropdown: Option<DropDown>,
    /// Scrolls the open list to the highlighted entry.
    list_scroll: ScrollHandle,
    mode: PanelMode,
    text: Entity<InputState>,
    replacement: Entity<InputState>,
    line: Entity<InputState>,
    /// The radio of the unit as the user set it in this go-to-line panel: the dialog's own control
    /// state, as in Sakura's modal `CDlgJump`. ジャンプ writes it to `m_bLineNumIsCRLF_ForJump`
    /// before it reads the number (`GetData`, `CDlgJump.cpp` 307-332); with a number that is not
    /// valid the dialog stays open with its radio as it was set (`IDC_BUTTON_JUMP`, 125-134), so
    /// `chosen` stays too. It lasts until the panel is closed or reused for the jump, and what
    /// other windows or macros do meanwhile does not change it. While it is `None` the panel
    /// shows and commits the remembered unit (`jump_unit`, the one owner of that value), so a
    /// macro's `Jump` is not written back over.
    chosen: Option<JumpUnit>,
    /// Where the caret was when the panel opened: the search start position that
    /// `F_JUMP_SRCHSTARTPOS` returns to, registered with the first search (`m_bSearch`,
    /// `m_ptEscCaretPos_PHY` in `CDlgFind.cpp`).
    pending_start: Option<Pos>,
    _subscriptions: Vec<Subscription>,
}

impl SearchPanel {
    fn new(
        editor: WeakEntity<EditorView>,
        mode: PanelMode,
        pending_start: Pos,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = |placeholder: &'static str, window: &mut Window, cx: &mut Context<Self>| {
            cx.new(|cx| input_field(window, cx).placeholder(placeholder))
        };
        let text = input("検索する文字列", window, cx);
        let replacement = input("置換後の文字列", window, cx);
        let line = input("行番号", window, cx);
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe_in(
            &text,
            window,
            |this: &mut Self, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.sync(cx),
                InputEvent::PressEnter { shift, .. } => {
                    // The key binding of Enter runs before the key listeners (`combo_key`), so
                    // an open list takes the key here: only the row is taken, nothing is found.
                    if this.take_open_row(Combo::Search, window, cx) {
                        return;
                    }
                    let direction = if *shift {
                        Direction::Backward
                    } else {
                        Direction::Forward
                    };
                    this.find(direction, window, cx);
                }
                _ => {}
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &replacement,
            window,
            |this: &mut Self, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.sync(cx),
                InputEvent::PressEnter { .. } => {
                    if this.take_open_row(Combo::Replace, window, cx) {
                        return;
                    }
                    this.replace_one(window, cx);
                }
                _ => {}
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &line,
            window,
            |this: &mut Self, _, event: &InputEvent, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.jump(window, cx);
                }
            },
        ));
        // A macro's `Jump` (or a jump of another panel) changes the remembered unit while this
        // panel is open (another tab or a split; Sakura's dialog is modal): redraw the radio
        // buttons. Nothing is copied; a choice made here stays as clicked.
        subscriptions.push(cx.observe_global::<JumpUnit>(|_, cx| cx.notify()));
        Self {
            editor,
            scope: cx.focus_handle(),
            dropdown: None,
            list_scroll: ScrollHandle::new(),
            mode,
            text,
            replacement,
            line,
            chosen: None,
            pending_start: Some(pending_start),
            _subscriptions: subscriptions,
        }
    }

    /// The unit that the go-to-line panel shows and would commit: the click made here, else the
    /// remembered unit.
    pub(crate) fn unit(&self, cx: &App) -> JumpUnit {
        self.chosen.unwrap_or_else(|| jump_unit(cx))
    }

    pub fn mode(&self) -> PanelMode {
        self.mode
    }

    /// The input that has the focus for the mode.
    fn main_input(&self) -> &Entity<InputState> {
        match self.mode {
            PanelMode::Jump => &self.line,
            _ => &self.text,
        }
    }

    pub fn focus_main(&self, window: &mut Window, cx: &mut Context<Self>) {
        // a dialog opens with its first edit focused and its text selected (`WM_INITDIALOG`)
        focus_selecting(self.main_input(), window, cx);
    }

    /// Moves the focus to the replacement input.
    pub fn focus_replacement(&self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.replacement.clone();
        input.update(cx, |state, cx| state.focus(window, cx));
    }

    fn combo_input(&self, combo: Combo) -> &Entity<InputState> {
        match combo {
            Combo::Search => &self.text,
            Combo::Replace => &self.replacement,
        }
    }

    fn combo_keys(combo: Combo, cx: &App) -> Vec<String> {
        match combo {
            Combo::Search => history::search_keys(cx),
            Combo::Replace => history::replace_keys(cx),
        }
    }

    /// The combo box that has the focus (the Jump panel has none).
    fn focused_combo(&self, window: &Window, cx: &App) -> Option<Combo> {
        let has_focus =
            |input: &Entity<InputState>| input.read(cx).focus_handle(cx).is_focused(window);
        match self.mode {
            PanelMode::Jump => None,
            PanelMode::Find => has_focus(&self.text).then_some(Combo::Search),
            PanelMode::Replace if has_focus(&self.text) => Some(Combo::Search),
            PanelMode::Replace => has_focus(&self.replacement).then_some(Combo::Replace),
        }
    }

    /// Opens the drop-down of a combo box, or closes it. With no history there is nothing to open.
    fn toggle_dropdown(&mut self, combo: Combo, cx: &mut Context<Self>) {
        if self.dropdown.is_some_and(|d| d.combo == combo) {
            self.dropdown = None;
        } else {
            let keys = Self::combo_keys(combo, cx);
            let current = self.combo_input(combo).read(cx).value();
            self.dropdown = (!keys.is_empty()).then(|| DropDown {
                combo,
                highlight: keys.iter().position(|k| k == current.as_str()).unwrap_or(0),
            });
            if let Some(open) = self.dropdown {
                self.list_scroll.scroll_to_item(open.highlight);
            }
        }
        cx.notify();
    }

    /// Enter in an open list of this combo box (`CBS_DROPDOWN` with the list dropped down): the
    /// highlighted row goes into the box and the list closes; the dialog's default button is not
    /// pressed. Returns whether a list of this box was open.
    fn take_open_row(&mut self, combo: Combo, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(open) = self.dropdown.filter(|d| d.combo == combo) else {
            return false;
        };
        match Self::combo_keys(combo, cx).get(open.highlight) {
            Some(text) => self.pick(combo, text.clone(), window, cx),
            None => {
                self.dropdown = None;
                cx.notify();
            }
        }
        true
    }

    /// Puts a string of the history in the input, closes the list and gives the focus back to
    /// the input.
    fn pick(&mut self, combo: Combo, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.dropdown = None;
        let input = self.combo_input(combo).clone();
        input.update(cx, |state, cx| {
            state.set_value(text, window, cx);
            state.focus(window, cx);
        });
        self.sync(cx);
        cx.notify();
    }

    /// Up and Down in a combo box whose list is closed go through the list and put the string in
    /// the edit field. From a string that is not in the list, both take the first.
    fn step(&mut self, combo: Combo, down: bool, window: &mut Window, cx: &mut Context<Self>) {
        let keys = Self::combo_keys(combo, cx);
        if keys.is_empty() {
            return;
        }
        let current = self.combo_input(combo).read(cx).value();
        let next = match (keys.iter().position(|k| k == current.as_str()), down) {
            (Some(i), true) => (i + 1).min(keys.len() - 1),
            (Some(i), false) => i.saturating_sub(1),
            (None, _) => 0,
        };
        self.pick(combo, keys[next].clone(), window, cx);
    }

    /// The keys of the combo boxes: Alt+Down and F4 open or close the list; with the list open
    /// Up, Down, Enter and Escape work in it; with it closed Up and Down go through the strings.
    /// Returns whether the key was used.
    fn combo_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let focused = self.focused_combo(window, cx);
        if self.dropdown.is_some_and(|d| Some(d.combo) != focused) {
            // The focus left the combo box: the list closes.
            self.dropdown = None;
            cx.notify();
        }
        let Some(combo) = focused else { return false };
        let key = event.keystroke.key.as_str();
        let m = &event.keystroke.modifiers;
        let plain = !m.alt && !m.control && !m.platform && !m.shift;
        if key == "f4" || (key == "down" && m.alt) {
            self.toggle_dropdown(combo, cx);
            return true;
        }
        let Some(open) = self.dropdown else {
            return match key {
                "down" | "up" if plain => {
                    self.step(combo, key == "down", window, cx);
                    true
                }
                _ => false,
            };
        };
        let keys = Self::combo_keys(combo, cx);
        match key {
            "down" | "up" if plain && !keys.is_empty() => {
                let highlight = if key == "down" {
                    (open.highlight + 1).min(keys.len() - 1)
                } else {
                    open.highlight.saturating_sub(1)
                };
                self.dropdown = Some(DropDown { highlight, ..open });
                self.list_scroll.scroll_to_item(highlight);
                cx.notify();
            }
            "escape" => {
                self.dropdown = None;
                cx.notify();
            }
            _ => return false,
        }
        true
    }

    /// An input with the button that opens its history, and the list under it while it is open.
    fn combo_box(&self, combo: Combo, cx: &mut Context<Self>) -> impl IntoElement {
        let (name, toggle_name) = combo_names(combo, self.mode);
        let (id, toggle_id) = match combo {
            Combo::Search => ("search-history", "search-history-toggle"),
            Combo::Replace => ("replace-history", "replace-history-toggle"),
        };
        let mut column = v_flex().gap_1().flex_1().child(
            h_flex()
                .gap_1()
                .child(
                    div()
                        .flex_1()
                        .child(Input::new(self.combo_input(combo)).aria_label(name)),
                )
                .child(
                    div().debug_selector(|| toggle_id.to_owned()).child(
                        Button::new(toggle_id)
                            .label("▼")
                            .accessibility_label(toggle_name)
                            // Sakura's 置換前 and 置換後 are combo boxes (`IDD_REPLACE`,
                            // `CBS_DROPDOWN`): Tab goes from one to the next control, and the
                            // arrow is reached by the mouse, F4 or Alt+Down only.
                            .tab_stop(false)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                // The button does not take the focus; the keys of the list go
                                // to the combo box that has it, so give it to this one.
                                let input = this.combo_input(combo).clone();
                                input.update(cx, |state, cx| state.focus(window, cx));
                                this.toggle_dropdown(combo, cx);
                            })),
                    ),
                ),
        );
        if let Some(open) = self.dropdown.filter(|d| d.combo == combo) {
            let entries = Self::combo_keys(combo, cx)
                .into_iter()
                .enumerate()
                .map(|(i, text)| {
                    let picked = text.clone();
                    crate::a11y::list_row(div().id((id, i)), &text, i == open.highlight)
                        .px_2()
                        .truncate()
                        .when(i == open.highlight, |d| d.bg(rgb(0xcce4f7)))
                        .hover(|d| d.bg(rgb(0xe5f1fb)))
                        .child(SharedString::from(text))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.pick(combo, picked.clone(), window, cx);
                        }))
                });
            column = column.child(
                crate::a11y::list_box(div().id(id), toggle_name)
                    .max_h(px(150.))
                    .overflow_y_scroll()
                    .track_scroll(&self.list_scroll)
                    .border_1()
                    .border_color(rgb(0xb0b0b0))
                    .bg(rgb(0xffffff))
                    .children(entries),
            );
        }
        column
    }

    /// Takes the pattern and the replacement from the inputs into the view's search state.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let (pattern, replacement) = (
            self.text.read(cx).value().to_string(),
            self.replacement.read(cx).value().to_string(),
        );
        self.editor
            .update(cx, |editor, cx| {
                editor.search.set_pattern(&pattern);
                editor.search.replace.text = replacement;
                cx.notify();
            })
            .ok();
    }

    /// 「すべて置換」は置換の繰返し.
    fn set_consecutive(&mut self, on: bool, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| {
                editor.search.replace.consecutive_all = on;
                cx.notify();
            })
            .ok();
        cx.notify();
    }

    /// 置換対象.
    fn set_target(&mut self, target: ReplaceTarget, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| {
                editor.search.replace.target = target;
                cx.notify();
            })
            .ok();
        cx.notify();
    }

    /// 範囲: the selection (true) or the whole file.
    fn set_area(&mut self, in_selection: bool, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| {
                editor.search.replace.in_selection = in_selection;
                cx.notify();
            })
            .ok();
        cx.notify();
    }

    /// 折り返し単位 / 改行単位 of the go-to-line panel.
    pub(crate) fn set_unit(&mut self, unit: JumpUnit, cx: &mut Context<Self>) {
        self.chosen = Some(unit);
        cx.notify();
    }

    /// Alt with the letter of a label (`IsDialogMessage`): switches the check box, chooses the radio
    /// button, presses the button, or moves the focus to the box after the label. Returns whether the
    /// letter was one of the dialog's (then the key is not typed into the box). The find and replace
    /// dialogs share their letters; none is on two controls of one dialog.
    pub(crate) fn use_access_key(
        &mut self,
        letter: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // Only what the keys need is copied: the editor stays the owner of the replacement text.
        let (options, consecutive_all) = self
            .editor
            .read_with(cx, |editor, _| {
                (
                    editor.search.options(),
                    editor.search.replace.consecutive_all,
                )
            })
            .unwrap_or_else(|_| (SearchOptions::default(), false));
        let mode = self.mode;
        let (find, replacing) = (mode == PanelMode::Find, mode == PanelMode::Replace);
        let searching = mode != PanelMode::Jump;
        let entries = [
            Entry::labelled(
                WHOLE_WORD,
                searching && !options.regex,
                Use::Option(Opt::WholeWord),
            ),
            Entry::labelled(MATCH_CASE, searching, Use::Option(Opt::MatchCase)),
            Entry::labelled(REGEX, searching, Use::Option(Opt::Regex)),
            Entry::labelled(
                NOTIFY_NOT_FOUND,
                searching,
                Use::Option(Opt::NotifyNotFound),
            ),
            Entry::labelled(WRAP, searching, Use::Option(Opt::Wrap)),
            Entry::labelled(AUTO_CLOSE, find, Use::Option(Opt::AutoClose)),
            Entry::labelled(
                AUTO_CLOSE_REPLACE,
                replacing,
                Use::Option(Opt::AutoCloseReplace),
            ),
            Entry::labelled(CONSECUTIVE, replacing && options.regex, Use::Consecutive),
            Entry::labelled(TARGET_MATCH, replacing, Use::Target(ReplaceTarget::Match)),
            // the keys of these two labels are in the middle of them
            Entry::keyed('1', replacing, Use::Target(ReplaceTarget::InsertBefore)),
            Entry::keyed('2', replacing, Use::Target(ReplaceTarget::InsertAfter)),
            Entry::labelled(
                TARGET_LINE,
                replacing,
                Use::Target(ReplaceTarget::DeleteLine),
            ),
            Entry::labelled(AREA_SELECTION, replacing, Use::Area(true)),
            Entry::labelled(AREA_ALL, replacing, Use::Area(false)),
            Entry::labelled(PREV, searching, Use::Find(Direction::Backward)),
            Entry::labelled(NEXT, searching, Use::Find(Direction::Forward)),
            Entry::labelled(MARK, searching, Use::Mark),
            Entry::labelled(REPLACE_ONE, replacing, Use::ReplaceOne),
            Entry::labelled(REPLACE_ALL, replacing, Use::ReplaceAll),
            Entry::labelled(CLOSE, true, Use::Close),
            Entry::labelled(HELP, true, Use::Help),
            Entry::labelled(BEFORE_LABEL, replacing, Use::FocusSearch),
            Entry::labelled(AFTER_LABEL, replacing, Use::FocusReplacement),
            Entry::labelled(LINE_LABEL, mode == PanelMode::Jump, Use::FocusLine),
            Entry::labelled(
                UNIT_ROWS,
                mode == PanelMode::Jump,
                Use::Unit(JumpUnit::Rows),
            ),
            Entry::labelled(
                UNIT_LINES,
                mode == PanelMode::Jump,
                Use::Unit(JumpUnit::Lines),
            ),
        ];
        let used = match pick(letter, &entries) {
            Found::Nothing => return false,
            Found::Several => return true,
            Found::One(used) => used,
        };
        match used {
            Use::Option(opt) => self.set_option(|o| opt.switch(o), cx),
            Use::Consecutive => self.set_consecutive(!consecutive_all, cx),
            Use::Target(target) => self.set_target(target, cx),
            Use::Area(in_selection) => self.set_area(in_selection, cx),
            Use::Find(direction) => self.find(direction, window, cx),
            Use::Mark => self.mark_lines(window, cx),
            Use::ReplaceOne => self.replace_one(window, cx),
            Use::ReplaceAll => self.replace_all(window, cx),
            Use::Close => self.close(window, cx),
            Use::Help => self.open_help(cx),
            Use::FocusSearch => focus_selecting(&self.text, window, cx),
            Use::FocusReplacement => focus_selecting(&self.replacement, window, cx),
            Use::FocusLine => focus_selecting(&self.line, window, cx),
            Use::Unit(unit) => self.set_unit(unit, cx),
        }
        true
    }

    /// Changes an option of the view's search state.
    fn set_option(&mut self, change: impl FnOnce(&mut SearchOptions), cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| {
                let mut options = editor.search.options();
                change(&mut options);
                editor.search.set_options(options);
                cx.notify();
            })
            .ok();
        cx.notify();
    }

    fn register_start(&mut self, cx: &mut Context<Self>) {
        if let Some(pos) = self.pending_start.take() {
            self.editor
                .update(cx, |editor, _| editor.search.set_start_pos(Some(pos)))
                .ok();
        }
    }

    /// 上検索 / 下検索.
    fn find(&mut self, direction: Direction, window: &mut Window, cx: &mut Context<Self>) {
        self.sync(cx);
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        if self.text.read(cx).value().is_empty() {
            editor.update(cx, |editor, cx| {
                editor.info_box(NO_PATTERN_MESSAGE, window, cx);
            });
            return;
        }
        let mode = self.mode;
        let (outcome, auto_close) = editor.update(cx, |editor, cx| {
            // `CDlgFind::GetData` and `CDlgReplace::GetData` put the strings in the history
            // before they search.
            remember_keys(editor, mode, cx);
            let outcome = editor.search(direction, window, cx);
            (outcome, editor.search.options().auto_close)
        });
        self.register_start(cx);
        // The find dialog closes itself after a search (`m_bAutoCloseDlgFind`); the replace
        // dialog stays.
        if outcome == SearchOutcome::Found && auto_close && self.mode == PanelMode::Find {
            self.close(window, cx);
        }
    }

    /// 該当行マーク: bookmarks the lines that match. The find dialog closes itself after it
    /// (`m_bAutoCloseDlgFind`), the replace dialog stays (`IDC_BUTTON_SETMARK` in
    /// `sakura_core/dlg/CDlgFind.cpp` and `CDlgReplace.cpp`). With no pattern nothing happens.
    fn mark_lines(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync(cx);
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        if self.text.read(cx).value().is_empty() {
            return;
        }
        let mode = self.mode;
        let auto_close = editor.update(cx, |editor, cx| {
            remember_keys(editor, mode, cx);
            editor.mark_matching_lines(window, cx);
            editor.search.options().auto_close && editor.search.pattern_error().is_none()
        });
        if auto_close && self.mode == PanelMode::Find {
            self.close(window, cx);
        }
    }

    fn replace_one(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync(cx);
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        let mode = self.mode;
        editor.update(cx, |editor, cx| {
            if editor.search.pattern().is_empty() {
                editor.info_box("置換条件を指定してください。", window, cx);
            } else {
                remember_keys(editor, mode, cx);
                editor.replace_next(window, cx);
            }
        });
        self.register_start(cx);
    }

    pub(super) fn replace_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.sync(cx);
        let Some(editor) = self.editor.upgrade() else {
            return;
        };
        let mode = self.mode;
        let finished = editor.update(cx, |editor, cx| {
            if editor.search.pattern().is_empty() {
                editor.info_box("置換条件を指定してください。", window, cx);
                return false;
            }
            remember_keys(editor, mode, cx);
            // The count is told by the dialog (`TopOkMessage( STR_DLGREPLC_REPLACE )`).
            let Some(count) = editor.replace_all(window, cx) else {
                return false;
            };
            editor.info_box(&super::state::replaced_count_message(count), window, cx);
            // Nothing in MATSU cancels it (`m_bCanceled`), so it always ends the way Sakura's does.
            editor.search.options().auto_close_replace
        });
        self.register_start(cx);
        // After すべて置換 the dialog closes itself (`m_bAutoCloseDlgReplace`, `OnBnClicked` of
        // `IDC_BUTTON_REPALCEALL` in `CDlgReplace.cpp` 570-585), also when nothing was replaced; not
        // after 置換 or 検索, nor when the pattern is empty or wrong (`GetData` fails).
        if finished {
            self.close(window, cx);
        }
    }

    pub(crate) fn jump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // `CDlgJump::GetData` takes the choice of the unit before it reads the number, so it is
        // kept even when the number is not valid.
        let unit = self.unit(cx);
        remember_jump_unit(cx, unit);
        let value = self.line.read(cx).value().to_string();
        let Ok(number) = value.trim().parse::<usize>() else {
            return;
        };
        self.editor
            .update(cx, |editor, cx| editor.go_to_line(number, unit, cx))
            .ok();
        self.close(window, cx);
    }

    /// `IDC_BUTTON_HELP`: the page of the function that opened the dialog (`CDlgFind`,
    /// `CDlgReplace`, `CDlgJump`).
    fn open_help(&self, cx: &mut Context<Self>) {
        crate::support::help::open_function_page(cx, self.help_function());
    }

    fn help_function(&self) -> &'static str {
        match self.mode {
            PanelMode::Find => "F_SEARCH_DIALOG",
            PanelMode::Replace => "F_REPLACE_DIALOG",
            PanelMode::Jump => "F_JUMP_DIALOG",
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor
            .update(cx, |editor, cx| editor.close_search_panel(window, cx))
            .ok();
    }

    fn option_box(
        &self,
        id: &'static str,
        label: &'static str,
        checked: bool,
        change: fn(&mut SearchOptions, bool),
        cx: &mut Context<Self>,
    ) -> Checkbox {
        Checkbox::new(id)
            .labelled(label)
            .checked(checked)
            .on_click(cx.listener(move |this, value: &bool, _, cx| {
                let value = *value;
                this.set_option(|o| change(o, value), cx);
            }))
    }

    fn render_find_options(
        &self,
        options: SearchOptions,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_1()
            // Sakura greys the box out while 正規表現 is on (`CDlgFind::OnBnClicked`).
            .child(
                self.option_box(
                    "search-whole-word",
                    WHOLE_WORD,
                    options.whole_word,
                    |o, v| o.whole_word = v,
                    cx,
                )
                .disabled(options.regex),
            )
            .child(self.option_box(
                "search-match-case",
                MATCH_CASE,
                options.match_case,
                |o, v| o.match_case = v,
                cx,
            ))
            .child(self.option_box("search-regex", REGEX, options.regex, |o, v| o.regex = v, cx))
            .child(self.option_box(
                "search-notify",
                NOTIFY_NOT_FOUND,
                options.notify_not_found,
                |o, v| o.notify_not_found = v,
                cx,
            ))
            .child(self.option_box("search-wrap", WRAP, options.wrap, |o, v| o.wrap = v, cx))
    }

    /// A radio button of the unit of the line number.
    fn unit_choice(
        &self,
        id: &'static str,
        label: &'static str,
        unit: JumpUnit,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // `BS_AUTORADIOBUTTON` (`IDD_JUMP`, `sakura_rc.rc`): a circle and the label. The radio
        // reports its role and checked state to UI Automation itself (GPUI's `checked`).
        div().debug_selector(|| id.to_owned()).child(
            Radio::new(id)
                .labelled(label)
                .checked(self.unit(cx) == unit)
                .on_click(cx.listener(move |this, _: &bool, _, cx| this.set_unit(unit, cx))),
        )
    }

    fn render_replace_targets(
        &self,
        target: ReplaceTarget,
        in_selection: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let choice = |id: &'static str,
                      label: &'static str,
                      this_target: ReplaceTarget,
                      cx: &mut Context<Self>| {
            Button::new(id)
                .labelled(label)
                .selected(target == this_target)
                .on_click(cx.listener(move |this, _, _, cx| this.set_target(this_target, cx)))
        };
        let range =
            |id: &'static str, label: &'static str, selection: bool, cx: &mut Context<Self>| {
                Button::new(id)
                    .labelled(label)
                    .selected(in_selection == selection)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_area(selection, cx)))
            };
        h_flex()
            .gap_4()
            .child(
                v_flex()
                    .gap_1()
                    .child("置換対象")
                    .child(choice(
                        "target-match",
                        TARGET_MATCH,
                        ReplaceTarget::Match,
                        cx,
                    ))
                    .child(choice(
                        "target-before",
                        TARGET_BEFORE,
                        ReplaceTarget::InsertBefore,
                        cx,
                    ))
                    .child(choice(
                        "target-after",
                        TARGET_AFTER,
                        ReplaceTarget::InsertAfter,
                        cx,
                    ))
                    .child(choice(
                        "target-line",
                        TARGET_LINE,
                        ReplaceTarget::DeleteLine,
                        cx,
                    )),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child("範囲")
                    .child(range("area-selection", AREA_SELECTION, true, cx))
                    .child(range("area-all", AREA_ALL, false, cx)),
            )
    }
}

/// The strings of the dialog go in the history: the search string, and in the replace dialog the
/// replacement too (`AddToSearchKeyArr`, `AddToReplaceKeyArr` in `CDlgFind::GetData` and
/// `CDlgReplace::GetData`).
fn remember_keys(editor: &mut EditorView, mode: PanelMode, cx: &mut Context<EditorView>) {
    // A pattern that does not compile ends the operation before anything is kept
    // (`CheckRegexpSyntax` comes first in `GetData`).
    if editor.search.pattern_error().is_some() {
        return;
    }
    // The options go to the shared data too, where the next dialog of any window reads them
    // (`m_pShareData->m_Common.m_sSearch.m_sSearchOption = m_sSearchOption`).
    let options = editor.search.options();
    let replace = (mode == PanelMode::Replace).then(|| editor.search.replace.clone());
    crate::saved_state::search::remember(cx, options, replace.as_ref());
    editor.remember_search_key(cx);
    if mode == PanelMode::Replace {
        editor.remember_replace_key(cx);
    }
}

impl Focusable for SearchPanel {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.main_input().read(cx).focus_handle(cx)
    }
}

impl Render for SearchPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let this = cx.entity();
        let (options, replace) = self
            .editor
            .read_with(cx, |editor, _| {
                (editor.search.options(), editor.search.replace.clone())
            })
            .unwrap_or_else(|_| (SearchOptions::default(), ReplaceSettings::default()));
        let title: SharedString = match self.mode {
            PanelMode::Find => "検索".into(),
            PanelMode::Replace => "置換".into(),
            PanelMode::Jump => "指定行へジャンプ".into(),
        };
        let close = div().debug_selector(|| "search-close".to_owned()).child(
            Button::new("search-close")
                .labelled(CLOSE)
                .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
        );
        let help = div().debug_selector(|| "search-help".to_owned()).child(
            Button::new("search-help")
                .labelled(HELP)
                .on_click(cx.listener(|this, _, _, cx| this.open_help(cx))),
        );
        let help_function = self.help_function();
        let body =
            match self.mode {
                PanelMode::Find => {
                    v_flex()
                        .gap_2()
                        .child(self.combo_box(Combo::Search, cx))
                        .child(self.render_find_options(options, cx))
                        .child(self.option_box(
                            "search-auto-close",
                            AUTO_CLOSE,
                            options.auto_close,
                            |o, v| o.auto_close = v,
                            cx,
                        ))
                        .child(
                            // The buttons wrap instead of sticking out of the panel when the labels are
                            // wider than the fixed panel (a large scale factor, a wide font).
                            h_flex()
                                .gap_2()
                                .flex_wrap()
                                .child(Button::new("search-prev").labelled(PREV).on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.find(Direction::Backward, window, cx);
                                    }),
                                ))
                                .child(Button::new("search-next").labelled(NEXT).on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.find(Direction::Forward, window, cx);
                                    }),
                                ))
                                .child(Button::new("search-mark").labelled(MARK).on_click(
                                    cx.listener(|this, _, window, cx| this.mark_lines(window, cx)),
                                ))
                                .child(close)
                                .child(help),
                        )
                        .into_any_element()
                }
                PanelMode::Replace => v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(BEFORE_LABEL)
                            .child(self.combo_box(Combo::Search, cx)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(AFTER_LABEL)
                            .child(self.combo_box(Combo::Replace, cx)),
                    )
                    .child(self.render_find_options(options, cx))
                    .child(self.render_replace_targets(replace.target, replace.in_selection, cx))
                    // `IDC_CHECK_bAutoCloseDlgReplace`
                    .child(self.option_box(
                        "replace-auto-close",
                        AUTO_CLOSE_REPLACE,
                        options.auto_close_replace,
                        |o, v| o.auto_close_replace = v,
                        cx,
                    ))
                    // `IDC_CHECK_CONSECUTIVEALL`, enabled only with 正規表現 (`CDlgReplace`).
                    .child(
                        Checkbox::new("replace-consecutive")
                            .labelled(CONSECUTIVE)
                            .checked(replace.consecutive_all)
                            .disabled(!options.regex)
                            .on_click(cx.listener(|this, value: &bool, _, cx| {
                                this.set_consecutive(*value, cx);
                            })),
                    )
                    .child(
                        // The buttons wrap instead of sticking out of the panel when the labels are
                        // wider than the fixed panel (a large scale factor, a wide font).
                        h_flex()
                            .gap_2()
                            .flex_wrap()
                            .child(
                                Button::new("search-prev")
                                    .labelled(PREV)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.find(Direction::Backward, window, cx);
                                    })),
                            )
                            .child(
                                Button::new("search-next")
                                    .labelled(NEXT)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.find(Direction::Forward, window, cx);
                                    })),
                            )
                            .child(Button::new("replace-one").labelled(REPLACE_ONE).on_click(
                                cx.listener(|this, _, window, cx| this.replace_one(window, cx)),
                            ))
                            .child(Button::new("replace-all").labelled(REPLACE_ALL).on_click(
                                cx.listener(|this, _, window, cx| this.replace_all(window, cx)),
                            ))
                            .child(Button::new("search-mark").labelled(MARK).on_click(
                                cx.listener(|this, _, window, cx| this.mark_lines(window, cx)),
                            ))
                            .child(close)
                            .child(help),
                    )
                    .into_any_element(),
                PanelMode::Jump => {
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_2()
                                .child(LINE_LABEL)
                                // `IDC_SPIN_LINENUM`: 1 to INT_MAX (`CDlgJump::SetData`)
                                .child(crate::spin::spin_box(
                                    "jump-line-spin",
                                    &self.line,
                                    "行番号",
                                    (1, i32::MAX),
                                    true,
                                    div()
                                        .debug_selector(|| "jump-line-box".to_owned())
                                        .w(px(LINE_BOX_WIDTH - crate::spin::ARROW_WIDTH))
                                        .child(Input::new(&self.line).aria_label("行番号")),
                                    window,
                                    cx,
                                )),
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .items_start()
                                .child(self.unit_choice(
                                    "jump-unit-rows",
                                    UNIT_ROWS,
                                    JumpUnit::Rows,
                                    cx,
                                ))
                                .child(self.unit_choice(
                                    "jump-unit-lines",
                                    UNIT_LINES,
                                    JumpUnit::Lines,
                                    cx,
                                )),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(Button::new("jump-ok").labelled("OK").on_click(
                                    cx.listener(|this, _, window, cx| this.jump(window, cx)),
                                ))
                                .child(close)
                                .child(help),
                        )
                        .into_any_element()
                }
            };
        crate::a11y::dialog(div().id("search-panel"), &title)
            .track_focus(&self.scope)
            .keep_tab_inside(&self.scope)
            // Alt+letter works from the boxes too, and does not type the letter
            .on_access_key(move |letter, window, cx| {
                this.update(cx, |panel, cx| panel.use_access_key(letter, window, cx))
            })
            // F1: the page of the dialog
            .on_help_key(help_function)
            .debug_selector(|| "search-panel".to_owned())
            .absolute()
            // mouse events stop here: a click on the panel must not move the caret or the
            // selection of the editor (a replace in the selection would lose it)
            .occlude()
            .top(px(8.))
            .right(px(24.))
            .w(px(380.))
            .p_2()
            .gap_2()
            .flex()
            .flex_col()
            .bg(rgb(0xf6f6f6))
            .text_color(rgb(0x1e1e1e))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0xb0b0b0))
            .rounded_md()
            .shadow_md()
            // The combo box keys come first: Up, Down and Enter would otherwise reach the input.
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.combo_key(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "escape" => {
                        this.close(window, cx);
                        cx.stop_propagation();
                    }
                    // F3 and Shift+F3 keep searching while the focus is in the panel
                    "f3" if this.mode != PanelMode::Jump => {
                        let direction = if event.keystroke.modifiers.shift {
                            Direction::Backward
                        } else {
                            Direction::Forward
                        };
                        this.find(direction, window, cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(div().font_weight(gpui::FontWeight::BOLD).child(title))
            .child(body)
    }
}

impl EditorView {
    /// Opens the panel in `mode` (or switches it to that mode), starts it with the selection or
    /// the word at the caret, and focuses its input.
    pub fn open_search_panel(
        &mut self,
        mode: PanelMode,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The Grep dialog is in the same place: only one of them is open.
        self.close_grep_panel(window, cx);
        // Going to a line does not touch the search.
        if mode != PanelMode::Jump {
            // The dialog starts with the options the last search of any window left
            // (`CDlgFind::SetData`: `m_sSearchOption = m_pShareData->...m_sSearchOption`).
            let saved = crate::saved_state::search::saved(cx);
            self.search.set_options(saved.options());
            if mode == PanelMode::Replace {
                self.search.replace.consecutive_all = saved.consecutive_all;
            }
            match self.dialog_seed(cx) {
                Some(seed) => self.search.set_pattern(&seed),
                // No selection and no word: the newest search string, if another window put
                // one in the history since this window last searched
                // (`CEditView::GetCurrentTextForSearchDlg`).
                None => {
                    if let Some((newest, sequence)) =
                        history::newer_search_key(cx, self.search.key_sequence())
                    {
                        self.search.set_pattern(&newest);
                        self.search.set_key_sequence(Some(sequence));
                    }
                }
            }
        }
        // Sakura starts the replace dialog with the selection if it spans lines, and the whole
        // file otherwise (`bSelected` in `Command_REPLACE_DIALOG`).
        if mode == PanelMode::Replace {
            let multi_line = self
                .cursor
                .selection()
                .is_some_and(|(a, b)| a.line != b.line);
            self.search.replace.in_selection = multi_line;
            self.search.replace.target = ReplaceTarget::Match;
            // The previous replacement, if it is news to this window (`Command_REPLACE_DIALOG`).
            if let Some((newest, sequence)) =
                history::newer_replace_key(cx, self.search.replace_sequence())
            {
                self.search.replace.text = newest;
                self.search.set_replace_sequence(Some(sequence));
            }
        }
        let caret = self.cursor.caret();
        let pattern = self.search.pattern().to_owned();
        let replacement = self.search.replace.text.clone();
        let panel = match &self.panel {
            Some(panel) => {
                panel.update(cx, |p, _| {
                    p.mode = mode;
                    // A click that was not committed is dropped when the panel is reused for the
                    // jump: it shows the remembered unit again.
                    if mode == PanelMode::Jump {
                        p.chosen = None;
                    }
                    p.dropdown = None;
                    p.pending_start = Some(caret);
                });
                panel.clone()
            }
            None => {
                let editor = cx.weak_entity();
                let panel = cx.new(|cx| SearchPanel::new(editor, mode, caret, window, cx));
                self.panel = Some(panel.clone());
                panel
            }
        };
        panel.update(cx, |p, cx| {
            p.text
                .update(cx, |state, cx| state.set_value(pattern, window, cx));
            p.replacement
                .update(cx, |state, cx| state.set_value(replacement, window, cx));
            p.line
                .update(cx, |state, cx| state.set_value("", window, cx));
            p.focus_main(window, cx);
            cx.notify();
        });
        cx.notify();
    }

    /// Shows the pattern in the panel's input, after something other than the panel changed it.
    pub(crate) fn show_pattern_in_panel(&self, window: &mut Window, cx: &mut Context<Self>) {
        let pattern = self.search.pattern().to_owned();
        let replacement = self.search.replace.text.clone();
        if let Some(panel) = &self.panel {
            panel.update(cx, |p, cx| {
                p.text
                    .update(cx, |state, cx| state.set_value(pattern, window, cx));
                p.replacement
                    .update(cx, |state, cx| state.set_value(replacement, window, cx));
            });
        }
    }

    /// Closes the panel and gives the focus back to the text.
    pub fn close_search_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.panel.take().is_some() {
            window.focus(&self.focus_handle, cx);
            cx.notify();
        }
    }

    pub fn search_panel_mode(&self, cx: &App) -> Option<PanelMode> {
        self.panel.as_ref().map(|p| p.read(cx).mode())
    }
}

#[cfg(test)]
mod a11y_tests {
    use super::*;

    #[test]
    fn the_combo_boxes_are_named_for_the_labels_of_the_dialogs() {
        assert_eq!(
            combo_names(Combo::Search, PanelMode::Find).0,
            "検索する文字列"
        );
        assert_eq!(combo_names(Combo::Search, PanelMode::Replace).0, "置換前");
        assert_eq!(combo_names(Combo::Replace, PanelMode::Replace).0, "置換後");
        // The history button says whose history it opens.
        assert_eq!(
            combo_names(Combo::Replace, PanelMode::Replace).1,
            "置換後の履歴"
        );
    }
}
