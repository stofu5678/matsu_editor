//! The widgets of the page 「マクロ」 that the dialog has no control for: the list of the 50 macros,
//! and what a click on a row and the button 設定 do.

use gpui::{
    AnyElement, ClickEvent, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, px, rgb,
};
use gpui_component::{h_flex, v_flex};

use super::super::super::dialog::CommonSettingsDialog;
use super::rows::{self, Fields};

pub(super) const LIST: &str = "macro_list";
pub(super) const SELECTED: &str = "macro_selected";
pub(super) const ID: &str = "macro_id";
pub(super) const NAME: &str = "macro_name";
pub(super) const FILE: &str = "macro_file";
pub(super) const RELOAD: &str = "macro_reload";
pub(super) const ON_OPENED: &str = "macro_on_opened";
pub(super) const ON_TYPE_CHANGED: &str = "macro_on_type_changed";
pub(super) const ON_SAVE: &str = "macro_on_save";

const SELECTED_BG: u32 = 0x2f6fd0;
const BORDER: u32 = 0xaaaaaa;
const HEADING_BG: u32 = 0xe6e6e6;
const ROW_HEIGHT: f32 = 20.0;

/// The list (`IDC_MACROLIST`): the heading and a row for each macro. A click selects a row and
/// puts its texts in the fields below.
pub(super) fn draw(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let spec = rows::spec();
    let selected = usize::try_from(dialog.values().int(SELECTED)).ok();
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
        .rows(LIST)
        .iter()
        .enumerate()
        .map(|(index, row)| {
            h_flex()
                .id(SharedString::from(format!("macro-row-{index}")))
                .h(px(ROW_HEIGHT))
                .items_center()
                .cursor_pointer()
                .when(selected == Some(index), |d| {
                    d.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
                })
                .children(spec.columns.iter().zip(row).map(|(c, text)| {
                    div()
                        .w(width(c.width))
                        .px_1()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(text.clone())
                }))
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    select(this, index, window, cx);
                }))
                .into_any_element()
        })
        .collect();
    v_flex()
        .id("macro-list")
        .h(px(130.))
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(rgb(BORDER))
        .child(heading)
        .child(
            v_flex()
                .id("macro-list-rows")
                .flex_1()
                .overflow_y_scroll()
                .children(body),
        )
        .into_any_element()
}

/// A row is selected (`CheckListPosition_Macro`): the Id box and the fields show it. Selecting the
/// row that is selected already changes nothing, so what was typed stays.
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
    dialog.set_input(FILE, &fields.file, window, cx);
    dialog.set_bool(RELOAD, fields.reload, cx);
    dialog.set_bool(ON_OPENED, fields.opened, cx);
    dialog.set_bool(ON_TYPE_CHANGED, fields.type_changed, cx);
    dialog.set_bool(ON_SAVE, fields.save, cx);
}

/// 設定 (`IDC_MACRO_REG`): puts the fields into the row of the macro the Id box names.
pub(super) fn register(
    dialog: &mut CommonSettingsDialog,
    _: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let text =
        |dialog: &CommonSettingsDialog, id: &str| dialog.input_text(id, cx).unwrap_or_default();
    let index = rows::index_of(&text(dialog, ID));
    let fields = Fields {
        name: text(dialog, NAME),
        file: text(dialog, FILE),
        reload: dialog.values().bool(RELOAD),
        opened: dialog.values().bool(ON_OPENED),
        type_changed: dialog.values().bool(ON_TYPE_CHANGED),
        save: dialog.values().bool(ON_SAVE),
    };
    let mut list = dialog.values().rows(LIST).to_vec();
    rows::register(&mut list, index, &fields);
    dialog.set_rows(LIST, list, cx);
}

/// `IDC_MACRODIRREF`: chooses the folder of the macros. The box starts the dialog if it names a
/// folder, and takes the folder back as it is.
pub(super) fn browse_folder(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    use crate::file_dialog::{self, Ask};
    let text = dialog.input_text("macro_dir", cx).unwrap_or_default();
    let native = if cfg!(windows) {
        text
    } else {
        text.replace('\\', "/")
    };
    let start = Some(std::path::PathBuf::from(native))
        .filter(|path| path.is_dir())
        .or_else(|| file_dialog::macro_folder(cx));
    // `STR_PROPCOMMACR_SEL_DIR`
    let receiver = file_dialog::ask(
        window,
        cx,
        Ask::folder()
            .start(start)
            .titled("Macroディレクトリの選択", "選択"),
    );
    cx.spawn_in(window, async move |this, cx| {
        let chosen = receiver.await.ok().flatten();
        if let Some(path) = chosen.and_then(|paths| paths.into_iter().next()) {
            let text = path.to_string_lossy().into_owned();
            this.update_in(cx, |this, window, cx| {
                this.set_input("macro_dir", &text, window, cx);
            })
            .ok();
        }
    })
    .detach();
}

/// The Python folder is not used (the macro host runs Python itself, ADR-0007): the button does
/// nothing, and is greyed out.
pub(super) fn browse_python(
    _: &mut CommonSettingsDialog,
    _: &mut Window,
    _: &mut Context<CommonSettingsDialog>,
) {
}

/// How many rows the list has.
#[cfg(test)]
pub(super) const ROWS: usize = matsu_config::settings::macros::MAX_CUSTMACRO;
