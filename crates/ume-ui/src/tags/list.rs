//! The tag jump list (ダイレクトタグジャンプ一覧): the window that lists the hits of a tags file and
//! asks which one to jump to.
//!
//! Sakura: `CDlgTagJumpList` (`sakura_core/dlg/CDlgTagJumpList.cpp`; the layout is `IDD_TAGJUMPLIST`
//! in `sakura_core/sakura_rc.rc`). It has two forms:
//!
//! * the direct tag jump (`F_DIRECT_TAGJUMP`, and `F_TAGJUMP` when the line names nothing) opens it
//!   when the word under the caret has several hits; the keyword is fixed and there are only the
//!   list and the buttons (the page buttons only when the hits do not fit one page);
//! * the keyword form (`F_TAGJUMP_KEYWORD`) has the keyword and two check boxes (大文字小文字の同一視,
//!   文字列の途中にマッチ); the list follows what is typed, 0.7 second after the last key (0.05
//!   second after the dialog opens and after a check box is clicked).
//!
//! The columns are キーワード, 階層, 行番号, 種類, ファイル名 and 備考. The first choice is the hit
//! that is the likeliest to be wanted ([`best_item`]). 50 hits are listed at a time; 前へ and 次へ
//! turn the pages. Enter, a double click or ジャンプ chooses, Esc or キャンセル does not.
//!
//! Here the dialog is a modal panel over the view (as the command list is), and the search runs on
//! a background thread so that a large tags file does not stop the window.
//!
//! Not done: the history drop-down of the keyword (the newest keyword is filled in), the size and
//! position that the dialog remembers, the help button.

use std::path::PathBuf;
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AppContext as _, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, px, relative, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{Disableable as _, h_flex, v_flex};
use ume_core::tag_file::{PAGE_SIZE, TagItem, TagPage, TagQuery, best_item, find_tags, kind_name};

use super::{TagJumpPrefs, prefs, remember_prefs};
use crate::view::EditorView;

/// `TAGJUMP_TIMER_DELAY`: the wait after a key before the list follows.
pub(crate) const TYPING_DELAY: Duration = Duration::from_millis(700);
/// `TAGJUMP_TIMER_DELAY_SHORT`.
pub(crate) const SHORT_DELAY: Duration = Duration::from_millis(50);

/// `STR_DLGTAGJMP1`, `STR_DLGTAGJMP2`: the row that says nothing was found.
const NOTICE: &str = "(通知)";
const NOT_FOUND: &str = "(候補は見つかりませんでした)";

/// The two forms of the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    /// `CDlgTagJumpList(true)`.
    Direct,
    /// `CDlgTagJumpList(false)`.
    Keyword,
}

/// What the dialog gives back: the chosen hit, or `None` for no choice.
pub(crate) type Choice = Option<TagItem>;

pub(crate) struct TagList {
    view: WeakEntity<EditorView>,
    form: Form,
    /// The file that the search starts from (`SetFileName`).
    document: PathBuf,
    keyword: Entity<InputState>,
    ignore_case: bool,
    partial: bool,
    page: TagPage,
    top: usize,
    selected: Option<usize>,
    /// Whether a search has been made (`UpdateData(bInit)`): before it an empty list says nothing.
    searched: bool,
    busy: bool,
    /// Tells a search that a newer one has begun.
    generation: u64,
    direct_mode: u8,
    keyword_mode: u8,
    result: Option<async_channel::Sender<Choice>>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl TagList {
    /// A dialog of `form`. `first` is the page that the caller has already searched (the direct
    /// form), else the dialog searches when it opens.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        view: WeakEntity<EditorView>,
        form: Form,
        document: PathBuf,
        keyword: &str,
        first: Option<TagPage>,
        result: async_channel::Sender<Choice>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = prefs(cx);
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("キーワード"));
        input.update(cx, |state, cx| {
            state.set_value(keyword.to_owned(), window, cx)
        });
        let mut subscriptions = Vec::new();
        subscriptions.push(cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.search(true, TYPING_DELAY, cx),
                InputEvent::PressEnter { .. } => this.choose(window, cx),
                _ => {}
            },
        ));
        let searched = first.is_some();
        let page = first.unwrap_or_default();
        let selected = (!page.items.is_empty()).then(|| best_item(&page.items, &document));
        let mut list = Self {
            view,
            form,
            document,
            keyword: input,
            // The direct form is always exact and case-sensitive (`SetData`).
            ignore_case: form == Form::Keyword && settings.ignore_case,
            partial: form == Form::Keyword && settings.partial,
            page,
            top: 0,
            selected,
            searched,
            busy: false,
            generation: 0,
            direct_mode: settings.direct_mode,
            keyword_mode: settings.keyword_mode,
            result: Some(result),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        if form == Form::Keyword {
            // `StartTimer(TAGJUMP_TIMER_DELAY_SHORT)` at the end of `SetData`
            list.search(true, SHORT_DELAY, cx);
        }
        list
    }

    /// Moves the focus where Sakura puts it: the keyword, or the list.
    pub(crate) fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        match self.form {
            Form::Keyword => self.keyword.update(cx, |state, cx| state.focus(window, cx)),
            Form::Direct => window.focus(&self.focus, cx),
        }
    }

    /// `FindNext`: searches again after `delay`. `new_find` starts from the first page; else the
    /// page is the one at [`top`](Self::top). A newer call makes the older ones give up.
    pub(crate) fn search(&mut self, new_find: bool, delay: Duration, cx: &mut Context<Self>) {
        if new_find {
            self.top = 0;
        }
        self.generation += 1;
        self.busy = true;
        let generation = self.generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            if !delay.is_zero() {
                cx.background_executor().timer(delay).await;
            }
            let Ok(Some(job)) = this.update(cx, |this, cx| {
                let current = this.generation == generation;
                current.then(|| this.job(cx))
            }) else {
                return;
            };
            let page = cx
                .background_executor()
                .spawn(async move { job.run() })
                .await;
            this.update(cx, |this, cx| {
                if this.generation == generation {
                    this.show(page, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// What `find_key` hands to `find_key_core`.
    fn job(&self, cx: &gpui::App) -> Job {
        Job {
            document: self.document.clone(),
            keyword: self.keyword.read(cx).value().to_string(),
            form: self.form,
            partial: self.partial,
            ignore_case: self.ignore_case,
            top: self.top,
            mode: match self.form {
                Form::Direct => self.direct_mode,
                Form::Keyword => self.keyword_mode,
            },
        }
    }

    /// `UpdateData(false)`: shows a page.
    fn show(&mut self, page: TagPage, cx: &mut Context<Self>) {
        self.selected = (!page.items.is_empty()).then(|| best_item(&page.items, &self.document));
        self.page = page;
        self.searched = true;
        self.busy = false;
        if let Some(selected) = self.selected {
            self.scroll.scroll_to_item(selected);
        }
        cx.notify();
    }

    /// The hits that are listed.
    #[cfg(test)]
    pub(crate) fn items(&self) -> &[TagItem] {
        &self.page.items
    }

    #[cfg(test)]
    pub(crate) fn selected(&self) -> Option<usize> {
        self.selected
    }

    #[cfg(test)]
    pub(crate) fn has_next(&self) -> bool {
        self.page.has_next
    }

    #[cfg(test)]
    pub(crate) fn top(&self) -> usize {
        self.top
    }

    pub(crate) fn set_options(&mut self, ignore_case: bool, partial: bool, cx: &mut Context<Self>) {
        self.ignore_case = ignore_case;
        self.partial = partial;
        self.search(true, SHORT_DELAY, cx);
    }

    /// 次へ.
    pub(crate) fn next_page(&mut self, cx: &mut Context<Self>) {
        if self.busy || !self.page.has_next {
            return;
        }
        self.top += PAGE_SIZE;
        self.search(false, Duration::ZERO, cx);
    }

    /// 前へ.
    pub(crate) fn previous_page(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.top == 0 {
            return;
        }
        self.top = self.top.saturating_sub(PAGE_SIZE);
        self.search(false, Duration::ZERO, cx);
    }

    /// Moves the choice by `delta` rows, within the page.
    pub(crate) fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.page.items.len();
        if count == 0 {
            return;
        }
        let at = self.selected.unwrap_or(0).saturating_add_signed(delta);
        let at = at.min(count - 1);
        self.selected = Some(at);
        self.scroll.scroll_to_item(at);
        cx.notify();
    }

    /// ジャンプ: `GetData`. The check boxes are kept for the next time whatever is chosen
    /// (「候補が空でもジャンプで閉じたときは、オプションを保存する」), the keyword only when a hit is
    /// chosen. Nothing chosen is the same as キャンセル.
    pub(crate) fn choose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The list still shows the answer to an older keyword: wait for the new one.
        if self.busy {
            return;
        }
        let chosen = self.selected.and_then(|i| self.page.items.get(i)).cloned();
        if self.form == Form::Keyword {
            let keyword = chosen
                .is_some()
                .then(|| self.keyword.read(cx).value().to_string());
            let (ignore_case, partial) = (self.ignore_case, self.partial);
            remember_prefs(cx, move |p: &mut TagJumpPrefs| {
                p.ignore_case = ignore_case;
                p.partial = partial;
                if let Some(keyword) = keyword {
                    p.remember_keyword(&keyword);
                }
            });
        }
        self.finish(chosen, window, cx);
    }

    /// キャンセル.
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish(None, window, cx);
    }

    fn finish(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(result) = self.result.take() {
            let _ = result.try_send(choice);
        }
        self.generation += 1;
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.support.tag_list = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let plain = {
            let m = &event.keystroke.modifiers;
            !m.alt && !m.control && !m.platform && !m.shift
        };
        match key {
            "escape" => self.cancel(window, cx),
            "up" if plain => self.move_selection(-1, cx),
            "down" if plain => self.move_selection(1, cx),
            "pageup" if plain => self.move_selection(-10, cx),
            "pagedown" if plain => self.move_selection(10, cx),
            // in the keyword form Enter reaches the input, which says so
            "enter" if self.form == Form::Direct => self.choose(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

/// The search of one page, with everything it needs, so that it can run on another thread.
struct Job {
    document: PathBuf,
    keyword: String,
    form: Form,
    partial: bool,
    ignore_case: bool,
    top: usize,
    mode: u8,
}

impl Job {
    /// `find_key`.
    fn run(self) -> TagPage {
        let query = match self.form {
            // `FindDirectTagJump`
            Form::Direct => TagQuery {
                top: self.top,
                ..TagQuery::direct(&self.keyword, self.mode)
            },
            Form::Keyword => TagQuery {
                keyword: &self.keyword,
                partial: self.partial,
                exact: false,
                ignore_case: self.ignore_case,
                ignore_case_by_tags: false,
                next_mode: self.mode,
                top: self.top,
            },
        };
        find_tags(&self.document, &query)
    }
}

impl Focusable for TagList {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

const TEXT: u32 = 0x222222;
const BACKGROUND: u32 = 0xf7f7f7;
const CHOSEN: u32 = 0x2f6fd0;

/// The share of the width of each column (`col.cx = nWidth * N / 100`).
const COLUMNS: [(&str, f32); 6] = [
    ("キーワード", 0.20),
    ("階層", 0.07),
    ("行番号", 0.08),
    ("種類", 0.09),
    ("ファイル名", 0.35),
    ("備考", 0.21),
];

impl Render for TagList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cell = |index: usize, text: String| {
            let (_, share) = COLUMNS[index];
            div()
                .w(relative(share))
                .px_1()
                .overflow_hidden()
                .whitespace_nowrap()
                .when(index == 1, |d| d.text_center())
                .when(index == 2, |d| d.flex().justify_end())
                .child(text)
        };
        let header = h_flex().bg(rgb(0xe4e4e4)).children(
            COLUMNS
                .iter()
                .enumerate()
                .map(|(i, (name, _))| cell(i, (*name).to_owned())),
        );
        let mut rows: Vec<gpui::AnyElement> = Vec::new();
        for (i, item) in self.page.items.iter().enumerate() {
            let depth = if item.via_next {
                format!("({})", item.depth)
            } else {
                item.depth.to_string()
            };
            let row = h_flex()
                .id(("tag-row", i))
                .children([
                    cell(0, item.keyword.clone()),
                    cell(1, depth),
                    cell(2, item.line.to_string()),
                    cell(3, kind_name(item.kind, &item.file).to_owned()),
                    cell(4, item.file.clone()),
                    cell(5, item.note.clone()),
                ])
                .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                    this.selected = Some(i);
                    if event.click_count() >= 2 {
                        this.choose(window, cx);
                    }
                    cx.notify();
                }));
            rows.push(if self.selected == Some(i) {
                row.bg(rgb(CHOSEN))
                    .text_color(rgb(0xffffff))
                    .into_any_element()
            } else {
                row.into_any_element()
            });
        }
        if self.searched && !self.busy && self.page.items.is_empty() {
            rows.push(
                h_flex()
                    .id("tag-none")
                    .children([
                        cell(0, NOTICE.to_owned()),
                        cell(1, String::new()),
                        cell(2, String::new()),
                        cell(3, String::new()),
                        cell(4, NOT_FOUND.to_owned()),
                        cell(5, String::new()),
                    ])
                    .into_any_element(),
            );
        }
        let base = self
            .document
            .parent()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let direct = self.form == Form::Direct;
        let hide_pages = direct && self.top == 0 && !self.page.has_next;
        let keyword_label = if self.busy {
            "キーワード 検索中..."
        } else {
            "キーワード(K)"
        };
        let ignore_case = self.ignore_case;
        let partial = self.partial;
        let title = if direct {
            "ダイレクトタグジャンプ一覧"
        } else {
            "キーワードを指定してタグジャンプ"
        };

        let panel = v_flex()
            .id("tag-list")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .track_focus(&self.focus)
            .key_context("TagList")
            .capture_key_down(cx.listener(Self::on_key_down))
            .w(px(760.))
            .max_w_full()
            .p_2()
            .gap_2()
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0x888888))
            .rounded_md()
            .shadow_md()
            .child(div().font_weight(gpui::FontWeight::BOLD).child(title))
            .child(
                v_flex()
                    .border_1()
                    .border_color(rgb(0xb0b0b0))
                    .bg(rgb(0xffffff))
                    .child(header)
                    .child(
                        v_flex()
                            .id("tag-rows")
                            .h(px(260.))
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .children(rows),
                    ),
            )
            .child(div().child(base))
            .child(
                h_flex()
                    .gap_2()
                    .items_end()
                    .when(!direct, |d| {
                        d.child(
                            v_flex()
                                .gap_1()
                                .flex_1()
                                .child(keyword_label)
                                .child(Input::new(&self.keyword))
                                .child(
                                    h_flex()
                                        .gap_3()
                                        .child(
                                            Checkbox::new("tag-icase")
                                                .label("大文字小文字の同一視(F)")
                                                .checked(ignore_case)
                                                .on_click(cx.listener(
                                                    move |this, value: &bool, _, cx| {
                                                        this.set_options(*value, partial, cx)
                                                    },
                                                )),
                                        )
                                        .child(
                                            Checkbox::new("tag-anywhere")
                                                .label("文字列の途中にマッチ(A)")
                                                .checked(partial)
                                                .on_click(cx.listener(
                                                    move |this, value: &bool, _, cx| {
                                                        this.set_options(ignore_case, *value, cx)
                                                    },
                                                )),
                                        ),
                                ),
                        )
                    })
                    .when(direct, |d| d.child(div().flex_1()))
                    .child(
                        v_flex()
                            .gap_1()
                            .when(!hide_pages, |d| {
                                d.child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            Button::new("tag-prev")
                                                .label("前へ(P)")
                                                .disabled(self.busy || self.top == 0)
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.previous_page(cx)
                                                })),
                                        )
                                        .child(
                                            Button::new("tag-next")
                                                .label("次へ(N)")
                                                .disabled(self.busy || !self.page.has_next)
                                                .on_click(
                                                    cx.listener(|this, _, _, cx| {
                                                        this.next_page(cx)
                                                    }),
                                                ),
                                        ),
                                )
                            })
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(Button::new("tag-jump").label("ジャンプ(J)").on_click(
                                        cx.listener(|this, _, window, cx| this.choose(window, cx)),
                                    ))
                                    .child(
                                        Button::new("tag-cancel").label("キャンセル(X)").on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.cancel(window, cx)
                                            }),
                                        ),
                                    ),
                            ),
                    ),
            );
        div()
            .id("tag-list-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| this.cancel(window, cx)),
            )
            .child(panel)
    }
}

impl EditorView {
    /// Opens the tag jump list and focuses it. The answer comes through `result`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn open_tag_list(
        &mut self,
        form: Form,
        document: PathBuf,
        keyword: &str,
        first: Option<TagPage>,
        result: async_channel::Sender<Choice>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().downgrade();
        let keyword = keyword.to_owned();
        let list =
            cx.new(|cx| TagList::new(view, form, document, &keyword, first, result, window, cx));
        list.update(cx, |list, cx| list.focus_first(window, cx));
        self.support.tag_list = Some(list);
        cx.notify();
    }
}
