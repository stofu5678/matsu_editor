//! The dialog that chooses a connection of `databases.ini` (ADR-0023): a list of the names, kinds
//! and commands, OK and Cancel. Sakura has nothing like it (its database function finds the one
//! SQL*Plus window), so the look follows [`crate::input_dialog`]: it is drawn inside the editor
//! window and covers the view, which takes no keys meanwhile.
//!
//! Keys: Up and Down (Home, End) move the choice, Enter is OK, Esc cancels, Tab and Shift+Tab move
//! between the list and the buttons (and stay inside the dialog). A double click on a row is OK.

use std::future::Future;

use gpui::{
    AppContext as _, ClickEvent, Context, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, KeyDownEvent, ParentElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled, WeakEntity, Window, div, px, rgb, rgba,
};

use crate::dialog_focus::KeepTabInside as _;
use crate::view::EditorView;

/// One row of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Choice {
    pub name: String,
    /// The kind as written in the file (`postgresql`).
    pub kind: String,
    pub command: String,
}

/// What to ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PickRequest {
    pub choices: Vec<Choice>,
    /// The row that starts chosen (the default connection of the file), if any.
    pub initial: Option<usize>,
    /// A line under the list, for the connections of the file that cannot be used.
    pub note: Option<String>,
}

pub(crate) struct ConnectionDialog {
    view: WeakEntity<EditorView>,
    request: PickRequest,
    chosen: usize,
    /// Keeps the chosen row in view when the list is longer than its box.
    scroll: ScrollHandle,
    /// Sends the chosen name once; dropping it unanswered (the window closed) is a cancel.
    answer: Option<async_channel::Sender<Option<String>>>,
    /// The panel: Tab stays inside it (`keep_tab_inside`).
    focus: FocusHandle,
    list_focus: FocusHandle,
    ok_focus: FocusHandle,
    cancel_focus: FocusHandle,
}

impl ConnectionDialog {
    fn new(
        request: PickRequest,
        view: WeakEntity<EditorView>,
        answer: async_channel::Sender<Option<String>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let chosen = request
            .initial
            .filter(|&i| i < request.choices.len())
            .unwrap_or(0);
        let scroll = ScrollHandle::new();
        scroll.scroll_to_item(chosen);
        Self {
            view,
            request,
            chosen,
            scroll,
            answer: Some(answer),
            focus: cx.focus_handle(),
            list_focus: cx.focus_handle().tab_stop(true),
            ok_focus: cx.focus_handle().tab_stop(true),
            cancel_focus: cx.focus_handle().tab_stop(true),
        }
    }

    /// The name of the chosen row (the tests read it).
    #[cfg(test)]
    pub(crate) fn chosen_name(&self) -> &str {
        &self.request.choices[self.chosen].name
    }

    fn finish(&mut self, name: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(answer) = self.answer.take() {
            let _ = answer.try_send(name);
        }
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.connection_dialog = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .request
            .choices
            .get(self.chosen)
            .map(|c| c.name.clone());
        self.finish(name, window, cx);
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish(None, window, cx);
    }

    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        self.chosen = index.min(self.request.choices.len().saturating_sub(1));
        self.scroll.scroll_to_item(self.chosen);
        cx.notify();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let last = self.request.choices.len().saturating_sub(1);
        match event.keystroke.key.as_str() {
            "escape" => self.cancel(window, cx),
            "up" if self.list_focus.is_focused(window) => {
                self.choose(self.chosen.saturating_sub(1), cx);
            }
            "down" if self.list_focus.is_focused(window) => self.choose(self.chosen + 1, cx),
            "home" if self.list_focus.is_focused(window) => self.choose(0, cx),
            "end" if self.list_focus.is_focused(window) => self.choose(last, cx),
            "enter" | "space" if self.cancel_focus.is_focused(window) => self.cancel(window, cx),
            "enter" => self.ok(window, cx),
            "space" if self.ok_focus.is_focused(window) => self.ok(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Focusable for ConnectionDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.list_focus.clone()
    }
}

const PANEL_TEXT: u32 = 0x222222;
const FOCUS_RING: u32 = 0x2f6fd0;
const CHOSEN_BG: u32 = 0xcfe0f8;
const TITLE: &str = "接続を選んでデータベースで実行";
const MESSAGE: &str = "実行する接続を選んでください。";

impl Render for ConnectionDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ring = |focused: bool| if focused { FOCUS_RING } else { 0xaaaaaa };
        let button = |id: &'static str, label: &'static str, handle: &FocusHandle| {
            crate::a11y::button(div().id(id), label)
                .track_focus(handle)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(ring(handle.is_focused(window))))
                .bg(rgb(0xf4f4f4))
                .child(label)
        };
        let ok = button("connection-ok", "OK", &self.ok_focus)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.ok(window, cx)));
        let cancel = button("connection-cancel", "キャンセル", &self.cancel_focus)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.cancel(window, cx)));

        let mut rows = Vec::new();
        for (index, choice) in self.request.choices.iter().enumerate() {
            let chosen = index == self.chosen;
            let shown = format!("{}、{}、{}", choice.name, choice.kind, choice.command);
            rows.push(
                crate::a11y::list_row(div().id(("connection-row", index)), &shown, chosen)
                    .px_2()
                    .py_1()
                    .flex()
                    .flex_col()
                    .bg(rgb(if chosen { CHOSEN_BG } else { 0xffffff }))
                    .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                        this.choose(index, cx);
                        window.focus(&this.list_focus, cx);
                        if event.click_count() >= 2 {
                            this.ok(window, cx);
                        }
                    }))
                    .child(div().child(format!("{}  [{}]", choice.name, choice.kind)))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(0x555555))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(choice.command.clone()),
                    ),
            );
        }
        let list = crate::a11y::group(div().id("connection-list"), MESSAGE)
            .role(gpui::Role::ListBox)
            .track_focus(&self.list_focus)
            .h(px(220.))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .border_1()
            .border_color(rgb(ring(self.list_focus.is_focused(window))))
            .bg(rgb(0xffffff))
            .children(rows);

        let note: Option<SharedString> = self.request.note.clone().map(Into::into);
        let panel = crate::a11y::dialog(div().id("connection-dialog"), TITLE)
            .aria_description(SharedString::from(MESSAGE))
            .track_focus(&self.focus)
            .keep_tab_inside(&self.focus)
            .key_context("ConnectionDialog")
            .on_key_down(cx.listener(Self::on_key_down))
            .w(px(520.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(PANEL_TEXT))
            .text_size(px(14.))
            .child(TITLE)
            .child(div().mt_2().child(MESSAGE))
            .child(list)
            .children(note.map(|note| {
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0x8a3b00))
                    .child(note)
            }))
            .child(
                div()
                    .mt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(ok)
                    .child(cancel),
            );
        div()
            .id("connection-dialog-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .bg(rgba(0x00000055))
            .flex()
            .items_center()
            .justify_center()
            .child(panel)
    }
}

impl EditorView {
    /// Shows the dialog over the view and returns the answer to wait for: the name of the chosen
    /// connection, or `None` for a cancel. The dialog is modal to the window; if another one is
    /// open, this one is cancelled at once.
    pub(crate) fn open_connection_dialog(
        &mut self,
        request: PickRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl Future<Output = Option<String>> + use<> {
        let (sender, receiver) = async_channel::bounded(1);
        if self.connection_dialog.is_some()
            || self.input_dialog.is_some()
            || request.choices.is_empty()
        {
            let _ = sender.try_send(None);
        } else {
            let view = cx.weak_entity();
            let dialog = cx.new(|cx| ConnectionDialog::new(request, view, sender, cx));
            let focus = dialog.read(cx).list_focus.clone();
            window.focus(&focus, cx);
            self.connection_dialog = Some(dialog);
            cx.notify();
        }
        async move { receiver.recv().await.ok().flatten() }
    }
}
