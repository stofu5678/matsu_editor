//! The view of the AI log tab: a header line and the records as text that can be selected and copied.

use gpui::{
    App, AppContext as _, ClipboardItem, Context, Entity, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, KeyDownEvent, ParentElement as _, Render, Styled as _,
    WeakEntity, Window, div, px, rgb,
};
use gpui_component::text::TextViewState;
use matsu_commands::keymap::Lookup;
use matsu_commands::sakura::FUNCTIONS;

use super::records::{Finished, Log, RecordId, Started};
use super::{Close, Copy, SelectAll};
use crate::app::Workspace;

/// The line above the records: what the tab is, and that it is not kept.
const HEADER: &str = "AI ログ: 生成 AI に送った文と、その答え（メモリだけに持ち、ファイルには書きません。閉じると消えます）";

/// The log of one window.
pub(crate) struct AiLogView {
    log: Log,
    state: Entity<TextViewState>,
    focus: FocusHandle,
    workspace: WeakEntity<Workspace>,
}

impl AiLogView {
    pub(crate) fn new(workspace: WeakEntity<Workspace>, cx: &mut Context<Self>) -> Self {
        let log = Log::default();
        let text = log.markdown();
        let state = cx.new(|cx| TextViewState::markdown(&text, cx));
        Self {
            log,
            state,
            focus: cx.focus_handle(),
            workspace,
        }
    }

    #[cfg(test)]
    pub(crate) fn log(&self) -> &Log {
        &self.log
    }

    /// Adds a running record. The view does not scroll by itself: the text view fills its list after it parses
    /// the text, so a scroll made here would land on an empty list.
    pub(crate) fn begin(&mut self, started: Started, cx: &mut Context<Self>) -> RecordId {
        let id = self.log.begin(started);
        self.show(cx);
        id
    }

    /// Ends record `id`. A record that the limit dropped is not there any more, and nothing changes.
    pub(crate) fn finish(&mut self, id: RecordId, finished: Finished, cx: &mut Context<Self>) {
        if self.log.finish(id, finished) {
            self.show(cx);
        }
    }

    /// Gives the text view the text of the log again.
    fn show(&mut self, cx: &mut Context<Self>) {
        let text = self.log.markdown();
        self.state.update(cx, |state, cx| state.set_text(&text, cx));
        cx.notify();
    }

    /// The text that is selected in the records.
    pub(crate) fn selected_text(&self, cx: &App) -> String {
        self.state.read(cx).selected_text()
    }

    /// Puts the selected text on the clipboard. Nothing is selected: the clipboard is left as it is.
    pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
        let text = self.selected_text(cx);
        if !text.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    pub(crate) fn select_all(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.select_all(cx));
    }

    /// Runs `f` on the window after the action that is running ends: the window changes the tabs,
    /// and this view is one of them.
    fn in_workspace(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) + 'static,
    ) {
        let host = self.workspace.clone();
        window.defer(cx, move |window, cx| {
            host.update(cx, |workspace, cx| f(workspace, window, cx))
                .ok();
        });
    }

    /// The keys of the user's key map for the functions that mean something without an editor
    /// (the same list the preview runs: [`crate::preview::WINDOW_FUNCTIONS`]).
    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.prefer_character_input {
            return;
        }
        let Some(chord) = crate::keys::chord_of(&event.keystroke) else {
            return;
        };
        let code = match crate::app::with_key_map(cx, |map| map.lookup(chord)) {
            Lookup::Command(reg) => FUNCTIONS
                .iter()
                .find(|f| f.name == reg.sakura)
                .map(|f| (f.code, f.name)),
            Lookup::NotImplemented {
                function: Some(f), ..
            } => Some((f.code, f.name)),
            _ => None,
        };
        let Some((code, name)) = code else {
            return;
        };
        if !crate::preview::WINDOW_FUNCTIONS.contains(&name) {
            return;
        }
        cx.stop_propagation();
        self.in_workspace(window, cx, move |ws, window, cx| {
            ws.run_after_event(code, window, cx);
        });
    }
}

impl Focusable for AiLogView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AiLogView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = gpui_base::TextView::new(&self.state)
            .scrollable(true)
            .selectable(true)
            // A log has no links or pictures of its own; a link in an answer is text.
            .on_link_click(|_, _, _, _| {});
        crate::a11y::preview_region(div().id("ai-log"), HEADER)
            .key_context("AiLog")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_action(cx.listener(|this, _: &Copy, _, cx| this.copy(cx)))
            .on_action(cx.listener(|this, _: &SelectAll, _, cx| this.select_all(cx)))
            .on_action(cx.listener(|this, _: &Close, window, cx| {
                this.in_workspace(window, cx, |ws, window, cx| {
                    ws.close_active_own_tab(window, cx)
                });
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x1e1e1e))
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(0xd0d0d0))
                    .bg(rgb(0xf3f3f3))
                    .text_size(px(12.))
                    .child(HEADER),
            )
            .child(div().flex_1().min_h_0().p_2().child(body))
    }
}
