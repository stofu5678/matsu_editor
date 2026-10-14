//! The macro host: runs a JavaScript (Node.js) or Python macro as a child process, and answers
//! the macro's calls to the editor's commands over JSON-RPC on the standard input and output
//! (docs/poc/json-rpc.md).
//!
//! Nothing here depends on GPUI, so the tests run the real interpreters against a plain text
//! model (`TextModel`).
//!
//! - `protocol`: the lines, the escaping and the messages.
//! - `api`: the basic API (one table). `codegen` (tests only) generates the JS and Python parts
//!   from it.
//! - `session`: the child process, its threads, the timeout and the cancel.
//! - `boot`: the boot libraries that the editor ships (`boot/`).
//! - `model`: a plain text model that implements the API.

pub mod api;
pub mod boot;
pub mod model;
pub mod protocol;
pub mod session;

#[cfg(test)]
mod codegen;
#[cfg(test)]
mod tests;

use std::path::Path;

use api::MacroDocument;
use model::TextModel;
use session::{EndReason, Finished, MacroConfig, Report, Stream};

/// How a finished macro reads in the output panel and the status line.
pub fn describe(finished: &Finished, timeout_hint: &str) -> String {
    match finished.reason {
        EndReason::TimedOut => format!("時間切れで中止しました ({timeout_hint})"),
        EndReason::Cancelled => "中断しました".to_string(),
        EndReason::Exited => match finished.exit_code {
            Some(0) => "正常に終了しました (終了コード 0)".to_string(),
            Some(code) => format!("エラーで終了しました (終了コード {code})"),
            None => format!("異常終了しました ({})", finished.status),
        },
    }
}

/// `--run-macro <macro> <input> <output>`: runs a macro on a text file without a window and
/// writes the resulting text. The macro's output goes to the standard output and the standard
/// error. Returns `None` when the arguments are not this mode.
pub fn run_cli(args: &[String]) -> Option<Result<String, String>> {
    let [_, mode, script, input, output] = args else {
        return None;
    };
    if mode != "--run-macro" {
        return None;
    }
    Some(run_cli_files(
        Path::new(script),
        Path::new(input),
        Path::new(output),
    ))
}

fn run_cli_files(script: &Path, input: &Path, output: &Path) -> Result<String, String> {
    let bytes = std::fs::read(input).map_err(|e| format!("{}: {e}", input.display()))?;
    let text = String::from_utf8_lossy(&bytes);
    let path = std::path::absolute(input)
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string())?;
    let mut doc = TextModel::new(&text, Some(path.clone()));
    let config = MacroConfig::for_script(script, Some(path)).map_err(|e| e.to_string())?;
    let timeout = config.timeout;
    let report = session::run_blocking(config, |message| {
        api::handle_message(&mut doc as &mut dyn MacroDocument, message)
    })
    .map_err(|e| e.to_string())?;
    print_report(&report);
    std::fs::write(output, doc.text()).map_err(|e| format!("{}: {e}", output.display()))?;
    let summary = describe(&report.finished, &format!("{} 秒", timeout.as_secs_f64()));
    match report.finished {
        Finished {
            reason: EndReason::Exited,
            exit_code: Some(0),
            ..
        } => Ok(summary),
        _ => Err(summary),
    }
}

fn print_report(report: &Report) {
    for (stream, text) in &report.output {
        match stream {
            Stream::Stdout => println!("{text}"),
            Stream::Stderr => eprintln!("{text}"),
        }
    }
}
