//! `F_BOOKMARK_VIEW`: `Command_FUNCLIST( .., OUTLINE_BOOKMARK )` shows the bookmarks in the outline pane
//! (`sakura_core/cmd/CViewCommander_Outline.cpp`, the list is `MakeFuncList_BookMark`).

use crate::outline::{Requested, Show};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct BookmarkView;

impl UiCommand for BookmarkView {
    const ID: &'static str = "bookmark_view";
    const SAKURA: &'static str = "F_BOOKMARK_VIEW";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        // `lparam1` is the action as for F_OUTLINE; 0 when a key or the menu runs it
        let show = match args.first() {
            Some(Arg::Int(n)) => Show::from_argument(*n),
            _ => Show::Normal,
        };
        view.run_outline(Requested::Bookmark, show, window, cx)
    }
}

register_ui_command!(BookmarkView);
