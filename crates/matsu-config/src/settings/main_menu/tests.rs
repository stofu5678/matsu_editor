use super::*;

fn names(name: &str) -> Option<u16> {
    match name {
        "Undo" => Some(30210),
        "FileOpen" => Some(30102),
        _ => None,
    }
}

fn profile(text: &str) -> Profile {
    Profile::from_bytes(text.as_bytes())
}

fn default_menu() -> MainMenu {
    MainMenu {
        rows: vec![
            MenuRow {
                key: Some('F'),
                ..MenuRow {
                    function: 34052,
                    ..MenuRow::blank()
                }
            },
            MenuRow::leaf(1, 30101),
        ],
        key_parentheses: true,
    }
}

fn read(text: &str) -> MainMenu {
    MainMenu::from_profile(&profile(text), &names, &default_menu())
}

fn leaf(level: usize, function: u16, key: char) -> MenuRow {
    MenuRow {
        key: Some(key),
        ..MenuRow::leaf(level, function)
    }
}

fn node(level: usize, name: &str) -> MenuRow {
    MenuRow {
        name: name.to_owned(),
        ..MenuRow::blank_at(level)
    }
}

impl MenuRow {
    fn blank_at(level: usize) -> Self {
        Self {
            level,
            ..Self::blank()
        }
    }
}

#[test]
fn no_section_means_the_default() {
    assert_eq!(read(""), default_menu());
    assert_eq!(
        read("[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=0\n"),
        default_menu()
    );
    assert!(MainMenu::read_raw(&profile(""), &names).is_none());
}

#[test]
fn the_rows_are_level_type_function_key_name() {
    let menu = read(
        "[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=5\nbKeyParentheses=0\n\
         MM[000]=0,0,0,F,My File\n\
         MM[001]=1,1,30102,O,\n\
         MM[002]=1,2,1,,\n\
         MM[003]=1,3,29002,,\n\
         MM[004]=0,0,34059,H,\n",
    );
    assert!(!menu.key_parentheses);
    assert_eq!(
        menu.rows,
        vec![
            MenuRow {
                key: Some('F'),
                ..node(0, "My File")
            },
            leaf(1, 30102, 'O'),
            MenuRow::separator(1),
            MenuRow {
                kind: RowKind::Special,
                function: 29002,
                ..MenuRow::blank_at(1)
            },
            MenuRow {
                function: 34059,
                key: Some('H'),
                ..MenuRow::blank()
            },
        ]
    );
}

#[test]
fn the_access_key_can_be_a_comma_or_missing() {
    let menu = read(
        "[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=4\n\
         MM[000]=1,1,30102,,,Name\n\
         MM[001]=1,1,30102,,Name\n\
         MM[002]=1,1,30102,\n\
         MM[003]=1,1,30102\n",
    );
    assert_eq!(menu.rows[0].key, Some(','));
    assert_eq!(menu.rows[0].name, "Name");
    assert_eq!(menu.rows[1].key, None);
    assert_eq!(menu.rows[1].name, "Name");
    assert_eq!((menu.rows[2].key, menu.rows[2].name.as_str()), (None, ""));
    assert_eq!((menu.rows[3].key, menu.rows[3].function), (None, 30102));
}

#[test]
fn functions_may_be_macro_names_and_names_are_cut() {
    let long = "あ".repeat(50);
    let menu = read(&format!(
        "[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=3\n\
         MM[000]=1,1,Undo,U,\n\
         MM[001]=1,1,Unknown,U,\n\
         MM[002]=0,0,0,A,{long}\n"
    ));
    assert_eq!(menu.rows[0].function, 30210);
    assert_eq!(menu.rows[1].function, 0, "an unknown name is F_DEFAULT");
    assert_eq!(menu.rows[2].name.chars().count(), MAX_MAIN_MENU_NAME_LEN);
}

#[test]
fn a_missing_line_is_a_blank_row_and_the_count_is_limited() {
    let menu = read("[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=3\nMM[001]=1,1,30102,O,\n");
    assert_eq!(menu.rows.len(), 3);
    assert_eq!(menu.rows[0], MenuRow::blank());
    assert_eq!(menu.rows[1].function, 30102);
    let menu = read("[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=9999\n");
    assert_eq!(menu.rows.len(), MAX_MAINMENU);
}

#[test]
fn a_menu_without_a_version_gets_the_items_of_newer_versions() {
    // F_FILENEW (30101) is in the menu; F_FILENEW_NEWWINDOW (30110) comes after it. The separator and
    // the function list marks come together, and a function the menu already has is not added again.
    let text = "[MainMenu]\nnMatsuMainMenuVer=1\nnMainMenuNum=4\n\
         MM[000]=0,0,34052,F,\n\
         MM[001]=1,1,30101,N,\n\
         MM[002]=1,1,30372,,\n\
         MM[003]=1,1,31370,,\n";
    let menu = read(text);
    let functions: Vec<u16> = menu.rows.iter().map(|r| r.function).collect();
    assert_eq!(
        functions,
        [
            34052, 30101, 30110, 30372, 1, 30988, 30989, 30393, 30394, 31370, 31337
        ]
    );
    assert_eq!(menu.rows[2], MenuRow::leaf(1, 30110), "no access key");
    assert_eq!(menu.rows[4], MenuRow::separator(1));
    // The version 2 file only gets the items of version 2.
    let menu = read(&text.replace("nMainMenuNum=4", "nMainMenuVer=1\nnMainMenuNum=4"));
    let functions: Vec<u16> = menu.rows.iter().map(|r| r.function).collect();
    assert_eq!(functions, [34052, 30101, 30372, 31370, 31337]);
    // The current version gets none.
    let menu = read(&text.replace("nMainMenuNum=4", "nMainMenuVer=2\nnMainMenuNum=4"));
    assert_eq!(menu.rows.len(), 4);
}

#[test]
fn writing_then_reading_gives_the_menu_back() {
    let menu = MainMenu {
        rows: vec![
            MenuRow {
                key: Some('A'),
                ..node(0, "A && B")
            },
            leaf(1, 30102, 'O'),
            MenuRow::separator(1),
            MenuRow {
                kind: RowKind::Special,
                function: 29002,
                ..MenuRow::blank_at(1)
            },
            leaf(1, 31112, ','),
        ],
        key_parentheses: false,
    };
    let mut file = profile("[Common]\nx=1\n");
    menu.write_to_profile(&mut file, None, None, Some(CURRENT_VERSION))
        .unwrap();
    assert_eq!(file.get("Common", "x"), Some("1"));
    assert_eq!(file.get("MainMenu", "MM[000]"), Some("0,0,0,A,A && B"));
    assert_eq!(file.get("MainMenu", "MM[004]"), Some("1,1,31112,,,"));
    assert_eq!(MainMenu::from_profile(&file, &names, &default_menu()), menu);
}

#[test]
fn writing_a_shorter_menu_removes_the_old_lines_and_keeps_the_text_of_unchanged_rows() {
    let mut file = profile(
        "[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=3\nbKeyParentheses=1\n\
         MM[000]=0,0,34052,F,\n\
         MM[001]=1,1,plugin/cmd,,\n\
         MM[002]=1,1,30102,O,\n",
    );
    let before = MainMenu::read_raw(&file, &names).unwrap();
    // The plug-in command reads as function 0; the row is not changed, so its text stays.
    let mut menu = before.clone();
    menu.rows.pop();
    menu.write_to_profile(&mut file, None, Some(&before), Some(CURRENT_VERSION))
        .unwrap();
    assert_eq!(file.get("MainMenu", "MM[001]"), Some("1,1,plugin/cmd,,"));
    assert_eq!(file.get("MainMenu", "MM[002]"), None);
    assert_eq!(file.get("MainMenu", "nMainMenuNum"), Some("2"));
}

#[test]
fn export_names_the_functions_and_import_reads_them_over_the_menu() {
    let menu = MainMenu {
        rows: vec![node(0, "X"), leaf(1, 30210, 'U'), leaf(1, 30999, 'V')],
        key_parentheses: true,
    };
    let exported = menu
        .export_profile(&|code| (code == 30210).then(|| "Undo".to_owned()))
        .unwrap();
    assert_eq!(
        exported.get("Info", "MENU_VERSION"),
        Some("SakuraEditorMainMenu Ver1")
    );
    assert_eq!(exported.get("MainMenu", "MM[001]"), Some("1,1,Undo,U,"));
    assert_eq!(exported.get("MainMenu", "MM[002]"), Some("1,1,30999,V,"));
    assert_eq!(exported.get("MainMenu", "nMainMenuVer"), None);

    let mut target = default_menu();
    assert!(target.import_profile(&exported, &names));
    assert_eq!(target, menu);

    // A file of another kind changes nothing; a file with no menu leaves the menu.
    let mut other = default_menu();
    assert!(!other.import_profile(
        &profile("[Info]\nMENU_VERSION=SakuraEditorMenu_Ver2\n"),
        &names
    ));
    assert!(other.import_profile(
        &profile("[Info]\nMENU_VERSION=SakuraEditorMainMenu Ver1\n"),
        &names
    ));
    assert_eq!(other, default_menu());
}

#[test]
fn the_page_shows_a_tree_and_unescapes_names() {
    let menu = MainMenu {
        rows: vec![
            node(0, "A && B"),
            leaf(1, 30102, 'O'),
            // Two levels at once: a child of the row before it.
            MenuRow {
                level: 3,
                ..MenuRow::separator(0)
            },
            // Under a function row: a sibling.
            leaf(2, 30103, 'S'),
            leaf(5, 30104, 'A'),
            MenuRow {
                function: 34052,
                ..MenuRow::blank_at(1)
            },
        ],
        key_parentheses: true,
    };
    let rows = menu.edit_rows();
    let levels: Vec<usize> = rows.iter().map(|r| r.level).collect();
    assert_eq!(levels, [0, 1, 1, 1, 1, 1]);
    assert_eq!(rows[0].name, "A & B");
    assert_eq!(rows[5].name, "", "a node with a function takes its label");
}

#[test]
fn ok_gives_the_types_of_the_functions_and_adds_the_common_settings() {
    let rows = vec![
        node(0, "A & B"),
        leaf(1, 30102, 'O'),
        MenuRow {
            kind: RowKind::Leaf,
            ..MenuRow::separator(1)
        },
        // A list: its type follows its function.
        leaf(1, 29002, 'R'),
        // A node that takes its name from its function.
        MenuRow {
            function: 34052,
            ..MenuRow::blank_at(0)
        },
        // The child of that node, and a row under a function row, which is dropped.
        leaf(1, 30103, 'S'),
        leaf(2, 30104, 'A'),
    ];
    let menu = MainMenu::from_edited(&rows, true);
    let kinds: Vec<_> = menu.rows.iter().map(|r| (r.level, r.kind)).collect();
    assert_eq!(
        kinds,
        [
            (0, RowKind::Node),
            (1, RowKind::Leaf),
            (1, RowKind::Separator),
            (1, RowKind::Special),
            (0, RowKind::Node),
            (1, RowKind::Leaf),
            (0, RowKind::Node),
            (1, RowKind::Leaf),
        ]
    );
    assert_eq!(menu.rows[0].name, "A && B");
    assert_eq!(menu.rows[4].name, "");
    // 共通設定 is added in a new menu at the end.
    assert_eq!(menu.rows[6].name, "auto_add");
    assert_eq!(menu.rows[7], MenuRow::leaf(1, F_OPTION));
    // With 共通設定 in the menu, nothing is added.
    let with_option = [node(0, "A"), leaf(1, F_OPTION, 'O')];
    assert_eq!(MainMenu::from_edited(&with_option, false).rows.len(), 2);
}

#[test]
fn ok_keeps_the_limits() {
    let mut rows: Vec<MenuRow> = (0..25).map(|i| node(0, &format!("m{i}"))).collect();
    rows.push(leaf(1, F_OPTION, 'O'));
    let menu = MainMenu::from_edited(&rows, true);
    // Only 20 menus fit in the menu bar; the 共通設定 below the 25th was dropped with it, so it is
    // added at the end of the last menu.
    assert_eq!(menu.rows.iter().filter(|r| r.level == 0).count(), 20);
    assert_eq!(menu.rows.last(), Some(&MenuRow::leaf(1, F_OPTION)));

    let rows: Vec<MenuRow> = std::iter::once(node(0, "a"))
        .chain((0..MAX_MAINMENU + 10).map(|_| leaf(1, 30101, 'A')))
        .collect();
    let menu = MainMenu::from_edited(&rows, true);
    assert_eq!(menu.rows.len(), MAX_MAINMENU);
}

#[test]
fn ampersands() {
    assert_eq!(supplement_ampersand("A & B && C"), "A && B && C");
    assert_eq!(remove_ampersand("A && B & C"), "A & B & C");
}

/// A menu saved by MATSU 0.5.2 or before: Sakura's version 2, no MATSU version, and none of MATSU's items.
const OLD_MATSU_MENU: &str = "[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=5\n\
     MM[000]=0,0,34052,F,\n\
     MM[001]=1,1,30603,L,\n\
     MM[002]=1,1,30604,P,\n\
     MM[003]=1,1,31370,U,\n\
     MM[004]=1,1,31371,,\n";

fn functions(menu: &MainMenu) -> Vec<u16> {
    menu.rows.iter().map(|r| r.function).collect()
}

#[test]
fn a_menu_saved_before_the_matsu_items_gets_them_once_in_their_places() {
    let menu = read(OLD_MATSU_MENU);
    assert_eq!(
        functions(&menu),
        [34052, 30603, 32700, 32701, 30604, 31370, 32702, 31371]
    );
    assert_eq!(menu.rows[2], leaf(1, 32700, 'N'), "the default menu's key");
    assert_eq!(menu.rows[6], leaf(1, 32702, 'G'));
    // Sakura's own version is not touched: version 2 adds none of Sakura's items.
    assert!(!functions(&menu).contains(&31337));
}

#[test]
fn a_menu_saved_with_the_matsu_version_is_not_given_the_items_back() {
    // The user took the items out and saved: the current MATSU version is written then.
    let text = OLD_MATSU_MENU.replace("nMainMenuVer=2", "nMainMenuVer=2\nnMatsuMainMenuVer=2");
    assert_eq!(functions(&read(&text)), [34052, 30603, 30604, 31370, 31371]);
}

/// A menu with the database items (30171 after 30170), saved with MATSU version 1 or none.
const MATSU_MENU_WITH_DATABASE: &str = "[MainMenu]\nnMainMenuVer=2\nnMatsuMainMenuVer=1\nnMainMenuNum=5\n\
     MM[000]=0,0,34056,T,\n\
     MM[001]=1,1,30170,P,\n\
     MM[002]=1,1,30171,S,\n\
     MM[003]=1,1,31430,H,\n\
     MM[004]=1,1,31270,X,\n";

#[test]
fn a_menu_of_matsu_version_1_gets_the_chooser_after_the_database_run_once() {
    let menu = read(MATSU_MENU_WITH_DATABASE);
    assert_eq!(functions(&menu), [34056, 30170, 30171, 32703, 31430, 31270]);
    assert_eq!(menu.rows[3], leaf(1, 32703, 'C'));
    // saved with version 2 (what a save writes now), reading it again adds nothing
    let mut file = profile("");
    menu.write_to_profile(&mut file, None, None, Some(CURRENT_VERSION))
        .unwrap();
    assert_eq!(file.get("MainMenu", "nMatsuMainMenuVer"), Some("2"));
    assert_eq!(MainMenu::from_profile(&file, &names, &default_menu()), menu);
    // a menu with no MATSU version at all gets it too, with version 1's items that have a place
    let text = MATSU_MENU_WITH_DATABASE.replace("nMatsuMainMenuVer=1\n", "");
    assert!(functions(&read(&text)).contains(&32703));
}

#[test]
fn a_menu_of_matsu_version_2_is_not_given_the_chooser() {
    let text = MATSU_MENU_WITH_DATABASE.replace("nMatsuMainMenuVer=1", "nMatsuMainMenuVer=2");
    assert_eq!(functions(&read(&text)), [34056, 30170, 30171, 31430, 31270]);
}

#[test]
fn the_chooser_is_left_out_of_a_menu_without_the_database_run_and_not_doubled() {
    // no 30171: nothing to follow, as for the other additions
    let text = MATSU_MENU_WITH_DATABASE.replace("MM[002]=1,1,30171,S,", "MM[002]=1,1,30172,S,");
    assert!(!functions(&read(&text)).contains(&32703));
    // 32703 is in the menu in another place already: it is not added again
    let text = MATSU_MENU_WITH_DATABASE.replace("MM[004]=1,1,31270,X,", "MM[004]=1,1,32703,,");
    let functions = functions(&read(&text));
    assert_eq!(functions.iter().filter(|&&f| f == 32703).count(), 1);
}

#[test]
fn the_matsu_items_the_menu_already_has_are_not_doubled() {
    // 32700 is in the menu in another place: it stays, 32701 follows it, 32702 is there already.
    let text = "[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=5\n\
         MM[000]=0,0,34052,F,\n\
         MM[001]=1,1,32700,,\n\
         MM[002]=1,1,30603,L,\n\
         MM[003]=1,1,31370,U,\n\
         MM[004]=1,1,32702,,\n";
    let once = read(text);
    assert_eq!(functions(&once), [34052, 32700, 32701, 30603, 31370, 32702]);
    // Reading what was written gives the same menu: the migration is idempotent.
    let mut file = profile(text);
    once.write_to_profile(&mut file, None, None, Some(CURRENT_VERSION))
        .unwrap();
    assert_eq!(MainMenu::from_profile(&file, &names, &default_menu()), once);
}

#[test]
fn a_matsu_item_whose_place_is_not_in_the_menu_is_left_out() {
    let text = "[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=2\n\
         MM[000]=0,0,34052,F,\n\
         MM[001]=1,1,30604,P,\n";
    assert_eq!(functions(&read(text)), [34052, 30604]);
}

#[test]
fn the_matsu_version_is_written_with_a_saved_menu_and_not_in_an_exported_file() {
    let mut file = profile("");
    let menu = read(OLD_MATSU_MENU);
    menu.write_to_profile(&mut file, None, None, Some(CURRENT_VERSION))
        .unwrap();
    assert_eq!(file.get("MainMenu", "nMatsuMainMenuVer"), Some("2"));
    let exported = menu.export_profile(&|_| None).unwrap();
    assert_eq!(exported.get("MainMenu", "nMatsuMainMenuVer"), None);
}

#[test]
fn a_matsu_item_gets_its_key_unless_a_sibling_in_the_same_submenu_has_it() {
    // N is taken by a sibling (in any case) in the submenu of 30603; M is taken only in another submenu.
    let text = "[MainMenu]\nnMainMenuVer=2\nnMainMenuNum=6\n\
         MM[000]=0,0,34052,F,\n\
         MM[001]=1,1,30603,L,\n\
         MM[002]=1,1,30604,n,\n\
         MM[003]=0,0,34053,G,\n\
         MM[004]=1,1,30601,M,\n\
         MM[005]=1,1,31370,U,\n";
    let menu = read(text);
    let key_of = |function| {
        menu.rows
            .iter()
            .find(|r| r.function == function)
            .unwrap()
            .key
    };
    assert_eq!(key_of(32700), None, "N clashes");
    assert_eq!(key_of(32701), Some('M'), "M is free in this submenu");
    assert_eq!(key_of(32702), Some('G'), "G is only a menu of the bar");
}
