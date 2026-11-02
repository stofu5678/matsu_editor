//! The outlines of the plugins among the choices of the outline rule of the screen page
//! (`CJackManager::RegisterPlug` and `CPropTypesScreen::AddOutlineMethod`), and what is written back.

use matsu_config::settings::type_rules::outline_type;

use super::super::pages::screen::{OUTLINES, outline_options};
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

#[test]
fn the_outlines_of_the_plugins_that_run_come_after_the_standard_ones_with_the_code_of_the_plug() {
    let place = Place::new("sheet-on", "");
    let code = Place::code();
    let user = place.user(code, true, true);
    plugin_outline::refresh_for(&user);
    let options = outline_options();
    assert_eq!(options.len(), OUTLINES.len() + 1);
    assert_eq!(&options[..OUTLINES.len()], OUTLINES);
    // the number is the function code of the plug, the text its label
    assert_eq!(options.last(), Some(&("Test outline", code)));
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

#[test]
fn the_outline_of_a_plugin_that_is_off_is_shown_as_the_first_choice_and_its_number_stays() {
    let place = Place::new("sheet-off", "");
    let code = Place::code();
    // the switch of the plugins is off: the list has the standard outlines only
    let user = place.user(code, false, true);
    plugin_outline::refresh_for(&user);
    assert_eq!(outline_options(), OUTLINES);
    // `SetData`: a number the list does not have selects the first (`nSelPos = 0`)
    let sheet = sheet_of(&user);
    assert_eq!(sheet.values.int(OUTLINE), OUTLINES[0].1);
    assert_eq!(sheet.values.int(OUTLINE_MODE), 0);
    assert_eq!(sheet.target.original.default_outline, code);
    // OK without a choice keeps the number, so that the outline is back when the plugin is
    assert_eq!(
        store(&sheet.values, &sheet.target.original).default_outline,
        code
    );
    // a choice the user made is written, the first of the list too
    let mut values = sheet.values.clone();
    choose(&sheet, &mut values, outline_type::TEXT);
    assert_eq!(
        store(&values, &sheet.target.original).default_outline,
        outline_type::TEXT
    );
}

#[test]
fn a_number_nobody_knows_stays_too_and_a_rule_file_can_still_be_left() {
    let place = Place::new("sheet-other", "");
    // 17 is no outline of Sakura (`OUTLINE_CODEMAX`) and of no plugin
    let user = place.user(17, true, true);
    plugin_outline::refresh_for(&user);
    let sheet = sheet_of(&user);
    assert_eq!(sheet.values.int(OUTLINE), OUTLINES[0].1);
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

#[test]
fn choosing_the_first_of_the_list_after_another_is_a_choice_and_replaces_the_number_that_stayed() {
    let place = Place::new("sheet-first", "");
    let user = place.user(Place::code(), false, true);
    plugin_outline::refresh_for(&user);
    let sheet = sheet_of(&user);
    // another rule and then the first one again: the user chose, whatever the list showed before
    let mut values = sheet.values.clone();
    choose(&sheet, &mut values, outline_type::TEXT);
    choose(&sheet, &mut values, OUTLINES[0].1);
    assert_eq!(
        store(&values, &sheet.target.original).default_outline,
        OUTLINES[0].1
    );
}
