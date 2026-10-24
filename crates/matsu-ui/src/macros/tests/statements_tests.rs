//! The statements of a key macro that wait or keep a flag: `Sleep`, `SetDrawSwitch` and the message
//! boxes (`CMacro::HandleCommand` and `HandleFunction` in `sakura_core/macro/CMacro.cpp`), and the
//! second argument of `ExecExtMacro` (the code of a macro instead of a file).

use super::*;

fn waiting(f: &Fixture, cx: &mut VisualTestContext) -> bool {
    f.view.read_with(cx, |v, _| v.macros.waiting.is_some())
}

fn draw_switch(f: &Fixture, cx: &mut VisualTestContext) -> bool {
    f.view.read_with(cx, |v, _| v.macros.host_state.draw_switch)
}

/// Lets the window and the timers go on a little, with the clock moved by `millis`.
fn pass(cx: &mut VisualTestContext, millis: u64) {
    cx.executor().advance_clock(Duration::from_millis(millis));
    for _ in 0..4 {
        cx.executor().advance_clock(Duration::from_millis(50));
        cx.run_until_parked();
    }
}

fn run_mac(f: &Fixture, cx: &mut VisualTestContext, text: &str) {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "s.mac", text);
    f.run(cx, F_EXECEXTMACRO, &[text_arg(&path)]);
    pass(cx, 0);
}

// ---- the message boxes ----

#[gpui::test]
fn a_message_box_holds_the_macro_until_it_is_closed(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    run_mac(&f, cx, "Char(65);\nInfoMsg('hello');\nChar(66);\n");
    assert_eq!(f.lines(cx), ["A"]);
    assert!(cx.has_pending_prompt() && waiting(&f, cx));
    // it does not go on by itself
    pass(cx, 5000);
    assert_eq!(f.lines(cx), ["A"]);
    cx.simulate_prompt_answer("OK");
    pass(cx, 0);
    assert_eq!(f.lines(cx), ["AB"]);
    assert!(!waiting(&f, cx) && !f.running(cx));
}

#[gpui::test]
fn every_message_statement_shows_a_box_and_the_answer_is_dropped(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    // the buttons of each: OK, OK, OK, OK/Cancel, Yes/No, and the type of `MessageBox` (4: Yes/No)
    let boxes: [(&str, &str); 6] = [
        ("ErrorMsg('e')", "OK"),
        ("WarnMsg('w')", "OK"),
        ("InfoMsg('i')", "OK"),
        ("OkCancelBox('q')", "キャンセル"),
        ("YesNoBox('q')", "いいえ"),
        ("MessageBox('m', 4)", "いいえ"),
    ];
    let mac: String = boxes
        .iter()
        .map(|(call, _)| format!("{call};\nChar(46);\n"))
        .collect();
    run_mac(&f, cx, &mac);
    for (i, (call, answer)) in boxes.iter().enumerate() {
        assert!(cx.has_pending_prompt(), "{call}");
        assert_eq!(f.lines(cx).concat(), ".".repeat(i), "{call}");
        cx.simulate_prompt_answer(answer);
        pass(cx, 0);
    }
    // a macro does not stop because a box was cancelled (the value of a statement is dropped)
    assert_eq!(f.lines(cx), ["......"]);
    assert!(!f.running(cx) && !waiting(&f, cx));
}

#[gpui::test]
fn a_message_box_without_a_text_is_skipped(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    run_mac(&f, cx, "ErrorMsg();\nChar(65);\n");
    assert!(!cx.has_pending_prompt());
    assert_eq!(f.lines(cx), ["A"]);
}

#[gpui::test]
fn the_cancel_button_ends_the_wait_for_a_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    run_mac(&f, cx, "InfoMsg('x');\nChar(65);\n");
    assert!(waiting(&f, cx));
    f.view.update(cx, |v, cx| v.cancel_macro(cx));
    assert!(!waiting(&f, cx));
    cx.simulate_prompt_answer("OK");
    pass(cx, 0);
    assert!(f.lines(cx).is_empty());
}

// ---- Sleep ----

#[gpui::test]
fn sleep_holds_the_macro_for_the_time(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    run_mac(&f, cx, "Char(65);\nSleep(1000);\nChar(66);\n");
    assert_eq!(f.lines(cx), ["A"]);
    assert!(waiting(&f, cx));
    pass(cx, 400);
    assert_eq!(f.lines(cx), ["A"], "not yet");
    pass(cx, 1000);
    assert_eq!(f.lines(cx), ["AB"]);
    assert!(!waiting(&f, cx));
}

#[gpui::test]
fn sleep_of_zero_and_a_missing_time_go_on(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    run_mac(&f, cx, "Sleep(0);\nChar(65);\nSleep();\nChar(66);\n");
    pass(cx, 100);
    assert_eq!(f.lines(cx), ["AB"]);
}

#[gpui::test]
fn a_negative_time_stops_the_macro(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    // `VariantChangeType` to an unsigned number fails, and `HandleFunction` returns false
    run_mac(&f, cx, "Sleep(-1);\nChar(65);\n");
    pass(cx, 100);
    assert!(f.lines(cx).is_empty());
    assert!(f.output(cx).contains("止まりました"), "{}", f.output(cx));
    assert!(!waiting(&f, cx));
}

#[gpui::test]
fn the_cancel_button_ends_a_sleep(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    run_mac(&f, cx, "Sleep(1000);\nChar(65);\n");
    assert!(waiting(&f, cx));
    f.view.update(cx, |v, cx| v.cancel_macro(cx));
    pass(cx, 2000);
    assert!(f.lines(cx).is_empty());
    // and the next macro starts at once
    run_mac(&f, cx, "Char(66);\n");
    assert_eq!(f.lines(cx), ["B"]);
}

// ---- SetDrawSwitch ----

#[gpui::test]
fn set_draw_switch_sets_the_flag_that_get_draw_switch_reads(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    assert!(draw_switch(&f, cx));
    run_mac(&f, cx, "SetDrawSwitch(0);\n");
    assert!(!draw_switch(&f, cx));
    run_mac(&f, cx, "SetDrawSwitch(1);\n");
    assert!(draw_switch(&f, cx));
    // `iVal` is the low 16 bits of the number
    run_mac(&f, cx, "SetDrawSwitch(65536);\n");
    assert!(!draw_switch(&f, cx));
    run_mac(&f, cx, "SetDrawSwitch(5);\nChar(65);\n");
    assert!(draw_switch(&f, cx));
    assert_eq!(f.lines(cx), ["A"]);
}

// ---- ExecExtMacro with a type ----

#[gpui::test]
fn exec_ext_macro_runs_the_text_of_a_key_macro(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    f.run(
        cx,
        F_EXECEXTMACRO,
        &[
            Arg::Text("Char(65);\nChar(66);\n".into()),
            Arg::Text("mac".into()),
        ],
    );
    pass(cx, 0);
    assert_eq!(f.lines(cx), ["AB"]);
    // the extension may have a dot and any case
    f.run(
        cx,
        F_EXECEXTMACRO,
        &[Arg::Text("Char(67);".into()), Arg::Text(".MAC".into())],
    );
    pass(cx, 0);
    assert_eq!(f.lines(cx), ["ABC"]);
}

#[gpui::test]
fn a_key_macro_can_name_a_macro_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    // the outer macro goes on after the inner one, as for a macro file
    run_mac(
        &f,
        cx,
        "Char(65);\nExecExternalMacro('Char(66);', 'mac');\nChar(67);\n",
    );
    pass(cx, 100);
    assert_eq!(f.lines(cx), ["ABC"]);
}

#[gpui::test]
fn a_text_of_a_script_runs_through_the_host(cx: &mut TestAppContext) {
    if !available(Language::JavaScript) {
        return;
    }
    let (f, cx) = setup(cx, "");
    f.run(
        cx,
        F_EXECEXTMACRO,
        &[
            Arg::Text("Editor.InsText('js'); console.log('r=ok');".into()),
            Arg::Text("cjs".into()),
        ],
    );
    f.wait_until_done(cx);
    assert_eq!(f.lines(cx), ["js"]);
    assert_eq!(results(&f.output(cx)), ["ok"], "{}", f.output(cx));
    // the file that held the code is gone
    let left = std::fs::read_dir(std::env::temp_dir().join("matsu-macro-text"))
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with(&format!("{}-", std::process::id()))
                })
                .count()
        })
        .unwrap_or(0);
    assert_eq!(left, 0);
}

#[gpui::test]
fn an_unknown_type_or_a_wrong_text_is_told(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    f.run(
        cx,
        F_EXECEXTMACRO,
        &[Arg::Text("x".into()), Arg::Text("nope".into())],
    );
    pass(cx, 0);
    assert!(f.output(cx).contains("nope"), "{}", f.output(cx));
    f.run(
        cx,
        F_EXECEXTMACRO,
        &[Arg::Text("NoSuchFunction(".into()), Arg::Text("mac".into())],
    );
    pass(cx, 0);
    assert!(f.lines(cx).is_empty());
    assert!(!f.running(cx));
}
