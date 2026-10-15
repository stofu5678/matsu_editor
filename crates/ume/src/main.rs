//! UME Editor: `ume [path...]` opens one window per file. The program only puts the parts together.

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    ume_ui::run(paths);
    ExitCode::SUCCESS
}
