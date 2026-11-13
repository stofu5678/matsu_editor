//! 「ワンライナーを入力して実行」 (`F_ONELINER_DIALOG`, ADR-0029): one line with the history of the lines that were run,
//! the place for the result, 実行, 「登録...」 and キャンセル.
//!
//! Sakura has no such dialog; this one is made like the one of 外部コマンド実行 (`crate::exec::dialog`): a modal panel over
//! the view, the history a list under a button 履歴, a box with the line in which Enter runs it.
//!
//! * 実行 gives the line and the place to the flow ([`run_dialog`]), which adds the line to the history (the
//!   history of 外部コマンド実行 is another one).
//! * 「登録...」 shows a box for the name. 登録 stores the line under that name in the first slot that has no
//!   command ([`super::store::register`]), with the place that is chosen, and says which slot it took; when all 50
//!   slots are taken it says so. The dialog stays open, so the line can be run as well.
//! * A document that cannot be edited offers only アウトプットに出す, as 外部コマンド実行's 編集中のウィンドウ is not
//!   offered then.

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AppContext as _, AsyncWindowContext, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::{h_flex, v_flex};
use matsu_config::settings::oneliner::OutputTo;

use super::flow::{Job, can_edit};
use crate::a11y::Labelled as _;
use crate::dialog_focus::KeepTabInside as _;
use crate::input_select::{focus_selecting, input_field};
use crate::view::EditorView;

/// What the dialog gives back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Choice {
    pub line: String,
    pub output: OutputTo,
}

pub(crate) struct OneLinerDialog {
    view: WeakEntity<EditorView>,
    pub(crate) line: Entity<InputState>,
    pub(crate) name: Entity<InputState>,
    /// The history, the most recent first.
    history: Vec<String>,
    pub(crate) output: OutputTo,
    /// The document can be edited (otherwise only アウトプットに出す is offered).
    editable: bool,
    list: bool,
    /// The box of the name is shown (after 「登録...」).
    pub(crate) registering: bool,
    /// What the last 「登録...」 said, under the choices.
    pub(crate) message: Option<String>,
    result: Option<async_channel::Sender<Option<Choice>>>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl OneLinerDialog {
    pub(crate) fn new(
        view: WeakEntity<EditorView>,
        history: Vec<String>,
        editable: bool,
        result: async_channel::Sender<Option<Choice>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // the newest line of the history is where the box starts, as in 外部コマンド実行
        let line = cx.new(|cx| input_field(window, cx).placeholder("ワンライナー (例: sort -u)"));
        let first = history.first().cloned().unwrap_or_default();
        line.update(cx, |state, cx| state.set_value(first, window, cx));
        let name = cx.new(|cx| input_field(window, cx).placeholder("名前"));
        let subscriptions = vec![
            cx.subscribe_in(
                &line,
                window,
                |this: &mut Self, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) && !this.registering {
                        this.ok(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &name,
                window,
                |this: &mut Self, _, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.register(cx);
                    }
                },
            ),
        ];
        Self {
            view,
            line,
            name,
            history,
            output: if editable {
                OutputTo::Replace
            } else {
                OutputTo::OutputWindow
            },
            editable,
            list: false,
            registering: false,
            message: None,
            result: Some(result),
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        focus_selecting(&self.line, window, cx);
    }

    /// The line and the place as they are now.
    pub(crate) fn choice(&self, cx: &gpui::App) -> Choice {
        Choice {
            line: self.line.read(cx).value().to_string(),
            output: self.output,
        }
    }

    /// 実行(E).
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let choice = self.choice(cx);
        if choice.line.trim().is_empty() {
            self.message = Some("ワンライナーを入力してください。".to_owned());
            cx.notify();
            return;
        }
        if let Some(result) = self.result.take() {
            let _ = result.try_send(Some(choice));
        }
        self.close(window, cx);
    }

    /// キャンセル(X).
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(result) = self.result.take() {
            let _ = result.try_send(None);
        }
        self.close(window, cx);
    }

    /// 「登録...」(R): shows the box for the name.
    pub(crate) fn start_registering(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.line.read(cx).value().trim().is_empty() {
            self.message = Some("登録するワンライナーを入力してください。".to_owned());
            cx.notify();
            return;
        }
        self.registering = true;
        self.list = false;
        self.message = None;
        focus_selecting(&self.name, window, cx);
        cx.notify();
    }

    /// 登録: keeps the line under the name in the first free slot and says which, or why not.
    pub(crate) fn register(&mut self, cx: &mut Context<Self>) {
        let choice = self.choice(cx);
        let name = self.name.read(cx).value().to_string();
        self.message = Some(
            match super::store::register(cx, &name, &choice.line, choice.output) {
                Ok(index) => {
                    self.registering = false;
                    format!("ワンライナー {index} に登録しました。")
                }
                Err(error) => error.to_string(),
            },
        );
        cx.notify();
    }

    /// Takes the dialog away from the view and gives the focus back to the text.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.support.oneliner_dialog = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            if self.registering {
                self.registering = false;
                focus_selecting(&self.line, window, cx);
                cx.notify();
            } else if self.list {
                self.list = false;
                cx.notify();
            } else {
                self.cancel(window, cx);
            }
            cx.stop_propagation();
        }
    }

    /// 履歴: opens or closes the list.
    fn toggle_list(&mut self, cx: &mut Context<Self>) {
        self.list = !self.list;
        cx.notify();
    }

    /// A row of the list: its text goes into the box.
    pub(crate) fn pick(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let text = text.to_owned();
        self.line
            .update(cx, |state, cx| state.set_value(text, window, cx));
        self.list = false;
        cx.notify();
    }

    /// The history, the most recent first, as the list shows it.
    #[cfg(test)]
    pub(crate) fn history_rows(&self) -> &[String] {
        &self.history
    }

    fn history_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = v_flex()
            .id("oneliner-history")
            .max_h(px(140.))
            .overflow_y_scroll()
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .bg(rgb(0xffffff));
        if self.history.is_empty() {
            list = list.child(div().p_1().text_color(rgb(0x888888)).child("(履歴なし)"));
        }
        for (i, text) in self.history.iter().cloned().enumerate() {
            let pick = text.clone();
            list = list.child(
                div()
                    .id(("oneliner-history", i))
                    .px_1()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0xdde8f5)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick(&pick, window, cx);
                    }))
                    .child(text),
            );
        }
        list
    }

    fn radio(
        &self,
        id: &'static str,
        label: &'static str,
        to: OutputTo,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Radio {
        Radio::new(id)
            .labelled(label)
            .checked(self.output == to)
            .disabled(!enabled)
            .on_click(cx.listener(move |this, _: &bool, _, cx| {
                this.output = to;
                cx.notify();
            }))
    }
}

impl Focusable for OneLinerDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for OneLinerDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editable = self.editable;
        let line_row = h_flex()
            .gap_2()
            .child(div().flex_1().child(Input::new(&self.line)))
            .child(
                Button::new("oneliner-history-button")
                    .labelled("履歴")
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_list(cx))),
            );
        let places = v_flex()
            .gap_1()
            .p_1()
            .border_1()
            .border_color(rgb(0xcccccc))
            .child("結果の置き場所")
            .child(
                h_flex()
                    .gap_3()
                    .child(self.radio(
                        "oneliner-replace",
                        "置き換える(R)",
                        OutputTo::Replace,
                        editable,
                        cx,
                    ))
                    .child(self.radio(
                        "oneliner-next-line",
                        "次の行に入れる(N)",
                        OutputTo::NextLine,
                        editable,
                        cx,
                    ))
                    .child(self.radio(
                        "oneliner-output",
                        "アウトプットに出す(O)",
                        OutputTo::OutputWindow,
                        true,
                        cx,
                    )),
            );
        let buttons = if self.registering {
            h_flex()
                .gap_2()
                .child(div().flex_1().child(Input::new(&self.name)))
                .child(
                    Button::new("oneliner-register-ok")
                        .labelled("登録")
                        .on_click(cx.listener(|this, _, _, cx| this.register(cx))),
                )
                .child(
                    Button::new("oneliner-register-back")
                        .labelled("戻る")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.registering = false;
                            focus_selecting(&this.line, window, cx);
                            cx.notify();
                        })),
                )
        } else {
            h_flex()
                .gap_2()
                .child(
                    Button::new("oneliner-ok")
                        .labelled("実行(E)")
                        .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
                )
                .child(
                    Button::new("oneliner-register")
                        .labelled("登録...(T)")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.start_registering(window, cx)),
                        ),
                )
                .child(
                    Button::new("oneliner-cancel")
                        .labelled("キャンセル(X)")
                        .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                )
        };
        let panel = v_flex()
            .id("oneliner-dialog")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .track_focus(&self.focus)
            .keep_tab_inside(&self.focus)
            .key_context("OneLinerDialog")
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.on_key_down(event, window, cx)
            }))
            .w(px(520.))
            .max_w_full()
            .max_h_full()
            .overflow_y_scroll()
            .p_3()
            .gap_2()
            .bg(rgb(0xf7f7f7))
            .text_color(rgb(0x222222))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0x888888))
            .rounded_md()
            .shadow_md()
            .child(
                div()
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("ワンライナーを入力して実行"),
            )
            .child(line_row)
            .when(self.list, |this| this.child(self.history_list(cx)))
            .child(
                div()
                    .text_color(rgb(0x666666))
                    .child("選んだ文字列 (なければ文書全体) が標準入力に渡されます。"),
            )
            .child(places)
            .when_some(self.message.clone(), |this, message| {
                this.child(div().text_color(rgb(0x994400)).child(message))
            })
            .child(buttons);
        div()
            .id("oneliner-dialog-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .child(panel)
    }
}

impl EditorView {
    /// Opens the dialog of the one-liners and focuses it.
    pub(crate) fn open_oneliner_dialog(
        &mut self,
        history: Vec<String>,
        editable: bool,
        result: async_channel::Sender<Option<Choice>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().downgrade();
        let dialog = cx.new(|cx| OneLinerDialog::new(view, history, editable, result, window, cx));
        dialog.update(cx, |dialog, cx| dialog.focus_first(window, cx));
        self.support.oneliner_dialog = Some(dialog);
        cx.notify();
    }
}

/// `F_ONELINER_DIALOG`: asks, remembers the line, and runs it. Cancelling does nothing.
pub(crate) async fn run_dialog(view: &WeakEntity<EditorView>, cx: &mut AsyncWindowContext) {
    let Ok(editable) = view.read_with(cx, can_edit) else {
        return;
    };
    let Ok(history) = cx.update(|_, app| super::table(app).recent) else {
        return;
    };
    let (sender, receiver) = async_channel::bounded(1);
    let opened = view.update_in(cx, move |view, window, cx| {
        view.open_oneliner_dialog(history, editable, sender, window, cx);
    });
    if opened.is_err() {
        return;
    }
    // cancelling is "done": nothing runs
    let Some(choice) = receiver.recv().await.ok().flatten() else {
        return;
    };
    {
        let line = choice.line.clone();
        cx.update(|_, app| super::store::remember(app, &line)).ok();
    }
    // The window may be another by now: the flow reads it again (and says so when the text cannot be changed).
    super::flow::run(
        view,
        cx,
        Job {
            line: choice.line,
            output: choice.output,
        },
    )
    .await;
}
