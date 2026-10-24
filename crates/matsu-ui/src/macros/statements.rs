//! The functions that a key macro (a `.mac` file) calls as statements.
//!
//! A `.mac` file may name a function that returns a value (`ChangeTabWidth(8);`): Sakura looks the
//! name up in both tables (`CSMacroMgr::GetFuncInfoByName`, which `CKeyMacroMgr::LoadKeyMacro`
//! uses), and `CMacro::HandleCommand` passes the call on to `CMacro::HandleFunction` for these
//! functions and drops the value: `F_CHGTABWIDTH`, `F_CHGWRAPCOLUMN`, `F_CHGINSSPACE`,
//! `F_SETDRAWSWITCH`, `F_MACROSLEEP` and the message boxes. Any other function falls to the
//! `default` case, which hands the function code to `CViewCommander::HandleCommand`, and that does
//! nothing for a function.
//!
//! Done here: the three that change the layout and the Tab key of this window, as the script
//! functions of the same names do (`matsu_plugin::functions`): `ChangeTabWidth`, `ChangeWrapColm`
//! (`ChangeWrapColumn`) and `ChangeInsertSpace`; `SetDrawSwitch`, which keeps the flag that
//! `GetDrawSwitch` reads; `Sleep`; and the message boxes (`MessageBox`, `ErrorMsg`, `WarnMsg`,
//! `InfoMsg`, `OkCancelBox`, `YesNoBox`).
//!
//! `Sleep` and the boxes make the macro wait (Sakura's `::Sleep` and `::MessageBox` block its
//! window): the next call runs when the time is over or the box is closed, through
//! [`crate::macros::track`], as for a dialog that a command opens. The answer of a box is not used
//! by a statement (`HandleCommand` drops the value).

use std::time::Duration;

use gpui::{Context, Window};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_plugin::functions::window::message_style;
use matsu_plugin::functions::{MAX_LINE_COLUMNS, MAX_TAB_WIDTH, MIN_LINE_COLUMNS};

use super::track;
use crate::view::EditorView;

/// The number of an argument that is a number; a missing one is 0 (`Argument[0] != nullptr ?
/// _wtoi( Argument[0] ) : 0` in `CMacro::HandleCommand`).
fn number(args: &[Arg]) -> Option<i64> {
    match args.first() {
        None => Some(0),
        Some(Arg::Int(n)) => Some(*n),
        Some(_) => None,
    }
}

impl EditorView {
    /// Runs `code` as the statement of a key macro if it is one of the functions above. Gives
    /// `None` for any other function, and `Some(false)` for a mistake in the arguments.
    pub(crate) fn run_function_statement(
        &mut self,
        code: u16,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        let name = FUNCTIONS.iter().find(|f| f.code == code)?.name;
        let method = match name {
            "F_MESSAGEBOX" => "MessageBox",
            "F_ERRORMSG" => "ErrorMsg",
            "F_WARNMSG" => "WarnMsg",
            "F_INFOMSG" => "InfoMsg",
            "F_OKCANCELBOX" => "OkCancelBox",
            "F_YESNOBOX" => "YesNoBox",
            _ => "",
        };
        if !method.is_empty() {
            return Some(self.message_box_statement(method, args, window, cx));
        }
        if !matches!(
            name,
            "F_CHGTABWIDTH"
                | "F_CHGWRAPCOLUMN"
                | "F_CHGINSSPACE"
                | "F_MACROSLEEP"
                | "F_SETDRAWSWITCH"
        ) {
            return None;
        }
        let Some(value) = number(args) else {
            self.notice = Some(format!("bad arguments: {name} takes a number"));
            return Some(false);
        };
        match name {
            // `HandleFunction`: a width of 1 to 64 that differs; the change is for this window
            // only (`m_bTabSpaceCurTemp`).
            "F_CHGTABWIDTH" => {
                if (1..=MAX_TAB_WIDTH).contains(&value) {
                    self.set_tab_width(u32::try_from(value).unwrap_or(4), cx);
                }
            }
            // 10 to 10240 columns; the method becomes 指定桁.
            "F_CHGWRAPCOLUMN" => {
                if (MIN_LINE_COLUMNS..=MAX_LINE_COLUMNS).contains(&value) {
                    self.set_wrap_columns(u32::try_from(value).unwrap_or(80), cx);
                }
            }
            // `CMacro::HandleFunction`: `SetDrawSwitchOfAllViews(iVal != 0)`, the low 16 bits. The
            // flag is the one that `GetDrawSwitch` reads; the text is painted as before.
            "F_SETDRAWSWITCH" => self.macros.host_state.draw_switch = value as i16 != 0,
            // `::Sleep( uintVal )`: a negative number does not convert to an unsigned one, and
            // `HandleFunction` returns false, which stops the macro.
            "F_MACROSLEEP" => {
                let Ok(millis) = u32::try_from(value) else {
                    self.notice = Some("Sleep: the time must not be negative".to_owned());
                    return Some(false);
                };
                self.sleep_statement(Duration::from_millis(u64::from(millis)), cx);
            }
            // 0 off, 1 on, anything else only reads; Sakura reads `iVal`, the low 16 bits.
            _ => match value as i16 {
                0 => self.prefs.settings.ins_space = false,
                1 => self.prefs.settings.ins_space = true,
                _ => {}
            },
        }
        Some(true)
    }

    /// `Sleep(n)` as a statement: the macro goes on after `delay`. Without a macro that waits
    /// (a function run by hand) there is nothing to hold, and nothing happens.
    fn sleep_statement(&mut self, delay: Duration, cx: &mut Context<Self>) {
        let Some(held) = track(cx) else {
            return;
        };
        let timer = cx.background_executor().timer(delay);
        cx.spawn(async move |_, _| {
            timer.await;
            drop(held);
        })
        .detach();
    }

    /// `MessageBox`, `ErrorMsg`, ... as a statement (`CMacro::HandleCommand`): shows the box and
    /// holds the macro until it is closed. Without the text nothing happens. Returns whether the
    /// call went well.
    fn message_box_statement(
        &mut self,
        method: &str,
        args: &[Arg],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(Arg::Text(text)) = args.first() else {
            // `Argument[0] == nullptr`: the call is skipped.
            return true;
        };
        // The type is the second argument of `MessageBox`, a missing one is 0 (`MB_OK`).
        let kind = Arg::first_int(args.get(1..).unwrap_or_default());
        let Some((buttons, icon)) = message_style(method, kind) else {
            return false;
        };
        let answer = self.message_prompt(text, buttons, icon, window, cx);
        super::keep_box(answer, window, cx);
        true
    }
}
