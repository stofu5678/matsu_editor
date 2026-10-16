//! Tests of the panes in GPUI's headless test window: a split shows the document in several
//! panes, edits in one pane reach the others, every pane has its own caret, and the commands act
//! on the active pane.

use gpui::{AppContext as _, Entity, Focusable as _, TestAppContext, VisualTestContext};
use gpui_component::Root;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::view::{EditorView, RunFunction};

const F_SPLIT_V: u16 = 31310;
const F_SPLIT_H: u16 = 31311;
const F_SPLIT_VH: u16 = 31312;
const F_RECKEYMACRO: u16 = 31250;

struct Fixture {
    document: Entity<SharedDocument>,
    workspace: Entity<Workspace>,
}

fn setup<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let shared = document.clone();
    let made: std::rc::Rc<std::cell::RefCell<Option<Entity<Workspace>>>> = Default::default();
    let slot = made.clone();
    let (_root, vcx) = cx.add_window_view(move |window, cx| {
        let workspace = cx.new(|cx| Workspace::new(shared, window, cx));
        *slot.borrow_mut() = Some(workspace.clone());
        Root::new(workspace, window, cx)
    });
    // Focus events reach a window only while it is active.
    vcx.update(|window, _| window.activate_window());
    vcx.run_until_parked();
    let workspace = made.borrow().clone().unwrap();
    (
        Fixture {
            document,
            workspace,
        },
        vcx,
    )
}

impl Fixture {
    fn run(&self, cx: &mut VisualTestContext, code: u16) {
        cx.dispatch_action(RunFunction { code });
        cx.run_until_parked();
    }

    fn pane(&self, cx: &mut VisualTestContext, index: usize) -> Entity<EditorView> {
        self.workspace
            .read_with(cx, |w, _| w.panes.pane_at(index).cloned())
            .unwrap_or_else(|| panic!("pane {index} is not shown"))
    }

    fn grid(&self, cx: &mut VisualTestContext) -> (u8, u8) {
        self.workspace.read_with(cx, |w, _| w.panes.grid())
    }

    fn active(&self, cx: &mut VisualTestContext) -> usize {
        self.workspace.read_with(cx, |w, _| w.panes.active_index())
    }

    fn shown(&self, cx: &mut VisualTestContext) -> usize {
        self.workspace.read_with(cx, |w, _| w.panes.shown().len())
    }

    fn lines(&self, cx: &mut VisualTestContext) -> Vec<String> {
        self.document.read_with(cx, |shared, _| {
            let doc = shared.doc();
            (0..doc.line_count())
                .map(|i| doc.line(i).unwrap().unwrap().text)
                .collect()
        })
    }

    fn caret_of(&self, cx: &mut VisualTestContext, index: usize) -> Pos {
        self.pane(cx, index)
            .read_with(cx, |v, _| v.cursor().caret())
    }

    fn recording(&self, cx: &mut VisualTestContext, index: usize) -> bool {
        self.pane(cx, index)
            .read_with(cx, |v, _| v.is_recording_key_macro())
    }

    /// Moves the focus to the pane, as a click on it does.
    fn click(&self, cx: &mut VisualTestContext, index: usize) {
        let pane = self.pane(cx, index);
        cx.update(|window, cx| window.focus(&pane.focus_handle(cx), cx));
        // the focus changes, and tells the window, when the frame is drawn
        let size = cx.update(|window, _| window.viewport_size());
        cx.simulate_resize(gpui::size(size.width + gpui::px(1.), size.height));
        cx.run_until_parked();
    }
}

fn caret(cx: &mut VisualTestContext, pane: &Entity<EditorView>) -> Pos {
    pane.read_with(cx, |v, _| v.cursor().caret())
}

fn put_caret(cx: &mut VisualTestContext, pane: &Entity<EditorView>, pos: Pos) {
    pane.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(pos, false)).unwrap();
    });
}

fn long_text() -> String {
    (0..200).map(|i| format!("line {i}\n")).collect()
}

#[gpui::test]
fn an_unsplit_window_has_one_pane(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    assert_eq!(f.grid(cx), (1, 1));
    assert_eq!(f.shown(cx), 1);
    assert_eq!(f.active(cx), 0);
}

#[gpui::test]
fn the_split_functions_make_two_and_four_panes(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_V);
    assert_eq!((f.grid(cx), f.shown(cx)), ((2, 1), 2));
    f.run(cx, F_SPLIT_V);
    assert_eq!((f.grid(cx), f.shown(cx)), ((1, 1), 1));
    f.run(cx, F_SPLIT_H);
    assert_eq!((f.grid(cx), f.shown(cx)), ((1, 2), 2));
    f.run(cx, F_SPLIT_H);
    assert_eq!((f.grid(cx), f.shown(cx)), ((1, 1), 1));
    f.run(cx, F_SPLIT_VH);
    assert_eq!((f.grid(cx), f.shown(cx)), ((2, 2), 4));
    f.run(cx, F_SPLIT_VH);
    assert_eq!((f.grid(cx), f.shown(cx)), ((1, 1), 1));
}

#[gpui::test]
fn the_other_split_function_adds_or_removes_one_direction(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_V);
    f.run(cx, F_SPLIT_H);
    assert_eq!(f.grid(cx), (2, 2));
    // from four panes, F_SPLIT_V removes the rows and F_SPLIT_H the columns
    f.run(cx, F_SPLIT_V);
    assert_eq!(f.grid(cx), (1, 2));
    f.run(cx, F_SPLIT_V);
    assert_eq!(f.grid(cx), (2, 2));
    f.run(cx, F_SPLIT_H);
    assert_eq!(f.grid(cx), (2, 1));
}

#[gpui::test]
fn an_edit_in_one_pane_is_seen_in_the_others(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "日本語のテキスト\nabc");
    f.run(cx, F_SPLIT_VH);
    let (a, d) = (f.pane(cx, 0), f.pane(cx, 3));
    put_caret(cx, &a, Pos::new(0, 0));
    cx.simulate_input("あ");
    assert_eq!(f.lines(cx)[0], "あ日本語のテキスト");
    // the other pane lays the changed text out again and has the same rows
    let rows = |cx: &mut VisualTestContext, p: &Entity<EditorView>| {
        p.read_with(cx, |v, _| v.layout().row_count())
    };
    assert_eq!(rows(cx, &a), rows(cx, &d));
    // an edit in the last pane, in wrapped text, is seen in the first
    d.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(Pos::new(1, 3), false))
            .unwrap();
    });
    f.click(cx, 3);
    cx.simulate_input("def");
    assert_eq!(f.lines(cx)[1], "abcdef");
    assert_eq!(rows(cx, &a), rows(cx, &d));
}

#[gpui::test]
fn the_edit_moves_the_carets_of_the_other_panes_only_as_the_text_requires(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef");
    f.run(cx, F_SPLIT_H);
    let (left, right) = (f.pane(cx, 0), f.pane(cx, 1));
    put_caret(cx, &left, Pos::new(0, 1));
    put_caret(cx, &right, Pos::new(1, 3));
    f.click(cx, 0);
    cx.simulate_input("X");
    // each pane has its own caret
    assert_eq!(caret(cx, &left), Pos::new(0, 2));
    assert_eq!(caret(cx, &right), Pos::new(1, 3));
}

#[gpui::test]
fn every_pane_has_its_own_selection(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcdef");
    f.run(cx, F_SPLIT_H);
    let (left, right) = (f.pane(cx, 0), f.pane(cx, 1));
    left.update(cx, |v, cx| {
        v.run_with(cx, |c| c.move_caret(Pos::new(0, 3), true))
            .unwrap();
    });
    assert!(left.read_with(cx, |v, _| v.cursor().has_selection()));
    assert!(!right.read_with(cx, |v, _| v.cursor().has_selection()));
}

#[gpui::test]
fn a_new_pane_starts_with_the_view_of_the_pane_it_comes_from(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abcdef\nghi");
    let first = f.pane(cx, 0);
    put_caret(cx, &first, Pos::new(1, 2));
    f.run(cx, F_SPLIT_H);
    // CopyViewStatus
    assert_eq!(f.caret_of(cx, 1), Pos::new(1, 2));
    assert_eq!(f.active(cx), 0);
}

#[gpui::test]
fn splitting_up_and_down_makes_the_pane_with_the_caret_active(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    f.run(cx, F_SPLIT_V);
    // the caret is on the first row: the upper pane stays active
    assert_eq!(f.active(cx), 0);
    f.run(cx, F_SPLIT_V);

    // the caret is far below the first view: the lower pane, which starts one view further down
    // and so has the caret below its first row, becomes active
    let first = f.pane(cx, 0);
    put_caret(cx, &first, Pos::new(190, 0));
    f.run(cx, F_SPLIT_V);
    assert_eq!(f.active(cx), 2);
    let lower = f.pane(cx, 2);
    let upper = f.pane(cx, 0);
    let top =
        |cx: &mut VisualTestContext, p: &Entity<EditorView>| p.read_with(cx, |v, _| v.scroll_y());
    assert!(top(cx, &lower) > top(cx, &upper));
}

#[gpui::test]
fn clicking_a_pane_makes_it_active_and_commands_act_on_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef");
    f.run(cx, F_SPLIT_H);
    let (left, right) = (f.pane(cx, 0), f.pane(cx, 1));
    put_caret(cx, &left, Pos::new(0, 0));
    put_caret(cx, &right, Pos::new(1, 3));
    f.click(cx, 1);
    assert_eq!(f.active(cx), 1);
    cx.simulate_input("!");
    assert_eq!(f.lines(cx), ["abc", "def!"]);
    f.click(cx, 0);
    assert_eq!(f.active(cx), 0);
    cx.simulate_input("?");
    assert_eq!(f.lines(cx), ["?abc", "def!"]);
}

#[gpui::test]
fn the_title_and_the_status_follow_the_active_pane(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef");
    f.run(cx, F_SPLIT_H);
    let (left, right) = (f.pane(cx, 0), f.pane(cx, 1));
    put_caret(cx, &left, Pos::new(0, 1));
    put_caret(cx, &right, Pos::new(1, 2));
    let status = |cx: &mut VisualTestContext, f: &Fixture| {
        f.workspace.read_with(cx, |w, cx| {
            let s = w.editor().read(cx).status(cx);
            (s.line, s.column)
        })
    };
    assert_eq!(status(cx, &f), (1, 2));
    f.click(cx, 1);
    assert_eq!(status(cx, &f), (2, 3));
}

#[gpui::test]
fn unsplitting_keeps_pane_0_with_the_view_of_the_active_pane(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef");
    f.run(cx, F_SPLIT_H);
    let right = f.pane(cx, 1);
    put_caret(cx, &right, Pos::new(1, 2));
    f.click(cx, 1);
    f.run(cx, F_SPLIT_H);
    assert_eq!(f.grid(cx), (1, 1));
    assert_eq!(f.active(cx), 0);
    assert_eq!(f.caret_of(cx, 0), Pos::new(1, 2));
    // the one pane has the focus and takes the keys
    cx.simulate_input("z");
    assert_eq!(f.lines(cx), ["abc", "dezf"]);
}

#[gpui::test]
fn the_panes_that_went_away_do_not_keep_the_document_busy(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_VH);
    let gone = f.pane(cx, 3).downgrade();
    f.run(cx, F_SPLIT_VH);
    cx.run_until_parked();
    assert!(gone.upgrade().is_none());
    // an edit still lays out only the pane that is left
    cx.simulate_input("x");
    assert_eq!(f.lines(cx), ["xabc"]);
}

#[gpui::test]
fn the_macro_recording_follows_the_active_pane(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_H);
    f.run(cx, F_RECKEYMACRO);
    assert!(f.recording(cx, 0));
    f.click(cx, 1);
    assert!(f.recording(cx, 1));
    assert!(!f.recording(cx, 0));
    // the title of the window shows it
    assert!(
        f.workspace
            .read_with(cx, |w, cx| w.editor().read(cx).title(cx))
            .contains("記録中")
    );
}

#[gpui::test]
fn the_search_pattern_follows_the_active_pane(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_H);
    f.pane(cx, 0)
        .update(cx, |v, _| v.search.set_pattern("needle"));
    f.click(cx, 1);
    assert_eq!(
        f.pane(cx, 1)
            .read_with(cx, |v, _| v.search.pattern().to_owned()),
        "needle"
    );
}

#[gpui::test]
fn scrolling_one_pane_scrolls_the_pane_beside_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    f.run(cx, F_SPLIT_VH);
    let scroll =
        |cx: &mut VisualTestContext, i: usize| f.pane(cx, i).read_with(cx, |v, _| v.scroll());
    f.pane(cx, 0).update(cx, |v, cx| {
        v.set_scroll(Some(40.0), Some((200.0, false)));
        cx.notify();
    });
    cx.run_until_parked();
    // SyncScrollV: pane 1 takes the top row; SyncScrollH: pane 2 takes the left column
    assert_eq!(scroll(cx, 1).1, 200.0);
    assert_eq!(scroll(cx, 2).0, 40.0);
    // the pane diagonal to it takes neither
    assert_eq!(scroll(cx, 3), (0.0, 0.0));
    // and not the other way round for the vertical scroll of pane 2
    assert_eq!(scroll(cx, 2).1, 0.0);
}

#[gpui::test]
fn rows_of_panes_scroll_vertically_on_their_own(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    f.run(cx, F_SPLIT_V);
    f.pane(cx, 0).update(cx, |v, cx| {
        v.set_scroll(None, Some((100.0, false)));
        cx.notify();
    });
    cx.run_until_parked();
    // the lower pane was staggered by the split; it did not follow
    assert_ne!(f.pane(cx, 2).read_with(cx, |v, _| v.scroll_y()), 100.0);
}

#[gpui::test]
fn dragging_a_bar_to_the_edge_removes_the_split(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_H);
    f.workspace.update_in(cx, |w, window, cx| {
        let (width, _) = w.panes.area_size();
        // a bar 10 pixels from the left edge is not a split (the limit is 32)
        w.do_split(10.0, 0.0, window, cx);
        let _ = width;
    });
    assert_eq!(f.grid(cx), (1, 1));
    // a bar in the middle that is moved keeps the split
    f.run(cx, F_SPLIT_H);
    f.workspace.update_in(cx, |w, window, cx| {
        let (width, _) = w.panes.area_size();
        w.do_split(width / 4.0, 0.0, window, cx);
    });
    assert_eq!(f.grid(cx), (1, 2));
    let moved = f.workspace.read_with(cx, |w, _| w.panes.h);
    assert!((moved - 0.25).abs() < 0.01);
}

#[gpui::test]
fn the_window_without_a_split_is_exactly_the_one_editor(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_H);
    f.run(cx, F_SPLIT_H);
    // back to one pane: typing, undo and redo work as before
    cx.simulate_input("x");
    assert_eq!(f.lines(cx), ["xabc"]);
    f.run(cx, 30210);
    assert_eq!(f.lines(cx), ["abc"]);
}

#[gpui::test]
fn a_modified_document_is_one_document_for_every_pane(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_VH);
    f.click(cx, 2);
    cx.simulate_input("x");
    // the window asks one question about one document, whichever pane is asked
    for i in 0..4 {
        let modified = f
            .pane(cx, i)
            .read_with(cx, |v, cx| v.document().read(cx).model().is_modified());
        assert!(modified);
    }
    let same = f.pane(cx, 0).read_with(cx, |v, _| v.document().clone());
    let other = f.pane(cx, 3).read_with(cx, |v, _| v.document().clone());
    assert_eq!(same, other);
}

#[gpui::test]
fn a_mouse_click_in_a_pane_makes_it_active_and_puts_the_caret_there(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef\nghi");
    f.run(cx, F_SPLIT_H);
    let right = f.pane(cx, 1);
    // the cell at row 2, column 1 of the right pane, from what its last paint measured
    let at = right.read_with(cx, |v, _| {
        let paint = v.paint.expect("the pane was painted");
        let m = v.metrics;
        gpui::point(
            gpui::px(paint.origin_x + 1.1 * m.cell_width),
            gpui::px(paint.origin_y + 2.5 * m.line_height),
        )
    });
    cx.simulate_click(at, gpui::Modifiers::none());
    assert_eq!(f.active(cx), 1);
    assert_eq!(f.caret_of(cx, 1), Pos::new(2, 1));
    assert_eq!(f.caret_of(cx, 0), Pos::new(0, 0));
}

#[gpui::test]
fn the_area_is_measured_before_the_first_split(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    let area = f.workspace.read_with(cx, |w, _| w.panes.area.get());
    assert!(f32::from(area.size.height) > 0.0 && f32::from(area.size.width) > 0.0);
}

#[gpui::test]
fn synchronized_scrolling_keeps_the_top_row_when_the_font_sizes_differ(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    f.run(cx, F_SPLIT_H);
    // the right pane has a larger font (the zoom is per pane)
    f.click(cx, 1);
    f.run(cx, 31122);
    f.click(cx, 0);
    let line_height = |cx: &mut VisualTestContext, i: usize| {
        f.pane(cx, i).read_with(cx, |v, _| v.metrics().line_height)
    };
    let (small, large) = (line_height(cx, 0), line_height(cx, 1));
    assert!(large > small);
    f.pane(cx, 0).update(cx, |v, cx| {
        v.set_scroll(None, Some((small * 10.0, false)));
        cx.notify();
    });
    cx.run_until_parked();
    // the same top row, not the same pixels
    let y = f.pane(cx, 1).read_with(cx, |v, _| v.scroll_y());
    assert!((y - large * 10.0).abs() < 0.01, "{y} vs {}", large * 10.0);
}

#[gpui::test]
fn a_click_in_the_panel_of_a_pane_activates_the_pane(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_H);
    f.pane(cx, 0)
        .update(cx, |v, _| v.search.set_pattern("needle"));
    f.click(cx, 1);
    f.pane(cx, 1)
        .update(cx, |v, _| v.search.set_pattern("other"));
    // the click handler of the pane (it sits around the panels) calls this
    f.workspace
        .update_in(cx, |w, window, cx| w.activate(0, window, cx));
    assert_eq!(f.active(cx), 0);
    assert_eq!(
        f.pane(cx, 0)
            .read_with(cx, |v, _| v.search.pattern().to_owned()),
        "other"
    );
}

#[gpui::test]
fn unsplitting_keeps_the_top_row_of_a_pane_with_another_font_size(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, &long_text());
    f.run(cx, F_SPLIT_H);
    f.click(cx, 1);
    f.run(cx, 31122);
    let large = f.pane(cx, 1).read_with(cx, |v, _| v.metrics().line_height);
    f.pane(cx, 1).update(cx, |v, cx| {
        v.set_scroll(None, Some((large * 10.0, false)));
        cx.notify();
    });
    f.run(cx, F_SPLIT_H);
    let small = f.pane(cx, 0).read_with(cx, |v, _| v.metrics().line_height);
    let y = f.pane(cx, 0).read_with(cx, |v, _| v.scroll_y());
    assert!((y - small * 10.0).abs() < 0.01, "{y} vs {}", small * 10.0);
}

#[gpui::test]
fn releasing_a_bar_outside_the_area_removes_the_split(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, F_SPLIT_H);
    f.workspace.update_in(cx, |w, window, cx| {
        w.panes.dragging = Some(super::Bar::Cols);
        let area = w.panes.area.get();
        // no move was seen; the button goes up left of the area (OnLButtonUp, bHUp)
        let up = gpui::MouseUpEvent {
            button: gpui::MouseButton::Left,
            position: gpui::point(area.origin.x - gpui::px(50.), area.origin.y + gpui::px(10.)),
            modifiers: gpui::Modifiers::none(),
            click_count: 1,
        };
        w.end_drag(&up, window, cx);
    });
    assert_eq!(f.grid(cx), (1, 1));
}
