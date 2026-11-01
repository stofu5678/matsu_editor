//! The dialogs of the color page: 文字色統一, 背景色統一 and the choice of the keyword sets
//! (`CDlgSameColor`, `CDlgKeywordSelect`), and the first set.

use gpui::{Entity, Window};

use super::super::pages::child;
use super::super::pages::keyword_select::{self, SLOTS, normalize, rearrange};
use super::super::pages::same_color::{SameColor, Which};
use super::*;
use crate::common_settings::model::Values;

/// The settings file with four keyword sets, A to D.
pub(super) fn keyword_sets() -> String {
    let mut text = String::from("[KeyWords]\r\nnKeyWordSetNum=4\r\n");
    for (i, name) in ["A", "B", "C", "D"].iter().enumerate() {
        text.push_str(&format!(
            "szSN[{i:02}]={name}\r\nnCASE[{i:02}]=1\r\nnKWN[{i:02}]=1\r\nszKW[{i:02}]={}\t\r\n",
            name.to_lowercase()
        ));
    }
    text
}

fn item_name(item: usize) -> &'static str {
    ColorSet::all_names().nth(item).unwrap()
}

/// Does something in the dialog that is open (its window and view), as a click does.
pub(super) fn in_child<V: 'static, R>(
    cx: &mut TestAppContext,
    view: &Entity<V>,
    f: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R,
) -> R {
    let handle = cx
        .update(|app| child::open_window(app))
        .expect("a dialog is open");
    let result = handle
        .update(cx, |_, window, app| {
            view.update(app, |view, vcx| f(view, window, vcx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

/// The sheet of type 0, at the color page.
pub(super) fn color_page(cx: &mut TestAppContext, text: &str) -> Ini {
    let (ini, a) = setup(cx, text);
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(1, dcx));
    ini
}

pub(super) fn sheet_values(cx: &mut TestAppContext) -> Values {
    with_dialog(cx, |d, _, _| d.values().clone())
}

// ---- 文字色統一, 背景色統一 ----

#[gpui::test]
fn same_text_color_changes_only_the_items_of_the_checked_colors(cx: &mut TestAppContext) {
    // Items 11, 12 and 14 use 112233, item 13 uses 445566, and 表示範囲 (no text color) has 112233 too.
    let mut text = String::from("[Types(0)]\r\n");
    for (item, color) in [
        (11, "112233"),
        (12, "112233"),
        (13, "445566"),
        (14, "112233"),
    ] {
        text.push_str(&format!("C[{}]=1,0,{color},f0fbff,0\r\n", item_name(item)));
    }
    text.push_str("C[PGV]=1,0,112233,f0fbff,0\r\n");
    let ini = color_page(cx, &text);
    choose(cx, COLOR_SEL, 0);
    color(cx, COLOR_TEXT, 0x00ff00);
    let dialog = with_dialog(cx, |d, _, dcx| d.same_color(Which::Text, dcx)).expect("opens");
    in_child(cx, &dialog, |d, _, _| {
        let state = d.state_mut();
        let at = state
            .colors()
            .iter()
            .position(|(c, _)| *c == 0x112233)
            .expect("112233 is listed");
        // Not 統一色 itself, and each color once.
        assert!(state.colors().iter().all(|(c, _)| *c != 0x00ff00));
        assert_eq!(
            state
                .colors()
                .iter()
                .filter(|(c, _)| *c == 0x112233)
                .count(),
            1
        );
        state.set_caret(at);
        // The items that use the color on the caret, by their names, without 表示範囲.
        assert_eq!(
            state.names_on_caret(),
            [11, 12, 14].map(|n| COLOR_ITEMS[n].0)
        );
        state.set_checked(at, true);
    });
    in_child(cx, &dialog, |d, window, dcx| d.ok(window, dcx));
    // The sheet shows the new colors at once.
    let v = sheet_values(cx);
    assert_eq!(v.int(item_key(11, Field::Text)), 0x00ff00);
    assert_eq!(v.int(item_key(13, Field::Text)), 0x445566);
    ok(cx);
    let c = |item: usize| ini.get("Types(0)", &format!("C[{}]", item_name(item)));
    assert_eq!(c(11).as_deref(), Some("1,0,00ff00,f0fbff,0"));
    assert_eq!(c(12).as_deref(), Some("1,0,00ff00,f0fbff,0"));
    assert_eq!(c(14).as_deref(), Some("1,0,00ff00,f0fbff,0"));
    assert_eq!(c(13).as_deref(), Some("1,0,445566,f0fbff,0"));
    assert_eq!(
        ini.get("Types(0)", "C[PGV]").as_deref(),
        Some("1,0,112233,f0fbff,0"),
        "an item without a text color is not touched"
    );
}

#[gpui::test]
fn same_back_color_uses_the_background_colors(cx: &mut TestAppContext) {
    // カーソル (item 2) has no background color; its "background" is the same as the others.
    let mut text = String::from("[Types(0)]\r\n");
    for (item, back) in [(11, "112233"), (12, "112233"), (13, "445566")] {
        text.push_str(&format!("C[{}]=1,0,000000,{back},0\r\n", item_name(item)));
    }
    text.push_str("C[CAR]=1,0,000000,112233,0\r\n");
    let ini = color_page(cx, &text);
    choose(cx, COLOR_SEL, 0);
    color(cx, COLOR_BACK, 0xabcdef);
    let dialog = with_dialog(cx, |d, _, dcx| d.same_color(Which::Back, dcx)).expect("opens");
    in_child(cx, &dialog, |d, _, _| {
        let state = d.state_mut();
        // 全チェック, and then 全解除 for the second.
        state.check_all(true);
        assert!(state.colors().iter().all(|(_, on)| *on));
        state.check_all(false);
        assert!(state.colors().iter().all(|(_, on)| !*on));
        let at = state
            .colors()
            .iter()
            .position(|(c, _)| *c == 0x445566)
            .unwrap();
        state.set_checked(at, true);
    });
    in_child(cx, &dialog, |d, window, dcx| d.ok(window, dcx));
    ok(cx);
    let back = |item: usize| ini.get("Types(0)", &format!("C[{}]", item_name(item)));
    assert_eq!(back(13).as_deref(), Some("1,0,000000,abcdef,0"));
    assert_eq!(back(11).as_deref(), Some("1,0,000000,112233,0"));
    assert_eq!(back(12).as_deref(), Some("1,0,000000,112233,0"));
}

#[gpui::test]
fn the_names_follow_the_item_not_the_place_of_the_color(cx: &mut TestAppContext) {
    // Sakura checks `g_ColorAttributeArr[i]` (the place of the color in the list) for the text colors;
    // here an item without a text color is never named, whichever place its color has in the list.
    let mut text = String::from("[Types(0)]\r\n");
    text.push_str("C[PGV]=1,0,010203,f0fbff,0\r\n");
    text.push_str(&format!("C[{}]=1,0,010203,f0fbff,0\r\n", item_name(30)));
    let _ini = color_page(cx, &text);
    let values = sheet_values(cx);
    let mut state = SameColor::new(&values, Which::Text, 0x00ff00);
    let at = state
        .colors()
        .iter()
        .position(|(c, _)| *c == 0x010203)
        .unwrap();
    state.set_caret(at);
    assert_eq!(state.names_on_caret(), [COLOR_ITEMS[30].0]);
    cancel(cx);
}

#[gpui::test]
fn a_color_dialog_without_a_check_leaves_the_file_as_it_is(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, "[Common]\r\nbDispSTATUSBAR=1\r\n");
    let before = ini.bytes();
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(1, dcx));
    let dialog = with_dialog(cx, |d, _, dcx| d.same_color(Which::Text, dcx)).expect("opens");
    // A second request brings the dialog that is open to the front instead of opening another.
    assert!(with_dialog(cx, |d, _, dcx| d.same_color(Which::Back, dcx)).is_none());
    in_child(cx, &dialog, |d, window, dcx| d.ok(window, dcx));
    ok(cx);
    assert_eq!(ini.bytes(), before);
}

#[gpui::test]
fn cancel_in_a_color_dialog_changes_nothing(cx: &mut TestAppContext) {
    let _ini = color_page(cx, "");
    let before = sheet_values(cx);
    let dialog = with_dialog(cx, |d, _, dcx| d.same_color(Which::Text, dcx)).expect("opens");
    in_child(cx, &dialog, |d, window, _| {
        d.state_mut().check_all(true);
        window.remove_window();
    });
    assert_eq!(sheet_values(cx), before);
    cancel(cx);
}

#[gpui::test]
fn the_buttons_follow_the_item(cx: &mut TestAppContext) {
    let _ini = color_page(cx, "");
    let usable = |cx: &mut TestAppContext, id: &str| {
        with_dialog(cx, |d, _, _| {
            let control = d.page_controls().into_iter().find(|c| c.id == id).unwrap();
            d.is_usable(&control)
        })
    };
    assert!(usable(cx, SAME_TEXT_COLOR) && usable(cx, SAME_BACK_COLOR));
    // カーソル has no background; 表示範囲 has no text color.
    choose(cx, COLOR_SEL, 2);
    assert!(usable(cx, SAME_TEXT_COLOR) && !usable(cx, SAME_BACK_COLOR));
    choose(cx, COLOR_SEL, 56);
    assert!(!usable(cx, SAME_TEXT_COLOR) && usable(cx, SAME_BACK_COLOR));
    cancel(cx);
}

// ---- 強調キーワード ----

/// A settings file with the four sets and the sets 1 to 3 on the slots 1 to 3 of the basic type.
pub(super) fn with_three_sets() -> String {
    format!(
        "{}[Types(0)]\r\nnInts=0,10240,0,4,0,1,0,0,0,0,0,0\r\nnKeywordSelect3=2\r\n",
        keyword_sets()
    )
}

#[gpui::test]
fn the_sheet_holds_the_sets_of_the_type_and_their_names(cx: &mut TestAppContext) {
    let _ini = color_page(cx, &with_three_sets());
    let v = sheet_values(cx);
    assert_eq!(
        keyword_select::sets_of(&v),
        [0, 1, 2, -1, -1, -1, -1, -1, -1, -1]
    );
    assert_eq!(keyword_select::names_of(&v), ["A", "B", "C", "D"]);
    cancel(cx);
}

#[gpui::test]
fn the_keyword_dialog_with_no_change_leaves_the_file_as_it_is(cx: &mut TestAppContext) {
    let (ini, a) = setup(cx, &with_three_sets());
    let before = ini.bytes();
    run(cx, a, "F_OPTION_TYPE", &[]);
    with_dialog(cx, |d, _, dcx| d.select_index(1, dcx));
    let dialog = with_dialog(cx, |d, _, dcx| d.keyword_select(dcx)).expect("opens");
    in_child(cx, &dialog, |d, _, _| {
        assert_eq!(d.sets(), [0, 1, 2, -1, -1, -1, -1, -1, -1, -1]);
    });
    in_child(cx, &dialog, |d, window, dcx| d.ok(window, dcx));
    ok(cx);
    assert_eq!(ini.bytes(), before, "OK without a change writes nothing");
}

#[gpui::test]
fn the_sets_two_to_ten_are_chosen_and_written_with_sakuras_keys(cx: &mut TestAppContext) {
    let ini = color_page(cx, &with_three_sets());
    let dialog = with_dialog(cx, |d, _, dcx| d.keyword_select(dcx)).expect("opens");
    in_child(cx, &dialog, |d, _, dcx| {
        d.choose(3, 3, dcx);
        d.choose(9, 1, dcx);
        // A choice of the first slot is kept as it is chosen.
        d.choose(0, 2, dcx);
    });
    in_child(cx, &dialog, |d, window, dcx| d.ok(window, dcx));
    // RearrangeKeywordSet: the empty slots 5 to 9 are below the last set, so the set in slot 10
    // moves up to slot 5.
    let sets = keyword_select::sets_of(&sheet_values(cx));
    assert_eq!(sets, [2, 1, 2, 3, 1, -1, -1, -1, -1, -1]);
    ok(cx);
    let key = |n: usize| ini.get("Types(0)", &format!("nKeywordSelect{n}"));
    assert_eq!(key(3).as_deref(), Some("2"));
    assert_eq!(key(4).as_deref(), Some("3"));
    assert_eq!(key(5).as_deref(), Some("1"));
    for n in 6..=10 {
        assert_eq!(key(n).as_deref(), Some("-1"), "nKeywordSelect{n}");
    }
    assert!(
        ini.get("Types(0)", "nInts")
            .unwrap()
            .starts_with("0,10240,0,4,2,1,")
    );
}

#[gpui::test]
fn the_first_set_is_chosen_on_the_page(cx: &mut TestAppContext) {
    let ini = color_page(cx, &with_three_sets());
    with_dialog(cx, |d, _, dcx| d.pick_first_set(3, dcx));
    assert_eq!(keyword_select::sets_of(&sheet_values(cx))[0], 3);
    ok(cx);
    let ints = ints(&ini, 0);
    assert_eq!((ints[4], ints[5]), (3, 1));
}

#[gpui::test]
fn a_set_the_list_does_not_have_is_no_choice(cx: &mut TestAppContext) {
    // The settings file has one set, but the type names set 1 for the second slot (`Combo_SetCurSel`
    // fails, `CB_ERR`).
    let text = "[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=A\r\nnCASE[00]=1\r\nnKWN[00]=0\r\nszKW[00]=\r\n\
                [Types(0)]\r\nnInts=0,10240,0,4,0,1,0,0,0,0,0,0\r\n";
    let ini = color_page(cx, text);
    let dialog = with_dialog(cx, |d, _, dcx| d.keyword_select(dcx)).expect("opens");
    in_child(cx, &dialog, |d, window, dcx| d.ok(window, dcx));
    ok(cx);
    assert!(
        ini.get("Types(0)", "nInts")
            .unwrap()
            .starts_with("0,10240,0,4,0,-1,"),
        "the set that is not in the list becomes none"
    );
}

#[gpui::test]
fn the_colors_follow_the_sets_when_they_move_up(cx: &mut TestAppContext) {
    // 強調キーワード2 is empty, 強調キーワード3 has a set: the set moves up with its colors.
    let text = format!(
        "{}[Types(0)]\r\nnInts=0,10240,0,4,0,-1,0,0,0,0,0,0\r\nnKeywordSelect3=1\r\n\
         C[KW2]=1,0,aa0000,f0fbff,0\r\nC[KW3]=1,1,00bb00,ffeeff,1\r\n",
        keyword_sets()
    );
    let ini = color_page(cx, &text);
    // The dialog's OK rearranges, as `RearrangeKeywordSet` does after `DoModal`.
    let dialog = with_dialog(cx, |d, _, dcx| d.keyword_select(dcx)).expect("opens");
    in_child(cx, &dialog, |d, window, dcx| d.ok(window, dcx));
    // The color buttons show the chosen item again.
    choose(cx, COLOR_SEL, 33);
    assert_eq!(value(cx, COLOR_TEXT), Value::Int(0x00bb00));
    assert_eq!(value(cx, COLOR_BOLD), Value::Bool(true));
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "C[KW2]").as_deref(),
        Some("1,1,00bb00,ffeeff,1")
    );
    assert_eq!(
        ini.get("Types(0)", "C[KW3]").as_deref(),
        Some("1,0,aa0000,f0fbff,0")
    );
    assert_eq!(
        ini.get("Types(0)", "nKeywordSelect3").as_deref(),
        Some("-1")
    );
    assert!(
        ini.get("Types(0)", "nInts")
            .unwrap()
            .starts_with("0,10240,0,4,0,1,")
    );
}

#[gpui::test]
fn the_keyword_sets_are_edited_in_the_common_settings(cx: &mut TestAppContext) {
    let _ini = color_page(cx, &with_three_sets());
    with_dialog(cx, |d, _, dcx| d.keyword_edit(dcx));
    let page = cx
        .update(|app| crate::common_settings::current_dialog(app))
        .expect("the common settings open")
        .read_with(cx, |d, _| d.page());
    assert_eq!(page, crate::common_settings::Page::Keyword);
    let handle = cx
        .update(|app| crate::common_settings::dialog_window(app))
        .unwrap();
    handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    cancel(cx);
}

#[test]
fn rearranging_moves_every_set_up_over_the_empty_slots() {
    let sheet = sheet(&Profile::default(), 0).unwrap();
    let mut v = sheet.values;
    v.set(
        KEYWORD_SETS,
        Value::Numbers(vec![-1, 4, -1, -1, 7, 2, -1, -1, -1, 9]),
    );
    rearrange(&mut v);
    assert_eq!(
        keyword_select::sets_of(&v),
        [4, 7, 2, 9, -1, -1, -1, -1, -1, -1]
    );
    // Nothing to move.
    let before = keyword_select::sets_of(&v);
    rearrange(&mut v);
    assert_eq!(keyword_select::sets_of(&v), before);
}

#[test]
fn a_choice_the_list_does_not_have_becomes_none() {
    assert_eq!(
        normalize([0, 3, 4, -1, 2, 1, 0, 9, -7, 3], 4),
        [0, 3, -1, -1, 2, 1, 0, -1, -1, 3]
    );
    assert_eq!(normalize([0; SLOTS], 0), [-1; SLOTS]);
}

#[test]
fn the_name_of_the_first_keyword_color_is_kw1() {
    // `rearrange` finds the colors of the slots after 強調キーワード1.
    assert_eq!(item_name(32), "KW1");
    assert_eq!(item_name(41), "KWA");
}

#[gpui::test]
fn a_choice_made_on_the_sheet_while_a_dialog_is_open_stays(cx: &mut TestAppContext) {
    let _ini = color_page(cx, &with_three_sets());
    let dialog = with_dialog(cx, |d, _, dcx| d.keyword_select(dcx)).expect("opens");
    // The sheet stays usable: the first set is chosen there while the dialog is open.
    with_dialog(cx, |d, _, dcx| d.pick_first_set(3, dcx));
    in_child(cx, &dialog, |d, window, dcx| {
        d.choose(5, 1, dcx);
        d.ok(window, dcx);
    });
    let sets = keyword_select::sets_of(&sheet_values(cx));
    assert_eq!(sets, [3, 1, 2, 1, -1, -1, -1, -1, -1, -1]);
    cancel(cx);
}
