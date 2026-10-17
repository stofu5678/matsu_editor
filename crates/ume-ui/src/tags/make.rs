//! The dialog of 「タグファイルの作成」 (`F_TAGS_MAKE`): the folder, 「サブフォルダーも対象にする」 and
//! the options for ctags; and the small window that stands for Sakura's 「中断」 dialog while ctags
//! runs.
//!
//! Sakura: `CDlgTagsMake` (`sakura_core/dlg/CDlgTagsMake.cpp`; the layout is `IDD_TAG_MAKE` in
//! `sakura_core/sakura_rc.rc`) and `CViewCommander::Command_TagsMake`
//! (`sakura_core/cmd/CViewCommander_TagJump.cpp`), whose flow is in [`super`].
//!
//! The dialog is a modal panel over the view. 作成 gives the choice back to the flow and the same
//! panel then shows the message 「タグファイルを作成中です。」 and 中断 until the flow closes it.
//!
//! Not done: the help button.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{
    AppContext as _, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, PathPromptOptions, Render, Styled, Subscription,
    WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{h_flex, v_flex};
use ume_core::tags_make::TagsMakeOptions;

use crate::view::EditorView;

/// What the dialog gives back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MakeChoice {
    pub folder: PathBuf,
    pub options: TagsMakeOptions,
}

enum Phase {
    /// Asking.
    Form,
    /// ctags is running.
    Running,
}

pub(crate) struct TagsMakeDialog {
    view: WeakEntity<EditorView>,
    folder: Entity<InputState>,
    command_line: Entity<InputState>,
    recursive: bool,
    phase: Phase,
    result: Option<async_channel::Sender<Option<MakeChoice>>>,
    /// Set by 中断; the flow hands it to the runner of ctags.
    cancel: Arc<AtomicBool>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl TagsMakeDialog {
    pub(crate) fn new(
        view: WeakEntity<EditorView>,
        folder: &str,
        options: &TagsMakeOptions,
        result: async_channel::Sender<Option<MakeChoice>>,
        cancel: Arc<AtomicBool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let folder_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("タグ作成フォルダー"));
        folder_input.update(cx, |state, cx| {
            state.set_value(folder.to_owned(), window, cx)
        });
        let command_line =
            cx.new(|cx| InputState::new(window, cx).placeholder("コマンドラインオプション"));
        command_line.update(cx, |state, cx| {
            state.set_value(options.command_line.clone(), window, cx)
        });
        let mut subscriptions = Vec::new();
        for input in [&folder_input, &command_line] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |this: &mut Self, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::PressEnter { .. }) {
                        this.make(cx);
                    }
                },
            ));
        }
        Self {
            view,
            folder: folder_input,
            command_line,
            recursive: options.recursive,
            phase: Phase::Form,
            result: Some(result),
            cancel,
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn focus_first(&self, window: &mut Window, cx: &mut Context<Self>) {
        self.folder.update(cx, |state, cx| state.focus(window, cx));
    }

    pub(crate) fn is_running(&self) -> bool {
        matches!(self.phase, Phase::Running)
    }

    /// `IDC_BUTTON_FOLDER_UP`: the folder above the one in the box.
    pub(crate) fn folder_up(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = PathBuf::from(self.folder.read(cx).value().to_string());
        if let Some(parent) = current.parent().filter(|p| !p.as_os_str().is_empty()) {
            let text = parent.display().to_string();
            self.folder
                .update(cx, |state, cx| state.set_value(text, window, cx));
        }
    }

    /// `IDC_BUTTON_TAG_MAKE_REF`: chooses the folder in the dialog of the OS.
    fn browse(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let chosen = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(Ok(Some(paths))) = chosen.await
                && let Some(path) = paths.into_iter().next()
            {
                this.update_in(cx, |this, window, cx| {
                    let text = path.display().to_string();
                    this.folder
                        .update(cx, |state, cx| state.set_value(text, window, cx));
                })
                .ok();
            }
        })
        .detach();
    }

    pub(crate) fn set_recursive(&mut self, recursive: bool, cx: &mut Context<Self>) {
        self.recursive = recursive;
        cx.notify();
    }

    /// 作成: `GetData`, then the flow runs ctags while the panel shows the wait.
    pub(crate) fn make(&mut self, cx: &mut Context<Self>) {
        if self.is_running() {
            return;
        }
        let choice = MakeChoice {
            folder: PathBuf::from(self.folder.read(cx).value().to_string()),
            options: TagsMakeOptions {
                recursive: self.recursive,
                command_line: self.command_line.read(cx).value().to_string(),
            },
        };
        self.phase = Phase::Running;
        if let Some(result) = self.result.take() {
            let _ = result.try_send(Some(choice));
        }
        cx.notify();
    }

    /// キャンセル before ctags runs, 中断 while it runs.
    pub(crate) fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_running() {
            self.cancel.store(true, Ordering::Relaxed);
            return;
        }
        if let Some(result) = self.result.take() {
            let _ = result.try_send(None);
        }
        self.close(window, cx);
    }

    /// Closes the panel and gives the focus back to the text.
    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.support.tags_make = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if event.keystroke.key == "escape" {
            self.cancel(window, cx);
            cx.stop_propagation();
        }
    }
}

impl Focusable for TagsMakeDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for TagsMakeDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body =
            match self.phase {
                Phase::Running => {
                    v_flex()
                        .gap_2()
                        // STR_ERR_CEDITVIEW_CMD05
                        .child("タグファイルを作成中です。")
                        .child(h_flex().child(
                            Button::new("tags-stop").label("中断(X)").on_click(
                                cx.listener(|this, _, window, cx| this.cancel(window, cx)),
                            ),
                        ))
                        .into_any_element()
                }
                Phase::Form => v_flex()
                    .gap_2()
                    .child("タグ作成フォルダー")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&self.folder)))
                            .child(Button::new("tags-browse").label("...").on_click(
                                cx.listener(|this, _, window, cx| this.browse(window, cx)),
                            ))
                            .child(Button::new("tags-up").label("上(B)").on_click(
                                cx.listener(|this, _, window, cx| this.folder_up(window, cx)),
                            )),
                    )
                    .child(
                        Checkbox::new("tags-recursive")
                            .label("サブフォルダーも対象にする")
                            .checked(self.recursive)
                            .on_click(cx.listener(|this, value: &bool, _, cx| {
                                this.set_recursive(*value, cx)
                            })),
                    )
                    .child("コマンドラインオプション")
                    .child(Input::new(&self.command_line))
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("tags-make")
                                    .label("作成(O)")
                                    .on_click(cx.listener(|this, _, _, cx| this.make(cx))),
                            )
                            .child(Button::new("tags-cancel").label("キャンセル(X)").on_click(
                                cx.listener(|this, _, window, cx| this.cancel(window, cx)),
                            )),
                    )
                    .into_any_element(),
            };
        let panel = v_flex()
            .id("tags-make")
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .track_focus(&self.focus)
            .key_context("TagsMake")
            .on_key_down(cx.listener(Self::on_key_down))
            .w(px(480.))
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
                    .child("タグファイルの作成"),
            )
            .child(body);
        div()
            .id("tags-make-overlay")
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
    /// Opens the dialog of 「タグファイルの作成」 and focuses it.
    pub(crate) fn open_tags_make(
        &mut self,
        folder: &str,
        options: &TagsMakeOptions,
        result: async_channel::Sender<Option<MakeChoice>>,
        cancel: Arc<AtomicBool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity().downgrade();
        let dialog =
            cx.new(|cx| TagsMakeDialog::new(view, folder, options, result, cancel, window, cx));
        dialog.update(cx, |dialog, cx| dialog.focus_first(window, cx));
        self.support.tags_make = Some(dialog);
        cx.notify();
    }
}
