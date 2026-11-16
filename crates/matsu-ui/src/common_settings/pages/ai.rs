//! 「生成 AI」: the settings page of the destinations that a text is sent to (ADR-0035, decisions 2, 5 and 7).
//!
//! Sakura has no such page, so it is MATSU Editor's own, right after 「ワンライナー」 and built like it: a list of
//! 50 numbered slots, and below it the fields of the slot the Id box names, which the button 設定 puts into the
//! list. A slot is a name, a command, the place of the result (置き換える, 次の行に入れる, ログのタブだけ) and
//! 送る前に確認する. OK writes `[AI]` (`matsu_config::settings::ai`) under the lock of the settings file; the
//! keys this program does not know stay as the file has them. Cancel writes nothing.
//!
//! * [`rows`]: the list (a table control that the page draws itself), the templates and what 設定 does.
//! * this file: the widgets (the click on a row, the note, 設定, ひな形を入れる).
//!
//! The limits and the refusals are the page 「ワンライナー」's (a name of 64 UTF-16 units or more, a command of
//! 1024 or more and a line break are refused with the reason; nothing is cut).
//!
//! The windows read the settings at the time they use them, so what OK wrote is used at once.

mod rows;

use gpui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, PromptLevel, Styled as _, Window,
    div, rgb,
};
use matsu_config::settings::ai::AiOutput;

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::model::{Control, Item};
use super::oneliner::{SlotList, draw_slots};

/// The controls with the prefix `ai_` hold only the state of the page (no key in the file): the list is kept
/// in the file as `[AI]` by its own store.
const LIST: &str = "ai_list";
const SELECTED: &str = "ai_selected";
const ID: &str = "ai_id";
const NAME: &str = "ai_name";
const COMMAND: &str = "ai_command";
const OUTPUT: &str = "ai_output";
const ASK: &str = "ai_ask";
const TEMPLATE: &str = "ai_template";

/// The color of a warning (the page 「ワンライナー」 uses the same).
const WARNING: u32 = 0xc00000;

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Label(
            "モード a は、選んだ文字列(選んでいないときは文書全体)をそのまま送ります。モード b は、入れた指示をつけて送ります。\nコマンドは、標準入力からプロンプトを読めるものを書きます。",
        ),
        Item::Custom(draw),
        Item::Row(vec![
            Control::text(NAME, "名前(&N)", "", "").into(),
            Control::number(ID, "Id:", "", 0, 0, 49).into(),
        ]),
        Item::Row(vec![
            Control::text(COMMAND, "コマンド(&C)", "", "").into(),
            Control::button("ai_register", "設定(&A)", register).into(),
        ]),
        Item::Row(vec![
            Control::choice(OUTPUT, "結果の置き場所(&O)", "", 1, &rows::OUTPUT_OPTIONS).into(),
            Control::check(ASK, "送る前に確認する(&K)", "", true).into(),
        ]),
        Item::Row(vec![
            Control::choice(TEMPLATE, "ひな形(&T)", "", 0, &rows::TEMPLATE_OPTIONS).into(),
            Control::button("ai_put_template", "ひな形を入れる(&I)", put_template).into(),
        ]),
        Item::Label(
            "ひな形は、コマンドに例を入れ、結果の置き場所を「次の行に入れる」にします。ひな形に指示はありません。\n選んだ文字列が、そのままプロンプトとして標準入力で渡ります。\nひな形の内容は、AI ツールの版で変わることがあります。",
        ),
        Item::Custom(draw_note),
        Item::Label(
            "送る内容と答えは、読み取り専用のログのタブに残ります。本文が外のサービスに出ることがあります。\nコマンドで AI のツールが切られているかは、MATSU Editor には分かりません。\nコマンドは 1 行で書きます。シェルは「シェル」のページのものです。コマンドのない番号は、登録されていません。",
        ),
        // The state of the list: the rows, kept in the file as `[AI]`, and the row that is selected (-1: none).
        Item::Hidden(vec![
            Control::choice(SELECTED, "", "", -1, &[]),
            Control::table(LIST, "", rows::spec()).named("生成 AI の送り先"),
        ]),
    ]
}

/// What OK does to the windows: nothing besides the file. The windows read `[AI]` from the settings that OK
/// made the new ones.
pub(in crate::common_settings) fn apply(
    _: &mut Env,
    _: &super::super::model::Values,
    _: Option<&super::super::model::Values>,
) {
}

fn draw(dialog: &CommonSettingsDialog, cx: &mut Context<CommonSettingsDialog>) -> AnyElement {
    draw_slots(
        &SlotList {
            spec: rows::spec(),
            rows: LIST,
            selected: SELECTED,
            prefix: "ai",
            dimmed: |_| false,
            shown: |_, text| text.to_owned(),
            select,
        },
        dialog,
        cx,
    )
}

/// A row is selected: the Id box and the fields show it. Selecting the row that is selected already changes
/// nothing, so what was typed stays.
fn select(
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
    dialog.set_bool(ASK, fields.ask, cx);
}

/// ひな形を入れる: puts the chosen template in the command and the place of the result 次の行に入れる. Nothing
/// is chosen: nothing changes. It is 設定 that puts the fields in the list.
fn put_template(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let Some(template) = rows::template_of(dialog.values().int(TEMPLATE)) else {
        return;
    };
    let fields = rows::apply_template(&current_fields(dialog, cx), template);
    dialog.set_input(NAME, &fields.name, window, cx);
    dialog.set_input(COMMAND, &fields.command, window, cx);
    dialog.set_int(OUTPUT, fields.output.number(), cx);
}

/// The fields as they are now.
fn current_fields(dialog: &CommonSettingsDialog, cx: &gpui::App) -> rows::Fields {
    rows::Fields {
        name: typed(dialog, NAME, cx),
        command: typed(dialog, COMMAND, cx),
        output: AiOutput::from_number(dialog.values().int(OUTPUT)),
        ask: dialog.values().bool(ASK),
    }
}

/// What the box says now: the values hold the text only once OK commits the boxes.
fn typed(dialog: &CommonSettingsDialog, id: &str, cx: &gpui::App) -> String {
    dialog.input_text(id, cx).unwrap_or_default()
}

/// 設定: puts the fields into the row of the slot the Id box names. A value that is refused is told, and the
/// row stays as it is.
fn register(
    dialog: &mut CommonSettingsDialog,
    window: &mut Window,
    cx: &mut Context<CommonSettingsDialog>,
) {
    let index = super::oneliner::index_of(&typed(dialog, ID, cx));
    let fields = current_fields(dialog, cx);
    // The row is changed in place; a refused value leaves the rows as they were and is told.
    // Nobody waits for this answer.
    if let Err(problem) = dialog.edit_rows(LIST, |rows| rows::register(rows, index, &fields), cx) {
        crate::boxes::tell(window, PromptLevel::Info, &problem.text(), None, cx);
    }
}

/// Under the fields: what is wrong with them now, so that 設定 does not surprise.
fn draw_note(dialog: &CommonSettingsDialog, cx: &mut Context<CommonSettingsDialog>) -> AnyElement {
    match super::oneliner::problem(&typed(dialog, NAME, cx), &typed(dialog, COMMAND, cx)) {
        Some(problem) => div()
            .text_color(rgb(WARNING))
            .child(problem.text())
            .into_any_element(),
        None => div().into_any_element(),
    }
}

#[cfg(test)]
mod tests;
