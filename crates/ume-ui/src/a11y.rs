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

use gpui::{
    A11ySubtreeBuilder, App, Bounds, Div, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Role, Stateful, StatefulInteractiveElement, Window,
};

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

/// An element that reports its node as disabled (AccessKit's `disabled`) while `disabled` is
/// set. GPUI's elements have no setter for it (`aria_*` has no disabled), so this wraps the
/// element and writes the flag after the element has written its own properties. Everything else
/// is the wrapped element's.
pub(crate) struct Disabled<E> {
    inner: E,
    disabled: bool,
}

/// `inner`, reported as disabled while `disabled` is set (see [`Disabled`]).
pub(crate) fn disabled<E: Element>(inner: E, disabled: bool) -> Disabled<E> {
    Disabled { inner, disabled }
}

impl<E: Element> IntoElement for Disabled<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for Disabled<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        self.inner.id()
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.inner.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.inner.request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.inner
            .prepaint(id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner.paint(
            id,
            inspector_id,
            bounds,
            request_layout,
            prepaint,
            window,
            cx,
        );
    }

    fn a11y_role(&self) -> Option<Role> {
        self.inner.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.inner.write_a11y_info(node);
        if self.disabled {
            node.set_disabled();
        }
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.inner.a11y_synthetic_children(prepaint, builder);
    }
}

/// Marks a gpui-component control that is a stateful element (a check box, a radio button) as
/// disabled. These are components, not elements, so [`disabled`] cannot wrap them: the flag is
/// written to the node through the synthetic children hook, which can reach its parent node. (A
/// test window builds no tree, so only the real window shows the effect.) gpui-component's
/// buttons and edit boxes are not stateful elements and cannot take it.
pub(crate) trait AriaDisabled: StatefulInteractiveElement + Sized {
    fn aria_disabled(self, disabled: bool) -> Self {
        if disabled {
            self.a11y_synthetic_children(|builder| builder.parent_node().set_disabled())
        } else {
            self
        }
    }
}

impl<T: StatefulInteractiveElement> AriaDisabled for T {}

/// A label of a menu item as `(name, shortcut)`: a menu draws the shortcut after the label,
/// four spaces apart (`menu::label_with_shortcut`). The name has no access key hint either.
pub(crate) fn split_menu_label(label: &str) -> (String, Option<String>) {
    let (name, shortcut) = match label.rsplit_once("    ") {
        Some((name, shortcut)) if !shortcut.is_empty() => (name, Some(shortcut.to_owned())),
        _ => (label, None),
    };
    (without_key_hint(&plain_name(name)), shortcut)
}

/// The content of a popup menu item that UME draws itself (`menu_icons::item_element`), as a node
/// inside the item. gpui-component gives the item node (`MenuItem`) a name only when it draws
/// the label itself, and has no way to give it a checked or disabled state, a shortcut or a
/// position; so the content carries them. A checked item is a `MenuItemCheckBox` that is
/// toggled on, any other a `Label`; the item takes its name from this node. The shortcut is
/// AccessKit's keyboard shortcut.
pub(crate) fn popup_item_content(
    element: Stateful<Div>,
    label: &str,
    checked: bool,
) -> Stateful<Div> {
    let (name, shortcut) = split_menu_label(label);
    let element = element
        .role(if checked {
            Role::MenuItemCheckBox
        } else {
            Role::Label
        })
        .aria_label(name);
    let element = match shortcut {
        Some(shortcut) => element.aria_keyshortcuts(shortcut),
        None => element,
    };
    if checked {
        element.aria_toggled(gpui::Toggled::True)
    } else {
        element
    }
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
    fn a_disabled_element_reports_disabled_and_keeps_the_rest() {
        let on = disabled(editor(gpui::div().id("e")), true);
        let node = node_of(&on).unwrap();
        assert!(node.is_disabled());
        assert_eq!(node.role(), Role::MultilineTextInput);
        assert_eq!(node.label(), Some(EDITOR_NAME));
        let off = disabled(editor(gpui::div().id("e")), false);
        assert!(!node_of(&off).unwrap().is_disabled());
        // No id: not in the tree, disabled or not.
        assert!(node_of(&disabled(gpui::div(), true)).is_none());
    }

    #[test]
    fn a_popup_item_content_carries_name_shortcut_and_check() {
        let plain = node_of(&popup_item_content(
            gpui::div().id("i"),
            "元に戻す(U)    Ctrl+Z",
            false,
        ))
        .unwrap();
        assert_eq!(plain.role(), Role::Label);
        assert_eq!(plain.label(), Some("元に戻す"));
        assert_eq!(plain.keyboard_shortcut(), Some("Ctrl+Z"));
        assert_eq!(plain.toggled(), None);

        let checked = node_of(&popup_item_content(
            gpui::div().id("i"),
            "折り返し(W)",
            true,
        ))
        .unwrap();
        assert_eq!(checked.role(), Role::MenuItemCheckBox);
        assert_eq!(checked.label(), Some("折り返し"));
        assert_eq!(checked.keyboard_shortcut(), None);
        assert_eq!(checked.toggled(), Some(gpui::Toggled::True));
    }

    #[test]
    fn a_menu_label_splits_into_name_and_shortcut() {
        assert_eq!(
            split_menu_label("元に戻す(U)    Ctrl+Z"),
            ("元に戻す".to_owned(), Some("Ctrl+Z".to_owned()))
        );
        assert_eq!(
            split_menu_label("開く(&O)..."),
            ("開く...".to_owned(), None)
        );
        assert_eq!(split_menu_label("コピー(C)"), ("コピー".to_owned(), None));
        assert_eq!(split_menu_label(""), (String::new(), None));
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
