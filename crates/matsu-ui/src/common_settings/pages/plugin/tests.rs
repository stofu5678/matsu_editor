use std::fs;
use std::path::Path;

use gpui::TestAppContext;
use matsu_config::plugins::{
    InstallError, PluginRec, PluginSettings, PluginState, SearchDirs, set_search_dirs,
};

use super::list::{
    ENABLE, LIST, REMOVE, SEARCH_NEW, SELECTED, URL, add_folder, can_open_url, can_remove, enabled,
    install_error_text, remove_selected, select, web_address,
};
use super::rows::{
    self, AUTHOR, CODE, COMMANDS, DESCRIPTION, FOLDER, ID, NAME, NUMBER, STATE, VERSION, rows_of,
    settings_of, uninstall,
};
use crate::common_settings::model::{Kind, Values, controls};
use crate::common_settings::tests::{Ini, cancel, check, ok, run, setup, with_dialog};
use crate::common_settings::{Page, all_controls};

const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../matsu-plugin/samples/plugins"
);

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap().flatten() {
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Puts the sample plugin `name` into `plugins` of `dir`.
fn put_sample(dir: &Path, name: &str) {
    copy_dir(
        &Path::new(SAMPLES).join(name),
        &dir.join("plugins").join(name),
    );
}

/// The folder of the settings file of the test.
fn dir_of(ini: &Ini) -> std::path::PathBuf {
    ini.path.parent().unwrap().to_owned()
}

fn dirs_in(dir: &Path) -> SearchDirs {
    SearchDirs::new(&dir.join("matsu.ini"), None)
}

fn registered(folder: &str, id: &str, commands: usize) -> PluginRec {
    PluginRec {
        id: id.into(),
        name: folder.into(),
        cmd_num: commands,
        state: PluginState::Stopped,
    }
}

const HELLO_JS_ID: &str = "io.github.stofu5678.matsu-editor.sample.hello-js";
const HELLO_VBS_ID: &str = "io.github.stofu5678.matsu-editor.sample.hello-vbs";

fn temp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

// ---- the rows ----

#[test]
fn an_empty_slot_shows_only_its_number() {
    let rows = rows_of(&PluginSettings::default(), None);
    assert_eq!(rows.len(), 40);
    assert_eq!(rows[0][NUMBER], "0");
    assert_eq!(rows[39][NUMBER], "39");
    assert!(rows[7][NAME].is_empty() && rows[7][STATE].is_empty() && rows[7][FOLDER].is_empty());
    assert_eq!(rows[7][CODE], "none");
}

#[test]
fn a_registered_plugin_shows_what_its_definition_says() {
    let dir = temp();
    put_sample(dir.path(), "HelloJs");
    let dirs = dirs_in(dir.path());
    let mut settings = PluginSettings::default();
    settings.table[2] = registered("HelloJs", HELLO_JS_ID, 1);
    let rows = rows_of(&settings, Some(&dirs));
    let row = &rows[2];
    // The name is the one of the language file (the language is Japanese).
    assert_eq!(row[NAME], "ハロー (JavaScript)");
    assert_eq!(row[STATE], "登録済み");
    assert_eq!(row[ID], HELLO_JS_ID);
    assert_eq!(row[FOLDER], "HelloJs");
    assert_eq!(row[COMMANDS], "1");
    assert_eq!(
        row[DESCRIPTION],
        "ファイルを開いたときにあいさつを書きます。コマンドが 1 つあります。"
    );
    assert_eq!(row[AUTHOR], "MATSU Editor");
    assert_eq!(row[VERSION], "1.0.0.0");
}

#[test]
fn what_matsu_cannot_run_is_said_in_the_state() {
    let dir = temp();
    put_sample(dir.path(), "HelloVbs");
    let dirs = dirs_in(dir.path());
    let mut settings = PluginSettings::default();
    settings.table[0] = registered("HelloVbs", HELLO_VBS_ID, 1);
    let mut added = registered("HelloVbs", "other", 1);
    added.state = PluginState::Installed;
    settings.table[1] = added;
    let rows = rows_of(&settings, Some(&dirs));
    assert_eq!(rows[0][STATE], "VBScript は使えません");
    assert_eq!(rows[1][STATE], "追加（VBScript は使えません）");

    // A DLL plugin and a plugin of a kind that nobody knows.
    let plugins = dir.path().join("plugins");
    for (name, kind) in [("Dll", "dll"), ("Odd", "perl"), ("Typeless", "")] {
        fs::create_dir_all(plugins.join(name)).unwrap();
        fs::write(
            plugins.join(name).join("plugin.def"),
            format!("\u{feff}[Plugin]\r\nId={name}\r\nType={kind}\r\n"),
        )
        .unwrap();
        settings.table[2] = registered(name, name, 0);
        let state = rows_of(&settings, Some(&dirs))[2][STATE].clone();
        let expected = match name {
            "Dll" => "DLL プラグインは使えません",
            "Odd" => "種別「perl」は使えません",
            _ => "種別がありません",
        };
        assert_eq!(state, expected);
    }
}

#[test]
fn a_plugin_whose_folder_is_gone_is_not_found() {
    let dir = temp();
    let dirs = dirs_in(dir.path());
    let mut settings = PluginSettings::default();
    settings.table[0] = registered("Gone", "gone", 2);
    let rows = rows_of(&settings, Some(&dirs));
    assert_eq!(rows[0][NAME], "-");
    assert_eq!(rows[0][STATE], "見つかりません");
    assert_eq!(rows[0][FOLDER], "Gone");
    // With nowhere to look the plugins are not found either.
    assert_eq!(rows_of(&settings, None)[0][STATE], "見つかりません");
}

#[test]
fn the_rows_make_the_table_again() {
    let dir = temp();
    put_sample(dir.path(), "HelloJs");
    let dirs = dirs_in(dir.path());
    let mut settings = PluginSettings {
        enabled: true,
        ..PluginSettings::default()
    };
    settings.table[0] = registered("HelloJs", HELLO_JS_ID, 1);
    settings.table[3] = registered("Gone", "gone", 5);
    settings.table[3].state = PluginState::Updated;
    settings.table[4] = PluginRec {
        id: "removed".into(),
        name: String::new(),
        cmd_num: 0,
        state: PluginState::Deleted,
    };
    let rows = rows_of(&settings, Some(&dirs));
    assert_eq!(settings_of(&rows, true), settings);
    // The check box is not in the rows.
    assert!(!settings_of(&rows, false).enabled);
}

#[test]
fn a_removed_slot_keeps_its_id_and_loses_its_folder() {
    let dir = temp();
    put_sample(dir.path(), "HelloJs");
    let dirs = dirs_in(dir.path());
    let mut settings = PluginSettings::default();
    settings.table[0] = registered("HelloJs", HELLO_JS_ID, 1);
    let mut rows = rows_of(&settings, Some(&dirs));
    uninstall(&mut rows, 0, Some(&dirs));
    assert_eq!(rows[0][STATE], "削除");
    assert_eq!(rows[0][NAME], "-");
    assert_eq!(rows[0][FOLDER], "");
    assert_eq!(rows[0][ID], HELLO_JS_ID);
    assert!(!rows::has_plugin(&rows[0]));
    assert!(rows::has_plugin(&rows_of(&settings, Some(&dirs))[0]));
    assert!(!rows::has_plugin(&rows[1]));
}

#[test]
fn the_page_loads_through_the_places_the_thread_knows() {
    let dir = temp();
    put_sample(dir.path(), "HelloJs");
    let ini =
        "[Plugin]\r\nEnablePlugin=1\r\nP[00].Name=HelloJs\r\nP[00].Id=x\r\nP[00].CmdNum=1\r\n";
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(ini.as_bytes());
    let profile = matsu_config::Profile::from_bytes(&bytes);
    let list = controls(&crate::common_settings::pages::plugin::items().unwrap());
    set_search_dirs(Some(dirs_in(dir.path())));
    let found = Values::load(&list, &profile);
    set_search_dirs(None);
    let lost = Values::load(&list, &profile);
    assert!(found.bool(ENABLE));
    assert_eq!(found.rows(LIST)[0][NAME], "ハロー (JavaScript)");
    assert_eq!(lost.rows(LIST)[0][STATE], "見つかりません");
}

#[test]
fn only_web_addresses_are_opened() {
    assert!(web_address("https://example.com/plugin"));
    assert!(web_address(" HTTP://example.com "));
    assert!(!web_address("file:///etc/passwd"));
    assert!(!web_address("ms-msdt:/id x"));
    assert!(!web_address("example.com"));
    assert!(!web_address(""));
}

#[test]
fn the_error_texts_are_sakuras() {
    assert_eq!(
        install_error_text(&InstallError::NoDefinition),
        "プラグイン定義ファイル（plugin.def）がありません"
    );
    assert_eq!(
        install_error_text(&InstallError::NoId),
        "Plugin.IDがありません"
    );
    assert_eq!(
        install_error_text(&InstallError::IdStartsWithDigit),
        "Plugin.IDの先頭に数字は使用できません"
    );
    assert_eq!(
        install_error_text(&InstallError::TableFull),
        "プラグインをこれ以上登録できません"
    );
    assert!(install_error_text(&InstallError::ReservedChar('/')).starts_with("Plugin.IDに\"/"));
}

// ---- the page ----

const INI: &str = "[Other]\r\nx=1\r\n\
[Plugin]\r\nEnablePlugin=1\r\n\
P[00].Name=HelloJs\r\nP[00].Id=io.github.stofu5678.matsu-editor.sample.hello-js\r\nP[00].CmdNum=1\r\n\
P[01].Name=HelloVbs\r\nP[01].Id=io.github.stofu5678.matsu-editor.sample.hello-vbs\r\nP[01].CmdNum=1\r\n\
P[02].Name=Gone\r\nP[02].Id=gone\r\nP[02].CmdNum=0\r\n\
[Common]\r\nbDispSTATUSBAR=1\r\n";

fn row_texts(cx: &mut TestAppContext, index: usize) -> Vec<String> {
    with_dialog(cx, |d, _, _| d.values().rows(LIST)[index].clone())
}

#[gpui::test]
fn the_page_lists_the_registered_plugins(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    put_sample(&dir_of(&ini), "HelloVbs");
    run(cx, handle, "F_OPTION_PLUGIN");
    assert_eq!(with_dialog(cx, |d, _, _| d.page()), Page::Plugin);
    assert!(!with_dialog(cx, |d, _, _| d.page_controls()).is_empty());
    assert!(with_dialog(cx, |d, _, _| d.values().bool(ENABLE)));
    assert_eq!(with_dialog(cx, |d, _, _| d.values().rows(LIST).len()), 40);
    assert_eq!(row_texts(cx, 0)[STATE], "登録済み");
    assert_eq!(row_texts(cx, 1)[STATE], "VBScript は使えません");
    assert_eq!(row_texts(cx, 2)[STATE], "見つかりません");
    assert_eq!(row_texts(cx, 3)[STATE], "");
    cancel(cx);
}

#[gpui::test]
fn the_buttons_follow_the_check_box_and_the_selected_row(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    run(cx, handle, "F_OPTION_PLUGIN");
    let usable = |cx: &mut TestAppContext, id: &str| {
        with_dialog(cx, |d, _, _| {
            let control = d.page_controls().into_iter().find(|c| c.id == id).unwrap();
            d.is_usable(&control)
        })
    };
    assert!(usable(cx, SEARCH_NEW));
    check(cx, ENABLE, false);
    assert!(!usable(cx, SEARCH_NEW));
    check(cx, ENABLE, true);
    // Nothing is selected: nothing to remove.
    assert!(!usable(cx, REMOVE));
    with_dialog(cx, |d, _, dcx| select(d, 0, dcx));
    assert!(usable(cx, REMOVE));
    // The sample has a web address; the one in the list of the page is read from the file.
    assert!(usable(cx, URL));
    // An empty slot has no plugin to remove.
    with_dialog(cx, |d, _, dcx| select(d, 5, dcx));
    assert!(!usable(cx, REMOVE));
    assert!(!usable(cx, URL));
    with_dialog(cx, |d, _, _| {
        assert!(!can_remove(d.values()) && !can_open_url(d.values()) && enabled(d.values()));
    });
    cancel(cx);
}

#[gpui::test]
fn a_new_plugin_goes_into_the_first_empty_slot_and_ok_writes_it(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    put_sample(&dir_of(&ini), "HelloVbs");
    // A third plugin that the table does not have.
    let plugins = dir_of(&ini).join("plugins");
    fs::create_dir_all(plugins.join("Newer")).unwrap();
    fs::write(
        plugins.join("Newer").join("plugin.def"),
        "\u{feff}[Plugin]\r\nId=newer.id\r\nName=Newer\r\nType=script\r\n[Command]\r\nC[1]=a.js\r\nC[2]=b.js\r\n",
    )
    .unwrap();
    run(cx, handle, "F_OPTION_PLUGIN");
    let found = with_dialog(cx, |d, _, dcx| {
        let dirs = super::search_dirs(dcx).unwrap();
        dirs.find_new(&settings_of(d.values().rows(LIST), true))
    });
    assert_eq!(found, ["Newer"]);
    let index = with_dialog(cx, |d, _, dcx| {
        let dirs = super::search_dirs(dcx).unwrap();
        add_folder(d, &dirs, "Newer", false, dcx)
    });
    assert_eq!(index, Ok(3));
    assert_eq!(row_texts(cx, 3)[STATE], "追加");
    assert_eq!(row_texts(cx, 3)[COMMANDS], "2");
    assert_eq!(with_dialog(cx, |d, _, _| d.values().int(SELECTED)), 3);
    ok(cx);

    let profile = ini.profile();
    assert_eq!(profile.get("Plugin", "P[03].Name"), Some("Newer"));
    assert_eq!(profile.get("Plugin", "P[03].Id"), Some("newer.id"));
    assert_eq!(profile.get("Plugin", "P[03].CmdNum"), Some("2"));
    // What was there stays.
    assert_eq!(profile.get("Plugin", "P[00].Name"), Some("HelloJs"));
    assert_eq!(profile.get("Plugin", "EnablePlugin"), Some("1"));
    assert_eq!(profile.get("Other", "x"), Some("1"));
    assert_eq!(profile.get("Common", "bDispSTATUSBAR"), Some("1"));
}

#[gpui::test]
fn the_same_id_asks_and_the_other_errors_are_reported(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    let plugins = dir_of(&ini).join("plugins");
    fs::create_dir_all(plugins.join("Copy")).unwrap();
    fs::copy(
        plugins.join("HelloJs").join("plugin.def"),
        plugins.join("Copy").join("plugin.def"),
    )
    .unwrap();
    fs::create_dir_all(plugins.join("Empty")).unwrap();
    run(cx, handle, "F_OPTION_PLUGIN");
    let add = |cx: &mut TestAppContext, folder: &'static str, replace: bool| {
        with_dialog(cx, |d, _, dcx| {
            let dirs = super::search_dirs(dcx).unwrap();
            add_folder(d, &dirs, folder, replace, dcx)
        })
    };
    assert_eq!(
        add(cx, "Copy", false),
        Err(InstallError::SameId {
            folder: "HelloJs".into()
        })
    );
    // The list did not change.
    assert_eq!(row_texts(cx, 0)[FOLDER], "HelloJs");
    assert_eq!(add(cx, "Copy", true), Ok(0));
    assert_eq!(row_texts(cx, 0)[FOLDER], "Copy");
    assert_eq!(row_texts(cx, 0)[STATE], "更新");
    assert_eq!(add(cx, "Empty", false), Err(InstallError::NoDefinition));
    cancel(cx);
}

#[gpui::test]
fn removing_a_plugin_empties_its_slot_when_ok_is_pressed(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    run(cx, handle, "F_OPTION_PLUGIN");
    // Nothing is selected: nothing happens.
    with_dialog(cx, |d, _, dcx| remove_selected(d, None, dcx));
    assert_eq!(row_texts(cx, 0)[STATE], "登録済み");
    with_dialog(cx, |d, _, dcx| {
        select(d, 0, dcx);
        remove_selected(d, None, dcx);
    });
    assert_eq!(row_texts(cx, 0)[STATE], "削除");
    ok(cx);
    let profile = ini.profile();
    assert_eq!(profile.get("Plugin", "P[00].Name"), Some(""));
    assert_eq!(profile.get("Plugin", "P[00].Id"), Some(""));
    assert_eq!(profile.get("Plugin", "P[01].Name"), Some("HelloVbs"));
    // The plugin is still on disk: only the registration went.
    assert!(
        dir_of(&ini)
            .join("plugins")
            .join("HelloJs")
            .join("plugin.def")
            .is_file()
    );
}

#[gpui::test]
fn ok_without_a_change_writes_the_table_as_it_was_and_cancel_writes_nothing(
    cx: &mut TestAppContext,
) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    let before = ini.bytes();
    run(cx, handle, "F_OPTION_PLUGIN");
    check(cx, ENABLE, false);
    cancel(cx);
    assert_eq!(ini.bytes(), before);
    run(cx, handle, "F_OPTION_PLUGIN");
    ok(cx);
    let profile = ini.profile();
    assert_eq!(profile.get("Plugin", "EnablePlugin"), Some("1"));
    assert_eq!(profile.get("Plugin", "P[02].Name"), Some("Gone"));
    assert_eq!(profile.get("Plugin", "P[03].Name"), None);
    run(cx, handle, "F_OPTION_PLUGIN");
    check(cx, ENABLE, false);
    ok(cx);
    assert_eq!(ini.profile().get("Plugin", "EnablePlugin"), Some("0"));
}

#[test]
fn every_control_of_the_page_is_known_to_the_dialog() {
    let list = controls(&crate::common_settings::pages::plugin::items().unwrap());
    let ids: Vec<_> = list.iter().map(|c| c.id).collect();
    for id in [ENABLE, SEARCH_NEW, REMOVE, URL, LIST, SELECTED] {
        assert!(ids.contains(&id), "{id}");
    }
    assert!(list.iter().any(|c| matches!(c.kind, Kind::Table { .. })));
    let all = all_controls();
    assert!(all.iter().any(|c| c.id == LIST));
    // The id of every control is unique among the pages.
    let mut seen = std::collections::HashSet::new();
    for control in &all {
        assert!(seen.insert(control.id) || matches!(control.kind, Kind::Radio { .. }));
    }
}
