//! The mouse, the scrollback, the links, the bell, the title and OSC 52 of the shell tab (ADR-0028, the fourth
//! PR), with real shells as in `terminal_tests`.
//!
//! Most programs here are small `sh` scripts that turn a mode on, say `ready`, and then echo what they get with
//! `cat -v` (the terminal in non-canonical mode, so a key reaches `cat` at once and shows as `^[[<0;6;4M`).
//! They need `sh`, `stty`, `cat -v` and `seq`, so they run on Linux and macOS (`unix_programs`). The tests
//! that every OS runs, with `cmd.exe` or PowerShell on Windows: a printed link (hover, question, opening),
//! and, on Windows, the title, OSC 52 and the scrollback. There is no simple Windows program that turns on
//! mouse reporting and shows what it got, so those tests are Unix only; the encoding itself is tested
//! without a shell in `matsu-terminal`.
//!
//! The test window is never the active window of the platform, so a bell takes the path of a window in the
//! background (the mark on the tab); the flash of a bell that rings while the terminal is looked at is not
//! reached here.

use gpui::{
    AnyWindowHandle, Entity, Modifiers, MouseButton, MouseDownEvent, Pixels, Point, ScrollDelta,
    ScrollWheelEvent, TestAppContext, TouchPhase, point,
};
use matsu_terminal::ShellCommand;

use super::terminal_input_tests::{clipboard, find, laid_out, set_clipboard};
use super::terminal_tests::{has_prompt, open, screen, terminal_window, wait_for_text, wait_until};
use super::tests::{answer, visual, workspace};
use crate::terminal::TerminalView;

const YES: &str = "はい(Y)";
const NO: &str = "いいえ(N)";

fn at(
    cx: &mut TestAppContext,
    view: &Entity<TerminalView>,
    (row, column): (usize, usize),
) -> Point<Pixels> {
    view.read_with(cx, |view, _| {
        view.cell_point(row, column, 0.5).expect("laid out")
    })
}

/// The modifier of a link click on this platform.
fn link_key() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers {
            platform: true,
            ..Modifiers::default()
        }
    } else {
        Modifiers {
            control: true,
            ..Modifiers::default()
        }
    }
}

fn link_click(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    view: &Entity<TerminalView>,
    cell: (usize, usize),
    modifiers: Modifiers,
) {
    let position = at(cx, view, cell);
    let vcx = visual(cx, handle);
    vcx.simulate_event(MouseDownEvent {
        position,
        modifiers,
        button: MouseButton::Left,
        click_count: 1,
        first_mouse: false,
    });
    vcx.simulate_mouse_up(position, MouseButton::Left, modifiers);
    cx.run_until_parked();
}

/// Opens `shell` in a shell tab and waits until `word` is on the screen and the screen is laid out.
fn started_with(
    cx: &mut TestAppContext,
    shell: ShellCommand,
    word: &str,
) -> (AnyWindowHandle, Entity<TerminalView>) {
    let handle = terminal_window(cx);
    let view = open(cx, handle, shell);
    wait_for_text(cx, &view, word);
    laid_out(cx, &view);
    (handle, view)
}

fn wheel(
    cx: &mut TestAppContext,
    handle: AnyWindowHandle,
    view: &Entity<TerminalView>,
    cell: (usize, usize),
    lines: f32,
    modifiers: Modifiers,
) {
    let position = at(cx, view, cell);
    visual(cx, handle).simulate_event(ScrollWheelEvent {
        position,
        delta: ScrollDelta::Lines(point(0.0, lines)),
        modifiers,
        touch_phase: TouchPhase::Moved,
    });
    cx.run_until_parked();
}

#[cfg(unix)]
fn first_line(cx: &mut TestAppContext, view: &Entity<TerminalView>) -> String {
    screen(cx, view).lines().next().unwrap_or("").to_owned()
}

#[cfg(unix)]
fn first_number(cx: &mut TestAppContext, view: &Entity<TerminalView>) -> i64 {
    first_line(cx, view)
        .split_whitespace()
        .last()
        .and_then(|number| number.parse().ok())
        .expect("the first line of the view ends in a number")
}

/// A program that prints a line with an address in it and stays.
fn printed_link() -> ShellCommand {
    if cfg!(windows) {
        ShellCommand::new(
            "cmd.exe",
            ["/Q", "/K", "echo see https://example.com/matsu-test end"],
        )
    } else {
        ShellCommand::new(
            "/bin/sh",
            [
                "-c",
                "printf 'see https://example.com/matsu-test end\\n'; exec cat",
            ],
        )
    }
}

#[gpui::test]
fn a_printed_address_is_underlined_with_the_key_and_opened_after_the_question(
    cx: &mut TestAppContext,
) {
    const LINE: &str = "see https://example.com/matsu-test end";
    const URL: &str = "https://example.com/matsu-test";
    let (handle, view) = started_with(cx, printed_link(), "matsu-test end");
    // the cell inside the address; the screen may be painted again (ConPTY), so it is looked for each time
    let cell = |cx: &mut TestAppContext| {
        wait_until(cx, "the line", |cx| {
            find(cx, &view, LINE, "https").is_some()
        });
        let (row, column) = find(cx, &view, LINE, "https").expect("the line");
        (row, column + 4)
    };

    // a click without the key selects, and asks nothing
    let at_url = cell(cx);
    link_click(cx, handle, &view, at_url, Modifiers::default());
    assert!(!has_prompt(cx, handle));

    // the address is underlined while the key is held over it, and not otherwise
    let hovered = |cx: &mut TestAppContext| view.read_with(cx, |view, _| view.hovered_uri());
    let position = at(cx, &view, at_url);
    visual(cx, handle).simulate_mouse_move(position, None, link_key());
    cx.run_until_parked();
    assert_eq!(hovered(cx).as_deref(), Some(URL));
    visual(cx, handle).simulate_mouse_move(position, None, Modifiers::default());
    cx.run_until_parked();
    assert_eq!(hovered(cx), None);

    // the question shows the address; "no" opens nothing, "yes" opens it
    let at_url = cell(cx);
    link_click(cx, handle, &view, at_url, link_key());
    assert!(has_prompt(cx, handle));
    let (message, _) = cx.pending_prompt().expect("a question");
    assert!(message.contains(URL), "{message}");
    assert!(view.read_with(cx, |view, _| view.asking_about_a_link()));
    answer(cx, handle, NO);
    assert_eq!(cx.opened_url(), None);
    let at_url = cell(cx);
    link_click(cx, handle, &view, at_url, link_key());
    answer(cx, handle, YES);
    assert_eq!(cx.opened_url().as_deref(), Some(URL));
    assert!(!view.read_with(cx, |view, _| view.asking_about_a_link()));
}

/// The title and OSC 52 on Windows: `cmd.exe` sets a title with `title`, and PowerShell writes the sequence. The
/// shell stays, or stays for a while (output right before the end of a program can be lost by ConPTY).
#[cfg(windows)]
mod windows_programs {
    use super::*;

    #[gpui::test]
    fn the_title_a_program_sets_is_the_caption_and_the_tooltip_leads_with_it(
        cx: &mut TestAppContext,
    ) {
        let handle = terminal_window(cx);
        let view = open(
            cx,
            handle,
            ShellCommand::new("cmd.exe", ["/Q", "/K", "title hello"]),
        );
        let caption = |cx: &mut TestAppContext| {
            workspace(cx, handle).read_with(cx, |ws, cx| ws.tab_panel(1).unwrap().caption(cx))
        };
        wait_until(cx, "the title", |cx| caption(cx) == "hello");
        assert_eq!(
            view.read_with(cx, |view, _| view.program_title().map(str::to_owned))
                .as_deref(),
            Some("hello")
        );
        let tooltip =
            workspace(cx, handle).read_with(cx, |ws, cx| ws.tab_panel(1).unwrap().tooltip(cx));
        assert!(tooltip.starts_with("hello\n"), "{tooltip}");
    }

    #[gpui::test]
    fn the_program_writes_the_clipboard_with_osc_52(cx: &mut TestAppContext) {
        set_clipboard(cx, "mine");
        let handle = terminal_window(cx);
        let _view = open(
            cx,
            handle,
            ShellCommand::new(
                "powershell.exe",
                [
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "Write-Host -NoNewline \"$([char]27)]52;c;aGVsbG8=$([char]7)\"; Start-Sleep -Seconds 5",
                ],
            ),
        );
        wait_until(cx, "the clipboard", |cx| {
            clipboard(cx).as_deref() == Some("hello")
        });
    }

    #[gpui::test]
    fn the_wheel_scrolls_back_through_the_scrollback_and_typing_returns_to_the_bottom(
        cx: &mut TestAppContext,
    ) {
        let (handle, view) = started_with(
            cx,
            ShellCommand::new(
                "cmd.exe",
                ["/Q", "/K", "for /l %i in (1,1,200) do @echo line %i"],
            ),
            "line 200",
        );
        // The scroll position is what is checked, not the text: ConPTY paints the screen again after the
        // resize, which can leave a second copy of the last lines in the scrollback.
        let offset = |cx: &mut TestAppContext| view.read_with(cx, |view, _| view.display_offset());
        assert_eq!(offset(cx), 0);
        wheel(cx, handle, &view, (2, 3), 5.0, Modifiers::default());
        assert!(offset(cx) > 0, "the view went back");
        // typing brings the view back to the live screen
        visual(cx, handle).simulate_input("x");
        wait_until(cx, "the bottom", |cx| offset(cx) == 0);
    }
}

/// The tests that need `sh`, `stty`, `cat -v` and `seq`.
#[cfg(unix)]
mod unix_programs {
    use super::super::terminal_input_tests::{drag, keys, selected};
    use super::*;

    fn program(script: &str) -> ShellCommand {
        ShellCommand::new("/bin/sh", ["-c", script])
    }

    /// Reports on (click mode, SGR), then echoes what it gets.
    const CLICKS: &str =
        r"stty -icanon -echo; printf '\033[?1000h\033[?1006h'; echo ready; exec cat -v";
    /// Reports on with drag motion (1002), SGR.
    const DRAGS: &str =
        r"stty -icanon -echo; printf '\033[?1002h\033[?1006h'; echo ready; exec cat -v";
    /// Reports on in the old encoding (no 1006).
    const OLD_CLICKS: &str = r"stty -icanon -echo; printf '\033[?1000h'; echo ready; exec cat -v";

    /// Opens the program in a shell tab and waits until it says `ready` and the screen is laid out.
    fn started(cx: &mut TestAppContext, script: &str) -> (AnyWindowHandle, Entity<TerminalView>) {
        started_at(cx, script, "ready")
    }

    /// Like `started`, for a program whose last word is `word`.
    fn started_at(
        cx: &mut TestAppContext,
        script: &str,
        word: &str,
    ) -> (AnyWindowHandle, Entity<TerminalView>) {
        started_with(cx, program(script), word)
    }

    fn press_and_release(
        cx: &mut TestAppContext,
        handle: AnyWindowHandle,
        view: &Entity<TerminalView>,
        cell: (usize, usize),
        modifiers: Modifiers,
    ) {
        let position = at(cx, view, cell);
        let vcx = visual(cx, handle);
        vcx.simulate_mouse_down(position, MouseButton::Left, modifiers);
        vcx.simulate_mouse_up(position, MouseButton::Left, modifiers);
        cx.run_until_parked();
    }

    fn shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..Modifiers::default()
        }
    }

    // ---- mouse reports ----

    #[gpui::test]
    fn clicks_reach_a_program_that_turned_mouse_reporting_on_instead_of_selecting(
        cx: &mut TestAppContext,
    ) {
        let (handle, view) = started(cx, CLICKS);
        press_and_release(cx, handle, &view, (3, 5), Modifiers::default());
        // the cell is 1-based in the report: column 6, row 4
        wait_for_text(cx, &view, "^[[<0;6;4M^[[<0;6;4m");
        assert!(
            !view.read_with(cx, |view, _| view.has_selection()),
            "a program that has the mouse selects nothing"
        );
    }

    #[gpui::test]
    fn the_old_encoding_is_used_when_the_program_did_not_ask_for_sgr(cx: &mut TestAppContext) {
        let (handle, view) = started(cx, OLD_CLICKS);
        press_and_release(cx, handle, &view, (0, 0), Modifiers::default());
        // button 0 is a space, the first column and row are `!`; the release is button 3 (`#`)
        wait_for_text(cx, &view, "^[[M !!^[[M#!!");
    }

    #[gpui::test]
    fn a_drag_is_reported_as_motion_with_the_button_down_once_per_cell(cx: &mut TestAppContext) {
        let (handle, view) = started(cx, DRAGS);
        let (from, near, to) = (
            at(cx, &view, (4, 2)),
            view.read_with(cx, |view, _| view.cell_point(4, 3, 0.2).unwrap()),
            view.read_with(cx, |view, _| view.cell_point(4, 3, 0.8).unwrap()),
        );
        let vcx = visual(cx, handle);
        vcx.simulate_mouse_down(from, MouseButton::Left, Modifiers::default());
        vcx.simulate_mouse_move(near, MouseButton::Left, Modifiers::default());
        // inside the same cell: nothing more
        vcx.simulate_mouse_move(to, MouseButton::Left, Modifiers::default());
        vcx.simulate_mouse_up(to, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        wait_for_text(cx, &view, "^[[<0;3;5M^[[<32;4;5M^[[<0;4;5m");
    }

    #[gpui::test]
    fn shift_with_the_mouse_selects_even_when_the_program_has_the_mouse(cx: &mut TestAppContext) {
        let (handle, view) = started(cx, CLICKS);
        let (from, to) = view.read_with(cx, |view, _| {
            (
                view.cell_point(0, 0, 0.25).unwrap(),
                view.cell_point(0, 4, 0.75).unwrap(),
            )
        });
        let vcx = visual(cx, handle);
        vcx.simulate_mouse_down(from, MouseButton::Left, shift());
        vcx.simulate_mouse_move(to, MouseButton::Left, shift());
        vcx.simulate_mouse_up(to, MouseButton::Left, shift());
        cx.run_until_parked();
        assert_eq!(selected(cx, &view).as_deref(), Some("ready"));
        // the program was told nothing about it: the next click is the first report it gets
        press_and_release(cx, handle, &view, (5, 7), Modifiers::default());
        wait_for_text(cx, &view, "^[[<0;8;6M");
        assert!(
            !screen(cx, &view).contains("^[[<0;1;1M"),
            "{}",
            screen(cx, &view)
        );
    }

    #[gpui::test]
    fn the_wheel_is_reported_while_the_program_has_the_mouse(cx: &mut TestAppContext) {
        let (handle, view) = started(cx, CLICKS);
        wheel(cx, handle, &view, (2, 3), 1.0, Modifiers::default());
        wait_for_text(cx, &view, "^[[<64;4;3M");
        wheel(cx, handle, &view, (2, 3), -2.0, Modifiers::default());
        wait_for_text(cx, &view, "^[[<64;4;3M^[[<65;4;3M^[[<65;4;3M");
    }

    #[gpui::test]
    fn a_sideways_wheel_is_reported_as_buttons_66_and_67(cx: &mut TestAppContext) {
        let (handle, view) = started(cx, CLICKS);
        let position = at(cx, &view, (2, 3));
        for sideways in [1.0, -1.0] {
            visual(cx, handle).simulate_event(ScrollWheelEvent {
                position,
                delta: ScrollDelta::Lines(point(sideways, 0.0)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.run_until_parked();
        }
        // positive is the wheel turned left (66)
        wait_for_text(cx, &view, "^[[<66;4;3M^[[<67;4;3M");
    }

    // ---- scrolling back ----

    /// 500 numbered lines, then `cat`.
    const NUMBERS: &str = "seq 1 500; exec cat";

    fn numbers(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<TerminalView>, i64) {
        let handle = terminal_window(cx);
        let view = open(cx, handle, program(NUMBERS));
        wait_for_text(cx, &view, "500");
        laid_out(cx, &view);
        let top = first_number(cx, &view);
        (handle, view, top)
    }

    #[gpui::test]
    fn the_wheel_scrolls_back_through_the_scrollback_and_typing_returns_to_the_bottom(
        cx: &mut TestAppContext,
    ) {
        let (handle, view, bottom) = numbers(cx);
        wheel(cx, handle, &view, (2, 3), 3.0, Modifiers::default());
        assert_eq!(first_number(cx, &view), bottom - 3);
        // the other way
        wheel(cx, handle, &view, (2, 3), -1.0, Modifiers::default());
        assert_eq!(first_number(cx, &view), bottom - 2);
        // a turn that is less than a line is kept until it makes one (a trackpad)
        let position = at(cx, &view, (2, 3));
        let line = view.read_with(cx, |view, _| {
            view.cell_point(1, 0, 0.5).unwrap().y - view.cell_point(0, 0, 0.5).unwrap().y
        });
        for _ in 0..2 {
            visual(cx, handle).simulate_event(ScrollWheelEvent {
                position,
                delta: ScrollDelta::Pixels(point(Pixels::ZERO, line * 0.6)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
        }
        cx.run_until_parked();
        assert_eq!(first_number(cx, &view), bottom - 3);
        // typing brings the view back to the live screen
        visual(cx, handle).simulate_input("x");
        cx.run_until_parked();
        assert_eq!(first_number(cx, &view), bottom);
    }

    #[gpui::test]
    fn the_scroll_keys_move_the_view_and_are_not_sent_to_the_program(cx: &mut TestAppContext) {
        let (handle, view, bottom) = numbers(cx);
        let rows = view.read_with(cx, |view, _| view.grid().1) as i64;
        keys(cx, handle, "ctrl-shift-up");
        assert_eq!(first_number(cx, &view), bottom - 1);
        keys(cx, handle, "ctrl-shift-down");
        assert_eq!(first_number(cx, &view), bottom);
        keys(cx, handle, "ctrl-shift-pageup");
        assert_eq!(first_number(cx, &view), bottom - (rows - 1));
        keys(cx, handle, "ctrl-shift-pagedown");
        assert_eq!(first_number(cx, &view), bottom);
        keys(cx, handle, "ctrl-shift-home");
        assert_eq!(first_number(cx, &view), 1);
        keys(cx, handle, "ctrl-shift-end");
        assert_eq!(first_number(cx, &view), bottom);
        // the keys did not reach the program: it would have echoed an escape sequence
        assert!(!screen(cx, &view).contains("^["), "{}", screen(cx, &view));
    }

    #[gpui::test]
    fn a_selection_works_while_the_view_is_scrolled_back(cx: &mut TestAppContext) {
        let (handle, view, bottom) = numbers(cx);
        wheel(cx, handle, &view, (2, 3), 5.0, Modifiers::default());
        let top = first_line(cx, &view);
        assert_eq!(top.trim(), (bottom - 5).to_string());
        drag(cx, handle, &view, 0, (0, top.len() - 1));
        assert_eq!(selected(cx, &view).as_deref(), Some(top.as_str()));
        // the view moves, the selection stays on its text
        wheel(cx, handle, &view, (2, 3), 2.0, Modifiers::default());
        assert_eq!(selected(cx, &view).as_deref(), Some(top.as_str()));
    }

    #[gpui::test]
    fn on_the_alternate_screen_the_wheel_is_arrow_keys_unless_the_program_has_the_mouse(
        cx: &mut TestAppContext,
    ) {
        let (handle, view) = started(
            cx,
            r"stty -icanon -echo; printf '\033[?1049h'; echo ready; exec cat -v",
        );
        wheel(cx, handle, &view, (2, 3), 2.0, Modifiers::default());
        wait_for_text(cx, &view, "^[[A^[[A");
        wheel(cx, handle, &view, (2, 3), -1.0, Modifiers::default());
        wait_for_text(cx, &view, "^[[A^[[A^[[B");
    }

    #[gpui::test]
    fn on_the_alternate_screen_in_application_cursor_mode_the_arrows_are_ss3(
        cx: &mut TestAppContext,
    ) {
        let (handle, view) = started(
            cx,
            r"stty -icanon -echo; printf '\033[?1049h\033[?1h'; echo ready; exec cat -v",
        );
        wheel(cx, handle, &view, (2, 3), 1.0, Modifiers::default());
        wait_for_text(cx, &view, "^[OA");
    }

    #[gpui::test]
    fn shift_with_the_wheel_scrolls_the_view_even_while_the_program_has_the_mouse(
        cx: &mut TestAppContext,
    ) {
        // the program reports the mouse, and the main screen has a scrollback
        let handle = terminal_window(cx);
        let view = open(
            cx,
            handle,
            program(
                r"seq 1 500; stty -icanon -echo; printf '\033[?1000h\033[?1006h'; echo ready; exec cat -v",
            ),
        );
        wait_for_text(cx, &view, "ready");
        laid_out(cx, &view);
        let bottom = first_number(cx, &view);
        wheel(cx, handle, &view, (2, 3), 3.0, shift());
        assert_eq!(first_number(cx, &view), bottom - 3);
        assert!(!screen(cx, &view).contains("^["), "{}", screen(cx, &view));
    }

    // ---- links ----

    /// A link with OSC 8 (https), one with OSC 8 that is not http (ftp), and an address that is only printed.
    const LINKS: &str = r"printf '\033]8;;https://example.com/x\033\\link\033]8;;\033\\ \033]8;;ftp://files.example/f\033\\files\033]8;;\033\\ plain http://plain.example/y end\n'; exec cat";

    fn links() -> (&'static str, &'static str) {
        // "link" is columns 0..=3, "files" 5..=9, "plain" 11..=15, the address from 17
        ("https://example.com/x", "http://plain.example/y")
    }

    #[gpui::test]
    fn a_link_is_opened_after_the_question_and_only_when_it_is_http_or_https(
        cx: &mut TestAppContext,
    ) {
        let (handle, view) = started_at(cx, LINKS, "end");
        let (osc, plain) = links();

        // a click without the key is not a link click: it selects
        link_click(cx, handle, &view, (0, 1), Modifiers::default());
        assert!(!has_prompt(cx, handle));
        assert_eq!(cx.opened_url(), None);

        // the question shows the address; "no" opens nothing
        link_click(cx, handle, &view, (0, 1), link_key());
        assert!(has_prompt(cx, handle));
        let (message, _) = cx.pending_prompt().expect("a question");
        assert!(message.contains(osc), "{message}");
        assert!(view.read_with(cx, |view, _| view.asking_about_a_link()));
        // a second click while the question is open asks nothing more
        link_click(cx, handle, &view, (0, 1), link_key());
        answer(cx, handle, NO);
        assert!(!has_prompt(cx, handle), "one question only");
        assert_eq!(cx.opened_url(), None);
        assert!(!view.read_with(cx, |view, _| view.asking_about_a_link()));

        // "yes" opens it
        link_click(cx, handle, &view, (0, 1), link_key());
        answer(cx, handle, YES);
        assert_eq!(cx.opened_url().as_deref(), Some(osc));

        // an OSC 8 link that is not http or https is refused with a message, and nothing is opened
        link_click(cx, handle, &view, (0, 6), link_key());
        let (message, _) = cx.pending_prompt().expect("a message");
        assert!(message.contains("ftp://files.example/f"), "{message}");
        assert!(message.contains("http と https"), "{message}");
        answer(cx, handle, "OK");
        assert_eq!(cx.opened_url().as_deref(), Some(osc), "ftp was not opened");

        // an address that is only printed is a link too
        link_click(cx, handle, &view, (0, 20), link_key());
        answer(cx, handle, YES);
        assert_eq!(cx.opened_url().as_deref(), Some(plain));

        // text that is not a link: the key does nothing
        link_click(cx, handle, &view, (0, 12), link_key());
        assert!(!has_prompt(cx, handle));
    }

    #[gpui::test]
    fn the_link_under_the_pointer_is_underlined_only_while_the_key_is_held(
        cx: &mut TestAppContext,
    ) {
        let (handle, view) = started_at(cx, LINKS, "end");
        let (osc, plain) = links();
        let hovered = |cx: &mut TestAppContext| view.read_with(cx, |view, _| view.hovered_uri());
        let over = |cx: &mut TestAppContext, cell: (usize, usize), modifiers: Modifiers| {
            let position = at(cx, &view, cell);
            visual(cx, handle).simulate_mouse_move(position, None, modifiers);
            cx.run_until_parked();
        };
        over(cx, (0, 1), Modifiers::default());
        assert_eq!(hovered(cx), None);
        over(cx, (0, 1), link_key());
        assert_eq!(hovered(cx).as_deref(), Some(osc));
        over(cx, (0, 20), link_key());
        assert_eq!(hovered(cx).as_deref(), Some(plain));
        over(cx, (0, 12), link_key());
        assert_eq!(hovered(cx), None, "the text between the links");
        over(cx, (0, 1), link_key());
        assert_eq!(hovered(cx).as_deref(), Some(osc));
        // the key is let go with the pointer still
        visual(cx, handle).simulate_modifiers_change(Modifiers::default());
        cx.run_until_parked();
        assert_eq!(hovered(cx), None);
    }

    #[gpui::test]
    fn a_link_is_not_opened_when_the_tab_was_closed_while_the_question_was_open(
        cx: &mut TestAppContext,
    ) {
        let (handle, view) = started_at(cx, LINKS, "end");
        link_click(cx, handle, &view, (0, 1), link_key());
        assert!(has_prompt(cx, handle));
        let ws = workspace(cx, handle);
        handle
            .update(cx, |_, window, app| {
                ws.update(app, |ws, cx| ws.close_shell_now(&view, window, cx));
            })
            .unwrap();
        cx.run_until_parked();
        answer(cx, handle, YES);
        assert_eq!(cx.opened_url(), None);
    }

    #[gpui::test]
    fn ctrl_click_goes_to_the_program_that_has_the_mouse_and_shift_makes_it_a_link_click(
        cx: &mut TestAppContext,
    ) {
        let (handle, view) = started_at(
            cx,
            &LINKS.replace("exec cat", "exec cat -v").replace(
                "printf '",
                r"stty -icanon -echo; printf '\033[?1000h\033[?1006h'; printf '",
            ),
            "end",
        );
        link_click(cx, handle, &view, (0, 1), link_key());
        // the program has it, with the Ctrl bit (16)
        if !cfg!(target_os = "macos") {
            wait_for_text(cx, &view, "^[[<16;2;1M");
        }
        assert!(
            !has_prompt(cx, handle),
            "no question: the program got the click"
        );
        let mut shifted = link_key();
        shifted.shift = true;
        link_click(cx, handle, &view, (0, 1), shifted);
        assert!(
            has_prompt(cx, handle),
            "Shift gives the mouse back to the user"
        );
        answer(cx, handle, NO);
    }

    // ---- the title ----

    #[gpui::test]
    fn the_program_sets_the_title_of_the_tab_and_resetting_it_goes_back_to_the_name(
        cx: &mut TestAppContext,
    ) {
        // push the (empty) title, set one, and pop it after a line is typed
        let (handle, view) = started(
            cx,
            r"printf '\033[22t\033]0;my title\007ready\n'; read a; printf '\033[23t'; exec cat",
        );
        let caption = |cx: &mut TestAppContext| {
            workspace(cx, handle).read_with(cx, |ws, cx| ws.tab_panel(1).unwrap().caption(cx))
        };
        wait_until(cx, "the title", |cx| caption(cx) == "my title");
        assert_eq!(
            view.read_with(cx, |view, _| view.title_text().to_owned()),
            "my title"
        );
        // the accessible name of the tab follows it
        let name = workspace(cx, handle)
            .read_with(cx, |ws, cx| ws.tab_panel(1).unwrap().accessible_name(cx));
        assert!(name.contains("my title"), "{name}");
        keys(cx, handle, "enter");
        wait_until(cx, "the name of the shell", |cx| caption(cx) == "sh");
        assert_eq!(
            view.read_with(cx, |view, _| view.program_title().map(str::to_owned)),
            None
        );
    }

    #[gpui::test]
    fn a_long_title_is_cut_before_it_is_shown(cx: &mut TestAppContext) {
        let long = "x".repeat(300);
        let script = format!(r"printf '\033]0;{long}\007ready\n'; exec cat");
        let (_handle, view) = started(cx, &script);
        wait_until(cx, "the title", |cx| {
            view.read_with(cx, |view, _| {
                view.title_text().chars().count() == matsu_terminal::MAX_TITLE_CHARS
            })
        });
    }

    // ---- the bell ----

    #[gpui::test]
    fn bells_that_come_quickly_are_one_and_mark_the_tab_until_it_is_looked_at(
        cx: &mut TestAppContext,
    ) {
        // three bells at once, then a title: the title comes after the bells in the same stream, so once it is
        // shown the three bells have been handled
        let (handle, view) = started(
            cx,
            r"printf 'ready\n'; read a; printf '\007\007\007\033]0;after\007'; exec cat",
        );
        let caption = |cx: &mut TestAppContext| {
            workspace(cx, handle).read_with(cx, |ws, cx| ws.tab_panel(1).unwrap().caption(cx))
        };
        assert!(!caption(cx).starts_with('●'));
        keys(cx, handle, "enter");
        wait_until(cx, "the title after the bells", |cx| {
            view.read_with(cx, |view, _| view.title_text() == "after")
        });
        assert_eq!(view.read_with(cx, |view, _| view.bells_rung()), 1);
        // the window is in the background in a test, so the tab is marked
        assert_eq!(caption(cx), "● after");
        // looking at the tab again takes the mark away
        handle
            .update(cx, |_, _, app| {
                view.update(app, |view, cx| view.clear_bell_mark(cx));
            })
            .unwrap();
        cx.run_until_parked();
        assert_eq!(caption(cx), "after");
    }

    // ---- OSC 52 ----

    #[gpui::test]
    fn the_program_writes_the_clipboard_with_osc_52_and_cannot_read_it(cx: &mut TestAppContext) {
        set_clipboard(cx, "mine");
        // "hello" to the clipboard and "world" to the primary selection (the clipboard where there is none);
        // then a read request, which is refused: nothing is written back to the program
        let (_handle, view) = started(
            cx,
            r"printf '\033]52;c;aGVsbG8=\007\033]52;p;d29ybGQ=\007\033]52;c;?\007ready\n'; stty -icanon -echo; exec cat -v",
        );
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        {
            wait_until(cx, "the clipboard", |cx| {
                clipboard(cx).as_deref() == Some("hello")
            });
            let primary = cx
                .update(|app| app.read_from_primary())
                .and_then(|item| item.text());
            assert_eq!(primary.as_deref(), Some("world"));
        }
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        wait_until(cx, "the clipboard", |cx| {
            clipboard(cx).as_deref() == Some("world")
        });
        // a refused read answers nothing: `cat -v` has echoed no response
        assert!(
            !screen(cx, &view).contains("^[]52"),
            "{}",
            screen(cx, &view)
        );
    }
}
