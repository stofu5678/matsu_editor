//! 「文字色統一」と「背景色統一」: `CDlgSameColor` (`sakura_core/typeprop/CDlgSameColor.cpp`, the
//! dialog `IDD_SAMECOLOR` of `sakura_rc.rc`).
//!
//! The dialog is opened from 文字色統一 or 背景色統一 on the color page, with the color of the chosen
//! item (統一色). It lists the other text (or background) colors the items of the type use, each with a
//! check box, and for the color on the caret the names of the items that use it. OK sets every item
//! that has a checked color to 統一色.
//!
//! # Where this differs from Sakura
//!
//! * `OnSelChangeListColors` skips the items by `g_ColorAttributeArr[i]`, the number of the color in the
//!   list, instead of the item `j` it is looking at (the text case only; the background case uses `j`).
//!   As written, the names shown depend on the position of the color in the list, so the dialog
//!   sometimes names an item that has no text color, or leaves one out. Here an item is skipped by its
//!   own attribute (type A).
//! * OK sets every item (`j < COLORIDX_LAST`) whose color is checked, including items that do not have
//!   such a color and that the lists never showed. Here only the items the dialog showed are set, so that
//!   a color the user cannot see or change on the page stays as it is (type A).

use gpui::{
    AnyElement, Context, FocusHandle, Focusable, InteractiveElement as _, IntoElement,
    KeyDownEvent, ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _,
    Styled as _, WeakEntity, Window, div, prelude::FluentBuilder as _, px, rgb,
};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::{Disableable as _, h_flex, v_flex};
use matsu_config::settings::ColorSet;

use super::super::ids::*;
use super::super::values::{color_of, color_value};
use super::child;
use super::color::{COLOR_ITEMS, sync};
use crate::common_settings::CommonSettingsDialog;
use crate::common_settings::model::{Value, Values};

/// Which color the dialog unifies (`m_wID`: `IDC_BUTTON_SAMETEXTCOLOR` or `IDC_BUTTON_SAMEBKCOLOR`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::type_settings) enum Which {
    Text,
    Back,
}

impl Which {
    /// The caption (`STR_DLGSMCLR_BTN1`, `STR_DLGSMCLR_BTN2`).
    pub(in crate::type_settings) fn caption(self) -> &'static str {
        match self {
            Which::Text => "文字色統一",
            Which::Back => "背景色統一",
        }
    }

    fn field(self) -> Field {
        match self {
            Which::Text => Field::Text,
            Which::Back => Field::Back,
        }
    }

    /// Does the item called `name` have this color (`COLOR_ATTRIB_NO_TEXT`, `COLOR_ATTRIB_NO_BACK`)?
    fn item_has(self, name: &str) -> bool {
        let Some(attributes) = ColorSet::attributes(name) else {
            return false;
        };
        match self {
            Which::Text => !attributes.no_text,
            Which::Back => !attributes.no_back,
        }
    }
}

/// An item of the type and the color the dialog looks at.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    /// The name on the list of the color page.
    name: &'static str,
    color: u32,
    /// The item has this color (`g_ColorAttributeArr`).
    has: bool,
}

/// What the dialog holds while it is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::type_settings) struct SameColor {
    which: Which,
    /// 統一色 (`m_cr`).
    color: u32,
    entries: Vec<Entry>,
    /// The colors of the list `IDC_LIST_COLORS` and whether each is checked (the item data).
    colors: Vec<(u32, bool)>,
    /// The color on the caret, which the lower list shows the items of.
    caret: Option<usize>,
}

impl SameColor {
    /// `OnInitDialog`: the colors of the items that have one, except 統一色, once each in the order
    /// the items have them, all unchecked; the first is on the caret.
    pub(in crate::type_settings) fn new(values: &Values, which: Which, color: u32) -> Self {
        let entries: Vec<Entry> = ColorSet::all_names()
            .enumerate()
            .map(|(item, name)| Entry {
                name: COLOR_ITEMS.get(item).map_or("", |(label, _)| label),
                color: color_of(values.int(item_key(item, which.field()))),
                has: which.item_has(name),
            })
            .collect();
        let mut colors: Vec<(u32, bool)> = Vec::new();
        for entry in entries.iter().filter(|e| e.has && e.color != color) {
            if !colors.iter().any(|(c, _)| *c == entry.color) {
                colors.push((entry.color, false));
            }
        }
        let caret = (!colors.is_empty()).then_some(0);
        Self {
            which,
            color,
            entries,
            colors,
            caret,
        }
    }

    pub(in crate::type_settings) fn colors(&self) -> &[(u32, bool)] {
        &self.colors
    }

    pub(in crate::type_settings) fn caret(&self) -> Option<usize> {
        self.caret
    }

    /// `LBN_SELCHANGE`: the color at `at` is on the caret.
    pub(in crate::type_settings) fn set_caret(&mut self, at: usize) {
        if at < self.colors.len() {
            self.caret = Some(at);
        }
    }

    /// A click on the check box of the color at `at`.
    pub(in crate::type_settings) fn set_checked(&mut self, at: usize, on: bool) {
        if let Some(color) = self.colors.get_mut(at) {
            color.1 = on;
        }
    }

    /// 全チェック and 全解除.
    pub(in crate::type_settings) fn check_all(&mut self, on: bool) {
        for color in &mut self.colors {
            color.1 = on;
        }
    }

    /// `OnSelChangeListColors`: the names of the items that use the color on the caret.
    pub(in crate::type_settings) fn names_on_caret(&self) -> Vec<&'static str> {
        let Some((color, _)) = self.caret.and_then(|at| self.colors.get(at)) else {
            return Vec::new();
        };
        self.entries
            .iter()
            .filter(|e| e.has && e.color == *color)
            .map(|e| e.name)
            .collect()
    }

    /// `IDOK`: puts 統一色 into the items that have a checked color (as the sheet has them now). Returns
    /// whether an item changed.
    pub(in crate::type_settings) fn apply(&self, values: &mut Values) -> bool {
        let mut changed = false;
        for (item, entry) in self.entries.iter().enumerate() {
            if !entry.has {
                continue;
            }
            // The sheet stays usable while the dialog is open: the color the item has now counts, not
            // the one it had when the dialog opened.
            let now = color_of(values.int(item_key(item, self.which.field())));
            if now == self.color {
                continue;
            }
            if self.colors.iter().any(|(c, on)| *on && *c == now) {
                values.set(
                    item_key(item, self.which.field()),
                    Value::Int(color_value(self.color)),
                );
                changed = true;
            }
        }
        if changed {
            // The controls show the chosen item again (`InvalidateRect` of the list and the buttons).
            sync(values, COLOR_SEL);
        }
        changed
    }
}

/// A `COLORREF` (`0x00BBGGRR`) as a GPUI color.
fn rgb_of(color: u32) -> gpui::Rgba {
    rgb(((color & 0xff) << 16) | (color & 0xff00) | ((color >> 16) & 0xff))
}

const BORDER: u32 = 0xaaaaaa;
const SELECTED_BG: u32 = 0x2f6fd0;
const ROW_HEIGHT: f32 = 24.0;

/// The dialog window.
pub(in crate::type_settings) struct SameColorDialog {
    state: SameColor,
    sheet: WeakEntity<CommonSettingsDialog>,
    focus: FocusHandle,
    focused: bool,
}

impl SameColorDialog {
    pub(in crate::type_settings) fn ok(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.state.clone();
        self.sheet
            .update(cx, |sheet, cx| {
                sheet.edit_values(
                    |values| {
                        state.apply(values);
                    },
                    cx,
                );
            })
            .ok();
        window.remove_window();
    }

    /// The state, for the tests.
    #[cfg(test)]
    pub(in crate::type_settings) fn state_mut(&mut self) -> &mut SameColor {
        &mut self.state
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => window.remove_window(),
            "enter" if !event.keystroke.modifiers.alt => self.ok(window, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    fn list(&self, cx: &mut Context<Self>) -> AnyElement {
        let caret = self.state.caret();
        let rows: Vec<AnyElement> = self
            .state
            .colors()
            .iter()
            .enumerate()
            .map(|(at, (color, checked))| {
                let name = format!("{color:06x}");
                h_flex()
                    .id(("same-color", at))
                    .role(Role::ListBoxOption)
                    .aria_label(name.clone())
                    .h(px(ROW_HEIGHT))
                    .px_1()
                    .gap_2()
                    .items_center()
                    .cursor_pointer()
                    .when(caret == Some(at), |row| row.bg(rgb(SELECTED_BG)))
                    .child(
                        Checkbox::new(("same-color-check", at))
                            .checked(*checked)
                            .accessibility_label(name)
                            .on_click(cx.listener(move |this, on: &bool, _, cx| {
                                this.state.set_checked(at, *on);
                                this.state.set_caret(at);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h(px(ROW_HEIGHT - 6.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(BORDER))
                            .bg(rgb_of(*color)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state.set_caret(at);
                        cx.notify();
                    }))
                    .into_any_element()
            })
            .collect();
        v_flex()
            .id("same-color-list")
            .role(Role::ListBox)
            .aria_label("変更対象の色")
            .h(px(180.))
            .flex_1()
            .overflow_y_scroll()
            .bg(rgb(0xffffff))
            .border_1()
            .border_color(rgb(BORDER))
            .children(rows)
            .into_any_element()
    }
}

impl Focusable for SameColorDialog {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SameColorDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        child::focus_once(&mut self.focused, &self.focus, window, cx);
        let names: Vec<SharedString> = self
            .state
            .names_on_caret()
            .into_iter()
            .map(SharedString::from)
            .collect();
        div()
            .id("same-color")
            .role(Role::Dialog)
            .aria_label(self.state.which.caption())
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
            .child(
                h_flex().gap_2().items_center().child("統一色").child(
                    div()
                        .w(px(86.))
                        .h(px(20.))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(BORDER))
                        .bg(rgb_of(self.state.color)),
                ),
            )
            .child("変更対象の色(C)を選んでチェック状態にしてください")
            .child(
                h_flex().gap_2().items_start().child(self.list(cx)).child(
                    v_flex()
                        .gap_2()
                        .child(
                            Button::new("same-color-all")
                                .label("全チェック(A)")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.state.check_all(true);
                                    cx.notify();
                                })),
                        )
                        .child(Button::new("same-color-none").label("全解除(N)").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.state.check_all(false);
                                cx.notify();
                            }),
                        )),
                ),
            )
            .child(
                v_flex()
                    .id("same-color-items")
                    .role(Role::ListBox)
                    .aria_label("選択中の色を使っている項目")
                    .h(px(110.))
                    .overflow_y_scroll()
                    .bg(rgb(0xe6e6e6))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .px_1()
                    .children(names),
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("same-color-ok")
                            .label("OK")
                            .on_click(cx.listener(|this, _, window, cx| this.ok(window, cx))),
                    )
                    .child(
                        Button::new("same-color-cancel")
                            .label("キャンセル(X)")
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("same-color-help")
                            .label("ヘルプ(H)")
                            .disabled(true),
                    ),
            )
    }
}

impl CommonSettingsDialog {
    /// 「文字色統一」 and 「背景色統一」: `CDlgSameColor::DoModal` with the color of the chosen item.
    pub(in crate::type_settings) fn same_color(
        &mut self,
        which: Which,
        cx: &mut Context<Self>,
    ) -> Option<gpui::Entity<SameColorDialog>> {
        let item = usize::try_from(self.values().int(COLOR_SEL)).unwrap_or(0);
        let color = color_of(self.values().int(item_key(item, which.field())));
        let state = SameColor::new(self.values(), which, color);
        let sheet = cx.entity().downgrade();
        let watched = sheet.clone();
        child::open(cx, which.caption(), 340., 470., move |window, cx| {
            child::close_with_sheet(watched, window, cx);
            SameColorDialog {
                state,
                sheet,
                focus: cx.focus_handle(),
                focused: false,
            }
        })
    }
}
