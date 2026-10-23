//! 「キー割り当て」: `IDD_PROP_KEYBIND` (`sakura_core/sakura_rc.rc`), `CPropKeybind`
//! (`sakura_core/prop/CPropComKeybind.cpp`).
//!
//! The page has the kind (種別) and the functions of the kind, the keys with the check boxes of
//! Shift, Ctrl and Alt, 割当 and 解除, the keys the selected function is assigned to, and the
//! function of the selected key. OK writes `[KeyBind]` (`CShareData_IO::ShareData_IO_KeyBind`)
//! under the lock of the settings file, and every window's key map and the shortcuts in the menus
//! change at once.
//!
//! The table is a [`Kind::Numbers`] control (nine numbers for each key, see [`bindings`]) and the
//! page draws itself ([`Item::Custom`], in [`draw`]). The choices of the lists and the check
//! boxes are controls the dialog does not draw ([`Item::Hidden`]), so that the Alt keys work.
//!
//! On macOS the page edits Sakura's table, as on every OS; MATSU Editor's macOS rules are put over
//! the user's rows when the key map is made (`UserSettings::key_map`).
//!
//! インポート and エクスポート read and write `.key` files ([`transfer`]).
//!
//! Not built: the access keys of the lists (`(&F)`, `(&Y)`, `(&G)`); the functions of the kind
//! プラグイン (there are no plugins).

mod bindings;
mod draw;
mod transfer;

use matsu_commands::keymap::Platform;

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::func_lookup::{KINDS, codes_of_kind};
use super::super::model::{Control, Item, Value, Values};
use crate::settings::UserSettings;
use bindings::{ALT, Assigned, Bindings, CTRL, SHIFT};

/// `STR_PROPCOMKEYBIND_UNASSIGN`: what the function of a key is when it has none.
pub(super) const UNASSIGNED: &str = "未割当";

pub(super) const KIND: &str = "keybind_kind";
pub(super) const FUNC_SEL: &str = "keybind_func_sel";
pub(super) const KEY_SEL: &str = "keybind_key_sel";
pub(super) const SHIFT_ID: &str = "keybind_shift";
pub(super) const CTRL_ID: &str = "keybind_ctrl";
pub(super) const ALT_ID: &str = "keybind_alt";
/// The function whose keys the list 「機能に割り当てられているキー」 shows (0: none).
pub(super) const SHOWN_FUNC: &str = "keybind_shown_func";
pub(super) const ROWS: &str = "keybind_rows";

/// The table of the values.
fn bindings_of(values: &Values) -> Bindings {
    Bindings::from_numbers(values.numbers(ROWS))
}

/// The index of the modifiers the check boxes say (`_SHIFT | _CTRL | _ALT`).
fn mods_of(values: &Values) -> usize {
    [(SHIFT_ID, SHIFT), (CTRL_ID, CTRL), (ALT_ID, ALT)]
        .into_iter()
        .filter(|(id, _)| values.bool(id))
        .fold(0, |mods, (_, bit)| mods | bit)
}

/// The selected row of a list, if there is one among `count`.
fn selected(values: &Values, id: &str, count: usize) -> Option<usize> {
    usize::try_from(values.int(id)).ok().filter(|&i| i < count)
}

/// The key selected in the key list.
fn key_of(values: &Values) -> Option<usize> {
    selected(values, KEY_SEL, bindings_of(values).rows.len())
}

/// The function selected in the function list (`Pos2FuncCode`).
pub(super) fn function_of(values: &Values) -> Option<u16> {
    let codes = codes_of_kind(values.int(KIND));
    selected(values, FUNC_SEL, codes.len()).map(|i| codes[i])
}

/// `LBN_SELCHANGE` of the function list: the list of the keys shows the keys of the function that
/// is selected now (none when nothing is selected).
fn show_keys_of_selection(values: &mut Values) {
    let code = function_of(values).unwrap_or(0);
    values.set(SHOWN_FUNC, Value::Int(i32::from(code)));
}

/// Choosing another kind empties the function list; the list of keys is not touched
/// (`CBN_SELCHANGE` only fills the list box again).
fn kind_changed(values: &mut Values, _: &'static str) {
    values.set(FUNC_SEL, Value::Int(-1));
}

fn function_changed(values: &mut Values, _: &'static str) {
    show_keys_of_selection(values);
}

/// 「割当」 can be used while a key and a function are selected (`WM_TIMER`).
fn can_assign(values: &Values) -> bool {
    key_of(values).is_some() && function_of(values).is_some()
}

/// 「解除」 can be used while a key is selected.
fn can_release(values: &Values) -> bool {
    key_of(values).is_some()
}

pub(in crate::common_settings) fn items() -> Option<Vec<Item>> {
    Some(vec![
        Item::Hidden(vec![
            Control::choice(KIND, "種別(&K)", "", 0, &KINDS).on_change(kind_changed),
            Control::choice(FUNC_SEL, "", "", -1, &[]).on_change(function_changed),
            // `SetData` selects the first key.
            Control::choice(KEY_SEL, "", "", 0, &[]),
            Control::choice(SHOWN_FUNC, "", "", 0, &[]),
            Control::choice("keybind_shown_sel", "", "", -1, &[]),
            Control::check(SHIFT_ID, "&Shift", "", false),
            Control::check(CTRL_ID, "&Ctrl", "", false),
            Control::check(ALT_ID, "A&Lt", "", false),
            Control::numbers(ROWS, bindings::load, bindings::store),
            Control::button("keybind_assign", "割当(&B)", |d, _, cx| d.assign_key(cx))
                .when(can_assign),
            Control::button("keybind_release", "解除(&R)", |d, _, cx| {
                d.release_key(cx)
            })
            .when(can_release),
            Control::button(
                "keybind_import",
                "インポート(&I)...",
                |d, window, cx| d.import_keys(window, cx),
            ),
            Control::button(
                "keybind_export",
                "エクスポート(&X)...",
                |d, window, cx| d.export_keys(window, cx),
            ),
        ]),
        Item::Custom(draw::draw),
    ])
}

impl CommonSettingsDialog {
    /// 「割当」: the selected function goes to the selected key with the modifiers that are
    /// checked; then the keys of the function are listed again.
    fn assign_key(&mut self, cx: &mut gpui::Context<Self>) {
        let values = self.values();
        let (Some(key), Some(code)) = (key_of(values), function_of(values)) else {
            return;
        };
        let mods = mods_of(values);
        let mut table = bindings_of(values);
        if table.set_function(key, mods, code) {
            self.set_numbers(ROWS, table.to_numbers(), cx);
        }
        self.show_keys_of_function(cx);
    }

    /// 「解除」: the function of the selected key with the checked modifiers is `F_DEFAULT`
    /// (none). The keys listed are those of the function selected now, so the list is empty when
    /// no function is selected.
    fn release_key(&mut self, cx: &mut gpui::Context<Self>) {
        let values = self.values();
        let Some(key) = key_of(values) else {
            return;
        };
        let mods = mods_of(values);
        let mut table = bindings_of(values);
        if table.set_function(key, mods, 0) {
            self.set_numbers(ROWS, table.to_numbers(), cx);
        }
        self.show_keys_of_function(cx);
    }

    fn show_keys_of_function(&mut self, cx: &mut gpui::Context<Self>) {
        let code = function_of(self.values()).unwrap_or(0);
        self.set_int(SHOWN_FUNC, i32::from(code), cx);
        self.set_int("keybind_shown_sel", -1, cx);
    }

    /// A click in the function list.
    fn select_function(&mut self, row: usize, cx: &mut gpui::Context<Self>) {
        self.set_int(FUNC_SEL, row as i32, cx);
        self.set_int("keybind_shown_sel", -1, cx);
    }

    /// A click in the key list.
    fn select_key(&mut self, row: usize, cx: &mut gpui::Context<Self>) {
        self.set_int(KEY_SEL, row as i32, cx);
    }

    /// A click in the list of the keys of the function: the key list goes to that key, and the
    /// check boxes to its modifiers.
    fn select_assigned(&mut self, row: usize, found: &Assigned, cx: &mut gpui::Context<Self>) {
        self.set_int("keybind_shown_sel", row as i32, cx);
        self.set_int(KEY_SEL, found.key as i32, cx);
        self.set_bool(SHIFT_ID, found.mods & SHIFT != 0, cx);
        self.set_bool(CTRL_ID, found.mods & CTRL != 0, cx);
        self.set_bool(ALT_ID, found.mods & ALT != 0, cx);
    }
}

/// What OK does to the windows: every window's key map and the menus' shortcuts take the table.
/// At startup the windows took the file as it is (`UserSettings::key_map`).
pub(in crate::common_settings) fn apply(env: &mut Env, after: &Values, before: Option<&Values>) {
    let Some(before) = before else {
        return;
    };
    if after.numbers(ROWS) == before.numbers(ROWS) {
        return;
    }
    // The settings the dialog has just written are the application's now.
    let Some(user) = env.cx.try_global::<UserSettings>().cloned() else {
        return;
    };
    let map = user.key_map(Platform::current());
    crate::app::set_key_map(env.cx, map.clone());
    for (_, workspace) in crate::file_ops::windows::workspaces(env.cx) {
        for view in workspace.read(env.cx).all_views() {
            let map = map.clone();
            view.update(env.cx, |view, cx| {
                view.keymap = map;
                cx.notify();
            });
        }
    }
}

#[cfg(test)]
mod tests;
