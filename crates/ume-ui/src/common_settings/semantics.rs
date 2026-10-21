//! The roles and names of the sheet's own elements for assistive technology (screen readers),
//! apart from the controls of gpui-component, which bring their own. Each function takes an
//! element that has an id (a node is in the accessibility tree only with an id and a role).
//!
//! * the page list on the left is a tab list: a tab per page, the shown one selected;
//! * the area on the right is the panel of the shown page;
//! * a group box is a group named by its title;
//! * a list of options is a list box, its rows options, the chosen one selected;
//! * an editable list is a table: a header row, a row per entry (the selected one marked) and a
//!   cell per column.
//!
//! Names never carry the access key mark (`crate::a11y::plain_name`).

use gpui::{Div, Orientation, Role, SharedString, Stateful, StatefulInteractiveElement as _};

/// The name of the tab list of the pages.
pub(super) const PAGE_LIST_NAME: &str = "ページ";

/// The whole window: a dialog named by the sheet.
pub(super) fn dialog(element: Stateful<Div>, name: &str) -> Stateful<Div> {
    element.role(Role::Dialog).aria_label(name.to_owned())
}

// AccessKit counts `position_in_set` from 0 (accesskit_windows 0.34.0 `node.rs` 681-686 adds 1 for
// UIA's `PositionInSet`) and takes the size of the set from the container, never from the item
// (`size_of_set_from_container`, accesskit_consumer 0.38.0 `node.rs` 612-624, and node.rs 688-692 of
// accesskit_windows): so the item has the zero-based position and the list has the count.
/// The tab list of the pages, with `count` pages.
pub(super) fn page_list(element: Stateful<Div>, count: usize) -> Stateful<Div> {
    element
        .role(Role::TabList)
        .aria_label(PAGE_LIST_NAME)
        .aria_size_of_set(count)
        .aria_orientation(Orientation::Vertical)
}

pub(super) fn page_tab(
    element: Stateful<Div>,
    title: &str,
    selected: bool,
    index: usize,
) -> Stateful<Div> {
    element
        .role(Role::Tab)
        .aria_label(title.to_owned())
        .aria_selected(selected)
        .aria_position_in_set(index)
}

/// The area of the shown page.
pub(super) fn page_panel(element: Stateful<Div>, title: &str) -> Stateful<Div> {
    element.role(Role::TabPanel).aria_label(title.to_owned())
}

pub(super) fn group(element: Stateful<Div>, title: &str) -> Stateful<Div> {
    element.role(Role::Group).aria_label(title.to_owned())
}

/// A group of controls with one name: the buttons of a choice, the color picker with its label.
pub(super) fn named_group(
    element: Stateful<Div>,
    role: Role,
    name: impl Into<SharedString>,
) -> Stateful<Div> {
    element.role(role).aria_label(name)
}

pub(super) fn list_box(element: Stateful<Div>, name: &str) -> Stateful<Div> {
    element.role(Role::ListBox).aria_label(name.to_owned())
}

pub(super) fn list_option(element: Stateful<Div>, text: &str, selected: bool) -> Stateful<Div> {
    element
        .role(Role::ListBoxOption)
        .aria_label(text.to_owned())
        .aria_selected(selected)
}

pub(super) fn table(
    element: Stateful<Div>,
    name: &str,
    rows: usize,
    columns: usize,
) -> Stateful<Div> {
    element
        .role(Role::Table)
        .aria_label(name.to_owned())
        // The header row is the row 0.
        .aria_row_count(rows + 1)
        .aria_column_count(columns)
}

pub(super) fn header_row(element: Stateful<Div>) -> Stateful<Div> {
    element.role(Role::Row).aria_row_index(0)
}

pub(super) fn column_header(element: Stateful<Div>, title: &str, column: usize) -> Stateful<Div> {
    element
        .role(Role::ColumnHeader)
        .aria_label(title.to_owned())
        .aria_column_index(column)
}

pub(super) fn table_row(element: Stateful<Div>, index: usize, selected: bool) -> Stateful<Div> {
    element
        .role(Role::Row)
        .aria_row_index(index + 1)
        .aria_selected(selected)
}

pub(super) fn cell(element: Stateful<Div>, text: &str, column: usize) -> Stateful<Div> {
    element
        .role(Role::Cell)
        .aria_label(text.to_owned())
        .aria_column_index(column)
}

#[cfg(test)]
mod tests {
    use gpui::{InteractiveElement as _, Role, div};

    use super::*;
    use crate::a11y::node_of;

    fn node(element: Stateful<Div>) -> gpui::accesskit::Node {
        node_of(&element).expect("an element with an id and a role is in the tree")
    }

    #[test]
    fn the_page_list_is_a_tab_list_of_tabs_and_the_shown_page_is_selected() {
        let list = node(page_list(div().id("pages"), 3));
        assert_eq!(list.role(), Role::TabList);
        assert_eq!(list.label(), Some(PAGE_LIST_NAME));
        // What UIA reads: the size from the container, the zero-based position from the item
        // (accesskit_windows 0.34.0 node.rs 681-692; accesskit_consumer 0.38.0 node.rs 612-624).
        assert_eq!(list.size_of_set(), Some(3));

        let shown = node(page_tab(div().id("t0"), "全般", true, 0));
        let other = node(page_tab(div().id("t1"), "ウィンドウ", false, 1));
        assert_eq!(shown.role(), Role::Tab);
        assert_eq!(shown.label(), Some("全般"));
        assert_eq!(shown.is_selected(), Some(true));
        assert_eq!(other.is_selected(), Some(false));
        assert_eq!(shown.position_in_set(), Some(0));
        assert_eq!(other.position_in_set(), Some(1));
        assert_eq!(other.size_of_set(), None);

        let panel = node(page_panel(div().id("body"), "全般"));
        assert_eq!(
            (panel.role(), panel.label()),
            (Role::TabPanel, Some("全般"))
        );
    }

    #[test]
    fn a_group_box_is_a_group_named_by_its_title() {
        let node = node(group(div().id("g"), "ファイルの扱い"));
        assert_eq!(node.role(), Role::Group);
        assert_eq!(node.label(), Some("ファイルの扱い"));
    }

    #[test]
    fn a_list_exposes_its_items_and_the_chosen_one() {
        let list = node(list_box(div().id("l"), "文字コード"));
        assert_eq!(
            (list.role(), list.label()),
            (Role::ListBox, Some("文字コード"))
        );
        let chosen = node(list_option(div().id("o1"), "UTF-8", true));
        let other = node(list_option(div().id("o2"), "SJIS", false));
        assert_eq!(chosen.role(), Role::ListBoxOption);
        assert_eq!(chosen.label(), Some("UTF-8"));
        assert_eq!(chosen.is_selected(), Some(true));
        assert_eq!(other.is_selected(), Some(false));
    }

    #[test]
    fn an_editable_list_is_a_table_with_a_header_rows_and_cells() {
        let table = node(table(div().id("t"), "キーワード", 4, 2));
        assert_eq!(table.role(), Role::Table);
        assert_eq!(table.row_count(), Some(5));
        assert_eq!(table.column_count(), Some(2));
        assert_eq!(node(header_row(div().id("h"))).row_index(), Some(0));
        let header = node(column_header(div().id("c"), "名前", 0));
        assert_eq!(
            (header.role(), header.label()),
            (Role::ColumnHeader, Some("名前"))
        );
        let row = node(table_row(div().id("r"), 0, true));
        assert_eq!(row.role(), Role::Row);
        assert_eq!(row.row_index(), Some(1));
        assert_eq!(row.is_selected(), Some(true));
        let cell = node(cell(div().id("x"), "abc", 1));
        assert_eq!((cell.role(), cell.label()), (Role::Cell, Some("abc")));
        assert_eq!(cell.column_index(), Some(1));
    }
}
