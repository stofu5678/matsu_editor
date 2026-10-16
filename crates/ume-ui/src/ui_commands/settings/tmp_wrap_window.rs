//! `F_TMPWRAPWINDOW`: Wrap at the right edge of the window (`WRAP_WINDOW_WIDTH`), for this view only.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::settings::WrapMethod;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct TmpWrapWindow;

impl UiCommand for TmpWrapWindow {
    const ID: &'static str = "tmp_wrap_window";
    const SAKURA: &'static str = "F_TMPWRAPWINDOW";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        view.set_wrap_method(WrapMethod::WindowWidth, cx);
        Ok(())
    }
}

register_ui_command!(TmpWrapWindow);
