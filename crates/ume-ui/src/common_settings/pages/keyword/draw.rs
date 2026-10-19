//! What the page draws itself: the choice of the set, the list of its keywords with the count, and the
//! check box of the case rule.

use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _, px,
    rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::{Disableable as _, h_flex, v_flex};
use ume_config::settings::keywords::{MAX_KEYWORD_LEN, compare_words};

use super::entry::{Entry, from_rows, space};
use super::{CASE, SETS, WORD_SEL, current};
use crate::common_settings::dialog::CommonSettingsDialog;

const SELECTED_BG: u32 = 0x2f6fd0;
const LIST_BORDER: u32 = 0xaaaaaa;
const ROW_HEIGHT: f32 = 22.0;

/// `IDC_COMBO_SET`: the name of the chosen set; the menu lists the sets.
pub(super) fn set_picker(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let values = dialog.values();
    let rows = values.rows(SETS);
    let names: Vec<String> = rows
        .iter()
        .map(|row| row.get(1).cloned().unwrap_or_default())
        .collect();
    let shown = current(values)
        .and_then(|at| names.get(at).cloned())
        .unwrap_or_default();
    let weak = cx.entity().downgrade();
    let picker = Button::new("keyword-set")
        .label(shown)
        .disabled(names.is_empty())
        .dropdown_menu(move |menu, _, _| {
            names.iter().enumerate().fold(menu, |menu, (at, name)| {
                let weak = weak.clone();
                menu.item(
                    PopupMenuItem::new(name.clone()).on_click(move |_, window, cx| {
                        weak.update(cx, |this, cx| this.keyword_select_set(Some(at), window, cx))
                            .ok();
                    }),
                )
            })
        });
    v_flex()
        .gap_1()
        .child("セット名")
        .child(picker)
        .into_any_element()
}

/// The keywords of the chosen set (`IDC_LIST_KEYWORD`) in the order of the set's case rule, and under them
/// the count (`IDC_STATIC_KEYWORD_COUNT`).
pub(super) fn word_list(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let values = dialog.values();
    let entries = from_rows(values.rows(SETS));
    let at = current(values);
    let chosen = usize::try_from(values.int(WORD_SEL)).ok();
    let shown: Vec<AnyElement> = at
        .and_then(|at| entries.get(at))
        .map(display_order)
        .unwrap_or_default()
        .into_iter()
        .map(|(place, word)| {
            div()
                .id(("keyword-word", place))
                .h(px(ROW_HEIGHT))
                .px_2()
                .flex()
                .items_center()
                .cursor_pointer()
                .when(chosen == Some(place), |this| {
                    this.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                })
                .child(word)
                .on_click(cx.listener(move |this, _, window: &mut Window, cx| {
                    this.keyword_select_word(Some(place), window, cx);
                }))
                .into_any_element()
        })
        .collect();
    // `STR_PROPCOMKEYWORD_INFO`: the longest keyword, how many there are, and how many can still be added.
    let count = at
        .and_then(|at| entries.get(at))
        .map_or(0, |entry| entry.words.len());
    let free = at.map_or(0, |at| space(&entries, at));
    let info = format!("(最大 {MAX_KEYWORD_LEN} 文字, 登録数 {count}, 空き {free} 個)");
    v_flex()
        .gap_1()
        .child(
            v_flex()
                .id("keyword-list")
                .h(px(300.))
                .w_full()
                .bg(rgb(0xffffff))
                .border_1()
                .border_color(rgb(LIST_BORDER))
                .overflow_y_scroll()
                .children(shown),
        )
        .child(div().flex().justify_end().child(info))
        .into_any_element()
}

/// The keywords with their places in the set, in the order the list shows them.
fn display_order(entry: &Entry) -> Vec<(usize, String)> {
    let mut shown: Vec<(usize, String)> = entry.words.iter().cloned().enumerate().collect();
    shown.sort_by(|a, b| compare_words(&a.1, &b.1, entry.case_sensitive));
    shown
}

/// `IDC_CHECK_KEYWORDCASE`
pub(super) fn case_box(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let values = dialog.values();
    let entries = from_rows(values.rows(SETS));
    let case = current(values)
        .and_then(|at| entries.get(at))
        .is_some_and(|entry| entry.case_sensitive);
    h_flex()
        .child(
            Checkbox::new("keyword-case")
                .label("英大文字小文字区別(C)")
                .checked(case)
                .disabled(current(values).is_none())
                .on_click(cx.listener(|this, _: &bool, _, cx| {
                    // The control flips the case rule of the chosen set.
                    this.set_bool(CASE, true, cx);
                })),
        )
        .into_any_element()
}
