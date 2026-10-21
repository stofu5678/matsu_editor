//! E2E screenshots: start the built `ume` program with a scenario, wait until its window is
//! drawn, and save a picture of the screen as `<platform>-<scenario>.png`. The workflow
//! (`.github/workflows/e2e.yml`) uploads the pictures as an artifact of the run, and
//! `scripts/e2e-screens.sh` downloads them, so that a person or an AI can look at the
//! screens of every platform without a device of each (see `docs/e2e-screens.md`).
//!
//! A scenario is a document and a key macro (`.mac`) that the program runs at the start (`-M=`):
//! the macro opens a window with a function that has a macro name (`OptionCommon`, `PrintPreview`,
//! ...). The pages of the settings dialogs after the first one need a click: they are taken on
//! Windows by `windows_uia.rs`, which drives the window with UI Automation.
//!
//! The pictures are for looking at, not for comparing: the text is drawn differently by run and
//! by system. The only checks are cheap and stable: the screen has a size a window fits in, and
//! it differs from the screen before the program started.
//!
//! The tests are `#[ignore]` because they need a display (a virtual one on Linux) and
//! `ume-script` built in the same profile as `ume`:
//!
//! ```sh
//! cargo build --release --locked -p ume -p ume-script
//! UME_E2E_SCREENS_DIR=/some/folder \
//!   cargo test --release --locked -p ume --test e2e_screens -- --ignored --test-threads=1
//! ```
//!
//! `UME_E2E_PLATFORM` is the prefix of the file names (the workflow sets the name of the job).
//! The program runs with a temporary profile like in `e2e_smoke.rs`.

mod common;

use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use common::{
    Scratch, capture_screen, check_screenshot, macro_path, screenshot_path, soft_screens,
    spawn_editor, stage_binaries,
};

/// How long to wait after the start before the first picture. The first start on a CI runner is
/// slow (software rendering, a cold disk).
const SETTLE: Duration = Duration::from_secs(12);

/// How long the picture may stay the same as the empty screen before the test fails (a short time
/// when the screen cannot be trusted, see `common::soft_screens`).
fn draw_limit() -> Duration {
    Duration::from_secs(if soft_screens() { 30 } else { 180 })
}

/// A document with Japanese text of several lines, for the main window behind the dialogs.
const SAMPLE: &str = "吾輩は猫である。名前はまだ無い。\r\n\
どこで生れたかとんと見当がつかぬ。\r\n\
何でも薄暗いじめじめした所でニャーニャー泣いていた事だけは記憶している。\r\n\
\r\n\
UME Editor: the quick brown fox jumps over the lazy dog.\r\n\
\t行頭にタブのある行\r\n\
全角スペース　と半角スペース と末尾の空白  \r\n\
ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789\r\n";

/// Starts the program with `macro_source` as its startup macro (none: no macro) on the sample
/// document, takes a picture when the window is drawn and ends the program.
fn take(name: &str, macro_source: Option<&str>) {
    let scratch = Scratch::new(name);
    let sample = scratch.0.join("sample.txt");
    // UTF-8 with a BOM, so that the encoding is not guessed.
    let mut bytes = b"\xef\xbb\xbf".to_vec();
    bytes.extend_from_slice(SAMPLE.as_bytes());
    fs::write(&sample, bytes).unwrap();

    let macro_arg = macro_source.map(|source| {
        let file = scratch.0.join("scenario.mac");
        fs::write(&file, source).unwrap();
        format!("-M={}", macro_path(&file))
    });
    let mut args = vec!["--separate-process"];
    if let Some(arg) = &macro_arg {
        args.push(arg);
    }
    let sample_arg = sample.to_str().unwrap().to_owned();
    args.push(&sample_arg);

    // The window of the test before can still be on the screen a moment after its process ended
    // (a slow runner): a picture taken now would be the baseline, and the new window would look
    // like no change.
    thread::sleep(Duration::from_secs(5));
    let target = screenshot_path(name);
    let empty = target.with_extension("empty.png");
    let empty_result = capture_screen(&empty);

    let exe = stage_binaries(&scratch.0);
    let log = scratch.0.join("output.log");
    let mut child = spawn_editor(&exe, &scratch.0, &args, &log);

    let started = Instant::now();
    thread::sleep(SETTLE);
    let result = loop {
        // The program ending by itself is a failure on every platform (a crash, a macro error).
        if let Ok(Some(status)) = child.try_wait() {
            let output = fs::read_to_string(&log).unwrap_or_default();
            panic!("{name}: ume ended early with {status}. Output:\n{output}");
        }
        let outcome = empty_result
            .clone()
            .and_then(|()| capture_screen(&target))
            .and_then(|()| check_screenshot(&target, &empty));
        match outcome {
            Ok(()) => break Ok(()),
            Err(error) if started.elapsed() > draw_limit() => break Err(error),
            Err(_) => thread::sleep(Duration::from_secs(5)),
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    let _ = fs::remove_file(&empty);

    if let Err(error) = result {
        // GitHub's macOS runners give `screencapture` the Screen Recording permission (checked on
        // macos-15, docs/e2e-screens.md), so a missing window fails there like on the others.
        if soft_screens() {
            eprintln!("WARNING: {name}: {error}");
            return;
        }
        let output = fs::read_to_string(&log).unwrap_or_default();
        panic!("{name}: {error}. Output:\n{output}");
    }
    eprintln!("saved {}", target.display());
}

macro_rules! scenario {
    ($test:ident, $name:expr, $macro:expr) => {
        #[test]
        #[ignore = "starts the editor: needs a display and ume-script (see the module comment)"]
        fn $test() {
            take($name, $macro);
        }
    };
}

scenario!(main_window, "main-window", None);
// `RMenu`: the pop-up menu of the right button, at the caret.
scenario!(open_menu, "open-menu", Some("RMenu();\n"));
// `OptionCommon` opens the common settings at the first page (全般); `OptionType` the settings of
// the type at its first page (スクリーン).
scenario!(
    common_settings,
    "common-settings",
    Some("OptionCommon();\n")
);
scenario!(type_settings, "type-settings", Some("OptionType();\n"));
scenario!(print_preview, "print-preview", Some("PrintPreview();\n"));
scenario!(
    print_settings,
    "print-settings",
    Some("PrintPageSetup();\n")
);
// `SearchDialog`: the find dialog (Ctrl+F).
scenario!(find_dialog, "find-dialog", Some("SearchDialog();\n"));
