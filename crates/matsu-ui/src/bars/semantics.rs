//! The roles and names of the toolbar and the function key bar for assistive technology.
//!
//! A toolbar button is a picture, so its name is the name of its function
//! (`crate::func_name`, the same as its tooltip's first line). A function key's name is the key
//! and the function: `F5 検索`. A button that is on (`pressed`, the toggles) is reported as
//! toggled. A button that is gray is reported as disabled by the caller (`crate::a11y::disabled`:
//! GPUI has no disabled state on a plain element).

use gpui::{
    Div, Role, Stateful, StatefulInteractiveElement as _, Toggled, prelude::FluentBuilder as _,
};

/// The name of the toolbar.
pub(super) const TOOLBAR_NAME: &str = "ツールバー";
/// The name of the function key bar.
pub(super) const FUNCKEY_BAR_NAME: &str = "ファンクションキー";

pub(super) fn toolbar(element: Stateful<Div>) -> Stateful<Div> {
    element.role(Role::Toolbar).aria_label(TOOLBAR_NAME)
}

/// A toolbar button named `name`. `toggled` is `Some` for a button that has an on and an off
/// state (the wrap, the status bar...): a plain button has none, and a toggle reports both states,
/// because the presence of the state is what makes a button a toggle for assistive technology.
pub(super) fn tool_button(
    element: Stateful<Div>,
    name: &str,
    toggled: Option<bool>,
) -> Stateful<Div> {
    element
        .role(Role::Button)
        .aria_label(name.to_owned())
        .when_some(toggled, |this, on| {
            this.aria_toggled(if on { Toggled::True } else { Toggled::False })
        })
}

/// The arrow beside the open button, which shows the menu of the recent files.
pub(super) fn drop_down_arrow(element: Stateful<Div>, name: &str) -> Stateful<Div> {
    element
        .role(Role::Button)
        .aria_label(format!("{name}のメニュー"))
}

pub(super) fn funckey_bar(element: Stateful<Div>) -> Stateful<Div> {
    element.role(Role::Toolbar).aria_label(FUNCKEY_BAR_NAME)
}

/// The name of function key `number` (from 1) whose function is called `function`: `F5 検索`, or
/// just `F5` for a key with no function.
pub(super) fn funckey_name(number: usize, function: &str) -> String {
    if function.is_empty() {
        format!("F{number}")
    } else {
        format!("F{number} {function}")
    }
}

/// A function key button. A key with no function is not a button, and stays out of the tree.
pub(super) fn funckey(element: Stateful<Div>, number: usize, function: &str) -> Stateful<Div> {
    if function.is_empty() {
        return element;
    }
    element
        .role(Role::Button)
        .aria_label(funckey_name(number, function))
}

#[cfg(test)]
mod tests {
    use gpui::{InteractiveElement as _, div};

    use super::*;
    use crate::a11y::node_of;

    #[test]
    fn a_toolbar_button_is_named_by_its_function() {
        let node = node_of(&tool_button(div().id("b"), "上書き保存", None)).unwrap();
        assert_eq!(node.role(), Role::Button);
        assert_eq!(node.label(), Some("上書き保存"));
        assert_eq!(node.toggled(), None);
    }

    #[test]
    fn a_toggle_reports_both_its_states() {
        let on = node_of(&tool_button(div().id("b"), "折り返し", Some(true))).unwrap();
        let off = node_of(&tool_button(div().id("b"), "折り返し", Some(false))).unwrap();
        assert_eq!(on.toggled(), Some(Toggled::True));
        assert_eq!(off.toggled(), Some(Toggled::False));
    }

    #[test]
    fn the_toolbar_and_its_arrow_are_named() {
        let bar = node_of(&toolbar(div().id("bar"))).unwrap();
        assert_eq!(
            (bar.role(), bar.label()),
            (Role::Toolbar, Some(TOOLBAR_NAME))
        );
        let arrow = node_of(&drop_down_arrow(div().id("a"), "開く")).unwrap();
        assert_eq!(arrow.role(), Role::Button);
        assert_eq!(arrow.label(), Some("開くのメニュー"));
    }

    #[test]
    fn a_function_key_is_named_by_its_number_and_its_function() {
        assert_eq!(funckey_name(5, "検索"), "F5 検索");
        assert_eq!(funckey_name(12, ""), "F12");
        let node = node_of(&funckey(div().id("k"), 3, "次を検索")).unwrap();
        assert_eq!(
            (node.role(), node.label()),
            (Role::Button, Some("F3 次を検索"))
        );
        let bar = node_of(&funckey_bar(div().id("bar"))).unwrap();
        assert_eq!(bar.label(), Some(FUNCKEY_BAR_NAME));
    }

    #[test]
    fn a_key_without_a_function_is_not_in_the_tree() {
        assert!(node_of(&funckey(div().id("k"), 1, "")).is_none());
    }
}
