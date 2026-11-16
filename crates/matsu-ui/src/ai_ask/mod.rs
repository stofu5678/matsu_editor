//! The window of mode b, 「指示をつけて送る...」 (`F_AI_ASK`, ADR-0035, 4): one window where the person types an
//! instruction, picks the destination and sees what will be sent, and presses 「送る」 or 「送らない」. MATSU Editor's
//! own window; Sakura has nothing like it.
//!
//! It is the confirmation of ADR-0030 (案 2, 4 and 7) and the field of the instruction in one place, so that what is
//! shown is what is sent. The part that shows the destination, the command line, the counts, the warnings and the head
//! and the tail of the text is [`crate::ai_confirm::details`], the same as in mode a; here it shows the final standard
//! input: the instruction, a blank line, `---` and the target text (`matsu_core::ai_send::instruction_head`; the
//! numbers of the lines are those of that input, `matsu_core::ai_send::preview_with_head`). Nothing is sent until
//! 「送る」 is pressed, and nothing here sends: the caller (`crate::oneliner::flow`) does, when the answer says so.
//!
//! * The field takes several lines. 「送る」 cannot be pressed while the instruction is empty (only white space is
//!   empty). There is no 「次から聞かない」: `AskBeforeSend` is for mode a only.
//! * The destination is one of those the caller offers: the one used last in this run when it is among them, else the
//!   first. Up and Down change it while the list has the focus; so does a click.
//! * The history of the instructions of this run ([`history`]) is under the field: a click on a row puts the
//!   instruction into the field. Up and Down in an empty field (or in a field that still has what a row put there)
//!   walk through the history, newest first, and Down at the newest comes back to the empty field.
//! * Esc is 「送らない」, and so is closing the window. When the target text is over 100 KB the focus starts on
//!   「送らない」, not in the field, so that a hasty Enter does not send it (ADR-0030, 案 4).
//!
//! Keys: Tab and Shift+Tab move among the field, the list of destinations, 「送る」 and 「送らない」 (the field has Tab
//! for itself in other windows; here the dialog keeps it for moving, and a tab character is typed by pasting it). Enter
//! in the field is a line break. Enter and Space press the focused button.

pub(crate) mod history;

use std::future::Future;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AppContext as _, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::input::{
    Escape, IndentInline, InputEvent, MoveDown, MoveUp, OutdentInline, Textarea, TextareaState,
};
use matsu_core::ai_send::{
    PreviewLimits, TextPreview, instruction_head, normalized_instruction, preview_with_head,
};

use crate::a11y::AriaDisabled as _;
use crate::ai_confirm::{
    FOCUS_RING, MUTED_TEXT, PANEL_TEXT, Request as ConfirmRequest, button, details, overlay,
    ring_color,
};
use crate::view::EditorView;

/// The line at the top of the details while there is an instruction.
const LEAD: &str = "指示と選んだ文字列をつないだ内容が、そのまま標準入力で送られます";
/// The line at the top of the details while the instruction is empty.
const LEAD_EMPTY: &str =
    "指示が空です。指示を入れると、送る内容がここに出ます（いまは選んだ文字列だけを出しています）";
/// How many rows of the history are shown at once (the rest is reached with the scroll or the keys).
const HISTORY_ROWS: usize = 4;

/// A destination of `[AI]` that the window offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Destination {
    /// The number of its slot in `[AI]`.
    pub index: usize,
    /// Its name (the one the user gave it).
    pub name: String,
    /// The command line that would run.
    pub command_line: String,
}

/// What the window needs.
#[derive(Clone, Debug)]
pub(crate) struct Request {
    /// What the target text looks like (the same range as the standard input is read from).
    pub material: TextPreview,
    /// The destinations to choose from; at least one.
    pub destinations: Vec<Destination>,
    /// The instructions of this run, the newest first.
    pub history: Vec<String>,
    /// Which of `destinations` is chosen first.
    pub initial: usize,
}

/// What 「送る」 gives: the destination and the instruction as it is sent (trimmed, in LF).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Chosen {
    /// The number of the slot of `[AI]`.
    pub index: usize,
    pub instruction: String,
}

/// The position in `destinations` of the one to choose first: the one used last when it is offered, else the first.
pub(crate) fn initial_destination(destinations: &[Destination], last: Option<usize>) -> usize {
    last.and_then(|last| destinations.iter().position(|d| d.index == last))
        .unwrap_or(0)
}

pub(crate) struct AiAskDialog {
    view: WeakEntity<EditorView>,
    request: Request,
    /// The position in `request.destinations` of the chosen one.
    selected: usize,
    input: Entity<TextareaState>,
    /// The row of the history whose instruction the field shows, while it is unchanged.
    browsing: Option<usize>,
    /// Sends the answer once; dropping it unanswered (the window closed) is 「送らない」.
    answer: Option<async_channel::Sender<Option<Chosen>>>,
    destinations_focus: FocusHandle,
    send_focus: FocusHandle,
    dont_send_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl AiAskDialog {
    fn new(
        request: Request,
        view: WeakEntity<EditorView>,
        answer: async_channel::Sender<Option<Chosen>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .rows(4)
                .placeholder("どうするかを書く（例: 英語に翻訳して）")
        });
        let change = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, _, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.on_change(cx);
                }
            },
        );
        let selected = request
            .initial
            .min(request.destinations.len().saturating_sub(1));
        Self {
            view,
            request,
            selected,
            input,
            browsing: None,
            answer: Some(answer),
            destinations_focus: cx.focus_handle(),
            send_focus: cx.focus_handle(),
            dont_send_focus: cx.focus_handle(),
            _subscriptions: vec![change],
        }
    }

    fn field_focus(&self, cx: &gpui::App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }

    /// The focus when the window opens: the field, or 「送らない」 for large text.
    fn first_focus(&self, cx: &gpui::App) -> FocusHandle {
        if self.request.material.large {
            self.dont_send_focus.clone()
        } else {
            self.field_focus(cx)
        }
    }

    /// The field changed by typing or by a row of the history.
    fn on_change(&mut self, cx: &mut Context<Self>) {
        if let Some(at) = self.browsing
            && self.request.history.get(at).map(String::as_str)
                != Some(self.input.read(cx).value().as_ref())
        {
            self.browsing = None;
        }
        cx.notify();
    }

    /// The text of the field (the field owns it; nothing here keeps a copy).
    fn value(&self, cx: &gpui::App) -> SharedString {
        self.input.read(cx).value()
    }

    /// Whether the instruction can be sent: it has something besides white space.
    fn can_send(&self, cx: &gpui::App) -> bool {
        !self.value(cx).trim().is_empty()
    }

    /// The destination that is chosen.
    fn destination(&self) -> &Destination {
        &self.request.destinations[self.selected]
    }

    /// What the details show now: the final standard input, or (with no instruction yet) the target text alone.
    fn confirm_request(&self, cx: &gpui::App) -> ConfirmRequest {
        let destination = self.destination();
        let (preview, lead) = match instruction_head(&self.value(cx)) {
            Ok(head) => (
                preview_with_head(&head, &self.request.material, &PreviewLimits::default()),
                LEAD,
            ),
            Err(_) => (self.request.material.clone(), LEAD_EMPTY),
        };
        ConfirmRequest {
            name: destination.name.clone(),
            command_line: destination.command_line.clone(),
            preview,
            lead: Some(lead),
        }
    }

    #[cfg(test)]
    pub(crate) fn request(&self) -> &Request {
        &self.request
    }

    #[cfg(test)]
    pub(crate) fn text(&self, cx: &gpui::App) -> String {
        self.value(cx).to_string()
    }

    #[cfg(test)]
    pub(crate) fn selected_index(&self) -> usize {
        self.destination().index
    }

    #[cfg(test)]
    pub(crate) fn shown(&self, cx: &gpui::App) -> ConfirmRequest {
        self.confirm_request(cx)
    }

    /// Ends the window and gives the focus back to the view. `send` is honored only with an instruction.
    fn finish(&mut self, send: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(answer) = self.answer.take() else {
            return;
        };
        let chosen = if send {
            normalized_instruction(&self.value(cx))
                .ok()
                .map(|instruction| Chosen {
                    index: self.destination().index,
                    instruction,
                })
        } else {
            None
        };
        if let Some(chosen) = &chosen {
            history::remember(cx, &chosen.instruction, chosen.index);
        }
        let _ = answer.try_send(chosen);
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.ai_ask = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    fn select_destination(&mut self, at: usize, cx: &mut Context<Self>) {
        if at < self.request.destinations.len() && at != self.selected {
            self.selected = at;
            cx.notify();
        }
    }

    /// Puts the instruction of row `at` of the history into the field.
    fn use_history(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.request.history.get(at).cloned() else {
            return;
        };
        self.browsing = Some(at);
        self.input.update(cx, |state, cx| {
            state.set_value(text, window, cx);
        });
        cx.notify();
    }

    /// Up (`older`) or Down in an empty field, or in one that shows an instruction of the history unchanged: walks
    /// through the history. Returns false when the key is the field's own (the field has text of its own).
    fn walk_history(&mut self, older: bool, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let history = self.request.history.len();
        if history == 0 || !(self.value(cx).is_empty() || self.browsing.is_some()) {
            return false;
        }
        match (self.browsing, older) {
            (None, true) => self.use_history(0, window, cx),
            (Some(at), true) if at + 1 < history => self.use_history(at + 1, window, cx),
            (Some(0), false) => {
                // back to the empty field that it was
                self.browsing = None;
                self.input.update(cx, |state, cx| {
                    state.set_value(String::new(), window, cx);
                });
                cx.notify();
            }
            (Some(at), false) => self.use_history(at - 1, window, cx),
            // already at the oldest, or Down in an empty field: nothing to walk to
            _ => {}
        }
        true
    }

    /// Moves the focus to the next (or previous) of the field, the destinations, 「送る」 and 「送らない」.
    fn move_focus(&self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let order = [
            self.field_focus(cx),
            self.destinations_focus.clone(),
            self.send_focus.clone(),
            self.dont_send_focus.clone(),
        ];
        let at = order.iter().position(|h| h.contains_focused(window, cx));
        let next = match (at, forward) {
            (Some(i), true) => (i + 1) % order.len(),
            (Some(i), false) => (i + order.len() - 1) % order.len(),
            (None, _) => 0,
        };
        window.focus(&order[next], cx);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let on_destinations = self.destinations_focus.is_focused(window);
        let on_send = self.send_focus.is_focused(window);
        let on_dont = self.dont_send_focus.is_focused(window);
        match event.keystroke.key.as_str() {
            "escape" => self.finish(false, window, cx),
            "up" if on_destinations => {
                self.select_destination(self.selected.saturating_sub(1), cx);
            }
            "down" if on_destinations => self.select_destination(self.selected + 1, cx),
            "enter" | "space" if on_send => {
                if self.can_send(cx) {
                    self.finish(true, window, cx);
                }
            }
            "enter" | "space" if on_dont => self.finish(false, window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Focusable for AiAskDialog {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.first_focus(cx)
    }
}

/// The first line of an instruction, for a row of the history (the rest is shown as `…`).
fn row_text(instruction: &str) -> String {
    let mut lines = instruction.lines();
    let first = lines.next().unwrap_or_default();
    let mut text: String = first.chars().take(80).collect();
    if first.chars().count() > 80 || lines.next().is_some() {
        text.push('…');
    }
    text
}

impl Render for AiAskDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let can_send = self.can_send(cx);
        let send = button("ai-ask-send", "送る", &self.send_focus, window)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                if this.can_send(cx) {
                    this.finish(true, window, cx);
                }
            }))
            .aria_disabled(!can_send);
        let send = if can_send { send } else { send.opacity(0.45) };
        let dont_send = button(
            "ai-ask-dont-send",
            "送らない",
            &self.dont_send_focus,
            window,
        )
        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.finish(false, window, cx)));

        let destinations = self
            .request
            .destinations
            .iter()
            .enumerate()
            .map(|(at, destination)| {
                let chosen = at == self.selected;
                crate::a11y::list_row(
                    div().id(("ai-ask-destination", at)),
                    &destination.name,
                    chosen,
                )
                .px_2()
                .h(px(22.))
                .flex()
                .items_center()
                .cursor_pointer()
                .when(chosen, |row| row.border_1().border_color(rgb(FOCUS_RING)))
                .child(format!(
                    "{} {}",
                    if chosen { "(●)" } else { "(　)" },
                    destination.name
                ))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.select_destination(at, cx);
                    window.focus(&this.destinations_focus, cx);
                }))
            })
            .collect::<Vec<_>>();
        let destinations = crate::a11y::group(div().id("ai-ask-destinations"), "送り先")
            .role(gpui::Role::ListBox)
            .track_focus(&self.destinations_focus)
            .flex()
            .flex_col()
            .border_1()
            .border_color(rgb(ring_color(self.destinations_focus.is_focused(window))))
            .bg(rgb(0xffffff))
            .max_h(px(110.))
            .overflow_y_scroll()
            .children(destinations);

        let mut panel = crate::a11y::dialog(div().id("ai-ask-dialog"), "AI に指示をつけて送る")
            .key_context("AiAskDialog")
            .on_key_down(cx.listener(Self::on_key_down))
            // Tab and Shift+Tab, while the focus is on something other than the field (the field's own are the
            // actions below). The window's own bindings would move the focus out of the dialog otherwise.
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                let modifiers = event.keystroke.modifiers;
                if event.keystroke.key == "tab"
                    && !modifiers.control
                    && !modifiers.alt
                    && !modifiers.platform
                    && !this.field_focus(cx).contains_focused(window, cx)
                {
                    this.move_focus(!modifiers.shift, window, cx);
                    cx.stop_propagation();
                }
            }))
            // Tab, Shift+Tab, Esc and the arrows of an empty field are taken as the field's own actions would take
            // them: the field is deeper than this window in the key contexts, so its bindings win and only an
            // action handler that runs before them can leave them out.
            .capture_action(cx.listener(|this, _: &IndentInline, window, cx| {
                this.move_focus(true, window, cx);
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(|this, _: &OutdentInline, window, cx| {
                this.move_focus(false, window, cx);
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(|this, _: &Escape, window, cx| {
                this.finish(false, window, cx);
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(|this, _: &MoveUp, window, cx| {
                if this.walk_history(true, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .capture_action(cx.listener(|this, _: &MoveDown, window, cx| {
                if this.walk_history(false, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .w(px(680.))
            .max_w_full()
            .max_h_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(PANEL_TEXT))
            .text_size(px(14.))
            .child(SharedString::from("AI に指示をつけて送る"))
            .child("指示（どうするか）")
            .child(
                div()
                    .border_1()
                    .border_color(rgb(ring_color(
                        self.field_focus(cx).contains_focused(window, cx),
                    )))
                    .child(Textarea::new(&self.input).aria_label("指示")),
            );

        if !self.request.history.is_empty() {
            let rows = self
                .request
                .history
                .iter()
                .enumerate()
                .map(|(at, instruction)| {
                    let current = self.browsing == Some(at);
                    crate::a11y::list_row(div().id(("ai-ask-history", at)), instruction, current)
                        .px_2()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .cursor_pointer()
                        .when(current, |row| row.border_1().border_color(rgb(FOCUS_RING)))
                        .child(row_text(instruction))
                        .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                            this.use_history(at, window, cx);
                            window.focus(&this.field_focus(cx), cx);
                        }))
                })
                .collect::<Vec<_>>();
            panel = panel
                .child(
                    div()
                        .text_color(rgb(MUTED_TEXT))
                        .child("これまでの指示（クリックで入れる。空の欄で ↑ ↓ でも選べます）"),
                )
                .child(
                    crate::a11y::group(div().id("ai-ask-history"), "これまでの指示")
                        .role(gpui::Role::ListBox)
                        .flex()
                        .flex_col()
                        .border_1()
                        .border_color(rgb(0xaaaaaa))
                        .bg(rgb(0xffffff))
                        .max_h(px(22. * HISTORY_ROWS as f32))
                        .overflow_y_scroll()
                        .children(rows),
                );
        }

        let panel = panel
            .child("送り先")
            .child(destinations)
            .child(details(&self.confirm_request(cx)).max_h(px(250.)))
            .child(
                div()
                    .mt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(send)
                    .child(dont_send),
            );
        overlay("ai-ask-overlay").child(panel)
    }
}

impl EditorView {
    /// Shows the window over the view and returns the answer to wait for: the destination and the instruction, or
    /// `None` for 「送らない」. The window is modal to the editor window; if another dialog is open, this one is
    /// answered 「送らない」 at once. Nothing is sent by this call.
    pub(crate) fn ask_ai_instruction(
        &mut self,
        request: Request,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl Future<Output = Option<Chosen>> + use<> {
        let (sender, receiver) = async_channel::bounded(1);
        if self.ai_ask.is_some() || self.ai_confirm.is_some() || self.input_dialog.is_some() {
            let _ = sender.try_send(None);
        } else {
            let view = cx.weak_entity();
            let dialog = cx.new(|cx| AiAskDialog::new(request, view, sender, window, cx));
            let focus = dialog.read(cx).first_focus(cx);
            window.focus(&focus, cx);
            self.ai_ask = Some(dialog);
            cx.notify();
        }
        async move { receiver.recv().await.ok().flatten() }
    }
}

#[cfg(test)]
mod tests;
