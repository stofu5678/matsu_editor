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
    assert_eq!(row[STATE], "無効");
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
    assert_eq!(row_texts(cx, 0)[STATE], "無効");
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
    assert_eq!(row_texts(cx, 0)[STATE], "無効");
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

// ---- what the user agreed to run (ADR-0026, chapter 4) ----

use super::list::{DISABLE_ONE, ENABLE_ONE, can_disable, can_enable, disable_selected, enable_row};
use super::rows::{CONSENT, MANIFEST, NOTE, can_enable as row_can_enable, write_consent};
use matsu_config::plugins::consent::{Consent, ConsentFile};

fn consent_path(ini: &Ini) -> std::path::PathBuf {
    dir_of(ini).join("plugins.ini")
}

fn enable(cx: &mut TestAppContext, index: usize) -> Result<(), String> {
    with_dialog(cx, |d, _, dcx| {
        let dirs = super::search_dirs(dcx).unwrap();
        select(d, index, dcx);
        enable_row(d, index, &dirs, dcx)
    })
}

#[test]
fn a_plugin_that_was_added_is_off_and_enabling_marks_the_row_with_the_files() {
    let dir = temp();
    put_sample(dir.path(), "HelloJs");
    let dirs = dirs_in(dir.path());
    let mut settings = PluginSettings::default();
    settings.table[0] = registered("HelloJs", HELLO_JS_ID, 1);
    let mut rows = rows_of(&settings, Some(&dirs));
    assert_eq!(rows[0][CONSENT], "off");
    assert!(row_can_enable(&rows[0]));
    rows::set_consent(&mut rows[0], true, "abc");
    assert_eq!(rows[0][STATE], "有効");
    assert_eq!(rows[0][MANIFEST], "abc");
    rows::set_consent(&mut rows[0], false, "");
    assert_eq!(rows[0][STATE], "無効");
    // A slot with nothing, and one that MATSU cannot run, cannot be enabled.
    assert!(!row_can_enable(&rows[5]));
    put_sample(dir.path(), "HelloVbs");
    settings.table[1] = registered("HelloVbs", HELLO_VBS_ID, 1);
    let rows = rows_of(&settings, Some(&dirs));
    assert!(!row_can_enable(&rows[1]));
    assert_eq!(rows[1][NOTE], "VBScript は使えません");
    // A plugin that is not there cannot either.
    settings.table[2] = registered("Gone", "gone", 0);
    assert!(!row_can_enable(&rows_of(&settings, Some(&dirs))[2]));
}

#[test]
fn what_ok_writes_to_plugins_ini_follows_the_rows() {
    let dir = temp();
    put_sample(dir.path(), "HelloJs");
    let dirs = dirs_in(dir.path());
    let mut settings = PluginSettings::default();
    settings.table[0] = registered("HelloJs", HELLO_JS_ID, 1);
    let mut rows = rows_of(&settings, Some(&dirs));
    let mut file = ConsentFile::default();
    // Nothing was done: nothing is written.
    assert!(!write_consent(&rows, &mut file));
    let files =
        matsu_config::plugins::consent::fingerprint(&dirs.plugin_dir("HelloJs").unwrap()).unwrap();
    let manifest = matsu_config::plugins::consent::manifest_text(&files);
    rows::set_consent(&mut rows[0], true, &manifest);
    assert!(write_consent(&rows, &mut file));
    assert_eq!(
        file.consent_for_dir(HELLO_JS_ID, dirs.plugin_dir("HelloJs").as_deref()),
        Consent::On
    );
    rows::set_consent(&mut rows[0], false, "");
    assert!(write_consent(&rows, &mut file));
    assert!(file.record(HELLO_JS_ID).is_none());
    // A removed plugin is forgotten too.
    rows::set_consent(&mut rows[0], true, &manifest);
    write_consent(&rows, &mut file);
    uninstall(&mut rows, 0, Some(&dirs));
    assert!(write_consent(&rows, &mut file));
    assert!(file.record(HELLO_JS_ID).is_none());
}

#[gpui::test]
fn enabling_in_the_dialog_is_kept_by_ok_and_cancel_keeps_nothing(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    run(cx, handle, "F_OPTION_PLUGIN");
    assert_eq!(row_texts(cx, 0)[STATE], "無効");
    assert_eq!(enable(cx, 0), Ok(()));
    assert_eq!(row_texts(cx, 0)[STATE], "有効");
    cancel(cx);
    assert!(!consent_path(&ini).exists(), "キャンセル writes nothing");

    run(cx, handle, "F_OPTION_PLUGIN");
    assert_eq!(enable(cx, 0), Ok(()));
    ok(cx);
    let file = ConsentFile::load(&consent_path(&ini));
    let record = file.record(HELLO_JS_ID).unwrap();
    assert_eq!(record.folder, "HelloJs");
    assert_eq!(record.date.len(), 10);
    let names: Vec<_> = record.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        names,
        [
            "command.js",
            "local/plugin_ja_JP.def",
            "opened.js",
            "plugin.def"
        ]
    );
    // The settings file does not hold the agreement.
    assert!(!String::from_utf8_lossy(&ini.bytes()).contains("Enabled"));

    // The next time, the plugin is on.
    run(cx, handle, "F_OPTION_PLUGIN");
    assert_eq!(row_texts(cx, 0)[STATE], "有効");
    cancel(cx);
}

#[gpui::test]
fn a_changed_file_stops_the_plugin_until_it_is_enabled_again(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    run(cx, handle, "F_OPTION_PLUGIN");
    enable(cx, 0).unwrap();
    ok(cx);

    let script = dir_of(&ini).join("plugins/HelloJs/command.js");
    fs::write(&script, "Editor.InsText('changed');").unwrap();
    run(cx, handle, "F_OPTION_PLUGIN");
    assert_eq!(row_texts(cx, 0)[STATE], "変更あり（停止）");
    assert_eq!(row_texts(cx, 0)[CONSENT], "changed");
    // OK without doing anything keeps the stop (and the old record).
    ok(cx);
    let record = ConsentFile::load(&consent_path(&ini))
        .record(HELLO_JS_ID)
        .unwrap();
    assert!(record.files.iter().any(|f| f.path == "command.js"));

    run(cx, handle, "F_OPTION_PLUGIN");
    with_dialog(cx, |d, _, dcx| select(d, 0, dcx));
    with_dialog(cx, |d, _, _| {
        assert!(can_enable(d.values()) && can_disable(d.values()));
    });
    enable(cx, 0).unwrap();
    assert_eq!(row_texts(cx, 0)[STATE], "有効");
    ok(cx);
    run(cx, handle, "F_OPTION_PLUGIN");
    assert_eq!(row_texts(cx, 0)[STATE], "有効");

    // A file that is added is a change too.
    fs::write(dir_of(&ini).join("plugins/HelloJs/extra.js"), "x").unwrap();
    cancel(cx);
    run(cx, handle, "F_OPTION_PLUGIN");
    assert_eq!(row_texts(cx, 0)[STATE], "変更あり（停止）");
    cancel(cx);
}

#[gpui::test]
fn disabling_forgets_the_agreement_and_the_buttons_follow_the_state(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    put_sample(&dir_of(&ini), "HelloVbs");
    run(cx, handle, "F_OPTION_PLUGIN");
    let usable = |cx: &mut TestAppContext, id: &str| {
        with_dialog(cx, |d, _, _| {
            let control = d.page_controls().into_iter().find(|c| c.id == id).unwrap();
            d.is_usable(&control)
        })
    };
    // Nothing selected, then the plugin that is off, on, and the one MATSU cannot run.
    assert!(!usable(cx, ENABLE_ONE) && !usable(cx, DISABLE_ONE));
    with_dialog(cx, |d, _, dcx| select(d, 0, dcx));
    assert!(usable(cx, ENABLE_ONE) && !usable(cx, DISABLE_ONE));
    enable(cx, 0).unwrap();
    assert!(!usable(cx, ENABLE_ONE) && usable(cx, DISABLE_ONE));
    with_dialog(cx, |d, _, dcx| select(d, 1, dcx));
    assert!(!usable(cx, ENABLE_ONE) && !usable(cx, DISABLE_ONE));
    ok(cx);
    assert!(
        ConsentFile::load(&consent_path(&ini))
            .record(HELLO_JS_ID)
            .is_some()
    );

    run(cx, handle, "F_OPTION_PLUGIN");
    with_dialog(cx, |d, w, dcx| {
        select(d, 0, dcx);
        disable_selected(d, w, dcx);
    });
    assert_eq!(row_texts(cx, 0)[STATE], "無効");
    ok(cx);
    assert!(
        ConsentFile::load(&consent_path(&ini))
            .record(HELLO_JS_ID)
            .is_none()
    );
}

#[gpui::test]
fn adding_a_plugin_does_not_enable_it_and_removing_one_forgets_it(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    run(cx, handle, "F_OPTION_PLUGIN");
    enable(cx, 0).unwrap();
    ok(cx);

    run(cx, handle, "F_OPTION_PLUGIN");
    // Another plugin is added: it is off, and the first one stays on.
    let plugins = dir_of(&ini).join("plugins");
    fs::create_dir_all(plugins.join("Newer")).unwrap();
    fs::write(
        plugins.join("Newer").join("plugin.def"),
        "\u{feff}[Plugin]\r\nId=newer.id\r\nType=script\r\n[Plug]\r\nDocumentBeforeSave=a.js\r\n",
    )
    .unwrap();
    let index = with_dialog(cx, |d, _, dcx| {
        let dirs = super::search_dirs(dcx).unwrap();
        add_folder(d, &dirs, "Newer", false, dcx)
    })
    .unwrap();
    assert_eq!(row_texts(cx, index)[STATE], "追加");
    assert_eq!(row_texts(cx, index)[CONSENT], "off");
    assert_eq!(row_texts(cx, 0)[STATE], "有効");
    // Enabling it shows in the state of the new row.
    enable(cx, index).unwrap();
    assert_eq!(row_texts(cx, index)[STATE], "追加・有効");
    with_dialog(cx, |d, _, dcx| {
        select(d, 0, dcx);
        remove_selected(d, None, dcx);
    });
    ok(cx);
    let file = ConsentFile::load(&consent_path(&ini));
    assert!(file.record(HELLO_JS_ID).is_none());
    assert!(file.record("newer.id").is_some());
}

// ---- a plugin that changes while the user decides, and one that is added again ----

use super::list::enable_row_checked;

#[gpui::test]
fn a_plugin_that_changed_while_the_screen_was_open_is_not_enabled(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    run(cx, handle, "F_OPTION_PLUGIN");
    let dir = dir_of(&ini).join("plugins/HelloJs");
    let shown = matsu_config::plugins::consent::fingerprint(&dir).unwrap();
    // The files are replaced after the user was shown the plugin.
    fs::write(dir.join("command.js"), "Editor.InsText('new hook');").unwrap();
    let result = with_dialog(cx, |d, _, dcx| {
        let dirs = super::search_dirs(dcx).unwrap();
        select(d, 0, dcx);
        enable_row_checked(d, 0, &dirs, Some(&shown), dcx)
    });
    assert!(result.unwrap_err().contains("変わりました"));
    assert_eq!(row_texts(cx, 0)[CONSENT], "off");
    // Shown again with the new files, it can be enabled.
    let shown = matsu_config::plugins::consent::fingerprint(&dir).unwrap();
    let result = with_dialog(cx, |d, _, dcx| {
        let dirs = super::search_dirs(dcx).unwrap();
        enable_row_checked(d, 0, &dirs, Some(&shown), dcx)
    });
    assert_eq!(result, Ok(()));
    assert_eq!(row_texts(cx, 0)[CONSENT], "on");
    cancel(cx);
}

#[gpui::test]
fn a_plugin_that_is_removed_and_added_again_before_ok_is_off(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    run(cx, handle, "F_OPTION_PLUGIN");
    enable(cx, 0).unwrap();
    ok(cx);
    assert!(
        ConsentFile::load(&consent_path(&ini))
            .record(HELLO_JS_ID)
            .is_some()
    );

    run(cx, handle, "F_OPTION_PLUGIN");
    assert_eq!(row_texts(cx, 0)[STATE], "有効");
    with_dialog(cx, |d, _, dcx| {
        select(d, 0, dcx);
        remove_selected(d, None, dcx);
    });
    let index = with_dialog(cx, |d, _, dcx| {
        let dirs = super::search_dirs(dcx).unwrap();
        add_folder(d, &dirs, "HelloJs", false, dcx)
    })
    .unwrap();
    // The agreement of the file does not come back with the folder.
    assert_eq!(row_texts(cx, index)[CONSENT], "off");
    assert_eq!(row_texts(cx, index)[STATE], "追加");
    ok(cx);
    assert!(
        ConsentFile::load(&consent_path(&ini))
            .record(HELLO_JS_ID)
            .is_none(),
        "the cancellation reached plugins.ini"
    );
}
// ---- the host (ADR-0026, PR 3) ----

use super::list::{RUN_TEST, can_run_test};

#[test]
fn a_plugin_that_three_failures_stopped_says_so_and_can_be_enabled_again() {
    let dir = temp();
    put_sample(dir.path(), "HelloJs");
    let dirs = dirs_in(dir.path());
    let files =
        matsu_config::plugins::consent::fingerprint(&dirs.plugin_dir("HelloJs").unwrap()).unwrap();
    let mut consent = ConsentFile::default();
    consent.enable(HELLO_JS_ID, "HelloJs", &files, "d").unwrap();
    consent.save(&dirs.consent_file()).unwrap();
    let mut settings = PluginSettings::default();
    settings.table[0] = registered("HelloJs", HELLO_JS_ID, 1);
    assert_eq!(rows_of(&settings, Some(&dirs))[0][STATE], "有効");
    crate::macros::plugin_call::set_stopped(HELLO_JS_ID, true);
    let rows = rows_of(&settings, Some(&dirs));
    crate::macros::plugin_call::set_stopped(HELLO_JS_ID, false);
    assert_eq!(rows[0][STATE], "停止（失敗が続きました）");
    assert_eq!(rows[0][CONSENT], "stopped");
    assert!(row_can_enable(&rows[0]));
    // The agreement stays in `plugins.ini` when OK writes the rows.
    let mut file = ConsentFile::load(&dirs.consent_file());
    assert!(!write_consent(&rows, &mut file));
    assert!(file.record(HELLO_JS_ID).is_some());
}

#[gpui::test]
fn the_test_run_button_needs_a_plugin_that_can_run_and_has_a_command(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    put_sample(&dir_of(&ini), "HelloVbs");
    run(cx, handle, "F_OPTION_PLUGIN");
    let usable = |cx: &mut TestAppContext| {
        with_dialog(cx, |d, _, _| {
            let control = d
                .page_controls()
                .into_iter()
                .find(|c| c.id == RUN_TEST)
                .unwrap();
            d.is_usable(&control) && can_run_test(d.values())
        })
    };
    assert!(!usable(cx), "nothing is selected");
    with_dialog(cx, |d, _, dcx| select(d, 0, dcx));
    assert!(usable(cx));
    with_dialog(cx, |d, _, dcx| select(d, 1, dcx));
    assert!(!usable(cx), "VBScript cannot run");
    with_dialog(cx, |d, _, dcx| select(d, 2, dcx));
    assert!(!usable(cx), "the folder is gone");
    cancel(cx);
}

// ---- the options of a plugin (ADR-0026, PR 5) ----

use super::list::{OPTION, can_open_options, open_options};
use super::rows::OPTIONS;

#[test]
fn a_row_says_how_many_options_the_plugin_defines() {
    let dir = temp();
    put_sample(dir.path(), "HelloJs");
    let dirs = dirs_in(dir.path());
    let mut settings = PluginSettings::default();
    settings.table[0] = registered("HelloJs", HELLO_JS_ID, 1);
    let rows = rows_of(&settings, Some(&dirs));
    assert_eq!(rows[0][OPTIONS], "1");
    assert_eq!(rows[1][OPTIONS], "");
}

#[gpui::test]
fn the_settings_button_needs_an_enabled_plugin_that_has_options(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    put_sample(&dir_of(&ini), "HelloVbs");
    run(cx, handle, "F_OPTION_PLUGIN");
    let usable = |cx: &mut TestAppContext| {
        with_dialog(cx, |d, _, _| {
            let control = d
                .page_controls()
                .into_iter()
                .find(|c| c.id == OPTION)
                .unwrap();
            d.is_usable(&control) && can_open_options(d.values())
        })
    };
    assert!(!usable(cx), "nothing is selected");
    with_dialog(cx, |d, _, dcx| select(d, 0, dcx));
    assert!(!usable(cx), "the plugin is not enabled");
    with_dialog(cx, |d, _, dcx| {
        let dirs = super::search_dirs(dcx).unwrap();
        enable_row(d, 0, &dirs, dcx).unwrap();
    });
    assert!(usable(cx));
    with_dialog(cx, |d, _, dcx| select(d, 1, dcx));
    assert!(!usable(cx), "VBScript cannot run");
    cancel(cx);
}

#[gpui::test]
fn the_settings_button_opens_the_dialog_of_the_options(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    put_sample(&dir_of(&ini), "HelloJs");
    run(cx, handle, "F_OPTION_PLUGIN");
    with_dialog(cx, |d, window, dcx| {
        select(d, 0, dcx);
        let dirs = super::search_dirs(dcx).unwrap();
        enable_row(d, 0, &dirs, dcx).unwrap();
        open_options(d, window, dcx);
    });
    let (options, _) = cx
        .update(|app| crate::plugin_options::dialog::current(app))
        .expect("the dialog of the options is open");
    options.read_with(cx, |d, _| {
        assert_eq!(d.form().entries.len(), 1);
        assert_eq!(d.form().entries[0].shown(), "Hello");
    });
    cancel(cx);
}
