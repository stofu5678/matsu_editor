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
use crate::access_keys::{Entry, Found, OnAccessKey as _, pick};
use crate::dialog_focus::KeepTabInside as _;
use crate::input_select::{focus_selecting, input_field};
use crate::support::help::OnHelpKey as _;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, PathPromptOptions, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, Subscription, WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::radio::Radio;
use gpui_component::{Disableable as _, h_flex, v_flex};
use matsu_config::settings::ExecHistory;
use matsu_core::exec::{
    DIALOG_ENCODINGS, ExecFlags, INPUT_BITS, OUTPUT_BITS, input_choice, output_choice,
};

use crate::view::EditorView;

/// The labels with their access keys (Alt+letter, [`ExecDialog::use_access_key`]).
const NAME: &str = "名前(N)";
const GET_STDOUT: &str = "標準出力を得る(S)";
const TO_OUTPUT: &str = "アウトプットウィンドウ(O)";
const TO_WINDOW: &str = "編集中のウィンドウ(C)";
const SEND_STDIN: &str = "標準入力に送る(I)";
const CUR_DIR: &str = "カレントディレクトリ(R)";
const RUN: &str = "実行(E)";
const CANCEL: &str = "キャンセル(X)";
const HELP: &str = crate::support::help::BUTTON_LABEL;

/// What an access key does.
#[derive(Clone, Copy)]
enum Use {
    /// The label 名前 in front of the box of the command: the focus goes to the box.
    FocusCommand,
    GetStdout,
    ToOutput,
    ToWindow,
    SendStdin,
    CurDir,
    Run,
    Cancel,
    /// ヘルプ(H): the page of the dialog.
    Help,
}

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

/// The open history list: its box and the row that Enter takes (one value, so that the row
/// cannot belong to another box).
#[derive(Clone, Copy)]
struct OpenList {
    field: Field,
    row: usize,
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
    list: Option<OpenList>,
    /// Scrolls the open list to its row.
    list_scroll: ScrollHandle,
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
        let command = cx.new(|cx| input_field(window, cx).placeholder("コマンド"));
        let first = history.commands.newest().unwrap_or_default().to_owned();
        command.update(cx, |state, cx| state.set_value(first, window, cx));
        let cur_dir = cx.new(|cx| input_field(window, cx).placeholder("カレントディレクトリ"));
        let first = history.cur_dirs.newest().unwrap_or_default().to_owned();
        cur_dir.update(cx, |state, cx| state.set_value(first, window, cx));
        let mut subscriptions = Vec::new();
        for (field, input) in [(Field::Command, &command), (Field::CurDir, &cur_dir)] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                move |this: &mut Self, _, event: &InputEvent, window, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        // The key binding of Enter runs before the key listeners, so the open
                        // list of this box takes it here: the row is taken and the command
                        // does not run. A list of the other box does not take it.
                        if this.list.is_some_and(|open| open.field == field) {
                            this.take_row(field, window, cx);
                        } else {
                            this.ok(window, cx);
                        }
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
            list_scroll: ScrollHandle::new(),
            result: Some(result),
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        focus_selecting(&self.command, window, cx);
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

    /// `IDC_BUTTON_HELP`: `CDlgExec` opens the page of `F_EXECMD_DIALOG`.
    fn open_help(&self, cx: &mut Context<Self>) {
        crate::support::help::open_function_page(cx, "F_EXECMD_DIALOG");
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

    /// カレントディレクトリ is switched; off, its history list closes.
    fn set_use_cur_dir(&mut self, on: bool, cx: &mut Context<Self>) {
        self.use_cur_dir = on;
        if !on && self.list.is_some_and(|open| open.field == Field::CurDir) {
            self.list = None;
        }
        cx.notify();
    }

    /// Alt with the letter of a label (`IsDialogMessage`): switches the check box, chooses the radio
    /// button, presses the button, or moves the focus to the box after the label. Returns whether the
    /// letter was one of the dialog's (then the key is not typed into the box).
    pub(crate) fn use_access_key(
        &mut self,
        letter: char,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let stdout = self.get_stdout;
        let entries = [
            Entry::labelled(NAME, true, Use::FocusCommand),
            Entry::labelled(GET_STDOUT, true, Use::GetStdout),
            Entry::labelled(TO_OUTPUT, stdout, Use::ToOutput),
            Entry::labelled(TO_WINDOW, stdout && self.editable, Use::ToWindow),
            Entry::labelled(SEND_STDIN, true, Use::SendStdin),
            Entry::labelled(CUR_DIR, true, Use::CurDir),
            Entry::labelled(RUN, true, Use::Run),
            Entry::labelled(CANCEL, true, Use::Cancel),
            Entry::labelled(HELP, true, Use::Help),
        ];
        match pick(letter, &entries) {
            Found::Nothing => return false,
            Found::Several => {}
            Found::One(Use::FocusCommand) => focus_selecting(&self.command, window, cx),
            Found::One(Use::GetStdout) => {
                self.get_stdout = !self.get_stdout;
                cx.notify();
            }
            Found::One(Use::ToOutput) => {
                self.to_window = false;
                cx.notify();
            }
            Found::One(Use::ToWindow) => {
                self.to_window = true;
                cx.notify();
            }
            Found::One(Use::SendStdin) => {
                self.send_stdin = !self.send_stdin;
                cx.notify();
            }
            Found::One(Use::CurDir) => self.set_use_cur_dir(!self.use_cur_dir, cx),
            Found::One(Use::Run) => self.ok(window, cx),
            Found::One(Use::Cancel) => self.cancel(window, cx),
            Found::One(Use::Help) => self.open_help(cx),
        }
        true
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

    /// 履歴: opens or closes the list of the box. It opens on the row that holds the text of the box.
    /// The box gets the focus, as in a combo box: the keys of the list go to the box that has it.
    fn toggle_list(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        if self.list.is_some_and(|open| open.field == field) {
            self.list = None;
        } else {
            let current = self.input(field).read(cx).value();
            let row = self
                .history_rows(field)
                .iter()
                .position(|row| row == current.as_str())
                .unwrap_or(0);
            self.list = Some(OpenList { field, row });
            self.list_scroll.scroll_to_item(row);
        }
        let input = self.input(field).clone();
        input.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    fn input(&self, field: Field) -> &Entity<InputState> {
        match field {
            Field::Command => &self.command,
            Field::CurDir => &self.cur_dir,
        }
    }

    /// The box that has the focus, if it is one of the two combo boxes.
    fn focused_field(&self, window: &Window, cx: &App) -> Option<Field> {
        [Field::Command, Field::CurDir]
            .into_iter()
            // `IDC_COMBO_CUR_DIR` is disabled while カレントディレクトリ is not checked
            .filter(|field| *field == Field::Command || self.use_cur_dir)
            .find(|field| {
                self.input(*field)
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window)
            })
    }

    /// The keys of a combo box (`CBS_DROPDOWN`): F4 and Alt+Down open or close the list; with it
    /// open Up, Down, Enter (see `take_row`) and Escape work in it; with it closed Up and Down go
    /// through the history. Returns whether the key was used.
    fn combo_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let focused = self.focused_field(window, cx);
        if self.list.is_some_and(|open| Some(open.field) != focused) {
            // The focus left the box of the list: the list closes.
            self.list = None;
            cx.notify();
        }
        let Some(field) = focused else {
            return false;
        };
        let key = event.keystroke.key.as_str();
        let m = &event.keystroke.modifiers;
        let plain = !m.alt && !m.control && !m.platform && !m.shift;
        if key == "f4" || (key == "down" && m.alt) {
            self.toggle_list(field, window, cx);
            return true;
        }
        let rows = self.history_rows(field);
        let down = key == "down";
        if !(plain && (down || key == "up")) || rows.is_empty() {
            return false;
        }
        let Some(open) = self.list else {
            // closed: the next row after the one that holds the text of the box
            let current = self.input(field).read(cx).value();
            let next = match (rows.iter().position(|r| r == current.as_str()), down) {
                (Some(i), true) => (i + 1).min(rows.len() - 1),
                (Some(i), false) => i.saturating_sub(1),
                (None, _) => 0,
            };
            let text = rows[next].clone();
            self.input(field)
                .update(cx, |state, cx| state.set_value(text, window, cx));
            return true;
        };
        let row = if down {
            (open.row + 1).min(rows.len() - 1)
        } else {
            open.row.saturating_sub(1)
        };
        self.list = Some(OpenList { field, row });
        self.list_scroll.scroll_to_item(row);
        cx.notify();
        true
    }

    /// Enter in an open list: the highlighted row goes into the box.
    fn take_row(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        let rows = self.history_rows(field);
        let row = self.list.map_or(0, |open| open.row);
        match rows.get(row) {
            Some(text) => self.pick(field, text, window, cx),
            None => {
                self.list = None;
                cx.notify();
            }
        }
    }

    /// A row of the list: its text goes into the box (`CBN_SELCHANGE`).
    fn pick(&mut self, field: Field, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input(field);
        let text = text.to_owned();
        input.update(cx, |state, cx| state.set_value(text, window, cx));
        self.list = None;
        cx.notify();
    }

    /// Which list is open: "command" or "dir".
    #[cfg(test)]
    pub(crate) fn open_list(&self) -> Option<&'static str> {
        self.list.map(|open| match open.field {
            Field::Command => "command",
            Field::CurDir => "dir",
        })
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
        let name = match field {
            Field::Command => "コマンドの履歴",
            Field::CurDir => "カレントディレクトリの履歴",
        };
        let row = self.list.map_or(usize::MAX, |open| open.row);
        let mut list = crate::a11y::list_box(v_flex().id(id), name)
            .max_h(px(140.))
            .overflow_y_scroll()
            .track_scroll(&self.list_scroll)
            .border_1()
            .border_color(rgb(0xaaaaaa))
            .bg(rgb(0xffffff));
        if empty {
            list = list.child(div().p_1().text_color(rgb(0x888888)).child("(履歴なし)"));
        }
        for (i, text) in rows.into_iter().enumerate() {
            let pick = text.clone();
            list = list.child(
                crate::a11y::list_row(div().id((id, i)), &text, i == row)
                    .px_1()
                    .when(i == row, |d| d.bg(rgb(0xcce4f7)))
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
        let this = cx.entity();
        let stdout = self.get_stdout;
        let output_item = self.output_item;
        let input_item = self.input_item;
        let command_row = h_flex()
            .gap_2()
            .child(div().flex_1().child(Input::new(&self.command)))
            .child(
                Button::new("exec-command-history-button")
                    .labelled("履歴")
                    // `IDC_COMBO_m_szCommand` is a combo box: Tab skips the button.
                    .tab_stop(false)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.toggle_list(Field::Command, window, cx)
                    })),
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
                div()
                    .debug_selector(|| "exec-dir-history-button".into())
                    .child(
                        Button::new("exec-dir-history-button")
                            .labelled("履歴")
                            .disabled(!self.use_cur_dir)
                            // `IDC_COMBO_CUR_DIR` is a combo box: Tab skips the button.
                            .tab_stop(false)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_list(Field::CurDir, window, cx)
                            })),
                    ),
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
            .child(NAME)
            .child(command_row)
            .when_some(
                self.list
                    .is_some_and(|open| open.field == Field::Command)
                    .then(|| self.history_list(Field::Command, cx)),
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
                            .labelled(GET_STDOUT)
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
                                    .labelled(TO_OUTPUT)
                                    .checked(!self.to_window)
                                    .disabled(!stdout)
                                    .on_click(cx.listener(|this, _: &bool, _, cx| {
                                        this.to_window = false;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Radio::new("exec-to-window")
                                    .labelled(TO_WINDOW)
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
                            .labelled(SEND_STDIN)
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
                    .labelled(CUR_DIR)
                    .checked(self.use_cur_dir)
                    .on_click(cx.listener(|this, value: &bool, _, cx| {
                        this.set_use_cur_dir(*value, cx);
                    })),
            )
            .child(dir_row)
            .when_some(
                self.list
                    .is_some_and(|open| open.field == Field::CurDir)
                    .then(|| self.history_list(Field::CurDir, cx)),
                |this, list| this.child(list),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("exec-ok")
                            .labelled(RUN)
                            .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
                    )
                    .child(
                        Button::new("exec-cancel")
                            .labelled(CANCEL)
                            .on_click(cx.listener(|this, _, window, cx| this.cancel(window, cx))),
                    )
                    .child(
                        Button::new("exec-help")
                            .labelled(HELP)
                            .on_click(cx.listener(|this, _, _, cx| this.open_help(cx))),
                    ),
            );
        let panel = crate::a11y::dialog(v_flex().id("exec-dialog"), "ファイル名を指定して実行")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .track_focus(&self.focus)
            .keep_tab_inside(&self.focus)
            // Alt+letter works from the boxes too, and does not type the letter
            .on_access_key(move |letter, window, cx| {
                this.update(cx, |dialog, cx| dialog.use_access_key(letter, window, cx))
            })
            // F1: the page of the dialog
            .on_help_key("F_EXECMD_DIALOG")
            .key_context("ExecDialog")
            // The combo box keys come first: Up, Down and Enter would otherwise reach the input.
            .capture_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.combo_key(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
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
