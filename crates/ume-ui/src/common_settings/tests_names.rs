//! The names a screen reader says for the input controls of the settings sheets: every field that
//! the user types into or picks from has one, even where Sakura draws its caption as a separate
//! text.

use ume_config::Profile;

use super::Page;
use super::model::{Control, Item, Kind};

/// The controls the user types into or picks from.
fn is_input(control: &Control) -> bool {
    matches!(
        control.kind,
        Kind::Number { .. } | Kind::Text | Kind::Choice { .. } | Kind::List { .. } | Kind::Color
    )
}

/// The controls the dialog draws (the hidden ones hold the state of a widget of the page).
fn drawn(items: &[Item], out: &mut Vec<Control>) {
    for item in items {
        match item {
            Item::Control(c) => out.push(*c),
            Item::Group(_, inner) | Item::Row(inner) => drawn(inner, out),
            Item::Columns(columns) => columns.iter().for_each(|c| drawn(c, out)),
            Item::Label(_) | Item::Custom(_) | Item::Hidden(_) => {}
        }
    }
}

/// The ids of the input controls of `controls` that have no name.
fn unnamed(controls: &[Control]) -> Vec<&'static str> {
    controls
        .iter()
        .filter(|c| is_input(c) && c.accessible_name().is_empty())
        .map(|c| c.id)
        .collect()
}

#[test]
fn every_input_of_the_common_settings_has_a_name() {
    let mut controls = Vec::new();
    for page in Page::ALL {
        drawn(&page.items().unwrap_or_default(), &mut controls);
    }
    assert_eq!(unnamed(&controls), Vec::<&str>::new());
}

#[test]
fn every_input_of_the_type_settings_has_a_name() {
    let sheet = crate::type_settings::sheet(&Profile::default(), 0).expect("the first type");
    let mut controls = Vec::new();
    for page in &sheet.pages {
        drawn(page.items.as_deref().unwrap_or_default(), &mut controls);
    }
    assert_eq!(unnamed(&controls), Vec::<&str>::new());
}

#[test]
fn a_name_given_by_the_page_wins_over_the_label() {
    let plain = Control::text("a", "名前(&N)", "k", "");
    assert_eq!(plain.accessible_name(), "名前");
    let empty = Control::text("b", "", "k", "");
    assert_eq!(empty.accessible_name(), "");
    assert_eq!(empty.named("見出し記号").accessible_name(), "見出し記号");
    assert_eq!(plain.named("別名").accessible_name(), "別名");
}
