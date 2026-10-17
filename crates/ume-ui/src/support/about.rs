//! The version information (バージョン情報, `F_ABOUT`).
//!
//! Sakura: `CViewCommander::Command_ABOUT` opens `CDlgAbout` (`sakura_core/dlg/CDlgAbout.cpp`),
//! which shows the name and the version, the commit and the repository, the compile information,
//! the date the program file was last written, a description (`IDS_ABOUT_DESCRIPTION`: free to use,
//! the copyright of the source stays with its authors, no warranty), the web address of the
//! project, and a button that copies the version text to the clipboard.
//!
//! Here the dialog shows the same kinds of things for UME Editor: the name and the version, the
//! repository, the build (profile, OS, CPU), the date the program file was last written, and a
//! description in the user's words (2026-10-17): the license, that UME Editor is Sakura Editor
//! made again in Rust with some behavior of its own, that parts of the code are ported under the
//! zlib License, and the systems it runs on. The copy button copies the first block (the text of the
//! "Ver." lines), as Sakura's `IDC_BUTTON_COPY` does. Not Sakura's own credits or its build number.
//!
//! The dialog is modal: it has the focus, Enter, Space and Esc close it.

use std::time::SystemTime;

use chrono::{DateTime, Local};
use gpui::{
    AppContext as _, ClickEvent, ClipboardItem, Context, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, ParentElement,
    Render, StatefulInteractiveElement, Styled, WeakEntity, Window, div, px, rgb, rgba,
};

use crate::view::EditorView;

/// The name of the product in English text (CLAUDE.md: "UME Editor").
pub(crate) const PRODUCT: &str = "UME Editor";

/// What the dialog shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AboutInfo {
    pub version: String,
    pub repository: String,
    pub license: String,
    /// For example "release, linux aarch64".
    pub build: String,
    /// The date the program file was last written, as "1999/9/9 00:00:00".
    pub last_modified: Option<String>,
}

impl AboutInfo {
    /// The information of this program.
    pub(crate) fn current() -> Self {
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let last_modified = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.metadata().ok())
            .and_then(|m| m.modified().ok())
            .map(format_time);
        Self {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            repository: env!("CARGO_PKG_REPOSITORY").to_owned(),
            license: env!("CARGO_PKG_LICENSE").to_owned(),
            build: format!(
                "{profile}, {} {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            ),
            last_modified,
        }
    }

    /// The lines that the copy button copies (`OnInitDialog`: "name Ver. x", "(GitURL ...)", a
    /// blank line, "Compile Info", "Last Modified"):
    ///
    /// ```text
    /// UME Editor Ver. 0.1.0
    /// (GitURL https://github.com/...)
    ///
    ///       Build: release, windows aarch64
    ///       Last Modified: 2026/10/16 12:00:00
    /// ```
    pub(crate) fn version_text(&self) -> String {
        let mut text = format!(
            "{PRODUCT} Ver. {}\r\n(GitURL {})\r\n\r\n      Build: {}\r\n",
            self.version, self.repository, self.build
        );
        if let Some(modified) = &self.last_modified {
            text.push_str(&format!("      Last Modified: {modified}\r\n"));
        }
        text
    }

    /// The description under the version text (`IDS_ABOUT_DESCRIPTION`), in the words the user
    /// chose (2026-10-17). The license comes from Cargo ("MIT OR Apache-2.0"), written with a
    /// lower-case "or" as the user wrote it.
    pub(crate) fn description(&self) -> String {
        format!(
            "ライセンスは {} です\n\
             ウメエディタ({PRODUCT})はサクラエディタ(SAKURA Editor)を元にRustで再実装したテキストエディタです\n\
             サクラエディタの設計を元に一部独自仕様になっています\n\
             コードの一部はzlib Licenseの元で移植しています\n\
             Windows、Mac、Linuxのマルチプラットフォームに対応しています\n",
            self.license.replace(" OR ", " or ")
        )
    }
}

/// "1999/9/9 00:00:00" in local time (`CDlgAbout`: `%d/%d/%d %02d:%02d:%02d`).
fn format_time(time: SystemTime) -> String {
    let local: DateTime<Local> = time.into();
    local.format("%Y/%-m/%-d %H:%M:%S").to_string()
}

impl EditorView {
    /// `F_ABOUT`: opens the version dialog.
    pub(crate) fn open_about(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.support.about.is_some() {
            return;
        }
        let view = cx.entity().downgrade();
        let dialog = cx.new(|cx| AboutDialog::new(view, AboutInfo::current(), cx));
        let focus = dialog.read(cx).focus.clone();
        window.focus(&focus, cx);
        self.support.about = Some(dialog);
        cx.notify();
    }
}

/// The dialog over the view.
pub(crate) struct AboutDialog {
    view: WeakEntity<EditorView>,
    info: AboutInfo,
    focus: FocusHandle,
}

impl AboutDialog {
    pub(crate) fn new(
        view: WeakEntity<EditorView>,
        info: AboutInfo,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            view,
            info,
            focus: cx.focus_handle(),
        }
    }

    #[cfg(test)]
    pub(crate) fn info(&self) -> &AboutInfo {
        &self.info
    }

    /// Closes the dialog and gives the focus back to the view.
    pub(crate) fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.support.about = None;
            window.focus(&view.focus_handle, cx);
            cx.notify();
        })
        .ok();
    }

    /// The copy button (`IDC_BUTTON_COPY`).
    pub(crate) fn copy(&self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.info.version_text()));
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(event.keystroke.key.as_str(), "escape" | "enter" | "space") {
            self.close(window, cx);
        }
        // the dialog is modal: no key reaches the view
        cx.stop_propagation();
    }
}

impl Focusable for AboutDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AboutDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let button = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0x888888))
                .bg(rgb(0xf4f4f4))
                .child(label)
        };
        let repository = self.info.repository.clone();
        let panel = div()
            .id("about")
            .track_focus(&self.focus)
            .key_context("AboutDialog")
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .w(px(460.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(rgb(0xf8f8f8))
            .border_1()
            .border_color(rgb(0x888888))
            .text_color(rgb(0x222222))
            .text_size(px(14.))
            .child(
                div()
                    .text_size(px(18.))
                    .child(format!("{PRODUCT} Ver. {}", self.info.version)),
            )
            .child(
                div()
                    .id("about-repository")
                    .text_color(rgb(0x0b57d0))
                    .child(repository.clone())
                    .on_click(
                        cx.listener(move |_, _: &ClickEvent, _, cx| cx.open_url(&repository)),
                    ),
            )
            .child(
                div()
                    .text_color(rgb(0x666666))
                    .child(format!("Build: {}", self.info.build))
                    .children(
                        self.info
                            .last_modified
                            .as_ref()
                            .map(|t| format!("Last Modified: {t}")),
                    ),
            )
            .child(div().mt_2().child(self.info.description()))
            .child(
                div()
                    .mt_2()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        button("about-copy", "コピー")
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.copy(cx))),
                    )
                    .child(button("about-ok", "OK").on_click(
                        cx.listener(|this, _: &ClickEvent, window, cx| this.close(window, cx)),
                    )),
            );
        div()
            .id("about-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .bg(rgba(0x00000055))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| cx.stop_propagation()),
            )
            .child(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> AboutInfo {
        AboutInfo {
            version: "1.2.3".into(),
            repository: "https://example.invalid/ume".into(),
            license: "MIT OR Apache-2.0".into(),
            build: "release, linux x86_64".into(),
            last_modified: Some("2026/10/16 12:00:00".into()),
        }
    }

    #[test]
    fn the_version_text_has_the_lines_of_sakuras_dialog() {
        assert_eq!(
            info().version_text(),
            "UME Editor Ver. 1.2.3\r\n(GitURL https://example.invalid/ume)\r\n\r\n      Build: release, linux x86_64\r\n      Last Modified: 2026/10/16 12:00:00\r\n"
        );
    }

    #[test]
    fn the_last_modified_line_is_left_out_when_unknown() {
        let mut i = info();
        i.last_modified = None;
        assert!(!i.version_text().contains("Last Modified"));
    }

    #[test]
    fn the_description_is_the_users_text() {
        assert_eq!(
            info().description(),
            "ライセンスは MIT or Apache-2.0 です\n\
             ウメエディタ(UME Editor)はサクラエディタ(SAKURA Editor)を元にRustで再実装したテキストエディタです\n\
             サクラエディタの設計を元に一部独自仕様になっています\n\
             コードの一部はzlib Licenseの元で移植しています\n\
             Windows、Mac、Linuxのマルチプラットフォームに対応しています\n"
        );
    }

    #[test]
    fn this_program_is_described_from_cargo() {
        let i = AboutInfo::current();
        assert_eq!(i.license, "MIT OR Apache-2.0");
        assert!(i.repository.starts_with("https://github.com/"));
        assert!(!i.version.is_empty());
        assert!(i.build.contains(std::env::consts::OS));
        assert!(i.version_text().starts_with("UME Editor Ver. "));
    }

    #[test]
    fn the_time_is_written_like_sakura_does() {
        let t = format_time(SystemTime::UNIX_EPOCH);
        // no zero padding of month and day, two digits for the clock
        assert!(regex_like(&t), "{t}");
    }

    /// `Y/M/D hh:mm:ss` with M and D not padded.
    fn regex_like(t: &str) -> bool {
        let (date, clock) = t.split_once(' ').unwrap();
        let d: Vec<_> = date.split('/').collect();
        let c: Vec<_> = clock.split(':').collect();
        d.len() == 3
            && c.len() == 3
            && d.iter()
                .all(|p| !p.starts_with('0') && p.parse::<u32>().is_ok())
            && c.iter().all(|p| p.len() == 2)
    }
}
