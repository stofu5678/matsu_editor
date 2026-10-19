//! 「ツールバー」: `IDD_PROP_TOOLBAR` (`sakura_core/sakura_rc.rc`), `CPropToolbar`
//! (`sakura_core/prop/CPropComToolbar.cpp`).
//!
//! The page has two lists: the functions of a kind (種別) that can be a button, with their icons,
//! and the buttons of the toolbar. The buttons between them (削除, ---, →, >>, ↑, ↓, 折返) change
//! the toolbar list. OK writes `[Toolbar]` (`nToolBarButtonNum`, `nTBB[000]`...:
//! `CShareData_IO::ShareData_IO_Toolbar`) under the lock of the settings file, and every window's
//! toolbar changes at once.
//!
//! The list of buttons is a [`Kind::Numbers`] control (its keys are in `[Toolbar]`, not
//! `[Common]`) and the page draws it itself ([`Item::Custom`]); the choices in the lists are
//! controls the dialog does not draw ([`Item::Hidden`]), so that the buttons' Alt keys work.
//!
//! The kind 外部マクロ lists the 50 registered macros (the page マクロ) with their names.
//!
//! Not built: the plugin commands (the kind プラグイン has none), `bToolBarIsFlat` (the buttons are
//! always flat).

use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement as _, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, div, img, prelude::FluentBuilder as _,
    px, rgb,
};
use gpui_component::button::Button;
use gpui_component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_component::{Disableable as _, h_flex, v_flex};
use ume_commands::sakura::FUNCTIONS;
use ume_config::settings::macros::RegisteredMacros;
use ume_config::{Profile, ProfileError};

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::func_lookup::{KINDS, codes_of_kind};
use super::super::model::{Control, Item, Value, Values};
use crate::bars::toolbar::ToolbarButtons;
use crate::bars::{self, icons};

const SECTION: &str = "Toolbar";
/// `MAX_TOOLBAR_BUTTON_ITEMS` (`CommonSetting.h`).
const MAX_BUTTONS: usize = 512;
/// `CMenuDrawer::TOOLBAR_BUTTON_F_TOOLBARWRAP`: the number of the button that wraps the line.
const WRAP: i32 = 384;
/// `STR_PROPCOMTOOL_ITEM1` to `ITEM3`.
const ITEM_SEPARATOR: &str = "───────────";
const ITEM_WRAP: &str = "――ツールバー折返――";
const ITEM_UNKNOWN: &str = "────不　明────";

/// The functions of a kind that have a button, as (button number, function code)
/// (`FindToolbarNoFromCommandId`: a function with no button is left out).
pub(super) fn functions_of_kind(kind: i32) -> Vec<(i32, u16)> {
    codes_of_kind(kind)
        .into_iter()
        .filter_map(|code| Some((bars::icon_of_function(code)? as i32 + 1, code)))
        .collect()
}

/// What a row of a list shows for button `number` (`DrawToolBarItemList`): the function's name,
/// or the text of a separator, a wrap or an unknown button; and the icon.
///
/// The name of a registered macro (a button of the kind 外部マクロ) is the one of `macros`.
pub(super) fn row_of(number: i32, macros: &RegisteredMacros) -> (String, Option<usize>) {
    match number {
        0 => (ITEM_SEPARATOR.to_owned(), None),
        WRAP => (ITEM_WRAP.to_owned(), None),
        n => {
            let known = usize::try_from(n - 1)
                .ok()
                .and_then(|i| bars::button_function(i).map(|code| (i, code)))
                .filter(|&(_, code)| code != 0);
            match known {
                Some((index, code)) => {
                    let mut name = crate::macros::title_of(macros, code)
                        .unwrap_or_else(|| bars::function_name(code));
                    if name.is_empty() {
                        name = FUNCTIONS
                            .iter()
                            .find(|f| f.code == code)
                            .map_or_else(String::new, |f| f.name.to_owned());
                    }
                    (name, bars::icon_of_function(code).filter(|&i| i == index))
                }
                None => (ITEM_UNKNOWN.to_owned(), None),
            }
        }
    }
}

// ---- the edits of the list (the handlers of the buttons in `DispatchEvent`) ----

/// The list of buttons with the selected row and the selected function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Edit {
    pub list: Vec<i32>,
    /// The selected row of the toolbar list (`None` is `LB_ERR`).
    pub sel: Option<usize>,
    /// The selected row of the function list.
    pub func_sel: Option<usize>,
    /// How many functions the function list has.
    pub func_count: usize,
}

impl Edit {
    fn insert_at(&mut self, at: usize, number: i32) -> bool {
        if self.list.len() >= MAX_BUTTONS {
            return false;
        }
        self.list.insert(at.min(self.list.len()), number);
        true
    }

    /// 「---」: a separator before the selected row (the top when none is selected); it becomes
    /// the selected one.
    pub(super) fn insert_separator(&mut self) {
        let at = self.sel.unwrap_or(0);
        if self.insert_at(at, 0) {
            self.sel = Some(at.min(self.list.len() - 1));
        }
    }

    /// 「折返」: the same with the wrap button.
    pub(super) fn insert_wrap(&mut self) {
        let at = self.sel.unwrap_or(0);
        if self.insert_at(at, WRAP) {
            self.sel = Some(at.min(self.list.len() - 1));
        }
    }

    /// 「削除」: removes the selected row; the one now at its place (or the last) is selected.
    pub(super) fn delete(&mut self) {
        let Some(at) = self.sel.filter(|&i| i < self.list.len()) else {
            return;
        };
        self.list.remove(at);
        self.sel = if self.list.is_empty() {
            None
        } else {
            Some(at.min(self.list.len() - 1))
        };
    }

    /// 「→」: inserts the selected function before the selected row (the top when none is
    /// selected). The row that was selected stays selected, and the function list moves on.
    pub(super) fn insert_function(&mut self, number: i32) {
        let Some(func) = self.func_sel else {
            return;
        };
        let at = self.sel.unwrap_or(0);
        if self.insert_at(at, number) {
            self.sel = Some(at + 1);
            self.func_sel = Some(func + 1).filter(|&f| f < self.func_count);
        }
    }

    /// 「>>」: appends the selected function; it becomes the selected row and the function list
    /// moves on.
    pub(super) fn add_function(&mut self, number: i32) {
        let Some(func) = self.func_sel else {
            return;
        };
        let at = self.list.len();
        if self.insert_at(at, number) {
            self.sel = Some(at);
            self.func_sel = Some(func + 1).filter(|&f| f < self.func_count);
        }
    }

    /// 「↑」.
    pub(super) fn up(&mut self) {
        if let Some(at) = self.sel.filter(|&i| i > 0 && i < self.list.len()) {
            self.list.swap(at, at - 1);
            self.sel = Some(at - 1);
        }
    }

    /// 「↓」.
    pub(super) fn down(&mut self) {
        if let Some(at) = self.sel.filter(|&i| i + 1 < self.list.len()) {
            self.list.swap(at, at + 1);
            self.sel = Some(at + 1);
        }
    }
}

// ---- the file ----

fn load(profile: &Profile) -> Vec<i32> {
    ToolbarButtons::from_profile(profile).0
}

/// `ShareData_IO_Toolbar` writing: the number of buttons and each `nTBB[NNN]`; entries left over
/// from a longer list are removed. Nothing is written when the list is the one the file already
/// gives (or Sakura's default for a file with no list).
fn store(list: &[i32], profile: &mut Profile) -> Result<bool, ProfileError> {
    if load(profile) == list {
        return Ok(false);
    }
    let mut changed = false;
    let mut put = |profile: &mut Profile, key: &str, value: String| -> Result<(), ProfileError> {
        if profile.get(SECTION, key) != Some(value.as_str()) {
            profile.set(SECTION, key, &value)?;
            changed = true;
        }
        Ok(())
    };
    put(profile, "nToolBarButtonNum", list.len().to_string())?;
    for (i, number) in list.iter().enumerate() {
        put(profile, &format!("nTBB[{i:03}]"), number.to_string())?;
    }
    for i in list.len()..MAX_BUTTONS {
        if !profile.remove(SECTION, &format!("nTBB[{i:03}]")) {
            // Entries are written in order, so the first missing one ends the leftovers.
            break;
        }
        changed = true;
    }
    Ok(changed)
}

// ---- the page ----

fn clear_function_choice(values: &mut Values, _: &'static str) {
    values.set("toolbar_func_sel", Value::Int(-1));
}

pub(in crate::common_settings) fn items() -> Vec<Item> {
    vec![
        Item::Hidden(vec![
            Control::choice("toolbar_kind", "種別(&K)", "", 0, &KINDS)
                .on_change(clear_function_choice),
            Control::choice("toolbar_func_sel", "", "", -1, &[]),
            Control::choice("toolbar_res_sel", "", "", 0, &[]),
            Control::numbers("toolbar_buttons", load, store),
            Control::button("toolbar_delete", "削除(&D)", |d, _, cx| {
                d.edit_toolbar(cx, Edit::delete)
            })
            .when(has_selection),
            Control::button("toolbar_separator", "---(&S)", |d, _, cx| {
                d.edit_toolbar(cx, Edit::insert_separator)
            }),
            Control::button("toolbar_insert", "→(&A)", |d, _, cx| {
                d.edit_toolbar_function(cx, Edit::insert_function)
            })
            .when(can_insert),
            Control::button("toolbar_add", ">>(&B)", |d, _, cx| {
                d.edit_toolbar_function(cx, Edit::add_function)
            })
            .when(has_function),
            Control::button("toolbar_up", "↑(&U)", |d, _, cx| {
                d.edit_toolbar(cx, Edit::up)
            })
            .when(can_move_up),
            Control::button("toolbar_down", "↓(&O)", |d, _, cx| {
                d.edit_toolbar(cx, Edit::down)
            })
            .when(can_move_down),
            Control::button("toolbar_wrap", "折返(&W)", |d, _, cx| {
                d.edit_toolbar(cx, Edit::insert_wrap)
            }),
        ]),
        Item::Row(vec![
            Control::check(
                "toolbar_flat",
                "フラットなボタン(&L)",
                "bToolBarIsFlat",
                true,
            )
            .in_section(SECTION)
            .unavailable()
            .into(),
        ]),
        Item::Custom(draw),
    ]
}

/// The state of the lists, from the values (`SetDlgItemsEnableState`).
fn edit_of(values: &Values) -> Edit {
    let list = values.numbers("toolbar_buttons").to_vec();
    let sel = usize::try_from(values.int("toolbar_res_sel"))
        .ok()
        .filter(|&i| i < list.len());
    let functions = functions_of_kind(values.int("toolbar_kind"));
    let func_sel = usize::try_from(values.int("toolbar_func_sel"))
        .ok()
        .filter(|&i| i < functions.len());
    Edit {
        list,
        sel,
        func_sel,
        func_count: functions.len(),
    }
}

fn has_selection(values: &Values) -> bool {
    edit_of(values).sel.is_some()
}

fn has_function(values: &Values) -> bool {
    edit_of(values).func_sel.is_some()
}

fn can_insert(values: &Values) -> bool {
    let edit = edit_of(values);
    edit.sel.is_some() && edit.func_sel.is_some()
}

fn can_move_up(values: &Values) -> bool {
    edit_of(values).sel.is_some_and(|i| i > 0)
}

fn can_move_down(values: &Values) -> bool {
    let edit = edit_of(values);
    edit.sel.is_some_and(|i| i + 1 < edit.list.len())
}

impl CommonSettingsDialog {
    /// Applies one of the edits of the toolbar list to the values.
    fn edit_toolbar(&mut self, cx: &mut Context<Self>, change: fn(&mut Edit)) {
        let mut edit = edit_of(self.values());
        change(&mut edit);
        self.put_toolbar_edit(edit, cx);
    }

    /// The same for the edits that take the selected function.
    fn edit_toolbar_function(&mut self, cx: &mut Context<Self>, change: fn(&mut Edit, i32)) {
        let mut edit = edit_of(self.values());
        let functions = functions_of_kind(self.values().int("toolbar_kind"));
        let Some(&(number, _)) = edit.func_sel.and_then(|i| functions.get(i)) else {
            return;
        };
        change(&mut edit, number);
        self.put_toolbar_edit(edit, cx);
    }

    fn put_toolbar_edit(&mut self, edit: Edit, cx: &mut Context<Self>) {
        let to_int = |sel: Option<usize>| sel.map_or(-1, |i| i as i32);
        self.set_numbers("toolbar_buttons", edit.list, cx);
        self.set_int("toolbar_res_sel", to_int(edit.sel), cx);
        self.set_int("toolbar_func_sel", to_int(edit.func_sel), cx);
    }
}

const SELECTED_BG: u32 = 0x2f6fd0;
const LIST_BORDER: u32 = 0xaaaaaa;
const ROW_HEIGHT: f32 = 24.0;

/// One row of a list: the icon and the name, highlighted when selected.
fn row_element(
    id: (&'static str, usize),
    number: i32,
    macros: &RegisteredMacros,
    selected: bool,
    scale: f32,
    cx: &mut Context<CommonSettingsDialog>,
    on_click: impl Fn(&mut CommonSettingsDialog, &mut Context<CommonSettingsDialog>) + 'static,
) -> AnyElement {
    let (name, icon) = row_of(number, macros);
    let side = icons::logical_size(scale).max(icons::ICON_SIZE as f32);
    let picture = icon.and_then(|i| icons::icon(i, icons::pixel_factor(scale)));
    h_flex()
        .id(id)
        .h(px(ROW_HEIGHT))
        .px_1()
        .gap_1()
        .items_center()
        .when(selected, |this| {
            this.bg(rgb(SELECTED_BG)).text_color(rgb(0xffffff))
        })
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(
            div()
                .w(px(side))
                .h(px(side))
                .flex_shrink_0()
                .when_some(picture, |this, picture| {
                    this.child(
                        img(picture)
                            .w(px(icons::logical_size(scale)))
                            .h(px(icons::logical_size(scale))),
                    )
                }),
        )
        .child(name)
        .into_any_element()
}

fn list_box(id: &'static str, rows: Vec<AnyElement>) -> AnyElement {
    v_flex()
        .id(id)
        .debug_selector(|| id.into())
        .w(px(250.))
        .h(px(380.))
        .border_1()
        .border_color(rgb(LIST_BORDER))
        .overflow_y_scroll()
        .children(rows)
        .into_any_element()
}

fn draw(dialog: &CommonSettingsDialog, cx: &mut Context<CommonSettingsDialog>) -> AnyElement {
    let values = dialog.values();
    let scale = dialog.scale_factor();
    let edit = edit_of(values);
    let kind = values.int("toolbar_kind");
    let kind_name = KINDS
        .iter()
        .find(|(_, v)| *v == kind)
        .map_or("", |(name, _)| *name);
    let functions = functions_of_kind(kind);
    let macros = crate::macros::registered_table(cx);

    let function_rows = functions
        .iter()
        .enumerate()
        .map(|(i, &(number, _))| {
            row_element(
                ("toolbar-func", i),
                number,
                &macros,
                edit.func_sel == Some(i),
                scale,
                cx,
                move |this, cx| this.set_int("toolbar_func_sel", i as i32, cx),
            )
        })
        .collect();
    let button_rows = edit
        .list
        .iter()
        .enumerate()
        .map(|(i, &number)| {
            row_element(
                ("toolbar-res", i),
                number,
                &macros,
                edit.sel == Some(i),
                scale,
                cx,
                move |this, cx| this.set_int("toolbar_res_sel", i as i32, cx),
            )
        })
        .collect();

    let weak = cx.entity().downgrade();
    let kind_button =
        Button::new("toolbar-kind")
            .label(kind_name)
            .dropdown_menu(move |menu, _, _| {
                KINDS.iter().fold(menu, |menu, &(name, value)| {
                    let weak = weak.clone();
                    menu.item(PopupMenuItem::new(name).on_click(move |_, _, cx| {
                        weak.update(cx, |this, cx| this.set_int("toolbar_kind", value, cx))
                            .ok();
                    }))
                })
            });

    let controls = dialog.page_controls();
    let button = |id: &'static str, cx: &mut Context<CommonSettingsDialog>| -> AnyElement {
        let Some(control) = controls.iter().find(|c| c.id == id).copied() else {
            return div().into_any_element();
        };
        let (label, _) = super::super::model::parse_label(control.label);
        let usable = dialog.is_usable(&control);
        let super::super::model::Kind::Button { run } = control.kind else {
            return div().into_any_element();
        };
        Button::new(id)
            .label(label)
            .disabled(!usable)
            .on_click(cx.listener(move |this, _, window: &mut Window, cx| run(this, window, cx)))
            .into_any_element()
    };
    let buttons = v_flex()
        .gap_2()
        .pt(px(30.))
        .child(button("toolbar_delete", cx))
        .child(button("toolbar_separator", cx))
        .child(button("toolbar_insert", cx))
        .child(button("toolbar_add", cx))
        .child(button("toolbar_up", cx))
        .child(button("toolbar_down", cx))
        .child(button("toolbar_wrap", cx));

    h_flex()
        .gap_2()
        .items_start()
        .child(
            v_flex()
                .gap_1()
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child("種別(K)")
                        .child(kind_button),
                )
                .child("機能(F)")
                .child(list_box("toolbar-functions", function_rows)),
        )
        .child(buttons)
        .child(
            v_flex()
                .gap_1()
                .child("ツールバー(T)")
                .child(list_box("toolbar-buttons", button_rows)),
        )
        .into_any_element()
}

/// What OK does to the windows: every window's toolbar takes the list. At startup the toolbar
/// took the file as it is (`crate::bars::install`).
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, before: Option<&Values>) {
    if before.is_none() {
        return;
    }
    let list = after.numbers("toolbar_buttons").to_vec();
    let now = env
        .cx
        .try_global::<ToolbarButtons>()
        .cloned()
        .unwrap_or_default();
    if now.0 != list {
        env.cx.set_global(ToolbarButtons(list));
    }
}

#[cfg(test)]
mod tests;
