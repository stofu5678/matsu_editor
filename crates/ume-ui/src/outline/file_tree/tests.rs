//! Tests of the file tree without a window: the settings lookup, the tree the items make, and the
//! listing of a folder.

use std::fs;
use std::path::{Path, PathBuf};

use ume_config::Profile;
use ume_config::dialog_dir::MetaSource;
use ume_config::settings::file_tree::{FileTreeItem, ItemType};

use super::list::{Listing, list};
use super::model::{DEFAULT_FOLDER_LABEL, FileTreeModel, Kind, Load, WILD_CARD_ERROR};
use super::source::{Inputs, Loaded, find_project_file, load_setting};

/// Meta folders for the tests: only the folders of the settings and of the program.
struct Meta {
    settings: Option<String>,
    program: Option<String>,
}

impl MetaSource for Meta {
    fn program_dir(&self) -> Option<String> {
        self.program.clone()
    }
    fn settings_dir(&self) -> Option<String> {
        self.settings.clone()
    }
    fn shell_folder(&self, _: &str) -> Option<String> {
        None
    }
    fn env_var(&self, _: &str) -> Option<String> {
        None
    }
}

fn no_meta() -> Meta {
    Meta {
        settings: None,
        program: None,
    }
}

fn dir_with(files: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for file in files {
        let path = dir.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"x").unwrap();
    }
    dir
}

fn item(item_type: ItemType, path: &str, label: &str, depth: i32) -> FileTreeItem {
    FileTreeItem {
        item_type,
        target_path: path.to_owned(),
        label: label.to_owned(),
        depth,
        target_file: "*.*".to_owned(),
        ..FileTreeItem::default()
    }
}

fn loaded(items: Vec<FileTreeItem>) -> Loaded {
    Loaded {
        items,
        ..Loaded::default()
    }
}

fn labels(model: &FileTreeModel, ids: &[usize]) -> Vec<String> {
    ids.iter().map(|&i| model.nodes[i].label.clone()).collect()
}

fn inputs(profile: &str, base: &Path, meta: Meta) -> Inputs {
    Inputs {
        profile: Profile::from_bytes(profile.as_bytes()),
        type_index: 2,
        base_dir: base.to_owned(),
        meta: Box::new(meta),
    }
}

// ---- the tree the items make (SetTreeFile) ----

#[test]
fn folder_items_group_the_items_below_them() {
    let items = vec![
        item(ItemType::Folder, "", "Docs", 0),
        item(ItemType::File, "/x/a.txt", "", 1),
        item(ItemType::Folder, "", "", 1),
        item(ItemType::File, "/x/b.txt", "b", 2),
        item(ItemType::File, "/x/c.txt", "c", 1),
        item(ItemType::File, "/x/d.txt", "d", 0),
    ];
    let model = FileTreeModel::build(&loaded(items), Path::new("/base"), &no_meta());
    // the top: the Folder item "Docs" and the file d
    assert_eq!(labels(&model, &model.roots), ["Docs", "d"]);
    let docs = &model.nodes[model.roots[0]];
    assert_eq!(docs.kind, Kind::Group);
    // under "Docs": a.txt (its label is its full path), the nameless folder, and c
    assert_eq!(
        labels(&model, &docs.children),
        [
            model.nodes[docs.children[0]]
                .path
                .as_ref()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            DEFAULT_FOLDER_LABEL.to_owned(),
            "c".to_owned()
        ]
    );
    assert_eq!(
        model.nodes[docs.children[0]].path,
        Some(PathBuf::from("/x/a.txt"))
    );
    let inner = &model.nodes[docs.children[1]];
    assert_eq!(inner.kind, Kind::Group);
    assert_eq!(labels(&model, &inner.children), ["b"]);
}

#[test]
fn a_depth_beyond_the_folders_hangs_on_the_last_folder_and_a_negative_one_on_the_top() {
    let items = vec![
        item(ItemType::Folder, "", "F", 0),
        // deeper than anything: under F
        item(ItemType::File, "/a", "deep", 5),
        // negative: the top
        item(ItemType::File, "/b", "neg", -3),
    ];
    let model = FileTreeModel::build(&loaded(items), Path::new("/base"), &no_meta());
    assert_eq!(labels(&model, &model.roots), ["F", "neg"]);
    assert_eq!(
        labels(&model, &model.nodes[model.roots[0]].children),
        ["deep"]
    );
}

#[test]
fn paths_are_made_absolute_and_label_the_node_when_it_has_no_label() {
    let items = vec![
        item(ItemType::Grep, ".", "", 0),
        item(ItemType::Grep, "sub/dir", "", 0),
        item(ItemType::File, "notes.txt", "", 0),
        item(ItemType::Grep, "", "", 0),
    ];
    let base = Path::new("/base/proj");
    let model = FileTreeModel::build(&loaded(items), base, &no_meta());
    let root = |i: usize| &model.nodes[model.roots[i]];
    assert_eq!(root(0).path.as_deref(), Some(base));
    assert_eq!(
        root(0).label,
        root(0).path.as_ref().unwrap().to_string_lossy()
    );
    assert_eq!(root(1).path, Some(base.join("sub").join("dir")));
    assert_eq!(root(2).path, Some(base.join("notes.txt")));
    assert_eq!(root(2).kind, Kind::FileItem);
    // a path that names nothing lists nothing
    assert_eq!(root(3).path, None);
    assert_eq!(root(3).load, Load::Done);
    assert!(!root(3).has_children());
    // the others are listed
    assert_eq!(model.pending_roots().len(), 2);
}

#[test]
fn a_backslash_in_a_path_separates_folders_on_every_os() {
    let items = vec![item(ItemType::File, r"sub\dir\a.txt", "", 0)];
    let model = FileTreeModel::build(&loaded(items), Path::new("/base"), &no_meta());
    let expected = Path::new("/base").join("sub").join("dir").join("a.txt");
    assert_eq!(model.nodes[0].path, Some(expected));
}

#[test]
fn an_item_of_an_unknown_type_adds_nothing_but_the_others_stay() {
    let items = vec![
        item(ItemType::Other(9), "/a", "x", 0),
        item(ItemType::File, "/b", "b", 0),
    ];
    let model = FileTreeModel::build(&loaded(items), Path::new("/base"), &no_meta());
    assert_eq!(labels(&model, &model.roots), ["b"]);
}

#[test]
fn iniroot_is_replaced_only_when_a_project_file_gave_the_items() {
    let items = vec![item(ItemType::File, "<iniroot>/a.txt", "", 0)];
    // from the settings: left as it is
    let model = FileTreeModel::build(&loaded(items.clone()), Path::new("/base"), &no_meta());
    assert!(model.nodes[0].label.contains("<iniroot>"));
    // from a project file: the folder of the file
    let found = Loaded {
        items: items.clone(),
        project_ini: Some(PathBuf::from("/proj/_sakurafiletree.ini")),
        ini_root: Some(PathBuf::from("/proj")),
    };
    let model = FileTreeModel::build(&found, Path::new("/base"), &no_meta());
    assert_eq!(model.nodes[0].path, Some(PathBuf::from("/proj/a.txt")));
    // from the project file of the settings: no folder, so an empty text (as in Sakura)
    let default = Loaded {
        items,
        project_ini: Some(PathBuf::from("/set/tree.ini")),
        ini_root: None,
    };
    let model = FileTreeModel::build(&default, Path::new("/base"), &no_meta());
    assert_eq!(model.nodes[0].path, Some(PathBuf::from("/a.txt")));
}

#[test]
fn meta_folders_are_expanded() {
    let items = vec![item(ItemType::File, "%SAKURADATA%/n.txt", "", 0)];
    let meta = Meta {
        settings: Some("/home/me/.config/ume".to_owned()),
        program: None,
    };
    let model = FileTreeModel::build(&loaded(items), Path::new("/base"), &meta);
    assert_eq!(
        model.nodes[0].path,
        Some(PathBuf::from("/home/me/.config/ume/n.txt"))
    );
}

// ---- listing a folder (SetTreeFileSub) ----

fn names(listing: Listing) -> (Vec<String>, Vec<String>) {
    match listing {
        Listing::Entries { folders, files } => (folders, files),
        Listing::WildCardError => panic!("the patterns were refused"),
    }
}

#[test]
fn a_listing_has_the_subfolders_first_then_the_files_in_the_order_of_their_names() {
    let dir = dir_with(&["b.txt", "A.md", "z/in.txt", "Y/in.txt", "c.rs"]);
    let item = item(ItemType::Grep, "", "", 0);
    let (folders, files) = names(list(dir.path(), &item));
    assert_eq!(folders, ["Y", "z"]);
    assert_eq!(files, ["A.md", "b.txt", "c.rs"]);
}

#[test]
fn the_patterns_pick_the_files_and_leave_the_folders_alone() {
    let dir = dir_with(&["a.rs", "b.txt", "日本語.rs", "sub/x.rs", "target/y.rs"]);
    let mut grep = item(ItemType::Grep, "", "", 0);
    grep.target_file = "*.rs".to_owned();
    let (folders, files) = names(list(dir.path(), &grep));
    assert_eq!(folders, ["sub", "target"]);
    assert_eq!(files, ["a.rs", "日本語.rs"]);
    // `!` leaves files out and `#` leaves folders out (as the Grep boxes do)
    grep.target_file = "*.*;!a.*;#target".to_owned();
    let (folders, files) = names(list(dir.path(), &grep));
    assert_eq!(folders, ["sub"]);
    assert_eq!(files, ["b.txt", "日本語.rs"]);
}

#[test]
fn hidden_files_are_left_out_unless_the_item_shows_them() {
    // the name of the shell for a hidden file; the attribute where there is one
    let dir = dir_with(&[".secret", "plain", ".cfg/x"]);
    let mut grep = item(ItemType::Grep, "", "", 0);
    grep.ignore_hidden = true;
    let (folders, files) = names(list(dir.path(), &grep));
    if cfg!(windows) {
        // a dot is no attribute on Windows
        assert_eq!(files, [".secret", "plain"]);
        assert_eq!(folders, [".cfg"]);
    } else {
        assert_eq!(files, ["plain"]);
        assert!(folders.is_empty());
    }
    grep.ignore_hidden = false;
    let (folders, files) = names(list(dir.path(), &grep));
    assert_eq!(files, [".secret", "plain"]);
    assert_eq!(folders, [".cfg"]);
}

#[test]
fn read_only_files_are_left_out_when_asked() {
    let dir = dir_with(&["rw.txt", "ro.txt"]);
    let ro = dir.path().join("ro.txt");
    let mut permissions = fs::metadata(&ro).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&ro, permissions).unwrap();
    let mut grep = item(ItemType::Grep, "", "", 0);
    grep.ignore_read_only = true;
    let (_, files) = names(list(dir.path(), &grep));
    assert_eq!(files, ["rw.txt"]);
    grep.ignore_read_only = false;
    let (_, files) = names(list(dir.path(), &grep));
    assert_eq!(files, ["ro.txt", "rw.txt"]);
}

#[test]
fn patterns_that_cannot_be_used_give_the_error_message() {
    let dir = dir_with(&["a.txt"]);
    // a wild card in the folder part
    let mut grep = item(ItemType::Grep, "", "", 0);
    grep.target_file = r"*\a.txt".to_owned();
    assert_eq!(list(dir.path(), &grep), Listing::WildCardError);
    // a full path to search
    grep.target_file = if cfg!(windows) {
        r"C:\a.txt".to_owned()
    } else {
        "/a.txt".to_owned()
    };
    assert_eq!(list(dir.path(), &grep), Listing::WildCardError);
}

#[test]
fn an_empty_pattern_lists_everything() {
    let dir = dir_with(&["a.txt", "b"]);
    let mut grep = item(ItemType::Grep, "", "", 0);
    grep.target_file = String::new();
    let (_, files) = names(list(dir.path(), &grep));
    assert_eq!(files, ["a.txt", "b"]);
}

#[test]
fn a_folder_that_is_not_there_has_nothing() {
    let grep = item(ItemType::Grep, "", "", 0);
    let (folders, files) = names(list(Path::new("/no/such/folder/for/ume"), &grep));
    assert!(folders.is_empty() && files.is_empty());
}

// ---- lazy expansion ----

#[test]
fn a_folder_is_listed_once_when_it_is_asked_for() {
    let dir = dir_with(&["top.txt", "sub/in.txt", "sub/deeper/x.txt", "empty/.keep"]);
    let mut grep = item(ItemType::Grep, &dir.path().to_string_lossy(), "Project", 0);
    grep.ignore_hidden = true;
    let mut model = FileTreeModel::build(&loaded(vec![grep]), Path::new("/unused"), &no_meta());
    let root = model.roots[0];
    // before the listing: it may have children (the `+`), and nothing is listed yet
    assert!(model.nodes[root].has_children());
    assert!(model.nodes[root].children.is_empty());
    let job = model.begin_load(root).expect("the Grep item is listed");
    // asked again while it is being listed: nothing
    assert!(model.begin_load(root).is_none());
    assert_eq!(model.nodes[root].load, Load::Loading);
    model.finish_load(root, job.run());
    assert_eq!(model.nodes[root].load, Load::Done);
    assert_eq!(
        labels(&model, &model.nodes[root].children),
        ["empty", "sub", "top.txt"]
    );
    assert!(model.begin_load(root).is_none(), "listed once");
    // the subfolders wait until they are opened
    let sub = model.nodes[root].children[1];
    assert_eq!(model.nodes[sub].kind, Kind::Dir);
    assert_eq!(model.nodes[sub].load, Load::Pending);
    assert!(model.nodes[sub].has_children());
    assert!(model.nodes[sub].children.is_empty());
    let job = model.begin_load(sub).unwrap();
    assert_eq!(job.dir, dir.path().join("sub"));
    model.finish_load(sub, job.run());
    assert_eq!(
        labels(&model, &model.nodes[sub].children),
        ["deeper", "in.txt"]
    );
    let file = model.nodes[sub].children[1];
    assert_eq!(
        model.nodes[file].file(),
        Some(dir.path().join("sub").join("in.txt").as_path())
    );
    // a folder with nothing to show loses its `+` (hidden files are left out)
    let empty = model.nodes[root].children[0];
    let job = model.begin_load(empty).unwrap();
    model.finish_load(empty, job.run());
    assert!(!model.nodes[empty].has_children());
}

#[test]
fn a_listing_error_is_a_node_and_files_and_groups_are_listed_by_nobody() {
    let dir = dir_with(&["a.txt"]);
    let mut grep = item(ItemType::Grep, &dir.path().to_string_lossy(), "", 0);
    grep.target_file = r"*\x".to_owned();
    let items = vec![
        grep,
        item(
            ItemType::File,
            &dir.path().join("a.txt").to_string_lossy(),
            "",
            0,
        ),
        item(ItemType::Folder, "", "G", 0),
    ];
    let mut model = FileTreeModel::build(&loaded(items), Path::new("/unused"), &no_meta());
    let (root, file, group) = (model.roots[0], model.roots[1], model.roots[2]);
    assert!(model.begin_load(file).is_none());
    assert!(model.begin_load(group).is_none());
    assert_eq!(
        model.nodes[file].file(),
        Some(dir.path().join("a.txt").as_path())
    );
    assert_eq!(model.nodes[group].file(), None);
    let job = model.begin_load(root).unwrap();
    model.finish_load(root, job.run());
    assert_eq!(
        labels(&model, &model.nodes[root].children),
        [WILD_CARD_ERROR]
    );
    let message = model.nodes[root].children[0];
    assert_eq!(model.nodes[message].kind, Kind::Message);
    assert_eq!(model.nodes[message].file(), None);
}

// ---- which items (LoadFileTreeSetting) ----

#[test]
fn the_default_settings_list_the_folder_of_the_document() {
    let dir = dir_with(&[]);
    let found = load_setting(&inputs("", dir.path(), no_meta()));
    assert_eq!(found.items.len(), 1);
    assert_eq!(found.items[0].target_path, ".");
    assert_eq!(found.items[0].target_file, "*.*");
    assert_eq!(found.project_ini, None);
}

#[test]
fn the_nearest_project_file_above_the_document_gives_the_items() {
    let dir = dir_with(&["a/b/c/x.txt"]);
    let project = |label: &str| {
        format!(
            "[FileTree]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\nFileTree(0).szLabelName={label}\r\n"
        )
    };
    fs::write(dir.path().join("_sakurafiletree.ini"), project("far")).unwrap();
    fs::write(dir.path().join("a/b/_sakurafiletree.ini"), project("near")).unwrap();
    let start = dir.path().join("a/b/c");
    let found = find_project_file(&start, "_sakurafiletree.ini").expect("found");
    assert_eq!(found.items[0].label, "near");
    assert_eq!(found.ini_root, Some(dir.path().join("a/b")));
    assert_eq!(
        found.project_ini,
        Some(dir.path().join("a/b/_sakurafiletree.ini"))
    );
    // the settings use it too, with the default name
    let loaded = load_setting(&inputs("", &start, no_meta()));
    assert_eq!(loaded.items[0].label, "near");
    // not when the setting says no
    let loaded = load_setting(&inputs(
        "[Common]\r\nbFileTreeProject=0\r\n",
        &start,
        no_meta(),
    ));
    assert_eq!(loaded.items[0].target_path, ".");
    // the name of the file is a setting
    fs::write(dir.path().join("a/my.ini"), project("mine")).unwrap();
    let loaded = load_setting(&inputs(
        "[Common]\r\nszFileTreeDefIniName=my.ini\r\n",
        &start,
        no_meta(),
    ));
    assert_eq!(loaded.items[0].label, "mine");
    assert_eq!(loaded.ini_root, Some(dir.path().join("a")));
}

#[test]
fn a_folder_without_a_project_file_has_none() {
    let dir = dir_with(&["x.txt"]);
    // a name nobody has
    assert!(find_project_file(dir.path(), "_no_such_ume_filetree.ini").is_none());
}

#[test]
fn the_default_project_file_is_read_when_no_project_file_is_found() {
    let dir = dir_with(&[]);
    let settings = dir_with(&[]);
    fs::write(
        settings.path().join("tree.ini"),
        "[FileTree]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\nFileTree(0).szLabelName=shared\r\n",
    )
    .unwrap();
    let meta = |settings: &Path| Meta {
        settings: Some(settings.to_string_lossy().into_owned()),
        program: None,
    };
    // a relative name starts at the folder of the settings file
    let text = "[Common]\r\nbFileTreeProject=0\r\nszFileTreeProjectIni=tree.ini\r\n";
    let loaded = load_setting(&inputs(text, dir.path(), meta(settings.path())));
    assert_eq!(loaded.items[0].label, "shared");
    assert_eq!(loaded.project_ini, Some(settings.path().join("tree.ini")));
    assert_eq!(loaded.ini_root, None);
    // an absolute one is used as it is
    let text = format!(
        "[Common]\r\nbFileTreeProject=0\r\nszFileTreeProjectIni={}\r\n",
        settings.path().join("tree.ini").display()
    );
    let loaded = load_setting(&inputs(&text, dir.path(), no_meta()));
    assert_eq!(loaded.items[0].label, "shared");
    // a file that is not there: the items of the settings
    let text = "[Common]\r\nbFileTreeProject=0\r\nszFileTreeProjectIni=missing.ini\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=1\r\nFileTree(0).szTargetPath=q.txt\r\n";
    let loaded = load_setting(&inputs(text, dir.path(), meta(settings.path())));
    assert_eq!(loaded.items[0].target_path, "q.txt");
    assert_eq!(loaded.project_ini, None);
    // the project search comes before the default file
    fs::write(
        dir.path().join("_sakurafiletree.ini"),
        "[FileTree]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\nFileTree(0).szLabelName=local\r\n",
    )
    .unwrap();
    let text = "[Common]\r\nszFileTreeProjectIni=tree.ini\r\n";
    let loaded = load_setting(&inputs(text, dir.path(), meta(settings.path())));
    assert_eq!(loaded.items[0].label, "local");
}

#[test]
fn a_backslash_in_the_name_of_the_default_project_file_separates_folders() {
    let settings = dir_with(&["conf/tree.ini"]);
    fs::write(
        settings.path().join("conf/tree.ini"),
        "[FileTree]\r\nnFileTreeItemCount=1\r\nFileTree(0).eItemType=2\r\nFileTree(0).szLabelName=nested\r\n",
    )
    .unwrap();
    let dir = dir_with(&[]);
    let meta = Meta {
        settings: Some(settings.path().to_string_lossy().into_owned()),
        program: None,
    };
    let text = "[Common]\r\nbFileTreeProject=0\r\nszFileTreeProjectIni=conf\\tree.ini\r\n";
    let loaded = load_setting(&inputs(text, dir.path(), meta));
    assert_eq!(loaded.items[0].label, "nested");
}

#[test]
fn the_settings_of_the_type_are_used_when_the_dock_setting_says_so() {
    let dir = dir_with(&[]);
    let text = "[Common]\r\nbFileTreeProject=0\r\nnOutlineDockSet=1\r\nnFileTreeItemCount=1\r\n\
                FileTree(0).eItemType=1\r\nFileTree(0).szTargetPath=common.txt\r\n\
                [Types(2)]\r\nbFileTreeProject=0\r\nnFileTreeItemCount=1\r\n\
                FileTree(0).eItemType=1\r\nFileTree(0).szTargetPath=type.txt\r\n";
    // the type of the inputs is 2
    let loaded = load_setting(&inputs(text, dir.path(), no_meta()));
    assert_eq!(loaded.items[0].target_path, "type.txt");
    let common = text.replace("nOutlineDockSet=1", "nOutlineDockSet=0");
    let loaded = load_setting(&inputs(&common, dir.path(), no_meta()));
    assert_eq!(loaded.items[0].target_path, "common.txt");
}
