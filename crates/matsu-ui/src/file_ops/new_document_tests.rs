//! New documents follow the OS's line ending, and matsu.ini's type settings when it has them
//! (ADR-0010). The window is GPUI's headless test window.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::Profile;

use super::windows::{editors, open_editor_window};
use crate::document::SharedDocument;
use crate::settings::UserSettings;
use crate::view::EditorView;

/// The name of the line ending of the OS in the status bar.
fn os_eol_name() -> &'static str {
    if cfg!(windows) { "CRLF" } else { "LF" }
}

/// The text of the line ending of the OS.
fn os_eol_text() -> &'static str {
    if cfg!(windows) { "\r\n" } else { "\n" }
}

fn temp_dir() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("matsu-ui-eol-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A settings file (UTF-8 with a BOM, as Sakura writes it).
fn user(text: &str) -> UserSettings {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    UserSettings::from_profile(Profile::from_bytes(&bytes), None)
}

/// Opens an untitled window, with `settings` read at startup if given.
fn window(cx: &mut TestAppContext, settings: Option<UserSettings>) -> AnyWindowHandle {
    cx.update(gpui_component::init);
    if let Some(settings) = settings {
        cx.update(|cx| cx.set_global(settings));
    }
    cx.update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap()
}

/// The newest editor window, which the test just opened.
fn view_of(cx: &mut VisualTestContext) -> Entity<EditorView> {
    cx.update(|_, app| editors(app).last().unwrap().1.clone())
}

fn format(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |v, app| v.status(app).file_format())
}

fn run(view: &Entity<EditorView>, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, args, window, cx)
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_new_document_has_the_line_ending_of_the_os(cx: &mut TestAppContext) {
    let handle = window(cx, None);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    assert_eq!(format(&view, cx), format!("UTF-8  {}", os_eol_name()));
    // Enter inserts it.
    cx.simulate_keystrokes("a enter b");
    let eol = view.read_with(cx, |v, app| {
        v.document().read(app).doc().line(0).unwrap().unwrap().eol
    });
    assert_eq!(eol.map(|e| e.as_str()), Some(os_eol_text()));
}

#[gpui::test]
fn saving_a_new_document_writes_the_line_ending_of_the_os(cx: &mut TestAppContext) {
    let dir = temp_dir();
    let path = dir.join("new.txt");
    let handle = window(cx, None);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    cx.simulate_input("a");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("b");
    run(&view, cx, "F_FILESAVE", &[]);
    let target = path.clone();
    cx.simulate_new_path_selection(move |_| Some(target));
    cx.run_until_parked();
    // The Save As options come after the path (#54); Enter keeps what they propose.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        format!("a{0}b", os_eol_text()).into_bytes()
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn a_settings_file_without_the_value_leaves_the_os_line_ending(cx: &mut TestAppContext) {
    let handle = window(cx, Some(user("[Types(0)]\r\nszTypeName=基本\r\n")));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    assert_eq!(format(&view, cx), format!("UTF-8  {}", os_eol_name()));
}

#[gpui::test]
fn the_settings_file_decides_when_it_has_the_values(cx: &mut TestAppContext) {
    // The opposite of the OS's line ending, so the setting is what is seen.
    let (set, name, text) = if cfg!(windows) {
        (2, "LF", "\n")
    } else {
        (1, "CRLF", "\r\n")
    };
    let ini =
        format!("[Types(0)]\r\neDefaultCodetype=3\r\neDefaultEoltype={set}\r\nbDefaultBom=0\r\n");
    let handle = window(cx, Some(user(&ini)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    assert_eq!(format(&view, cx), format!("UTF-16  {name}"));
    cx.simulate_keystrokes("a enter");
    let eol = view.read_with(cx, |v, app| {
        v.document().read(app).doc().line(0).unwrap().unwrap().eol
    });
    assert_eq!(eol.map(|e| e.as_str()), Some(text));
}

#[gpui::test]
fn closing_the_file_leaves_a_document_with_the_defaults(cx: &mut TestAppContext) {
    let (set, name) = if cfg!(windows) {
        (2, "LF")
    } else {
        (1, "CRLF")
    };
    let ini = format!("[Types(0)]\r\neDefaultEoltype={set}\r\n");
    let handle = window(cx, Some(user(&ini)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_FILECLOSE", &[]);
    assert_eq!(format(&view, cx), format!("UTF-8  {name}"));
}

/// The status bar text of a new window that opens `file`.
fn format_after_opening(cx: &mut TestAppContext, file: &std::path::Path) -> String {
    let handle = window(cx, None);
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(
        &view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(file.display().to_string())],
    );
    format(&view, cx)
}

#[gpui::test]
fn a_file_without_a_line_ending_takes_the_os_default(cx: &mut TestAppContext) {
    let dir = temp_dir();
    let plain = dir.join("plain.txt");
    std::fs::write(&plain, b"abc").unwrap();
    assert_eq!(
        format_after_opening(cx, &plain),
        format!("UTF-8  {}", os_eol_name())
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn a_file_with_line_endings_keeps_them(cx: &mut TestAppContext) {
    let dir = temp_dir();
    let crlf = dir.join("crlf.txt");
    std::fs::write(&crlf, b"a\r\nb").unwrap();
    assert_eq!(format_after_opening(cx, &crlf), "UTF-8  CRLF");
    let lf = dir.join("lf.txt");
    std::fs::write(&lf, b"a\nb").unwrap();
    assert_eq!(format_after_opening(cx, &lf), "UTF-8  LF");
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_view_takes_the_type_of_the_file_it_loads(cx: &mut TestAppContext) {
    let dir = temp_dir();
    let path = dir.join("run.bat");
    std::fs::write(&path, b"echo").unwrap();
    let ini = "[Other]\r\nnTypesCount=2\r\n[Types(1)]\r\nszTypeExts=bat\r\n";
    let handle = window(cx, Some(user(ini)));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    assert_eq!(view.read_with(cx, |v, _| v.view_settings().type_index), 0);
    run(
        &view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(path.display().to_string())],
    );
    assert_eq!(view.read_with(cx, |v, _| v.view_settings().type_index), 1);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn reopening_keeps_the_zoom_of_the_view(cx: &mut TestAppContext) {
    let dir = temp_dir();
    let path = dir.join("a.txt");
    std::fs::write(&path, b"abc").unwrap();
    let handle = window(cx, Some(user("[Types(0)]\r\nszTypeName=基本\r\n")));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(
        &view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(path.display().to_string())],
    );
    view.update(cx, |v, cx| v.zoom_font(2, cx));
    let zoomed = view.read_with(cx, |v, _| v.font_px());
    run(&view, cx, "F_FILE_REOPEN_UTF8", &[Arg::Int(1)]);
    assert_eq!(view.read_with(cx, |v, _| v.font_px()), zoomed);
    std::fs::remove_dir_all(&dir).ok();
}

#[gpui::test]
fn the_type_of_a_new_file_name_decides(cx: &mut TestAppContext) {
    // A file that does not exist yet has the settings of its type.
    let dir = temp_dir();
    let ini = "[Other]\r\nnTypesCount=2\r\n[Types(0)]\r\neDefaultEoltype=2\r\n\
               [Types(1)]\r\nszTypeExts=bat\r\neDefaultEoltype=1\r\n";
    cx.update(gpui_component::init);
    cx.update(|cx| cx.set_global(user(ini)));
    assert_eq!(
        format_after_opening(cx, &dir.join("run.bat")),
        "UTF-8  CRLF"
    );
    assert_eq!(format_after_opening(cx, &dir.join("a.txt")), "UTF-8  LF");
    std::fs::remove_dir_all(&dir).ok();
}

/// The message of the migration from the old settings is in the status bar, also after the window reset its
/// notice (a first start loads a file or makes a new document, which clears it).
#[gpui::test]
async fn the_migration_message_is_shown_in_the_status_bar(cx: &mut TestAppContext) {
    crate::settings::set_startup_notice("migrated");
    let handle = window(cx, Some(UserSettings::none()));
    let mut vcx = VisualTestContext::from_window(handle, cx);
    let view = view_of(&mut vcx);
    let shown = view.read_with(&vcx, |v, app| v.status(app).notice);
    assert_eq!(shown.as_deref(), Some("migrated"));
    // Another notice has the place; when it goes, the message is back.
    view.update(&mut vcx, |v, _| v.notice = Some("other".to_owned()));
    let shown = view.read_with(&vcx, |v, app| v.status(app).notice);
    assert_eq!(shown.as_deref(), Some("other"));
    view.update(&mut vcx, |v, _| v.notice = None);
    let shown = view.read_with(&vcx, |v, app| v.status(app).notice);
    assert_eq!(shown.as_deref(), Some("migrated"));
}
