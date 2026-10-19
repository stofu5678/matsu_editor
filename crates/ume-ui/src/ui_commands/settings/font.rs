//! `F_FONT`: shows the font dialog (フォント設定...) and makes the font the common font of every window.
//!
//! Sakura: `Command_FONT` (`sakura_core/cmd/CViewCommander_Settings.cpp`) asks for a font with
//! `MySelectFont` (any font, not only fixed-pitch ones: `USE_UNFIXED_FONT`), stores it as the common
//! font (`m_sView.m_lf`, `m_nPointSize`), sets `m_bFontIs_FIXED_PITCH` to true whatever the font is and
//! tells every window (`PM_CHANGESETTING_FONT`). A window whose type has a font of its own keeps it.
//! The dialog is `crate::font_dialog`; the macro function is `SelectFont`.

use gpui::{Context, Window};
use ume_commands::command::{Arg, Result};

use crate::font_dialog::{self, apply};
use crate::ui_command::{UiCommand, register_ui_command};
use crate::view::EditorView;

#[derive(Default)]
pub struct Font;

impl UiCommand for Font {
    const ID: &'static str = "font";
    const SAKURA: &'static str = "F_FONT";

    fn run(
        &self,
        view: &mut EditorView,
        _: &[Arg],
        window: &mut Window,
        cx: &mut Context<EditorView>,
    ) -> Result {
        let owner = font_dialog::owner_of(window);
        // A macro waits until the dialog is closed (the guard goes with `done`, answered or not).
        let tracked = crate::macros::track(cx);
        // Every window changes, so this view must not be borrowed meanwhile.
        view.defer_in_window(window, cx, move |_, app| {
            let current = apply::common_font(app).font;
            font_dialog::choose(
                app,
                owner,
                current,
                Box::new(move |app, font| {
                    let _tracked = tracked;
                    apply::set_common_font(app, &font);
                }),
            );
        })
    }
}

register_ui_command!(Font);
