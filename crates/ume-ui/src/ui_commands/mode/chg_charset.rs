//! `F_CHG_CHARSET`: sets the encoding and the BOM that a save of this document writes (文字コード
//! セット指定). The text is not converted, and the document is not modified.
//!
//! Sakura: `CViewCommander::Command_CHG_CHARSET` (`sakura_core/cmd/CViewCommander_ModeChange.cpp`),
//! the dialog `CDlgSetCharSet` (`sakura_core/dlg/CDlgSetCharSet.cpp`, `IDD_SETCHARSET` in
//! `sakura_rc.rc`), and the macro row of `sakura_core/macro/CMacro.cpp` (`ChgCharSet(code, bom)`):
//!
//! * Without an encoding (no argument, an empty one, `CODE_NONE` (-1) or `CODE_AUTODETECT` (99)) the
//!   dialog asks, starting from the encoding and the BOM of the document. Cancel changes nothing.
//! * With one, it is set at once. The BOM is the second argument (non-zero), and only an encoding
//!   that can have one keeps it (`CCodeTypeName::UseBom`).
//!
//! The dialog is the one of the Save As without its line ending (`file_ops::save_as_dialog`). The
//! status bar's encoding part runs this on a double click.

use gpui::{Context, Window};
use ume_commands::command::{Arg, CommandError, Result};
use ume_core::encoding::Encoding;

use crate::file_ops::save_as_dialog::ask_charset;
use crate::file_ops::start;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct ChgCharset;

/// `CODE_NONE`: no encoding given.
const CODE_NONE: i64 = -1;

/// The encoding in the first argument, or `None` for "ask in the dialog".
fn encoding_to_set(args: &[Arg]) -> std::result::Result<Option<Encoding>, CommandError> {
    match args.first() {
        None => Ok(None),
        Some(Arg::Text(text)) if text.is_empty() => Ok(None),
        Some(Arg::Int(n))
            if *n == CODE_NONE || *n == i64::from(ume_core::encoding::AUTODETECT_CODE) =>
        {
            Ok(None)
        }
        Some(Arg::Int(n)) => i32::try_from(*n)
            .ok()
            .and_then(Encoding::from_sakura_code)
            .map(Some)
            .ok_or_else(|| CommandError::BadArgs(format!("{n} is not an encoding number"))),
        Some(other) => Err(CommandError::BadArgs(format!(
            "the encoding must be a number, not {other:?}"
        ))),
    }
}

/// The BOM of the second argument: non-zero is on, a missing one is off (`_wtoi`).
fn bom_to_set(args: &[Arg]) -> bool {
    matches!(args.get(1), Some(Arg::Int(n)) if *n != 0)
}

/// `SetCodeSetChg( eCharSet, CCodeTypeName( eCharSet ).UseBom() & bBom )`.
fn apply(view: &mut EditorView, encoding: Encoding, bom: bool, cx: &mut Context<EditorView>) {
    view.document().update(cx, |shared, cx| {
        shared.set_format(encoding, bom && encoding.can_have_bom());
        cx.notify();
    });
    cx.notify();
}

impl UiCommand for ChgCharset {
    const ID: &'static str = "chg_charset";
    const SAKURA: &'static str = "F_CHG_CHARSET";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        if let Some(encoding) = encoding_to_set(args)? {
            apply(view, encoding, bom_to_set(args), cx);
            return Ok(());
        }
        let shared = view.document().read(cx).file();
        let current = (shared.encoding, shared.bom);
        start(window, cx, async move |view, cx| {
            if let Some((encoding, bom)) = ask_charset(&view, cx, current).await {
                view.update(cx, |view, cx| apply(view, encoding, bom, cx))
                    .ok();
            }
        });
        Ok(())
    }
}

register_ui_command!(ChgCharset);

#[cfg(test)]
mod tests;
