//! `F_OPEN_HfromtoC`: opens the C/C++ header or source of the same name.
//!
//! Sakura: `Command_OPEN_HfromtoC` (`sakura_core/cmd/CViewCommander_File.cpp`) and
//! `CEditView::OPEN_ExtFromtoExt` (`sakura_core/view/CEditView_Command.cpp`); the lists of
//! extensions and their order are in `file_ops::header_source`. The file opens in a new window
//! with the encoding of this document; if it is open already its window comes to the front. A
//! document without a file, one that is not C/C++, or a pair that does not exist beeps there and
//! says so on the status bar here.
//!
//! Not done: Sakura also pushes the position to the tag jump stack, so that `F_TAGJUMPBACK` can
//! return; this program has no tag jump yet.

use crate::file_ops::header_source::counterpart;
use crate::file_ops::load::open_paths;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

#[derive(Default)]
pub struct OpenHeaderSource;

impl UiCommand for OpenHeaderSource {
    const ID: &'static str = "open_header_source";
    const SAKURA: &'static str = "F_OPEN_HfromtoC";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let shared = view.document().read(cx);
        let encoding = shared.file().encoding;
        let target = shared
            .path()
            .and_then(|path| counterpart(path, std::path::Path::is_file));
        let Some(target) = target else {
            view.notice = Some("対応するヘッダー(ソース)ファイルがない".to_owned());
            cx.notify();
            return Ok(());
        };
        start(window, cx, async move |view, cx| {
            open_paths(&view, cx, vec![target], Some(encoding)).await;
        });
        Ok(())
    }
}

register_ui_command!(OpenHeaderSource);
