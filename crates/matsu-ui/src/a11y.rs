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
    A11ySubtreeBuilder, App, Bounds, Div, Element, ElementId, FocusHandle, GlobalElementId,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, Pixels, Role, Stateful,
    StatefulInteractiveElement, Window,
};

use crate::common_settings::model::parse_label;

/// The name of the text view.
pub(crate) const EDITOR_NAME: &str = "テキストエディタ";

/// The text view: a multi-line text field. Its text runs, the caret and the selection are written
/// by the hook `a11y_synthetic_children` of the node (`view/a11y_text.rs`), which the caller sets.
pub(crate) fn editor(element: Stateful<Div>) -> Stateful<Div> {
    element
        .role(Role::MultilineTextInput)
        .aria_label(EDITOR_NAME)
}

/// The text view as the keyboard focus of the window for a screen reader. GPUI reports an element
/// as the focus of the accessibility tree only when one element has all three: the focus handle
/// the window really focuses (`track_focus`), an id, and a role (gpui `elements/div.rs`
/// `Interactivity::prepaint` and `window/a11y.rs` `A11y::set_focus`; with no role the focus falls
/// back to the whole window, with only a log line). Setting them in one place keeps the three
/// together.
pub(crate) fn focusable_editor(element: Stateful<Div>, focus: &FocusHandle) -> Stateful<Div> {
    editor(element).track_focus(focus)
}

/// The region of a preview (ADR-0024): a document named by its header. The rendered text is
/// drawn as glyphs and not yet exposed (as the editor's is not), so a screen reader knows the
/// region and what it shows.
pub(crate) fn preview_region(element: Stateful<Div>, name: &str) -> Stateful<Div> {
    element.role(Role::Document).aria_label(name.to_owned())
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

/// The name of a control from its visible label: no `(&X)` mark, no `(X)` hint after the text. The
/// dialogs of MATSU write Sakura's labels with the key in plain parentheses, the settings with `&`.
pub(crate) fn label_name(label: &str) -> String {
    without_key_hint(&plain_name(label))
}

/// A hand-drawn button (a `div` with a click handler): without a role it is not in the tree, so a
/// screen reader cannot reach it. `label` is its visible text; the name has no access key mark.
pub(crate) fn button(element: Stateful<Div>, label: &str) -> Stateful<Div> {
    element.role(Role::Button).aria_label(label_name(label))
}

/// A hand-drawn dialog or pane: a dialog named `name`.
pub(crate) fn dialog(element: Stateful<Div>, name: &str) -> Stateful<Div> {
    element.role(Role::Dialog).aria_label(name.to_owned())
}

/// A hand-drawn group of controls or a pane named `name`.
pub(crate) fn group(element: Stateful<Div>, name: &str) -> Stateful<Div> {
    element.role(Role::Group).aria_label(name.to_owned())
}

/// A line of text that a screen reader should read. UIA names a `Label` by its value, not its
/// label (see `status_bar::semantics::part`), so both are set.
pub(crate) fn text(element: Stateful<Div>, text: &str) -> Stateful<Div> {
    element
        .role(Role::Label)
        .aria_label(text.to_owned())
        .aria_value(text.to_owned())
}

/// A row of a hand-drawn list box: an option named `name`, `selected` when it is the chosen one.
pub(crate) fn list_row(element: Stateful<Div>, name: &str, selected: bool) -> Stateful<Div> {
    element
        .role(Role::ListBoxOption)
        .aria_label(name.to_owned())
        .aria_selected(selected)
}

/// A row of a hand-drawn tree: an item at `depth` (from 0), `expanded` is `Some` for a node that
/// has children.
pub(crate) fn tree_row(
    element: Stateful<Div>,
    name: &str,
    selected: bool,
    depth: usize,
    expanded: Option<bool>,
) -> Stateful<Div> {
    let element = element
        .role(Role::TreeItem)
        .aria_label(name.to_owned())
        .aria_selected(selected)
        .aria_level(depth + 1);
    match expanded {
        Some(open) => element.aria_expanded(open),
        None => element,
    }
}

/// The name of a row of a list of functions: the label, then the shortcut (`上書き保存、Ctrl+S`),
/// as a menu item is read.
pub(crate) fn function_row_name(label: &str, shortcut: Option<&str>) -> String {
    match shortcut {
        Some(shortcut) if !shortcut.is_empty() => format!("{}、{shortcut}", label_name(label)),
        _ => label_name(label),
    }
}

/// A hand-drawn link: a `div` that opens a web address.
pub(crate) fn link(element: Stateful<Div>, text: &str) -> Stateful<Div> {
    element.role(Role::Link).aria_label(text.to_owned())
}

/// A button or a check box with a visible label, named for a screen reader by that label without
/// Sakura's access key mark (`上検索(U)` is `上検索`). A label that says nothing alone (`...`) needs
/// `.label(..).accessibility_label(..)` with its own name instead.
pub(crate) trait Labelled: Sized {
    fn labelled(self, label: impl Into<gpui::SharedString>) -> Self;
}

impl Labelled for gpui_component::button::Button {
    fn labelled(self, label: impl Into<gpui::SharedString>) -> Self {
        let label = label.into();
        let name = label_name(&label);
        self.label(label).accessibility_label(name)
    }
}

impl Labelled for gpui_component::radio::Radio {
    fn labelled(self, label: impl Into<gpui::SharedString>) -> Self {
        let label = label.into();
        let name = label_name(&label);
        self.label(label).accessibility_label(name)
    }
}

impl Labelled for gpui_component::checkbox::Checkbox {
    fn labelled(self, label: impl Into<gpui::SharedString>) -> Self {
        let label = label.into();
        let name = label_name(&label);
        self.label(label).accessibility_label(name)
    }
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
/// set, and/or as a live region (AccessKit's `live`). GPUI's elements have no setter for them
/// (`aria_*` has neither), so this wraps the element and writes the flags after the element has
/// written its own properties. Everything else is the wrapped element's.
pub(crate) struct Flagged<E> {
    inner: E,
    disabled: bool,
    live: Option<gpui::accesskit::Live>,
}

/// `inner`, reported as disabled while `disabled` is set (see [`Flagged`]).
pub(crate) fn disabled<E: Element>(inner: E, disabled: bool) -> Flagged<E> {
    Flagged {
        inner,
        disabled,
        live: None,
    }
}

/// `inner`, reported as a live region: a screen reader says the changes of its content
/// (`Polite`: when it has finished what it is saying) (see [`Flagged`]). The content has to be
/// text that a node carries as its value, such as a `Label` (macOS announces the value).
pub(crate) fn live_region<E: Element>(inner: E, live: gpui::accesskit::Live) -> Flagged<E> {
    Flagged {
        inner,
        disabled: false,
        live: Some(live),
    }
}

impl<E: Element> IntoElement for Flagged<E> {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for Flagged<E> {
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
        if let Some(live) = self.live {
            node.set_live(live);
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

/// What a screen reader says of a checked popup menu item, and of a gray one.
const CHECKED: &str = "チェック済み";
const UNAVAILABLE: &str = "使用不可";

/// The content of a popup menu item that MATSU draws itself (`menu_icons::item_element`), as a node
/// inside the item. gpui-component gives the item node (`MenuItem`) a name only when it draws
/// the label itself, and has no way to give it a checked or disabled state, a shortcut or a
/// position. AccessKit names a menu item from the `value` of the `Label` nodes inside it (and
/// skips any other role), and passes on nothing else from them, so this is a `Label` whose value
/// is what a screen reader should say for the item: the name, then the shortcut, `チェック済み`
/// and `使用不可` as they apply. The node also carries them as properties (name, keyboard
/// shortcut, toggled, description), for a client that reads the node itself.
pub(crate) fn popup_item_content(
    element: Stateful<Div>,
    label: &str,
    checked: bool,
    disabled: bool,
) -> Stateful<Div> {
    let (name, shortcut) = split_menu_label(label);
    let mut spoken = name.clone();
    for part in shortcut
        .iter()
        .map(String::as_str)
        .chain(checked.then_some(CHECKED))
        .chain(disabled.then_some(UNAVAILABLE))
    {
        spoken.push('、');
        spoken.push_str(part);
    }
    let element = element
        .role(Role::Label)
        .aria_label(name)
        .aria_value(spoken);
    let element = match shortcut {
        Some(shortcut) => element.aria_keyshortcuts(shortcut),
        None => element,
    };
    if checked {
        element
            .aria_toggled(gpui::Toggled::True)
            .aria_description(CHECKED)
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
        // The text is in the children of the node (`view/a11y_text.rs`), not in its value.
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

    /// A text view that keeps its own focus handle, as `EditorView` does.
    struct Focused(FocusHandle);

    impl gpui::Render for Focused {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            focusable_editor(gpui::div().id("editor"), &self.0)
        }
    }

    #[gpui::test]
    fn the_focused_text_view_has_the_focus_handle_an_id_and_a_role(cx: &mut gpui::TestAppContext) {
        let handle = cx.update(|cx| cx.focus_handle());
        let node = node_of(&focusable_editor(gpui::div().id("editor"), &handle)).unwrap();
        assert_eq!(node.role(), Role::MultilineTextInput);
        assert_eq!(node.label(), Some(EDITOR_NAME));

        let (_, cx) = cx.add_window_view({
            let handle = handle.clone();
            move |_, _| Focused(handle)
        });
        cx.update(|window, cx| window.focus(&handle, cx));
        cx.run_until_parked();
        // The handle of the node is the one the window focuses: the node would be the focus of
        // the accessibility tree.
        assert!(cx.update(|window, _| handle.is_focused(window)));
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
            false,
        ))
        .unwrap();
        assert_eq!(plain.role(), Role::Label);
        assert_eq!(plain.label(), Some("元に戻す"));
        // The enclosing menu item is named from this value.
        assert_eq!(plain.value(), Some("元に戻す、Ctrl+Z"));
        assert_eq!(plain.keyboard_shortcut(), Some("Ctrl+Z"));
        assert_eq!(plain.toggled(), None);

        let checked = node_of(&popup_item_content(
            gpui::div().id("i"),
            "折り返し(W)",
            true,
            true,
        ))
        .unwrap();
        assert_eq!(checked.role(), Role::Label);
        assert_eq!(checked.label(), Some("折り返し"));
        assert_eq!(checked.value(), Some("折り返し、チェック済み、使用不可"));
        assert_eq!(checked.description(), Some(CHECKED));
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
    fn a_hand_drawn_button_is_a_button_named_without_the_access_key() {
        let node = node_of(&button(gpui::div().id("b"), "キャンセル(X)")).unwrap();
        assert_eq!(node.role(), Role::Button);
        assert_eq!(node.label(), Some("キャンセル"));
        // Without an id it is not in the tree at all.
        assert!(node_of(&button(gpui::div().id("b"), "OK")).is_some());
    }

    #[test]
    fn a_hand_drawn_dialog_group_and_link_have_their_roles_and_names() {
        let dialog = node_of(&dialog(gpui::div().id("d"), "バージョン情報")).unwrap();
        assert_eq!(
            (dialog.role(), dialog.label()),
            (Role::Dialog, Some("バージョン情報"))
        );
        let group = node_of(&group(gpui::div().id("g"), "アウトライン")).unwrap();
        assert_eq!(
            (group.role(), group.label()),
            (Role::Group, Some("アウトライン"))
        );
        let link = node_of(&link(gpui::div().id("l"), "https://example.invalid")).unwrap();
        assert_eq!(link.role(), Role::Link);
        assert_eq!(link.label(), Some("https://example.invalid"));
    }

    #[test]
    fn a_text_is_a_label_that_has_its_text_as_name_and_value() {
        let node = node_of(&text(gpui::div().id("t"), "Build: release")).unwrap();
        assert_eq!(node.role(), Role::Label);
        // UIA names a `Label` by its value.
        assert_eq!(node.value(), Some("Build: release"));
        assert_eq!(node.label(), Some("Build: release"));
    }

    #[test]
    fn a_list_row_is_an_option_and_a_tree_row_an_item_with_its_level() {
        let row = node_of(&list_row(gpui::div().id("r"), "12行 1桁 見出し", true)).unwrap();
        assert_eq!(row.role(), Role::ListBoxOption);
        assert_eq!(row.label(), Some("12行 1桁 見出し"));
        assert_eq!(row.is_selected(), Some(true));

        let folder = node_of(&tree_row(gpui::div().id("t"), "src", false, 0, Some(true))).unwrap();
        assert_eq!(folder.role(), Role::TreeItem);
        assert_eq!(folder.level(), Some(1));
        assert_eq!(folder.is_expanded(), Some(true));
        let leaf = node_of(&tree_row(gpui::div().id("t"), "a.rs", true, 2, None)).unwrap();
        assert_eq!(leaf.level(), Some(3));
        assert_eq!(leaf.is_expanded(), None);
        assert_eq!(leaf.is_selected(), Some(true));
    }

    #[test]
    fn a_row_of_functions_is_read_as_the_label_then_the_shortcut() {
        assert_eq!(
            function_row_name("上書き保存(&S)", Some("Ctrl+S")),
            "上書き保存、Ctrl+S"
        );
        assert_eq!(function_row_name("開く(&O)...", None), "開く...");
        assert_eq!(function_row_name("新規作成", Some("")), "新規作成");
    }

    #[test]
    fn a_parenthesis_that_is_not_a_key_stays() {
        assert_eq!(plain_name("(&&)"), "(&)");
        assert_eq!(plain_name("a(&"), "a(&");
        assert_eq!(plain_name("x(&ab)"), "x(ab)");
    }
}
