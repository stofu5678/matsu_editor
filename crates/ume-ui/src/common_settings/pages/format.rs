//! 「書式」: `IDD_PROP_FORMAT` (`sakura_core/sakura_rc.rc`), `CPropFormat`
//! (`sakura_core/prop/CPropComFormat.cpp`).
//!
//! All live: the formats of `F_INS_DATE` and `F_INS_TIME` (`ume_commands::format_options`), the
//! 見出し記号 of the outline of a text (`crate::outline::HeadingMarks`) and the 引用符 of
//! `F_COPYLINESASPASSAGE` (`crate::clipboard::QuoteString`). Not drawn: the two boxes 「表示例」,
//! which show the format of the moment.

use ume_commands::convert::datetime::{DEFAULT_DATE_PICTURE, DEFAULT_TIME_PICTURE};
use ume_commands::format_options::{self, FormatOptions};
use ume_outline::text::DEFAULT_HEADING_MARKS;

use super::super::Env;
use super::super::model::{Control, Item, Values};

/// `m_nDateFormatType`, `m_nTimeFormatType`: 0 is the standard format, 1 the one written below.
const CUSTOM: i32 = 1;

/// `EnableFormatPropInput`: the picture is edited only when 「カスタム」 is chosen.
fn custom_date(values: &Values) -> bool {
    values.int("date_format_type") == CUSTOM
}

/// `EnableFormatPropInput`.
fn custom_time(values: &Values) -> bool {
    values.int("time_format_type") == CUSTOM
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Group(
            "日付書式",
            vec![Item::Row(vec![
                Control::radio("date_format_type", "標準(&S)", "nDateFormatType", 0, 0).into(),
                Control::radio("date_format_type", "カスタム(&C)", "nDateFormatType", 1, 0).into(),
                Control::text("date_format", "&D:", "szDateFormat", DEFAULT_DATE_PICTURE)
                    .when(custom_date)
                    .into(),
            ])],
        ),
        Item::Group(
            "時刻書式",
            vec![Item::Row(vec![
                Control::radio("time_format_type", "標準(&N)", "nTimeFormatType", 0, 0).into(),
                Control::radio("time_format_type", "カスタム(&U)", "nTimeFormatType", 1, 0).into(),
                Control::text("time_format", "&T:", "szTimeFormat", DEFAULT_TIME_PICTURE)
                    .when(custom_time)
                    .into(),
            ])],
        ),
        Item::Group(
            "見出し記号(&R)",
            vec![
                Control::text("heading_marks", "", "szMidashiKigou", DEFAULT_HEADING_MARKS)
                    .named("見出し記号")
                    .into(),
            ],
        ),
        Item::Group(
            "引用符(&Q)",
            vec![
                Control::text("quote_string", "", "szInyouKigou", "> ")
                    .named("引用符")
                    .into(),
            ],
        ),
    ]
}

/// What OK does to the windows: the formats of the commands, the marks of the outline and the
/// quote of the commands that copy rows as a passage.
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, _before: Option<&Values>) {
    let text = |id: &str| match after.get(id) {
        Some(super::super::model::Value::Text(t)) => t.clone(),
        _ => String::new(),
    };
    format_options::set(FormatOptions {
        date_type: i32::from(after.int("date_format_type") != 0),
        date_picture: text("date_format"),
        time_type: i32::from(after.int("time_format_type") != 0),
        time_picture: text("time_format"),
    });
    env.cx
        .set_global(crate::outline::HeadingMarks(text("heading_marks")));
    env.cx
        .set_global(crate::clipboard::QuoteString(text("quote_string")));
}
