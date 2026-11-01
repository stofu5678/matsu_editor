//! 強調キーワード on the color page: the first keyword set, `CDlgKeywordSelect` for the ten sets, and the
//! way into the sets themselves.
//!
//! Sakura: `CPropTypesColor` (`IDC_COMBO_SET`, `IDC_BUTTON_KEYWORD_SELECT`, `IDC_BUTTON_EDITKEYWORD`,
//! `sakura_core/typeprop/CPropTypesColor.cpp`), `CDlgKeywordSelect`
//! (`sakura_core/typeprop/CDlgKeywordSelect.cpp`, the dialog `IDD_DIALOG_KEYWORD_SELECT`). The type keeps
//! the number of the set for each of the ten slots (`m_nKeyWordSetIdx`, `-1` for none), in `nInts` for the
//! first two and in `nKeywordSelect3` to `nKeywordSelect10` for the rest (`sakura_core/env/CShareData_IO.cpp`,
//! `ShareData_IO_TypeInts`, `ShareData_IO_Types`). The sheet holds the ten numbers ([`KEYWORD_SETS`]) as
//! `m_nSet`; the coloring reads them from the type (`matsu-coloring`, `ColorRules::from_type`).
//!
//! * The list of the first set is on the page; the other nine are chosen in the dialog.
//! * After the dialog, `RearrangeKeywordSet` moves the sets up over the empty slots, and the colors of the
//!   slots with them ([`rearrange`]), so that a set always has the color of the slot it is in.
//! * 共通設定 (`IDC_BUTTON_EDITKEYWORD`) opens the keyword sets for editing. Sakura opens the page
//!   強調キーワード of the common settings as a dialog of its own and renumbers `m_nSet` when it is closed with OK;
//!   here the sheet of the common settings opens at that page.

use gpui::{
    AnyElement, App, Context, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyDownEvent, ParentElement as _, Render, Role, StatefulInteractiveElement as _, Styled as _,
    WeakEntity, Window, div, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::{Disableable as _, h_flex, v_flex};
use matsu_config::settings::{ColorSet, DocumentTypes};

use super::super::ids::*;
use super::super::values::names_value;
use super::child;
use super::color::sync;
use crate::common_settings::model::{Value, Values};
use crate::common_settings::{CommonSettingsDialog, Page};

/// `MAX_KEYWORDSET_PER_TYPE` and `KEYWORD_SELECT_NUM`.
pub(in crate::type_settings) const SLOTS: usize = 10;

/// The number of the color item 強調キーワード1 (`COLORIDX_KEYWORD1`).
fn first_keyword_item() -> usize {
    ColorSet::all_names()
        .position(|name| name == "KW1")
        .unwrap_or(32)
}

/// The sets of the type as the sheet holds them: always [`SLOTS`] numbers.
pub(in crate::type_settings) fn sets_of(values: &Values) -> [i32; SLOTS] {
    let mut sets = [-1; SLOTS];
    for (slot, set) in values.numbers(KEYWORD_SETS).iter().take(SLOTS).enumerate() {
        sets[slot] = *set;
    }
    sets
}

/// The names of the sets the sheet knows (`CKeyWordSetMgr::GetTypeName`).
pub(in crate::type_settings) fn names_of(values: &Values) -> Vec<String> {
    values
        .rows(KEYWORD_NAMES)
        .iter()
        .map(|row| row.first().cloned().unwrap_or_default())
        .collect()
}

/// `RearrangeKeywordSet`: every slot that has no set takes the next set below it, and the color of the
/// slot (色分け/表示, 太字, 下線, 文字色, 背景色) goes with it. The slots below the last set are empty.
pub(in crate::type_settings) fn rearrange(values: &mut Values) {
    let mut sets = sets_of(values);
    let first = first_keyword_item();
    for i in 0..SLOTS {
        if sets[i] != -1 {
            continue;
        }
        let Some(j) = (i..SLOTS).find(|&j| sets[j] != -1) else {
            // Nothing below is set.
            break;
        };
        sets[i] = sets[j];
        sets[j] = -1;
        for field in Field::ALL {
            let (a, b) = (item_key(first + i, field), item_key(first + j, field));
            if let (Some(x), Some(y)) = (values.get(a).cloned(), values.get(b).cloned()) {
                values.set(a, y);
                values.set(b, x);
            }
        }
    }
    values.set(KEYWORD_SETS, Value::Numbers(sets.to_vec()));
    // The controls show the chosen item again (`InvalidateRect` of the color buttons and the list).
    sync(values, COLOR_SEL);
}

/// `GetData` of the dialog: the number of the line chosen in each list, minus the blank first line. A set that
/// the list does not have is no choice (`CB_ERR`), so it becomes `-1`.
pub(in crate::type_settings) fn normalize(sets: [i32; SLOTS], count: usize) -> [i32; SLOTS] {
    sets.map(|set| match usize::try_from(set) {
        Ok(at) if at < count => set,
        _ => -1,
    })
}

/// The first keyword set: the list `IDC_COMBO_SET`.
pub(in crate::type_settings) fn first_set(
    dialog: &CommonSettingsDialog,
    cx: &mut Context<CommonSettingsDialog>,
) -> AnyElement {
    let values = dialog.values();
    let chosen = sets_of(values)[0];
    let weak = cx.entity().downgrade();
    h_flex()
        .gap_2()
        .items_center()
        .child("強調キーワード1")
        .child(child::set_dropdown(
            "keyword-set-1",
            chosen,
            names_of(values),
            move |set, _, cx: &mut App| {
                weak.update(cx, |dialog, cx| dialog.pick_first_set(set, cx))
                    .ok();
            },
        ))
        .into_any_element()
}

/// How often the sheet looks whether the common settings are closed.
const WATCH: std::time::Duration = std::time::Duration::from_millis(500);

/// The sets type `index` has in the settings file now.
fn file_sets(cx: &App, index: usize) -> Option<[i32; SLOTS]> {
    let profile = crate::type_settings::current_profile(cx);
    DocumentTypes::from_profile(&profile)
        .get(index)
        .map(|t| t.keyword_set_idx)
}

/// The dialog window.
pub(in crate::type_settings) struct KeywordSelectDialog {
    sets: [i32; SLOTS],
    /// The sets when the dialog opened: only the slots the user changed here are taken at OK, so that a
    /// choice made on the sheet meanwhile stays.
    initial: [i32; SLOTS],
    names: Vec<String>,
    sheet: WeakEntity<CommonSettingsDialog>,
    focus: FocusHandle,
    focused: bool,
}

impl KeywordSelectDialog {
    /// The sets the lists show now, for the tests.
    #[cfg(test)]
    pub(in crate::type_settings) fn sets(&self) -> [i32; SLOTS] {
        self.sets
    }

    /// The user chose `set` in the list of `slot`.
    pub(in crate::type_settings) fn choose(
        &mut self,
        slot: usize,
        set: i32,
        cx: &mut Context<Self>,
    ) {
        if let Some(target) = self.sets.get_mut(slot) {
            *target = set;
            cx.notify();
        }
    }

    /// `IDOK`.
    pub(in crate::type_settings) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (chosen, initial) = (self.sets, self.initial);
        // The sets the file has now, not the ones the dialog opened with (the sets may have been edited
        // in the common settings meanwhile).
        let count = crate::type_settings::keyword_names(&DocumentTypes::from_profile(
            &crate::type_settings::current_profile(cx),
        ))
        .len();
        self.sheet
            .update(cx, |sheet, cx| {
                sheet.edit_values(
                    |values| {
                        let mut sets = sets_of(values);
                        for slot in 0..SLOTS {
                            if chosen[slot] != initial[slot] {
                                sets[slot] = chosen[slot];
                            }
                        }
                        let sets = normalize(sets, count);
                        values.set(KEYWORD_SETS, Value::Numbers(sets.to_vec()));
                        rearrange(values);
                    },
                    cx,
                );
            })
            .ok();
        window.remove_window();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => window.remove_window(),
            "enter" if !event.keystroke.modifiers.alt => self.ok(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Focusable for KeywordSelectDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for KeywordSelectDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        child::focus_once(&mut self.focused, &self.focus, window, cx);
        let weak = cx.entity().downgrade();
        let rows: Vec<AnyElement> = (0..SLOTS)
            .map(|slot| {
                let weak = weak.clone();
                let shown =
                    if usize::try_from(self.sets[slot]).is_ok_and(|at| at < self.names.len()) {
                        self.sets[slot]
                    } else {
                        -1
                    };
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .w(px(110.))
                            .child(format!("強調キーワード{}", slot + 1)),
                    )
                    .child(child::set_dropdown(
                        ("keyword-select", slot),
                        shown,
                        self.names.clone(),
                        move |set, _, cx: &mut App| {
                            weak.update(cx, |dialog, cx| dialog.choose(slot, set, cx))
                                .ok();
                        },
                    ))
                    .into_any_element()
            })
            .collect();
        div()
            .id("keyword-select-dialog")
            .role(Role::Dialog)
            .aria_label("強調キーワードの設定")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down))
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(rgb(0xf0f0f0))
            .text_color(rgb(0x222222))
            .text_size(px(13.))
            .child(v_flex().id("keyword-select-rows").gap_2().children(rows))
            .child(
                h_flex()
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("keyword-select-ok")
                            .label("OK")
                            .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
                    )
                    .child(
                        Button::new("keyword-select-cancel")
                            .label("キャンセル(X)")
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("keyword-select-help")
                            .label("ヘルプ(H)")
                            .disabled(true),
                    ),
            )
    }
}

impl CommonSettingsDialog {
    /// The names of the keyword sets the settings file has now (`CKeyWordSetMgr`), taken into the
    /// sheet. The sets are edited in the common settings while the sheet is open.
    fn refresh_keyword_names(&mut self, cx: &mut Context<Self>) {
        let profile = crate::type_settings::current_profile(cx);
        let names = crate::type_settings::keyword_names(&DocumentTypes::from_profile(&profile));
        if names != names_of(self.values()) {
            self.edit_values(|values| values.set(KEYWORD_NAMES, names_value(&names)), cx);
        }
    }

    /// The first set was chosen in its list.
    pub(in crate::type_settings) fn pick_first_set(&mut self, set: i32, cx: &mut Context<Self>) {
        self.refresh_keyword_names(cx);
        self.edit_values(
            |values| {
                let mut sets = sets_of(values);
                sets[0] = set;
                values.set(KEYWORD_SETS, Value::Numbers(sets.to_vec()));
            },
            cx,
        );
    }

    /// 「2～10...」: `CDlgKeywordSelect::DoModal` with the sets of the sheet.
    pub(in crate::type_settings) fn keyword_select(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Entity<KeywordSelectDialog>> {
        self.refresh_keyword_names(cx);
        let sets = sets_of(self.values());
        let names = names_of(self.values());
        let sheet = cx.entity().downgrade();
        let watched = sheet.clone();
        child::open(
            cx,
            "強調キーワードの設定",
            330.,
            470.,
            move |window, cx| {
                child::close_with_sheet(watched, window, cx);
                KeywordSelectDialog {
                    sets,
                    initial: sets,
                    names,
                    sheet,
                    focus: cx.focus_handle(),
                    focused: false,
                }
            },
        )
    }

    /// 「共通設定...」: the keyword sets are edited on the page 強調キーワード of the common settings.
    pub(in crate::type_settings) fn keyword_edit(&mut self, cx: &mut Context<Self>) {
        let index = usize::try_from(self.values().int(TYPE_INDEX)).unwrap_or(0);
        let before = file_sets(cx, index);
        crate::common_settings::open(cx, Some(Page::Keyword), self.opener());
        // When that sheet is closed, the names are read again, and the sets of this sheet follow the
        // renumbering it did to the type (`keyword/store.rs`) if the user had not chosen others.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(WATCH).await;
                if cx.update(|app| crate::common_settings::current_dialog(app).is_none()) {
                    break;
                }
            }
            this.update(cx, |dialog, cx| {
                dialog.after_keyword_edit(index, before, cx)
            })
            .ok();
        })
        .detach();
    }

    /// The common settings were closed: take the names, and the renumbered sets of the type.
    fn after_keyword_edit(
        &mut self,
        index: usize,
        before: Option<[i32; SLOTS]>,
        cx: &mut Context<Self>,
    ) {
        self.refresh_keyword_names(cx);
        let now = file_sets(cx, index);
        if let (Some(before), Some(now)) = (before, now) {
            // Each slot the user has not changed follows the renumbering; a slot the user changed keeps
            // the choice (its number is the one the sets had when it was chosen).
            self.edit_values(
                |values| {
                    let mut sets = sets_of(values);
                    for slot in 0..SLOTS {
                        if sets[slot] == before[slot] {
                            sets[slot] = now[slot];
                        }
                    }
                    values.set(KEYWORD_SETS, Value::Numbers(sets.to_vec()));
                },
                cx,
            );
        }
    }
}
