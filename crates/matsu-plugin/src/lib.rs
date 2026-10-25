//! The macro host: runs a JavaScript (Node.js), Python, Ruby or PHP macro as a child process, and answers
//! the macro's calls to the editor over JSON-RPC on the standard input and output
//! (docs/poc/json-rpc.md, ADR-0007).
//!
//! Nothing here depends on a UI toolkit. The editor passes a `MacroConfig`, receives
//! `HostEvent`s from the reader threads, and runs each request on a `MacroDocument` that it
//! implements on top of its commands. The tests run the real interpreters against a plain text
//! model (`TextModel`).
//!
//! - `protocol`: the lines, the escaping and the messages.
//! - `bridge`: runs a macro's calls by Sakura's macro names on the commands (`matsu-commands`).
//! - `catalog`: every callable name (the basic API and every row of Sakura's macro tables) in one
//!   list, which the code generator and the bundled engines read.
//! - `functions`: the macro functions that need the editor itself (the layout, the clipboard, the
//!   window), and `host`: what the window gives them.
//! - `api`: the basic API (one table). `codegen` (tests only) generates the JS, Python, Ruby and PHP parts
//!   from it.
//! - `session`: the child process, its threads, the timeout and the cancel.
//! - `boot`: the boot libraries that the editor ships (`src/boot/`) and their private folder.
//! - `model`: a plain text model that implements the API.

pub mod api;
pub mod applescript;
pub mod boot;
pub mod bridge;
pub mod catalog;
pub mod functions;
pub mod host;
pub mod model;
pub mod protocol;
pub mod session;

#[cfg(test)]
mod codegen;
#[cfg(test)]
mod tests;

/// The JSON value of a call's argument and result, for a host that answers a call itself.
pub use serde_json::Value;

use session::{EndReason, Finished};

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

#[cfg(test)]
mod describe_tests {
    use super::*;

    fn finished(reason: EndReason, exit_code: Option<i32>) -> Finished {
        Finished {
            reason,
            exit_code,
            status: "signal: 9".into(),
        }
    }

    #[test]
    fn describes_every_ending() {
        let hint = "30 秒";
        assert!(describe(&finished(EndReason::TimedOut, None), hint).contains(hint));
        assert_eq!(
            describe(&finished(EndReason::Cancelled, None), hint),
            "中断しました"
        );
        assert!(describe(&finished(EndReason::Exited, Some(0)), hint).contains("正常"));
        assert!(describe(&finished(EndReason::Exited, Some(2)), hint).contains("終了コード 2"));
        assert!(describe(&finished(EndReason::Exited, None), hint).contains("signal: 9"));
    }
}
