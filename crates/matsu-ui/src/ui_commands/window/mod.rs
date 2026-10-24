//! Sakura's window group (ウィンドウ系): arranging the editor windows, going to the next one, the
//! window list, closing all or the others, and the output.
//!
//! The flows are in `crate::window_ops`. The functions of the tabs and groups (`F_SHOWTAB`,
//! `F_BIND_WINDOW`, `F_GROUPCLOSE`, ...) work on the tabs of a window (ADR-0015); their flows are in
//! `crate::tabs`. The split view (`F_SPLIT_*`) is in `ui_commands/split`.

pub mod bind_window;
pub mod cascade;
pub mod dlgwinlist;
pub mod groupclose;
pub mod maximize_h;
pub mod maximize_v;
pub mod minimize_all;
pub mod nextgroup;
pub mod nextwindow;
pub mod prevgroup;
pub mod preview_tab;
pub mod prevwindow;
pub mod redraw;
pub mod showtab;
pub mod tab_closeleft;
pub mod tab_closeother;
pub mod tab_closeright;
pub mod tab_jointnext;
pub mod tab_jointprev;
pub mod tab_moveleft;
pub mod tab_moveright;
pub mod tab_separate;
pub mod tile_h;
pub mod tile_v;
pub mod topmost;
pub mod win_closeall;
pub mod win_output;
pub mod winlist;
