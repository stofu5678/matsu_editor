//! The box of a command that runs: Sakura's 「コマンド実行中・・・」 dialog (`IDD_EXECRUNNING`,
//! `CDlgCancel`) with the command line and the button キャンセル(X) that stops the process. The
//! rest of the window stays usable, but while the output goes into the document the box covers
//! the view and has the keys (Escape stops), so that nothing is typed into the text meanwhile.

use crate::a11y::Labelled as _;
use gpui::{
    AppContext as _, Context, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, Styled, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::{Disableable as _, h_flex, v_flex};
use ume_platform::exec::Stopper;

use crate::view::EditorView;

pub(crate) struct ExecRunning {
    command: String,
    stopper: Stopper,
    /// The output goes into the document: the box covers the view and takes the keys.
    modal: bool,
    /// 中断 was pressed.
    stopping: bool,
    focus: FocusHandle,
}

impl ExecRunning {
    fn new(command: &str, stopper: Stopper, modal: bool, cx: &mut Context<Self>) -> Self {
        Self {
            command: command.to_owned(),
            stopper,
            modal,
            stopping: false,
            focus: cx.focus_handle(),
        }
    }

    /// キャンセル(X): ends the process and the ones it started. The flow sees the end and goes on.
    pub(crate) fn stop(&mut self, cx: &mut Context<Self>) {
        if self.stopping {
            return;
        }
        self.stopping = true;
        let stopper = self.stopper.clone();
        // Stopping can take a moment (Windows starts `taskkill`): not on the window's thread.
        cx.background_executor()
            .spawn(async move { stopper.stop() })
            .detach();
        cx.notify();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            self.stop(cx);
        }
        if self.modal {
            // nothing reaches the text while the output goes into it
            cx.stop_propagation();
        }
    }
}

impl Focusable for ExecRunning {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ExecRunning {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = v_flex()
            .id("exec-running")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .track_focus(&self.focus)
            .key_context("ExecRunning")
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, _, cx| this.on_key_down(event, cx)),
            )
            .w(px(420.))
            .max_w_full()
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
                    .child("コマンド実行中・・・"),
            )
            .child(
                div()
                    .max_h(px(60.))
                    .overflow_hidden()
                    .child(self.command.clone()),
            )
            .child(
                h_flex().child(
                    Button::new("exec-stop")
                        .labelled(if self.stopping {
                            "中断しています…"
                        } else {
                            "キャンセル(X)"
                        })
                        .disabled(self.stopping)
                        .on_click(cx.listener(|this, _, _, cx| this.stop(cx))),
                ),
            );
        if self.modal {
            div()
                .id("exec-running-overlay")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .child(panel)
        } else {
            // beside the text: the window stays usable
            div()
                .id("exec-running-corner")
                .absolute()
                .bottom_2()
                .right_2()
                .child(panel)
        }
    }
}

impl EditorView {
    /// Shows the box of a running command and, when the output goes into the document, gives it
    /// the focus.
    pub(crate) fn open_exec_running(
        &mut self,
        command: &str,
        stopper: Stopper,
        modal: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let running = cx.new(|cx| ExecRunning::new(command, stopper, modal, cx));
        if modal {
            let focus = running.read(cx).focus.clone();
            window.focus(&focus, cx);
        }
        self.support.exec_running = Some(running);
        cx.notify();
    }
}
