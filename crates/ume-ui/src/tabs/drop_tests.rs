//! Tests of files dropped on the window (`CEditWnd::OnDropFiles`): anywhere on the frame, the
//! limit of files, and "close and open" (`bDropFileAndClose`).

use std::path::PathBuf;

use gpui::{AnyWindowHandle, TestAppContext, point, px};
use ume_config::Profile;

use super::tests::{
    active_tab, active_view, answer, captions, tab_count, tab_mode, type_text, visual, window,
    windows,
};
use crate::settings::UserSettings;

const DONT_SAVE: &str = "保存しない(N)";

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ume-ui-drop-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn files(dir: &std::path::Path, names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|name| {
            let path = dir.join(name);
            std::fs::write(&path, format!("text of {name}")).unwrap();
            path
        })
        .collect()
}

/// Drops `paths` on the window at `position`, the way GPUI hands over a drop from the OS.
fn drop_files(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    position: gpui::Point<gpui::Pixels>,
    paths: Vec<PathBuf>,
) {
    let vcx = visual(cx, handle);
    // a frame is drawn by the resize: the drop needs the boxes of the last frame
    vcx.simulate_resize(gpui::size(px(1000.), px(700.)));
    vcx.run_until_parked();
    vcx.simulate_event(gpui::FileDropEvent::Entered {
        position,
        paths: gpui::ExternalPaths(paths.into()),
    });
    vcx.simulate_event(gpui::FileDropEvent::Submit { position });
    vcx.simulate_event(gpui::FileDropEvent::Ended);
    cx.run_until_parked();
}

/// Settings of `[Common]` as the common settings dialog leaves them in `UserSettings`.
fn common(cx: &mut TestAppContext, lines: &str) {
    let profile = Profile::from_bytes(format!("[Common]\n{lines}\n").as_bytes());
    cx.update(|app| app.set_global(UserSettings::from_profile(profile, None)));
}

/// A window whose only document is modified by typing, so that it takes no file. The drops that
/// follow come right after a key: GPUI would not take them by the hover of an element.
fn busy_window(cx: &mut TestAppContext) -> AnyWindowHandle {
    let handle = window(cx, true);
    type_text(cx, handle, "x");
    handle
}

fn on_text() -> gpui::Point<gpui::Pixels> {
    point(px(500.), px(300.))
}

#[gpui::test]
fn a_dropped_file_is_a_tab_wherever_it_lands(cx: &mut TestAppContext) {
    let dir = temp_dir("anywhere");
    let all = files(&dir, &["a.txt", "b.txt", "c.txt", "d.txt"]);
    let handle = busy_window(cx);
    // the menu bar, the status bar and the text (the test window is 1000x700)
    let places = [
        point(px(50.), px(10.)),
        point(px(500.), px(688.)),
        on_text(),
    ];
    for (place, path) in places.into_iter().zip(&all) {
        drop_files(cx, handle, place, vec![path.clone()]);
    }
    assert_eq!(windows(cx), 1, "tabs, not windows");
    assert_eq!(
        captions(cx, handle),
        ["(無題)1(更新)", "a.txt", "b.txt", "c.txt"].map(String::from)
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn a_file_dropped_on_the_tab_bar_is_a_tab(cx: &mut TestAppContext) {
    let dir = temp_dir("tab-bar");
    let all = files(&dir, &["a.txt"]);
    let handle = busy_window(cx);
    // the tab bar is under the menu bar
    drop_files(cx, handle, point(px(500.), px(45.)), all);
    assert_eq!(windows(cx), 1);
    assert_eq!(tab_count(cx, handle), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn a_dropped_file_is_a_window_when_the_windows_are_not_grouped(cx: &mut TestAppContext) {
    let dir = temp_dir("window");
    let all = files(&dir, &["a.txt"]);
    let handle = busy_window(cx);
    tab_mode(cx, true, false);
    drop_files(cx, handle, point(px(50.), px(10.)), all);
    assert_eq!(windows(cx), 2);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn an_untitled_unmodified_document_takes_the_first_file(cx: &mut TestAppContext) {
    let dir = temp_dir("untitled");
    let all = files(&dir, &["a.txt", "b.txt"]);
    let handle = window(cx, true);
    drop_files(cx, handle, point(px(500.), px(688.)), all);
    assert_eq!(windows(cx), 1);
    assert_eq!(captions(cx, handle), ["a.txt", "b.txt"].map(String::from));
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn a_file_that_is_open_already_comes_to_the_front(cx: &mut TestAppContext) {
    let dir = temp_dir("again");
    let all = files(&dir, &["a.txt", "b.txt"]);
    let handle = busy_window(cx);
    drop_files(cx, handle, on_text(), all.clone());
    assert_eq!(tab_count(cx, handle), 3);
    assert_eq!(active_tab(cx, handle), 2);
    drop_files(cx, handle, point(px(50.), px(10.)), vec![all[0].clone()]);
    assert_eq!(tab_count(cx, handle), 3, "no second copy");
    assert_eq!(active_tab(cx, handle), 1);
    assert_eq!(windows(cx), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn at_most_eight_files_are_taken_by_default(cx: &mut TestAppContext) {
    let dir = temp_dir("limit");
    let names: Vec<String> = (0..10).map(|i| format!("f{i}.txt")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let all = files(&dir, &names);
    let handle = busy_window(cx);
    drop_files(cx, handle, on_text(), all);
    assert_eq!(tab_count(cx, handle), 1 + 8);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_limit_is_read_from_the_settings_at_the_time_of_the_drop(cx: &mut TestAppContext) {
    let dir = temp_dir("limit-setting");
    let all = files(&dir, &["a.txt", "b.txt", "c.txt", "d.txt"]);
    let handle = busy_window(cx);
    common(cx, "nDropFileNumMax=2");
    drop_files(cx, handle, on_text(), all.clone());
    assert_eq!(tab_count(cx, handle), 1 + 2);
    // 0 is held to 1, as the dialog does
    common(cx, "nDropFileNumMax=0");
    drop_files(cx, handle, on_text(), all[2..].to_vec());
    assert_eq!(tab_count(cx, handle), 1 + 2 + 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn close_and_open_takes_only_the_first_file_and_replaces_the_document(cx: &mut TestAppContext) {
    let dir = temp_dir("close-open");
    let all = files(&dir, &["a.txt", "b.txt"]);
    let handle = busy_window(cx);
    common(cx, "bDropFileAndClose=1");
    drop_files(cx, handle, point(px(50.), px(10.)), all.clone());
    // the document is modified: the question comes first, and nothing is lost before the answer
    assert_eq!(tab_count(cx, handle), 1);
    answer(cx, handle, DONT_SAVE);
    cx.run_until_parked();
    assert_eq!(tab_count(cx, handle), 1, "no new tab");
    assert_eq!(windows(cx), 1);
    assert_eq!(captions(cx, handle), ["a.txt".to_string()]);
    let text = active_view(cx, handle).read_with(cx, |v, cx| {
        v.document().read(cx).doc().line(0).unwrap().unwrap().text
    });
    assert_eq!(text, "text of a.txt");
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn close_and_open_keeps_the_document_when_the_question_is_cancelled(cx: &mut TestAppContext) {
    let dir = temp_dir("close-open-cancel");
    let all = files(&dir, &["a.txt"]);
    let handle = busy_window(cx);
    common(cx, "bDropFileAndClose=1");
    drop_files(cx, handle, on_text(), all);
    answer(cx, handle, "キャンセル");
    cx.run_until_parked();
    assert_eq!(captions(cx, handle), ["(無題)1(更新)".to_string()]);
    std::fs::remove_dir_all(&dir).ok();
}
