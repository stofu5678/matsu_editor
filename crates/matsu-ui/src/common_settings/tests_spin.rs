//! The spin buttons (up-down controls) of the number boxes of the settings sheets: which boxes
//! have one and its range (`SetUpDownRange` in `sakura_core/prop/` and `sakura_core/typeprop/`),
//! and that the arrows and the Up and Down keys move the number.

use gpui::{AppContext as _, Modifiers, TestAppContext};
use matsu_config::Profile;

use super::Page;
use super::dialog_window;
use super::model::{Control, Kind};
use super::tests::{ok, run, setup, type_in, visual, with_dialog};
use super::tests_names::drawn;

/// Every number box that has an up-down control in Sakura's settings sheets, with the range of
/// the control. `CPropCommon*::DispatchEvent` (`WM_INITDIALOG`) and `CPropTypes*`: each line
/// names the `IDC_SPIN_*` it is.
const SAKURA_SPINS: [(&str, (i32, i32)); 19] = [
    // CPropComGeneral.cpp: IDC_SPIN_REPEATEDSCROLLLINENUM, _MAX_MRU_FILE, _MAX_MRU_FOLDER
    ("repeated_scroll_lines", (1, 10)),
    ("max_mru_file", (0, 36)),
    ("max_mru_folder", (0, 36)),
    // CPropComWin.cpp: IDC_SPIN_nRulerHeight, _nRulerBottomSpace, _nLineNumberRightSpace,
    // _FUNCKEYWND_GROUPNUM
    ("ruler_height", (2, 32)),
    ("ruler_bottom_space", (0, 32)),
    ("line_number_right_space", (0, 32)),
    ("funckey_group_num", (1, 12)),
    // CPropComFile.cpp: IDC_SPIN_AUTOLOAD_DELAY, _nDropFileNumMax, _AUTOBACKUP_INTERVAL,
    // _ALERT_FILESIZE
    ("autoload_delay", (0, 32767)),
    ("drop_file_num_max", (1, 99)),
    ("auto_save_interval", (1, 32767)),
    ("alert_file_size", (1, 2048)),
    // CPropComBackup.cpp: IDC_SPIN_BACKUP_GENS
    ("backup_generations", (1, 99)),
    // CPropTypesScreen.cpp: IDC_SPIN_MAXLINELEN, _CHARSPACE, _LINESPACE, _TABSPACE
    ("max_line_ketas", (10, 10240)),
    ("column_space", (0, 64)),
    ("line_space", (-128, 128)),
    ("tab_space", (1, 64)),
    // CPropTypesWindow.cpp: IDC_SPIN_LINENUMWIDTH, IDC_UPDOWN_BACKIMG_TRANSPARENCY
    ("line_num_width", (2, 11)),
    ("bg_transparency", (0, 255)),
    // CPropTypesColor.cpp: IDC_SPIN_LCColNum, 2 and 3 (the last two are checked below)
    ("line_comment1_pos", (1, 1000)),
];

fn all_controls() -> Vec<Control> {
    let mut controls = Vec::new();
    for page in Page::ALL {
        drawn(&page.items().unwrap_or_default(), &mut controls);
    }
    let sheet = crate::type_settings::sheet(&Profile::default(), 0).expect("the first type");
    for page in &sheet.pages {
        drawn(page.items.as_deref().unwrap_or_default(), &mut controls);
    }
    controls
}

#[test]
fn the_number_boxes_have_the_spin_buttons_and_ranges_of_sakura() {
    let controls = all_controls();
    let spin = |id: &str| controls.iter().find(|c| c.id == id).and_then(|c| c.spin);
    for (id, range) in SAKURA_SPINS {
        assert_eq!(spin(id), Some(range), "{id}");
    }
    // the second and third line comment columns are the same control as the first
    assert_eq!(spin("line_comment2_pos"), Some((1, 1000)));
    assert_eq!(spin("line_comment3_pos"), Some((1, 1000)));
}

#[test]
fn a_number_box_that_sakura_gives_no_spin_has_none() {
    let controls = all_controls();
    let numbers: Vec<&Control> = controls
        .iter()
        .filter(|c| matches!(c.kind, Kind::Number { .. }))
        .collect();
    assert!(numbers.len() > 20, "{}", numbers.len());
    for control in &numbers {
        let in_sakura = SAKURA_SPINS.iter().any(|(id, _)| *id == control.id)
            || control.id == "line_comment2_pos"
            || control.id == "line_comment3_pos";
        assert_eq!(
            control.spin.is_some(),
            in_sakura,
            "{}: a number box has a spin button exactly where Sakura has an up-down control \
             (SAKURA_SPINS)",
            control.id
        );
    }
}

#[gpui::test]
fn the_up_and_down_keys_of_a_number_box_move_the_number(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_GENERAL");
    let window = cx.update(|app| dialog_window(app)).unwrap();
    type_in(cx, "repeated_scroll_lines", "3");
    // Alt+N: the focus goes to the box 行数(&N)
    cx.simulate_keystrokes(window, "alt-n");
    cx.simulate_keystrokes(window, "up");
    assert_eq!(text(cx, "repeated_scroll_lines"), "4");
    cx.simulate_keystrokes(window, "down down");
    assert_eq!(text(cx, "repeated_scroll_lines"), "2");
    // the ends of the range (1 to 10): no wrapping
    cx.simulate_keystrokes(window, "down down down");
    assert_eq!(text(cx, "repeated_scroll_lines"), "1");
    type_in(cx, "repeated_scroll_lines", "10");
    cx.simulate_keystrokes(window, "up");
    assert_eq!(text(cx, "repeated_scroll_lines"), "10");
    // a number typed out of the range: the step ends in the range
    type_in(cx, "repeated_scroll_lines", "50");
    cx.simulate_keystrokes(window, "down");
    assert_eq!(text(cx, "repeated_scroll_lines"), "10");
    // and it is what OK keeps
    ok(cx);
}

#[gpui::test]
fn the_arrows_of_a_number_box_move_the_number(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_GENERAL");
    let window = cx.update(|app| dialog_window(app)).unwrap();
    type_in(cx, "repeated_scroll_lines", "3");
    let cx = visual(cx, window);
    cx.run_until_parked();
    let click = |cx: &mut gpui::VisualTestContext, selector: &'static str| {
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("no {selector}"));
        cx.simulate_click(bounds.center(), Modifiers::none());
        cx.run_until_parked();
    };
    click(cx, "cs-repeated_scroll_lines-spin-up");
    assert_eq!(text_in(cx, "repeated_scroll_lines"), "4");
    click(cx, "cs-repeated_scroll_lines-spin-down");
    click(cx, "cs-repeated_scroll_lines-spin-down");
    assert_eq!(text_in(cx, "repeated_scroll_lines"), "2");
}

#[gpui::test]
fn a_number_box_without_a_spin_has_no_arrows(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_FILE");
    let window = cx.update(|app| dialog_window(app)).unwrap();
    let cx = visual(cx, window);
    cx.run_until_parked();
    // 警告するファイルの大きさ (`IDC_SPIN_ALERT_FILESIZE`) has them; the size from which a file is
    // read by ranges is MATSU Editor's own and has none.
    assert!(cx.debug_bounds("cs-alert_file_size-spin-up").is_some());
    assert!(cx.debug_bounds("cs-huge_file_size_mb-spin-up").is_none());
}

fn text(cx: &mut TestAppContext, id: &'static str) -> String {
    with_dialog(cx, |d, _, dcx| d.input_text(id, dcx).expect("a box"))
}

fn text_in(cx: &mut gpui::VisualTestContext, id: &'static str) -> String {
    let dialog = cx
        .update(|_, app| super::current_dialog(app))
        .expect("open");
    cx.read_entity(&dialog, |d, app| d.input_text(id, app).expect("a box"))
}

#[gpui::test]
fn the_access_key_of_a_choice_list_moves_the_focus_to_it_and_keeps_the_choice(
    cx: &mut TestAppContext,
) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_GENERAL");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Ai, dcx));
    let window = cx.update(|app| dialog_window(app)).unwrap();
    // 結果の置き場所(&O): "次の行に入れる" (1) is chosen
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int("ai_output")), 1);
    assert!(!with_dialog(cx, |d, w, _| d.choice_has_focus(
        "ai_output",
        1,
        w
    )));
    cx.simulate_keystrokes(window, "alt-o");
    // Win32 moves the focus to the list and does not change it (it went to the next choice here)
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int("ai_output")), 1);
    assert!(with_dialog(cx, |d, w, _| d.choice_has_focus(
        "ai_output",
        1,
        w
    )));
    cx.simulate_keystrokes(window, "alt-o");
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int("ai_output")), 1);
}

#[gpui::test]
fn the_focused_choice_is_the_element_that_is_drawn_and_space_chooses_it(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_GENERAL");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Ai, dcx));
    let window = cx.update(|app| dialog_window(app)).unwrap();
    cx.simulate_keystrokes(window, "alt-o");
    assert!(with_dialog(cx, |d, w, _| d.choice_has_focus(
        "ai_output",
        1,
        w
    )));
    // The choice changes behind the focus; Space goes to the element that has the focus, which
    // is only the drawn one, and chooses the value of its button.
    with_dialog(cx, |d, _, dcx| d.set_int("ai_output", 2, dcx));
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int("ai_output")), 2);
    cx.simulate_keystrokes(window, "space");
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int("ai_output")), 1);
}

#[test]
fn a_choice_is_a_radio_button_named_by_its_text_on_the_element_that_has_the_focus() {
    use gpui::{InteractiveElement as _, Role, Toggled, div};

    let chosen = crate::a11y::node_of(&super::semantics::choice(
        div().id("c"),
        "次の行に入れる",
        true,
    ))
    .expect("an id and a role");
    assert_eq!(chosen.role(), Role::RadioButton);
    assert_eq!(chosen.label(), Some("次の行に入れる"));
    assert_eq!(chosen.toggled(), Some(Toggled::True));
    let other = crate::a11y::node_of(&super::semantics::choice(
        div().id("d"),
        "置き換える",
        false,
    ))
    .unwrap();
    assert_eq!(other.toggled(), Some(Toggled::False));
}
