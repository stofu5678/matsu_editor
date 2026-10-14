//! Cursor movement commands, each with the variant that extends the selection (`*_sel`).

pub mod cursor_down;
pub mod cursor_down_sel;
pub mod cursor_left;
pub mod cursor_left_sel;
pub mod cursor_right;
pub mod cursor_right_sel;
pub mod cursor_up;
pub mod cursor_up_sel;
pub mod go_file_end;
pub mod go_file_end_sel;
pub mod go_file_top;
pub mod go_file_top_sel;
pub mod go_line_end;
pub mod go_line_end_sel;
pub mod go_line_top;
pub mod go_line_top_sel;
