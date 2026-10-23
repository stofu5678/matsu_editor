//! `F_RECONVERT` (再変換): asks the IME to convert the text at the caret again.
//!
//! Sakura: `CViewCommander::Command_Reconvert` (`sakura_core/cmd/CViewCommander_Edit_advanced.cpp`),
//! with `SetReconvertStruct` and `SetSelectionFromReonvert` (`sakura_core/view/CEditView_Ime.cpp`);
//! the steps are in [`crate::reconvert`]. The OS part is `matsu_platform::reconvert`.
//!
//! * Windows: as in Sakura, through IMM32. The IME widens the range, the range is selected, and
//!   the IME starts the conversion; the result replaces the selection. Not run by CI (no IME);
//!   the user checks it.
//! * macOS and Linux: a program cannot start a reconversion there (the input method asks the
//!   text view for its text itself, when the user presses its key). The command says so on the
//!   status bar (ADR-0018, type B: the nearest form; Sakura does nothing when the IME has no
//!   support, but a menu item that does nothing without a word is worse).
//!
//! There is no macro name for it in Sakura (`CSMacroMgr.cpp`), so a script cannot call it.

use std::rc::Rc;

use gpui::{Context, Window};
use matsu_commands::command::{Arg, Result};

use crate::reconvert::OsIme;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;
use crate::window_ops::raw_handle;

#[derive(Default)]
pub struct Reconvert;

impl UiCommand for Reconvert {
    const ID: &'static str = "reconvert";
    const SAKURA: &'static str = "F_RECONVERT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        // a window without an OS handle (the headless tests) has no IME to ask
        if let Some(handle) = raw_handle(window) {
            view.start_reconvert(Rc::new(OsIme(handle)), window, cx);
        }
        Ok(())
    }
}

register_ui_command!(Reconvert);
