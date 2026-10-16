//! Sakura's window group (ウィンドウ系): arranging the editor windows, going to the next one, the
//! window list, closing all or the others, and the output.
//!
//! The flows are in `crate::window_ops`. The functions of the tabs and groups (`F_BIND_WINDOW`,
//! `F_GROUPCLOSE`, ...) and of the split view (`F_SPLIT_*`) are not here: they wait for the
//! decision on tabs and for the split view.

pub mod cascade;
pub mod dlgwinlist;
pub mod maximize_h;
pub mod maximize_v;
pub mod minimize_all;
pub mod nextwindow;
pub mod prevwindow;
pub mod redraw;
pub mod tab_closeother;
pub mod tile_h;
pub mod tile_v;
pub mod topmost;
pub mod win_closeall;
pub mod win_output;
pub mod winlist;
