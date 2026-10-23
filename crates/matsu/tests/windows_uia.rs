//! Drives the window of the built `matsu` program with UI Automation on Windows, by the
//! accessibility names of its controls, the way a user does (`crates/matsu-ui/src/a11y.rs` and the
//! `semantics.rs` files give the names): open a menu and choose an item, go through the pages of
//! the settings dialogs, toggle a check box and press OK, type text and read the status bar, and
//! close without saving.
//!
//! A picture of the screen is saved at every step (`<platform>-uia-<step>.png`, see
//! `e2e_screens.rs`), so that the pages that need a click can be looked at too.
//!
//! The test runs only on CI or with `MATSU_UIA_TESTS=1` (it opens windows and posts keys), and it is
//! `#[ignore]` like the other E2E tests: see `.github/workflows/e2e.yml`.
#![cfg(windows)]

mod common;

use std::fs;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

use common::uia::{Element, Uia};
use common::{Scratch, capture_screen, screenshot_path, spawn_editor, stage_binaries};

/// The program ends with the test, also when the test fails.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A picture of the screen for looking at. A failed capture is told, not a failure of the test.
fn shot(step: &str) {
    if common::soft_screens() {
        return;
    }
    let path = screenshot_path(&format!("uia-{step}"));
    match capture_screen(&path) {
        Ok(()) => eprintln!("saved {}", path.display()),
        Err(error) => eprintln!("WARNING: no picture of {step}: {error}"),
    }
}

fn pause(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}

/// The text of the status bar part `カーソル位置`.
fn position(uia: &Uia, pid: u32) -> Option<String> {
    uia.find(pid, |e| e.name().starts_with("カーソル位置: "))
        .map(|(_, e)| e.name())
}

/// Opens the menu `menu` of the menu bar and chooses its item by the access key `key`, the way a
/// user does. The items of a pop-up menu have no accessibility names yet, so UIA cannot find them;
/// if the key does not open what `ready` looks for, the button of the toolbar named `toolbar`
/// (which does the same) is pressed instead, and the test says so.
fn choose_menu_item(
    uia: &Uia,
    pid: u32,
    main: &Element,
    menu: &str,
    key: char,
    toolbar: &str,
    ready: impl Fn(&Element) -> bool,
) {
    let (_, bar_item) = uia.wait(pid, &format!("the menu {menu}"), |e| {
        e.is_button() && e.name() == menu
    });
    bar_item.invoke();
    pause(500);
    shot(&format!("menu-{menu}"));
    main.post_key(key);
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) {
        if uia.find(pid, &ready).is_some() {
            return;
        }
        pause(300);
    }
    eprintln!("NOTE: the access key {key} of the menu {menu} did not open it; pressing {toolbar}");
    // The open menu takes the first click away, so close it before.
    main.post_key('\u{1b}');
    pause(500);
    let (_, button) = uia.wait(pid, toolbar, |e| e.is_button() && e.name() == toolbar);
    button.invoke();
}

/// Goes through every page of the settings dialog that has the page `first`, with a picture of
/// each, and comes back to the first. Returns the window of the dialog.
fn visit_pages(uia: &Uia, pid: u32, first: &str, prefix: &str) -> Element {
    let (dialog, _) = uia.wait(pid, &format!("the settings page {first}"), |e| {
        e.is_tab_item() && e.name() == first
    });
    pause(800);
    let pages: Vec<Element> = dialog
        .descendants(uia)
        .into_iter()
        .filter(Element::is_tab_item)
        .collect();
    assert!(
        pages.len() >= 5,
        "the dialog has only {} pages:\n{}",
        pages.len(),
        dialog.dump(uia)
    );
    for (i, page) in pages.iter().enumerate() {
        page.select();
        pause(500);
        eprintln!("{prefix}: page {} is {:?}", i + 1, page.name());
        shot(&format!("{prefix}-page-{:02}", i + 1));
    }
    pages[0].select();
    pause(500);
    dialog
}

#[test]
#[ignore = "starts the editor and drives it: needs a desktop (see the module comment)"]
fn drives_the_window_by_its_names() {
    let Some(uia) = Uia::connect() else {
        return;
    };
    let scratch = Scratch::new("uia");
    let sample = scratch.0.join("sample.txt");
    let original = "\u{feff}日本語の文書\r\n二行目\r\n".as_bytes().to_vec();
    fs::write(&sample, &original).unwrap();
    let exe = stage_binaries(&scratch.0);
    let log = scratch.0.join("output.log");
    let sample_arg = sample.to_str().unwrap().to_owned();
    let child = Running(spawn_editor(
        &exe,
        &scratch.0,
        &["--separate-process", &sample_arg],
        &log,
    ));
    let pid = child.0.id();

    // The window is up when the text view has its name.
    let (main, _) = uia.wait(pid, "the text view", |e| e.name() == "テキストエディタ");
    pause(1000);
    shot("main-window");
    eprintln!("the main window:\n{}", main.dump(&uia));
    let before = position(&uia, pid).expect("the cursor position in the status bar");

    // 設定 -> 共通設定: every page, a check box on the first, OK.
    choose_menu_item(&uia, pid, &main, "設定", 'c', "共通設定...", |e| {
        e.is_tab_item() && e.name() == "全般"
    });
    let dialog = visit_pages(&uia, pid, "全般", "common-settings");
    let check = dialog
        .descendants(&uia)
        .into_iter()
        .find(|e| e.is_check_box() && e.is_enabled())
        .unwrap_or_else(|| {
            panic!(
                "no enabled check box on the first page:\n{}",
                dialog.dump(&uia)
            )
        });
    let was = check.is_checked();
    check.toggle();
    pause(300);
    assert_eq!(check.is_checked(), !was, "the check box did not flip");
    shot("common-settings-toggled");
    let (_, ok) = uia.wait(pid, "the OK button", |e| e.is_button() && e.name() == "OK");
    ok.invoke();
    uia.wait_gone(pid, "the common settings", |e| {
        e.is_tab_item() && e.name() == "全般"
    });

    // 設定 -> タイプ別設定: every page, then cancel.
    choose_menu_item(
        &uia,
        pid,
        &main,
        "設定",
        'y',
        "タイプ別設定...",
        |e| e.is_tab_item() && e.name() == "スクリーン",
    );
    // The first page of the settings of the type is 「スクリーン」.
    visit_pages(&uia, pid, "スクリーン", "type-settings");
    let (_, cancel) = uia.wait(pid, "the cancel button", |e| {
        e.is_button() && e.name().starts_with("キャンセル")
    });
    cancel.invoke();
    uia.wait_gone(pid, "the type settings", |e| {
        e.is_tab_item() && e.name() == "スクリーン"
    });

    // Typing changes the cursor position in the status bar. The keys of `SendInput` first; if the
    // program does not take them (some runners do not give keys to a window), characters posted
    // to the window.
    let (_, text_view) = uia.wait(pid, "the text view", |e| e.name() == "テキストエディタ");
    let moved = |wait: Duration| {
        let start = Instant::now();
        while start.elapsed() < wait {
            if position(&uia, pid).is_some_and(|text| text != before) {
                return true;
            }
            pause(200);
        }
        false
    };
    let sent = text_view.send_text(&main, "あいう");
    if !(sent && moved(Duration::from_secs(10))) {
        eprintln!("NOTE: the keys did not reach the text view (sent: {sent}); posting characters");
        main.post_chars("あいう");
        assert!(
            moved(Duration::from_secs(20)),
            "the cursor position stayed {before:?} after typing; the tree:\n{}",
            uia.dump(pid)
        );
    }
    shot("typed");

    // Close without saving: answer the question with 「保存しない」.
    main.post_close();
    let (_, no) = uia.wait(pid, "the answer 保存しない", |e| {
        e.is_button() && e.name().starts_with("保存しない")
    });
    shot("close-question");
    no.invoke();
    let mut child = child;
    let start = Instant::now();
    loop {
        if child.0.try_wait().unwrap().is_some() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "the program did not end after the answer; the tree:\n{}",
            uia.dump(pid)
        );
        pause(200);
    }
    assert_eq!(
        fs::read(&sample).unwrap(),
        original,
        "the file was saved although the answer was 保存しない"
    );
}
