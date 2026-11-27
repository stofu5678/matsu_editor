//! The file association of the type list: the check boxes 右クリックメニューに追加 and
//! ダブルクリックで開く (`IDC_CHECK_EXT_RMENU`, `IDC_CHECK_EXT_DBLCLICK`) and the warning before
//! the first change (`CDlgTypeList::AlertFileAssociation`, `STR_DLGTYPELIST_ACC`).
//!
//! Sakura: `CDlgTypeList::DispatchEvent` (`LBN_SELCHANGE` and the two `BN_CLICKED`) in
//! `sakura_core/typeprop/CDlgTypeList.cpp`. The registry is in `matsu_platform::file_association`;
//! only Windows has it. On macOS and Linux the check boxes are not shown (as before): which way
//! they should work there is the user's decision.
//!
//! The state of the selected type is read from the registry when the dialog opens, when the
//! selection changes and after every write (`LBN_SELCHANGE`), and kept only to draw the check
//! boxes. A click never trusts it: it reads the registry again and toggles from that, so a type
//! that shares an extension with another, or another program that changed the registry while the
//! dialog is open, cannot make the click do the wrong thing.
//!
//! The check boxes have no access keys in Sakura (`IDC_CHECK_EXT_RMENU` and `IDC_CHECK_EXT_DBLCLICK`
//! in `sakura_core/sakura_rc.rc`, lines 684-685: no `&` in the labels). Here they are tab stops of
//! the dialog and Space presses the one that has the focus; a grey one is not a tab stop.

use std::io;

use gpui::{
    AnyElement, App, ClickEvent, Context, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, StatefulInteractiveElement, Styled, Window, div, px, rgb,
};
use matsu_platform::file_association::{self, Change, TypeState};

use super::list::TypeList;
use super::with_user;

/// `STR_DLGTYPELIST_ACC`: asked before the first change in the dialog.
const ALERT: &str = "Windowsの関連付け設定を変更しようとしています。\n\
    この操作は同じ設定を利用する他のソフトにも影響を与える可能性があります。\n\
    実施しますか？";
/// `STR_DLGTYPELIST_ERR1`: the association failed.
const ERROR_ASSOCIATE: &str = "関連付けに失敗しました\n";
/// `STR_DLGTYPELIST_ERR2`: removing the association failed.
const ERROR_UNASSOCIATE: &str = "関連付け解除に失敗しました\n";
/// The state could not be read (not in Sakura: it has no such failure).
const ERROR_READ: &str = "関連付けの状態を読めませんでした\n";

/// The labels of the check boxes (`IDD_TYPELIST`: `sakura_core/sakura_rc.rc`).
const MENU_LABEL: &str = "右クリックメニューに追加";
const DEFAULT_LABEL: &str = "ダブルクリックで開く";

/// Where the association is read and written: the registry of Windows, or a fake in the tests.
pub(super) trait Backend {
    /// Does this OS have the association at all?
    fn supported(&self) -> bool;
    fn read(&self, exts: &str) -> io::Result<TypeState>;
    fn write(&self, exts: &str, change: Change) -> io::Result<()>;
}

/// The real thing (`matsu_platform::file_association`).
pub(super) struct System;

impl Backend for System {
    fn supported(&self) -> bool {
        file_association::supported()
    }

    fn read(&self, exts: &str) -> io::Result<TypeState> {
        file_association::current_type_state(exts)
    }

    fn write(&self, exts: &str, change: Change) -> io::Result<()> {
        file_association::apply_to_system(exts, change)
    }
}

/// What the check boxes show: the state read for the selected type, or why it could not be read.
pub(super) struct Shown {
    state: Result<TypeState, String>,
}

/// What to tell the user about the extensions where Windows opens another program for the double
/// click although MATSU is registered (`UserChoice`, which a program cannot change). `None` when
/// there are none.
pub(super) fn overridden_note(extensions: &[String]) -> Option<String> {
    if extensions.is_empty() {
        return None;
    }
    let list = extensions
        .iter()
        .map(|e| format!(".{e}"))
        .collect::<Vec<_>>()
        .join(" ");
    Some(format!(
        "{list} は、Windows の「既定のアプリ」で別のアプリが選ばれているため、\
         ダブルクリックではそのアプリが開きます。MATSU Editor で開くには、Windows の設定の\
         「アプリ」の「既定のアプリ」で、拡張子ごとに MATSU Editor を選んでください。"
    ))
}

impl TypeList {
    /// The extension list of the selected type.
    fn selected_exts(&self, cx: &App) -> String {
        with_user(cx, |user| {
            user.document_types()
                .get(self.selected)
                .map(|t| t.type_exts.clone())
                .unwrap_or_default()
        })
    }

    /// Reads what the check boxes show for the selected type. Called when the selection changes,
    /// not while drawing. A failure is kept as the text to show and is read again next time.
    pub(super) fn read_association(&mut self, cx: &App) {
        self.association = if self.backend.supported() {
            let exts = self.selected_exts(cx);
            Some(Shown {
                state: self.backend.read(&exts).map_err(|e| e.to_string()),
            })
        } else {
            None
        };
    }

    /// The click on a check box, or Space on it. The warning comes first, once for the dialog.
    fn change_association(&mut self, menu: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.backend.supported() {
            return;
        }
        let exts = self.selected_exts(cx);
        // From the registry as it is now, not from what was drawn.
        let state = match self.backend.read(&exts) {
            Ok(state) => state,
            Err(e) => {
                self.say(&format!("{ERROR_READ}{e}"), window, cx);
                self.read_association(cx);
                cx.notify();
                return;
            }
        };
        // ダブルクリック is grey while 右クリック is off (`EnableWindow(hwndDblClick, m_bExtRMenu)`).
        if !state.available || (!menu && !state.menu) {
            return;
        }
        let change = if menu {
            Change::Menu(!state.menu)
        } else {
            Change::Default(!state.default)
        };
        if self.alerted {
            self.write_association(&exts, change, window, cx);
        } else {
            self.confirm(ALERT, window, cx, move |this, window, cx| {
                this.alerted = true;
                this.write_association(&exts, change, window, cx);
            });
        }
    }

    /// Writes the registry for the type with the extension list `exts`, says why when it fails, and
    /// reads the state again either way.
    fn write_association(
        &mut self,
        exts: &str,
        change: Change,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(e) = self.backend.write(exts, change) {
            let head = if change == Change::Menu(false) {
                ERROR_UNASSOCIATE
            } else {
                ERROR_ASSOCIATE
            };
            self.say(&format!("{head}{e}"), window, cx);
        }
        self.read_association(cx);
        cx.notify();
    }

    /// The check boxes under the list; nothing where this OS has no association.
    pub(super) fn render_association(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let shown = self.association.as_ref()?;
        let (menu_on, default_on, available, can_default, note) = match &shown.state {
            Ok(s) => (
                s.menu,
                s.default,
                s.available,
                s.available && s.menu,
                if s.default {
                    overridden_note(&s.overridden)
                } else {
                    None
                },
            ),
            Err(e) => (false, false, false, false, Some(format!("{ERROR_READ}{e}"))),
        };
        let menu = crate::a11y::disabled(
            check_box(
                "type-list-ext-menu",
                MENU_LABEL,
                menu_on,
                available.then_some(&self.menu_focus),
            )
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.change_association(true, window, cx)
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "space" {
                    this.change_association(true, window, cx);
                    cx.stop_propagation();
                }
            })),
            !available,
        );
        let default = crate::a11y::disabled(
            check_box(
                "type-list-ext-default",
                DEFAULT_LABEL,
                default_on,
                can_default.then_some(&self.default_focus),
            )
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.change_association(false, window, cx)
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "space" {
                    this.change_association(false, window, cx);
                    cx.stop_propagation();
                }
            })),
            !can_default,
        );
        Some(
            div()
                .mt_2()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().flex().child(menu).child(default))
                .children(note.map(|text| {
                    crate::a11y::text(div().id("type-list-ext-note"), &text)
                        .max_w(px(460.))
                        .text_size(px(12.))
                        .text_color(rgb(0x9a3412))
                        .child(text)
                }))
                .into_any_element(),
        )
    }
}

/// A check box. `focus` is its focus handle while it can be used (a tab stop), and `None` while
/// it is grey.
fn check_box(
    id: &'static str,
    label: &'static str,
    checked: bool,
    focus: Option<&FocusHandle>,
) -> gpui::Stateful<gpui::Div> {
    let color = if focus.is_some() {
        super::list::TEXT
    } else {
        0x888888
    };
    let element = div()
        .id(id)
        .role(gpui::Role::CheckBox)
        .aria_label(label)
        .aria_toggled(if checked {
            gpui::Toggled::True
        } else {
            gpui::Toggled::False
        })
        .flex()
        .items_center()
        .gap_2()
        .mr_4()
        .text_color(rgb(color))
        .child(
            div()
                .w(px(16.))
                .h(px(16.))
                .border_1()
                .border_color(rgb(0x666666))
                .bg(rgb(0xffffff))
                .flex()
                .items_center()
                .justify_center()
                .child(if checked { "✓" } else { "" }),
        )
        .child(label);
    match focus {
        Some(focus) => element.track_focus(focus).cursor_pointer(),
        None => element,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_note_names_the_extensions_and_says_where_to_change_it() {
        assert_eq!(overridden_note(&[]), None);
        let note = overridden_note(&["c".to_owned(), "h".to_owned()]).unwrap();
        assert!(note.starts_with(".c .h は"), "{note}");
        assert!(note.contains("既定のアプリ"), "{note}");
    }
}
