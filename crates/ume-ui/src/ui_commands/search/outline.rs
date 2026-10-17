//! `F_OUTLINE`: `Command_FUNCLIST` (`sakura_core/cmd/CViewCommander_Outline.cpp`) shows the outline pane for the kind of
//! outline of the document type. The argument is the action (0 show, 1 show and analyze again, 2 toggle);
//! the menu and the key give none, which is 0. The rules are in `crate::outline`.

use crate::outline::{Requested, Show};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct Outline;

impl UiCommand for Outline {
    const ID: &'static str = "outline";
    const SAKURA: &'static str = "F_OUTLINE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let show = match args.first() {
            Some(Arg::Int(n)) => Show::from_argument(*n),
            _ => Show::Normal,
        };
        view.run_outline(Requested::Default, show, window, cx)
    }
}

register_ui_command!(Outline);
