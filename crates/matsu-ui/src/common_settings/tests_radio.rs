//! The radio buttons of the pages of the common settings act as one group each, as the automatic
//! radio buttons of Sakura's property sheets do: Tab stops on the group once, at the chosen
//! button, and the arrow keys move to the next button of the group and choose it.

use gpui::TestAppContext;

use super::dialog_window;
use super::tests::{run, setup, visual, with_dialog};
use crate::radio_group::Ring;
use crate::radio_group::testing::{assert_stops_at_checked, at, trail};

/// The group 「ファイルダイアログの初期位置」 of the page 編集 (`IDC_RADIO_CURDIR`,
/// `IDC_RADIO_MRUDIR`, `IDC_RADIO_SELDIR`): the values 0, 1 and 2.
const GROUP: &str = "open_dialog_dir";

fn ring(cx: &mut TestAppContext) -> Ring {
    with_dialog(cx, |d, _, _| {
        d.radio_groups
            .get(GROUP)
            .expect("the group is drawn")
            .ring
            .clone()
    })
}

fn value(cx: &mut TestAppContext) -> i32 {
    with_dialog(cx, |d, _, _| d.values().int(GROUP))
}

#[gpui::test]
fn a_group_is_one_tab_stop_at_the_chosen_button_and_the_arrow_keys_choose(cx: &mut TestAppContext) {
    // 最近使ったフォルダー (1) is the button that is chosen
    let (_ini, handle) = setup(cx, "[Common]\r\neOpenDialogDir=1\r\n");
    run(cx, handle, "F_OPTION_EDIT");
    assert_eq!(value(cx), 1);
    let window = cx.update(|app| dialog_window(app)).unwrap();
    let ring = ring(cx);
    // Tab and Shift+Tab stop on the group once, at the chosen button
    let forward = trail(visual(cx, window), &[&ring], "tab", 60);
    assert_stops_at_checked(&forward, &[1]);
    let back = trail(visual(cx, window), &[&ring], "shift-tab", 60);
    assert_stops_at_checked(&back, &[1]);
    for _ in 0..60 {
        if at(visual(cx, window), &[&ring]).is_some() {
            break;
        }
        visual(cx, window).simulate_keystrokes("tab");
    }
    assert_eq!(at(visual(cx, window), &[&ring]), Some((0, 1)));
    for (key, chosen) in [
        ("down", 2),
        // round the end to the first one, and from there to the last
        ("right", 0),
        ("up", 2),
        ("left", 1),
    ] {
        visual(cx, window).simulate_keystrokes(key);
        assert_eq!(value(cx), chosen, "{key}");
        assert_eq!(
            at(visual(cx, window), &[&ring]),
            Some((0, usize::try_from(chosen).unwrap())),
            "{key}"
        );
    }
    // Tab leaves the group
    visual(cx, window).simulate_keystrokes("tab");
    assert_eq!(at(visual(cx, window), &[&ring]), None);
}

#[gpui::test]
fn the_access_key_of_a_radio_button_still_chooses_it(cx: &mut TestAppContext) {
    // the letters (`&M`) choose a button directly, whichever has the focus
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_EDIT");
    assert_eq!(value(cx), 0);
    let window = cx.update(|app| dialog_window(app)).unwrap();
    visual(cx, window).simulate_keystrokes("alt-m");
    assert_eq!(value(cx), 1);
}

#[gpui::test]
fn the_access_key_of_a_radio_button_takes_the_focus_and_the_arrow_keys_go_on_from_it(
    cx: &mut TestAppContext,
) {
    // Sakura: an access key checks the radio button and focuses it (also when it is checked), and
    // the arrow keys then move within its group
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_EDIT");
    let window = cx.update(|app| dialog_window(app)).unwrap();
    let ring = ring(cx);
    assert_eq!(at(visual(cx, window), &[&ring]), None);
    visual(cx, window).simulate_keystrokes("alt-m");
    assert_eq!(value(cx), 1);
    assert_eq!(at(visual(cx, window), &[&ring]), Some((0, 1)));
    visual(cx, window).simulate_keystrokes("down");
    assert_eq!(value(cx), 2);
    assert_eq!(at(visual(cx, window), &[&ring]), Some((0, 2)));
    // the key of the checked button focuses it again after the focus left the group
    visual(cx, window).simulate_keystrokes("tab");
    assert_eq!(at(visual(cx, window), &[&ring]), None);
    visual(cx, window).simulate_keystrokes("alt-m");
    assert_eq!(value(cx), 1);
    visual(cx, window).simulate_keystrokes("tab");
    visual(cx, window).simulate_keystrokes("alt-m");
    assert_eq!(value(cx), 1);
    assert_eq!(at(visual(cx, window), &[&ring]), Some((0, 1)));
}

#[gpui::test]
fn a_click_on_a_radio_button_checks_it_and_takes_the_focus(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    run(cx, handle, "F_OPTION_EDIT");
    let window = cx.update(|app| dialog_window(app)).unwrap();
    let ring = ring(cx);
    // `cs-{GROUP}-2`
    let bounds = visual(cx, window)
        .debug_bounds("cs-open_dialog_dir-2")
        .expect("the radio button");
    visual(cx, window).simulate_click(bounds.center(), gpui::Modifiers::none());
    assert_eq!(value(cx), 2);
    assert_eq!(at(visual(cx, window), &[&ring]), Some((0, 2)));
}
