//! Tests of the minimap in GPUI's headless test windows: the settings, the toggle, what the pane
//! draws, and the mouse.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{
    AppContext as _, Entity, Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext,
    TouchPhase, VisualTestContext, point, px,
};
use gpui_component::Root;
use ume_config::Profile;
use ume_config::history::HistoryStore;
use ume_core::buffer::{Document, Pos};
use ume_core::model::DocumentModel;

use super::MiniMapSettings;
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::geometry::Metrics;
use crate::history::{HistoryOptions, HistoryState};
use crate::settings::UserSettings;
use crate::view::{EditorView, RunFunction};

const F_SHOWMINIMAP: u16 = 31104;

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn lines(n: usize) -> String {
    (0..n).map(|i| format!("line {i}\n")).collect()
}

struct Fixture {
    workspace: Entity<Workspace>,
}

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    settings: MiniMapSettings,
) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    cx.update(|cx| cx.set_global(settings));
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let made: std::rc::Rc<std::cell::RefCell<Option<Entity<Workspace>>>> = Default::default();
    let slot = made.clone();
    let (_root, vcx) = cx.add_window_view(move |window, cx| {
        let workspace = cx.new(|cx| Workspace::new(document, window, cx));
        *slot.borrow_mut() = Some(workspace.clone());
        Root::new(workspace, window, cx)
    });
    vcx.update(|window, _| window.activate_window());
    vcx.run_until_parked();
    let workspace = made.borrow().clone().unwrap();
    (Fixture { workspace }, vcx)
}

fn shown() -> MiniMapSettings {
    MiniMapSettings {
        shown: true,
        ..MiniMapSettings::default()
    }
}

impl Fixture {
    fn view(&self, cx: &mut VisualTestContext) -> Entity<EditorView> {
        self.workspace.read_with(cx, |w, _| w.editor().clone())
    }

    fn minimap(&self, cx: &mut VisualTestContext) -> Entity<super::panel::MiniMap> {
        self.workspace.read_with(cx, |w, _| w.minimap.clone())
    }

    /// The pane's area in the window, once it has been painted.
    fn area(&self, cx: &mut VisualTestContext) -> gpui::Bounds<gpui::Pixels> {
        self.minimap(cx).read_with(cx, |m, _| m.area.get())
    }

    fn top_row(&self, cx: &mut VisualTestContext) -> usize {
        self.minimap(cx).read_with(cx, |m, _| m.top_row)
    }

    fn caret(&self, cx: &mut VisualTestContext) -> Pos {
        self.view(cx).read_with(cx, |v, _| v.cursor.caret())
    }

    /// The point of the row `row` of the text, `dx` pixels from the left of the pane.
    fn point_of(
        &self,
        cx: &mut VisualTestContext,
        row: usize,
        dx: f32,
    ) -> gpui::Point<gpui::Pixels> {
        let area = self.area(cx);
        let top = self.top_row(cx);
        let row_height = super::geometry::row_height(MiniMapSettings::default().font_size);
        point(
            area.left() + px(dx),
            area.top() + px((row - top) as f32 * row_height + 1.0),
        )
    }
}

// ---- the settings ----

#[gpui::test]
fn without_a_file_the_minimap_is_off_with_sakuras_defaults(cx: &mut TestAppContext) {
    cx.update(|cx| super::install(cx, &UserSettings::default()));
    let s = cx.update(|cx| super::current(cx));
    assert_eq!(
        s,
        MiniMapSettings {
            shown: false,
            font_size: -2,
            quality: 3,
            width: 150,
        }
    );
}

#[gpui::test]
fn the_keys_of_the_file_are_read(cx: &mut TestAppContext) {
    let profile = profile(
        "[Common]\r\nbDispMiniMap=1\r\nnMiniMapFontSize=-3\r\nnMiniMapQuality=5\r\nnMiniMapWidth=200\r\n",
    );
    assert_eq!(
        MiniMapSettings::from_profile(&profile),
        MiniMapSettings {
            shown: true,
            font_size: -3,
            quality: 5,
            width: 200,
        }
    );
    let _ = cx;
}

#[test]
fn a_width_that_makes_no_sense_leaves_the_scroll_bar() {
    let narrow = MiniMapSettings {
        width: -5,
        ..MiniMapSettings::default()
    };
    assert!(narrow.pane_width() >= super::geometry::MIN_WIDTH);
    assert_eq!(MiniMapSettings::default().pane_width(), 150.0);
}

/// A settings file with `text`, read as `app::run` reads it, with the folder that goes with it.
struct Ini(PathBuf);

impl Ini {
    fn new(cx: &mut TestAppContext, text: &str) -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("ume-ui-minimap-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ume.ini");
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        std::fs::write(&path, bytes).unwrap();
        cx.update(|cx| {
            let user = UserSettings::load_from(std::slice::from_ref(&path));
            cx.set_global(HistoryState::on_disk(
                HistoryStore::new(path.clone(), false),
                HistoryOptions::default(),
            ));
            super::install(cx, &user);
            cx.set_global(user);
        });
        Self(path)
    }

    fn get(&self, key: &str) -> Option<String> {
        let profile = Profile::load(&self.0).ok()?;
        profile.get("Common", key).map(str::to_owned)
    }
}

impl Drop for Ini {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

#[gpui::test]
fn the_toggle_writes_only_its_key_and_keeps_the_rest_of_the_file(cx: &mut TestAppContext) {
    let ini = Ini::new(
        cx,
        "[Common]\r\nnMiniMapWidth=200\r\nnMiniMapFontSize=-3\r\n[Other]\r\nx=1\r\n",
    );
    assert!(!cx.update(|cx| super::current(cx)).shown);
    cx.update(super::toggle);
    assert!(cx.update(|cx| super::current(cx)).shown);
    assert_eq!(ini.get("bDispMiniMap").as_deref(), Some("1"));
    cx.update(super::toggle);
    assert!(!cx.update(|cx| super::current(cx)).shown);
    assert_eq!(ini.get("bDispMiniMap").as_deref(), Some("0"));
    // The other keys are read and stay as they were; neither is written by the toggle.
    let s = cx.update(|cx| super::current(cx));
    assert_eq!((s.width, s.font_size), (200, -3));
    assert_eq!(ini.get("nMiniMapWidth").as_deref(), Some("200"));
    let profile = Profile::load(&ini.0).unwrap();
    assert_eq!(profile.get("Other", "x"), Some("1"));
}

#[gpui::test]
fn a_file_with_the_minimap_on_starts_with_it_on(cx: &mut TestAppContext) {
    let _ini = Ini::new(cx, "[Common]\r\nbDispMiniMap=1\r\n");
    assert!(cx.update(|cx| super::current(cx)).shown);
}

#[test]
fn the_dialog_starts_from_the_state_of_the_windows() {
    let mut profile = profile("[Common]\r\nbDispMiniMap=0\r\n");
    super::put_into(&mut profile, shown());
    assert_eq!(profile.get("Common", "bDispMiniMap"), Some("1"));
}

// ---- the command and the window ----

#[gpui::test]
fn the_function_shows_and_hides_the_minimap_in_every_window(cx: &mut TestAppContext) {
    let (first, cx) = setup(cx, &lines(10), MiniMapSettings::default());
    assert!(first.minimap(cx).read_with(cx, |m, _| m.source().is_none()));
    cx.dispatch_action(RunFunction {
        code: F_SHOWMINIMAP,
    });
    cx.run_until_parked();
    assert!(cx.update(|_, cx| super::current(cx).shown));
    // the pane follows the active view of the window and has been painted
    let view = first.view(cx);
    assert_eq!(
        first.minimap(cx).read_with(cx, |m, _| m.source().cloned()),
        Some(view)
    );
    assert!(first.area(cx).size.width > px(0.));
    assert_eq!(f32::from(first.area(cx).size.width), 150.0);
    cx.dispatch_action(RunFunction {
        code: F_SHOWMINIMAP,
    });
    cx.run_until_parked();
    assert!(!cx.update(|_, cx| super::current(cx).shown));
}

#[gpui::test]
fn the_width_is_the_setting(cx: &mut TestAppContext) {
    let wide = MiniMapSettings {
        shown: true,
        width: 220,
        ..MiniMapSettings::default()
    };
    let (fixture, cx) = setup(cx, &lines(10), wide);
    assert_eq!(f32::from(fixture.area(cx).size.width), 220.0);
}

#[gpui::test]
fn the_menu_check_follows_the_state(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(3), MiniMapSettings::default());
    let checked = |cx: &mut VisualTestContext| {
        cx.update(|_, app| {
            let view = fixture.workspace.read(app).editor().read(app);
            crate::menu_state::FuncStates::of_view(view, false, false, 1, app)
                .is_checked(F_SHOWMINIMAP)
        })
    };
    assert!(!checked(cx));
    cx.dispatch_action(RunFunction {
        code: F_SHOWMINIMAP,
    });
    cx.run_until_parked();
    assert!(checked(cx));
}

// ---- what it draws ----

#[gpui::test]
fn only_the_rows_asked_for_are_given(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(5000), shown());
    let view = fixture.view(cx);
    let rows = view.read_with(cx, |v, app| v.minimap_rows(2000, 40, 100, app));
    assert_eq!(rows.len(), 40);
    assert_eq!(rows[0].row, 2000);
    assert_eq!(rows[0].pieces[0].text, "line 2000");
    assert_eq!(rows[39].row, 2039);
}

#[gpui::test]
fn a_long_line_is_cut_at_the_width_of_the_pane(cx: &mut TestAppContext) {
    let long = "x".repeat(100_000);
    let (fixture, cx) = setup(cx, &long, shown());
    let view = fixture.view(cx);
    let rows = view.read_with(cx, |v, app| v.minimap_rows(0, 5, 150, app));
    let given: usize = rows[0].pieces.iter().map(|p| p.text.chars().count()).sum();
    assert!(given <= 150, "{given}");
}

#[gpui::test]
fn the_text_follows_the_edits(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, "abc\ndef\n", shown());
    let first = |cx: &mut VisualTestContext| {
        fixture
            .view(cx)
            .read_with(cx, |v, app| v.minimap_rows(0, 3, 100, app))
            .iter()
            .map(|r| r.pieces.iter().map(|p| p.text.clone()).collect::<String>())
            .collect::<Vec<_>>()
    };
    assert_eq!(first(cx), ["abc", "def", ""]);
    cx.simulate_input("x");
    cx.run_until_parked();
    assert_eq!(first(cx), ["xabc", "def", ""]);
    cx.dispatch_action(crate::view::Undo);
    cx.run_until_parked();
    assert_eq!(first(cx), ["abc", "def", ""]);
}

#[gpui::test]
fn the_rows_the_window_shows_are_marked(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(500), shown());
    let view = fixture.view(cx);
    view.update(cx, |v, _| {
        v.set_metrics(Metrics {
            cell_width: 8.0,
            line_height: 20.0,
            view_width: 800.0,
            view_height: 200.0,
        });
        v.scroll_by(0.0, -100.0);
    });
    let page = view.read_with(cx, |v, _| v.minimap_page());
    // 10 rows fit, and the text is scrolled by 5 rows
    assert_eq!(page, 5..15);
    view.update(cx, |v, _| v.scroll_by(0.0, -200.0));
    assert_eq!(view.read_with(cx, |v, _| v.minimap_page()), 15..25);
}

// ---- the mouse ----

#[gpui::test]
fn a_click_moves_the_caret_to_the_start_of_the_row_and_the_window_follows(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(500), shown());
    fixture.view(cx).update(cx, |v, _| {
        v.set_metrics(Metrics {
            cell_width: 8.0,
            line_height: 20.0,
            view_width: 800.0,
            view_height: 200.0,
        })
    });
    let at = fixture.point_of(cx, 40, 30.0);
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(fixture.caret(cx), Pos::new(40, 0));
    // the text is scrolled so that the caret is in view
    let (page, caret_row) = fixture
        .view(cx)
        .read_with(cx, |v, _| (v.minimap_page(), v.cursor.caret().line));
    assert!(page.contains(&caret_row), "{page:?}");
}

#[gpui::test]
fn shift_click_extends_the_selection(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(100), shown());
    let at = fixture.point_of(cx, 10, 30.0);
    cx.simulate_click(at, Modifiers::none());
    let at = fixture.point_of(cx, 20, 30.0);
    cx.simulate_click(
        at,
        Modifiers {
            shift: true,
            ..Modifiers::none()
        },
    );
    cx.run_until_parked();
    let view = fixture.view(cx);
    let selection = view.read_with(cx, |v, _| v.cursor.selection());
    assert_eq!(
        selection.map(|(start, end)| (start.line, end.line)),
        Some((10, 20))
    );
    // without Shift the selection goes away
    let at = fixture.point_of(cx, 5, 30.0);
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert!(!view.read_with(cx, |v, _| v.cursor.has_selection()));
    assert_eq!(fixture.caret(cx), Pos::new(5, 0));
}

#[gpui::test]
fn dragging_follows_the_pointer_and_stops_at_the_button_up(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(300), shown());
    let none = Modifiers::none();
    let at = fixture.point_of(cx, 10, 30.0);
    cx.simulate_mouse_down(at, MouseButton::Left, none);
    assert_eq!(fixture.caret(cx), Pos::new(10, 0));
    let at = fixture.point_of(cx, 25, 30.0);
    cx.simulate_mouse_move(at, MouseButton::Left, none);
    assert_eq!(fixture.caret(cx), Pos::new(25, 0));
    let at = fixture.point_of(cx, 25, 30.0);
    cx.simulate_mouse_up(at, MouseButton::Left, none);
    // after the button is up, moving does nothing
    let at = fixture.point_of(cx, 60, 30.0);
    cx.simulate_mouse_move(at, None, none);
    assert_eq!(fixture.caret(cx), Pos::new(25, 0));
}

#[gpui::test]
fn dragging_below_the_pane_scrolls_the_minimap(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(3000), shown());
    assert_eq!(fixture.top_row(cx), 0);
    let area = fixture.area(cx);
    let none = Modifiers::none();
    let at = fixture.point_of(cx, 10, 30.0);
    cx.simulate_mouse_down(at, MouseButton::Left, none);
    // the pointer below the pane: the minimap scrolls and the caret goes down with it
    cx.simulate_mouse_move(
        point(area.left() + px(30.), area.bottom() + px(30.)),
        MouseButton::Left,
        none,
    );
    cx.simulate_mouse_up(
        point(area.left() + px(30.), area.bottom() + px(30.)),
        MouseButton::Left,
        none,
    );
    assert!(fixture.top_row(cx) > 0);
    assert!(fixture.caret(cx).line > 10);
}

#[gpui::test]
fn the_wheel_scrolls_the_minimap_ten_times_as_far_as_the_text(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(3000), shown());
    let area = fixture.area(cx);
    let event = |lines: f32| ScrollWheelEvent {
        position: area.center(),
        delta: ScrollDelta::Lines(point(0.0, lines)),
        modifiers: Modifiers::none(),
        touch_phase: TouchPhase::Moved,
    };
    // a notch of 3 lines, the wheel turned down (negative)
    cx.simulate_event(event(-3.0));
    assert_eq!(fixture.top_row(cx), 30);
    cx.simulate_event(event(3.0));
    assert_eq!(fixture.top_row(cx), 0);
    // not above the first row
    cx.simulate_event(event(3.0));
    assert_eq!(fixture.top_row(cx), 0);
}

#[gpui::test]
fn a_click_on_the_scroll_bar_does_not_move_the_caret(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(3000), shown());
    let area = fixture.area(cx);
    // the track below the thumb pages down
    let at = point(area.right() - px(5.), area.bottom() - px(5.));
    cx.simulate_click(at, Modifiers::none());
    cx.run_until_parked();
    assert_eq!(fixture.caret(cx), Pos::new(0, 0));
    assert!(fixture.top_row(cx) > 0);
}

#[gpui::test]
fn a_shorter_document_brings_the_top_row_back(cx: &mut TestAppContext) {
    let (fixture, cx) = setup(cx, &lines(3000), shown());
    let area = fixture.area(cx);
    cx.simulate_click(
        point(area.right() - px(5.), area.bottom() - px(5.)),
        Modifiers::none(),
    );
    assert!(fixture.top_row(cx) > 0);
    // everything is deleted: the minimap shows the first row again after the next paint
    cx.dispatch_action(RunFunction { code: 30401 }); // F_SELECTALL
    cx.simulate_input("x");
    cx.run_until_parked();
    assert_eq!(fixture.top_row(cx), 0);
}
