//! The roles and names of the status bar's parts for assistive technology.
//!
//! Every part is a label (`Text` in UI Automation, `AXStaticText` on macOS). The other parts are
//! named by what the part means and what it shows: `改行コード: CRLF`. The message is named by
//! what it says, and is a live region while it says something, so a screen reader reads each new
//! notice (`保存しました`) without being asked ([`message`]).
//!
//! The message used to be a `Role::Status`. AccessKit gives that role the control type
//! `StatusBar` on Windows and an `AXGroup` on macOS, which a tool that reads the texts of a window
//! (the sandbox harnesses, a screen reader's text review) does not list, and the live region
//! announcement on macOS reads the value of the node, which a status does not need to have.

use gpui::accesskit::Live;
use gpui::{AnyElement, Div, IntoElement as _, Role, Stateful, StatefulInteractiveElement as _};

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
    let label = part_label(part, text);
    // UIA names a `Label` by its value, not its label (accesskit_windows 0.34.0 `node.rs` 404-413,
    // `label_comes_from_value` in accesskit_consumer 0.38.0 `node.rs` 727), so both are set.
    element
        .role(Role::Label)
        .aria_label(label.clone())
        .aria_value(label)
}

/// The message part, finished: a live region while there is a message. Windows raises
/// `UIA_LiveRegionChangedEventId` when the name of a live node changes (accesskit_windows 0.34.0
/// `adapter.rs` 257, 316), macOS posts an announcement of the value (accesskit_macos 0.26.3
/// `event.rs` 236, 299), and AT-SPI gets the live setting. With no message it is not live: the
/// message going away is not announced.
pub(super) fn message(element: Stateful<Div>, text: &str) -> AnyElement {
    if text.is_empty() {
        element.into_any_element()
    } else {
        crate::a11y::live_region(element, Live::Polite).into_any_element()
    }
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
    fn the_message_is_a_label_that_reads_as_the_message() {
        let message = node_of(&part(div().id("m"), Part::Message, "保存しました")).unwrap();
        assert_eq!(message.role(), Role::Label);
        assert_eq!(platform_name(&message), Some("保存しました"));
        // macOS announces the value of a live node
        assert_eq!(message.value(), Some("保存しました"));
        let empty = node_of(&part(div().id("m"), Part::Message, "")).unwrap();
        assert_eq!(platform_name(&empty), Some("メッセージ"));
    }

    #[test]
    fn the_message_is_a_polite_live_region_only_while_it_says_something() {
        // `message` gives an `AnyElement`, which has no node to read: the same wrapper is read
        // here, and the choice between the two is tested through the window below.
        let live = |text: &str| {
            let element = part(div().id("m"), Part::Message, text);
            node_of(&crate::a11y::live_region(element, Live::Polite))
                .unwrap()
                .live()
        };
        assert_eq!(live("保存しました"), Some(Live::Polite));
        let off = node_of(&part(div().id("m"), Part::Message, "")).unwrap();
        assert_eq!(off.live(), None, "not live");
        let eol = node_of(&part(div().id("e"), Part::Eol, "CRLF")).unwrap();
        assert_eq!(eol.live(), None, "the other parts never are");
    }

    #[test]
    fn a_part_that_shows_nothing_keeps_its_name() {
        let rec = node_of(&part(div().id("r"), Part::Mode, "")).unwrap();
        assert_eq!(platform_name(&rec), Some("入力モード"));
        let bar = node_of(&bar(div().id("bar"))).unwrap();
        assert_eq!((bar.role(), bar.label()), (Role::Group, Some(BAR_NAME)));
    }
}
