//! Tests of the `SmartIndent` jack on a window, with the real `matsu-script` (skipped when that program
//! is not built next to the tests). The handler of the plugin writes the char it was called for as
//! `[<char>]` in the output panel, so the panel shows which chars called it.

use std::fs;
use std::time::Duration;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::input_options::SmartIndent;
use matsu_commands::plugin_commands;
use matsu_commands::sakura::FUNCTIONS;
use matsu_config::Profile;
use matsu_config::plugins::consent::{ConsentFile, fingerprint};
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginRec, PluginSettings, PluginState, SearchDirs};

use crate::document::SharedDocument;
use crate::macros::plugin_call::tests::script_host;
use crate::macros::plugin_call::{set_test_interpreter, shutdown};
use crate::settings::UserSettings;
use crate::view::EditorView;

const ID: &str = "test.indent";
const FOLDER: &str = "Indent";

/// The function code of the plug `SmartIndent` of the plugin in the first slot.
const CODE: u16 = plugin_commands::FIRST;

/// A folder with the settings file and the plugin `Indent`, whose `SmartIndent` handler is `script`.
struct Place {
    dir: tempfile::TempDir,
}

impl Place {
    fn new(resident: bool, script: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let plugin = dir.path().join("plugins").join(FOLDER);
        fs::create_dir_all(&plugin).unwrap();
        let def = format!(
            "[Plugin]\nId={ID}\nName=Indent\nType=wsh\n\n[Plug]\nSmartIndent=indent.js\n\
             SmartIndent.Label=Test indent\n\n[Wsh]\nUseCache={}\n\n[Matsu]\nSmartIndent.Chars=}}\n",
            u8::from(resident)
        );
        fs::write(plugin.join("plugin.def"), def).unwrap();
        fs::write(plugin.join("indent.js"), script).unwrap();
        Self { dir }
    }

    fn settings(&self) -> std::path::PathBuf {
        self.dir.path().join("matsu.ini")
    }

    fn dirs(&self) -> SearchDirs {
        SearchDirs::new(&self.settings(), None)
    }

    /// The user turned plugins on and agreed to this one.
    fn enable(&self, cx: &mut VisualTestContext, enabled: bool) {
        let dirs = self.dirs();
        let def = dirs.load_def(FOLDER, DEFAULT_LANGUAGE).unwrap();
        let mut settings = PluginSettings {
            enabled,
            ..PluginSettings::default()
        };
        settings.table[0] = PluginRec {
            id: def.id.clone(),
            name: FOLDER.into(),
            cmd_num: 0,
            state: PluginState::Stopped,
        };
        let mut profile = Profile::new();
        settings.store(&mut profile).unwrap();
        profile.save(&self.settings()).unwrap();
        let files = fingerprint(&dirs.plugin_dir(FOLDER).unwrap()).unwrap();
        let mut consent = ConsentFile::default();
        consent
            .enable(&def.id, FOLDER, &files, "2026-11-02")
            .unwrap();
        consent.save(&dirs.consent_file()).unwrap();
        let user = UserSettings::from_profile(profile, Some(self.settings()));
        cx.update(|_, app| app.set_global(user));
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<EditorView>, &mut VisualTestContext) {
    cx.update(gpui_component::init);
    let shared = cx.new(|_| SharedDocument::empty());
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(shared, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.executor().allow_parking();
    (view, vcx)
}

/// A window whose type has the plug as its smart indent, with the plugin of `place` agreed to.
fn window<'a>(
    cx: &'a mut TestAppContext,
    place: &Place,
    enabled: bool,
) -> Option<(Entity<EditorView>, &'a mut VisualTestContext)> {
    let host = script_host()?;
    set_test_interpreter(Some(host));
    let (view, cx) = setup(cx);
    place.enable(cx, enabled);
    view.update(cx, |view, _| {
        view.prefs.settings.input.smart_indent = SmartIndent::Plugin(CODE);
    });
    Some((view, cx))
}

fn finish(cx: &mut VisualTestContext) {
    set_test_interpreter(None);
    cx.update(|_, app| shutdown(app, ID));
}

fn type_char(view: &Entity<EditorView>, cx: &mut VisualTestContext, c: char) {
    view.update(cx, |view, cx| view.type_char(c, cx));
    cx.run_until_parked();
}

fn run(view: &Entity<EditorView>, cx: &mut VisualTestContext, function: &str) {
    let code = FUNCTIONS.iter().find(|f| f.name == function).unwrap().code;
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    cx.run_until_parked();
}

/// The lines of the document, without their line endings.
fn lines(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Vec<String> {
    view.read_with(cx, |view, app| {
        let doc = view.document().read(app).doc();
        (0..doc.line_count())
            .map(|i| doc.line(i).unwrap().map(|l| l.text).unwrap_or_default())
            .collect()
    })
}

/// The text of the document with its line endings as `\n`.
fn text(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, app| {
        let doc = view.document().read(app).doc();
        (0..doc.line_count())
            .map(|i| {
                let line = doc.line(i).unwrap().unwrap();
                format!(
                    "{}{}",
                    line.text,
                    if line.eol.is_some() { "\n" } else { "" }
                )
            })
            .collect()
    })
}

/// The chars the handler was called for, in order, from the lines `[<char>]` of the output panel
/// (`CR` for the Enter key).
fn called(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Vec<String> {
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    output
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix('[')?
                .strip_suffix(']')
                .map(str::to_owned)
        })
        .collect()
}

/// Lets the window and the plugin process run until `done` is true.
fn wait_until(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
    what: &str,
    done: impl Fn(&Entity<EditorView>, &mut VisualTestContext) -> bool,
) {
    for _ in 0..3000 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if done(view, cx) {
            // A little more time: nothing else may follow.
            for _ in 0..10 {
                cx.executor().advance_clock(Duration::from_millis(20));
                cx.run_until_parked();
                std::thread::sleep(Duration::from_millis(5));
            }
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "{what}: did not happen\n{:?}\n{}",
        lines(view, cx),
        view.read_with(cx, |v, _| v.macro_output_text())
    );
}

/// Waits the time a call would take to run and be seen, and no more is called.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..60 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(5));
    }
}

const LOG: &str = "const c = Indent.GetChar(); console.log('[' + (c === '\\r' ? 'CR' : c) + ']');";

#[gpui::test]
fn the_plugin_is_called_for_a_line_break_and_the_declared_chars_only(cx: &mut TestAppContext) {
    let place = Place::new(true, LOG);
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("ab{");
    settle(cx);
    assert!(called(&view, cx).is_empty(), "{:?}", called(&view, cx));
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the call for the line break", |v, cx| {
        called(v, cx).len() == 1
    });
    assert_eq!(called(&view, cx), ["CR"]);
    // `}` is declared in `[Matsu] SmartIndent.Chars`; the other chars are not.
    cx.simulate_input("x}");
    wait_until(&view, cx, "the call for }", |v, cx| {
        called(v, cx).len() == 2
    });
    cx.simulate_input("y;)");
    settle(cx);
    assert_eq!(called(&view, cx), ["CR", "}"]);
    assert_eq!(lines(&view, cx), ["ab{", "x}y;)"]);
    finish(cx);
}

#[gpui::test]
fn the_result_of_the_plugin_is_an_undo_step_of_its_own(cx: &mut TestAppContext) {
    let place = Place::new(true, "Editor.InsText('    ');");
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x{");
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the indentation of the plugin", |v, cx| {
        lines(v, cx) == ["x{", "    "]
    });
    // One undo takes the plugin's part only; the line break was typed before it.
    run(&view, cx, "F_UNDO");
    assert_eq!(text(&view, cx), "x{\n");
    run(&view, cx, "F_UNDO");
    assert_eq!(text(&view, cx), "x{");
    run(&view, cx, "F_REDO");
    run(&view, cx, "F_REDO");
    assert_eq!(text(&view, cx), "x{\n    ");
    finish(cx);
}

#[gpui::test]
fn a_plugin_that_is_too_slow_inserts_nothing_and_the_input_is_not_lost(cx: &mut TestAppContext) {
    // The handler never ends: the limit of the jack (0.5 seconds) stops it.
    let place = Place::new(true, "console.log('[start]'); while (true) {}");
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x{");
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the handler to start", |v, cx| {
        called(v, cx) == ["start"]
    });
    // Nothing waits for the handler: the window is not held, and the time limit ends it.
    assert_eq!(text(&view, cx), "x{\n");
    wait_until(&view, cx, "the time limit", |v, cx| {
        v.read_with(cx, |v, _| v.macro_output_text())
            .contains("時間切れ")
    });
    assert_eq!(text(&view, cx), "x{\n");
    // The window is free again: the input goes on, and the next line break starts the plugin again.
    cx.simulate_input("uv");
    assert_eq!(lines(&view, cx), ["x{", "uv"]);
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the handler to start again", |v, cx| {
        called(v, cx) == ["start", "start"]
    });
    finish(cx);
}

#[gpui::test]
fn the_input_while_the_plugin_works_is_kept_and_the_plugin_stops(cx: &mut TestAppContext) {
    let place = Place::new(true, "console.log('[start]'); while (true) {}");
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x{");
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the handler to start", |v, cx| {
        called(v, cx) == ["start"]
    });
    // The user goes on typing while the handler runs: every char is in the text, and the call ends
    // with no word, long before the time limit.
    cx.simulate_input("uv");
    assert_eq!(lines(&view, cx), ["x{", "uv"]);
    wait_until(&view, cx, "the end of the call", |v, cx| {
        !v.read_with(cx, |v, app| v.plugin_in_call(app))
    });
    cx.simulate_input("w");
    assert_eq!(lines(&view, cx), ["x{", "uvw"]);
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(
        !output.contains("時間切れ") && !output.contains("中断"),
        "{output}"
    );
    finish(cx);
}

#[gpui::test]
fn a_plugin_is_stopped_when_the_user_typed_on_and_adds_nothing(cx: &mut TestAppContext) {
    // It asks again and again, and inserts only at the end: it never gets there, because the first
    // request after the user typed is refused (or, on a machine so slow that the user is late by more
    // than the limit of the jack, the time limit ends it). The order does not depend on how long
    // anything takes: a request is answered on the window, one at a time, so it comes before or after
    // the typing, and an insertion is never answered after it.
    let place = Place::new(
        true,
        "for (let i = 0; i < 1000000; i++) Editor.GetLineCount(0); \
         Editor.InsText('PLUGIN'); console.log('[inserted]');",
    );
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x");
    type_char(&view, cx, '\r');
    cx.simulate_input("u");
    settle(cx);
    for _ in 0..40 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(15));
    }
    assert_eq!(lines(&view, cx), ["x", "u"]);
    assert!(called(&view, cx).is_empty());
    finish(cx);
}

#[gpui::test]
fn a_plugin_is_stopped_when_the_user_selected_the_text_and_replaces_nothing(
    cx: &mut TestAppContext,
) {
    // The caret does not move and the text does not change, but the selection does: an insertion
    // would replace the selected text. The handler asks again and again and inserts only at the end,
    // so it cannot get past the user whatever the speed of the machine (see the test above).
    let place = Place::new(
        true,
        "console.log('[start]'); for (let i = 0; i < 1000000; i++) Editor.GetLineCount(0); \
         Editor.InsText('PLUGIN'); console.log('[inserted]');",
    );
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x");
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the handler to start", |v, cx| {
        called(v, cx) == ["start"]
    });
    run(&view, cx, "F_SELECTALL");
    for _ in 0..60 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(text(&view, cx), "x\n");
    assert_eq!(called(&view, cx), ["start"]);
    finish(cx);
}

#[gpui::test]
fn what_the_user_types_while_the_plugin_works_is_not_in_its_undo_step(cx: &mut TestAppContext) {
    // The handler inserts and then goes on working; the user types meanwhile.
    let place = Place::new(
        true,
        "Editor.InsText('    '); console.log('[inserted]'); \
         const t = Date.now(); while (Date.now() - t < 400) {}",
    );
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x");
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the insertion of the plugin", |v, cx| {
        called(v, cx) == ["inserted"]
    });
    cx.simulate_input("u");
    wait_until(&view, cx, "the end of the call", |v, cx| {
        !v.read_with(cx, |v, app| v.plugin_in_call(app))
    });
    assert_eq!(text(&view, cx), "x\n    u");
    // The first Undo takes what the user typed, the second the plugin's indentation.
    run(&view, cx, "F_UNDO");
    assert_eq!(text(&view, cx), "x\n    ");
    run(&view, cx, "F_UNDO");
    assert_eq!(text(&view, cx), "x\n");
    finish(cx);
}

#[gpui::test]
fn work_of_the_window_that_is_not_the_users_does_not_stop_the_plugin(cx: &mut TestAppContext) {
    // The marks of the outline and of the diff are set through a context of the view, in the middle of
    // a call: neither the text nor the caret changes, and the plugin goes on.
    let place = Place::new(
        true,
        "console.log('[start]'); const t = Date.now(); while (Date.now() - t < 150) {} \
         Editor.InsText('    ');",
    );
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x");
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the handler to start", |v, cx| {
        called(v, cx) == ["start"]
    });
    view.update(cx, |view, cx| view.run_with(cx, |_| ()));
    wait_until(&view, cx, "the indentation of the plugin", |v, cx| {
        text(v, cx) == "x\n    "
    });
    finish(cx);
}

#[gpui::test]
fn a_mouse_drop_while_the_plugin_works_is_not_in_its_undo_step(cx: &mut TestAppContext) {
    use gpui::{Modifiers, MouseButton, MouseDownEvent, MouseUpEvent, point, px};
    use matsu_core::buffer::Pos;
    let place = Place::new(
        true,
        "Editor.InsText('    '); console.log('[inserted]'); \
         const t = Date.now(); while (Date.now() - t < 600) {}",
    );
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("abcdef ghi");
    // the window has to be painted for the mouse to hit the text
    cx.run_until_parked();
    type_char(&view, cx, '\r');
    wait_until(&view, cx, "the insertion of the plugin", |v, cx| {
        called(v, cx) == ["inserted"]
    });
    // A selection that the plugin's place follows (it is the user's, but not yet a change the plugin
    // has seen), so that only the drop ends the call.
    view.update(cx, |v, cx| {
        v.run_with(cx, |c| c.select_range(Pos::new(0, 0), Pos::new(0, 3)))
            .unwrap();
        v.indent_guard_follow(cx);
    });
    let at = |view: &Entity<EditorView>, cx: &mut VisualTestContext, col: usize| {
        view.read_with(cx, |v, _| {
            let paint = v.paint.expect("the window was painted");
            let m = v.metrics;
            point(
                px(paint.origin_x + (col as f32 + 0.1) * m.cell_width),
                px(paint.origin_y + 0.5 * m.line_height - v.scroll_y),
            )
        })
    };
    let (inside, target) = (at(&view, cx, 1), at(&view, cx, 8));
    cx.simulate_event(MouseDownEvent {
        button: MouseButton::Left,
        position: inside,
        modifiers: Modifiers::default(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_mouse_move(target, MouseButton::Left, Modifiers::default());
    cx.simulate_event(MouseUpEvent {
        button: MouseButton::Left,
        position: target,
        modifiers: Modifiers::default(),
        click_count: 1,
    });
    assert_eq!(lines(&view, cx)[0], "def gabchi");
    wait_until(&view, cx, "the end of the call", |v, cx| {
        !v.read_with(cx, |v, app| v.plugin_in_call(app))
    });
    // One Undo takes the drop only; the plugin's indentation stays.
    run(&view, cx, "F_UNDO");
    assert_eq!(text(&view, cx), "abcdef ghi\n    ");
    finish(cx);
}

#[gpui::test]
fn a_plugin_that_is_not_resident_is_not_called(cx: &mut TestAppContext) {
    let place = Place::new(false, LOG);
    let Some((view, cx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x{");
    type_char(&view, cx, '\r');
    settle(cx);
    assert!(called(&view, cx).is_empty());
    assert_eq!(text(&view, cx), "x{\n");
    finish(cx);
}

#[gpui::test]
fn a_plugin_that_is_off_is_not_called_and_the_type_keeps_its_value(cx: &mut TestAppContext) {
    let place = Place::new(true, LOG);
    let Some((view, cx)) = window(cx, &place, false) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    cx.simulate_input("x{");
    type_char(&view, cx, '\r');
    settle(cx);
    assert!(called(&view, cx).is_empty());
    assert_eq!(text(&view, cx), "x{\n");
    assert_eq!(
        view.read_with(cx, |v, _| v.prefs.settings.input.smart_indent),
        SmartIndent::Plugin(CODE)
    );
    finish(cx);
}

/// The list of the smart indent choices as the page shows it, made again from the disk.
fn smart_indents(cx: &mut VisualTestContext, now: i32) -> Vec<(String, i32)> {
    crate::type_settings::forget_smart_indent_plugs();
    cx.update(|_, app| crate::type_settings::smart_indent_types(app, now))
}

fn builtin() -> Vec<(String, i32)> {
    vec![("なし".to_owned(), 0), ("C/C++".to_owned(), 1)]
}

#[gpui::test]
fn the_type_settings_list_the_plugs_of_the_plugins_that_can_run(cx: &mut TestAppContext) {
    let place = Place::new(true, LOG);
    let Some((_view, vcx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let mut with_plug = builtin();
    with_plug.push(("Test indent".to_owned(), i32::from(CODE)));
    assert_eq!(smart_indents(vcx, 0), with_plug);
    assert_eq!(smart_indents(vcx, i32::from(CODE)), with_plug);
    // Not agreed to any more: not listed, and a type that has it shows it as a plugin that cannot be used.
    fs::remove_file(place.dirs().consent_file()).unwrap();
    assert_eq!(smart_indents(vcx, 0), builtin());
    let mut gone = builtin();
    gone.push(("（使えないプラグイン 20000）".to_owned(), 20_000));
    assert_eq!(smart_indents(vcx, i32::from(CODE)), gone);
    // Plugins off.
    place.enable(vcx, false);
    assert_eq!(smart_indents(vcx, 0), builtin());
    assert_eq!(smart_indents(vcx, i32::from(CODE)), gone);
    finish(vcx);
}

#[gpui::test]
fn a_plugin_that_is_not_resident_is_listed_with_the_reason(cx: &mut TestAppContext) {
    let place = Place::new(false, LOG);
    let Some((_view, vcx)) = window(cx, &place, true) else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let list = smart_indents(vcx, 0);
    assert_eq!(list.len(), 3);
    assert!(list[2].0.starts_with("Test indent"));
    assert!(list[2].0.contains("UseCache=1"), "{:?}", list[2].0);
    finish(vcx);
}

#[test]
fn only_a_line_break_and_the_declared_chars_are_wanted() {
    let def = |text: &str| matsu_config::plugins::PluginDef::from_bytes(text.as_bytes(), None);
    let plain = def("[Plugin]\n");
    // a line break is wanted whatever char the command took for it (NEL, LS and PS too)
    for c in ['\r', '\n', '\u{85}', '\u{2028}'] {
        assert!(super::wants(&plain, c, true), "{c:?}");
    }
    assert!(!super::wants(&plain, '}', false) && !super::wants(&plain, 'a', false));
    // a char that is not a line break, such as NEL while the document does not count it
    assert!(!super::wants(&plain, '\u{85}', false));
    let declared = def("[Plugin]\n[Matsu]\nSmartIndent.Chars=}{;\n");
    for c in ['}', '{', ';'] {
        assert!(super::wants(&declared, c, false), "{c:?}");
    }
    assert!(!super::wants(&declared, ')', false) && !super::wants(&declared, ' ', false));
}
