//! The roles and names of the status bar's parts for assistive technology.
//!
//! The message part is a status (a place a screen reader may watch for changes). The other parts
//! are labels whose name is what the part means and what it shows: `改行コード: CRLF`.

use gpui::{Div, Role, Stateful, StatefulInteractiveElement as _, prelude::FluentBuilder as _};

use crate::status::Part;

/// The name of the whole bar.
pub(super) const BAR_NAME: &str = "ステータスバー";

/// What `part` means, in Sakura's words where it has some.
pub(super) fn part_name(part: Part) -> &'static str {
    match part {
        Part::Message => "メッセージ",
        Part::Position => "カーソル位置",
        Part::Eol => "改行コード",
        Part::CharCode => "カーソル位置の文字コード",
        Part::Charset => "文字コード",
        Part::Rec => "キーマクロの記録",
        Part::Mode => "入力モード",
        Part::Zoom => "拡大率",
    }
}

/// The name of `part` showing `text`: the meaning and the text, or the meaning alone for a part
/// that shows nothing.
pub(super) fn part_label(part: Part, text: &str) -> String {
    match (part, text.is_empty()) {
        // The message is what it says.
        (Part::Message, false) => text.to_owned(),
        (_, true) => part_name(part).to_owned(),
        (_, false) => format!("{}: {text}", part_name(part)),
    }
}

pub(super) fn part(element: Stateful<Div>, part: Part, text: &str) -> Stateful<Div> {
    let role = if part == Part::Message {
        Role::Status
    } else {
        Role::Label
    };
    let label = part_label(part, text);
    // UIA names a `Label` by its value, not its label (accesskit_windows 0.34.0 `node.rs` 404-413,
    // `label_comes_from_value` in accesskit_consumer 0.38.0 `node.rs` 727), so both are set.
    element
        .role(role)
        .aria_label(label.clone())
        .when(role == Role::Label, |this| this.aria_value(label))
}

/// The name a platform gives `node`: UIA reads the value for a `Label` and the label for the rest.
#[cfg(test)]
pub(super) fn platform_name(node: &gpui::accesskit::Node) -> Option<&str> {
    if node.role() == Role::Label {
        node.value()
    } else {
        node.label()
    }
}

pub(super) fn bar(element: Stateful<Div>) -> Stateful<Div> {
    element.role(Role::Group).aria_label(BAR_NAME)
}

#[cfg(test)]
mod tests {
    use gpui::{InteractiveElement as _, div};

    use super::*;
    use crate::a11y::node_of;

    #[test]
    fn every_part_has_a_distinct_name() {
        let mut names: Vec<_> = Part::ALL.into_iter().map(part_name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Part::ALL.len());
    }

    #[test]
    fn a_part_reads_its_meaning_and_its_text() {
        let eol = node_of(&part(div().id("p"), Part::Eol, "CRLF")).unwrap();
        assert_eq!(eol.role(), Role::Label);
        assert_eq!(eol.label(), Some("改行コード: CRLF"));
        assert_eq!(platform_name(&eol), Some("改行コード: CRLF"));
        let position = node_of(&part(div().id("p"), Part::Position, "12 行 3 列")).unwrap();
        assert_eq!(position.label(), Some("カーソル位置: 12 行 3 列"));
        assert_eq!(platform_name(&position), Some("カーソル位置: 12 行 3 列"));
    }

    #[test]
    fn the_message_is_a_status_that_reads_as_the_message() {
        let message = node_of(&part(div().id("m"), Part::Message, "保存しました")).unwrap();
        assert_eq!(message.role(), Role::Status);
        assert_eq!(platform_name(&message), Some("保存しました"));
        let empty = node_of(&part(div().id("m"), Part::Message, "")).unwrap();
        assert_eq!(platform_name(&empty), Some("メッセージ"));
    }

    #[test]
    fn a_part_that_shows_nothing_keeps_its_name() {
        let rec = node_of(&part(div().id("r"), Part::Mode, "")).unwrap();
        assert_eq!(platform_name(&rec), Some("入力モード"));
        let bar = node_of(&bar(div().id("bar"))).unwrap();
        assert_eq!((bar.role(), bar.label()), (Role::Group, Some(BAR_NAME)));
    }
}
