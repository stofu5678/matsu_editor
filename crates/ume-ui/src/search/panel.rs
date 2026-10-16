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

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{Disableable as _, Selectable as _, h_flex, v_flex};
use ume_core::buffer::Pos;
use ume_core::search::{Direction, ReplaceTarget};

use super::history;
use super::ops::SearchOutcome;
use super::state::{NO_PATTERN_MESSAGE, ReplaceSettings, SearchOptions};
use crate::view::EditorView;

/// Which dialog the panel is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelMode {
    Find,
    Replace,
    Jump,
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
    dropdown: Option<DropDown>,
    /// Scrolls the open list to the highlighted entry.
    list_scroll: ScrollHandle,
    mode: PanelMode,
    text: Entity<InputState>,
    replacement: Entity<InputState>,
    line: Entity<InputState>,
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
            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder))
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
                InputEvent::PressEnter { .. } => this.replace_one(window, cx),
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
        Self {
            editor,
            dropdown: None,
            list_scroll: ScrollHandle::new(),
            mode,
            text,
            replacement,
            line,
            pending_start: Some(pending_start),
            _subscriptions: subscriptions,
        }
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
        let input = self.main_input().clone();
        input.update(cx, |state, cx| state.focus(window, cx));
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
            "enter" if plain && open.highlight < keys.len() => {
                self.pick(combo, keys[open.highlight].clone(), window, cx);
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
        let (id, toggle_id) = match combo {
            Combo::Search => ("search-history", "search-history-toggle"),
            Combo::Replace => ("replace-history", "replace-history-toggle"),
        };
        let mut column =
            v_flex().gap_1().flex_1().child(
                h_flex()
                    .gap_1()
                    .child(div().flex_1().child(Input::new(self.combo_input(combo))))
                    .child(Button::new(toggle_id).label("▼").on_click(
                        cx.listener(move |this, _, _, cx| this.toggle_dropdown(combo, cx)),
                    )),
            );
        if let Some(open) = self.dropdown.filter(|d| d.combo == combo) {
            let entries = Self::combo_keys(combo, cx)
                .into_iter()
                .enumerate()
                .map(|(i, text)| {
                    let picked = text.clone();
                    div()
                        .id((id, i))
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
                div()
                    .id(id)
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

    fn replace_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
                editor.replace_all(window, cx);
            }
        });
        self.register_start(cx);
    }

    fn jump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.line.read(cx).value().to_string();
        let Ok(number) = value.trim().parse::<usize>() else {
            return;
        };
        self.editor
            .update(cx, |editor, cx| editor.go_to_line(number, cx))
            .ok();
        self.close(window, cx);
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
            .label(label)
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
                    "単語単位で探す(W)",
                    options.whole_word,
                    |o, v| o.whole_word = v,
                    cx,
                )
                .disabled(options.regex),
            )
            .child(self.option_box(
                "search-match-case",
                "英大文字と小文字を区別する(C)",
                options.match_case,
                |o, v| o.match_case = v,
                cx,
            ))
            .child(self.option_box(
                "search-regex",
                "正規表現(E)",
                options.regex,
                |o, v| o.regex = v,
                cx,
            ))
            .child(self.option_box(
                "search-notify",
                "見つからないときにメッセージを表示(M)",
                options.notify_not_found,
                |o, v| o.notify_not_found = v,
                cx,
            ))
            .child(self.option_box(
                "search-wrap",
                "先頭（末尾）から再検索する(Z)",
                options.wrap,
                |o, v| o.wrap = v,
                cx,
            ))
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
                .label(label)
                .selected(target == this_target)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.editor
                        .update(cx, |editor, cx| {
                            editor.search.replace.target = this_target;
                            cx.notify();
                        })
                        .ok();
                    cx.notify();
                }))
        };
        let range =
            |id: &'static str, label: &'static str, selection: bool, cx: &mut Context<Self>| {
                Button::new(id)
                    .label(label)
                    .selected(in_selection == selection)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.editor
                            .update(cx, |editor, cx| {
                                editor.search.replace.in_selection = selection;
                                cx.notify();
                            })
                            .ok();
                        cx.notify();
                    }))
            };
        h_flex()
            .gap_4()
            .child(
                v_flex()
                    .gap_1()
                    .child("置換対象")
                    .child(choice(
                        "target-match",
                        "選択文字(0)",
                        ReplaceTarget::Match,
                        cx,
                    ))
                    .child(choice(
                        "target-before",
                        "選択始点(1)挿入",
                        ReplaceTarget::InsertBefore,
                        cx,
                    ))
                    .child(choice(
                        "target-after",
                        "選択終点(2)追加",
                        ReplaceTarget::InsertAfter,
                        cx,
                    ))
                    .child(choice(
                        "target-line",
                        "行削除(3)",
                        ReplaceTarget::DeleteLine,
                        cx,
                    )),
            )
            .child(
                v_flex()
                    .gap_1()
                    .child("範囲")
                    .child(range("area-selection", "選択範囲(S)", true, cx))
                    .child(range("area-all", "ファイル全体(O)", false, cx)),
            )
    }
}

/// The strings of the dialog go in the history: the search string, and in the replace dialog the
/// replacement too (`AddToSearchKeyArr`, `AddToReplaceKeyArr` in `CDlgFind::GetData` and
/// `CDlgReplace::GetData`).
fn remember_keys(editor: &mut EditorView, mode: PanelMode, cx: &mut Context<EditorView>) {
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let close = Button::new("search-close")
            .label("閉じる(X)")
            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)));
        let body =
            match self.mode {
                PanelMode::Find => v_flex()
                    .gap_2()
                    .child(self.combo_box(Combo::Search, cx))
                    .child(self.render_find_options(options, cx))
                    .child(self.option_box(
                        "search-auto-close",
                        "検索ダイアログを自動的に閉じる(L)",
                        options.auto_close,
                        |o, v| o.auto_close = v,
                        cx,
                    ))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("search-prev").label("上検索(U)").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.find(Direction::Backward, window, cx);
                                }),
                            ))
                            .child(Button::new("search-next").label("下検索(D)").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.find(Direction::Forward, window, cx);
                                }),
                            ))
                            .child(
                                Button::new("search-mark")
                                    .label("該当行マーク(M)")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.mark_lines(window, cx)
                                    })),
                            )
                            .child(close),
                    )
                    .into_any_element(),
                PanelMode::Replace => v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child("置換前(N)")
                            .child(self.combo_box(Combo::Search, cx)),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child("置換後(P)")
                            .child(self.combo_box(Combo::Replace, cx)),
                    )
                    .child(self.render_find_options(options, cx))
                    .child(self.render_replace_targets(replace.target, replace.in_selection, cx))
                    // `IDC_CHECK_CONSECUTIVEALL`, enabled only with 正規表現 (`CDlgReplace`).
                    .child(
                        Checkbox::new("replace-consecutive")
                            .label("「すべて置換」は置換の繰返し(L)")
                            .checked(replace.consecutive_all)
                            .disabled(!options.regex)
                            .on_click(cx.listener(|this, value: &bool, _, cx| {
                                let value = *value;
                                this.editor
                                    .update(cx, |editor, cx| {
                                        editor.search.replace.consecutive_all = value;
                                        cx.notify();
                                    })
                                    .ok();
                                cx.notify();
                            })),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Button::new("search-prev").label("上検索(U)").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.find(Direction::Backward, window, cx);
                                }),
                            ))
                            .child(Button::new("search-next").label("下検索(D)").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.find(Direction::Forward, window, cx);
                                }),
                            ))
                            .child(Button::new("replace-one").label("置換(R)").on_click(
                                cx.listener(|this, _, window, cx| this.replace_one(window, cx)),
                            ))
                            .child(Button::new("replace-all").label("すべて置換(A)").on_click(
                                cx.listener(|this, _, window, cx| this.replace_all(window, cx)),
                            ))
                            .child(
                                Button::new("search-mark")
                                    .label("該当行マーク(M)")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.mark_lines(window, cx)
                                    })),
                            )
                            .child(close),
                    )
                    .into_any_element(),
                PanelMode::Jump => {
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_2()
                                .child("行番号(N)")
                                .child(Input::new(&self.line)),
                        )
                        .child(
                            h_flex()
                                .gap_2()
                                .child(Button::new("jump-ok").label("OK").on_click(
                                    cx.listener(|this, _, window, cx| this.jump(window, cx)),
                                ))
                                .child(close),
                        )
                        .into_any_element()
                }
            };
        div()
            .id("search-panel")
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
            match self.search_seed(cx) {
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
