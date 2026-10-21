// Ported from Sakura Editor (https://github.com/sakura-editor/sakura):
//   sakura_core/dlg/CDlgCtrlCode.cpp (the table of control codes, how the list shows a code and
//   which keys choose one), the descriptions from sakura_core/sakura_rc.rc (STR_ERR_DLGCTL5 to
//   STR_ERR_DLGCTL37, STR_DLGCTRLCODE_*), and the dialog IDD_CTRLCODE.
//
// Copyright (C) 2002-2003, MIK
// Copyright (C) 2006, ryoji
// Copyright (C) 2011, nasukoji
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the dialog is a modal list over the
// editor view (GPUI) instead of a Windows dialog, the table is a Rust array, and the tests are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! The control code dialog (コントロールコード入力, `F_CTRL_CODE_DIALOG`).
//!
//! Sakura: `CViewCommander::Command_CtrlCode_Dialog` (`sakura_core/cmd/CViewCommander_Insert.cpp`)
//! shows `CDlgCtrlCode`, a list of the 33 control codes (0x00 to 0x1f and 0x7f) with four
//! columns: コード (`0x1B`), 表記 (`^[`, or `^?` for DEL), 名前 (`ESC`) and 説明. The first row
//! is chosen. OK, Enter and a double click choose the row; Esc cancels. A key also chooses a code
//! at once: the letter A to Z for 0x01 to 0x1a, and the keys of `@ [ \ ] ^ _ /` for NUL, ESC, FS,
//! GS, RS, US and DEL (`LVN_KEYDOWN` with the virtual key codes of the table). The chosen code is
//! then typed with `F_CTRL_CODE` so that a key macro records it.
//!
//! Not done: the help button (the help is not ported). The key `@` of a US keyboard is on the
//! key of the back quote, so both choose NUL.

use gpui::{
    AppContext as _, ClickEvent, Context, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, ParentElement, Render, ScrollHandle,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, px, rgb,
};
use ume_commands::command::Arg;

use crate::view::EditorView;

/// One control code of the list (`ctrl_info_t`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Code {
    pub code: u32,
    /// The key that chooses it: the name of the key in GPUI (`vKey` of the table).
    pub key: &'static str,
    pub name: &'static str,
    /// 説明 (`STR_ERR_DLGCTL5` + index).
    pub description: &'static str,
}

impl Code {
    /// The first column: `0x1B`.
    pub(crate) fn hex(&self) -> String {
        format!("0x{:02X}", self.code)
    }

    /// The second column: `^[` for 0x1b, `^?` for DEL (the code 0x40 and above would be `･`).
    pub(crate) fn symbol(&self) -> String {
        match self.code {
            0..=0x1f => format!("^{}", char::from(b'@' + self.code as u8)),
            0x7f => "^?".to_owned(),
            _ => "･".to_owned(),
        }
    }
}

macro_rules! code {
    ($code:expr, $key:expr, $name:expr, $description:expr) => {
        Code {
            code: $code,
            key: $key,
            name: $name,
            description: $description,
        }
    };
}

/// `p_ctrl_list` with the descriptions of the string table, in Sakura's order.
pub(crate) const CODES: [Code; 33] = [
    code!(0x00, "@", "NUL", "空文字"),
    code!(0x01, "a", "SOH", "ヘッダー開始"),
    code!(0x02, "b", "STX", "テキスト開始"),
    code!(0x03, "c", "ETX", "テキスト終了"),
    code!(0x04, "d", "EOT", "転送終了"),
    code!(0x05, "e", "ENQ", "照会"),
    code!(0x06, "f", "ACK", "受信OK"),
    code!(0x07, "g", "BEL", "警告(ベル)"),
    code!(0x08, "h", "BS", "後退"),
    code!(0x09, "i", "HT", "タブ"),
    code!(0x0a, "j", "LF", "改行"),
    code!(0x0b, "k", "VT", "垂直タブ"),
    code!(0x0c, "l", "FF", "改ページ"),
    code!(0x0d, "m", "CR", "復帰"),
    code!(0x0e, "n", "SO", "シフトアウト"),
    code!(0x0f, "o", "SI", "シフトイン"),
    code!(0x10, "p", "DLE", "データリンクエスケープ"),
    code!(0x11, "q", "DC1", "装置制御1"),
    code!(0x12, "r", "DC2", "装置制御2"),
    code!(0x13, "s", "DC3", "装置制御3"),
    code!(0x14, "t", "DC4", "装置制御4"),
    code!(0x15, "u", "NAK", "受信失敗"),
    code!(0x16, "v", "SYN", "同期"),
    code!(0x17, "w", "ETB", "転送ブロック終了"),
    code!(0x18, "x", "CAN", "キャンセル"),
    code!(0x19, "y", "EM", "メディア終了"),
    code!(0x1a, "z", "SUB", "置換"),
    code!(0x1b, "[", "ESC", "エスケープ"),
    code!(0x1c, "\\", "FS", "フォーム区切"),
    code!(0x1d, "]", "GS", "グループ区切"),
    code!(0x1e, "^", "RS", "レコード区切"),
    code!(0x1f, "_", "US", "ユニット区切"),
    code!(0x7f, "/", "DEL", "削除"),
];

/// The row a key chooses (`LVN_KEYDOWN`). Besides the keys of the table: the back quote is the
/// `@` key of a US keyboard, and `-` the `^` key of a JIS one is not needed (GPUI names the key by
/// the character).
pub(crate) fn row_of_key(key: &str) -> Option<usize> {
    let key = if key == "`" { "@" } else { key };
    CODES.iter().position(|code| code.key == key)
}

/// `F_CTRL_CODE` (`Funccode_enum.h`).
const F_CTRL_CODE: u16 = 30793;

impl EditorView {
    /// `F_CTRL_CODE_DIALOG`: opens the dialog.
    pub(crate) fn open_ctrl_code_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.support.ctrl_code.is_some() {
            return;
        }
        let view = cx.entity().downgrade();
        let dialog = cx.new(|cx| CtrlCodeDialog::new(view, cx));
        let focus = dialog.read(cx).focus.clone();
        window.focus(&focus, cx);
        self.support.ctrl_code = Some(dialog);
        cx.notify();
    }
}

/// The modal dialog over the view.
pub(crate) struct CtrlCodeDialog {
    view: WeakEntity<EditorView>,
    selected: usize,
    focus: FocusHandle,
    scroll: ScrollHandle,
}

impl CtrlCodeDialog {
    fn new(view: WeakEntity<EditorView>, cx: &mut Context<Self>) -> Self {
        Self {
            view,
            selected: 0,
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
        }
    }

    /// The row that is chosen.
    #[cfg(test)]
    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    /// Closes the dialog and gives the focus back; types the code of row `row` if there is one
    /// (`HandleCommand(F_CTRL_CODE, ...)`).
    fn finish(&mut self, row: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let view = self.view.clone();
        view.update(cx, |view, cx| {
            view.support.ctrl_code = None;
            window.focus(&view.focus_handle, cx);
            if let Some(code) = row.and_then(|row| CODES.get(row)) {
                view.run_function(F_CTRL_CODE, &[Arg::Int(i64::from(code.code))], window, cx);
            }
            cx.notify();
        })
        .ok();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let last = CODES.len() - 1;
        let key = event.keystroke.key.as_str();
        match key {
            "escape" => self.finish(None, window, cx),
            "enter" => self.finish(Some(self.selected), window, cx),
            "up" => self.selected = self.selected.saturating_sub(1),
            "down" => self.selected = (self.selected + 1).min(last),
            "pageup" => self.selected = self.selected.saturating_sub(10),
            "pagedown" => self.selected = (self.selected + 10).min(last),
            "home" => self.selected = 0,
            "end" => self.selected = last,
            _ => {
                if let Some(row) = row_of_key(key) {
                    self.selected = row;
                    self.finish(Some(row), window, cx);
                }
            }
        }
        self.scroll.scroll_to_item(self.selected);
        // the dialog is modal: no key reaches the view
        cx.stop_propagation();
        cx.notify();
    }
}

impl Focusable for CtrlCodeDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

const TEXT: u32 = 0x222222;
const CHOSEN: u32 = 0x2f6fd0;
const BACKGROUND: u32 = 0xf7f7f7;

impl Render for CtrlCodeDialog {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = CODES.iter().enumerate().map(|(i, code)| {
            let row = crate::a11y::list_row(
                div().id(("ctrl-code", i)),
                &format!(
                    "{} {} {} {}",
                    code.hex(),
                    code.symbol(),
                    code.name,
                    code.description
                ),
                i == self.selected,
            )
            .px_2()
            .h(px(24.))
            .flex()
            .items_center()
            .gap_2()
            .whitespace_nowrap()
            .child(div().w(px(56.)).child(code.hex()))
            .child(div().w(px(40.)).child(code.symbol()))
            .child(div().w(px(48.)).child(code.name))
            .child(div().w(px(180.)).child(code.description))
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                this.selected = i;
                if event.click_count() >= 2 {
                    this.finish(Some(i), window, cx);
                }
                cx.notify();
            }));
            if i == self.selected {
                row.bg(rgb(CHOSEN)).text_color(rgb(0xffffff))
            } else {
                row
            }
        });
        let button = |id: &'static str, label: &'static str| {
            crate::a11y::button(div().id(id), label)
                .px_4()
                .h(px(28.))
                .flex()
                .items_center()
                .border_1()
                .border_color(rgb(0x888888))
                .bg(rgb(0xf4f4f4))
                .child(label)
        };
        let header = div()
            .px_2()
            .h(px(24.))
            .flex()
            .items_center()
            .gap_2()
            .text_color(rgb(0x666666))
            .child(div().w(px(56.)).child("コード"))
            .child(div().w(px(40.)).child("表記"))
            .child(div().w(px(48.)).child("名前"))
            .child(div().w(px(180.)).child("説明"));
        div()
            .id("ctrl-code-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| this.finish(None, window, cx)),
            )
            .child(
                crate::a11y::dialog(div().id("ctrl-code-dialog"), "コントロールコード")
                    .track_focus(&self.focus)
                    .key_context("CtrlCodeDialog")
                    .on_key_down(cx.listener(Self::on_key_down))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .bg(rgb(BACKGROUND))
                    .border_1()
                    .border_color(rgb(0x888888))
                    .text_color(rgb(TEXT))
                    .text_size(px(14.))
                    .child("コントロールコード")
                    .child(header)
                    .child(
                        div()
                            .id("ctrl-code-list")
                            .role(gpui::Role::ListBox)
                            .aria_label("コード")
                            .max_h(px(320.))
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll)
                            .bg(rgb(0xffffff))
                            .border_1()
                            .border_color(rgb(0x888888))
                            .children(rows),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(button("ctrl-code-ok", "OK").on_click(cx.listener(
                                |this, _: &ClickEvent, window, cx| {
                                    this.finish(Some(this.selected), window, cx)
                                },
                            )))
                            .child(button("ctrl-code-cancel", "キャンセル(X)").on_click(
                                cx.listener(|this, _: &ClickEvent, window, cx| {
                                    this.finish(None, window, cx)
                                }),
                            )),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_sakuras_33_codes_in_order() {
        assert_eq!(CODES.len(), 33);
        let codes: Vec<u32> = CODES.iter().map(|c| c.code).collect();
        let expected: Vec<u32> = (0..=0x1f).chain([0x7f]).collect();
        assert_eq!(codes, expected);
        assert!(
            CODES
                .iter()
                .all(|c| !c.name.is_empty() && !c.description.is_empty())
        );
    }

    #[test]
    fn a_row_shows_the_code_the_symbol_the_name_and_the_description() {
        let esc = &CODES[0x1b];
        assert_eq!(
            (esc.hex(), esc.symbol(), esc.name, esc.description),
            ("0x1B".to_owned(), "^[".to_owned(), "ESC", "エスケープ")
        );
        assert_eq!(CODES[0].symbol(), "^@");
        assert_eq!(CODES[1].hex(), "0x01");
        assert_eq!(CODES[32].hex(), "0x7F");
        assert_eq!(CODES[32].symbol(), "^?");
    }

    #[test]
    fn the_keys_choose_the_codes_as_in_sakura() {
        // A to Z are 0x01 to 0x1a; the others are the keys of the table.
        assert_eq!(row_of_key("a"), Some(1));
        assert_eq!(row_of_key("z"), Some(26));
        assert_eq!(row_of_key("["), Some(27));
        assert_eq!(row_of_key("\\"), Some(28));
        assert_eq!(row_of_key("]"), Some(29));
        assert_eq!(row_of_key("^"), Some(30));
        assert_eq!(row_of_key("_"), Some(31));
        assert_eq!(row_of_key("/"), Some(32));
        assert_eq!(row_of_key("@"), Some(0));
        assert_eq!(row_of_key("`"), Some(0));
        assert_eq!(row_of_key("1"), None);
        assert_eq!(row_of_key("f1"), None);
    }
}
