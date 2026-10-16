//! Sakura's file group (ファイル操作系): open, save, close, reopen, exit and print.
//!
//! The flows of open, save and close are in `crate::file_ops`; the commands read their arguments
//! and start them.

pub mod exit_all;
pub mod exit_all_editors;
pub mod file_close;
pub mod file_close_open;
pub mod file_new;
pub mod file_new_new_window;
pub mod file_open;
pub mod file_reopen;
pub mod file_reopen_cesu8;
pub mod file_reopen_euc;
pub mod file_reopen_jis;
pub mod file_reopen_latin1;
pub mod file_reopen_sjis;
pub mod file_reopen_unicode;
pub mod file_reopen_unicodebe;
pub mod file_reopen_utf7;
pub mod file_reopen_utf8;
pub mod file_save;
pub mod file_save_as;
pub mod file_save_as_dialog;
pub mod file_save_close;
pub mod file_save_quiet;
pub mod print;
pub mod print_pagesetup;
pub mod print_preview;
pub mod win_close;
