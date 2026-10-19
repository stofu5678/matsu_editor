//! `F_CHANGETYPE`: applies a document type to this document, for as long as the window shows it
//! (タイプ別設定一時適用). A macro-only function.
//!
//! Sakura: `Command_CHANGETYPE` (`sakura_core/cmd/CViewCommander_Settings.cpp`) takes the number
//! of the type plus one (`ChangeType(1)` is 基本); 0 keeps the type the document has and locks it.
//! A number that is no type does nothing. In the macro, the argument is `int nTypeIndex`
//! (`sakura_core/Funccode_x.hsrc`, `CMacro::HandleCommand`).
//!
//! UME Editor also takes the name of the type as a text argument (`ChangeType("C/C++")`), the
//! first type with that name.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::doc_types::with_user;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Changetype;

impl UiCommand for Changetype {
    const ID: &'static str = "changetype";
    const SAKURA: &'static str = "F_CHANGETYPE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let number = match args.first() {
            Some(Arg::Int(n)) => *n,
            // The name of the type: its number is the index plus one.
            Some(Arg::Text(name)) => {
                match with_user(cx, |user| user.document_types().index_of_name(name)) {
                    Some(i) => i64::try_from(i + 1).unwrap_or(-1),
                    None => return Ok(()),
                }
            }
            // A missing number is 0.
            _ => 0,
        };
        if view.change_type(number, cx) {
            // `CEditDoc::OnChangeType`: the macro of タイプ変更後.
            view.run_auto_macro(crate::macros::Auto::TypeChanged, None, window, cx);
        }
        Ok(())
    }
}

register_ui_command!(Changetype);
