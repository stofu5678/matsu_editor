use super::*;

fn profile(text: &str) -> Profile {
    Profile::from_bytes(text.as_bytes())
}

fn text(profile: &Profile) -> String {
    String::from_utf8(profile.to_bytes()).unwrap()
}

#[test]
fn the_default_is_one_grep_item_for_the_current_folder() {
    // CShareData::InitFileTree
    let tree = FileTree::default();
    assert!(tree.project);
    assert_eq!(tree.project_ini, "");
    assert_eq!(tree.items.len(), 1);
    let item = &tree.items[0];
    assert_eq!(item.item_type, ItemType::Grep);
    assert_eq!(item.target_path, ".");
    assert_eq!(item.target_file, "*.*");
    assert!(item.ignore_hidden && !item.ignore_read_only && !item.ignore_system);
    assert_eq!(FileTree::from_profile(&Profile::new(), "Common"), tree);
}

#[test]
fn the_keys_are_sakuras() {
    let p = profile(
        "[Common]\r\nbFileTreeProject=0\r\nszFileTreeProjectIni=tree.ini\r\nnFileTreeItemCount=3\r\n\
         FileTree(0).eItemType=0\r\nFileTree(0).szTargetPath=C:\\work\r\nFileTree(0).szLabelName=Work\r\n\
         FileTree(0).nDepth=1\r\nFileTree(0).szTargetFile=*.rs;*.toml\r\nFileTree(0).bIgnoreHidden=0\r\n\
         FileTree(0).bIgnoreReadOny=1\r\nFileTree(0).bIgnoreSystem=1\r\n\
         FileTree(1).eItemType=1\r\nFileTree(1).szTargetPath=C:\\notes.txt\r\nFileTree(1).nDepth=2\r\n\
         FileTree(2).eItemType=2\r\nFileTree(2).szLabelName=Docs\r\nFileTree(2).nDepth=0\r\n",
    );
    let tree = FileTree::from_profile(&p, "Common");
    assert!(!tree.project);
    assert_eq!(tree.project_ini, "tree.ini");
    assert_eq!(tree.items.len(), 3);
    assert_eq!(
        tree.items[0],
        FileTreeItem {
            item_type: ItemType::Grep,
            target_path: "C:\\work".into(),
            label: "Work".into(),
            depth: 1,
            target_file: "*.rs;*.toml".into(),
            ignore_hidden: false,
            ignore_read_only: true,
            ignore_system: true,
        }
    );
    assert_eq!(tree.items[1].item_type, ItemType::File);
    assert_eq!(tree.items[1].target_path, "C:\\notes.txt");
    assert_eq!(tree.items[1].depth, 2);
    // a field the file lacks is the default of the item
    assert!(tree.items[1].ignore_hidden);
    assert_eq!(tree.items[2].item_type, ItemType::Folder);
    assert_eq!(tree.items[2].label, "Docs");
}

#[test]
fn a_type_section_has_its_own_items() {
    let p = profile(
        "[Common]\r\nnFileTreeItemCount=0\r\n[Types(3)]\r\nnFileTreeItemCount=1\r\n\
         FileTree(0).eItemType=1\r\nFileTree(0).szTargetPath=a.txt\r\n",
    );
    assert!(FileTree::from_profile(&p, "Common").items.is_empty());
    let tree = FileTree::from_profile(&p, "Types(3)");
    assert_eq!(tree.items.len(), 1);
    assert_eq!(tree.items[0].target_path, "a.txt");
    // a section without the keys is the default
    assert_eq!(FileTree::from_profile(&p, "Types(4)"), FileTree::default());
}

#[test]
fn the_count_is_held_to_zero_and_twenty() {
    let p = profile("[Common]\r\nnFileTreeItemCount=-5\r\n");
    assert!(FileTree::from_profile(&p, "Common").items.is_empty());
    let p = profile("[Common]\r\nnFileTreeItemCount=99\r\n");
    let tree = FileTree::from_profile(&p, "Common");
    assert_eq!(tree.items.len(), MAX_ITEMS);
    // the first place keeps the default item, the others are empty
    assert_eq!(tree.items[0].target_path, ".");
    assert_eq!(tree.items[1].target_path, "");
    assert_eq!(tree.items[1].target_file, "");
}

#[test]
fn an_unknown_item_type_is_kept_as_it_is() {
    let p = profile("[Common]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=7\r\n");
    let tree = FileTree::from_profile(&p, "Common");
    assert_eq!(tree.items[0].item_type, ItemType::Other(7));
    let mut out = Profile::new();
    tree.write_to(&mut out, "Common").unwrap();
    assert_eq!(out.get("Common", "FileTree(0).eItemType"), Some("7"));
}

#[test]
fn writing_and_reading_again_gives_the_same_tree() {
    let tree = FileTree {
        project: false,
        project_ini: "..\\tree.ini".into(),
        items: vec![
            FileTreeItem {
                item_type: ItemType::Folder,
                label: "ソース".into(),
                ..FileTreeItem::default()
            },
            FileTreeItem {
                item_type: ItemType::Grep,
                target_path: "%MYDOC%\\src".into(),
                label: "src".into(),
                depth: 1,
                target_file: "*.rs;!target".into(),
                ignore_hidden: false,
                ignore_read_only: true,
                ignore_system: true,
            },
            FileTreeItem {
                item_type: ItemType::File,
                target_path: "README.md".into(),
                depth: 1,
                ..FileTreeItem::default()
            },
        ],
    };
    let mut p = Profile::new();
    tree.write_to(&mut p, "Types(2)").unwrap();
    let back = FileTree::from_profile(&Profile::from_bytes(&p.to_bytes()), "Types(2)");
    // fields that are not written for a kind of item come back as the default
    let mut expected = tree.clone();
    expected.items[2].target_file = String::new();
    expected.items[2].ignore_hidden = true;
    // the first place of the file starts as the item of InitFileTree, as in Sakura
    expected.items[0].target_path = ".".into();
    expected.items[0].target_file = "*.*".into();
    assert_eq!(back, expected);
}

#[test]
fn only_the_fields_of_the_kind_are_written() {
    let tree = FileTree {
        project: true,
        project_ini: String::new(),
        items: vec![
            FileTreeItem {
                item_type: ItemType::File,
                target_path: "a.txt".into(),
                ..FileTreeItem::default()
            },
            FileTreeItem {
                item_type: ItemType::Folder,
                label: "F".into(),
                ..FileTreeItem::default()
            },
        ],
    };
    let mut p = Profile::new();
    tree.write_to(&mut p, "Common").unwrap();
    let keys: Vec<&str> = p.entries("Common").map(|(k, _)| k).collect();
    assert_eq!(
        keys,
        [
            "bFileTreeProject",
            "szFileTreeProjectIni",
            "nFileTreeItemCount",
            "FileTree(0).eItemType",
            "FileTree(0).szTargetPath",
            "FileTree(0).nDepth",
            "FileTree(1).eItemType",
            "FileTree(1).szLabelName",
            "FileTree(1).nDepth",
        ]
    );
}

#[test]
fn writing_leaves_the_rest_of_the_file_and_drops_the_old_items() {
    let original = "; my settings\r\n[Common]\r\nnTabWidth=8\r\nnFileTreeItemCount=3\r\n\
                    FileTree(0).eItemType=1\r\nFileTree(0).szTargetPath=x\r\nFileTree(1).eItemType=1\r\n\
                    FileTree(2).eItemType=1\r\nFileTree(2).szTargetPath=z\r\nbWordWrap=1\r\n";
    let mut p = profile(original);
    let tree = FileTree {
        items: vec![FileTreeItem {
            item_type: ItemType::File,
            target_path: "x".into(),
            ..FileTreeItem::default()
        }],
        ..FileTree::default()
    };
    tree.write_to(&mut p, "Common").unwrap();
    let out = text(&p);
    assert!(out.starts_with("; my settings\r\n[Common]\r\nnTabWidth=8\r\n"));
    assert!(out.contains("bWordWrap=1\r\n"));
    assert!(out.contains("nFileTreeItemCount=1\r\n"));
    assert!(!out.contains("FileTree(1)"));
    assert!(!out.contains("FileTree(2)"));
    assert!(out.contains("FileTree(0).szTargetPath=x\r\n"));
}

#[test]
fn writing_the_same_tree_twice_changes_nothing() {
    let mut p = Profile::new();
    let tree = FileTree::default();
    tree.write_to(&mut p, "Common").unwrap();
    let once = p.to_bytes();
    tree.write_to(&mut p, "Common").unwrap();
    assert_eq!(p.to_bytes(), once);
}

#[test]
fn a_project_file_has_the_items_in_the_section_file_tree() {
    let p = profile(
        "[FileTree]\r\nnFileTreeItemCount=2\r\nFileTree(0).eItemType=0\r\n\
         FileTree(0).szTargetPath=<iniroot>\\src\r\nFileTree(0).szTargetFile=*.rs\r\n\
         FileTree(1).eItemType=2\r\nFileTree(1).szLabelName=Docs\r\nFileTree(1).nDepth=0\r\n",
    );
    let items = read_project_items(&p);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].target_path, "<iniroot>\\src");
    // the items of a project file start from SFileTreeItem(), not from the item of InitFileTree
    assert_eq!(items[1].item_type, ItemType::Folder);
    // no count means no items; a negative count too
    assert!(read_project_items(&Profile::new()).is_empty());
    assert!(read_project_items(&profile("[FileTree]\r\nnFileTreeItemCount=-1\r\n")).is_empty());
    // the keys of a section other than [FileTree] are not read
    assert!(
        read_project_items(&profile(
            "[Common]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=1\r\n"
        ))
        .is_empty()
    );
}

#[test]
fn a_project_file_is_written_and_read_back() {
    let items = vec![
        FileTreeItem {
            item_type: ItemType::Grep,
            target_path: "<iniroot>".into(),
            target_file: "*.*".into(),
            ..FileTreeItem::default()
        },
        FileTreeItem {
            item_type: ItemType::Folder,
            label: "x".into(),
            ..FileTreeItem::default()
        },
    ];
    let mut p = Profile::new();
    write_project_items(&mut p, &items).unwrap();
    assert_eq!(p.get("FileTree", "nFileTreeItemCount"), Some("2"));
    assert_eq!(read_project_items(&p), items);
}

#[test]
fn a_huge_count_in_a_project_file_is_cut() {
    let p = profile("[FileTree]\r\nnFileTreeItemCount=2147483647\r\n");
    assert_eq!(read_project_items(&p).len(), MAX_PROJECT_ITEMS);
}

#[test]
fn the_name_of_the_project_file_has_a_default() {
    assert_eq!(
        default_project_ini_name(&Profile::new()),
        "_sakurafiletree.ini"
    );
    let p = profile("[Common]\r\nszFileTreeDefIniName=my.ini\r\n");
    assert_eq!(default_project_ini_name(&p), "my.ini");
}

#[test]
fn the_options_alone_leave_the_items_as_they_are() {
    // `CDlgFileTree::GetData` with a project file: the flag and the file name are written, the
    // items of the settings are not touched.
    let mut p = profile(
        "[Common]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=1\r\n\
         FileTree(0).szTargetPath=a.txt\r\nFileTree(0).nDepth=0\r\n",
    );
    let mut tree = FileTree::from_profile(&p, COMMON_SECTION);
    tree.project = true;
    tree.project_ini = "def.ini".to_owned();
    tree.items.clear();
    tree.write_options_to(&mut p, COMMON_SECTION).unwrap();
    assert_eq!(p.get("Common", "bFileTreeProject"), Some("1"));
    assert_eq!(p.get("Common", "szFileTreeProjectIni"), Some("def.ini"));
    assert_eq!(p.get("Common", "nFileTreeItemCount"), Some("1"));
    assert_eq!(p.get("Common", "FileTree(0).szTargetPath"), Some("a.txt"));
}
