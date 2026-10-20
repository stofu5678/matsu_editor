//! Accessibility names (AccessKit through GPUI): what a screen reader reads for the controls.
//!
//! A node reaches the platform's accessibility tree (UI Automation, AX, AT-SPI) only if its
//! element has an id and a role, so every call site here sets both. Controls from gpui-component
//! (check boxes, radio buttons, buttons, edit boxes) already carry their role and the name they
//! draw; what this module adds is the name without Sakura's access key mark, and a way to read
//! what an element will report in the tests.
//!
//! The tests read the elements, not the whole tree: GPUI's test platform never connects an
//! assistive technology, so a test window builds no tree and `debug_a11y_tree_json` stays `None`.

use gpui::{Div, Role, Stateful, StatefulInteractiveElement as _};

use crate::common_settings::model::parse_label;

/// The name of the text view.
pub(crate) const EDITOR_NAME: &str = "テキストエディタ";

/// The text view: a multi-line text field. Its text is not exposed yet (that needs the visible
/// lines as text runs with their positions), so a screen reader knows it is there and can focus
/// it, but cannot read the text.
pub(crate) fn editor(element: Stateful<Div>) -> Stateful<Div> {
    element
        .role(Role::MultilineTextInput)
        .aria_label(EDITOR_NAME)
}

/// The name of a control for a screen reader, from a Sakura label: `フリーカーソル(&F)` is
/// `フリーカーソル`, `&URL` is `URL`, `&&` is `&`. The access key is not part of the name (the
/// platform has its own way to tell it).
pub(crate) fn plain_name(label: &str) -> String {
    let mut text = String::with_capacity(label.len());
    let mut rest = label;
    while let Some(at) = rest.find("(&") {
        let after = &rest[at + 2..];
        let mut chars = after.chars();
        match (chars.next(), chars.next()) {
            // `(&F)`, but not `(&&)`.
            (Some(key), Some(')')) if key != '&' => {
                text.push_str(&rest[..at]);
                rest = &after[key.len_utf8() + 1..];
            }
            _ => {
                text.push_str(&rest[..at + 2]);
                rest = after;
            }
        }
    }
    text.push_str(rest);
    parse_label(&text).0
}

/// The name of a menu as drawn, without the access key that Sakura writes after it: `ファイル(F)`
/// is `ファイル`, and `開く(O)...` is `開く...`. Anything else stays.
pub(crate) fn without_key_hint(label: &str) -> String {
    let (stem, dots) = match label.strip_suffix("...") {
        Some(stem) => (stem, "..."),
        None => (label, ""),
    };
    if let Some(open) = stem.rfind('(')
        && let Some(key) = stem[open + 1..].strip_suffix(')')
        && key.chars().count() == 1
        && key.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return format!("{}{dots}", &stem[..open]);
    }
    label.to_owned()
}

/// What `element` reports to the accessibility tree: its role and the properties it writes.
/// `None` if the element is not in the tree (no role or no id).
#[cfg(test)]
pub(crate) fn node_of<E: gpui::Element>(element: &E) -> Option<gpui::accesskit::Node> {
    element.id()?;
    let mut node = gpui::accesskit::Node::new(element.a11y_role()?);
    element.write_a11y_info(&mut node);
    Some(node)
}

#[cfg(test)]
mod tests {
    use gpui::InteractiveElement as _;

    use super::*;

    #[test]
    fn the_access_key_mark_is_not_part_of_the_name() {
        assert_eq!(plain_name("フリーカーソル(&F)"), "フリーカーソル");
        assert_eq!(plain_name("開く(&O)..."), "開く...");
        assert_eq!(plain_name("&URL"), "URL");
        assert_eq!(plain_name("Fish && Chips"), "Fish & Chips");
        assert_eq!(plain_name("(&A)と(&B)"), "と");
        assert_eq!(plain_name("plain"), "plain");
        assert_eq!(plain_name(""), "");
    }

    #[test]
    fn the_text_view_is_a_named_multi_line_text_field() {
        let node = node_of(&editor(gpui::div().id("editor"))).unwrap();
        assert_eq!(node.role(), Role::MultilineTextInput);
        assert_eq!(node.label(), Some(EDITOR_NAME));
        // Not exposed yet: no value.
        assert_eq!(node.value(), None);
    }

    /// A view that is only a text view with its name.
    struct Probe;

    impl gpui::Render for Probe {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            editor(gpui::div().id("editor"))
        }
    }

    /// The tree of a window, as `debug_a11y_tree_json` gives it. GPUI's test platform never
    /// connects a screen reader, so a test window builds no tree and the dump is `None`; if a
    /// later GPUI builds one, this checks the names through it as well (and the other tests
    /// still check the elements).
    #[gpui::test]
    fn the_tree_of_a_test_window_has_the_names_when_gpui_builds_it(cx: &mut gpui::TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| Probe);
        cx.run_until_parked();
        let json = cx.update(|window, _| window.debug_a11y_tree_json());
        if let Some(json) = json {
            assert!(json.contains("MultilineTextInput"), "{json}");
            assert!(json.contains(EDITOR_NAME), "{json}");
        }
    }

    #[test]
    fn the_key_hint_of_a_menu_is_left_out() {
        assert_eq!(without_key_hint("ファイル(F)"), "ファイル");
        assert_eq!(without_key_hint("開く(O)..."), "開く...");
        assert_eq!(without_key_hint("ヘルプ"), "ヘルプ");
        assert_eq!(without_key_hint("設定(あ)"), "設定(あ)");
        assert_eq!(without_key_hint("関数(ab)"), "関数(ab)");
        assert_eq!(without_key_hint("(F)"), "");
    }

    #[test]
    fn a_parenthesis_that_is_not_a_key_stays() {
        assert_eq!(plain_name("(&&)"), "(&)");
        assert_eq!(plain_name("a(&"), "a(&");
        assert_eq!(plain_name("x(&ab)"), "x(ab)");
    }
}
