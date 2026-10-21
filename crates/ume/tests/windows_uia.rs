//! Drives the window of the built `ume` program with UI Automation on Windows, by the
//! accessibility names of its controls, the way a user does (`crates/ume-ui/src/a11y.rs` and the
//! `semantics.rs` files give the names): open a menu and choose an item, go through the pages of
//! the settings dialogs, toggle a check box and press OK, type text and read the status bar, and
//! close without saving.
//!
//! A picture of the screen is saved at every step (`<platform>-uia-<step>.png`, see
//! `e2e_screens.rs`), so that the pages that need a click can be looked at too.
//!
//! The test runs only on CI or with `UME_UIA_TESTS=1` (it opens windows and posts keys), and it is
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

/// Opens the menu `menu` of the menu bar and chooses its item that `is_item` accepts.
fn choose_menu_item(uia: &Uia, pid: u32, menu: &str, is_item: impl Fn(&str) -> bool) {
    let (_, bar_item) = uia.wait(pid, &format!("the menu {menu}"), |e| {
        e.is_button() && e.name() == menu
    });
    bar_item.invoke();
    pause(500);
    shot(&format!("menu-{menu}"));
    let (_, item) = uia.wait(pid, &format!("an item of the menu {menu}"), |e| {
        is_item(&e.name())
    });
    item.invoke();
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
    choose_menu_item(&uia, pid, "設定", |name| name.starts_with("共通設定"));
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
    choose_menu_item(&uia, pid, "設定", |name| {
        name.starts_with("タイプ別設定") && !name.starts_with("タイプ別設定一覧")
    });
    // The first page of the settings of the type is 「スクリーン」.
    visit_pages(&uia, pid, "スクリーン", "type-settings");
    let (_, cancel) = uia.wait(pid, "the cancel button", |e| {
        e.is_button() && e.name().starts_with("キャンセル")
    });
    cancel.invoke();
    uia.wait_gone(pid, "the type settings", |e| {
        e.is_tab_item() && e.name() == "スクリーン"
    });

    // Typing changes the cursor position in the status bar.
    main.post_chars("あいう");
    let start = Instant::now();
    loop {
        let now = position(&uia, pid);
        if now.as_deref().is_some_and(|text| text != before) {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "the cursor position stayed {before:?} after typing; the tree:\n{}",
            uia.dump(pid)
        );
        pause(200);
    }
    shot("typed");

    // Close without saving: answer the question with 「いいえ」.
    main.post_close();
    let (_, no) = uia.wait(pid, "the answer いいえ", |e| {
        e.is_button() && e.name().starts_with("いいえ")
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
        "the file was saved although the answer was いいえ"
    );
}
