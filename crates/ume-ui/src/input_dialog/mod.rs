//! A dialog with one line of text: a message, a text field and OK / Cancel.
//!
//! Sakura: `CDlgInput1` (`sakura_core/dlg/CDlgInput1.cpp`, the dialog `IDD_INPUT1` of
//! `sakura_rc.rc`): the message above the field, the field filled with the given text and limited
//! to a maximum length, OK takes the text, Cancel (or Esc) takes nothing. `InputBox` of the macros
//! uses it. It is drawn inside the editor window and covers the view, which takes no keys
//! meanwhile (as [`crate::file_ops::save_as_dialog`] does).
//!
//! The dialog is not tied to the macros: [`EditorView::open_input_dialog`] takes an
//! [`InputRequest`] and answers the text. The pages of the common settings that ask for a name
//! (the custom menu, the keyword sets) could use it instead of their own fields.
//!
//! Keys: Enter in the field is OK, Esc cancels, Tab and Shift+Tab move between the field and the
//! buttons.

use std::future::Future;

use crate::input_select::{focus_selecting, input_field};
use gpui::{
    AppContext as _, ClickEvent, Context, Entity, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, Subscription, WeakEntity, Window, div, px, rgb, rgba,
};
use gpui_component::input::{Input, InputEvent, InputState};
use ume_plugin::functions::window::truncate_utf16;

use crate::view::EditorView;

/// What to ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputRequest {
    /// The caption of the dialog.
    pub title: String,
    /// The line above the field.
    pub message: String,
    /// The text the field starts with. It is cut to `max_len`.
    pub default: String,
    /// The longest text the field takes, in UTF-16 code units (`EM_LIMITTEXT` counts those).
    pub max_len: usize,
}

pub(crate) struct InputDialog {
    view: WeakEntity<EditorView>,
    title: SharedString,
    message: SharedString,
    input: Entity<InputState>,
    max_len: usize,
    /// The text of the field after the last change, within the maximum length.
    accepted: String,
    /// Sends the answer once; dropping it unanswered (the window closed) is a cancel.
    answer: Option<async_channel::Sender<Option<String>>>,
    ok_focus: FocusHandle,
    cancel_focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl InputDialog {
    fn new(
        request: InputRequest,
        view: WeakEntity<EditorView>,
        answer: async_channel::Sender<Option<String>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| input_field(window, cx));
        let default = truncate_utf16(&request.default, request.max_len).to_owned();
        let accepted = default.clone();
        input.update(cx, |state, cx| state.set_value(default, window, cx));
        let subscription = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, _, event: &InputEvent, window, cx| match event {
                InputEvent::Change => this.limit(window, cx),
                InputEvent::PressEnter { .. } => this.ok(window, cx),
                _ => {}
            },
        );
        Self {
            view,
            title: request.title.into(),
            message: request.message.into(),
            input,
            max_len: request.max_len,
            accepted,
            answer: Some(answer),
            ok_focus: cx.focus_handle(),
            cancel_focus: cx.focus_handle(),
            _subscriptions: vec![subscription],
        }
    }

    /// The message above the field.
    #[cfg(test)]
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    /// The text of the field.
    fn text(&self, cx: &gpui::App) -> String {
        self.input.read(cx).value().to_string()
    }

    /// Keeps the field within the maximum length: what is typed or pasted beyond it is cut off
    /// (the edit control of Sakura's dialog refuses it), and the text that was there stays.
    fn limit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.text(cx);
        match limit_edit(&self.accepted, &text, self.max_len) {
            Some((kept, caret)) => {
                self.accepted.clone_from(&kept);
                // The caret goes back to where the refused edit was, not to the end of the text.
                // `set_value` clears the undo history: the field has no way to refuse an edit
                // before it is recorded, and `replace_all`, which keeps the history, records the
                // correction as a step whose undo brings the refused text back (and is cut again).
                self.input.update(cx, |state, cx| {
                    state.set_value(kept, window, cx);
                    state.set_selected_range(caret..caret, cx);
                });
            }
            None => self.accepted = text,
        }
    }

    fn focus_field(&self, window: &mut Window, cx: &mut Context<Self>) {
        focus_selecting(&self.input, window, cx);
    }

    /// Ends the dialog with `text` (`None` cancels) and gives the focus back to the view.
    fn finish(&mut self, text: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(answer) = self.answer.take() {
            let _ = answer.try_send(text);
        }
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.input_dialog = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.text(cx);
        let kept = truncate_utf16(&text, self.max_len).to_owned();
        self.finish(Some(kept), window, cx);
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish(None, window, cx);
    }

    /// Moves the focus to the next (or previous) of the field, OK and Cancel.
    fn move_focus(&self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let field = self.input.read(cx).focus_handle(cx);
        let order = [field, self.ok_focus.clone(), self.cancel_focus.clone()];
        let at = order.iter().position(|h| h.contains_focused(window, cx));
        let next = match (at, forward) {
            (Some(i), true) => (i + 1) % order.len(),
            (Some(i), false) => (i + order.len() - 1) % order.len(),
            (None, _) => 0,
        };
        window.focus(&order[next], cx);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => self.cancel(window, cx),
            "tab" => self.move_focus(!event.keystroke.modifiers.shift, window, cx),
            "enter" if self.cancel_focus.is_focused(window) => self.cancel(window, cx),
            "enter" if self.ok_focus.is_focused(window) => self.ok(window, cx),
            "space" if self.cancel_focus.is_focused(window) => self.cancel(window, cx),
            "space" if self.ok_focus.is_focused(window) => self.ok(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Focusable for InputDialog {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }
}

/// The field after an edit from `before` to `after`, when `after` is longer than `max` UTF-16 code
/// units: the part that was inserted is cut so that the whole fits, and the text around it stays
/// (inserting `X` at the start of a full field gives nothing, not a field without its last
/// character). `None` when `after` fits. The second value is the byte offset of the caret
/// after the kept part of the insertion.
///
/// The change event of the field does not say what range was edited, so the range is inferred
/// from the common start and end of the two texts. When a selection is replaced by a text that
/// begins or ends with the selected text itself, the old text is taken as untouched and the
/// replacement is refused; the text that was there is never lost.
fn limit_edit(before: &str, after: &str, max: usize) -> Option<(String, usize)> {
    if after.encode_utf16().count() <= max {
        return None;
    }
    // The edit replaced the middle of `before`: what the two have in common at the start and at
    // the end stays, and the rest of `after` is what was inserted.
    let prefix = before
        .char_indices()
        .zip(after.chars())
        .take_while(|((_, a), b)| a == b)
        .map(|((at, a), _)| at + a.len_utf8())
        .last()
        .unwrap_or(0);
    let (before_rest, after_rest) = (&before[prefix..], &after[prefix..]);
    let suffix = before_rest
        .chars()
        .rev()
        .zip(after_rest.chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    let inserted = &after_rest[..after_rest.len() - suffix];
    let tail = &before_rest[before_rest.len() - suffix..];
    let units = |text: &str| text.encode_utf16().count();
    let others = units(&before[..prefix]) + units(tail);
    let room = max.saturating_sub(others);
    let mut kept = String::from(&before[..prefix]);
    kept.push_str(truncate_utf16(inserted, room));
    let caret = kept.len();
    kept.push_str(tail);
    Some((kept, caret))
}

const PANEL_TEXT: u32 = 0x222222;
const FOCUS_RING: u32 = 0x2f6fd0;

impl Render for InputDialog {
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
        let ok = button("input-ok", "OK", &self.ok_focus)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.ok(window, cx)));
        let cancel = button("input-cancel", "キャンセル", &self.cancel_focus)
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| this.cancel(window, cx)));
        let panel = crate::a11y::dialog(div().id("input-dialog"), &self.title)
            .aria_description(self.message.clone())
            .key_context("InputDialog")
            .on_key_down(cx.listener(Self::on_key_down))
            .w(px(420.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(PANEL_TEXT))
            .text_size(px(14.))
            .child(self.title.clone())
            .child(div().mt_2().child(self.message.clone()))
            .child(Input::new(&self.input).aria_label(self.message.clone()))
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
            .id("input-dialog-overlay")
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
    /// Shows the dialog over the view and returns the answer to wait for: the text, or `None` for
    /// a cancel. The dialog is modal to the window; if another one is open, this one is cancelled
    /// at once.
    pub(crate) fn open_input_dialog(
        &mut self,
        request: InputRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl Future<Output = Option<String>> + use<> {
        let (sender, receiver) = async_channel::bounded(1);
        if self.input_dialog.is_some() {
            let _ = sender.try_send(None);
        } else {
            let view = cx.weak_entity();
            let dialog = cx.new(|cx| InputDialog::new(request, view, sender, window, cx));
            dialog.update(cx, |dialog, cx| dialog.focus_field(window, cx));
            self.input_dialog = Some(dialog);
            cx.notify();
        }
        async move { receiver.recv().await.ok().flatten() }
    }
}

#[cfg(test)]
mod tests;
