//! Repository tasks, run as `cargo xtask <task>`.
//!
//! Tasks:
//! - `sakura-tables --sakura <path>`: regenerate `crates/matsu-commands/src/sakura/*.rs` from a
//!   clone of <https://github.com/sakura-editor/sakura> (`macro_functions.rs` comes from
//!   `macro_table.rs`).
//!
//! To check that the committed tables are up to date, run the test with the path of the clone:
//! `SAKURA_DIR=<path> cargo test -p xtask`. Without `SAKURA_DIR` that test is skipped.

mod macro_table;
mod matsu_extras;
mod parse;
mod render;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fs};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("sakura-tables") => {
            let sakura = match args[1..] {
                [ref flag, ref path] if flag == "--sakura" => PathBuf::from(path),
                _ => {
                    return Err(
                        "usage: cargo xtask sakura-tables --sakura <path to a Sakura clone>".into(),
                    );
                }
            };
            let root = repository_root();
            for (path, content) in generate(&sakura)? {
                let target = root.join(&path);
                fs::write(&target, content).map_err(|e| format!("{}: {e}", target.display()))?;
                println!("wrote {path}");
            }
            Ok(())
        }
        _ => Err("tasks: sakura-tables".into()),
    }
}

/// The repository root: the parent of this crate's directory.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent")
        .to_path_buf()
}

/// Builds the content of every generated file: `(path relative to the repository root, text)`.
fn generate(sakura: &Path) -> Result<Vec<(String, String)>, String> {
    let mut tables = parse::read_tables(sakura)?;
    matsu_extras::apply(&mut tables)?;
    let commit = sakura_commit(sakura)?;
    let mut files = render::render(&tables, &commit);
    files.push(macro_table::render(sakura, &tables, &commit)?);
    Ok(files)
}

fn sakura_commit(sakura: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(sakura)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err(format!("{} is not a git clone of Sakura", sakura.display()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regenerates the tables in memory and compares them with the committed files.
    /// Runs only when `SAKURA_DIR` points to a Sakura clone at the commit named in the files.
    #[test]
    fn committed_tables_match_sakura() {
        let Some(sakura) = env::var_os("SAKURA_DIR") else {
            eprintln!("SAKURA_DIR is not set; skipping");
            return;
        };
        let root = repository_root();
        for (path, expected) in generate(Path::new(&sakura)).expect("generate") {
            let committed = fs::read_to_string(root.join(&path)).expect("read committed file");
            // Git may check out the files with CRLF on Windows.
            assert_eq!(
                committed.replace("\r\n", "\n"),
                expected,
                "{path} is out of date"
            );
        }
    }
}
