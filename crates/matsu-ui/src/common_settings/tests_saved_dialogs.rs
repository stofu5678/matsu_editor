//! Tests of the choices of the Grep, Grep 置換, DIFF and history dialogs that the settings file
//! keeps (`crate::saved_state`), with a real settings file: what the dialogs start with, what they
//! write and when. The keys and the defaults themselves are tested in `saved_state`.
//!
//! Expectations cite `CDlgGrep.cpp`, `CDlgGrepReplace.cpp`, `CDlgDiff.cpp`, `CDlgFavorite.cpp`,
//! `CNormalProcess.cpp` and `CMacro.cpp`.

use gpui::{AnyWindowHandle, Entity, TestAppContext};
use matsu_commands::command::Arg;
use matsu_core::encoding::Encoding;

use super::tests::{Ini, code_of, run, setup};
use crate::file_ops::windows::workspace_of;
use crate::grep::format::{LineType, OutputStyle};
use crate::grep::panel::GrepPanel;
use crate::grep::settings::GrepSettings;
use crate::view::EditorView;

const ALL_KEYS: &str = "[Common]\r\n\
    bGrepSubFolder=0\r\nbGrepOutputLine=2\r\nnGrepOutputStyle=3\r\n\
    bGrepOutputFileOnly=1\r\nbGrepOutputBaseFolder=1\r\nbGrepSeparateFolder=1\r\n\
    bGrepExceptFileRegexp=1\r\nbGrepDefaultFolder=1\r\nbGrepBackup=0\r\nnGrepCharSet=4\r\n\
    bSearchRegularExp=1\r\nbSearchLoHiCase=1\r\nbSearchWordOnly=1\r\n";

/// The same choices, the other way round: what a dialog writes over them.
const KEYS_OFF: &str = "[Common]\r\n\
    bGrepSubFolder=1\r\nbGrepOutputLine=1\r\nnGrepOutputStyle=1\r\n\
    bGrepOutputFileOnly=0\r\nbGrepOutputBaseFolder=0\r\nbGrepSeparateFolder=0\r\n\
    bGrepExceptFileRegexp=0\r\nbGrepDefaultFolder=0\r\nbGrepBackup=1\r\nnGrepCharSet=99\r\n\
    bSearchRegularExp=0\r\nbSearchLoHiCase=0\r\nbSearchWordOnly=0\r\n";

fn view_of(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<EditorView> {
    cx.update(|app| workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone())
}

fn grep_panel(cx: &mut TestAppContext, handle: AnyWindowHandle) -> Entity<GrepPanel> {
    view_of(cx, handle)
        .read_with(cx, |v, _| v.grep.panel.clone())
        .expect("the dialog is open")
}

fn settings_of(cx: &mut TestAppContext, panel: &Entity<GrepPanel>) -> GrepSettings {
    panel.read_with(cx, |p, _| p.settings.clone())
}

/// Puts the key, the file and the folder in the boxes, and `bak` in the one of the excluded files.
fn fill(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    panel: &Entity<GrepPanel>,
    boxes: [&str; 3],
) {
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                let [key, file, folder] = boxes;
                // an excluded file that is a pattern in both forms: the default ones are not
                // regular expressions
                for (state, text) in [
                    (&p.key, key),
                    (&p.file, file),
                    (&p.folder, folder),
                    (&p.exclude_file, "bak"),
                ] {
                    state.update(cx, |s, cx| s.set_value(text, window, cx));
                }
            })
        })
        .unwrap();
}

fn press_ok(cx: &mut TestAppContext, handle: AnyWindowHandle, panel: &Entity<GrepPanel>) {
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.ok(window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

fn press_cancel(cx: &mut TestAppContext, handle: AnyWindowHandle, panel: &Entity<GrepPanel>) {
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| p.cancel(window, cx))
        })
        .unwrap();
    cx.run_until_parked();
}

/// What the user chose in the dialog, the other way round from `KEYS_OFF`.
fn choose_everything(settings: &mut GrepSettings) {
    settings.sub_folder = false;
    settings.line_type = LineType::NoHitLine;
    settings.style = OutputStyle::ResultOnly;
    settings.file_only = true;
    settings.base_folder = true;
    settings.separate_folder = true;
    settings.except_file_regex = true;
    settings.charset = Some(Encoding::Utf8);
    settings.options.regex = true;
    settings.options.match_case = true;
    settings.options.whole_word = true;
}

fn get(ini: &Ini, key: &str) -> Option<String> {
    ini.get(key)
}

// ---- Grep ----

#[gpui::test]
fn the_dialogs_start_with_the_choices_of_the_file(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, ALL_KEYS);
    // Grep 置換 first (a window that has not used the other dialog): it takes everything but the
    // three check boxes of the output (`CDlgGrepReplace::DoModal`)
    run(cx, handle, "F_GREP_REPLACE_DLG");
    let panel = grep_panel(cx, handle);
    let s = settings_of(cx, &panel);
    assert!(!s.sub_folder && !s.backup && s.except_file_regex && s.default_folder);
    assert_eq!(s.line_type, LineType::NoHitLine);
    assert_eq!(s.style, OutputStyle::ResultOnly);
    assert_eq!(s.charset, Some(Encoding::Utf8));
    assert!(s.options.regex && s.options.match_case && s.options.whole_word);
    assert!(!s.file_only && !s.base_folder && !s.separate_folder);
    press_cancel(cx, handle, &panel);
    // Grep takes them too
    run(cx, handle, "F_GREP_DIALOG");
    let panel = grep_panel(cx, handle);
    let s = settings_of(cx, &panel);
    assert!(s.file_only && s.base_folder && s.separate_folder);
    assert!(!s.sub_folder && s.except_file_regex && s.default_folder);
    assert_eq!(s.charset, Some(Encoding::Utf8));
}

#[gpui::test]
fn a_cancelled_dialog_leaves_the_file_and_the_next_dialog_alone(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, KEYS_OFF);
    let before = ini.bytes();
    run(cx, handle, "F_GREP_DIALOG");
    let panel = grep_panel(cx, handle);
    panel.update(cx, |p, _| choose_everything(&mut p.settings));
    press_cancel(cx, handle, &panel);
    assert_eq!(ini.bytes(), before, "キャンセル writes nothing");
    run(cx, handle, "F_GREP_DIALOG");
    let panel = grep_panel(cx, handle);
    let s = settings_of(cx, &panel);
    assert!(s.sub_folder && !s.file_only && s.charset.is_none());
    assert_eq!(s.line_type, LineType::Line);
    assert!(!s.options.regex);
}

#[gpui::test]
fn ok_writes_the_choices_with_sakuras_keys(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, KEYS_OFF);
    let dir = tempfile::tempdir().unwrap();
    run(cx, handle, "F_GREP_DIALOG");
    let panel = grep_panel(cx, handle);
    panel.update(cx, |p, _| choose_everything(&mut p.settings));
    fill(
        cx,
        handle,
        &panel,
        ["a.c", "*.txt", &dir.path().to_string_lossy()],
    );
    press_ok(cx, handle, &panel);
    for (key, value) in [
        ("bGrepSubFolder", "0"),
        ("bGrepOutputLine", "2"),
        ("nGrepOutputStyle", "3"),
        ("bGrepOutputFileOnly", "1"),
        ("bGrepOutputBaseFolder", "1"),
        ("bGrepSeparateFolder", "1"),
        ("bGrepExceptFileRegexp", "1"),
        ("nGrepCharSet", "4"),
        // shared with the find dialog
        ("bSearchRegularExp", "1"),
        ("bSearchLoHiCase", "1"),
        ("bSearchWordOnly", "1"),
        // not a choice of the Grep dialog, nor was it clicked
        ("bGrepBackup", "1"),
        ("bGrepDefaultFolder", "0"),
    ] {
        assert_eq!(get(&ini, key).as_deref(), Some(value), "{key}");
    }
}

#[gpui::test]
fn a_refused_ok_keeps_what_is_written_before_the_checks_and_nothing_after(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, KEYS_OFF);
    run(cx, handle, "F_GREP_DIALOG");
    let panel = grep_panel(cx, handle);
    panel.update(cx, |p, _| choose_everything(&mut p.settings));
    // the folder is not there
    fill(
        cx,
        handle,
        &panel,
        ["a.c", "*.txt", "/no/such/folder/at/all"],
    );
    press_ok(cx, handle, &panel);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    // the character set, the form and the check boxes of the output are written first
    for (key, value) in [
        ("bGrepOutputLine", "2"),
        ("nGrepOutputStyle", "3"),
        ("bGrepOutputFileOnly", "1"),
        ("bGrepExceptFileRegexp", "1"),
        ("nGrepCharSet", "4"),
    ] {
        assert_eq!(get(&ini, key).as_deref(), Some(value), "{key}");
    }
    // the sub folder and the options of the pattern are written when the boxes are accepted
    for (key, value) in [
        ("bGrepSubFolder", "1"),
        ("bSearchRegularExp", "0"),
        ("bSearchLoHiCase", "0"),
        ("bSearchWordOnly", "0"),
    ] {
        assert_eq!(get(&ini, key).as_deref(), Some(value), "{key}");
    }
}

#[gpui::test]
fn an_empty_condition_keeps_no_options_of_the_pattern(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, KEYS_OFF);
    let dir = tempfile::tempdir().unwrap();
    run(cx, handle, "F_GREP_DIALOG");
    let panel = grep_panel(cx, handle);
    panel.update(cx, |p, _| choose_everything(&mut p.settings));
    // a search of the file names only
    fill(
        cx,
        handle,
        &panel,
        ["", "*.txt", &dir.path().to_string_lossy()],
    );
    press_ok(cx, handle, &panel);
    assert_eq!(get(&ini, "bSearchRegularExp").as_deref(), Some("0"));
    assert_eq!(get(&ini, "bSearchWordOnly").as_deref(), Some("0"));
    // the rest is kept
    assert_eq!(get(&ini, "bGrepSubFolder").as_deref(), Some("0"));
    assert_eq!(get(&ini, "nGrepCharSet").as_deref(), Some("4"));
}

#[gpui::test]
fn a_search_of_the_text_of_the_window_leaves_the_sub_folder_alone(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, KEYS_OFF);
    run(cx, handle, "F_GREP_DIALOG");
    let panel = grep_panel(cx, handle);
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.set_from_this_text(true, window, cx);
                choose_everything(&mut p.settings);
            })
        })
        .unwrap();
    fill(cx, handle, &panel, ["a.c", "", ""]);
    press_ok(cx, handle, &panel);
    assert_eq!(get(&ini, "bGrepSubFolder").as_deref(), Some("1"));
    assert_eq!(get(&ini, "bGrepOutputLine").as_deref(), Some("2"));
    assert_eq!(get(&ini, "bSearchRegularExp").as_deref(), Some("1"));
}

#[gpui::test]
fn grep_replace_keeps_the_backup_once_it_has_a_condition(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, KEYS_OFF);
    let dir = tempfile::tempdir().unwrap();
    run(cx, handle, "F_GREP_REPLACE_DLG");
    let panel = grep_panel(cx, handle);
    panel.update(cx, |p, _| {
        choose_everything(&mut p.settings);
        p.settings.backup = false;
    });
    // no condition: the dialog says so and writes nothing
    let before = ini.bytes();
    fill(
        cx,
        handle,
        &panel,
        ["", "*.txt", &dir.path().to_string_lossy()],
    );
    press_ok(cx, handle, &panel);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    assert_eq!(ini.bytes(), before);
    // with one
    fill(
        cx,
        handle,
        &panel,
        ["a.c", "*.txt", &dir.path().to_string_lossy()],
    );
    press_ok(cx, handle, &panel);
    assert_eq!(get(&ini, "bGrepBackup").as_deref(), Some("0"));
    assert_eq!(get(&ini, "nGrepOutputStyle").as_deref(), Some("3"));
    assert_eq!(get(&ini, "bGrepSubFolder").as_deref(), Some("0"));
}

#[gpui::test]
fn the_default_folder_is_written_at_the_click(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, KEYS_OFF);
    cx.update(|app| crate::saved_state::grep::remember_default_folder(app, true));
    assert_eq!(get(&ini, "bGrepDefaultFolder").as_deref(), Some("1"));
    run(cx, handle, "F_GREP_DIALOG");
    let panel = grep_panel(cx, handle);
    assert!(settings_of(cx, &panel).default_folder);
}

#[gpui::test]
fn the_macro_grep_keeps_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, KEYS_OFF);
    let dir = tempfile::tempdir().unwrap();
    let before = ini.bytes();
    let view = view_of(cx, handle);
    // `Grep key, files, folder, flags, code`: the sub folder, the case, a regular expression, the
    // form of the result and the character set, all other than what the file has
    let args = [
        Arg::Text("a.c".into()),
        Arg::Text("*.txt".into()),
        Arg::Text(dir.path().to_string_lossy().into_owned()),
        Arg::Int(0x0001 | 0x0004 | 0x0008 | 0x0040 | 0x20000),
        Arg::Int(4),
    ];
    let code = code_of("F_GREP");
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &args, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(ini.bytes(), before, "the macro does not use the dialog");
}

// ---- DIFF ----

fn diff_panel(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
) -> Entity<crate::diff::panel::DiffPanel> {
    view_of(cx, handle)
        .read_with(cx, |v, _| v.diff.panel.clone())
        .expect("the dialog is open")
}

#[gpui::test]
fn the_diff_dialog_starts_with_the_flags_of_the_file_and_ok_writes_them(cx: &mut TestAppContext) {
    let (ini, handle) = setup(
        cx,
        "[Other]\r\nnDiffFlgOpt=3\r\n[Common]\r\nbSearchNOTIFYNOTFOUND=0\r\nbSearchAll=0\r\n",
    );
    run(cx, handle, "F_DIFF_DIALOG");
    let panel = diff_panel(cx, handle);
    assert_eq!(panel.read_with(cx, |p, _| p.flags()), 3);
    // キャンセル writes nothing
    let before = ini.bytes();
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.set_flags(0x0004 | 0x0008 | 0x0020 | 0x0040);
                p.cancel(window, cx)
            })
        })
        .unwrap();
    assert_eq!(ini.bytes(), before);
    // OK writes before it looks for the other side, which this one does not have
    run(cx, handle, "F_DIFF_DIALOG");
    let panel = diff_panel(cx, handle);
    assert_eq!(panel.read_with(cx, |p, _| p.flags()), 3);
    let flags = 0x0004 | 0x0008 | 0x0020 | 0x0040;
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.set_flags(flags);
                p.set_file("", window, cx);
                p.ok(window, cx)
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        ini.profile().get("Other", "nDiffFlgOpt"),
        Some(flags.to_string().as_str())
    );
    // the next dialog, in any window, starts with the flags
    run(cx, handle, "F_DIFF_DIALOG");
    assert_eq!(
        diff_panel(cx, handle).read_with(cx, |p, _| p.flags()),
        flags
    );
}

#[gpui::test]
fn the_search_options_of_the_diff_dialog_are_the_ones_of_the_find_dialog(cx: &mut TestAppContext) {
    let (ini, handle) = setup(
        cx,
        "[Common]\r\nbSearchNOTIFYNOTFOUND=0\r\nbSearchAll=1\r\nbSearchRegularExp=1\r\n",
    );
    run(cx, handle, "F_DIFF_DIALOG");
    let panel = diff_panel(cx, handle);
    // the dialog starts with the shared ones
    assert_eq!(panel.read_with(cx, |p, _| p.search_flags()), (false, true));
    handle
        .update(cx, |_, window, app| {
            panel.update(app, |p, cx| {
                p.set_file("", window, cx);
                p.ok(window, cx)
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert_eq!(get(&ini, "bSearchNOTIFYNOTFOUND").as_deref(), Some("0"));
    assert_eq!(get(&ini, "bSearchAll").as_deref(), Some("1"));
    // the others stay
    assert_eq!(get(&ini, "bSearchRegularExp").as_deref(), Some("1"));
}

#[gpui::test]
fn a_missing_key_leaves_the_flags_the_dialog_always_had(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "[Common]\r\n");
    run(cx, handle, "F_DIFF_DIALOG");
    assert_eq!(
        diff_panel(cx, handle).read_with(cx, |p, _| p.flags()),
        matsu_core::diff::DiffOptions::NOTIFY_NO_DIFF
    );
}

#[gpui::test]
fn the_macro_diff_keeps_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Other]\r\nnDiffFlgOpt=3\r\n");
    let dir = tempfile::tempdir().unwrap();
    let other = dir.path().join("other.txt");
    std::fs::write(&other, "x\ny\n").unwrap();
    let before = ini.bytes();
    let view = view_of(cx, handle);
    let args = [
        Arg::Text(other.to_string_lossy().into_owned()),
        Arg::Int(0x0004 | 0x0040),
    ];
    let code = code_of("F_DIFF");
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| view.run_function(code, &args, window, cx))
        })
        .unwrap();
    for _ in 0..50 {
        cx.run_until_parked();
    }
    assert_eq!(ini.bytes(), before, "the macro runs with its own flags");
    assert_eq!(
        cx.update(|app| crate::diff::remembered(app).flags),
        3,
        "and the dialog still starts with the saved ones"
    );
}

// ---- the history window ----

fn open_history_window(cx: &mut TestAppContext) -> AnyWindowHandle {
    let before = cx.windows();
    cx.update(crate::history::dialog::open);
    cx.run_until_parked();
    cx.windows()
        .into_iter()
        .find(|w| !before.contains(w))
        .expect("the dialog opens a window")
}

#[gpui::test]
fn the_history_window_opens_where_it_was_closed(cx: &mut TestAppContext) {
    let (ini, _handle) = setup(cx, "[Common]\r\nrcFavoriteDialog=100,120,500,420\r\n");
    let dialog = open_history_window(cx);
    let bounds = dialog
        .update(cx, |_, window, _| window.window_bounds().get_bounds())
        .unwrap();
    assert_eq!(
        crate::saved_state::dialog_rect::Rect::of_bounds(bounds).text(),
        "100,120,500,420"
    );
    // closed whichever way, it keeps its place
    dialog
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        ini.profile().get("Common", "rcFavoriteDialog"),
        Some("100,120,500,420")
    );
}

#[gpui::test]
fn a_place_off_the_screen_is_brought_back(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(
        cx,
        "[Common]\r\nrcFavoriteDialog=90000,90000,90400,90300\r\n",
    );
    let dialog = open_history_window(cx);
    let bounds = dialog
        .update(cx, |_, window, _| window.window_bounds().get_bounds())
        .unwrap();
    let areas = cx.update(|app| crate::saved_state::dialog_rect::work_areas(app));
    let rect = crate::saved_state::dialog_rect::Rect::of_bounds(bounds);
    assert_eq!(
        (rect.width(), rect.height()),
        (400, 300),
        "the size is kept"
    );
    assert!(
        areas.iter().any(|a| rect.left >= a.left
            && rect.right <= a.right
            && rect.top >= a.top
            && rect.bottom <= a.bottom),
        "{rect:?} is inside {areas:?}"
    );
}
