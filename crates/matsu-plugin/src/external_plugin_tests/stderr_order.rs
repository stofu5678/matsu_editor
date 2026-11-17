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
