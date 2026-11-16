//! The widgets of the page 「ワンライナー」 that the dialog has no control for: the list of the 50 slots, the
//! click on a row, the note under the fields and the button 設定. They follow the page 「マクロ」.

use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    PromptLevel, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px, rgb,
};
use gpui_component::{h_flex, v_flex};
use matsu_config::settings::oneliner::OutputTo;

use super::super::super::dialog::CommonSettingsDialog;
use super::super::super::table::{Row, TableSpec};
use super::rows::{self, Fields};

pub(super) const LIST: &str = "oneliner_list";
pub(super) const SELECTED: &str = "oneliner_selected";
pub(super) const ID: &str = "oneliner_id";
pub(super) const NAME: &str = "oneliner_name";
pub(super) const COMMAND: &str = "oneliner_command";
pub(super) const OUTPUT: &str = "oneliner_output";

const SELECTED_BG: u32 = 0x2f6fd0;
const BORDER: u32 = 0xaaaaaa;
const HEADING_BG: u32 = 0xe6e6e6;
/// The color of a row that cannot be used.
const DIMMED: u32 = 0x888888;
const ROW_HEIGHT: f32 = 20.0;
/// The color of a warning (the page 「シェル」 uses the same).
const WARNING: u32 = 0xc00000;

/// What a list of slots needs besides its rows: the page names it (the ids, the spec) and says which rows are
/// shown dimmed.
pub(in crate::common_settings) struct SlotList {
    pub spec: &'static TableSpec,
    /// The control that holds the rows.
    pub rows: &'static str,
    /// The control that holds the number of the row that is selected (-1: none).
    pub selected: &'static str,
    /// The prefix of the element ids.
    pub prefix: &'static str,
    /// A row that cannot be used is shown dimmed.
    pub dimmed: fn(&Row) -> bool,
    /// What a cell shows for the text the row holds in `column`.
    pub shown: fn(usize, &str) -> String,
    /// A click on row `index`.
    pub select:
        fn(&mut CommonSettingsDialog, usize, &mut Window, &mut Context<CommonSettingsDialog>),
}

/// The list: the heading and a row for each slot. A click selects a row and puts its texts in the fields
/// below.
pub(super) fn draw(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    draw_slots(
        &SlotList {
            spec: rows::spec(),
            rows: LIST,
            selected: SELECTED,
            prefix: "oneliner",
            dimmed: rows::is_ai_row,
            shown: rows::shown,
            select,
        },
        dialog,
        cx,
    )
}

/// Draws a [`SlotList`] (the page 「生成 AI」 draws its list with it too).
pub(in crate::common_settings) fn draw_slots(
    list: &SlotList,
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let spec = list.spec;
    let selected = usize::try_from(dialog.values().int(list.selected)).ok();
    let (dimmed, shown, select, prefix) = (list.dimmed, list.shown, list.select, list.prefix);
    let width = |percent: u32| gpui::relative(percent as f32 / 100.);
    let heading = h_flex()
        .bg(rgb(HEADING_BG))
        .border_b_1()
        .border_color(rgb(BORDER))
        .children(
            spec.columns
                .iter()
                .map(|c| div().w(width(c.width)).px_1().child(c.title)),
        );
    let body: Vec<AnyElement> = dialog
        .values()
        .rows(list.rows)
        .iter()
        .enumerate()
        .map(|(index, row)| {
            h_flex()
                .id(SharedString::from(format!("{prefix}-row-{index}")))
                .h(px(ROW_HEIGHT))
                .items_center()
                .cursor_pointer()
                .when(dimmed(row), |d| d.text_color(rgb(DIMMED)))
                .when(selected == Some(index), |d| {
                    d.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                })
                .children(
                    spec.columns
                        .iter()
                        .zip(row)
                        .enumerate()
                        .map(|(i, (c, text))| {
                            div()
                                .w(width(c.width))
                                .px_1()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .child(shown(i, text))
                        }),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    select(this, index, window, cx);
                }))
                .into_any_element()
        })
        .collect();
    v_flex()
        .id(SharedString::from(format!("{prefix}-list")))
        .h(px(190.))
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(rgb(BORDER))
        .child(heading)
        .child(
            v_flex()
                .id(SharedString::from(format!("{prefix}-list-rows")))
                .flex_1()
                .overflow_y_scroll()
                .children(body),
        )
        .into_any_element()
}

/// A row is selected: the Id box and the fields show it. Selecting the row that is selected already changes
/// nothing, so what was typed stays.
pub(super) fn select(
    dialog: &mut CommonSettingsDialog,
    index: usize,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let Some(row) = dialog.values().rows(LIST).get(index).cloned() else {
        return;
    };
    let number = i32::try_from(index).unwrap_or(-1);
    if dialog.values().int(SELECTED) == number {
        return;
    }
    let fields = rows::fields_of(&row);
    dialog.set_int(SELECTED, number, cx);
    dialog.set_input(ID, &index.to_string(), window, cx);
    dialog.set_input(NAME, &fields.name, window, cx);
    dialog.set_input(COMMAND, &fields.command, window, cx);
    dialog.set_int(OUTPUT, fields.output.number(), cx);
}

/// The fields as they are now.
fn current_fields(dialog: &CommonSettingsDialog, cx: &gpui::App) -> Fields {
    Fields {
        name: typed(dialog, NAME, cx),
        command: typed(dialog, COMMAND, cx),
        output: OutputTo::from_number(dialog.values().int(OUTPUT)),
    }
}

/// What the box says now: the values hold the text only once OK commits the boxes.
fn typed(dialog: &CommonSettingsDialog, id: &str, cx: &gpui::App) -> String {
    dialog.input_text(id, cx).unwrap_or_default()
}

/// 設定: puts the fields into the row of the slot the Id box names. A value that is refused is told, and the
/// row stays as it is.
pub(super) fn register(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let index = rows::index_of(&typed(dialog, ID, cx));
    let fields = current_fields(dialog, cx);
    // The row is changed in place; a refused value leaves the rows as they were and is told.
    // Nobody waits for this answer.
    if let Err(problem) = dialog.edit_rows(LIST, |rows| rows::register(rows, index, &fields), cx) {
        crate::boxes::tell(window, PromptLevel::Info, &problem.text(), None, cx);
    }
}

/// Under the fields: what is wrong with them now, so that 設定 does not surprise.
pub(super) fn draw_note(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    match rows::problem(&typed(dialog, NAME, cx), &typed(dialog, COMMAND, cx)) {
        Some(problem) => div()
            .text_color(rgb(WARNING))
            .child(problem.text())
            .into_any_element(),
        None => div().into_any_element(),
    }
}
