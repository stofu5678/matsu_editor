//! The status bar as Sakura's parts, and what a double click or a right click on a part does.
//!
//! Sakura: `CEditWnd::OnSize` makes the parts (`pszLabel`: the width of a part is the width of its
//! widest text), `CCaret::ShowCaretPosInfo` fills them, and `CEditWnd::DispatchEvent` (`WM_NOTIFY`
//! from the status bar, `sakura_core/window/CEditWnd.cpp`) handles the clicks:
//!
//! | part | double click |
//! |------|--------------|
//! | line and column | `F_JUMP_DIALOG` |
//! | code at the caret | `ShowCodeBox`: the codes of the character in each encoding |
//! | encoding | `F_CHG_CHARSET` |
//! | REC | `F_RECKEYMACRO` |
//! | typing mode | `F_CHGMOD_INS` |
//!
//! A right click on the line ending opens the menu of the input line endings, which runs
//! `F_CHGMOD_EOL`. The first part is the message (the menu help and the size of the selection) and
//! has no border; the others have one on the left. REC is gray, and dark and bold while this window
//! records a key macro (`WM_DRAWITEM`).

use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseUpEvent, ParentElement, Pixels, Point, PromptLevel, SharedString,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px, rgb,
};
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::encoding::{CharCodeOptions, Encoding, try_char_code};

use crate::app::Workspace;
use crate::menu::{Entry, Item, MenuAction};
use crate::status::{Part, StatusInfo};
use crate::view::EditorView;

/// The size of the text (px).
const FONT_SIZE: f32 = 12.0;
/// What a part adds to the width of its text: the border and the room around the text
/// (`nBdrWidth`).
const PART_PADDING: f32 = 12.0;
/// The title of the box of `ShowCodeBox` (`GSTR_APPNAME`).
const BOX_TITLE: &str = "MATSU Editor";

/// The function a double click on `part` runs (`CEditWnd::DispatchEvent`, `NM_DBLCLK`), by name.
pub(crate) fn double_click_function(part: Part) -> Option<&'static str> {
    match part {
        Part::Position => Some("F_JUMP_DIALOG"),
        Part::Charset => Some("F_CHG_CHARSET"),
        Part::Rec => Some("F_RECKEYMACRO"),
        Part::Mode => Some("F_CHGMOD_INS"),
        _ => None,
    }
}

/// The name a part has in the tests.
fn selector(part: Part) -> String {
    format!("status-part-{}", part as usize)
}

fn code_of(name: &str) -> Option<u16> {
    FUNCTIONS.iter().find(|f| f.name == name).map(|f| f.code)
}

/// The menu of a right click on the line ending (`NM_RCLICK`): CRLF, LF and CR.
///
/// Sakura adds NEL, LS and PS when `m_bEnableExtEol` is on (`CEditWnd::OnNotify`; the labels are
/// `STR_EDITWND_MENU_NEL`, `_LS` and `_PS` in `sakura_rc.rc`).
pub(crate) fn eol_menu_entries() -> Vec<Entry> {
    let item = |name: &str, label: Option<&str>| {
        let function = FUNCTIONS.iter().find(|f| f.name == name)?;
        Some(Entry::Item(Item {
            label: label.or(function.label)?.to_owned(),
            action: MenuAction::Function(function.code),
            shortcut: None,
            enabled: true,
        }))
    };
    let mut entries: Vec<Entry> = ["F_CHGMOD_EOL_CRLF", "F_CHGMOD_EOL_LF", "F_CHGMOD_EOL_CR"]
        .into_iter()
        .filter_map(|name| item(name, None))
        .collect();
    if matsu_commands::edit_options::get().ext_eol {
        entries.extend(
            [
                ("F_CHGMOD_EOL_NEL", "入力改行コード指定(&NEL)"),
                ("F_CHGMOD_EOL_LS", "入力改行コード指定(L&S)"),
                ("F_CHGMOD_EOL_PS", "入力改行コード指定(&PS)"),
            ]
            .into_iter()
            .filter_map(|(name, label)| item(name, Some(label))),
        );
    }
    entries
}

/// The message of `ShowCodeBox` for the character that starts `rest` (the text from the caret to
/// the end of the line, without the line ending): the character, its code point, and its code in
/// each encoding. `None` when there is no character. A code that an encoding cannot write is `-`.
///
/// Sakura: `ShowCodeBox` in `sakura_core/window/CEditWnd.cpp`; the format is
/// `STR_ERR_DLGEDITWND13`. The options of the status bar are off for it, so the codes are bytes
/// (the UTF-16 one is the code point), except that the code point shows a surrogate pair as
/// `U+1F600`.
pub(crate) fn code_box_message(rest: &str) -> Option<String> {
    let first = rest.chars().next()?;
    let bytes = CharCodeOptions {
        unicode_in_sjis: false,
        unicode_in_jis: false,
        unicode_in_euc: false,
        utf8_codepoint: false,
        surrogate_codepoint: false,
    };
    let code = |encoding: Encoding| try_char_code(encoding, rest, &bytes).unwrap_or("-".to_owned());
    let code_point = try_char_code(
        Encoding::Utf16Le,
        rest,
        &CharCodeOptions {
            surrogate_codepoint: true,
            ..bytes
        },
    )
    .unwrap_or("-".to_owned());
    Some(format!(
        "文字:\t\t{first} ({code_point})\n\nSJIS:\t\t{}\nJIS:\t\t{}\nEUC:\t\t{}\nLatin1:\t\t{}\nUTF-16:\t\t{}\nUTF-8:\t\t{}\nCESU-8:\t\t{}",
        code(Encoding::ShiftJis),
        code(Encoding::Jis),
        code(Encoding::EucJp),
        code(Encoding::Latin1),
        code(Encoding::Utf16Le),
        code(Encoding::Utf8),
        code(Encoding::Cesu8),
    ))
}

/// The width of `text` in the font of the status bar.
fn text_width(window: &Window, font_family: &SharedString, text: &str) -> Pixels {
    let mut style = window.text_style();
    style.font_family = font_family.clone();
    let run = style.to_run(text.len());
    window
        .text_system()
        .layout_line(text, px(FONT_SIZE), &[run], None)
        .width
}

impl Workspace {
    /// The status bar: the message on the left, then the other parts, each as wide as its label.
    pub(crate) fn render_status_bar(
        &self,
        info: &StatusInfo,
        font_family: &SharedString,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let parts = Part::ALL.into_iter().map(|part| {
            let text = info.text(part);
            let cell = semantics::part(div().id(("status-part", part as usize)), part, &text)
                // names the part in the tests (`debug_selector` does nothing in release builds)
                .debug_selector(move || selector(part))
                .h_full()
                .flex();
            if part == Part::Message {
                // No border, and the rest of the width.
                let cell = cell
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .px_2()
                    .child(text.clone());
                return semantics::message(cell, &text);
            }
            let width = text_width(window, font_family, part.width_label()) + px(PART_PADDING);
            let cell = cell
                .flex_none()
                .w(width)
                .overflow_hidden()
                .whitespace_nowrap()
                .px_1()
                .border_l_1()
                .border_color(rgb(0xb8b8b8));
            let cell = if part == Part::Rec {
                // COLOR_BTNTEXT, drawn twice (bold) while recording; COLOR_3DSHADOW otherwise.
                if info.recording {
                    cell.text_color(rgb(0x1e1e1e)).font_weight(FontWeight::BOLD)
                } else {
                    cell.text_color(rgb(0x808080))
                }
            } else {
                cell
            };
            cell.child(text)
                .when(
                    double_click_function(part).is_some() || part == Part::CharCode,
                    |cell| {
                        cell.on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.standard_click() && event.click_count() == 2 {
                                this.status_double_click(part, window, cx);
                            }
                        }))
                    },
                )
                .when(part == Part::Eol, |cell| {
                    cell.on_mouse_up(
                        MouseButton::Right,
                        cx.listener(move |this, event: &MouseUpEvent, window, cx| {
                            this.status_right_click(part, event.position, window, cx);
                        }),
                    )
                })
                .into_any_element()
        });
        semantics::bar(div().id("status-bar"))
            .flex()
            .flex_row()
            .items_center()
            .py_0p5()
            .text_size(px(FONT_SIZE))
            .bg(rgb(0xe8e8e8))
            .text_color(rgb(0x333333))
            .children(parts)
            .into_any_element()
    }

    /// A double click on `part` (`NM_DBLCLK`).
    pub(crate) fn status_double_click(
        &mut self,
        part: Part,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // After this update ends: the functions read the state of the window (the tabs and the
        // outline), which cannot be read while the window is being updated.
        let editor = self.editor().clone();
        window.defer(cx, move |window, cx| {
            editor.update(cx, |view, cx| {
                if part == Part::CharCode {
                    view.show_code_box(window, cx);
                } else if let Some(code) = double_click_function(part).and_then(code_of) {
                    view.run_function(code, &[], window, cx);
                }
            });
        });
    }

    /// A right click on `part` at `at` (`NM_RCLICK`): the menu of the line endings.
    pub(crate) fn status_right_click(
        &mut self,
        part: Part,
        at: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if part != Part::Eol {
            return;
        }
        let editor = self.editor().clone();
        window.defer(cx, move |window, cx| {
            editor.update(cx, |view, cx| {
                view.open_menu_entries_at(eol_menu_entries(), at, window, cx);
            });
        });
    }
}

impl EditorView {
    /// `ShowCodeBox`: tells the codes of the character at the caret in a message box. Nothing
    /// happens at the end of a line or of the file.
    pub(crate) fn show_code_box(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(message) = self
            .caret_char_rest(cx)
            .and_then(|rest| code_box_message(&rest))
        else {
            return;
        };
        let answer = crate::boxes::prompt(
            window,
            PromptLevel::Info,
            BOX_TITLE,
            Some(&message),
            &["OK"],
            cx,
        );
        cx.spawn_in(window, async move |_, _| {
            let _ = answer.await;
        })
        .detach();
    }
}

mod semantics;
#[cfg(test)]
mod tests;
