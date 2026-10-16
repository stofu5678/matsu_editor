//! UME Editor: `ume [path...]` opens one window per file. The program only puts the parts together.

// A release build on Windows is a GUI program, so starting it does not open a console window
// next to the editor. A debug build keeps the console for the logs of development. (The attribute
// means nothing on other OSes.)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    ume_ui::run(paths);
    ExitCode::SUCCESS
}
