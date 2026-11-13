//! The confirmation window before text is sent to a generative AI (ADR-0030, 案 2, 4 and 7).
//!
//! An AI is a one-liner (ADR-0029) with the AI mark. Before it runs, this window shows where the
//! text goes (the name of the one-liner), the command line, the first and the last lines of the
//! text, how big it is, and the lines that look like secrets (partly masked). Nothing is sent
//! until 「送る」 is pressed. It is drawn inside the editor window and covers the view, which takes
//! no keys meanwhile (like [`crate::input_dialog`]).
//!
//! The API for the code that runs the one-liner: build a [`Request`] with
//! [`Request::from_document`] (it reads only the head, the tail and the counts, within a byte
//! budget: `matsu_core::ai_send`), then `view.ask_ai_confirm(request, window, cx).await` gives an
//! [`Answer`].
//!
//! Large text (over 100 KB, ADR-0030 案 4): the window says so and 「送らない」 has the focus, so
//! that a hasty Enter does not send it.
//!
//! Keys: Esc is 「送らない」; Tab and Shift+Tab move among the check box and the two buttons;
//! Space toggles the check box; Enter and Space press the focused button. Closing the window
//! without an answer is 「送らない」.

use std::future::Future;

use gpui::{
    AppContext as _, ClickEvent, Context, FocusHandle, Focusable, InteractiveElement as _,
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString,
    StatefulInteractiveElement as _, Styled, WeakEntity, Window, div, px, rgb, rgba,
};
use matsu_core::ai_send::{
    LARGE_TEXT_BYTES, PreviewLimits, PreviewLine, TextPreview, preview_range,
};
use matsu_core::buffer::{Document, Pos, Result as BufferResult};

use crate::view::EditorView;

/// What to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Request {
    /// The destination: the name the user gave the one-liner.
    pub name: String,
    /// The command line that will run.
    pub command_line: String,
    /// The text, as far as the window shows it.
    pub preview: TextPreview,
}

impl Request {
    /// A request for the text of `doc` between `from` and `to` (the selection, or the whole
    /// document). Reads the head, the tail and the counts only; see [`preview_range`] for how the
    /// work is bounded.
    pub(crate) fn from_document(
        name: impl Into<String>,
        command_line: impl Into<String>,
        doc: &Document,
        from: Pos,
        to: Pos,
    ) -> BufferResult<Self> {
        Ok(Self {
            name: name.into(),
            command_line: command_line.into(),
            preview: preview_range(doc, from, to, &PreviewLimits::default())?,
        })
    }
}

/// The user's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Answer {
    /// 「送る」 was pressed. Esc, 「送らない」 and closing the window are false.
    pub send: bool,
    /// The check box 「次から聞かない（このワンライナーだけ）」 was on when 「送る」 was pressed.
    /// It is false when nothing is sent: not sending changes no setting.
    pub dont_ask_again: bool,
}

impl Answer {
    const NO: Answer = Answer {
        send: false,
        dont_ask_again: false,
    };
}

pub(crate) struct AiConfirmDialog {
    view: WeakEntity<EditorView>,
    request: Request,
    dont_ask_again: bool,
    /// Sends the answer once; dropping it unanswered (the window closed) is 「送らない」.
    answer: Option<async_channel::Sender<Answer>>,
    check_focus: FocusHandle,
    send_focus: FocusHandle,
    dont_send_focus: FocusHandle,
}

impl AiConfirmDialog {
    fn new(
        request: Request,
        view: WeakEntity<EditorView>,
        answer: async_channel::Sender<Answer>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            view,
            request,
            dont_ask_again: false,
            answer: Some(answer),
            check_focus: cx.focus_handle(),
            send_focus: cx.focus_handle(),
            dont_send_focus: cx.focus_handle(),
        }
    }

    /// The focus when the window opens: 「送る」, or 「送らない」 for large text.
    fn first_focus(&self) -> &FocusHandle {
        if self.request.preview.large {
            &self.dont_send_focus
        } else {
            &self.send_focus
        }
    }

    #[cfg(test)]
    pub(crate) fn request(&self) -> &Request {
        &self.request
    }

    /// Ends the window with `send` and gives the focus back to the view.
    fn finish(&mut self, send: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(answer) = self.answer.take() {
            let _ = answer.try_send(Answer {
                send,
                dont_ask_again: send && self.dont_ask_again,
            });
        }
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.ai_confirm = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    fn toggle_check(&mut self, cx: &mut Context<Self>) {
        self.dont_ask_again = !self.dont_ask_again;
        cx.notify();
    }

    /// Moves the focus to the next (or previous) of the check box, 「送る」 and 「送らない」.
    fn move_focus(&self, forward: bool, window: &mut Window, cx: &mut Context<Self>) {
        let order = [
            self.check_focus.clone(),
            self.send_focus.clone(),
            self.dont_send_focus.clone(),
        ];
        let at = order.iter().position(|h| h.is_focused(window));
        let next = match (at, forward) {
            (Some(i), true) => (i + 1) % order.len(),
            (Some(i), false) => (i + order.len() - 1) % order.len(),
            (None, _) => 0,
        };
        window.focus(&order[next], cx);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let on_check = self.check_focus.is_focused(window);
        let on_send = self.send_focus.is_focused(window);
        let on_dont = self.dont_send_focus.is_focused(window);
        match event.keystroke.key.as_str() {
            "escape" => self.finish(false, window, cx),
            "tab" => self.move_focus(!event.keystroke.modifiers.shift, window, cx),
            "space" if on_check => self.toggle_check(cx),
            "enter" | "space" if on_send => self.finish(true, window, cx),
            "enter" | "space" if on_dont => self.finish(false, window, cx),
            // Enter on the check box does nothing: it must not send by accident.
            "enter" if on_check => {}
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Focusable for AiConfirmDialog {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.first_focus().clone()
    }
}

const PANEL_TEXT: u32 = 0x222222;
const FOCUS_RING: u32 = 0x2f6fd0;
const WARNING_TEXT: u32 = 0xa02020;
const MUTED_TEXT: u32 = 0x555555;

/// The size as the window says it, in KB or MB.
fn size_text(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes.div_ceil(1024))
    }
}

/// `1,234`: the thousands are separated so that a long count can be read.
fn count_text(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The line with the counts: `12,345 文字 / 678 行 / 24 KB`. A lower bound has 「以上」.
fn counts_text(preview: &TextPreview) -> String {
    let more = if preview.chars_exact { "" } else { " 以上" };
    format!(
        "{}{more} 文字 / {} 行 / {}",
        count_text(preview.chars),
        count_text(preview.lines),
        size_text(preview.bytes),
    )
}

fn numbered(line: &PreviewLine) -> String {
    format!("{:>5} | {}", line.number, line.text)
}

impl Render for AiConfirmDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preview = &self.request.preview;
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
        let send = button("ai-confirm-send", "送る", &self.send_focus).on_click(
            cx.listener(|this, _: &ClickEvent, window, cx| this.finish(true, window, cx)),
        );
        let dont_send = button("ai-confirm-dont-send", "送らない", &self.dont_send_focus).on_click(
            cx.listener(|this, _: &ClickEvent, window, cx| this.finish(false, window, cx)),
        );

        let check = div()
            .id("ai-confirm-dont-ask")
            .role(gpui::Role::CheckBox)
            .aria_label("次から聞かない（このワンライナーだけ）")
            .aria_toggled(if self.dont_ask_again {
                gpui::Toggled::True
            } else {
                gpui::Toggled::False
            })
            .track_focus(&self.check_focus)
            .flex()
            .items_center()
            .gap_2()
            .px_1()
            .border_1()
            .border_color(rgb(ring(self.check_focus.is_focused(window))))
            .child(
                div()
                    .w(px(16.))
                    .h(px(16.))
                    .border_1()
                    .border_color(rgb(0x666666))
                    .bg(rgb(0xffffff))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if self.dont_ask_again { "✓" } else { "" }),
            )
            .child("次から聞かない（このワンライナーだけ）")
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.toggle_check(cx);
                window.focus(&this.check_focus, cx);
            }));

        let mut text_block = div()
            .id("ai-confirm-text")
            .role(gpui::Role::Group)
            .aria_label("送る文字")
            .p_2()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .flex()
            .flex_col()
            .overflow_x_hidden();
        if preview.lines == 0 {
            text_block = text_block.child("（空）");
        }
        for line in &preview.head {
            text_block = text_block.child(numbered(line));
        }
        if preview.omitted_lines > 0 {
            text_block = text_block.child(div().text_color(rgb(MUTED_TEXT)).child(format!(
                "      （{} 行を省略）",
                count_text(preview.omitted_lines)
            )));
        }
        for line in &preview.tail {
            text_block = text_block.child(numbered(line));
        }

        let mut body = div()
            .id("ai-confirm-body")
            .flex()
            .flex_col()
            .gap_2()
            .max_h(px(430.))
            .overflow_y_scroll()
            .child(format!("送り先: {}", self.request.name))
            .child(div().child(format!("コマンド: {}", self.request.command_line)))
            .child(counts_text(preview));
        if preview.large {
            body = body.child(div().text_color(rgb(WARNING_TEXT)).child(format!(
                "テキストが大きい（{} を超えています）。本当に全部を送るか確かめてください。\
                         初めに選ばれているのは「送らない」です。",
                size_text(LARGE_TEXT_BYTES)
            )));
        }
        if preview.secret_lines_total > 0 {
            let mut warning = div()
                .id("ai-confirm-secrets")
                .role(gpui::Role::Group)
                .aria_label("秘密に見える行")
                .p_2()
                .border_1()
                .border_color(rgb(WARNING_TEXT))
                .text_color(rgb(WARNING_TEXT))
                .flex()
                .flex_col()
                .child(format!(
                    "警告: 秘密（API キー、トークン、秘密鍵など）に見える行が {} 行あります。\
                     一部を伏せて表示しています。",
                    count_text(preview.secret_lines_total)
                ));
            for line in &preview.secrets {
                let kinds: Vec<&str> = line.kinds.iter().map(|k| k.label()).collect();
                warning = warning.child(format!(
                    "{} 行目 ({}): {}",
                    line.number,
                    kinds.join("、"),
                    line.text
                ));
            }
            if preview.secret_lines_total > preview.secrets.len() {
                warning = warning.child(format!(
                    "ほか {} 行",
                    count_text(preview.secret_lines_total - preview.secrets.len())
                ));
            }
            body = body.child(warning);
        }
        body = body
            .child("送る文字（最初と最後。秘密に見える部分は伏せています）")
            .child(text_block);
        if !preview.secrets_complete {
            body = body.child(
                div().text_color(rgb(MUTED_TEXT)).child(
                    "大きいので、秘密に見える行の検査は先頭の一部だけです。文字数は少なく数えています。",
                ),
            );
        }
        body = body
            .child(div().text_color(rgb(MUTED_TEXT)).child(
                "MATSU は、このコマンドのフラグが AI のツールを本当に切るかを確かめられません。",
            ))
            .child(
                div().text_color(rgb(MUTED_TEXT)).child(
                    "秘密に見える行の検査は形によるものです。見つけられない秘密もあります。",
                ),
            );

        let title: SharedString = "AI に送る前の確認".into();
        let panel = crate::a11y::dialog(div().id("ai-confirm-dialog"), &title)
            .key_context("AiConfirmDialog")
            .on_key_down(cx.listener(Self::on_key_down))
            .w(px(640.))
            .max_w_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf0f0f0))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(PANEL_TEXT))
            .text_size(px(14.))
            .child(title.clone())
            .child(body)
            .child(check)
            .child(
                div()
                    .mt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(send)
                    .child(dont_send),
            );
        div()
            .id("ai-confirm-overlay")
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
    /// Shows the window over the view and returns the answer to wait for. The window is modal to
    /// the editor window; if another dialog of this kind is open, this one is answered 「送らない」
    /// at once. Nothing is sent by this call: the caller sends the text only when the answer says
    /// `send`.
    pub(crate) fn ask_ai_confirm(
        &mut self,
        request: Request,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl Future<Output = Answer> + use<> {
        let (sender, receiver) = async_channel::bounded(1);
        if self.ai_confirm.is_some() || self.input_dialog.is_some() {
            let _ = sender.try_send(Answer::NO);
        } else {
            let view = cx.weak_entity();
            let dialog = cx.new(|cx| AiConfirmDialog::new(request, view, sender, cx));
            let focus = dialog.read(cx).first_focus().clone();
            window.focus(&focus, cx);
            self.ai_confirm = Some(dialog);
            cx.notify();
        }
        async move { receiver.recv().await.unwrap_or(Answer::NO) }
    }
}

#[cfg(test)]
mod tests;
