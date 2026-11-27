//! The file association of the type list: the check boxes 右クリックメニューに追加 and
//! ダブルクリックで開く (`IDC_CHECK_EXT_RMENU`, `IDC_CHECK_EXT_DBLCLICK`) and the warning before
//! the first change (`CDlgTypeList::AlertFileAssociation`, `STR_DLGTYPELIST_ACC`).
//!
//! Sakura: `CDlgTypeList::DispatchEvent` (`LBN_SELCHANGE` and the two `BN_CLICKED`) in
//! `sakura_core/typeprop/CDlgTypeList.cpp`. The registry is in `matsu_platform::file_association`;
//! only Windows has it. On macOS and Linux the check boxes are not shown (as before): which way
//! they should work there is the user's decision.
//!
//! The state of a type is read from the registry the first time the type is selected and kept for
//! the dialog (`m_bRegistryChecked`). It is dropped for a type when this dialog changes it (the
//! state is read again then); a change made by another program while the dialog is open is not
//! seen until the dialog is opened again.

use gpui::{
    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px, rgb,
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

/// The labels of the check boxes (`IDD_TYPELIST`: `sakura_core/sakura_rc.rc`).
const MENU_LABEL: &str = "右クリックメニューに追加";
const DEFAULT_LABEL: &str = "ダブルクリックで開く";

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

    /// What the check boxes show for the selected type, read from the registry when the type has
    /// not been looked at yet. `None` where this OS has no association.
    fn association(&mut self, cx: &App) -> Option<(String, TypeState)> {
        if !file_association::supported() {
            return None;
        }
        let exts = self.selected_exts(cx);
        let state = self
            .associations
            .entry(exts.clone())
            .or_insert_with(|| file_association::current_type_state(&exts))
            .clone();
        Some((exts, state))
    }

    /// The click on a check box. The warning comes first, once for the dialog.
    fn change_association(&mut self, menu: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some((exts, state)) = self.association(cx) else {
            return;
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
        if let Err(e) = file_association::apply_to_system(exts, change) {
            let head = if change == Change::Menu(false) {
                ERROR_UNASSOCIATE
            } else {
                ERROR_ASSOCIATE
            };
            self.say(&format!("{head}{e}"), window, cx);
        }
        self.associations.remove(exts);
        cx.notify();
    }

    /// The check boxes under the list; nothing where this OS has no association.
    pub(super) fn render_association(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (_, state) = self.association(cx)?;
        let check_box = |id: &'static str, label: &'static str, checked: bool, enabled: bool| {
            let color = if enabled { super::list::TEXT } else { 0x888888 };
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
            if enabled {
                element.cursor_pointer()
            } else {
                element
            }
        };
        let menu = crate::a11y::disabled(
            check_box(
                "type-list-ext-menu",
                MENU_LABEL,
                state.menu,
                state.available,
            )
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.change_association(true, window, cx)
            })),
            !state.available,
        );
        let default_enabled = state.available && state.menu;
        let default = crate::a11y::disabled(
            check_box(
                "type-list-ext-default",
                DEFAULT_LABEL,
                state.default,
                default_enabled,
            )
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                this.change_association(false, window, cx)
            })),
            !default_enabled,
        );
        let note = if state.default {
            overridden_note(&state.overridden)
        } else {
            None
        };
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
