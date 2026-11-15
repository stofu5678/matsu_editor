use gpui::TestAppContext;
use matsu_config::Profile;
use matsu_config::terminal::{DEFAULT_SCROLLBACK_LINES, MAX_SCROLLBACK_LINES, TerminalSettings};
use matsu_terminal::CommandLineError;

use super::{
    OPTION_AS_META, SCROLLBACK, SHELL, items, items_for, problem, problem_text, settings_of,
};
use crate::common_settings::model::{Kind, Values, controls};
#[cfg(target_os = "macos")]
use crate::common_settings::tests::check;
use crate::common_settings::tests::{cancel, ok, run, setup, type_in, with_dialog};
use crate::common_settings::{Page, all_controls};

fn bytes(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn open(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) {
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Terminal, dcx));
}

fn now(cx: &mut TestAppContext) -> TerminalSettings {
    cx.update(|app| crate::terminal::settings::current(app))
}

// ---- the controls ----

#[test]
fn the_page_is_called_shell_and_comes_before_the_one_liners() {
    assert_eq!(Page::Terminal.title(), "シェル");
    assert_eq!(Page::ALL[Page::ALL.len() - 2], Page::Terminal);
    assert_eq!(Page::ALL.last(), Some(&Page::OneLiner));
    assert!(Page::Terminal.items().is_some());
}

#[test]
fn the_keys_are_in_terminal_and_not_in_common() {
    let list = controls(&items_for(true));
    let by_id = |id: &str| list.iter().find(|c| c.id == id).expect(id);
    let shell = by_id(SHELL);
    assert_eq!((shell.section, shell.key), ("Terminal", "Shell"));
    assert!(matches!(shell.kind, Kind::Text));
    let lines = by_id(SCROLLBACK);
    assert_eq!((lines.section, lines.key), ("Terminal", "ScrollbackLines"));
    match lines.kind {
        Kind::Number { min, max } => {
            assert_eq!(min, 0);
            assert_eq!(max, MAX_SCROLLBACK_LINES as i32);
        }
        _ => panic!("the scrollback is a number"),
    }
    let meta = by_id(OPTION_AS_META);
    assert_eq!((meta.section, meta.key), ("Terminal", "OptionAsMeta"));
    assert!(matches!(meta.kind, Kind::Check));
    // Every one of them is live and in the section.
    assert!(list.iter().all(|c| c.live && c.section == "Terminal"));
    // They are on the sheet of the dialog.
    let all = all_controls();
    assert!(all.iter().any(|c| c.id == SHELL));
    assert!(all.iter().any(|c| c.id == SCROLLBACK));
}

#[test]
fn only_macos_has_the_option_check_box() {
    let has = |mac| {
        controls(&items_for(mac))
            .iter()
            .any(|c| c.id == OPTION_AS_META)
    };
    assert!(has(true));
    assert!(!has(false));
    assert_eq!(
        controls(&items())
            .iter()
            .any(|control| control.id == OPTION_AS_META),
        cfg!(target_os = "macos")
    );
}

#[test]
fn the_values_come_from_the_file_or_the_defaults() {
    let list = controls(&items_for(true));
    let values = Values::load(&list, &Profile::new());
    assert_eq!(values.text(SHELL), "");
    assert_eq!(values.int(SCROLLBACK), DEFAULT_SCROLLBACK_LINES as i32);
    assert!(!values.bool(OPTION_AS_META));
    assert_eq!(settings_of(&values), TerminalSettings::default());

    let file = Profile::from_bytes(&bytes(
        "[Terminal]\r\nShell=cmd.exe /K title \"a b\"\r\nScrollbackLines=123\r\nOptionAsMeta=1\r\n",
    ));
    let values = Values::load(&list, &file);
    assert_eq!(values.text(SHELL), "cmd.exe /K title \"a b\"");
    assert_eq!(values.int(SCROLLBACK), 123);
    assert!(values.bool(OPTION_AS_META));
    assert_eq!(
        settings_of(&values),
        TerminalSettings::from_profile(&file),
        "the page and the reader of the file agree"
    );
}

#[test]
fn values_outside_the_range_are_taken_the_way_the_file_is() {
    let list = controls(&items_for(true));
    for (text, lines) in [
        ("99999", MAX_SCROLLBACK_LINES),
        ("-3", DEFAULT_SCROLLBACK_LINES),
        ("abc", DEFAULT_SCROLLBACK_LINES),
    ] {
        let file =
            Profile::from_bytes(&bytes(&format!("[Terminal]\r\nScrollbackLines={text}\r\n")));
        assert_eq!(
            settings_of(&Values::load(&list, &file)).scrollback_lines,
            lines,
            "{text}"
        );
    }
}

#[test]
fn a_command_line_that_cannot_be_read_is_said_so() {
    assert_eq!(problem(""), None);
    assert_eq!(problem("   "), None);
    assert_eq!(problem("sh -l"), None);
    assert_eq!(
        problem("\"sh -c unclosed"),
        Some(CommandLineError::UnclosedQuote)
    );
    assert_eq!(problem("\"\""), Some(CommandLineError::Empty));
    assert!(problem_text(CommandLineError::UnclosedQuote).contains("引用符"));
    assert!(problem_text(CommandLineError::Empty).contains("既定のシェル"));
}

// ---- the dialog ----

#[gpui::test]
fn the_page_shows_what_the_file_says(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(
        cx,
        "[Terminal]\r\nShell=/bin/zsh -l\r\nScrollbackLines=123\r\n",
    );
    // The start of the application took the file's settings.
    assert_eq!(now(cx).shell, "/bin/zsh -l");
    assert_eq!(now(cx).scrollback_lines, 123);
    open(cx, handle);
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::Terminal);
    assert!(!with_dialog(cx, |d, _, _| d.page_controls()).is_empty());
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().text(SHELL).to_owned()),
        "/bin/zsh -l"
    );
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(SCROLLBACK)), 123);
    cancel(cx);
}

#[gpui::test]
fn ok_writes_the_section_and_keeps_the_rest_of_the_file(cx: &mut TestAppContext) {
    let (ini, handle) = setup(
        cx,
        "[Other]\r\nx=1\r\n[Terminal]\r\nFutureKey=keep me\r\nScrollbackLines=10\r\n",
    );
    assert_eq!(now(cx).scrollback_lines, 10);
    open(cx, handle);
    type_in(cx, SHELL, "cmd.exe /K title \"a b\"");
    type_in(cx, SCROLLBACK, "777");
    ok(cx);

    let after = ini.profile();
    assert_eq!(
        after.get("Terminal", "Shell"),
        Some("cmd.exe /K title \"a b\"")
    );
    assert_eq!(after.get("Terminal", "ScrollbackLines"), Some("777"));
    assert_eq!(after.get("Terminal", "FutureKey"), Some("keep me"));
    assert_eq!(after.get("Other", "x"), Some("1"));
    // Read again by the reader of the file, and taken by the tabs that start next.
    let read = TerminalSettings::from_profile(&after);
    assert_eq!(read.shell, "cmd.exe /K title \"a b\"");
    assert_eq!(read.scrollback_lines, 777);
    assert_eq!(now(cx), read);
}

#[gpui::test]
fn a_number_outside_the_range_is_taken_into_it_by_ok(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    open(cx, handle);
    type_in(cx, SCROLLBACK, "999999");
    ok(cx);
    assert_eq!(
        ini.profile().get("Terminal", "ScrollbackLines"),
        Some(MAX_SCROLLBACK_LINES.to_string().as_str())
    );
    assert_eq!(now(cx).scrollback_lines, MAX_SCROLLBACK_LINES);
}

#[gpui::test]
fn clearing_the_shell_goes_back_to_the_default(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Terminal]\r\nShell=zsh\r\n");
    assert_eq!(now(cx).shell, "zsh");
    open(cx, handle);
    type_in(cx, SHELL, "");
    ok(cx);
    assert_eq!(ini.profile().get("Terminal", "Shell"), Some(""));
    assert_eq!(now(cx).shell, "");
    assert_eq!(crate::terminal::settings::shell_of(&now(cx)), None);
}

/// The note under the box is made from the text in the box, not from the value of the last OK.
#[gpui::test]
fn the_note_follows_the_text_in_the_box(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    open(cx, handle);
    let live = |cx: &mut TestAppContext| {
        with_dialog(cx, |d, _, dcx| d.input_text(SHELL, dcx)).expect("the box")
    };
    type_in(cx, SHELL, "\"sh -c unclosed");
    assert_eq!(problem(&live(cx)), Some(CommandLineError::UnclosedQuote));
    type_in(cx, SHELL, "sh -l");
    assert_eq!(problem(&live(cx)), None);
    cancel(cx);
}

#[gpui::test]
fn cancel_changes_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Terminal]\r\nShell=zsh\r\nScrollbackLines=42\r\n");
    let before = ini.bytes();
    let settings = now(cx);
    open(cx, handle);
    type_in(cx, SHELL, "fish");
    type_in(cx, SCROLLBACK, "9");
    cancel(cx);
    assert_eq!(ini.bytes(), before);
    assert_eq!(now(cx), settings);
}

#[cfg(target_os = "macos")]
#[gpui::test]
fn the_option_check_box_is_written_and_taken(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "");
    open(cx, handle);
    check(cx, OPTION_AS_META, true);
    ok(cx);
    assert_eq!(ini.profile().get("Terminal", "OptionAsMeta"), Some("1"));
    assert!(now(cx).option_as_meta);
}

/// Off macOS the page has no such box, and OK leaves what the file says.
#[cfg(not(target_os = "macos"))]
#[gpui::test]
fn the_option_setting_of_the_file_is_kept_where_the_page_has_no_box(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Terminal]\r\nOptionAsMeta=1\r\n");
    open(cx, handle);
    assert!(!with_dialog(cx, |d, _, _| d
        .page_controls()
        .iter()
        .any(|c| c.id == OPTION_AS_META)));
    ok(cx);
    assert_eq!(ini.profile().get("Terminal", "OptionAsMeta"), Some("1"));
}
