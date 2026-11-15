//! The text input, the paste, the selection and the copy of the shell tab (ADR-0028, the third PR): the IME
//! through the input handler, Ctrl+V and the question about several lines, the mouse selection, Ctrl+C.
//!
//! The shells are real, as in `terminal_tests`: a PTY with a program of the test's own. The programs echo what
//! they get, and the tests wait for that with a time limit. The keys are the ones of the platform the test
//! runs on (Cmd+C and Cmd+V on macOS), because `translate` takes the platform from the build.

use gpui::{
    AnyWindowHandle, ClipboardItem, Context, Entity, EntityInputHandler, Modifiers, MouseButton,
    MouseDownEvent, TestAppContext, Window,
};
use matsu_terminal::ShellCommand;

use super::terminal_tests::{
    has_prompt, open, running, screen, staying, terminal_window, wait_for_end, wait_for_text,
    wait_until,
};
use super::tests::{answer, visual};
use crate::terminal::TerminalView;

const YES: &str = "はい(Y)";
const NO: &str = "いいえ(N)";

const COPY: &str = if cfg!(target_os = "macos") {
    "cmd-c"
} else {
    "ctrl-c"
};
const PASTE: &str = if cfg!(target_os = "macos") {
    "cmd-v"
} else {
    "ctrl-v"
};

/// A program that reads a line and tells what it got (`got-<line>`), then echoes (Unix). On Windows the
/// shell itself runs what is typed, and `echo x` prints `x`.
fn line_reader() -> ShellCommand {
    if cfg!(windows) {
        ShellCommand::new("cmd.exe", ["/Q", "/K", "echo hello"])
    } else {
        ShellCommand::new("/bin/sh", ["-c", "read a; echo got-$a; exec cat"])
    }
}

/// What to type so that `line_reader` prints `word` as a line of its own, and the line it prints.
fn typed(word: &str) -> String {
    if cfg!(windows) {
        format!("echo {word}")
    } else {
        word.to_owned()
    }
}

fn answer_line(word: &str) -> String {
    if cfg!(windows) {
        word.to_owned()
    } else {
        format!("got-{word}")
    }
}

fn has_line(cx: &mut TestAppContext, view: &Entity<TerminalView>, line: &str) -> bool {
    screen(cx, view).lines().any(|l| l == line)
}

fn wait_for_line(cx: &mut TestAppContext, view: &Entity<TerminalView>, line: &str) {
    let view = view.clone();
    let line = line.to_owned();
    wait_until(cx, &format!("the line {line:?}"), move |cx| {
        has_line(cx, &view, &line)
    });
}

pub(super) fn with_view(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    view: &Entity<TerminalView>,
    f: impl FnOnce(&mut TerminalView, &mut Window, &mut Context<TerminalView>),
) {
    handle
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| f(view, window, cx));
        })
        .unwrap();
    cx.run_until_parked();
}

/// The IME is composing `text`.
fn mark(cx: &mut TestAppContext, handle: AnyWindowHandle, view: &Entity<TerminalView>, text: &str) {
    with_view(cx, handle, view, |view, window, cx| {
        view.replace_and_mark_text_in_range(None, text, None, window, cx);
    });
}

/// The IME commits `text` (what GPUI does with the result string of the IME).
fn commit(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    view: &Entity<TerminalView>,
    text: &str,
) {
    with_view(cx, handle, view, |view, window, cx| {
        view.replace_text_in_range(None, text, window, cx);
    });
}

fn composing(cx: &mut TestAppContext, view: &Entity<TerminalView>) -> bool {
    view.read_with(cx, |view, _| view.is_composing())
}

pub(super) fn set_clipboard(cx: &mut TestAppContext, text: &str) {
    cx.write_to_clipboard(ClipboardItem::new_string(text.to_owned()));
}

pub(super) fn clipboard(cx: &mut TestAppContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

pub(super) fn keys(cx: &mut TestAppContext, handle: AnyWindowHandle, keystrokes: &str) {
    visual(cx, handle).simulate_keystrokes(keystrokes);
    cx.run_until_parked();
}

/// The row of the screen that has exactly `line`, and the column its `needle` starts in; `None` while the
/// screen does not show the line.
pub(super) fn find(
    cx: &mut TestAppContext,
    view: &Entity<TerminalView>,
    line: &str,
    needle: &str,
) -> Option<(usize, usize)> {
    let text = screen(cx, view);
    let row = text.lines().position(|l| l == line)?;
    let column = line.find(needle).expect("the needle is in the line");
    Some((row, column))
}

/// Does `attempt` again until it says it worked, or fails after a while.
///
/// A selection can be taken away by the terminal between two steps of a test: the tab is laid out after the
/// shell started, so the terminal is resized, and on Windows ConPTY answers a resize by painting the screen
/// again with erase sequences; `alacritty_terminal` clears a selection that an erase touches (and one when the
/// number of columns changes). A step that needs the selection is therefore done again from its start, and
/// what it checks is checked in the same attempt.
fn until_it_holds(
    cx: &mut TestAppContext,
    what: &str,
    mut attempt: impl FnMut(&mut TestAppContext) -> bool,
) {
    wait_until(cx, what, move |cx| attempt(cx));
}

/// Waits until the screen of the view is laid out (the mouse and the IME need its place).
pub(super) fn laid_out(cx: &mut TestAppContext, view: &Entity<TerminalView>) {
    let view = view.clone();
    wait_until(cx, "the screen to be laid out", move |cx| {
        view.read_with(cx, |view, _| view.cell_point(0, 0, 0.0).is_some())
    });
}

/// Drags over `columns` of `row` (the first to the last, both included) with the left button.
pub(super) fn drag(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    view: &Entity<TerminalView>,
    row: usize,
    columns: (usize, usize),
) {
    let (from, to) = view.read_with(cx, |view, _| {
        (
            view.cell_point(row, columns.0, 0.25).expect("laid out"),
            view.cell_point(row, columns.1, 0.75).expect("laid out"),
        )
    });
    let vcx = visual(cx, handle);
    vcx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
    vcx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
}

pub(super) fn click(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    view: &Entity<TerminalView>,
    (row, column): (usize, usize),
    count: usize,
) {
    let at = view.read_with(cx, |view, _| {
        view.cell_point(row, column, 0.25).expect("laid out")
    });
    let vcx = visual(cx, handle);
    vcx.simulate_event(MouseDownEvent {
        position: at,
        modifiers: Modifiers::default(),
        button: MouseButton::Left,
        click_count: count,
        first_mouse: false,
    });
    vcx.simulate_mouse_up(at, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
}

pub(super) fn selected(cx: &mut TestAppContext, view: &Entity<TerminalView>) -> Option<String> {
    view.read_with(cx, |view, _| view.selected_text())
}

// ---- the text input (the IME) ----

/// A shell that prints `alpha beta gamma` and stays.
fn words() -> ShellCommand {
    if cfg!(windows) {
        ShellCommand::new("cmd.exe", ["/Q", "/K", "echo alpha beta gamma"])
    } else {
        ShellCommand::new("/bin/sh", ["-c", "printf 'alpha beta gamma\\n'; exec cat"])
    }
}

/// Japanese where the PTY of the platform takes UTF-8 for sure; the Windows console is left to ASCII here.
fn kana_and_kanji() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("kana", "typed")
    } else {
        ("かな", "漢字")
    }
}

#[gpui::test]
fn text_the_ime_commits_reaches_the_program_and_text_being_composed_does_not(
    cx: &mut TestAppContext,
) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    let (composed, committed) = kana_and_kanji();
    mark(cx, handle, &view, composed);
    assert!(composing(cx, &view));
    // The platform asks what is being composed.
    let units = composed.encode_utf16().count();
    with_view(cx, handle, &view, |view, window, cx| {
        assert_eq!(view.marked_text_range(window, cx), Some(0..units));
        assert_eq!(view.preedit(), Some(composed));
    });
    // A new preedit replaces the old one: it is still not written.
    mark(cx, handle, &view, composed);
    commit(cx, handle, &view, committed);
    assert!(!composing(cx, &view), "the commit ends the composition");
    wait_for_text(cx, &view, committed);
    // What was written before the commit reached the program first: it is not there.
    assert!(!screen(cx, &view).contains(composed));
}

#[gpui::test]
fn enter_while_the_ime_composes_does_not_reach_the_program(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, line_reader());
    if cfg!(windows) {
        wait_for_text(cx, &view, "hello");
    }
    let (composed, _) = kana_and_kanji();
    mark(cx, handle, &view, composed);
    // The platform gives the IME the keys first; one that comes to the view while the IME composes is not the
    // program's. (Enter would end the line `read` waits for.)
    keys(cx, handle, "enter");
    keys(cx, handle, "backspace");
    keys(cx, handle, "left");
    assert!(
        composing(cx, &view),
        "the view does not end the composition itself"
    );
    // The composition ends with a commit; now Enter is the program's.
    commit(cx, handle, &view, &typed("abc"));
    keys(cx, handle, "enter");
    wait_for_line(cx, &view, &answer_line("abc"));
    if !cfg!(windows) {
        // A line of nothing would have been read by the first Enter.
        assert!(!has_line(cx, &view, "got-"));
    }
}

#[gpui::test]
fn a_dead_key_is_composed_and_the_accent_alone_is_not_written(cx: &mut TestAppContext) {
    if cfg!(windows) {
        // Windows delivers a dead key through `WM_CHAR` after the next key, not through the composition;
        // the path of the character is the one of `text_the_ime_commits_...`.
        return;
    }
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    // Linux (X11): the dead key is a preedit, and the next key arrives as a key with the composed text.
    mark(cx, handle, &view, "\u{b4}");
    assert!(composing(cx, &view));
    visual(cx, handle).simulate_input("é");
    cx.run_until_parked();
    assert!(!composing(cx, &view));
    wait_for_text(cx, &view, "é");
    assert!(
        !screen(cx, &view).contains('\u{b4}'),
        "the accent alone was not written"
    );
}

#[gpui::test]
fn the_candidate_window_opens_at_the_cursor_and_follows_the_preedit(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, staying());
    wait_for_text(cx, &view, "hello");
    // The cursor is read again in every attempt: on Windows ConPTY can paint the screen again after the tab
    // is laid out (see `until_it_holds`), which moves the prompt and the cursor with it, so a cell read once
    // may be stale by the time it is compared. Each attempt reads the cursor and the candidate window's
    // place back to back; one that straddles a move fails its comparison and is done again.
    let near = |a: gpui::Pixels, b: gpui::Pixels| (f32::from(a) - f32::from(b)).abs() < 0.01;
    // `range` of the preedit, `columns_on` cells past the cursor: the place the window is asked for, and the
    // cell of the cursor that it is expected at.
    let measure = |cx: &mut TestAppContext, range: std::ops::Range<usize>, columns_on: usize| {
        let (row, column) = view.read_with(cx, |view, _| view.cursor());
        let mut bounds = None;
        with_view(cx, handle, &view, |view, window, cx| {
            bounds = view.bounds_for_range(range, gpui::Bounds::default(), window, cx);
        });
        let expected = view.read_with(cx, |view, _| view.cell_point(row, column + columns_on, 0.0));
        Some((bounds?, expected?))
    };
    // No preedit: the cell of the cursor.
    until_it_holds(cx, "the window to open at the cell of the cursor", |cx| {
        measure(cx, 0..0, 0).is_some_and(|(bounds, expected)| {
            near(bounds.origin.x, expected.x)
                && near(bounds.origin.y + bounds.size.height / 2.0, expected.y)
        })
    });
    // Two wide characters: the second starts two cells on, and a range past the text is after it.
    mark(cx, handle, &view, "かな");
    until_it_holds(cx, "the window to follow the preedit", |cx| {
        let (Some((second, second_at)), Some((after, after_at)), Some((whole, _))) = (
            measure(cx, 1..1, 2),
            measure(cx, 2..2, 4),
            measure(cx, 0..2, 0),
        ) else {
            return false;
        };
        near(second.origin.x, second_at.x)
            && near(after.origin.x, after_at.x)
            && near(whole.size.width, second.size.width * 4.0)
    });
}

// ---- paste ----

#[gpui::test]
fn a_paste_of_one_line_is_written_without_a_question(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, line_reader());
    if cfg!(windows) {
        wait_for_text(cx, &view, "hello");
    }
    set_clipboard(cx, &typed("abc"));
    keys(cx, handle, PASTE);
    assert!(!has_prompt(cx, handle));
    keys(cx, handle, "enter");
    wait_for_line(cx, &view, &answer_line("abc"));
}

#[gpui::test]
fn a_paste_of_several_lines_asks_first_and_no_writes_nothing(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, line_reader());
    if cfg!(windows) {
        wait_for_text(cx, &view, "hello");
    }
    set_clipboard(cx, &format!("{}\n{}", typed("one"), typed("two")));
    keys(cx, handle, PASTE);
    assert!(has_prompt(cx, handle), "several lines are asked about");
    answer(cx, handle, NO);
    assert!(!view.read_with(cx, |view, _| view.asking_about_a_paste()));
    // Nothing was written: the next line the program reads is the typed one.
    commit(cx, handle, &view, &typed("x"));
    keys(cx, handle, "enter");
    wait_for_line(cx, &view, &answer_line("x"));
    assert!(!has_line(cx, &view, &answer_line("one")));
    assert!(!has_line(cx, &view, &answer_line("two")));
}

#[gpui::test]
fn a_paste_of_several_lines_is_written_when_the_answer_is_yes(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, line_reader());
    if cfg!(windows) {
        wait_for_text(cx, &view, "hello");
    }
    // A line break at the end: the driver echoes the second line, and without it the program's answer would
    // start in the middle of that echo.
    set_clipboard(cx, &format!("{}\n{}\n", typed("one"), typed("two")));
    keys(cx, handle, PASTE);
    assert!(has_prompt(cx, handle));
    // A second paste while the question is open does not ask again.
    keys(cx, handle, PASTE);
    answer(cx, handle, YES);
    wait_for_line(cx, &view, &answer_line("one"));
    if cfg!(windows) {
        wait_for_line(cx, &view, &answer_line("two"));
    }
    assert!(!has_prompt(cx, handle));
}

#[gpui::test]
fn the_answer_to_the_paste_question_is_not_acted_on_when_the_shell_has_ended(
    cx: &mut TestAppContext,
) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, line_reader());
    if cfg!(windows) {
        wait_for_text(cx, &view, "hello");
    }
    set_clipboard(cx, &format!("{}\n{}", typed("one"), typed("two")));
    keys(cx, handle, PASTE);
    assert!(has_prompt(cx, handle));
    // The shell goes away while the question is open (as when the user closes it from another place).
    view.update(cx, |view, _| view.shutdown());
    assert!(!running(cx, &view));
    answer(cx, handle, YES);
    cx.run_until_parked();
    // No error, nothing written, and the view can be asked again.
    assert!(!view.read_with(cx, |view, _| view.asking_about_a_paste()));
    assert!(!has_prompt(cx, handle));
    assert!(matsu_terminal::wait_for_closing(
        std::time::Duration::from_secs(60)
    ));
}

/// A program that turns bracketed paste on and then echoes (`cat`). The terminal in the PTY shows ESC as `^[`.
#[cfg(not(windows))]
fn bracketed() -> ShellCommand {
    ShellCommand::new("/bin/sh", ["-c", "printf '\\033[?2004h'; exec cat"])
}

// The programs of the Windows tests cannot turn bracketed paste on (ConPTY has no way to be told what a
// console program writes as ESC [ ? 2004 h from a test command); the encoding itself is tested in
// `matsu-terminal` (`paste.rs`), and this is the path from the key to the encoding on the other systems.
#[cfg(not(windows))]
#[gpui::test]
fn a_paste_is_in_brackets_when_the_program_asked_for_them_and_asks_nothing(
    cx: &mut TestAppContext,
) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, bracketed());
    let v = view.clone();
    wait_until(cx, "bracketed paste to be on", move |cx| {
        v.read_with(cx, |view, _| view.bracketed_paste())
    });
    set_clipboard(cx, "one\ntwo\n");
    keys(cx, handle, PASTE);
    assert!(
        !has_prompt(cx, handle),
        "brackets make the lines data: no question"
    );
    // The start marker and the first line; the terminal driver echoes ESC as ^[.
    wait_for_text(cx, &view, "^[[200~one");
}

// ---- selection and copy ----

#[gpui::test]
fn a_drag_selects_and_the_copy_key_copies_it_without_writing_the_interrupt(
    cx: &mut TestAppContext,
) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, words());
    wait_for_line(cx, &view, "alpha beta gamma");
    laid_out(cx, &view);
    until_it_holds(cx, "the drag to be copied", |cx| {
        let Some((row, column)) = find(cx, &view, "alpha beta gamma", "beta") else {
            return false;
        };
        drag(cx, handle, &view, row, (column, column + 3));
        if selected(cx, &view).as_deref() != Some("beta") {
            return false;
        }
        set_clipboard(cx, "before");
        keys(cx, handle, COPY);
        clipboard(cx).as_deref() == Some("beta")
    });
    assert_eq!(selected(cx, &view), None, "copying clears the selection");
    // The key was the copy's: the program is alive, and no ^C was written (the driver would show it).
    visual(cx, handle).simulate_input(if cfg!(windows) { "echo ok" } else { "ok" });
    keys(cx, handle, "enter");
    wait_for_line(cx, &view, "ok");
    assert!(running(cx, &view));
    assert!(!screen(cx, &view).contains("^C"));
}

#[gpui::test]
fn the_copy_key_without_a_selection_goes_to_the_program_as_the_interrupt(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    // A program that waits, and that the interrupt ends.
    let shell = if cfg!(windows) {
        ShellCommand::new("cmd.exe", ["/C", "echo hello & ping -n 60 127.0.0.1 >nul"])
    } else {
        ShellCommand::new("/bin/sh", ["-c", "printf 'hello\\n'; sleep 60"])
    };
    let view = open(cx, handle, shell);
    wait_for_text(cx, &view, "hello");
    set_clipboard(cx, "before");
    if cfg!(target_os = "macos") {
        // Ctrl+C is the program's whatever there is, on macOS.
        keys(cx, handle, "ctrl-c");
    } else {
        keys(cx, handle, COPY);
    }
    wait_for_end(cx, &view);
    assert_eq!(
        clipboard(cx).as_deref(),
        Some("before"),
        "nothing was copied"
    );
}

#[cfg(target_os = "macos")]
#[gpui::test]
fn ctrl_c_is_the_programs_on_macos_even_with_a_selection(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let shell = ShellCommand::new("/bin/sh", ["-c", "printf 'alpha beta gamma\\n'; sleep 60"]);
    let view = open(cx, handle, shell);
    wait_for_line(cx, &view, "alpha beta gamma");
    laid_out(cx, &view);
    until_it_holds(cx, "a selection", |cx| {
        let Some((row, column)) = find(cx, &view, "alpha beta gamma", "beta") else {
            return false;
        };
        drag(cx, handle, &view, row, (column, column + 3));
        selected(cx, &view).is_some()
    });
    set_clipboard(cx, "before");
    keys(cx, handle, "ctrl-c");
    wait_for_end(cx, &view);
    assert_eq!(clipboard(cx).as_deref(), Some("before"));
}

#[gpui::test]
fn a_drag_goes_on_past_the_edge_of_the_screen(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, words());
    wait_for_line(cx, &view, "alpha beta gamma");
    laid_out(cx, &view);
    until_it_holds(cx, "the drag past the edge", |cx| {
        let Some((row, column)) = find(cx, &view, "alpha beta gamma", "beta") else {
            return false;
        };
        let from = view.read_with(cx, |view, _| {
            view.cell_point(row, column, 0.25).expect("laid out")
        });
        // far to the right of the window, on the same row
        let far = gpui::point(gpui::px(5000.), from.y);
        let vcx = visual(cx, handle);
        vcx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        vcx.simulate_mouse_move(far, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        // the selection followed the pointer out of the screen: to the end of the row
        let followed = selected(cx, &view).as_deref() == Some("beta gamma");
        visual(cx, handle).simulate_mouse_up(far, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        if !followed || selected(cx, &view).as_deref() != Some("beta gamma") {
            return false;
        }
        // the button is up: moving the pointer changes nothing
        let back = view.read_with(cx, |view, _| view.cell_point(row, 0, 0.25).unwrap());
        visual(cx, handle).simulate_mouse_move(back, None, Modifiers::default());
        cx.run_until_parked();
        selected(cx, &view).as_deref() == Some("beta gamma")
    });
}

#[gpui::test]
fn double_click_selects_a_word_and_triple_click_the_line(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, words());
    wait_for_line(cx, &view, "alpha beta gamma");
    laid_out(cx, &view);
    for (count, expected) in [(2, "beta"), (3, "alpha beta gamma")] {
        until_it_holds(cx, &format!("{count} clicks to select"), |cx| {
            let Some((row, column)) = find(cx, &view, "alpha beta gamma", "beta") else {
                return false;
            };
            click(cx, handle, &view, (row, column + 1), count);
            selected(cx, &view).as_deref() == Some(expected)
        });
    }
    // a click clears the selection
    let (row, column) = find(cx, &view, "alpha beta gamma", "beta").expect("the line is there");
    click(cx, handle, &view, (row, column + 1), 1);
    assert_eq!(selected(cx, &view), None);
}

#[gpui::test]
fn typing_and_pasting_clear_the_selection(cx: &mut TestAppContext) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, words());
    wait_for_line(cx, &view, "alpha beta gamma");
    laid_out(cx, &view);
    // each step is its own attempt: it selects, checks the selection, does the thing and checks it is gone
    until_it_holds(cx, "typing to clear a selection", |cx| {
        let Some((row, column)) = find(cx, &view, "alpha beta gamma", "alpha") else {
            return false;
        };
        drag(cx, handle, &view, row, (column, column + 4));
        if selected(cx, &view).as_deref() != Some("alpha") {
            return false;
        }
        visual(cx, handle).simulate_input("x");
        cx.run_until_parked();
        selected(cx, &view).is_none()
    });
    set_clipboard(cx, "p");
    until_it_holds(cx, "pasting to clear a selection", |cx| {
        // the typed "x" is on the line now: the word is found by its own text
        let Some(line) = screen(cx, &view)
            .lines()
            .find(|l| l.starts_with("alpha"))
            .map(str::to_owned)
        else {
            return false;
        };
        let Some((row, column)) = find(cx, &view, &line, "alpha") else {
            return false;
        };
        drag(cx, handle, &view, row, (column, column + 4));
        if selected(cx, &view).as_deref() != Some("alpha") {
            return false;
        }
        keys(cx, handle, PASTE);
        selected(cx, &view).is_none()
    });
}

#[gpui::test]
fn the_copy_item_of_the_menu_is_gray_until_something_is_selected(cx: &mut TestAppContext) {
    use crate::terminal::menus::mark_items;
    let handle = terminal_window(cx);
    let view = open(cx, handle, words());
    wait_for_line(cx, &view, "alpha beta gamma");
    let ws = super::tests::workspace(cx, handle);
    let copy_disabled = |cx: &mut TestAppContext| {
        let mut disabled = None;
        handle
            .update(cx, |_, _, app| {
                let mut items = crate::terminal::menus::menus().remove(0).owned().items;
                mark_items(&mut items, ws.read(app), app);
                disabled = items.iter().find_map(|item| match item {
                    gpui::OwnedMenuItem::Action { name, disabled, .. }
                        if name.starts_with("コピー") =>
                    {
                        Some(*disabled)
                    }
                    _ => None,
                });
            })
            .unwrap();
        disabled.expect("the copy item")
    };
    laid_out(cx, &view);
    assert!(copy_disabled(cx));
    until_it_holds(cx, "a selection to light the copy item", |cx| {
        let Some((row, column)) = find(cx, &view, "alpha beta gamma", "gamma") else {
            return false;
        };
        drag(cx, handle, &view, row, (column, column + 4));
        selected(cx, &view).is_some() && !copy_disabled(cx)
    });
}
