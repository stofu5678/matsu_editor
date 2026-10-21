//! E2E smoke tests: start the built `ume` program, let a startup macro (`-M=`) type text, save the
//! file and quit, and check the bytes of the saved file.
//!
//! Nothing else in the test suite starts the executable: the other tests run inside GPUI's test
//! platform. These tests find what breaks only in a real process: the window, the GPU, the
//! single-instance channel, and the search for `ume-script` next to the editor.
//!
//! The tests are `#[ignore]` because they need a display (a virtual one is enough, see
//! `.github/workflows/e2e.yml`) and `ume-script` built in the same profile as `ume`:
//!
//! ```sh
//! cargo build --release --locked -p ume -p ume-script
//! cargo test --release --locked -p ume --test e2e_smoke -- --ignored --test-threads=1
//! ```
//!
//! The editor runs with `--separate-process` (it neither joins nor is joined by another UME
//! Editor), and with `HOME`, `USERPROFILE`, `APPDATA` and `XDG_CONFIG_HOME` in a temporary folder,
//! so it never reads or writes the settings of the user who runs the test.

mod common;

use std::fs;
use std::io::Read;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use common::{Scratch, macro_path, spawn_editor, stage_binaries};

/// How long the editor may run. The first start on a CI runner can be slow (software rendering).
const TIME_LIMIT: Duration = Duration::from_secs(120);

/// Runs `ume` with `args` in a scratch environment until it ends. Returns its output (stdout and
/// stderr). Panics, with the output, if it does not end in time or does not end with success.
fn run_editor(home: &Path, args: &[&str]) -> String {
    let exe = stage_binaries(home);
    let log = home.join("output.log");
    let mut child = spawn_editor(&exe, home, args, &log);

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if started.elapsed() > TIME_LIMIT {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(100));
    };

    let mut bytes = Vec::new();
    if let Ok(mut file) = fs::File::open(&log) {
        let _ = file.read_to_end(&mut bytes);
    }
    let output = String::from_utf8_lossy(&bytes).into_owned();
    match status {
        None => panic!("ume did not end within {TIME_LIMIT:?}. Output:\n{output}"),
        Some(status) if !status.success() => {
            panic!("ume ended with {status}. Output:\n{output}")
        }
        Some(_) => output,
    }
}

/// Runs `macro_file` as the startup macro of a new empty document in the encoding of `-CODE=`
/// `code`, and returns the bytes of `saved` afterwards.
fn run_macro(scratch: &Scratch, macro_file: &str, code: &str, saved: &Path) -> Vec<u8> {
    let macro_arg = format!("-M={}", scratch.0.join(macro_file).display());
    let code_arg = format!("-CODE={code}");
    let output = run_editor(&scratch.0, &["--separate-process", &macro_arg, &code_arg]);
    fs::read(saved).unwrap_or_else(|e| {
        panic!(
            "the macro did not save {}: {e}. Output:\n{output}",
            saved.display()
        )
    })
}

/// A key macro (`.mac`, run by the editor itself) writes Japanese text in Shift_JIS with LF.
#[test]
#[ignore = "starts the editor: needs a display and ume-script (see the module comment)"]
fn key_macro_saves_shift_jis_with_lf() {
    let scratch = Scratch::new("mac");
    let saved = scratch.0.join("sjis.txt");
    // The characters are typed with `Char(code point)`, as a recorded key macro has them, and
    // `Char(13)` is Enter. (`InsText` is not available in a key macro yet: the player stops at
    // it with "F_INSTEXT_W はまだ使えない".)
    // `FileSaveAs(path, encoding, line ending)`: 2 is LF (S_FileSaveAs).
    let source = format!(
        "Char(26085);\nChar(26412);\nChar(35486);\nChar(13);\nChar(97);\nChar(98);\nChar(99);\nChar(13);\nFileSaveAs('{}', 0, 2);\nExitAll();\n",
        macro_path(&saved)
    );
    fs::write(scratch.0.join("smoke.mac"), source).unwrap();

    let bytes = run_macro(&scratch, "smoke.mac", "0", &saved);
    assert_eq!(bytes, b"\x93\xfa\x96\x7b\x8c\xea\nabc\n");
}

/// A script of the bundled engine (`.js`, QuickJS in `ume-script`) writes UTF-8 with CRLF.
#[test]
#[ignore = "starts the editor: needs a display and ume-script (see the module comment)"]
fn bundled_quickjs_macro_saves_utf8_with_crlf() {
    let scratch = Scratch::new("js");
    let saved = scratch.0.join("utf8.txt");
    // 1 is CRLF (S_FileSaveAs).
    let source = format!(
        "Editor.InsText('日本語\\nabc\\n');\nEditor.FileSaveAs('{}', 0, 1);\nEditor.ExitAll();\n",
        macro_path(&saved)
    );
    fs::write(scratch.0.join("smoke.js"), source).unwrap();

    let bytes = run_macro(&scratch, "smoke.js", "4", &saved);
    // `-CODE=4` is UTF-8; whether it has a BOM is the default of the editor, so only the text is
    // compared.
    let body = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(&bytes);
    assert_eq!(body, "日本語\r\nabc\r\n".as_bytes());
}
