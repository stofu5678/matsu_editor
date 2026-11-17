//! The lines that a handler writes to standard error before its answer arrive before the call is
//! finished (the host reads the two streams on two threads). Each test skips when the interpreter
//! is missing.

use std::time::Instant;

use super::*;

/// Each call writes one line to standard error and answers at once. One resident process serves
/// all the calls, so many rounds are cheap, and a line that arrives after its call shows up in
/// the next report (or in none).
const ROUNDS: usize = 300;

fn one_line_per_call(language: Language, name: &str, code: &str) {
    if !available(language) {
        return;
    }
    let dir = plugin_dir(&[(name, code)]);
    let mut host = host(dir.path(), language, true);
    let mut world = World::new("");
    for round in 0..ROUNDS {
        let report = world.call_jack(&mut host, "Command", name, &json!({"n": round}), PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        assert_eq!(
            report.lines(Stream::Stderr),
            [format!("e{round}")],
            "{language:?}, round {round}: {:?}",
            report.output
        );
    }
}

#[test]
fn a_node_handler_stderr_line_comes_before_the_end_of_its_call() {
    one_line_per_call(
        Language::JavaScript,
        "h.cjs",
        "process.stderr.write('e' + Plugin.args.n + '\\n');",
    );
}

#[test]
fn a_python_handler_stderr_line_comes_before_the_end_of_its_call() {
    one_line_per_call(
        Language::Python,
        "h.py",
        "import sys\nprint('e%d' % Plugin.args['n'], file=sys.stderr)\n",
    );
}

#[test]
fn a_ruby_handler_stderr_line_comes_before_the_end_of_its_call() {
    one_line_per_call(
        Language::Ruby,
        "h.rb",
        "$stderr.puts \"e#{Plugin.args['n']}\"\n",
    );
}

#[test]
fn a_handler_that_keeps_writing_to_stderr_after_its_answer_cannot_hold_the_call() {
    if !available(Language::Python) {
        return;
    }
    // A thread goes on writing after the handler has returned and the answer is sent.
    let code = "import sys, threading\nprint(\"before\", file=sys.stderr, flush=True)\ndef flood():\n    while True:\n        print(\"flood\", file=sys.stderr, flush=True)\nthreading.Thread(target=flood, daemon=True).start()\n";
    let dir = plugin_dir(&[("h.py", code)]);
    let mut host = host(dir.path(), Language::Python, false);
    let mut world = World::new("");
    let started = Instant::now();
    let report = world.call(&mut host, "h.py");
    assert_eq!(report.outcome, Outcome::Ok);
    assert_eq!(report.lines(Stream::Stderr)[0], "before");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert!(!host.is_running());
}

#[test]
fn stderr_written_while_no_call_runs_is_still_delivered_with_the_next_call() {
    if !available(Language::JavaScript) {
        return;
    }
    // A resident process writes between two calls; the line is not part of the first call, and
    // the next call still ends normally and brings its own line.
    let code = "
if (Plugin.args.n === 0) { setTimeout(() => process.stderr.write('idle\\n'), 50); }
process.stderr.write('call' + Plugin.args.n + '\\n');";
    let dir = plugin_dir(&[("h.cjs", code)]);
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    let first = world.call_jack(&mut host, "Command", "h.cjs", &json!({"n": 0}), PATIENT);
    assert_eq!(first.outcome, Outcome::Ok);
    assert_eq!(first.lines(Stream::Stderr)[0], "call0");
    std::thread::sleep(Duration::from_millis(200));
    let second = world.call_jack(&mut host, "Command", "h.cjs", &json!({"n": 1}), PATIENT);
    assert_eq!(second.outcome, Outcome::Ok);
    let lines = second.lines(Stream::Stderr);
    assert!(lines.contains(&"call1"), "{lines:?}");
}

#[test]
fn a_call_that_writes_nothing_to_stderr_does_not_wait_for_a_quiet_time() {
    if !available(Language::JavaScript) {
        return;
    }
    // The marker ends the call as soon as it has come: no time is added to a call (the jacks
    // that run on a key, such as SmartIndent, would show it). A wait of 20 ms per call would
    // take 2 s for these calls alone.
    let dir = plugin_dir(&[("h.cjs", "const x = 1;")]);
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    // The process is started by the first call.
    let first = world.call(&mut host, "h.cjs");
    assert_eq!(first.outcome, Outcome::Ok);
    let started = Instant::now();
    for _ in 0..100 {
        let report = world.call(&mut host, "h.cjs");
        assert_eq!(report.outcome, Outcome::Ok);
        assert!(report.lines(Stream::Stderr).is_empty());
    }
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "100 calls took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_marker_like_line_of_the_plugin_is_not_the_marker_of_another_call() {
    if !available(Language::JavaScript) {
        return;
    }
    // The handler writes markers for other calls (the previous one, one far ahead) before its
    // line. They are not taken for this call's marker, and they are not shown.
    let code = "
const n = Plugin.args.n;
for (const id of [n, n + 1000]) process.stderr.write('\\x1fmatsu-stderr-done ' + id + '\\n');
process.stderr.write('e' + n + '\\n');";
    let dir = plugin_dir(&[("h.cjs", code)]);
    let mut host = host(dir.path(), Language::JavaScript, true);
    let mut world = World::new("");
    for round in 0..100 {
        let args = json!({ "n": round });
        let report = world.call_jack(&mut host, "Command", "h.cjs", &args, PATIENT);
        assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
        assert_eq!(
            report.lines(Stream::Stderr),
            [format!("e{round}")],
            "round {round}: {:?}",
            report.output
        );
    }
}

#[test]
fn a_stderr_text_without_a_line_break_is_output_and_the_marker_is_not_shown() {
    if !available(Language::Python) {
        return;
    }
    // The marker lands on the same line as the text: the text is the call's output, the marker
    // is not shown, and the call does not wait for the cap.
    let code = "import sys\nsys.stderr.write('partial')\n";
    let dir = plugin_dir(&[("h.py", code)]);
    let mut host = host(dir.path(), Language::Python, true);
    let mut world = World::new("");
    let first = world.call(&mut host, "h.py");
    assert_eq!(first.outcome, Outcome::Ok);
    let started = Instant::now();
    for round in 0..100 {
        let report = world.call(&mut host, "h.py");
        assert_eq!(report.outcome, Outcome::Ok);
        assert_eq!(
            report.lines(Stream::Stderr),
            ["partial"],
            "round {round}: {:?}",
            report.output
        );
    }
    // The cap (250 ms) per call would take 25 s.
    assert!(
        started.elapsed() < Duration::from_millis(3000),
        "100 calls took {:?}",
        started.elapsed()
    );
}

/// A handler that closes standard error: the marker cannot be written, but the answer is sent, the
/// call succeeds (at the latest at the cap), and the next call of the process succeeds too.
fn closing_stderr_keeps_the_answer(language: Language, name: &str, code: &str) {
    if !available(language) {
        return;
    }
    let dir = plugin_dir(&[(name, code)]);
    let mut host = host(dir.path(), language, true);
    let mut world = World::new("");
    let started = Instant::now();
    for round in 0..2 {
        let report = world.call(&mut host, name);
        assert_eq!(
            report.outcome,
            Outcome::Ok,
            "{language:?}, round {round}: {:?}",
            report.output
        );
    }
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_node_handler_that_closes_stderr_still_gets_its_answer_sent() {
    // The second call finds it closed already.
    let code = "try { require('fs').closeSync(2); } catch (err) {}";
    closing_stderr_keeps_the_answer(Language::JavaScript, "h.cjs", code);
}

#[test]
fn a_python_handler_that_closes_stderr_still_gets_its_answer_sent() {
    closing_stderr_keeps_the_answer(Language::Python, "h.py", "import sys\nsys.stderr.close()\n");
}

#[test]
fn a_ruby_handler_that_closes_stderr_still_gets_its_answer_sent() {
    closing_stderr_keeps_the_answer(Language::Ruby, "h.rb", "$stderr.close\n");
}

#[test]
fn a_php_handler_that_closes_stderr_still_gets_its_answer_sent() {
    closing_stderr_keeps_the_answer(Language::Php, "h.php", "<?php\nfclose(STDERR);\n");
}

#[test]
fn a_plugin_that_cannot_write_the_marker_still_ends_its_call() {
    if !available(Language::Python) {
        return;
    }
    // Standard error is closed by the handler: the marker cannot be written, and the call ends
    // at the cap after its answer.
    let dir = plugin_dir(&[("h.py", "import os\nos.close(2)\n")]);
    let mut host = host(dir.path(), Language::Python, false);
    let mut world = World::new("");
    let started = Instant::now();
    let report = world.call(&mut host, "h.py");
    assert_eq!(report.outcome, Outcome::Ok, "{:?}", report.output);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
}
