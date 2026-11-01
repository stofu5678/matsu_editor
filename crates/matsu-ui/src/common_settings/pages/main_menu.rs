//! 「メインメニュー」: `IDD_PROP_MAINMENU` (`sakura_core/sakura_rc.rc`), `CPropMainMenu`
//! (`sakura_core/prop/CPropComMainMenu.cpp`).
//!
//! The page edits the main menu as a tree (メニュー): the functions of a kind (機能, chosen by 種別)
//! are put in with 上⇒, 下⇒ and >>, a node with ＋ and a line with ---; 削除, ↑, ↓, → and ← change
//! the selected row ([`tree`]), 全開 and 全閉 open and close the nodes. The access key of a row and
//! the name of a node are typed in a box and set with 設定 (Sakura sets the key with a key press or
//! a double click, and the name with F2 in the tree). 検査 looks for the problems of the menu,
//! クリア empties it, 初期設定 brings back Sakura's menu, and インポート and エクスポート read and
//! write a menu file ([`transfer`]).
//!
//! OK writes `[MainMenu]` (`CShareData_IO::IO_MainMenu`) under the lock of the settings file and the
//! menus of every window change at once: the menu bar of the window, and the menus of macOS, are
//! made again from the settings of the application, which OK replaces. Nothing is written when the
//! menu is the one the file already gives. The menu is one [`Kind::Numbers`] control ([`data`]); the
//! page draws the lists itself ([`Item::Custom`]).
//!
//! Differences from Sakura: no row is selected when the window opens; the access key and the name are
//! set with a box and a button; the file dialogs have no file type filter.
//!
//! A function row whose function cannot be read (a plug-in command, a macro name that does not exist)
//! reads as function 0, and OK turns it into an empty node, as `GetDataTree` does with `F_NODE`.
//!
//! Not built: the plug-in commands (the kind プラグイン has none), the registered macros (the kind
//! 外部マクロ lists every slot). A function or a separator on the menu bar itself (level 0) is kept in
//! the file but not shown, because the menu bar here holds menus only. The access keys of the custom
//! menus and the other menus that Sakura builds with `MakeMenuLabel` do not follow the setting
//! 「アクセスキーを必ず( )付で表示」 (only the main menu does).

mod data;
mod draw;
mod transfer;
mod tree;

use gpui::{App, Context, PromptLevel, Window};
use matsu_config::settings::RowKind;
use matsu_config::settings::main_menu::{F_NODE, MAX_MAIN_MENU_NAME_LEN};

use super::super::Env;
use super::super::dialog::CommonSettingsDialog;
use super::super::func_lookup::{KINDS, codes_of_kind};
use super::super::model::{Control, Item, Value, Values};
use crate::func_name::FuncNames;
use data::State;
use tree::{PageRow, Place, Tree};

const DATA: &str = "mainmenu_data";
const KIND: &str = "mainmenu_kind";
const FUNC_SEL: &str = "mainmenu_func_sel";
const RES_SEL: &str = "mainmenu_res_sel";
const NAME: &str = "mainmenu_name";
const KEY: &str = "mainmenu_key";

/// `STR_SPECIAL_FUNC`: the kind that `CPropMainMenu::SetData` adds after the kinds of `CFuncLookup`.
const SPECIAL_KIND: i32 = 18;
/// The kinds the page offers: those of `CFuncLookup` and 特別機能.
static PAGE_KINDS: [(&str, i32); 19] = {
    let mut kinds = [("", 0); 19];
    let mut i = 0;
    while i < KINDS.len() {
        kinds[i] = KINDS[i];
        i += 1;
    }
    kinds[SPECIAL_KIND as usize] = ("特別機能", SPECIAL_KIND);
    kinds
};
/// The special function list of `sakura_core/func/Funccode.cpp` (`F_WINDOW_LIST` to `F_PLUGIN_LIST`).
const SPECIAL_CODES: [u16; 6] = [29001, 29002, 29003, 29004, 29005, 29006];
/// What a row shows for a command that is not known: Sakura shows nothing useful for it (the function is
/// `F_DEFAULT`), so the row says what it is, with the text the file has. (MATSU's own wording.)
const UNKNOWN_LABEL: &str = "（不明）";
/// `STR_PROPCOMMAINMENU_SEP`.
const SEPARATOR_TEXT: &str = "――――――――――";

/// The functions of kind `kind` in the order of the list (`CFuncLookup::Pos2FuncCode`, and
/// the special function list for 特別機能).
fn functions_of_kind(kind: i32) -> Vec<u16> {
    if kind == SPECIAL_KIND {
        SPECIAL_CODES.to_vec()
    } else {
        codes_of_kind(kind)
    }
}

/// `CFuncLookup::Funccode2Name`: the names of the functions, from the lookup that the other pages
/// share ([`crate::func_name`]): a registered macro by its name, a custom menu by its own.
fn function_names(cx: &App) -> impl Fn(u16) -> String + '_ {
    let names = FuncNames::new();
    move |code| names.name(cx, code)
}

/// The name of a row (`SetData`): a node with its own name shows it, a separator a line, the other rows the
/// name of the function (`name_of`, [`function_names`]).
fn row_name(row: &PageRow, name_of: &dyn Fn(u16) -> String) -> String {
    let row = &row.row;
    match row.kind {
        RowKind::Separator => SEPARATOR_TEXT.to_owned(),
        RowKind::Leaf if row.is_unknown() => {
            format!("{UNKNOWN_LABEL}{}", row.text.as_deref().unwrap_or_default())
        }
        RowKind::Node if row.function == F_NODE => row.name.clone(),
        _ => name_of(row.function),
    }
}

/// The text of a row of the tree (`MakeDispLabel`): `>` for a key that another row has, the name cut to
/// the limit, and the access key in parentheses.
fn row_text(row: &PageRow, name_of: &dyn Fn(u16) -> String) -> String {
    let name: String = {
        let mut units = 0;
        row_name(row, name_of)
            .chars()
            .take_while(|c| {
                units += c.len_utf16();
                units <= MAX_MAIN_MENU_NAME_LEN
            })
            .collect()
    };
    let mark = if row.duplicate { ">" } else { "" };
    match row.row.key {
        Some(key) => format!("{mark}{name}({key})"),
        None => format!("{mark}{name}"),
    }
}

// ---- the state of the page, from the values ----

fn state_of(values: &Values) -> State {
    State::decode(values.numbers(DATA))
}

/// The tree and the selections the page holds now (`SetDlgItemsEnableState` looks at them).
fn tree_of(values: &Values) -> Tree {
    let rows = state_of(values).rows;
    let sel = usize::try_from(values.int(RES_SEL))
        .ok()
        .filter(|&i| i < rows.len());
    let functions = functions_of_kind(values.int(KIND));
    let func_sel = usize::try_from(values.int(FUNC_SEL))
        .ok()
        .filter(|&i| i < functions.len());
    Tree {
        rows,
        sel,
        func_sel,
        func_count: functions.len(),
    }
}

fn has_selection(values: &Values) -> bool {
    tree_of(values).sel.is_some()
}

fn can_move_up(values: &Values) -> bool {
    let tree = tree_of(values);
    tree.sel.is_some_and(|i| tree.prev_sibling(i).is_some())
}

fn can_move_down(values: &Values) -> bool {
    let tree = tree_of(values);
    tree.sel.is_some_and(|i| tree.next_sibling(i).is_some())
}

/// 「→」 is usable when a row of the level is before the selected row (it does nothing unless that row is a
/// node, as in Sakura).
fn can_move_right(values: &Values) -> bool {
    can_move_up(values)
}

fn can_move_left(values: &Values) -> bool {
    let tree = tree_of(values);
    tree.sel.is_some_and(|i| tree.parent(i).is_some())
}

fn can_insert(values: &Values) -> bool {
    let tree = tree_of(values);
    tree.sel.is_some() && tree.func_sel.is_some()
}

fn can_add(values: &Values) -> bool {
    tree_of(values).func_sel.is_some()
}

fn can_set_key(values: &Values) -> bool {
    let tree = tree_of(values);
    tree.sel
        .and_then(|i| tree.rows.get(i))
        .is_some_and(|r| r.row.kind != RowKind::Separator)
}

fn can_set_name(values: &Values) -> bool {
    let tree = tree_of(values);
    tree.sel.is_some_and(|i| tree.is_node(i))
}

fn clear_function_choice(values: &mut Values, _: &'static str) {
    values.set(FUNC_SEL, Value::Int(-1));
}

pub(in crate::common_settings) fn items() -> Option<Vec<Item>> {
    Some(vec![
        Item::Custom(draw::choosers),
        Item::Custom(draw::lists),
        Item::Row(vec![
            Control::text(NAME, "ノード名(&V)", "", "").into(),
            Control::button("mainmenu_set_name", "設定(&J)", |d, window, cx| {
                d.main_menu_set_name(window, cx)
            })
            .when(can_set_name)
            .into(),
            Control::text(KEY, "アクセスキー(&Y)", "", "").into(),
            Control::button("mainmenu_set_key", "設定(&G)", |d, window, cx| {
                d.main_menu_set_key(window, cx)
            })
            .when(can_set_key)
            .into(),
        ]),
        Item::Row(vec![
            Control::button(
                "mainmenu_import",
                "インポート(&I)...",
                |d, window, cx| d.main_menu_import(window, cx),
            )
            .into(),
            Control::button(
                "mainmenu_export",
                "エクスポート(&X)...",
                |d, window, cx| d.main_menu_export(window, cx),
            )
            .into(),
            Control::button("mainmenu_check", "検査(&T)", |d, window, cx| {
                d.main_menu_check(window, cx)
            })
            .into(),
            Control::button("mainmenu_clear", "クリア(&C)", |d, window, cx| {
                d.main_menu_confirm(
                    "メニューの設定をクリアします。\nよろしいですか？",
                    window,
                    cx,
                    CommonSettingsDialog::main_menu_clear,
                )
            })
            .into(),
            Control::button(
                "mainmenu_initialize",
                "初期設定(&E)",
                |d, window, cx| {
                    d.main_menu_confirm(
                        "メニューの設定を初期状態に戻します。\nよろしいですか？",
                        window,
                        cx,
                        CommonSettingsDialog::main_menu_initialize,
                    )
                },
            )
            .into(),
        ]),
        Item::Hidden(vec![
            Control::choice(KIND, "種別(&K)", "", 0, &PAGE_KINDS).on_change(clear_function_choice),
            Control::choice(FUNC_SEL, "", "", -1, &[]),
            Control::choice(RES_SEL, "", "", -1, &[]),
            Control::numbers(DATA, data::load, data::store),
            Control::button(
                "mainmenu_parentheses",
                "アクセスキーを必ず( )付で表示(&P)",
                |d, _, cx| {
                    let on = !state_of(d.values()).key_parentheses;
                    d.main_menu_set_parentheses(on, cx);
                },
            ),
            Control::button("mainmenu_delete", "削除(&D)", |d, window, cx| {
                d.main_menu_delete(window, cx)
            })
            .when(has_selection),
            Control::button("mainmenu_node", "＋(&N)", |d, window, cx| {
                d.main_menu_edit(window, cx, Tree::insert_node)
            }),
            Control::button("mainmenu_separator", "---(&S)", |d, window, cx| {
                d.main_menu_edit(window, cx, Tree::insert_separator)
            }),
            Control::button("mainmenu_above", "上⇒(&Q)", |d, window, cx| {
                d.main_menu_insert_function(Place::Above, window, cx)
            })
            .when(can_insert),
            Control::button("mainmenu_below", "下⇒(&A)", |d, window, cx| {
                d.main_menu_insert_function(Place::Below, window, cx)
            })
            .when(can_insert),
            Control::button("mainmenu_add", ">>(&B)", |d, window, cx| {
                d.main_menu_insert_function(Place::Add, window, cx)
            })
            .when(can_add),
            Control::button("mainmenu_up", "↑(&U)", |d, window, cx| {
                d.main_menu_edit(window, cx, Tree::up)
            })
            .when(can_move_up),
            Control::button("mainmenu_down", "↓(&O)", |d, window, cx| {
                d.main_menu_edit(window, cx, Tree::down)
            })
            .when(can_move_down),
            Control::button("mainmenu_right", "→(&R)", |d, window, cx| {
                d.main_menu_edit(window, cx, Tree::right)
            })
            .when(can_move_right),
            Control::button("mainmenu_left", "←(&L)", |d, window, cx| {
                d.main_menu_edit(window, cx, Tree::left)
            })
            .when(can_move_left),
            Control::button("mainmenu_expand", "全開(&H)", |d, window, cx| {
                d.main_menu_edit(window, cx, |tree| tree.expand_all(true))
            }),
            Control::button("mainmenu_collapse", "全閉(&Z)", |d, window, cx| {
                d.main_menu_edit(window, cx, |tree| tree.expand_all(false))
            }),
        ]),
    ])
}

impl CommonSettingsDialog {
    /// Puts `state` in the values.
    fn main_menu_put(&mut self, state: &State, cx: &mut Context<Self>) {
        self.set_numbers(DATA, state.encode(), cx);
    }

    /// The edit boxes show what the selected row has.
    fn main_menu_show_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tree = tree_of(self.values());
        let row = tree.sel.and_then(|i| tree.rows.get(i));
        let key = row
            .and_then(|r| r.row.key)
            .map(String::from)
            .unwrap_or_default();
        let name = {
            let name_of = function_names(cx);
            row.filter(|r| r.row.kind == RowKind::Node)
                .map(|r| row_name(r, &name_of))
                .unwrap_or_default()
        };
        self.set_input(KEY, &key, window, cx);
        self.set_input(NAME, &name, window, cx);
    }

    /// Takes the tree as it is after an edit.
    fn main_menu_put_tree(&mut self, tree: Tree, window: &mut Window, cx: &mut Context<Self>) {
        let to_int = |sel: Option<usize>| sel.map_or(-1, |i| i as i32);
        let mut state = state_of(self.values());
        state.rows = tree.rows;
        self.main_menu_put(&state, cx);
        self.set_int(RES_SEL, to_int(tree.sel), cx);
        self.set_int(FUNC_SEL, to_int(tree.func_sel), cx);
        self.main_menu_show_selected(window, cx);
    }

    /// Applies one of the edits of the tree.
    fn main_menu_edit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        change: impl FnOnce(&mut Tree),
    ) {
        let mut tree = tree_of(self.values());
        change(&mut tree);
        self.main_menu_put_tree(tree, window, cx);
    }

    /// 「上⇒」, 「下⇒」 and 「>>」: the function chosen in the list goes into the tree.
    fn main_menu_insert_function(
        &mut self,
        place: Place,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let functions = functions_of_kind(self.values().int(KIND));
        let mut tree = tree_of(self.values());
        let Some(&code) = tree.func_sel.and_then(|i| functions.get(i)) else {
            return;
        };
        tree.insert_function(place, code);
        self.main_menu_put_tree(tree, window, cx);
    }

    /// A click on a row of the tree.
    pub(super) fn main_menu_select(
        &mut self,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.main_menu_edit(window, cx, |tree| tree.select(row));
    }

    /// A click on the button of a node.
    pub(super) fn main_menu_toggle(
        &mut self,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.main_menu_edit(window, cx, |tree| tree.toggle(row));
    }

    /// 「削除」: removes the selected row; asks first when it has rows under it
    /// (`STR_PROPCOMMAINMENU_DEL`).
    fn main_menu_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let tree = tree_of(self.values());
        let Some(sel) = tree.sel else { return };
        if tree.has_children(sel) {
            self.main_menu_confirm(
                "選択している項目を下位項目ごと削除します。\nよろしいですか？",
                window,
                cx,
                |d, window, cx| d.main_menu_edit(window, cx, Tree::delete),
            );
        } else {
            self.main_menu_edit(window, cx, Tree::delete);
        }
    }

    /// Asks (OK / キャンセル) and, when it is OK, runs `action`.
    fn main_menu_confirm(
        &mut self,
        question: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
        action: fn(&mut Self, &mut Window, &mut Context<Self>),
    ) {
        let answer = crate::boxes::prompt(
            window,
            PromptLevel::Info,
            question,
            None,
            &["OK", "キャンセル"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if let Ok(0) = answer.await {
                this.update_in(cx, action).ok();
            }
        })
        .detach();
    }

    /// 「クリア」: the tree is empty.
    fn main_menu_clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut state = state_of(self.values());
        state.rows.clear();
        self.main_menu_put(&state, cx);
        self.set_int(RES_SEL, -1, cx);
        self.main_menu_show_selected(window, cx);
    }

    /// 「初期設定」: Sakura's menu (`MainMenu.ini`), with its setting of the access keys.
    fn main_menu_initialize(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = State::of_menu(crate::menu::default_main_menu());
        self.main_menu_put(&state, cx);
        self.set_int(RES_SEL, -1, cx);
        self.main_menu_show_selected(window, cx);
    }

    /// 「検査」: looks for the problems and says what it found; the first row with a problem is selected.
    fn main_menu_check(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut tree = tree_of(self.values());
        let (message, first_error) = tree.check();
        if let Some(row) = first_error {
            tree.select(row);
        }
        self.main_menu_put_tree(tree, window, cx);
        let text = if message.is_empty() {
            "問題ありませんでした。"
        } else {
            message.as_str()
        };
        // Nobody waits for this answer.
        crate::boxes::tell(window, PromptLevel::Info, text, None, cx);
    }

    /// 「設定」 beside the name (`TVN_ENDLABELEDIT`): the box becomes the name of the selected node.
    fn main_menu_set_name(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.input_text(NAME, cx) else {
            return;
        };
        let mut tree = tree_of(self.values());
        let shown = {
            let name_of = function_names(cx);
            tree.sel
                .and_then(|i| tree.rows.get(i))
                .map(|r| row_name(r, &name_of))
                .unwrap_or_default()
        };
        tree.set_name(&text, &shown);
        self.main_menu_put_tree(tree, window, cx);
    }

    /// 「設定」 beside the access key (`WM_KEYDOWN` and `NM_DBLCLK` of the tree): the first character of the
    /// box is the key of the selected row; an empty box clears it. A space is no key.
    fn main_menu_set_key(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = self.input_text(KEY, cx) else {
            return;
        };
        let key = text.chars().next().filter(|&c| c != ' ' && !c.is_control());
        self.main_menu_edit(window, cx, |tree| tree.set_key(key));
    }

    /// 「アクセスキーを必ず( )付で表示」 (`IDC_CHECK_KEY_PARENTHESES`).
    pub(super) fn main_menu_set_parentheses(&mut self, on: bool, cx: &mut Context<Self>) {
        let mut state = state_of(self.values());
        state.key_parentheses = on;
        self.main_menu_put(&state, cx);
    }
}

/// What OK does to the windows: nothing of its own. OK writes `[MainMenu]` and replaces the
/// application's settings, and `commit` makes the menus again from them.
pub(in crate::common_settings) fn apply(_env: &mut Env, _after: &Values, _before: Option<&Values>) {
}

#[cfg(test)]
mod tests;
