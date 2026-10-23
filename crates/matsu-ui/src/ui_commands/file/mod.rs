//! Sakura's file group (ファイル操作系): open, save, close, reopen, exit, print, view mode, and handing the file to the OS.
//!
//! The flows of open, save and close are in `crate::file_ops`; the commands read their arguments
//! and start them.

pub mod base64_decode;
pub mod browse;
pub mod exit_all;
pub mod exit_all_editors;
pub mod file_close;
pub mod file_close_open;
pub mod file_new;
pub mod file_new_new_window;
pub mod file_open;
pub mod file_open_dropdown;
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
pub mod file_save_all;
pub mod file_save_as;
pub mod file_save_as_dialog;
pub mod file_save_close;
pub mod file_save_quiet;
pub mod open_command_prompt;
pub mod open_command_prompt_as_admin;
pub mod open_folder_in_explorer;
pub mod open_header_source;
pub mod open_powershell;
pub mod open_powershell_as_admin;
pub mod print;
pub mod print_pagesetup;
pub mod print_preview;
pub mod property_file;
pub mod uudecode;
pub mod view_mode;
pub mod win_close;
