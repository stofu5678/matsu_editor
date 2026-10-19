//! Tests of the registered macros (登録マクロ): `F_USERMACRO_0 + n` runs macro `n` of `[Macro]`, the
//! macros that run by themselves (オープン後, タイプ変更後, 保存前), the names in the lists.
//!
//! The macros are key macro files (`.mac`), so that no interpreter is needed; the one test of a
//! script (`GetSaveFilename`) runs only where Node.js is installed.

use std::path::{Path, PathBuf};

use gpui::{TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_config::Profile;
use ume_config::settings::macros::{RegisteredMacros, function_of_index};
use ume_plugin::session::Language;

use super::{Fixture, available, setup, write};
use crate::menu::{Lists, MenuAction};
use crate::settings::UserSettings;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

/// Puts a settings file with the macro folder `dir` and `macros` (lines of `[Macro]`) in place of
/// the settings of the application.
fn settings(cx: &mut VisualTestContext, dir: &Path, macros: &str) {
    let text = format!(
        "[Folders]\nszMACROFOLDER={}\n[Macro]\n{macros}",
        dir.display()
    );
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    let profile = Profile::from_bytes(&bytes);
    let path = dir.join("ume.ini");
    cx.update(|_, app| app.set_global(UserSettings::from_profile(profile, Some(path))));
}

/// Calls the function of macro `index`, as a key or a menu does.
fn call(f: &Fixture, cx: &mut VisualTestContext, index: usize) -> bool {
    let code = function_of_index(index).unwrap();
    let ran = f.view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    cx.run_until_parked();
    ran
}

fn lines(f: &Fixture, cx: &mut VisualTestContext) -> Vec<String> {
    f.lines(cx)
}

#[gpui::test]
fn a_registered_macro_runs_by_its_function_code(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.mac", "Char(65);\n");
    let (f, cx) = setup(cx, "");
    settings(
        cx,
        dir.path(),
        "Name[002]=挿入\nFile[002]=a.mac\nReloadWhenExecute[002]=1\n",
    );
    assert!(call(&f, cx, 2));
    assert_eq!(lines(&f, cx), ["A"]);
    // The function is 31600 + 2.
    assert_eq!(function_of_index(2), Some(31602));
    assert!(!f.running(cx));
}

#[gpui::test]
fn a_file_with_a_folder_of_its_own_is_run_as_it_is(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let file = write(other.path(), "abs.mac", "Char(66);\n");
    let (f, cx) = setup(cx, "");
    settings(
        cx,
        dir.path(),
        &format!("Name[000]=\nFile[000]={}\n", file.display()),
    );
    assert!(call(&f, cx, 0));
    assert_eq!(lines(&f, cx), ["B"]);
}

#[gpui::test]
fn a_macro_that_cannot_run_is_told_with_its_number_and_file(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "");
    settings(cx, dir.path(), "Name[001]=x\nFile[001]=missing.mac\n");
    // Not registered: no file.
    assert!(!call(&f, cx, 5));
    assert!(
        f.output(cx).contains("マクロ 5 () の実行に失敗しました。"),
        "{}",
        f.output(cx)
    );
    // Registered, but the file is not there.
    assert!(!call(&f, cx, 1));
    assert!(
        f.output(cx)
            .contains("マクロ 1 (missing.mac) の実行に失敗しました。"),
        "{}",
        f.output(cx)
    );
    assert!(lines(&f, cx).is_empty());
}

#[gpui::test]
fn a_macro_is_read_again_only_when_the_page_says_so(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "keep.mac", "Char(65);\n");
    write(dir.path(), "again.mac", "Char(65);\n");
    let (f, cx) = setup(cx, "");
    settings(
        cx,
        dir.path(),
        "Name[000]=\nFile[000]=keep.mac\nReloadWhenExecute[000]=0\n\
         Name[001]=\nFile[001]=again.mac\nReloadWhenExecute[001]=1\n",
    );
    assert!(call(&f, cx, 0) && call(&f, cx, 1));
    assert_eq!(lines(&f, cx), ["AA"]);
    write(dir.path(), "keep.mac", "Char(66);\n");
    write(dir.path(), "again.mac", "Char(66);\n");
    // The macro that is not read again runs what it read the first time.
    assert!(call(&f, cx, 0));
    assert_eq!(lines(&f, cx), ["AAA"]);
    assert!(call(&f, cx, 1));
    assert_eq!(lines(&f, cx), ["AAAB"]);
    // Another file in the slot is read.
    write(dir.path(), "other.mac", "Char(67);\n");
    settings(
        cx,
        dir.path(),
        "Name[000]=\nFile[000]=other.mac\nReloadWhenExecute[000]=0\n",
    );
    assert!(call(&f, cx, 0));
    assert_eq!(lines(&f, cx), ["AAABC"]);
}

#[gpui::test]
fn a_registered_macro_called_while_recording_is_recorded_as_one_call(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.mac", "Char(65);\n");
    let saved = dir.path().join("saved.mac");
    let (f, cx) = setup(cx, "");
    settings(cx, dir.path(), "Name[000]=\nFile[000]=a.mac\n");
    cx.simulate_keystrokes("ctrl-shift-m");
    assert!(call(&f, cx, 0));
    assert_eq!(lines(&f, cx), ["A"]);
    cx.simulate_keystrokes("ctrl-m");
    let chosen = saved.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    let text = std::fs::read_to_string(&saved).unwrap();
    let calls: Vec<&str> = text
        .trim_start_matches('\u{feff}')
        .lines()
        .filter(|l| !l.starts_with("//") && !l.trim().is_empty())
        .collect();
    // One call of the file, not the `Char` it made.
    assert_eq!(calls.len(), 1, "{text}");
    assert!(
        calls[0].starts_with("ExecExternalMacro(") && calls[0].contains("a.mac"),
        "{text}"
    );
}

#[test]
fn every_slot_can_be_called_by_a_button_or_an_item_even_when_it_has_no_file() {
    // Sakura does not gray out an empty slot (`IsFuncEnable`); the call tells that it failed.
    assert!(crate::bars::is_available(31600));
    assert!(crate::bars::is_available(31649));
    assert!(!crate::bars::is_available(31650));
}

// ---- the names ----

#[test]
fn the_title_is_the_name_or_the_file_and_an_empty_slot_says_so() {
    let mut table = RegisteredMacros::default();
    table.macros[0].file = "a.mac".into();
    table.macros[1].file = "b.mac".into();
    table.macros[1].name = "名前".into();
    assert_eq!(
        super::super::title_of(&table, 31600).as_deref(),
        Some("a.mac")
    );
    assert_eq!(
        super::super::title_of(&table, 31601).as_deref(),
        Some("名前")
    );
    assert_eq!(
        super::super::title_of(&table, 31602).as_deref(),
        Some("マクロ 2 (未登録)")
    );
    // Not a registered macro.
    assert_eq!(super::super::title_of(&table, 30101), None);
    assert_eq!(super::super::title_of(&table, 31650), None);
}

#[test]
fn the_main_menu_lists_the_macros_that_have_a_file() {
    let mut table = RegisteredMacros::default();
    table.macros[3].file = "c.mac".into();
    table.macros[7].name = "七".into();
    table.macros[7].file = "g.mac".into();
    table.macros[8].name = "no file".into();
    let list = super::super::MacroList(table);
    let items = list.items(code_of("F_USERMACRO_LIST"));
    let shown: Vec<(&str, &MenuAction)> = items
        .iter()
        .map(|i| (i.label.as_str(), &i.action))
        .collect();
    assert_eq!(
        shown,
        [
            ("c.mac", &MenuAction::Function(31603)),
            ("七", &MenuAction::Function(31607)),
        ]
    );
    assert!(items.iter().all(|i| i.enabled));
    assert!(list.items(code_of("F_CUSTMENU_LIST")).is_empty());
}

// ---- the macros that run by themselves ----

/// Macros 0, 1 and 2 put O, T and S at the caret; they are オープン後, タイプ変更後 and 保存前.
fn auto_macros(cx: &mut VisualTestContext, dir: &Path, which: &str) {
    write(dir, "o.mac", "Char(79);\n");
    write(dir, "t.mac", "Char(84);\n");
    write(dir, "s.mac", "Char(83);\n");
    settings(
        cx,
        dir,
        &format!(
            "Name[000]=\nFile[000]=o.mac\nName[001]=\nFile[001]=t.mac\nName[002]=\nFile[002]=s.mac\n{which}"
        ),
    );
}

#[gpui::test]
fn the_macro_of_opening_runs_after_the_type_is_chosen_and_the_file_is_open(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "x.txt", "abc");
    let (f, cx) = setup(cx, "");
    auto_macros(cx, dir.path(), "nMacroOnOpened=0\nnMacroOnTypeChanged=1\n");
    f.run(
        cx,
        code_of("F_FILEOPEN"),
        &[Arg::Text(file.display().to_string())],
    );
    cx.run_until_parked();
    // タイプ変更後 (T) comes first, then オープン後 (O); the caret is at the top of the file.
    assert_eq!(lines(&f, cx), ["TOabc"]);
}

#[gpui::test]
fn the_macro_of_opening_runs_in_a_new_untitled_window_too(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (_f, cx) = setup(cx, "");
    auto_macros(cx, dir.path(), "nMacroOnOpened=0\n");
    cx.update(|_, app| gpui_component::init(app));
    // The start of the application, or `F_FILENEW` in a new window: an untitled window opens.
    let before = cx.update(|_, app| crate::file_ops::windows::editors(app).len());
    cx.update(|_, app| crate::file_ops::windows::open_from_outside(app, Vec::new()))
        .unwrap();
    cx.run_until_parked();
    let views = cx.update(|_, app| crate::file_ops::windows::editors(app));
    assert_eq!(views.len(), before + 1);
    let (_, view) = views.last().unwrap();
    let text = view.read_with(cx, |view, app| {
        view.document()
            .read(app)
            .doc()
            .line(0)
            .unwrap()
            .unwrap()
            .text
    });
    assert_eq!(text, "O");
}

#[gpui::test]
fn the_macro_of_the_type_runs_when_the_type_is_changed(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "");
    auto_macros(cx, dir.path(), "nMacroOnOpened=0\nnMacroOnTypeChanged=1\n");
    f.run(cx, code_of("F_CHANGETYPE"), &[Arg::Int(2)]);
    cx.run_until_parked();
    assert_eq!(lines(&f, cx), ["T"]);
}

#[gpui::test]
fn the_macro_of_saving_runs_before_the_file_is_written(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "x.txt", "abc");
    let (f, cx) = setup(cx, "");
    auto_macros(cx, dir.path(), "nMacroOnSave=2\n");
    f.run(
        cx,
        code_of("F_FILEOPEN"),
        &[Arg::Text(file.display().to_string())],
    );
    cx.run_until_parked();
    assert_eq!(lines(&f, cx), ["abc"], "no macro of opening");
    cx.simulate_input("x");
    f.run(cx, code_of("F_FILESAVE"), &[]);
    cx.run_until_parked();
    // The macro put its S in the text, and the file has it.
    assert_eq!(std::fs::read(&file).unwrap(), b"xSabc");
}

#[gpui::test]
fn the_save_waits_for_a_macro_of_saving_that_waits_for_a_dialog(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "x.txt", "abc");
    write(dir.path(), "w.mac", "TypeList();\nChar(83);\n");
    let (f, cx) = setup(cx, "");
    settings(
        cx,
        dir.path(),
        "Name[000]=\nFile[000]=w.mac\nnMacroOnSave=0\n",
    );
    f.run(
        cx,
        code_of("F_FILEOPEN"),
        &[Arg::Text(file.display().to_string())],
    );
    cx.run_until_parked();
    cx.simulate_input("x");
    f.run(cx, code_of("F_FILESAVE"), &[]);
    cx.run_until_parked();
    // The macro waits for the dialog of the type list: nothing is written yet.
    assert!(f.view.read_with(cx, |v, _| v.doc_type.list.is_some()));
    assert_eq!(std::fs::read(&file).unwrap(), b"abc");
    f.view.update(cx, |v, cx| {
        v.doc_type.list = None;
        cx.notify();
    });
    cx.executor().allow_parking();
    for _ in 0..20 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(100));
        cx.run_until_parked();
    }
    assert_eq!(std::fs::read(&file).unwrap(), b"xSabc");
}

#[gpui::test]
fn the_macro_after_the_macro_of_opening_waits_for_it(cx: &mut TestAppContext) {
    // The start of an untitled window: `-M=` must not be refused because the hook still runs.
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "w.mac", "TypeList();\nChar(66);\n");
    let (f, cx) = setup(cx, "");
    settings(
        cx,
        dir.path(),
        "Name[000]=\nFile[000]=w.mac\nnMacroOnOpened=0\n",
    );
    let ran = std::rc::Rc::new(std::cell::Cell::new(false));
    let flag = ran.clone();
    let (window, view) = (cx.windows()[0], f.view.clone());
    // After the update of the window ends, as the real call is made.
    cx.update(|_, app| {
        app.defer(move |app| {
            super::super::run_auto_in(window, &view, super::super::Auto::Opened, app, move |_| {
                flag.set(true)
            })
        })
    });
    cx.run_until_parked();
    assert!(f.view.read_with(cx, |v, _| v.doc_type.list.is_some()));
    assert!(!ran.get(), "it waits for the dialog of the hook");
    f.view.update(cx, |v, cx| {
        v.doc_type.list = None;
        cx.notify();
    });
    cx.executor().allow_parking();
    for _ in 0..20 {
        cx.executor()
            .advance_clock(std::time::Duration::from_millis(100));
        cx.run_until_parked();
    }
    assert!(ran.get());
    assert_eq!(lines(&f, cx), ["B"]);
}

#[gpui::test]
fn the_questions_of_the_save_are_asked_again_when_the_macro_changed_the_text(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "x.txt", "abc");
    // An emoji: Shift_JIS cannot hold it.
    write(dir.path(), "e.mac", "Char(128512);\n");
    let (f, cx) = setup(cx, "");
    settings(
        cx,
        dir.path(),
        "Name[000]=\nFile[000]=e.mac\nnMacroOnSave=0\n",
    );
    f.run(
        cx,
        code_of("F_FILEOPEN"),
        &[Arg::Text(file.display().to_string()), Arg::Int(0)],
    );
    cx.run_until_parked();
    cx.simulate_input("x");
    f.run(cx, code_of("F_FILESAVE"), &[]);
    cx.run_until_parked();
    assert!(
        cx.has_pending_prompt(),
        "the emoji the macro added is asked about"
    );
    cx.simulate_prompt_answer("続行(Y)");
    cx.run_until_parked();
    assert_eq!(std::fs::read(&file).unwrap(), b"x?abc");
}

#[gpui::test]
fn closing_the_file_runs_the_macro_of_opening_for_the_untitled_document(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "x.txt", "abc");
    let (f, cx) = setup(cx, "");
    auto_macros(cx, dir.path(), "");
    f.run(
        cx,
        code_of("F_FILEOPEN"),
        &[Arg::Text(file.display().to_string())],
    );
    cx.run_until_parked();
    // The macro is set after the file is open, so that only the close runs it.
    auto_macros(cx, dir.path(), "nMacroOnOpened=0\n");
    f.run(cx, code_of("F_FILECLOSE"), &[]);
    cx.run_until_parked();
    assert_eq!(lines(&f, cx), ["O"]);
}

#[gpui::test]
fn saving_under_a_name_runs_the_macro_of_opening_but_a_plain_save_does_not(
    cx: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().unwrap();
    let other = dir.path().join("other.txt");
    let (f, cx) = setup(cx, "abc");
    auto_macros(cx, dir.path(), "nMacroOnOpened=0\n");
    f.run(
        cx,
        code_of("F_FILESAVEAS"),
        &[Arg::Text(other.display().to_string())],
    );
    cx.run_until_parked();
    assert_eq!(lines(&f, cx), ["Oabc"]);
    f.run(cx, code_of("F_FILESAVE"), &[]);
    cx.run_until_parked();
    assert_eq!(lines(&f, cx), ["Oabc"], "a plain save runs nothing");
}

#[gpui::test]
fn no_macro_runs_by_itself_when_none_is_named_or_the_slot_is_empty(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = write(dir.path(), "x.txt", "abc");
    let (f, cx) = setup(cx, "");
    // 9 has no file; -1 is none.
    settings(
        cx,
        dir.path(),
        "nMacroOnOpened=9\nnMacroOnTypeChanged=-1\nnMacroOnSave=9\n",
    );
    f.run(
        cx,
        code_of("F_FILEOPEN"),
        &[Arg::Text(file.display().to_string())],
    );
    cx.run_until_parked();
    cx.simulate_input("x");
    f.run(cx, code_of("F_FILESAVE"), &[]);
    cx.run_until_parked();
    assert_eq!(std::fs::read(&file).unwrap(), b"xabc");
    assert_eq!(f.output(cx), "");
}

#[gpui::test]
fn shift_held_skips_the_macros_that_run_by_themselves(cx: &mut TestAppContext) {
    // `RunAutoMacro`: not run while Shift is held.
    let dir = tempfile::tempdir().unwrap();
    let (f, cx) = setup(cx, "");
    auto_macros(cx, dir.path(), "nMacroOnOpened=0\n");
    cx.simulate_modifiers_change(gpui::Modifiers::shift());
    let view = f.view.clone();
    view.update_in(cx, |view, window, cx| {
        view.run_auto_macro(super::super::Auto::Opened, None, window, cx);
    });
    cx.run_until_parked();
    assert!(lines(&f, cx).is_empty());
    cx.simulate_modifiers_change(gpui::Modifiers::default());
    view.update_in(cx, |view, window, cx| {
        view.run_auto_macro(super::super::Auto::Opened, None, window, cx);
    });
    cx.run_until_parked();
    assert_eq!(lines(&f, cx), ["O"]);
}

#[gpui::test]
fn the_macro_of_saving_is_told_the_path_of_the_save(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    // `GetSaveFilename` answers the file the save will write while the macro runs.
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "p.cjs",
        "Editor.GoFileTop();\nEditor.InsText(Editor.GetSaveFilename() + '|');\n",
    );
    let (f, cx) = setup(cx, "text");
    settings(
        cx,
        dir.path(),
        "Name[000]=\nFile[000]=p.cjs\nnMacroOnSave=0\n",
    );
    let target = PathBuf::from(dir.path()).join("saved-as.txt");
    let view = f.view.clone();
    let path = target.clone();
    view.update_in(cx, |view, window, cx| {
        view.run_auto_macro(super::super::Auto::BeforeSave, Some(&path), window, cx);
    });
    f.wait_until_done(cx);
    let first = lines(&f, cx).remove(0);
    assert_eq!(first, format!("{}|text", target.display()));
    // After the macro, it is forgotten: a macro that starts later is not told the old path.
    let kept = view.update(cx, |view, _| view.macros.registered.save_path().is_some());
    assert!(!kept);
}
