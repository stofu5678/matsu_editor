//! Sakura's main menu as a model, built from the table of `ume_commands::sakura::MAIN_MENU`.
//!
//! The model knows nothing of GPUI: labels with access keys, separators, submenus, the shortcut of
//! each item from the key map, and whether the item can run. [`crate::app`] turns it into GPUI
//! menus. Tests of the menu are tests of this model.
//!
//! Sakura: `CEditWnd::LayoutMainMenu` / `InitMenu` (`sakura_core/window/CEditWnd.cpp`) build the
//! menu from the table; `CKeyBind::MakeMenuLabel` (`sakura_core/func/CKeyBind.cpp`) puts the access
//! key in the label; `CKeyBind::GetMenuLabel` adds the shortcut after a tab.

use ume_commands::keymap::{Chord, DisplayStyle, KeyMap};
use ume_commands::sakura::{FUNCTIONS, KEY_PARENTHESES, MAIN_MENU, MenuItem, MenuKind};

/// `F_EXITALL`, "サクラエディタの全終了". Until a command implements it, the item runs the `Exit`
/// action of the application (which the first window already had).
pub const F_EXITALL: u16 = 30195;

/// What an item does when it is chosen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    /// Run the Sakura function with this code on the editor.
    Function(u16),
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
/// the "key in parentheses" setting, [`KEY_PARENTHESES`], on). A GPUI menu has no mnemonic mark,
/// so the `&` that Sakura puts in front of the key is left out.
///
/// * If the name has a `(` followed by the key, the name already says it: `開く(O)` stays.
/// * A name that ends in `...` gets the key before the dots: `開く...` becomes `開く(O)...`.
/// * Otherwise `(K)` goes at the end.
///
/// `&&` in the name is a literal `&` (the string table's escape).
pub fn menu_label(name: &str, key: Option<char>) -> String {
    let plain = name.replace("&&", "&");
    let Some(key) = key else {
        return plain;
    };
    // The other style (a letter of the name as the key) is not built.
    const { assert!(KEY_PARENTHESES) };
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
        ume_commands::registry::Registry::global()
            .get_by_code(code)
            .is_some()
            || crate::ui_command::UiRegistry::global()
                .get_by_code(code)
                .is_some()
    }
}

fn name_of(item: &MenuItem) -> String {
    let label = item.label.or_else(|| {
        FUNCTIONS
            .iter()
            .find(|f| f.code == item.code)
            .and_then(|f| f.label)
    });
    menu_label(label.unwrap_or("-- undefined name --"), item.access_key)
}

fn leaf(item: &MenuItem, keymap: &KeyMap, available: &dyn Available) -> Entry {
    let (action, enabled) = if available.is_available(item.code) {
        (MenuAction::Function(item.code), true)
    } else if item.code == F_EXITALL {
        (MenuAction::Exit, true)
    } else {
        (MenuAction::Function(item.code), false)
    };
    Entry::Item(Item {
        label: name_of(item),
        action,
        shortcut: keymap.shortcut_for(item.code),
        enabled,
    })
}

/// Builds the entries of the items after `*at` that are deeper than `level`.
fn entries(at: &mut usize, level: u8, keymap: &KeyMap, available: &dyn Available) -> Vec<Entry> {
    let mut out = Vec::new();
    while let Some(item) = MAIN_MENU.get(*at) {
        if item.level <= level {
            break;
        }
        *at += 1;
        out.push(match item.kind {
            MenuKind::Separator => Entry::Separator,
            MenuKind::Leaf => leaf(item, keymap, available),
            // The list of recent files, windows and so on: nothing to list yet.
            MenuKind::Special => Entry::Item(Item {
                label: "(なし)".to_owned(),
                action: MenuAction::Placeholder,
                shortcut: None,
                enabled: false,
            }),
            MenuKind::Node => Entry::Submenu {
                label: name_of(item),
                entries: entries(at, item.level, keymap, available),
            },
        });
    }
    out
}

/// Sakura's main menu: the menus of the menu bar, in order.
pub fn main_menu(keymap: &KeyMap, available: &dyn Available) -> Vec<TopMenu> {
    let mut tops = Vec::new();
    let mut at = 0;
    while let Some(item) = MAIN_MENU.get(at) {
        at += 1;
        debug_assert_eq!(item.level, 0, "the table starts each menu at level 0");
        tops.push(TopMenu {
            label: name_of(item),
            entries: entries(&mut at, 0, keymap, available),
        });
    }
    tops
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
    use ume_commands::keymap::{KeyMapOptions, Platform};

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
                .find(|i| i.label.starts_with("サクラエディタの全終了"))
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
}
