//! `ume-script`, the script host that ships next to `ume`. The editor starts it for the macros
//! of the bundled languages, and it speaks the same JSON-RPC macro protocol as the external
//! interpreters (docs/poc/json-rpc.md): `ume-script --engine lua`. The script comes in the
//! `initialize` notification, so a path in any character set works.
//!
//! A script that fails ends the process with exit code 1 and its error on the standard error;
//! one that loops forever is killed by the editor, as any macro process is.

use std::cell::RefCell;
use std::io::{self, Write};
use std::process::ExitCode;
use std::rc::Rc;

use ume_script::engine::{self, ENGINES};
use ume_script::rpc::{Connection, Editor};

fn main() -> ExitCode {
    let Some(name) = engine_argument() else {
        eprintln!(
            "usage: ume-script --engine <{}>  (the editor starts this program)",
            ENGINES.join("|")
        );
        return ExitCode::from(2);
    };
    let Some(engine) = engine::by_name(&name) else {
        eprintln!(
            "ume-script: unknown engine \"{name}\" (one of: {})",
            ENGINES.join(", ")
        );
        return ExitCode::from(2);
    };

    let mut connection = Connection::new(io::stdin().lock(), io::stdout());
    let init = match connection.read_initialize() {
        Ok(init) => init,
        Err(message) => {
            eprintln!("ume-script: {message}");
            return ExitCode::from(2);
        }
    };
    let editor: Rc<RefCell<dyn Editor>> = Rc::new(RefCell::new(connection));
    let result = engine.run(&init, editor);
    let _ = io::stdout().flush();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// The value of `--engine`.
fn engine_argument() -> Option<String> {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next(), args.next()) {
        (Some("--engine"), Some(name), None) => Some(name),
        _ => None,
    }
}
