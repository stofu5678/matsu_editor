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
//! (`ChangeWrapColumn`) and `ChangeInsertSpace`. Not done: `Sleep`, `SetDrawSwitch` and the
//! message boxes (`MessageBox`, `ErrorMsg`, `WarnMsg`, `InfoMsg`, `OkCancelBox`, `YesNoBox`);
//! they stop the macro as a function that is not implemented yet.

use gpui::Context;
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_plugin::functions::{MAX_LINE_COLUMNS, MAX_TAB_WIDTH, MIN_LINE_COLUMNS};

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
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        let name = FUNCTIONS.iter().find(|f| f.code == code)?.name;
        if !matches!(name, "F_CHGTABWIDTH" | "F_CHGWRAPCOLUMN" | "F_CHGINSSPACE") {
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
            // 0 off, 1 on, anything else only reads; Sakura reads `iVal`, the low 16 bits.
            _ => match value as i16 {
                0 => self.prefs.settings.ins_space = false,
                1 => self.prefs.settings.ins_space = true,
                _ => {}
            },
        }
        Some(true)
    }
}
