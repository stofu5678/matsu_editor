//! Drives the window of the built `matsu` program on macOS through the Accessibility API, the way
//! `windows_uia.rs` does with UI Automation on Windows: open menus of the menu bar and choose an
//! item, see a dialog open, take a picture of the screen, close the dialog, type, and check that
//! the program is still running.
//!
//! The elements inside the window (the toolbar buttons, the text view, the status bar, the About
//! panel) are in the accessibility tree with the names of `crates/matsu-ui/src/a11y.rs` (AccessKit),
//! so the steps look for them by role and name, as `windows_uia.rs` does.
//!
//! The script language is AppleScript run by `osascript` with System Events (no extra program is
//! needed). GitHub's macOS runners already give `osascript` and `bash` the Accessibility and the
//! Screen Recording permissions in the TCC databases (checked on macos-15, see
//! docs/e2e-screens.md). On a machine without them the test fails with the error of System Events.
//!
//! The test runs only on CI or with `MATSU_AX_TESTS=1` (it opens windows and posts keys), and it is
//! `#[ignore]` like the other E2E tests: see `.github/workflows/e2e.yml`.
#![cfg(target_os = "macos")]

mod common;

use std::fs;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use common::{Scratch, capture_screen, screenshot_path, spawn_editor, stage_binaries};

/// The program ends with the test, also when the test fails.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn pause(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}

/// A picture of the screen for looking at. A failed capture is told, not a failure of the test.
fn shot(step: &str) {
    if common::soft_screens() {
        return;
    }
    let path = screenshot_path(&format!("ax-{step}"));
    match capture_screen(&path) {
        Ok(()) => eprintln!("saved {}", path.display()),
        Err(error) => eprintln!("WARNING: no picture of {step}: {error}"),
    }
}

/// Runs AppleScript `body` inside `tell process whose unix id is pid` of System Events and returns
/// what it returns.
fn osa(pid: u32, body: &str) -> Result<String, String> {
    let script = format!(
        "tell application \"System Events\"\nset p to first process whose unix id is {pid}\n{body}\nend tell"
    );
    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("cannot run osascript: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(format!(
            "osascript failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// The names of the windows of the program, one per line.
fn windows(pid: u32) -> Vec<String> {
    let body = "set out to {}\nrepeat with w in (windows of p)\nset end of out to (name of w as text)\nend repeat\nset AppleScript's text item delimiters to linefeed\nreturn out as text";
    osa(pid, body)
        .map(|s| s.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// The names of the items of the menu bar item that starts with `menu`.
fn menu_items(pid: u32, menu: &str) -> Result<Vec<String>, String> {
    let body = format!(
        "set m to menu 1 of (first menu bar item of menu bar 1 of p whose name starts with \"{menu}\")\nset out to {{}}\nrepeat with i in (menu items of m)\nset end of out to (name of i as text)\nend repeat\nset AppleScript's text item delimiters to linefeed\nreturn out as text"
    );
    osa(pid, &body).map(|s| s.lines().map(str::to_owned).collect())
}

/// AppleScript that lists every element of the first window as `role | name | description | value`,
/// one per line (a property an element does not have is empty).
const TREE: &str = "set out to {}\nset els to entire contents of window 1 of p\nrepeat with i from 1 to count of els\nset e to item i of els\nset r to \"\"\nset n to \"\"\nset d to \"\"\nset v to \"\"\ntry\nset r to (role of e) as text\nend try\ntry\nset n to (name of e) as text\nend try\ntry\nset d to (description of e) as text\nend try\ntry\nset v to (value of e) as text\nend try\nset end of out to r & \" | \" & n & \" | \" & d & \" | \" & v\nend repeat\nset AppleScript's text item delimiters to linefeed\nreturn out as text";

/// The lines of the tree of the first window (see `TREE`). An element with a long text (the About
/// description) takes several lines; the lines of a text are not a role, so look for a role and a
/// name with `has`.
fn tree(pid: u32) -> String {
    osa(pid, TREE).unwrap_or_default()
}

/// Whether the tree has an element of `role` (for example `AXButton`) whose name is `name`.
fn has(tree: &str, role: &str, name: &str) -> bool {
    tree.contains(&format!("{role} | {name} | "))
}

/// Waits until the tree of the first window satisfies `ok`; returns the tree.
fn wait_tree(pid: u32, what: &str, ok: impl Fn(&str) -> bool) -> String {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(40) {
        let text = tree(pid);
        if ok(&text) {
            return text;
        }
        pause(500);
    }
    panic!("no {what} within 40 seconds:\n{}", dump(pid));
}

/// The text of the status bar part that starts with `prefix` (for example `カーソル位置: `).
fn status(tree: &str, prefix: &str) -> Option<String> {
    tree.lines()
        .find_map(|l| l.strip_prefix(&format!("AXStaticText | {prefix}")))
        .map(|rest| rest.split(" | ").next().unwrap_or(rest).to_owned())
}

/// What the accessibility tree of the first window looks like (for the failure messages).
fn dump(pid: u32) -> String {
    let bar = osa(
        pid,
        "return name of every menu bar item of menu bar 1 of p as text",
    )
    .unwrap_or_else(|e| e);
    let tree = osa(pid, TREE).unwrap_or_else(|e| e);
    format!(
        "windows: {:?}\nmenu bar: {bar}\nwindow 1: {tree}",
        windows(pid)
    )
}

/// Chooses the item that starts with `item` in the menu bar item that starts with `menu`.
fn choose(pid: u32, menu: &str, item: &str) {
    if let Ok(items) = menu_items(pid, menu) {
        eprintln!("the menu {menu}: {items:?}");
    }
    let body = format!(
        "set frontmost of p to true\nclick (first menu item of menu 1 of (first menu bar item of menu bar 1 of p whose name starts with \"{menu}\") whose name starts with \"{item}\")"
    );
    if let Err(error) = osa(pid, &body) {
        panic!("cannot choose {menu} -> {item}: {error}\n{}", dump(pid));
    }
}

/// Waits until the program has `count` windows; the names of the windows are returned.
fn wait_windows(pid: u32, what: &str, ok: impl Fn(&[String]) -> bool) -> Vec<String> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(40) {
        let names = windows(pid);
        if ok(&names) {
            return names;
        }
        pause(500);
    }
    panic!("no {what} within 40 seconds:\n{}", dump(pid));
}

#[test]
#[ignore = "starts the editor and drives it: needs a desktop (see the module comment)"]
fn drives_the_window_through_the_accessibility_api() {
    if std::env::var_os("CI").is_none() && std::env::var_os("MATSU_AX_TESTS").is_none() {
        eprintln!("skipped: set MATSU_AX_TESTS=1 to open windows and post keys");
        return;
    }
    let scratch = Scratch::new("ax");
    let sample = scratch.0.join("sample.txt");
    let original = "\u{feff}日本語の文書\r\n二行目\r\n".as_bytes().to_vec();
    fs::write(&sample, &original).unwrap();
    let exe = stage_binaries(&scratch.0);
    let log = scratch.0.join("output.log");
    let sample_arg = sample.to_str().unwrap().to_owned();
    let mut child = Running(spawn_editor(
        &exe,
        &scratch.0,
        &["--separate-process", &sample_arg],
        &log,
    ));
    let pid = child.0.id();

    // The main window is up when System Events lists a window of the process, and it is the one
    // of the file: its title is "(無題)" until the file is read, and the later comparison of the
    // windows with this list would otherwise see the title change (CI, 2026-10-22).
    let main = wait_windows(pid, "main window of sample.txt", |w| {
        w.iter().any(|name| name.contains("sample.txt"))
    });
    eprintln!("the windows at the start: {main:?}");
    // The view is up when the text view has its name; the toolbar and the status bar are there.
    let first = wait_tree(pid, "text view", |t| {
        has(t, "AXTextArea", "テキストエディタ")
    });
    pause(1000);
    shot("main-window");
    eprintln!("the main window:\n{}", dump(pid));
    for button in ["新規作成", "上書き保存", "共通設定..."] {
        assert!(
            has(&first, "AXButton", button),
            "no toolbar button {button}:\n{first}"
        );
    }
    let before = status(&first, "カーソル位置: ").expect("the cursor position in the status bar");

    // 設定 -> 共通設定...: a second window opens, a picture is taken, Escape closes it.
    choose(pid, "設定", "共通設定");
    let with_dialog = wait_windows(pid, "common settings window", |w| w.len() > main.len());
    eprintln!("the windows with the common settings: {with_dialog:?}");
    pause(1000);
    shot("common-settings");
    osa(pid, "key code 53").ok();
    close_dialog(pid, main.len(), "common settings");

    // ヘルプ -> バージョン情報: the About box.
    choose(pid, "ヘルプ", "バージョン情報");
    // The About box is a panel inside the main window (no window of its own): an AXWindow of
    // subrole dialog named バージョン情報 in the tree, with an OK button. Esc closes it.
    let panel = wait_tree(pid, "About panel", |t| has(t, "AXWindow", "バージョン情報"));
    assert!(
        has(&panel, "AXButton", "OK"),
        "no OK button in the About panel:\n{panel}"
    );
    assert!(
        panel.contains("AXStaticText | MATSU Editor Ver. "),
        "no version text in the About panel:\n{panel}"
    );
    pause(500);
    shot("about");
    osa(pid, "key code 53").unwrap();
    wait_tree(pid, "closed About panel", |t| {
        !has(t, "AXWindow", "バージョン情報")
    });
    assert_eq!(windows(pid), main, "the windows changed with the About box");

    // Typing reaches the text view (the cursor position of the status bar changes), the program
    // stays up, and the file is not changed on disk.
    osa(pid, "set frontmost of p to true\nkeystroke \"abc\"").unwrap();
    let moved = wait_tree(pid, "a moved cursor position", |t| {
        status(t, "カーソル位置: ").is_some_and(|now| now != before)
    });
    shot("typed");
    eprintln!(
        "the cursor moved from {before:?} to {:?}",
        status(&moved, "カーソル位置: ")
    );
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "the program ended by itself; its output:\n{}",
        fs::read_to_string(&log).unwrap_or_default()
    );
    assert_eq!(
        fs::read(&sample).unwrap(),
        original,
        "the file changed on disk without a save"
    );
}

/// Closes the dialog that opened over the main window: Escape first, then the close button of
/// the window, and waits until the windows are those of the start again.
fn close_dialog(pid: u32, base: usize, what: &str) {
    for attempt in 0..3 {
        if windows(pid).len() <= base {
            return;
        }
        let key = match attempt {
            0 => "key code 53",
            1 => {
                "click (first button of window 1 of p whose name starts with \"キャンセル\" or name starts with \"閉じる\" or name starts with \"OK\")"
            }
            _ => "click button 1 of window 1 of p",
        };
        if let Err(error) = osa(pid, &format!("set frontmost of p to true\n{key}")) {
            eprintln!("NOTE: closing the {what} (try {attempt}): {error}");
        }
        pause(1500);
    }
    wait_windows(pid, &format!("closed {what}"), |w| w.len() <= base);
}
