//! `F_SETFONTSIZE`: sets the font size, or moves it up or down the zoom table (フォントサイズ設定).
//! The macro function is `SetFontSize(size, shift, mode)`.
//!
//! Sakura: `Command_SETFONTSIZE` (`sakura_core/cmd/CViewCommander_Settings.cpp`). `size` is a size in
//! 1/10 point (brought into 1 to 72 points); with `size` 0, `shift` moves that many steps of the
//! zoom table (`crate::settings::zoom`) from the size of the font of the file. `mode` says what
//! changes: 0 the common font, 1 the font of the document type (which then has a font of its own),
//! 2 this window only (「一時適用フォント」, what `F_SETFONTSIZEUP` and `F_SETFONTSIZEDOWN` do). Only
//! TrueType fonts change; any other mode, or both numbers 0, does nothing.
//!
//! Modes 0 and 1 write the settings file and every window of the font takes the size, so they run
//! when the command has ended (a macro goes on before the settings change).

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::font_dialog::apply;
use crate::font_dialog::model::is_truetype;
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

/// The argument `at` as a number; a missing one is 0 (`Argument[n] != nullptr ? _wtoi : 0`).
fn number(args: &[Arg], at: usize) -> i32 {
    match args.get(at) {
        Some(Arg::Int(n)) => i32::try_from(*n).unwrap_or(if *n < 0 { i32::MIN } else { i32::MAX }),
        _ => 0,
    }
}

#[derive(Default)]
pub struct SetFontSize;

impl UiCommand for SetFontSize {
    const ID: &'static str = "set_font_size";
    const SAKURA: &'static str = "F_SETFONTSIZE";

    fn run(
        &self,
        view: &mut EditorView,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let (size, shift, mode) = (number(args, 0), number(args, 1), number(args, 2));
        match mode {
            2 => view.set_temporary_font_size(size, shift, cx),
            0 | 1 => {
                let truetype = if mode == 0 {
                    is_truetype(apply::common_font(cx).font.log_font.out_precision)
                } else {
                    view.view_settings().font_truetype
                };
                let Some((tenths, _)) = view.plan_font_size(size, shift, truetype, false) else {
                    return Ok(());
                };
                let index = view.view_settings().type_index;
                return view.defer_in_window(window, cx, move |_, app| {
                    if mode == 0 {
                        apply::set_common_size(app, tenths);
                    } else {
                        apply::set_type_size(app, index, tenths);
                    }
                });
            }
            _ => {}
        }
        Ok(())
    }
}

register_ui_command!(SetFontSize);
