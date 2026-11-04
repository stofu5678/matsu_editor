//! The outlines of the plugins among the choices of the outline rule of the screen page
//! (`CJackManager::RegisterPlug` and `CPropTypesScreen::AddOutlineMethod`), and what is written back.

use matsu_config::settings::type_rules::outline_type;

use super::super::pages::screen::{OUTLINES, outline_types};
use super::super::values::store;
use super::*;
use crate::plugin_outline::{self, tests::Place};

fn sheet_of(user: &UserSettings) -> super::super::TypeSheet {
    sheet(&user.profile().cloned().unwrap(), 0).unwrap()
}

/// The control `id` of the sheet (the first one: the radio buttons of a group share the id).
fn control(sheet: &super::super::TypeSheet, id: &str) -> crate::common_settings::model::Control {
    *sheet.controls.iter().find(|c| c.id == id).unwrap()
}

/// What the dialog does when the user chooses `number` in the list of outlines.
fn choose(
    sheet: &super::super::TypeSheet,
    values: &mut crate::common_settings::model::Values,
    number: i32,
) {
    values.set(OUTLINE, Value::Int(number));
    (control(sheet, OUTLINE).changed)(values, OUTLINE);
}

/// The choices of the list for the number `now`.
fn options(cx: &mut gpui::TestAppContext, now: i32) -> Vec<(String, i32)> {
    cx.update(|app| outline_types(app, now))
}

#[gpui::test]
fn the_outlines_of_the_plugins_that_run_come_after_the_standard_ones_with_the_code_of_the_plug(
    cx: &mut gpui::TestAppContext,
) {
    let place = Place::new("sheet-on", "");
    let code = Place::code();
    let user = place.user(code, true, true);
    plugin_outline::refresh_for(&user);
    let list = options(cx, code);
    assert_eq!(list.len(), OUTLINES.len() + 1);
    assert!(
        list.iter()
            .zip(OUTLINES)
            .all(|(a, b)| a.0 == b.0 && a.1 == b.1)
    );
    // the number is the function code of the plug, the text its label
    assert_eq!(list.last(), Some(&("Test outline".to_owned(), code)));
    // the number in the file selects it
    let sheet = sheet_of(&user);
    assert_eq!(sheet.values.int(OUTLINE), code);
    // nothing changed: the same number is written
    assert_eq!(
        store(&sheet.values, &sheet.target.original).default_outline,
        code
    );
    // another choice is written, and the plugin one can be chosen again
    let mut values = sheet.values.clone();
    choose(&sheet, &mut values, outline_type::PYTHON);
    assert_eq!(
        store(&values, &sheet.target.original).default_outline,
        outline_type::PYTHON
    );
    choose(&sheet, &mut values, code);
    assert_eq!(store(&values, &sheet.target.original).default_outline, code);
}

#[gpui::test]
fn the_outline_of_a_plugin_that_is_off_is_listed_as_unusable_and_its_number_stays(
    cx: &mut gpui::TestAppContext,
) {
    let place = Place::new("sheet-off", "");
    let code = Place::code();
    // the switch of the plugins is off: the list has the standard outlines, and the number the type has
    let user = place.user(code, false, true);
    plugin_outline::refresh_for(&user);
    let sheet = sheet_of(&user);
    let list = options(cx, code);
    assert_eq!(list.len(), OUTLINES.len() + 1);
    let label = format!("（使えないプラグイン {code}）");
    assert_eq!(list.last(), Some(&(label, code)));
    // the page shows it, and OK without a choice keeps the number
    assert_eq!(sheet.values.int(OUTLINE), code);
    assert_eq!(sheet.values.int(OUTLINE_MODE), 0);
    assert_eq!(
        store(&sheet.values, &sheet.target.original).default_outline,
        code
    );
    // another choice is written
    let mut values = sheet.values.clone();
    choose(&sheet, &mut values, outline_type::TEXT);
    assert_eq!(
        store(&values, &sheet.target.original).default_outline,
        outline_type::TEXT
    );
    // and the first of the list too
    choose(&sheet, &mut values, OUTLINES[0].1);
    assert_eq!(
        store(&values, &sheet.target.original).default_outline,
        OUTLINES[0].1
    );
}

#[gpui::test]
fn a_number_nobody_knows_is_listed_as_unusable_and_stays_too_and_a_rule_file_can_still_be_left(
    cx: &mut gpui::TestAppContext,
) {
    let place = Place::new("sheet-other", "");
    // 17 is no outline of Sakura (`OUTLINE_CODEMAX`) and of no plugin
    let user = place.user(17, true, true);
    plugin_outline::refresh_for(&user);
    let sheet = sheet_of(&user);
    assert_eq!(sheet.values.int(OUTLINE), 17);
    assert_eq!(
        options(cx, 17).last(),
        Some(&("（使えないプラグイン 17）".to_owned(), 17))
    );
    assert_eq!(
        store(&sheet.values, &sheet.target.original).default_outline,
        17
    );

    // a rule file is the mode of the other radio button; going back to the standard rule writes the
    // first of the list
    let user = place.user(outline_type::FILE, true, true);
    let sheet = sheet_of(&user);
    assert_eq!(sheet.values.int(OUTLINE_MODE), 1);
    assert_eq!(
        store(&sheet.values, &sheet.target.original).default_outline,
        outline_type::FILE
    );
    let mut values = sheet.values.clone();
    values.set(OUTLINE_MODE, Value::Int(0));
    (control(&sheet, OUTLINE_MODE).changed)(&mut values, OUTLINE_MODE);
    assert_eq!(
        store(&values, &sheet.target.original).default_outline,
        OUTLINES[0].1
    );
}
