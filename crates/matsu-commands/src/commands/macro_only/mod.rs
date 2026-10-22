//! The functions that only macros call (`F_MOVECURSOR`, `F_SET_QUOTESTRING`, `F_COMMITUNDOBUFFER`
//! and the others). Sakura has no menu item or key for them; `CMacro::HandleCommand` in
//! `sakura_core/macro/CMacro.cpp` reads their arguments and calls the command.
//!
//! The ones that need the window (the output panel, the status bar, the files and their
//! encodings, scrolling, the window itself) are UI commands in `matsu-ui`
//! (`ui_commands/macro_only`).

pub mod addrefundobuffer;
pub mod appendundobuffercursor;
pub mod clipboardempty;
pub mod commitundobuffer;
pub mod movecursor;
pub mod movecursorlayout;
pub mod set_quotestring;
pub mod setundobuffer;
