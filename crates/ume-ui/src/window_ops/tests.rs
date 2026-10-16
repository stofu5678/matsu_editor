//! Tests of the window functions in GPUI's headless test windows. The OS is a fake [`Screen`]
//! that remembers the rectangles, so the flows run as they do on a desktop; the calls into the OS
//! themselves (`ume_platform::window`) are not run here.

use std::collections::HashMap;

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use ume_commands::command::Arg;
use ume_commands::sakura::FUNCTIONS;
use ume_platform::window::Error;
use ume_platform::window::arrange::{Direction, Rect};

use super::{
    Screen, all_handles, cascade_windows, maximize_window, minimize_windows, tile_windows, topmost,
    window_after,
};
use crate::document::SharedDocument;
use crate::file_ops::windows::{editors, open_editor_window};
use crate::view::EditorView;

const DONT_SAVE: &str = "保存しない(N)";
const CANCEL: &str = "キャンセル";

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

struct Win {
    handle: AnyWindowHandle,
    view: Entity<EditorView>,
}

impl Win {
    fn run(&self, cx: &mut TestAppContext, function: &str) -> bool {
        self.run_with(cx, function, &[])
    }

    fn run_with(&self, cx: &mut TestAppContext, function: &str, args: &[Arg]) -> bool {
        let code = code_of(function);
        let view = self.view.clone();
        let ok = self
            .handle
            .update(cx, |_, window, app| {
                view.update(app, |view, cx| view.run_function(code, args, window, cx))
            })
            .unwrap();
        cx.run_until_parked();
        ok
    }

    fn cx<'a>(&self, cx: &'a mut TestAppContext) -> &'a mut VisualTestContext {
        VisualTestContext::from_window(self.handle, cx).into_mut()
    }

    fn type_text(&self, cx: &mut TestAppContext, text: &str) {
        self.cx(cx).simulate_input(text);
    }

    fn notice(&self, cx: &mut TestAppContext) -> Option<String> {
        self.view.read_with(cx, |view, _| view.notice.clone())
    }

    fn has_list(&self, cx: &mut TestAppContext) -> bool {
        self.view
            .read_with(cx, |view, _| view.window_list.is_some())
    }

    fn is_open(&self, cx: &mut TestAppContext) -> bool {
        cx.windows().contains(&self.handle)
    }
}

/// `count` untitled windows, oldest first.
fn windows(cx: &mut TestAppContext, count: usize) -> Vec<Win> {
    cx.update(gpui_component::init);
    (0..count)
        .map(|_| {
            let handle = cx
                .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
                .unwrap();
            cx.run_until_parked();
            let view = cx.update(|app| {
                editors(app)
                    .into_iter()
                    .find(|(h, _)| *h == handle)
                    .map(|(_, view)| view)
                    .unwrap()
            });
            Win { handle, view }
        })
        .collect()
}

fn answer(w: &Win, cx: &mut TestAppContext, button: &str) {
    let cx = w.cx(cx);
    assert!(cx.has_pending_prompt(), "no prompt to answer {button:?}");
    cx.simulate_prompt_answer(button);
    cx.run_until_parked();
}

// ---- the arranging, on a fake screen ----

#[derive(Default)]
struct FakeScreen {
    work: Option<Rect>,
    frames: HashMap<AnyWindowHandle, Rect>,
    minimized: Vec<AnyWindowHandle>,
    topmost: HashMap<AnyWindowHandle, bool>,
    /// The windows in the order they were placed.
    placed: Vec<(AnyWindowHandle, Rect)>,
    raised: Vec<AnyWindowHandle>,
    minimized_now: Vec<AnyWindowHandle>,
}

const WORK: Rect = Rect::new(0, 0, 1000, 800);

impl FakeScreen {
    fn new() -> Self {
        Self {
            work: Some(WORK),
            ..Self::default()
        }
    }

    fn rect_of(&self, handle: AnyWindowHandle) -> Rect {
        self.placed.iter().find(|(h, _)| *h == handle).unwrap().1
    }
}

impl Screen for FakeScreen {
    fn work_area(&mut self, _: AnyWindowHandle) -> Result<Rect, Error> {
        self.work.ok_or(Error::Unsupported("no screen"))
    }

    fn frame(&mut self, window: AnyWindowHandle) -> Result<Rect, Error> {
        self.frames
            .get(&window)
            .copied()
            .ok_or(Error::Failed("frame"))
    }

    fn is_minimized(&mut self, window: AnyWindowHandle) -> Result<bool, Error> {
        Ok(self.minimized.contains(&window))
    }

    fn place(&mut self, window: AnyWindowHandle, rect: Rect) -> Result<(), Error> {
        self.placed.push((window, rect));
        Ok(())
    }

    fn cascade_step(&self) -> (i32, i32) {
        (30, 40)
    }

    fn minimize(&mut self, window: AnyWindowHandle) {
        self.minimized_now.push(window);
    }

    fn raise(&mut self, window: AnyWindowHandle) {
        self.raised.push(window);
    }

    fn is_topmost(&mut self, window: AnyWindowHandle) -> Result<bool, Error> {
        Ok(self.topmost.get(&window).copied().unwrap_or(false))
    }

    fn set_topmost(&mut self, window: AnyWindowHandle, topmost: bool) -> Result<(), Error> {
        self.topmost.insert(window, topmost);
        Ok(())
    }
}

fn handles(cx: &mut TestAppContext) -> Vec<AnyWindowHandle> {
    cx.update(all_handles)
}

#[gpui::test]
fn the_windows_are_listed_in_the_order_they_were_opened(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    let all = handles(cx);
    assert_eq!(all, [w[0].handle, w[1].handle, w[2].handle]);
}

#[gpui::test]
fn a_cascade_puts_the_current_window_last_and_on_top(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    let all = handles(cx);
    let mut screen = FakeScreen::new();
    cascade_windows(&mut screen, &all, w[0].handle).unwrap();
    // The others in order, then the current window.
    let order: Vec<_> = screen.placed.iter().map(|(h, _)| *h).collect();
    assert_eq!(order, [w[1].handle, w[2].handle, w[0].handle]);
    assert_eq!(screen.rect_of(w[1].handle), Rect::at(0, 0, 800, 640));
    assert_eq!(screen.rect_of(w[2].handle), Rect::at(30, 40, 800, 640));
    assert_eq!(screen.rect_of(w[0].handle), Rect::at(60, 80, 800, 640));
    // Raised in the same order: the current window ends on top.
    assert_eq!(screen.raised, order);
}

#[gpui::test]
fn a_cascade_leaves_minimized_windows_alone(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    let all = handles(cx);
    let mut screen = FakeScreen::new();
    screen.minimized.push(w[1].handle);
    cascade_windows(&mut screen, &all, w[2].handle).unwrap();
    let order: Vec<_> = screen.placed.iter().map(|(h, _)| *h).collect();
    assert_eq!(order, [w[0].handle, w[2].handle]);
}

#[gpui::test]
fn tiles_share_the_work_area_with_the_current_window_first(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    let all = handles(cx);

    let mut screen = FakeScreen::new();
    tile_windows(&mut screen, &all, w[2].handle, false).unwrap();
    assert_eq!(screen.rect_of(w[2].handle), Rect::at(0, 0, 333, 800));
    assert_eq!(screen.rect_of(w[1].handle), Rect::at(333, 0, 333, 800));
    assert_eq!(screen.rect_of(w[0].handle), Rect::at(666, 0, 333, 800));
    // The current window gets the focus.
    assert_eq!(screen.raised, [w[2].handle]);

    let mut screen = FakeScreen::new();
    tile_windows(&mut screen, &all, w[0].handle, true).unwrap();
    assert_eq!(screen.rect_of(w[0].handle), Rect::at(0, 0, 1000, 266));
    assert_eq!(screen.rect_of(w[1].handle), Rect::at(0, 266, 1000, 266));
    assert_eq!(screen.rect_of(w[2].handle), Rect::at(0, 532, 1000, 266));
}

#[gpui::test]
fn minimized_windows_take_no_room_in_a_tile(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    let all = handles(cx);
    let mut screen = FakeScreen::new();
    screen.minimized.push(w[0].handle);
    tile_windows(&mut screen, &all, w[1].handle, false).unwrap();
    assert_eq!(screen.placed.len(), 2);
    assert_eq!(screen.rect_of(w[1].handle), Rect::at(0, 0, 500, 800));
    assert_eq!(screen.rect_of(w[2].handle), Rect::at(500, 0, 500, 800));
}

#[gpui::test]
fn maximizing_in_one_direction_keeps_the_other(cx: &mut TestAppContext) {
    let w = windows(cx, 1);
    let mut screen = FakeScreen::new();
    screen
        .frames
        .insert(w[0].handle, Rect::at(200, 100, 500, 400));
    maximize_window(&mut screen, w[0].handle, true).unwrap();
    assert_eq!(screen.rect_of(w[0].handle), Rect::at(200, 0, 500, 800));
    maximize_window(&mut screen, w[0].handle, false).unwrap();
    assert_eq!(screen.placed[1].1, Rect::at(0, 100, 1000, 400));
}

#[gpui::test]
fn topmost_switches_sets_and_clears(cx: &mut TestAppContext) {
    let w = windows(cx, 1);
    let h = w[0].handle;
    let mut screen = FakeScreen::new();
    topmost(&mut screen, h, 0).unwrap();
    assert!(screen.topmost[&h], "0 switches it on");
    topmost(&mut screen, h, 0).unwrap();
    assert!(!screen.topmost[&h], "and off again");
    topmost(&mut screen, h, 1).unwrap();
    assert!(screen.topmost[&h]);
    topmost(&mut screen, h, 1).unwrap();
    assert!(screen.topmost[&h]);
    topmost(&mut screen, h, 2).unwrap();
    assert!(!screen.topmost[&h]);
    // Any other number does nothing.
    topmost(&mut screen, h, 7).unwrap();
    assert!(!screen.topmost[&h]);
}

#[gpui::test]
fn a_screen_that_cannot_place_windows_is_an_error_and_moves_nothing(cx: &mut TestAppContext) {
    let w = windows(cx, 2);
    let all = handles(cx);
    let mut screen = FakeScreen::new();
    screen.work = None;
    assert!(cascade_windows(&mut screen, &all, w[0].handle).is_err());
    assert!(tile_windows(&mut screen, &all, w[0].handle, true).is_err());
    assert!(screen.placed.is_empty());
}

// ---- the next and the previous window ----

#[gpui::test]
fn the_next_and_the_previous_window_wrap_around(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    let all = handles(cx);
    let next = |from: usize| window_after(&all, w[from].handle, Direction::Next);
    let prev = |from: usize| window_after(&all, w[from].handle, Direction::Previous);
    assert_eq!(next(0), Some(w[1].handle));
    assert_eq!(next(2), Some(w[0].handle));
    assert_eq!(prev(0), Some(w[2].handle));
    assert_eq!(prev(1), Some(w[0].handle));
    // A window that is not in the list goes nowhere.
    assert_eq!(window_after(&all[..2], w[2].handle, Direction::Next), None);
}

#[gpui::test]
fn the_next_window_function_brings_the_next_window_to_the_front(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    assert!(w[0].run(cx, "F_NEXTWINDOW"));
    assert_eq!(cx.update(|app| app.active_window()), Some(w[1].handle));
    assert!(w[1].run(cx, "F_PREVWINDOW"));
    assert_eq!(cx.update(|app| app.active_window()), Some(w[0].handle));
    assert!(w[0].run(cx, "F_PREVWINDOW"));
    assert_eq!(cx.update(|app| app.active_window()), Some(w[2].handle));
}

// ---- closing all or the others ----

#[gpui::test]
fn close_all_closes_every_window_without_changes(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    assert!(w[1].run(cx, "F_WIN_CLOSEALL"));
    assert!(w.iter().all(|w| !w.is_open(cx)));
}

#[gpui::test]
fn close_all_asks_about_a_modified_window_and_cancel_stops(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    w[1].type_text(cx, "x");
    w[0].run(cx, "F_WIN_CLOSEALL");
    // The others go first, in order: window 1 asks, and cancel keeps it and the rest.
    assert!(w[1].is_open(cx));
    answer(&w[1], cx, CANCEL);
    assert!(
        w[0].is_open(cx),
        "the window that runs it is the last to close"
    );
    assert!(w[1].is_open(cx));

    // Run again and let the modified one go: now all of them close.
    w[0].run(cx, "F_WIN_CLOSEALL");
    answer(&w[1], cx, DONT_SAVE);
    assert!(w.iter().all(|w| !w.is_open(cx)));
}

#[gpui::test]
fn close_all_keeps_the_windows_closed_before_a_cancel(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    w[2].type_text(cx, "x");
    w[0].run(cx, "F_WIN_CLOSEALL");
    // Window 1 had no changes and is closed; window 2 asks.
    assert!(!w[1].is_open(cx));
    answer(&w[2], cx, CANCEL);
    assert!(w[2].is_open(cx));
    assert!(w[0].is_open(cx));
}

#[gpui::test]
fn close_others_keeps_the_current_window(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    w[2].type_text(cx, "x");
    w[1].run(cx, "F_TAB_CLOSEOTHER");
    // Window 0 has no changes and goes; window 2 asks.
    assert!(!w[0].is_open(cx));
    answer(&w[2], cx, DONT_SAVE);
    assert!(!w[2].is_open(cx));
    assert!(w[1].is_open(cx), "the window that runs it stays");
}

#[gpui::test]
fn close_others_stops_at_a_cancel(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    w[0].type_text(cx, "x");
    w[1].run(cx, "F_TAB_CLOSEOTHER");
    assert!(w[0].is_open(cx));
    answer(&w[0], cx, CANCEL);
    assert!(w[0].is_open(cx));
    assert!(w[1].is_open(cx));
    // The window after the cancelled one is not asked.
    assert!(w[2].is_open(cx));
}

// ---- the functions that need the OS, where there is none ----

#[gpui::test]
fn the_functions_that_need_the_os_say_so_in_a_headless_window(cx: &mut TestAppContext) {
    let w = windows(cx, 2);
    for function in [
        "F_CASCADE",
        "F_TILE_V",
        "F_TILE_H",
        "F_MAXIMIZE_V",
        "F_MAXIMIZE_H",
        "F_TOPMOST",
    ] {
        assert!(!w[0].run(cx, function), "{function}");
        let notice = w[0].notice(cx).expect(function);
        assert!(
            notice.contains(&function.to_lowercase().replace("f_", "")),
            "{notice}"
        );
    }
}

#[gpui::test]
fn the_argument_of_topmost_is_a_number(cx: &mut TestAppContext) {
    let w = windows(cx, 1);
    assert!(!w[0].run_with(cx, "F_TOPMOST", &[Arg::Text("x".into())]));
    assert!(w[0].notice(cx).unwrap().contains("bad arguments"));
}

// ---- minimize all, redraw, output ----

#[gpui::test]
fn minimize_all_minimizes_every_window(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    let all = handles(cx);
    let mut screen = FakeScreen::new();
    minimize_windows(&mut screen, &all);
    assert_eq!(
        screen.minimized_now,
        [w[0].handle, w[1].handle, w[2].handle]
    );
}

#[gpui::test]
fn redraw_runs(cx: &mut TestAppContext) {
    let w = windows(cx, 1);
    assert!(w[0].run(cx, "F_REDRAW"));
}

#[gpui::test]
fn the_output_function_shows_the_macro_output_panel(cx: &mut TestAppContext) {
    let w = windows(cx, 1);
    assert!(!w[0].view.read_with(cx, |v, _| v.macros.output.is_open()));
    assert!(w[0].run(cx, "F_WIN_OUTPUT"));
    assert!(w[0].view.read_with(cx, |v, _| v.macros.output.is_open()));
}

// ---- the window list ----

#[gpui::test]
fn the_popup_chooses_a_window_by_its_access_key(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    assert!(w[0].run(cx, "F_WINLIST"));
    assert!(w[0].has_list(cx));
    // Keys that are not an access key are ignored.
    w[0].cx(cx).simulate_keystrokes("9");
    assert!(w[0].has_list(cx));
    w[0].cx(cx).simulate_keystrokes("3");
    cx.run_until_parked();
    assert!(!w[0].has_list(cx));
    assert_eq!(cx.update(|app| app.active_window()), Some(w[2].handle));
}

#[gpui::test]
fn the_popup_moves_with_the_arrows_and_closes_with_escape(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    w[1].run(cx, "F_WINLIST");
    // It starts on the current window.
    w[1].cx(cx).simulate_keystrokes("down enter");
    cx.run_until_parked();
    assert!(!w[1].has_list(cx));
    assert_eq!(cx.update(|app| app.active_window()), Some(w[2].handle));

    w[0].run(cx, "F_WINLIST");
    assert!(w[0].has_list(cx));
    w[0].cx(cx).simulate_keystrokes("escape");
    assert!(!w[0].has_list(cx));
    // Running it again closes it, too.
    w[0].run(cx, "F_WINLIST");
    w[0].run(cx, "F_WINLIST");
    assert!(!w[0].has_list(cx));
}

#[gpui::test]
fn the_dialog_checks_windows_and_closes_them(cx: &mut TestAppContext) {
    let w = windows(cx, 3);
    assert!(w[0].run(cx, "F_DLGWINLIST"));
    assert!(w[0].has_list(cx));
    // Check the 2nd and the 3rd window, then press the close button.
    w[0].cx(cx).simulate_keystrokes("down space down space");
    let list = w[0]
        .view
        .read_with(cx, |v, _| v.window_list.clone().unwrap());
    w[0].handle
        .update(cx, |_, window, app| {
            list.update(app, |list, cx| {
                list.run_on_checked("F_WINCLOSE", window, cx);
            });
        })
        .unwrap();
    cx.run_until_parked();
    assert!(w[0].is_open(cx));
    assert!(!w[1].is_open(cx));
    assert!(!w[2].is_open(cx));
    // The dialog stays open.
    assert!(w[0].has_list(cx));
}

#[gpui::test]
fn the_dialog_asks_before_it_closes_a_modified_window(cx: &mut TestAppContext) {
    let w = windows(cx, 2);
    w[1].type_text(cx, "x");
    w[0].run(cx, "F_DLGWINLIST");
    w[0].cx(cx).simulate_keystrokes("down space");
    let list = w[0]
        .view
        .read_with(cx, |v, _| v.window_list.clone().unwrap());
    w[0].handle
        .update(cx, |_, window, app| {
            list.update(app, |list, cx| {
                list.run_on_checked("F_WINCLOSE", window, cx)
            });
        })
        .unwrap();
    cx.run_until_parked();
    answer(&w[1], cx, CANCEL);
    assert!(w[1].is_open(cx));
}

#[gpui::test]
fn the_dialog_closes_with_escape_and_enter_chooses_a_window(cx: &mut TestAppContext) {
    let w = windows(cx, 2);
    w[0].run(cx, "F_DLGWINLIST");
    w[0].cx(cx).simulate_keystrokes("escape");
    assert!(!w[0].has_list(cx));
    w[0].run(cx, "F_DLGWINLIST");
    w[0].cx(cx).simulate_keystrokes("down enter");
    cx.run_until_parked();
    assert!(!w[0].has_list(cx));
    assert_eq!(cx.update(|app| app.active_window()), Some(w[1].handle));
}
