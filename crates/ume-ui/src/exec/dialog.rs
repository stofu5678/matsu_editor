//! The dialog of 外部コマンド実行 (`IDD_EXEC`, `CDlgExec`): the command with its history,
//! 標準出力を得る with its encoding and the place for it, 標準入力に送る with its encoding, and
//! カレントディレクトリ with its history.
//!
//! Sakura: `sakura_core/dlg/CDlgExec.cpp`; the layout is `IDD_EXEC` in `sakura_core/sakura_rc.rc`.
//! It is a modal panel over the view, like the other dialogs of this program. The two combo boxes
//! of Sakura are a text box with a 履歴 button that opens the list below it.
//!
//! Not done: the help button.

use crate::a11y::Labelled as _;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, PathPromptOptions, Render,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::{Disableable as _, h_flex, v_flex};
use ume_config::settings::ExecHistory;
use ume_core::exec::{
    DIALOG_ENCODINGS, ExecFlags, INPUT_BITS, OUTPUT_BITS, input_choice, output_choice,
};

use crate::view::EditorView;

/// What the dialog gives back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Choice {
    pub command: String,
    /// Empty when カレントディレクトリ is not checked (`GetData`).
    pub cur_dir: String,
    pub flags: ExecFlags,
}

/// The box whose history list is open.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Command,
    CurDir,
}

pub(crate) struct ExecDialog {
    view: WeakEntity<EditorView>,
    pub(crate) command: Entity<InputState>,
    pub(crate) cur_dir: Entity<InputState>,
    history: ExecHistory,
    /// 標準出力を得る.
    pub(crate) get_stdout: bool,
    /// 編集中のウィンドウ (else アウトプットウィンドウ).
    pub(crate) to_window: bool,
    /// 標準入力に送る.
    pub(crate) send_stdin: bool,
    /// カレントディレクトリ.
    pub(crate) use_cur_dir: bool,
    /// The items of the two combo boxes of the encodings.
    pub(crate) output_item: usize,
    pub(crate) input_item: usize,
    /// The document can be edited (otherwise 編集中のウィンドウ is not offered).
    editable: bool,
    list: Option<Field>,
    result: Option<async_channel::Sender<Option<Choice>>>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl ExecDialog {
    pub(crate) fn new(
        view: WeakEntity<EditorView>,
        history: ExecHistory,
        flags: ExecFlags,
        editable: bool,
        result: async_channel::Sender<Option<Choice>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // `SetData`: the newest of each history, and the choices of the last time
        let flags = flags.for_dialog(editable);
        let command = cx.new(|cx| InputState::new(window, cx).placeholder("コマンド"));
        let first = history.commands.newest().unwrap_or_default().to_owned();
        command.update(cx, |state, cx| state.set_value(first, window, cx));
        let cur_dir = cx.new(|cx| InputState::new(window, cx).placeholder("カレントディレクトリ"));
        let first = history.cur_dirs.newest().unwrap_or_default().to_owned();
        cur_dir.update(cx, |state, cx| state.set_value(first, window, cx));
        let mut subscriptions = Vec::new();
        for input in [&command, &cur_dir] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |this: &mut Self, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.ok(window, cx);
                    }
                },
            ));
        }
        Self {
            view,
            command,
            cur_dir,
            history,
            get_stdout: flags.get_stdout(),
            to_window: flags.0 & ExecFlags::TO_WINDOW != 0,
            send_stdin: flags.send_stdin(),
            use_cur_dir: flags.uses_cur_dir(),
            output_item: output_choice(flags),
            input_item: input_choice(flags),
            editable,
            list: None,
            result: Some(result),
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.command.update(cx, |state, cx| state.focus(window, cx));
    }

    /// `GetData`: the sum of bits of the choices.
    pub(crate) fn flags(&self) -> ExecFlags {
        let mut bits = 0;
        if self.get_stdout {
            bits |= ExecFlags::GET_STDOUT;
        }
        if self.to_window {
            bits |= ExecFlags::TO_WINDOW;
        }
        if self.send_stdin {
            bits |= ExecFlags::SEND_STDIN;
        }
        if self.use_cur_dir {
            bits |= ExecFlags::CUR_DIR;
        }
        bits |= OUTPUT_BITS[self.output_item.min(OUTPUT_BITS.len() - 1)];
        bits |= INPUT_BITS[self.input_item.min(INPUT_BITS.len() - 1)];
        ExecFlags(bits)
    }

    /// `GetData`: the command, the folder (only when it is checked) and the choices.
    pub(crate) fn choice(&self, cx: &gpui::App) -> Choice {
        Choice {
            command: self.command.read(cx).value().to_string(),
            cur_dir: if self.use_cur_dir {
                self.cur_dir.read(cx).value().to_string()
            } else {
                String::new()
            },
            flags: self.flags(),
        }
    }

    /// 実行(E).
    pub(crate) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let choice = self.choice(cx);
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

    /// Takes the dialog away from the view and gives the focus back to the text.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.support.exec_dialog = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            if self.list.take().is_some() {
                cx.notify();
            } else {
                self.cancel(window, cx);
            }
            cx.stop_propagation();
        }
    }

    /// 履歴: opens or closes the list of the box.
    fn toggle_list(&mut self, field: Field, cx: &mut Context<Self>) {
        self.list = if self.list == Some(field) {
            None
        } else {
            Some(field)
        };
        cx.notify();
    }

    /// A row of the list: its text goes into the box (`CBN_SELCHANGE`).
    fn pick(&mut self, field: Field, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let input = match field {
            Field::Command => &self.command,
            Field::CurDir => &self.cur_dir,
        };
        let text = text.to_owned();
        input.update(cx, |state, cx| state.set_value(text, window, cx));
        self.list = None;
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn pick_command(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.pick(Field::Command, text, window, cx);
    }

    /// `IDC_BUTTON_REFERENCE` (a program) and `IDC_BUTTON_REFERENCE2` (a folder).
    fn browse(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: field == Field::Command,
            directories: field == Field::CurDir,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = chosen.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update_in(cx, |this, window, cx| {
                    let text = quote_if_needed(&path, field);
                    this.pick(field, &text, window, cx);
                })
                .ok();
            }
        })
        .detach();
    }

    fn history_rows(&self, field: Field) -> Vec<String> {
        match field {
            Field::Command => self.history.commands.items().to_vec(),
            Field::CurDir => self.history.cur_dirs.items().to_vec(),
        }
    }

    fn history_list(&self, field: Field, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.history_rows(field);
        let (id, empty) = match field {
            Field::Command => ("exec-command-history", rows.is_empty()),
            Field::CurDir => ("exec-dir-history", rows.is_empty()),
        };
        let mut list = v_flex()
            .id(id)
            .max_h(px(140.))
            .overflow_y_scroll()
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .bg(rgb(0xffffff));
        if empty {
            list = list.child(div().p_1().text_color(rgb(0x888888)).child("(履歴なし)"));
        }
        for (i, text) in rows.into_iter().enumerate() {
            let pick = text.clone();
            list = list.child(
                div()
                    .id((id, i))
                    .px_1()
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(0xdde8f5)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.pick(field, &pick, window, cx);
                    }))
                    .child(text),
            );
        }
        list
    }

    /// The button that goes through the encodings of a combo box.
    fn encoding_button(
        &self,
        id: &'static str,
        item: usize,
        enabled: bool,
        cx: &mut Context<Self>,
        next: impl Fn(&mut Self) + 'static,
    ) -> Button {
        Button::new(id)
            .labelled(DIALOG_ENCODINGS[item.min(DIALOG_ENCODINGS.len() - 1)].combo_name())
            .disabled(!enabled)
            .on_click(cx.listener(move |this, _, _, cx| {
                next(this);
                cx.notify();
            }))
    }
}

/// A program or a folder as it is put in the box: a name with a space is quoted for the
/// command line, as the file dialog of Sakura does not (the box is the command line itself).
fn quote_if_needed(path: &std::path::Path, field: Field) -> String {
    let text = path.display().to_string();
    if field == Field::Command && text.contains(' ') {
        format!("\"{text}\"")
    } else {
        text
    }
}

impl Focusable for ExecDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ExecDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let stdout = self.get_stdout;
        let output_item = self.output_item;
        let input_item = self.input_item;
        let command_row = h_flex()
            .gap_2()
            .child(div().flex_1().child(Input::new(&self.command)))
            .child(
                Button::new("exec-command-history-button")
                    .labelled("履歴")
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_list(Field::Command, cx))),
            )
            .child(
                Button::new("exec-browse")
                    .label("...")
                    .accessibility_label("コマンドを参照")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.browse(Field::Command, window, cx)),
                    ),
            );
        let dir_row = h_flex()
            .gap_2()
            .child(div().flex_1().child(Input::new(&self.cur_dir)))
            .child(
                Button::new("exec-dir-history-button")
                    .labelled("履歴")
                    .disabled(!self.use_cur_dir)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_list(Field::CurDir, cx))),
            )
            .child(
                Button::new("exec-dir-browse")
                    .label("...")
                    .accessibility_label("カレントディレクトリを参照")
                    .disabled(!self.use_cur_dir)
                    .on_click(
                        cx.listener(|this, _, window, cx| this.browse(Field::CurDir, window, cx)),
                    ),
            );
        let body = v_flex()
            .gap_2()
            .child("名前(N)")
            .child(command_row)
            .when_some(
                (self.list == Some(Field::Command)).then(|| self.history_list(Field::Command, cx)),
                |this, list| this.child(list),
            )
            .child(
                div()
                    .text_color(rgb(0x666666))
                    .child("[Note] $Fで編集中ファイル名, $$で$"),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new("exec-stdout")
                            .labelled("標準出力を得る(S)")
                            .checked(stdout)
                            .on_click(cx.listener(|this, value: &bool, _, cx| {
                                this.get_stdout = *value;
                                cx.notify();
                            })),
                    )
                    .child(self.encoding_button(
                        "exec-out-encoding",
                        output_item,
                        stdout,
                        cx,
                        |d| {
                            d.output_item = (d.output_item + 1) % DIALOG_ENCODINGS.len();
                        },
                    )),
            )
            .child(
                v_flex()
                    .gap_1()
                    .p_1()
                    .border_1()
                    .border_color(rgb(0xcccccc))
                    .child("標準出力リダイレクト先")
                    .child(
                        h_flex()
                            .gap_3()
                            .child(
                                Radio::new("exec-to-output")
                                    .labelled("アウトプットウィンドウ(O)")
                                    .checked(!self.to_window)
                                    .disabled(!stdout)
                                    .on_click(cx.listener(|this, _: &bool, _, cx| {
                                        this.to_window = false;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Radio::new("exec-to-window")
                                    .labelled("編集中のウィンドウ(C)")
                                    .checked(self.to_window)
                                    .disabled(!(stdout && self.editable))
                                    .on_click(cx.listener(|this, _: &bool, _, cx| {
                                        this.to_window = true;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Checkbox::new("exec-stdin")
                            .labelled("標準入力に送る(I)")
                            .checked(self.send_stdin)
                            .on_click(cx.listener(|this, value: &bool, _, cx| {
                                this.send_stdin = *value;
                                cx.notify();
                            })),
                    )
                    .child(self.encoding_button(
                        "exec-in-encoding",
                        input_item,
                        self.send_stdin,
                        cx,
                        |d| d.input_item = (d.input_item + 1) % DIALOG_ENCODINGS.len(),
                    )),
            )
            .child(
                Checkbox::new("exec-cur-dir")
                    .labelled("カレントディレクトリ(R)")
                    .checked(self.use_cur_dir)
                    .on_click(cx.listener(|this, value: &bool, _, cx| {
                        this.use_cur_dir = *value;
                        if !*value && this.list == Some(Field::CurDir) {
                            this.list = None;
                        }
                        cx.notify();
                    })),
            )
            .child(dir_row)
            .when_some(
                (self.list == Some(Field::CurDir)).then(|| self.history_list(Field::CurDir, cx)),
                |this, list| this.child(list),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("exec-ok")
                            .labelled("実行(E)")
                            .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
                    )
                    .child(
                        Button::new("exec-cancel")
                            .labelled("キャンセル(X)")
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    ),
            );
        let panel = v_flex()
            .id("exec-dialog")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .track_focus(&self.focus)
            .key_context("ExecDialog")
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
                    .child("ファイル名を指定して実行"),
            )
            .child(body);
        div()
            .id("exec-dialog-overlay")
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
    /// Opens the dialog of 外部コマンド実行 and focuses it.
    pub(crate) fn open_exec_dialog(
        &mut self,
        history: ExecHistory,
        flags: ExecFlags,
        editable: bool,
        result: async_channel::Sender<Option<Choice>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().downgrade();
        let dialog =
            cx.new(|cx| ExecDialog::new(view, history, flags, editable, result, window, cx));
        dialog.update(cx, |dialog, cx| dialog.focus_first(window, cx));
        self.support.exec_dialog = Some(dialog);
        cx.notify();
    }
}
