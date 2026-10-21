//! `F_CREATEKEYBINDLIST` (キー割り当て一覧をコピー): copies the list of the key assignments as text.
//!
//! Sakura: `CViewCommander::Command_CREATEKEYBINDLIST` (`sakura_core/cmd/CViewCommander_Clipboard.cpp`)
//! with `CKeyBind::CreateKeyBindList`; the text is made by
//! [`ume_commands::keymap::create_key_bind_list`]. The table is the one of the application
//! (Sakura's `m_pKeyNameArr` of the common settings), the names of the functions are the ones the
//! settings page shows ([`crate::func_name`]).

use gpui::{Context, Window};
use ume_commands::clipboard::{ClipData, ClipKind, Clipboard as _};
use ume_commands::command::{Arg, Result};
use ume_commands::keymap::create_key_bind_list;

use crate::app::with_key_map;
use crate::clipboard::GpuiClipboard;
use crate::func_name::{FuncNames, UNKNOWN_NAME};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct CreateKeyBindList;

impl UiCommand for CreateKeyBindList {
    const ID: &'static str = "createkeybindlist";
    const SAKURA: &'static str = "F_CREATEKEYBINDLIST";

    fn run(
        &self,
        _: &mut EditorView,
        _: &[Arg],
        _: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let rows = with_key_map(cx, |map| map.rows());
        let names = FuncNames::new();
        let text = create_key_bind_list(&rows, &|code| {
            // `Funccode2Name` fails for a function that has no name
            Some(names.name(cx, code)).filter(|name| name != UNKNOWN_NAME)
        });
        GpuiClipboard(cx).set(ClipData::new(text, ClipKind::Normal));
        Ok(())
    }
}

register_ui_command!(CreateKeyBindList);
