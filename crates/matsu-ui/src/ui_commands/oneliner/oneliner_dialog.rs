//! `F_ONELINER_DIALOG`: 「ワンライナーを入力して実行」 (ADR-0029). See [`crate::oneliner::dialog`].

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::file_ops::start;
use crate::oneliner::dialog::run_dialog;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct OnelinerDialog;

impl UiCommand for OnelinerDialog {
    const ID: &'static str = "oneliner_dialog";
    const SAKURA: &'static str = "F_ONELINER_DIALOG";

    fn run(
        &self,
        _view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        start(window, cx, async move |view, cx| {
            run_dialog(&view, cx).await;
        });
        Ok(())
    }
}

register_ui_command!(OnelinerDialog);
