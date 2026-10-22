//! MATSU Editor: `matsu [--separate-process] [--] [path...]`. One process runs for each settings file:
//! a second launch hands its files to the running one and ends (ADR-0016). The program only puts the
//! parts together.

// A release build on Windows is a GUI program, so starting it does not open a console window
// next to the editor. A debug build keeps the console for the logs of development. (The attribute
// means nothing on other OSes.)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    matsu_ui::launch(std::env::args_os().skip(1).collect());
    ExitCode::SUCCESS
}
