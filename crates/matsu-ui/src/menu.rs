//! Sakura's main menu as a model, built from the table of `matsu_commands::sakura::MAIN_MENU`.
//!
//! The model knows nothing of GPUI: labels with access keys, separators, submenus, the shortcut of
//! each item from the key map, and whether the item can run. [`crate::app`] turns it into GPUI
//! menus. Tests of the menu are tests of this model.
//!
//! Sakura: `CEditWnd::LayoutMainMenu` / `InitMenu` (`sakura_core/window/CEditWnd.cpp`) build the
//! menu from the table; `CKeyBind::MakeMenuLabel` (`sakura_core/func/CKeyBind.cpp`) puts the access
//! key in the label; `CKeyBind::GetMenuLabel` adds the shortcut after a tab.

use std::sync::OnceLock;

use matsu_commands::keymap::{Chord, DisplayStyle, KeyMap};
use matsu_commands::sakura::{FUNCTIONS, KEY_PARENTHESES, MAIN_MENU, MenuKind};
use matsu_config::settings::main_menu::{F_NODE, MAX_MAINMENU_TOP};
use matsu_config::settings::{CustomMenus, MainMenu, MenuRow, RowKind};

/// `F_EXITALL`, "マツエディタの全終了" (Sakura: "サクラエディタの全終了"). Until a command implements
/// it, the item runs the `Exit` action of the application (which the first window already had).
pub const F_EXITALL: u16 = 30195;

/// What an item does when it is chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuAction {
    /// Run the Sakura function with this code on the editor.
    Function(u16),
    /// Open this recent file (`IDM_SELMRU + index`; the path is what the item showed).
    RecentFile(String),
    /// Open the open dialog in this recent folder (`IDM_SELOPENFOLDER + index`; the path is what
    /// the item showed).
    RecentFolder(String),
    /// Quit the application.
    Exit,
    /// Nothing: a list that Sakura fills in at run time (recent files, windows, ...), which is
    /// empty here.
    Placeholder,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The label as shown, with the access key as `(K)` and without the shortcut.
    pub label: String,
    pub action: MenuAction,
    /// The first key that runs the function in the current key map.
    pub shortcut: Option<Chord>,
    /// A function without a command (neither core nor UI) is shown, as in Sakura, but grayed out.
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Item(Item),
    Separator,
    Submenu { label: String, entries: Vec<Entry> },
}

/// One menu of the menu bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopMenu {
    pub label: String,
    pub entries: Vec<Entry>,
}

/// The label Sakura shows for `name` with the access key `key` (`CKeyBind::MakeMenuLabel` with
/// the "key in parentheses" setting on). See [`menu_label_with`].
pub fn menu_label(name: &str, key: Option<char>) -> String {
    menu_label_with(name, key, true)
}

/// The label Sakura shows for `name` with the access key `key` (`CKeyBind::MakeMenuLabel`).
/// `parentheses` is the setting `bKeyParentheses` of the main menu. A GPUI menu has no mnemonic
/// mark, so the `&` that Sakura puts in front of the key is left out.
///
/// * Without `parentheses`, a name that has the key (or its lower case) as a letter already shows
///   it: the label is the name.
/// * If the name has a `(` followed by the key, the name already says it: `開く(O)` stays.
/// * A name that ends in `...` gets the key before the dots: `開く...` becomes `開く(O)...`.
/// * Otherwise `(K)` goes at the end.
///
/// `&&` in the name is a literal `&` (the string table's escape).
pub fn menu_label_with(name: &str, key: Option<char>, parentheses: bool) -> String {
    let plain = name.replace("&&", "&");
    let Some(key) = key else {
        return plain;
    };
    if !parentheses && (plain.contains(key) || key.to_lowercase().any(|c| plain.contains(c))) {
        return plain;
    }
    if let Some(open) = plain.find('(')
        && plain[open..].contains(key)
    {
        return plain;
    }
    match plain.strip_suffix("...") {
        Some(stem) => format!("{stem}({key})..."),
        None => format!("{plain}({key})"),
    }
}

/// What the program can do for a Sakura function code.
pub trait Available {
    fn is_available(&self, code: u16) -> bool;
}

impl<F: Fn(u16) -> bool> Available for F {
    fn is_available(&self, code: u16) -> bool {
        self(code)
    }
}

/// The functions that have a core or a UI command in this program.
pub struct Registered;

impl Available for Registered {
    fn is_available(&self, code: u16) -> bool {
        // A registered macro (`F_USERMACRO_0 + n`) can always be called; Sakura does not gray out
        // a slot that has no file (`IsFuncEnable`), the call says that it failed.
        matsu_config::settings::macros::index_of_function(code).is_some()
            // A plugin command that is loaded (`F_PLUGCOMMAND_FIRST` to `F_PLUGCOMMAND_LAST`).
            || matsu_commands::plugin_commands::info(code).is_some()
            || matsu_commands::registry::Registry::global()
                .get_by_code(code)
                .is_some()
            || crate::ui_command::UiRegistry::global()
                .get_by_code(code)
                .is_some()
    }
}

/// Sakura's default main menu (`MainMenu.ini`) as rows: nodes and functions take their names from
/// the function's label, so no row has a name of its own.
pub fn default_main_menu() -> &'static MainMenu {
    static DEFAULT: OnceLock<MainMenu> = OnceLock::new();
    DEFAULT.get_or_init(|| MainMenu {
        rows: MAIN_MENU
            .iter()
            .map(|item| MenuRow {
                level: usize::from(item.level),
                kind: match item.kind {
                    MenuKind::Node => RowKind::Node,
                    MenuKind::Leaf => RowKind::Leaf,
                    MenuKind::Separator => RowKind::Separator,
                    MenuKind::Special => RowKind::Special,
                },
                function: item.code,
                key: item.access_key,
                name: String::new(),
                text: None,
            })
            .collect(),
        key_parentheses: KEY_PARENTHESES,
    })
}

/// The label of function `code` in the table (`LS(code)`, `Funccode2Name`).
fn function_label(code: u16) -> Option<&'static str> {
    FUNCTIONS
        .iter()
        .find(|f| f.code == code)
        .and_then(|f| f.label)
}

/// The name of a row as the menu shows it (`LayoutMainMenu`, `InitMenu`, `InitMenu_Function`): a node
/// with its own name (or `F_NODE`) shows it, any other row the label of its function.
fn name_of(row: &MenuRow, parentheses: bool) -> String {
    if row.kind == RowKind::Node && (row.function == F_NODE || !row.name.is_empty()) {
        return menu_label_with(&row.name, row.key, parentheses);
    }
    if let Some(label) = matsu_commands::plugin_commands::label(row.function) {
        return menu_label_with(&label, row.key, parentheses);
    }
    let label = function_label(row.function).unwrap_or("-- undefined name --");
    menu_label_with(label, row.key, parentheses)
}

/// The items of the lists that Sakura fills in at run time (`MenuKind::Special`): the recent files,
/// the windows, ... `items` gives the items for the row with this function code. A list with no
/// items shows a disabled "(なし)".
pub trait Lists {
    fn items(&self, code: u16) -> Vec<Item>;

    /// The entries the list adds to the menu: its items, unless the list has submenus
    /// (`F_PLUGIN_LIST` has one for each plugin).
    fn entries(&self, code: u16) -> Vec<Entry> {
        self.items(code).into_iter().map(Entry::Item).collect()
    }
}

/// No list is filled in.
pub struct NoLists;

impl Lists for NoLists {
    fn items(&self, _: u16) -> Vec<Item> {
        Vec::new()
    }
}

/// What the menus are built with.
struct Build<'a> {
    parentheses: bool,
    keymap: &'a KeyMap,
    available: &'a dyn Available,
    lists: &'a dyn Lists,
    /// The names of the custom menus, which a function row of one shows (`InitMenu_Function`).
    custom_menus: &'a CustomMenus,
}

impl Build<'_> {
    /// A function row (`InitMenu_Function`). The function of a custom menu shows the name of the menu
    /// and is grayed out when the menu has no items.
    fn leaf(&self, row: &MenuRow) -> Entry {
        let custom = crate::custom_menu::model::menu_index(row.function);
        let (action, enabled) = if let Some(index) = custom {
            let has_items = self
                .custom_menus
                .get(index)
                .is_some_and(|m| !m.items.is_empty());
            (
                MenuAction::Function(row.function),
                has_items && self.available.is_available(row.function),
            )
        } else if self.available.is_available(row.function) {
            (MenuAction::Function(row.function), true)
        } else if row.function == F_EXITALL {
            (MenuAction::Exit, true)
        } else {
            (MenuAction::Function(row.function), false)
        };
        let label = match custom.and_then(|index| self.custom_menus.name(index)) {
            Some(name) => menu_label_with(&name, row.key, self.parentheses),
            None => name_of(row, self.parentheses),
        };
        Entry::Item(Item {
            label,
            action,
            shortcut: self.keymap.shortcut_for(row.function),
            enabled,
        })
    }

    /// The entries of a list that Sakura fills in at run time: its items, or a disabled "(なし)".
    fn special(&self, code: u16) -> Vec<Entry> {
        let entries = self.lists.entries(code);
        if !entries.is_empty() {
            return entries;
        }
        vec![Entry::Item(Item {
            label: "(なし)".to_owned(),
            action: MenuAction::Placeholder,
            shortcut: None,
            enabled: false,
        })]
    }

    /// Builds the entries of the rows after `*at` that are deeper than `level`. A row more than one
    /// level below the row before it ends the menu (`InitMenu` stops there: it has no submenu to put
    /// the row in).
    fn entries(&self, rows: &[MenuRow], at: &mut usize, level: usize) -> Vec<Entry> {
        let mut out = Vec::new();
        while let Some(row) = rows.get(*at) {
            if row.level <= level {
                break;
            }
            if row.level > level + 1 {
                *at = rows.len();
                break;
            }
            *at += 1;
            match row.kind {
                RowKind::Separator => out.push(Entry::Separator),
                RowKind::Leaf => out.push(self.leaf(row)),
                // The list of recent files, windows and so on: the items go into this menu.
                RowKind::Special => out.extend(self.special(row.function)),
                RowKind::Node => out.push(Entry::Submenu {
                    label: name_of(row, self.parentheses),
                    entries: self.entries(rows, at, row.level),
                }),
            }
        }
        out
    }
}

/// Sakura's default main menu: the menus of the menu bar, in order. The lists that Sakura fills in
/// at run time are empty.
pub fn main_menu(keymap: &KeyMap, available: &dyn Available) -> Vec<TopMenu> {
    main_menu_with(keymap, available, &NoLists)
}

/// As [`main_menu`], with the run-time lists that `lists` fills in.
pub fn main_menu_with(
    keymap: &KeyMap,
    available: &dyn Available,
    lists: &dyn Lists,
) -> Vec<TopMenu> {
    main_menu_for(
        default_main_menu(),
        keymap,
        available,
        lists,
        &CustomMenus::default(),
    )
}

/// The menus of the menu bar for `menu` (`CEditWnd::LayoutMainMenu` and `InitMenu`): each row of
/// level 0 starts a menu, up to [`MAX_MAINMENU_TOP`]; the rows after it, until the next one, are in
/// it. A node is a menu and a list that Sakura fills in at run time is a menu with its items. A
/// function or a separator on the menu bar is not shown (a menu bar here holds menus only).
/// `custom_menus` names the custom menus that function rows open.
pub fn main_menu_for(
    menu: &MainMenu,
    keymap: &KeyMap,
    available: &dyn Available,
    lists: &dyn Lists,
    custom_menus: &CustomMenus,
) -> Vec<TopMenu> {
    let build = Build {
        parentheses: menu.key_parentheses,
        keymap,
        available,
        lists,
        custom_menus,
    };
    let tops: Vec<usize> = menu
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.level == 0)
        .map(|(i, _)| i)
        .take(MAX_MAINMENU_TOP)
        .collect();
    let mut out = Vec::new();
    for (n, &start) in tops.iter().enumerate() {
        let end = tops.get(n + 1).copied().unwrap_or(menu.rows.len());
        let head = &menu.rows[start];
        let body = &menu.rows[start + 1..end];
        let entries = match head.kind {
            RowKind::Node => build.entries(body, &mut 0, 0),
            RowKind::Special => build.special(head.function),
            RowKind::Leaf | RowKind::Separator => continue,
        };
        out.push(TopMenu {
            label: name_of(head, build.parentheses),
            entries,
        });
    }
    out
}

/// The text of the shortcut as the menu shows it: Mac symbols on macOS, Sakura's `Ctrl+O` style
/// elsewhere.
pub fn shortcut_text(chord: Chord, style: DisplayStyle) -> String {
    chord.display(style)
}

/// The label of an item with its shortcut after it (Sakura puts a tab between them; a GPUI menu
/// has no columns).
pub fn label_with_shortcut(item: &Item, style: DisplayStyle) -> String {
    match item.shortcut {
        Some(chord) => format!("{}    {}", item.label, shortcut_text(chord, style)),
        None => item.label.clone(),
    }
}

#[cfg(test)]
mod tests {
    use matsu_commands::keymap::{KeyMapOptions, Platform};

    use super::*;

    fn keymap() -> KeyMap {
        KeyMap::new(KeyMapOptions::for_platform(Platform::Other))
    }

    fn all(_: u16) -> bool {
        true
    }

    fn none(_: u16) -> bool {
        false
    }

    fn top<'a>(menus: &'a [TopMenu], label: &str) -> &'a TopMenu {
        menus.iter().find(|m| m.label == label).unwrap()
    }

    fn items(entries: &[Entry]) -> Vec<&Item> {
        entries
            .iter()
            .filter_map(|e| match e {
                Entry::Item(i) => Some(i),
                _ => None,
            })
            .collect()
    }

    // The expectations of the labels are the three branches of `CKeyBind::MakeMenuLabel`.
    #[test]
    fn labels_carry_the_access_key() {
        assert_eq!(menu_label("新規作成", Some('N')), "新規作成(N)");
        assert_eq!(menu_label("開く...", Some('O')), "開く(O)...");
        assert_eq!(menu_label("開く(O)...", Some('O')), "開く(O)...");
        assert_eq!(menu_label("A && B", Some('B')), "A & B(B)");
        assert_eq!(menu_label("UTF-8", None), "UTF-8");
    }

    #[test]
    fn the_menu_bar_is_sakuras() {
        let menus = main_menu(&keymap(), &all);
        let labels: Vec<_> = menus.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels[0], "ファイル(F)");
        assert_eq!(labels[1], "編集(E)");
        let tops = MAIN_MENU.iter().filter(|i| i.level == 0).count();
        assert_eq!(menus.len(), tops);
    }

    #[test]
    fn every_table_row_becomes_one_entry() {
        fn count(entries: &[Entry]) -> usize {
            entries
                .iter()
                .map(|e| match e {
                    Entry::Submenu { entries, .. } => 1 + count(entries),
                    _ => 1,
                })
                .sum()
        }
        let menus = main_menu(&keymap(), &all);
        let rows: usize = menus.iter().map(|m| count(&m.entries)).sum();
        assert_eq!(rows + menus.len(), MAIN_MENU.len());
    }

    #[test]
    fn the_file_menu_has_separators_submenus_and_shortcuts() {
        let menus = main_menu(&keymap(), &all);
        let file = top(&menus, "ファイル(F)");
        assert!(file.entries.contains(&Entry::Separator));
        let reopen = file
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::Submenu { label, entries } if label == "開き直す(W)" => Some(entries),
                _ => None,
            })
            .expect("the reopen submenu");
        assert!(
            items(reopen)
                .iter()
                .any(|i| i.label == "UTF-8で開き直す(8)")
        );
        let open = items(&file.entries)
            .into_iter()
            .find(|i| i.label == "開く(O)...")
            .unwrap();
        assert_eq!(open.action, MenuAction::Function(30102));
        // Sakura's table: Ctrl+O is F_FILEOPEN (CKeyBind.cpp).
        assert_eq!(
            open.shortcut.unwrap().display(DisplayStyle::Sakura),
            "Ctrl+O"
        );
        assert_eq!(
            label_with_shortcut(open, DisplayStyle::Sakura),
            "開く(O)...    Ctrl+O"
        );
    }

    // Plan A (ADR-0008): on macOS the menu shows the Mac chord (Cmd+O) where the key map has one.
    #[test]
    fn the_shortcuts_follow_the_key_map_of_the_os() {
        let mac = KeyMap::new(KeyMapOptions::for_platform(Platform::MacOs));
        let menus = main_menu(&mac, &all);
        let open = items(&top(&menus, "ファイル(F)").entries)
            .into_iter()
            .find(|i| i.label == "開く(O)...")
            .unwrap()
            .shortcut
            .unwrap();
        assert_eq!(open.display(DisplayStyle::Mac), "⌘O");
    }

    #[test]
    fn the_edit_menu_runs_undo_and_redo_by_code() {
        let menus = main_menu(&keymap(), &all);
        let edit = items(&top(&menus, "編集(E)").entries);
        assert_eq!(edit[0].label, "元に戻す(U)");
        assert_eq!(edit[0].action, MenuAction::Function(30210));
        assert_eq!(edit[1].action, MenuAction::Function(30211));
    }

    #[test]
    fn a_function_without_a_command_is_grayed_out() {
        let menus = main_menu(&keymap(), &|code| code == 30210);
        let edit = items(&top(&menus, "編集(E)").entries);
        assert!(edit[0].enabled);
        assert!(!edit[1].enabled);
        let menus = main_menu(&keymap(), &none);
        assert!(
            items(&top(&menus, "編集(E)").entries)
                .iter()
                .all(|i| !i.enabled)
        );
    }

    #[test]
    fn lists_filled_in_at_run_time_are_disabled_placeholders() {
        let menus = main_menu(&keymap(), &all);
        fn placeholders(entries: &[Entry], out: &mut usize) {
            for e in entries {
                match e {
                    Entry::Item(i) if i.action == MenuAction::Placeholder => {
                        assert!(!i.enabled);
                        *out += 1;
                    }
                    Entry::Submenu { entries, .. } => placeholders(entries, out),
                    _ => {}
                }
            }
        }
        let mut n = 0;
        for m in &menus {
            placeholders(&m.entries, &mut n);
        }
        let specials = MAIN_MENU
            .iter()
            .filter(|i| i.kind == MenuKind::Special)
            .count();
        assert_eq!(n, specials);
    }

    #[test]
    fn exit_all_quits_until_a_command_does() {
        let find = |menus: &[TopMenu]| {
            items(&top(menus, "ファイル(F)").entries)
                .into_iter()
                .find(|i| i.label.starts_with("マツエディタの全終了"))
                .cloned()
                .unwrap()
        };
        let without = find(&main_menu(&keymap(), &none));
        assert_eq!((without.action, without.enabled), (MenuAction::Exit, true));
        let with = find(&main_menu(&keymap(), &all));
        assert_eq!(with.action, MenuAction::Function(F_EXITALL));
    }

    #[test]
    fn the_program_s_commands_fill_in_the_menu() {
        // Undo and redo are core commands.
        let menus = main_menu(&keymap(), &Registered);
        let edit = items(&top(&menus, "編集(E)").entries);
        assert!(edit[0].enabled && edit[1].enabled);
    }

    // ---- the menu of the user ([MainMenu]) ----

    fn node(level: usize, function: u16, name: &str, key: Option<char>) -> MenuRow {
        MenuRow {
            level,
            kind: RowKind::Node,
            function,
            key,
            name: name.to_owned(),
            text: None,
        }
    }

    fn function(level: usize, function: u16, key: Option<char>) -> MenuRow {
        MenuRow {
            key,
            ..MenuRow::leaf(level, function)
        }
    }

    fn special(level: usize, function: u16) -> MenuRow {
        MenuRow {
            kind: RowKind::Special,
            ..MenuRow::leaf(level, function)
        }
    }

    fn user(rows: Vec<MenuRow>, key_parentheses: bool) -> Vec<TopMenu> {
        let menu = MainMenu {
            rows,
            key_parentheses,
        };
        main_menu_for(&menu, &keymap(), &all, &NoLists, &CustomMenus::default())
    }

    #[test]
    fn the_default_rows_are_the_table() {
        // A row of the default menu has no name of its own: it shows the label of its function, and
        // that is the label of the table.
        for item in MAIN_MENU {
            if let Some(label) = item.label {
                assert_eq!(function_label(item.code), Some(label), "{}", item.code);
            }
        }
        let menu = default_main_menu();
        assert_eq!(menu.rows.len(), MAIN_MENU.len());
        assert!(menu.key_parentheses);
        // The 共通設定 stays in it (OK of the page adds it when it is missing).
        assert!(menu.rows.iter().any(|r| r.function == 31112));
    }

    #[test]
    fn a_users_menu_has_levels_separators_lists_and_access_keys() {
        let menus = user(
            vec![
                node(0, 0, "My && File", Some('F')),
                function(1, 30102, Some('O')),
                MenuRow::separator(1),
                node(1, 34005, "", Some('W')),
                function(2, 30119, Some('W')),
                special(1, 29002),
                function(1, 30103, None),
                node(0, 34059, "", Some('H')),
                function(1, 31455, Some('A')),
            ],
            true,
        );
        assert_eq!(menus.len(), 2);
        assert_eq!(menus[0].label, "My & File(F)");
        assert_eq!(menus[1].label, "ヘルプ(H)");
        let entries = &menus[0].entries;
        assert!(
            matches!(&entries[0], Entry::Item(i) if i.label == "開く(O)..." && i.action == MenuAction::Function(30102))
        );
        assert_eq!(entries[1], Entry::Separator);
        // A node that takes its name from its function.
        assert!(
            matches!(&entries[2], Entry::Submenu { label, entries } if label == "開き直す(W)" && entries.len() == 1)
        );
        // A list that Sakura fills in at run time is a disabled "(なし)" without items.
        assert!(matches!(&entries[3], Entry::Item(i) if i.action == MenuAction::Placeholder));
        // A function without a key shows no key.
        assert!(matches!(&entries[4], Entry::Item(i) if i.label == "上書き保存"));
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn without_parentheses_a_letter_of_the_name_is_the_access_key() {
        let rows = vec![
            node(0, 0, "File", Some('F')),
            function(1, 30101, Some('N')),
            function(1, 30102, Some('o')),
            function(1, 30103, Some('K')),
        ];
        let with = user(rows.clone(), true);
        assert_eq!(with[0].label, "File(F)");
        let without = user(rows, false);
        // `File` has an F, `新規作成` has no N: the key is added after the name; 開く... has no o.
        assert_eq!(without[0].label, "File");
        let labels: Vec<_> = items(&without[0].entries)
            .iter()
            .map(|i| i.label.clone())
            .collect();
        assert_eq!(labels, ["新規作成(N)", "開く(o)...", "上書き保存(K)"]);
        // The key itself, or its lower case, must be in the name (`wcschr` twice in `MakeMenuLabel`).
        assert_eq!(menu_label_with("Save", Some('S'), false), "Save");
        assert_eq!(menu_label_with("Save", Some('A'), false), "Save");
        assert_eq!(menu_label_with("Save", Some('s'), false), "Save(s)");
        assert_eq!(
            menu_label_with("開く(O)...", Some('O'), false),
            "開く(O)..."
        );
    }

    #[test]
    fn a_list_on_the_menu_bar_is_a_menu_and_a_function_there_is_left_out() {
        let menus = user(
            vec![
                function(0, 30101, Some('N')),
                MenuRow::separator(0),
                special(0, 29001),
                node(0, 0, "Mine", None),
                function(1, 30102, None),
            ],
            true,
        );
        let labels: Vec<_> = menus.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(labels, ["ウィンドウリスト", "Mine"]);
        assert_eq!(menus[0].entries.len(), 1, "the (なし) of the empty list");
    }

    #[test]
    fn at_most_twenty_menus_show_and_a_row_two_levels_down_ends_its_menu() {
        let rows: Vec<MenuRow> = (0..25)
            .map(|i| node(0, 0, &format!("m{i}"), None))
            .collect();
        assert_eq!(user(rows, true).len(), 20);
        let menus = user(
            vec![
                node(0, 0, "A", None),
                function(1, 30101, None),
                function(3, 30102, None),
                function(1, 30103, None),
                node(0, 0, "B", None),
                function(1, 30104, None),
            ],
            true,
        );
        assert_eq!(menus[0].entries.len(), 1, "InitMenu stops at the row");
        assert_eq!(menus[1].entries.len(), 1, "the next menu is not affected");
    }

    #[test]
    fn the_lists_fill_in_the_special_rows_of_a_users_menu() {
        struct One;
        impl Lists for One {
            fn items(&self, code: u16) -> Vec<Item> {
                (code == 29002)
                    .then(|| Item {
                        label: "recent".to_owned(),
                        action: MenuAction::RecentFile("x".to_owned()),
                        shortcut: None,
                        enabled: true,
                    })
                    .into_iter()
                    .collect()
            }
        }
        let menu = MainMenu {
            rows: vec![node(0, 0, "A", None), special(1, 29002)],
            key_parentheses: true,
        };
        let menus = main_menu_for(&menu, &keymap(), &all, &One, &CustomMenus::default());
        assert!(matches!(&menus[0].entries[0], Entry::Item(i) if i.label == "recent"));
    }

    #[test]
    fn a_custom_menu_row_shows_the_name_of_the_menu_and_is_grayed_out_when_it_is_empty() {
        let mut menus = CustomMenus::default().menus().to_vec();
        menus[3].name = "Build".to_owned();
        let menus = CustomMenus::from_menus(menus);
        let menu = MainMenu {
            rows: vec![
                node(0, 0, "A", None),
                // `F_CUSTMENU_1` is empty, `F_CUSTMENU_3` is named but empty too, the right-click menu has items.
                function(1, 31503, Some('B')),
                function(1, 31501, None),
                function(1, 31580, None),
            ],
            key_parentheses: true,
        };
        let built = main_menu_for(&menu, &keymap(), &all, &NoLists, &menus);
        let found = items(&built[0].entries);
        assert_eq!(
            (found[0].label.as_str(), found[0].enabled),
            ("Build(B)", false)
        );
        // Menu 1 has Sakura's seven items.
        assert!(found[1].enabled);
        assert_eq!(found[2].label, "右クリックメニュー");
        assert!(found[2].enabled);
    }
}
