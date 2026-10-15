//! UME Editor: `ume [path]` opens the window. The program only puts the parts together.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let path = std::env::args_os().nth(1).map(PathBuf::from);
    match ume_ui::run(path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("ume: {err}");
            ExitCode::FAILURE
        }
    }
}
