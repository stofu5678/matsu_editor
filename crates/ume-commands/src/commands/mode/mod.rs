//! Sakura's mode group (モード切り替え系): cancelling the modes and the line ending that Enter
//! inserts. (`F_BEGIN_SEL` is in the selection group, as Sakura has it.)

pub mod cancel_mode;
pub mod chgmod_eol;
pub mod chgmod_eol_cr;
pub mod chgmod_eol_crlf;
pub mod chgmod_eol_lf;
