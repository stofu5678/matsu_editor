//! The search box of the toolbar (`F_SEARCH_BOX`).
//!
//! Sakura: `CMainToolBar::CreateToolBar` makes a combo box with a drop-down list in the place of
//! the button (`sakura_core/window/CMainToolBar.cpp`); the list is the shared search history
//! (`AcceptSharedSearchKey`). Keys of the edit field (`ProcSearchBox`): Enter puts the text in the
//! history, makes it the search key of the active view, remembers the caret as the place the
//! search started from and runs `F_SEARCH_NEXT`, which uses the options the view has (the ones
//! of the find panel); Tab moves the focus to the window. When the focus leaves the box, its
//! text becomes the search key too (`CBN_KILLFOCUS` in `CEditWnd::OnCommand`). `F_SEARCH_BOX`
//! (`Command_SEARCH_BOX`) moves the focus into the box.
//!
//! Esc also gives the focus back to the text, which Sakura's box does not do (a combo box does not
//! use Esc). The text follows the search key of the other windows
//! (`AcceptSharedSearchKey`: `m_bInheritKeyOtherView`, which is on in Sakura and cannot be turned
//! off here either).

use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, ScrollHandle, SharedString, StatefulInteractiveElement,
    Styled, Subscription, WeakEntity, Window, deferred, div, px, rgb,
};
use gpui_component::Sizable as _;
use gpui_component::input::{Escape, IndentInline, Input, InputEvent, InputState};

use crate::app::Workspace;
use crate::search::history;
use crate::view::{EditorView, OpenRecentFile, OpenRecentFolder, RunFunction};

/// `_MAX_PATH`: a longer string is not put in the history (`strText.size() < _MAX_PATH`).
const MAX_PATH: usize = 260;
/// The width of the box (`DpiScaleX( 160 )`).
pub(crate) const WIDTH: f32 = 160.0;
/// The height of a toolbar line.
const HEIGHT: f32 = 22.0;
/// `F_SEARCH_NEXT`.
const SEARCH_NEXT: u16 = 30902;
/// `F_SEARCH_BOX`.
const SEARCH_BOX: u16 = 30908;

pub(crate) struct SearchBox {
    workspace: WeakEntity<Workspace>,
    input: Entity<InputState>,
    /// The entry of the open list that Enter takes.
    dropdown: Option<usize>,
    list_scroll: ScrollHandle,
    /// The pane that was the active one when the field got the focus: the text is its search key
    /// when the focus leaves, even if the focus leaves because another tab became the active one.
    /// `None` when the focus is not in the field, or the text was taken already.
    owner: Option<WeakEntity<EditorView>>,
    _subscriptions: Vec<Subscription>,
}

/// What the box shows after `AcceptSharedSearchKey`: the newest string of the history if another
/// window searched since this view did (or the view has no search key), else the view's key.
/// `None` when the history is empty (the box keeps its text) or already shows it.
fn accepted_text(
    history_newest: Option<&str>,
    view_pattern: &str,
    news: bool,
    shown: &str,
) -> Option<String> {
    let newest = history_newest?;
    let text = if news || view_pattern.is_empty() {
        newest
    } else {
        view_pattern
    };
    (text != shown).then(|| text.to_owned())
}

impl SearchBox {
    pub(crate) fn new(
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let input = cx.new(|cx| InputState::new(window, cx));
            let subscriptions = vec![
                cx.subscribe_in(
                    &input,
                    window,
                    |this: &mut Self, _, event: &InputEvent, window, cx| match event {
                        InputEvent::Focus => this.begin_session(cx),
                        InputEvent::PressEnter { .. } => this.press_enter(window, cx),
                        InputEvent::Blur => {
                            this.dropdown = None;
                            this.take_as_key(cx);
                            cx.notify();
                        }
                        _ => {}
                    },
                ),
                // The list is read when it is drawn; the text follows the news.
                cx.observe_global_in::<history::SearchHistory>(window, |this, window, cx| {
                    this.accept_shared_key(window, cx);
                }),
                cx.observe_window_activation(window, |this, window, cx| {
                    if window.is_window_active() {
                        this.accept_shared_key(window, cx);
                    }
                }),
            ];
            Self {
                workspace,
                input,
                dropdown: None,
                list_scroll: ScrollHandle::new(),
                owner: None,
                _subscriptions: subscriptions,
            }
        })
    }

    /// The text in the box.
    pub(crate) fn text(&self, cx: &App) -> String {
        self.input.read(cx).value().to_string()
    }

    /// Whether the edit field has the focus.
    pub(crate) fn is_focused(&self, window: &Window, cx: &App) -> bool {
        self.input.read(cx).focus_handle(cx).is_focused(window)
    }

    /// `CMainToolBar::SetFocusSearchBox`.
    ///
    /// `editor` is the pane of the window that asks (the window is being updated: it cannot be
    /// read from here).
    pub(crate) fn focus_with(
        &mut self,
        editor: &Entity<EditorView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.owner = Some(editor.downgrade());
        self.focus_input(window, cx);
    }

    fn focus_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input.update(cx, |state, cx| state.focus(window, cx));
    }

    /// Notes the pane the text will be the search key of.
    fn begin_session(&mut self, cx: &App) {
        self.owner = self.editor(cx).map(|editor| editor.downgrade());
    }

    /// The pane the user works in.
    fn editor(&self, cx: &App) -> Option<Entity<EditorView>> {
        self.workspace
            .upgrade()
            .map(|workspace| workspace.read(cx).editor().clone())
    }

    /// Gives the focus back to the text of the active pane.
    fn focus_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(editor) = self.editor(cx) {
            self.leave(&editor, window, cx);
        }
    }

    /// Gives the focus to `editor` and takes the text of the box as the search key (`CBN_KILLFOCUS`;
    /// the blur event of the field comes later and finds nothing to do). `editor` is the pane of
    /// the window that asks, which may be being updated.
    pub(crate) fn leave(
        &mut self,
        editor: &Entity<EditorView>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dropdown = None;
        self.take_as_key(cx);
        let handle = editor.read(cx).focus_handle.clone();
        window.focus(&handle, cx);
        cx.notify();
    }

    /// Makes `text` the search key of `editor` (`AddToSearchKeyArr`, `m_strCurSearchKey`).
    fn make_key(editor: &mut EditorView, text: &str, cx: &mut Context<EditorView>) {
        if text.encode_utf16().count() < MAX_PATH {
            let sequence = history::remember_search(cx, text);
            editor.search.set_key_sequence(Some(sequence));
        } else if let Some((_, sequence)) =
            history::newer_search_key(cx, editor.search.key_sequence())
        {
            // Not for the history, but this key is the newest one for the view: the box does not
            // swap it for an older string.
            editor.search.set_key_sequence(Some(sequence));
        }
        editor.search.set_pattern(text);
        cx.notify();
    }

    /// `CBN_KILLFOCUS`: what is in the box when the focus leaves is the search key.
    pub(in crate::bars) fn take_as_key(&mut self, cx: &mut Context<Self>) {
        let text = self.text(cx);
        // No session: the focus left already and the text was taken then.
        let Some(editor) = self.owner.take().and_then(|owner| owner.upgrade()) else {
            return;
        };
        if text.is_empty() {
            return;
        }
        // Enter has done it already.
        let done = history::search_keys(cx).first() == Some(&text)
            && editor.read(cx).search.pattern() == text;
        if !done {
            editor.update(cx, |editor, cx| Self::make_key(editor, &text, cx));
        }
    }

    /// Moves the focus to the view and then runs `run` on it, once this event is handled.
    fn run_in_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        run: impl FnOnce(&mut EditorView, &mut Window, &mut Context<EditorView>) + 'static,
    ) {
        let Some(editor) = self.editor(cx) else {
            return;
        };
        self.focus_editor(window, cx);
        window.defer(cx, move |window, app| {
            editor.update(app, |editor, cx| run(editor, window, cx));
        });
    }

    /// Enter in the edit field: with the list open it takes the highlighted string (as a combo box
    /// does), else it searches.
    fn press_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let keys = history::search_keys(cx);
        match self.dropdown.and_then(|i| keys.get(i)) {
            Some(text) => self.pick(text.clone(), window, cx),
            None => self.search(window, cx),
        }
    }

    /// `ProcSearchBox` (Enter): the text is the search key, and the search goes forward.
    fn search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.text(cx);
        let Some(editor) = self.editor(cx) else {
            return;
        };
        if text.is_empty() {
            return;
        }
        self.dropdown = None;
        // The text is taken here; the blur that follows has nothing to do.
        self.owner = None;
        editor.update(cx, |editor, cx| {
            Self::make_key(editor, &text, cx);
            window.focus(&editor.focus_handle, cx);
            // "検索開始時のカーソル位置登録条件を変更"
            let caret = editor.cursor.caret();
            editor.search.set_start_pos(Some(caret));
        });
        self.accept_shared_key(window, cx);
        editor.update(cx, |editor, cx| {
            editor.run_function(SEARCH_NEXT, &[], window, cx);
        });
        cx.notify();
    }

    /// `AcceptSharedSearchKey`: the text of the box follows the search key. (The list is the
    /// history, read when it is drawn.) A box that has the focus keeps what is being typed.
    pub(crate) fn accept_shared_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_focused(window, cx) {
            cx.notify();
            return;
        }
        let Some(editor) = self.editor(cx) else {
            return;
        };
        let (pattern, sequence) = {
            let view = editor.read(cx);
            (view.search.pattern().to_owned(), view.search.key_sequence())
        };
        let news = history::newer_search_key(cx, sequence);
        let newest = history::search_keys(cx).into_iter().next();
        let shown = self.text(cx);
        if let Some(text) = accepted_text(newest.as_deref(), &pattern, news.is_some(), &shown) {
            self.input
                .update(cx, |state, cx| state.set_value(text, window, cx));
        }
        cx.notify();
    }

    /// Opens the list, or closes it. With no history there is nothing to open.
    fn toggle_dropdown(&mut self, cx: &mut Context<Self>) {
        if self.dropdown.is_some() {
            self.dropdown = None;
        } else {
            let keys = history::search_keys(cx);
            let current = self.text(cx);
            self.dropdown =
                (!keys.is_empty()).then(|| keys.iter().position(|k| *k == current).unwrap_or(0));
            if let Some(highlight) = self.dropdown {
                self.list_scroll.scroll_to_item(highlight);
            }
        }
        cx.notify();
    }

    /// Puts a string of the history in the edit field and closes the list.
    fn pick(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        self.dropdown = None;
        self.input.update(cx, |state, cx| {
            state.set_value(text, window, cx);
            state.focus(window, cx);
        });
        cx.notify();
    }

    /// Up and Down with the list closed go through the strings; from a string that is not in the
    /// list both take the first.
    fn step(&mut self, down: bool, window: &mut Window, cx: &mut Context<Self>) {
        let keys = history::search_keys(cx);
        if keys.is_empty() {
            return;
        }
        let current = self.text(cx);
        let next = match (keys.iter().position(|k| *k == current), down) {
            (Some(i), true) => (i + 1).min(keys.len() - 1),
            (Some(i), false) => i.saturating_sub(1),
            (None, _) => 0,
        };
        self.pick(keys[next].clone(), window, cx);
    }

    /// Up and Down go through the history or the open list; F4 and Alt+Down open and close the
    /// list. (Tab, Escape and Enter are actions of the edit field, see [`Render`].) Returns whether
    /// the key was used.
    fn on_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = event.keystroke.key.as_str();
        let m = &event.keystroke.modifiers;
        let plain = !m.alt && !m.control && !m.platform && !m.shift;
        let keys = history::search_keys(cx);
        match key {
            "f4" if plain => self.toggle_dropdown(cx),
            "down" if m.alt => self.toggle_dropdown(cx),
            "down" | "up" if plain && self.dropdown.is_none() => {
                self.step(key == "down", window, cx);
            }
            "down" | "up" if plain && !keys.is_empty() => {
                let open = self.dropdown.unwrap_or(0);
                let highlight = if key == "down" {
                    (open + 1).min(keys.len() - 1)
                } else {
                    open.saturating_sub(1)
                };
                self.dropdown = Some(highlight);
                self.list_scroll.scroll_to_item(highlight);
                cx.notify();
            }
            _ => return false,
        }
        true
    }

    /// The highlighted entry of the open list, for the tests.
    #[cfg(test)]
    pub(crate) fn open_entry(&self) -> Option<usize> {
        self.dropdown
    }
}

impl Focusable for SearchBox {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }
}

impl Render for SearchBox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The list belongs to the edit field: it closes when the focus is elsewhere.
        if self.dropdown.is_some() && !self.is_focused(window, cx) {
            self.dropdown = None;
        }
        let list = self.dropdown.map(|highlight| {
            let entries = history::search_keys(cx)
                .into_iter()
                .enumerate()
                .map(|(i, text)| {
                    let picked = text.clone();
                    div()
                        .id(("search-box-entry", i))
                        .debug_selector(move || format!("search-box-entry-{i}"))
                        .px_2()
                        .h(px(20.))
                        .truncate()
                        .when(i == highlight, |d| d.bg(rgb(0xcce4f7)))
                        .hover(|d| d.bg(rgb(0xe5f1fb)))
                        .child(SharedString::from(text))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.pick(picked.clone(), window, cx);
                        }))
                });
            // Over the editor, which is drawn after the toolbar.
            deferred(
                div()
                    .id("search-box-list")
                    .absolute()
                    .top(px(HEIGHT + 1.))
                    .left_0()
                    .w(px(WIDTH))
                    .max_h(px(200.))
                    .overflow_y_scroll()
                    .track_scroll(&self.list_scroll)
                    .occlude()
                    .border_1()
                    .border_color(rgb(0xb0b0b0))
                    .bg(rgb(0xffffff))
                    .text_size(px(12.))
                    .children(entries),
            )
            .with_priority(1)
        });
        div()
            .id("search-box")
            .debug_selector(|| "search-box".into())
            .relative()
            .flex()
            .items_center()
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .mx_1()
            .text_size(px(12.))
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.on_key(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            // `ProcSearchBox`: Tab moves the focus to the window. The edit field passes Tab and
            // Escape on (it is one line); Escape also closes the list.
            .on_action(cx.listener(|this, _: &IndentInline, window, cx| {
                this.focus_editor(window, cx);
            }))
            // A menu item chosen while the field has the focus: the focus goes to the view first,
            // and the view does it (`CEditWnd::OnCommand`).
            .on_action(cx.listener(|this, action: &RunFunction, window, cx| {
                let code = action.code;
                // The field has the focus already.
                if code == SEARCH_BOX {
                    return;
                }
                this.run_in_editor(window, cx, move |editor, window, cx| {
                    editor.run_function(code, &[], window, cx);
                });
            }))
            .on_action(cx.listener(|this, action: &OpenRecentFile, window, cx| {
                let path = action.path.clone();
                this.run_in_editor(window, cx, move |editor, window, cx| {
                    editor.open_recent_file(path, window, cx);
                });
            }))
            .on_action(cx.listener(|this, action: &OpenRecentFolder, window, cx| {
                let path = action.path.clone();
                this.run_in_editor(window, cx, move |editor, window, cx| {
                    editor.open_recent_folder(path, window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &Escape, window, cx| {
                if this.dropdown.take().is_some() {
                    cx.notify();
                } else {
                    this.focus_editor(window, cx);
                }
            }))
            .child(
                div()
                    .flex_1()
                    .child(Input::new(&self.input).small().cleanable(false)),
            )
            .child(
                div()
                    .id("search-box-toggle")
                    .debug_selector(|| "search-box-toggle".into())
                    .w(px(16.))
                    .h(px(HEIGHT))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|d| d.bg(rgb(0xd8e6f5)))
                    .child("▼")
                    .text_size(px(8.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        // The arrow does not take the focus; the keys of the list go to the box.
                        this.begin_session(cx);
                        this.focus_input(window, cx);
                        this.toggle_dropdown(cx);
                    })),
            )
            .children(list)
    }
}

#[cfg(test)]
mod tests {
    use super::accepted_text;

    #[test]
    fn the_box_shows_the_newest_string_when_there_is_news_or_no_key() {
        assert_eq!(
            accepted_text(Some("new"), "old", true, "old"),
            Some("new".into())
        );
        assert_eq!(
            accepted_text(Some("new"), "", false, ""),
            Some("new".into())
        );
    }

    #[test]
    fn the_box_shows_the_key_of_the_view_otherwise() {
        assert_eq!(
            accepted_text(Some("new"), "mine", false, "new"),
            Some("mine".into())
        );
        // Already there: nothing to set.
        assert_eq!(accepted_text(Some("new"), "mine", false, "mine"), None);
    }

    #[test]
    fn an_empty_history_leaves_the_box_alone() {
        assert_eq!(accepted_text(None, "mine", false, "typed"), None);
        assert_eq!(accepted_text(None, "", true, "typed"), None);
    }
}
