//! Tests of the commands of the plugins: how they are registered, named, read from and written to the
//! settings, and run. The sample plugin is `HelloCommands` (`matsu-plugin/samples/plugins`), in slot 2 of
//! the plugin table, so that its commands are 20201 and 20202.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui::{TestAppContext, VisualTestContext};
use matsu_commands::keymap::{Chord, Key, Modifiers, Platform, Resolved};
use matsu_commands::plugin_commands as pc;
use matsu_config::Profile;
use matsu_config::plugins::consent::{ConsentFile, fingerprint};
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginRec, PluginSettings, PluginState, SearchDirs};
use matsu_config::settings::CustomMenus;
use matsu_plugin::plugin_host::PluginApi;
use matsu_plugin::{Value, json};

use super::*;
use crate::macros::plugin_call::DefApi;
use crate::macros::plugin_call::tests::{copy_dir, script_host, setup, text_of, wait_for};

const ID: &str = "io.github.stofu5678.matsu-editor.sample.commands";
const SLOT: usize = 2;
const SAMPLES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../matsu-plugin/samples/plugins"
);

struct Place {
    dir: tempfile::TempDir,
}

impl Place {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        copy_dir(
            &Path::new(SAMPLES).join("HelloCommands"),
            &dir.path().join("plugins").join("HelloCommands"),
        );
        Self { dir }
    }

    fn settings(&self) -> PathBuf {
        self.dir.path().join("matsu.ini")
    }

    fn dirs(&self) -> SearchDirs {
        SearchDirs::new(&self.settings(), None)
    }
}

/// What the user did: the switch, the agreement, and any other section of the settings file.
struct Setup<'a> {
    enabled: bool,
    agree: bool,
    extra: &'a str,
}

impl Setup<'_> {
    const ON: Setup<'static> = Setup {
        enabled: true,
        agree: true,
        extra: "",
    };
}

fn user_of(place: &Place, setup: &Setup) -> UserSettings {
    let dirs = place.dirs();
    let def = dirs.load_def("HelloCommands", DEFAULT_LANGUAGE).unwrap();
    let mut settings = PluginSettings {
        enabled: setup.enabled,
        ..PluginSettings::default()
    };
    settings.table[SLOT] = PluginRec {
        id: def.id.clone(),
        name: "HelloCommands".into(),
        cmd_num: 2,
        state: PluginState::Stopped,
    };
    let mut profile = Profile::new();
    settings.store(&mut profile).unwrap();
    let mut text = String::from_utf8(profile.to_bytes()).unwrap();
    text.push_str(setup.extra);
    let profile = Profile::from_bytes(text.as_bytes());
    profile.save(&place.settings()).unwrap();
    if setup.agree {
        let files = fingerprint(&dirs.plugin_dir("HelloCommands").unwrap()).unwrap();
        let mut consent = ConsentFile::default();
        consent
            .enable(&def.id, "HelloCommands", &files, "2026-11-01")
            .unwrap();
        consent.save(&dirs.consent_file()).unwrap();
    } else {
        let _ = fs::remove_file(dirs.consent_file());
    }
    UserSettings::from_profile(profile, Some(place.settings()))
}

/// The user's settings are the application's, and the table of the commands is made from them.
fn install(cx: &mut TestAppContext, place: &Place, setup: &Setup) -> UserSettings {
    let user = user_of(place, setup);
    let global = user.clone();
    cx.update(|app| {
        app.set_global(global);
        refresh(app);
    });
    user
}

fn codes() -> Vec<u16> {
    pc::commands().iter().map(|c| c.code).collect()
}

#[gpui::test]
fn the_commands_of_plugin_def_are_registered_for_an_enabled_plugin_only(cx: &mut TestAppContext) {
    let place = Place::new();
    install(cx, &place, &Setup::ON);
    let first = pc::function_code(SLOT, 1).unwrap();
    assert_eq!((first, codes()), (20201, vec![20201, 20202]));
    let commands = pc::commands();
    assert_eq!(commands[0].label, "Insert the number of this command");
    assert_eq!(commands[0].handler, "insert.js");
    assert_eq!(commands[0].plugin_name, "Commands (JavaScript)");
    assert_eq!(pc::name_of(first).unwrap(), format!("{ID}/01"));
    assert_eq!(pc::by_name(&format!("{ID}/02")), Some(20202));

    // The user did not agree to this plugin: no command, though the plugin is in the table (its name
    // is known, so that a saved entry is written as it was).
    install(
        cx,
        &place,
        &Setup {
            agree: false,
            ..Setup::ON
        },
    );
    assert!(codes().is_empty());
    assert_eq!(pc::name_of(first).unwrap(), format!("{ID}/01"));
    assert_eq!(pc::by_name(&format!("{ID}/01")), None);
    // Plugins are switched off.
    install(
        cx,
        &place,
        &Setup {
            enabled: false,
            ..Setup::ON
        },
    );
    assert!(codes().is_empty());
    // A file of the plugin changed after the user agreed.
    install(cx, &place, &Setup::ON);
    let file = place
        .dirs()
        .plugin_dir("HelloCommands")
        .unwrap()
        .join("insert.js");
    fs::write(&file, "// changed").unwrap();
    cx.update(|app| refresh(app));
    assert!(codes().is_empty());
}

#[gpui::test]
fn plugin_add_command_registers_a_command_with_the_next_number(cx: &mut TestAppContext) {
    let place = Place::new();
    install(cx, &place, &Setup::ON);
    let dirs = place.dirs();
    let def = dirs.load_def("HelloCommands", DEFAULT_LANGUAGE).unwrap();
    let mut api = DefApi::new(&dirs, "HelloCommands", def, 2).in_slot(SLOT);
    let added = api.call(
        "Plugin.AddCommand",
        &[json!("added.js"), json!("Added"), json!("")],
    );
    assert_eq!(added, Ok(Value::Null));
    assert_eq!(codes(), vec![20201, 20202, 20203]);
    let info = pc::info(20203).unwrap();
    assert_eq!(
        (info.label.as_str(), info.handler.as_str()),
        ("Added", "added.js")
    );
    assert!(info.added);
    // The label is the handler when there is none (`CPlugin::ReadPluginDefCommand` does so).
    api.call("Plugin.AddCommand", &[json!("added.js")]).unwrap();
    assert_eq!(pc::label(20204).as_deref(), Some("added.js"));
    // A handler outside the folder of the plugin, and an empty one, are refused: the files of the
    // folder are what the user agreed to.
    for handler in ["../outside.js", "/etc/outside.js", "", "a/../../b.js"] {
        assert!(
            api.call("Plugin.AddCommand", &[json!(handler)]).is_err(),
            "{handler:?}"
        );
    }
    // A plugin that is not in the table cannot add one.
    let def = dirs.load_def("HelloCommands", DEFAULT_LANGUAGE).unwrap();
    let mut loose = DefApi::new(&dirs, "HelloCommands", def, 1);
    assert!(
        loose
            .call("Plugin.AddCommand", &[json!("added.js")])
            .is_err()
    );

    // The commands that were added stay when the settings are read again, and go with the plugin.
    cx.update(|app| refresh(app));
    assert_eq!(codes(), vec![20201, 20202, 20203, 20204]);
    install(
        cx,
        &place,
        &Setup {
            enabled: false,
            ..Setup::ON
        },
    );
    assert!(codes().is_empty());
}

#[gpui::test]
fn a_command_added_at_run_time_gives_way_to_one_the_definition_now_declares(
    cx: &mut TestAppContext,
) {
    let place = Place::new();
    install(cx, &place, &Setup::ON);
    let dirs = place.dirs();
    let def = dirs.load_def("HelloCommands", DEFAULT_LANGUAGE).unwrap();
    let mut api = DefApi::new(&dirs, "HelloCommands", def, 2).in_slot(SLOT);
    api.call(
        "Plugin.AddCommand",
        &[json!("added.js"), json!("Dynamic"), json!("")],
    )
    .unwrap();
    assert_eq!(pc::label(20203).as_deref(), Some("Dynamic"));
    // The plugin is updated: its definition now declares command 3, and the user enables it again.
    let def_file = dirs.def_file("HelloCommands").unwrap();
    let mut text = fs::read_to_string(&def_file).unwrap();
    text = text.replace(
        "C[2].Label=Add a command\n",
        "C[2].Label=Add a command\nC[3]=insert.js\nC[3].Label=Declared\n",
    );
    fs::write(&def_file, text).unwrap();
    install(cx, &place, &Setup::ON);
    let third: Vec<_> = pc::commands()
        .into_iter()
        .filter(|c| c.code == 20203)
        .collect();
    assert_eq!(third.len(), 1, "{third:?}");
    assert_eq!(third[0].label, "Declared");
    assert!(!third[0].added);
    assert_eq!(codes(), vec![20201, 20202, 20203]);
}

#[gpui::test]
fn a_binding_and_a_button_saved_for_an_added_command_work_once_it_is_added(
    cx: &mut TestAppContext,
) {
    let place = Place::new();
    let extra = format!(
        "[KeyBind]\r\nKeyBind[000]=0041,0,0,{ID}/03,0,0,0,0,0,A\r\n\
         [Toolbar]\r\nnToolBarButtonNum=1\r\nnTBB[000]={ID}/03\r\n"
    );
    let user = install(
        cx,
        &place,
        &Setup {
            extra: &extra,
            ..Setup::ON
        },
    );
    // At startup command 3 is not there: the key is unbound and the button is an undefined one.
    cx.update(|app| {
        crate::app::set_key_map(app, user.key_map(Platform::current()));
        crate::bars::toolbar::install(app, &user);
    });
    let bound = |cx: &mut TestAppContext| {
        cx.update(|app| crate::app::with_key_map(app, |map| !map.shortcuts_for(20203).is_empty()))
    };
    let buttons = |cx: &mut TestAppContext| {
        cx.update(|app| {
            app.global::<crate::bars::toolbar::ToolbarButtons>()
                .0
                .clone()
        })
    };
    assert!(!bound(cx));
    assert_eq!(buttons(cx), vec![-1]);

    // The plugin adds the command while it runs: the key and the button work from now on.
    let dirs = place.dirs();
    let def = dirs.load_def("HelloCommands", DEFAULT_LANGUAGE).unwrap();
    let mut api = DefApi::new(&dirs, "HelloCommands", def, 2).in_slot(SLOT);
    api.call("Plugin.AddCommand", &[json!("added.js")]).unwrap();
    cx.update(commands_changed);
    assert!(bound(cx));
    assert_eq!(buttons(cx), vec![20203]);
}

#[gpui::test]
fn a_plugin_can_have_49_commands_and_no_more(cx: &mut TestAppContext) {
    let place = Place::new();
    install(cx, &place, &Setup::ON);
    let dirs = place.dirs();
    let def = dirs.load_def("HelloCommands", DEFAULT_LANGUAGE).unwrap();
    let mut api = DefApi::new(&dirs, "HelloCommands", def, 2).in_slot(SLOT);
    for _ in 3..=49 {
        api.call("Plugin.AddCommand", &[json!("added.js")]).unwrap();
    }
    assert_eq!(codes().len(), 49);
    assert!(api.call("Plugin.AddCommand", &[json!("added.js")]).is_err());
}

const PLUGIN_SECTIONS: &str = "[CustMenu]\r\nnCMIN[03]=3\r\nnCMIF[03][00]=30102\r\n\
     nCMIF[03][01]=io.github.stofu5678.matsu-editor.sample.commands/02\r\nnCMIK[03][01]=P\r\n\
     nCMIF[03][02]=io.github.stofu5678.matsu-editor.sample.commands/09\r\n";

#[gpui::test]
fn a_saved_menu_entry_is_the_command_while_the_plugin_is_enabled_and_text_otherwise(
    cx: &mut TestAppContext,
) {
    let place = Place::new();
    let user = install(
        cx,
        &place,
        &Setup {
            extra: PLUGIN_SECTIONS,
            ..Setup::ON
        },
    );
    let menus = user.custom_menus();
    let items = &menus.get(3).unwrap().items;
    assert_eq!((items[1].function, items[1].key), (20202, Some('P')));
    assert!(items[1].text.is_none());
    // Command 9 does not exist: the text stays.
    assert!(items[2].is_unknown());
    assert_eq!(items[2].text.as_deref(), Some(&*format!("{ID}/09")));

    // Writing the menu gives the name, as Sakura does, not the number.
    let mut profile = Profile::from_bytes(b"");
    let name_of = |code: u16| pc::name_of(code);
    menus
        .write_to_profile(&mut profile, Some(&name_of), None)
        .unwrap();
    assert_eq!(
        profile.get("CustMenu", "nCMIF[03][01]"),
        Some(&*format!("{ID}/02"))
    );
    assert_eq!(
        profile.get("CustMenu", "nCMIF[03][02]"),
        Some(&*format!("{ID}/09"))
    );

    // The plugin is not enabled (the user did not agree): the entries are texts, and saving keeps them.
    let user = install(
        cx,
        &place,
        &Setup {
            agree: false,
            extra: PLUGIN_SECTIONS,
            ..Setup::ON
        },
    );
    let menus = user.custom_menus();
    let items = &menus.get(3).unwrap().items;
    assert!(items[1].is_unknown() && items[2].is_unknown());
    assert_eq!(items[1].text.as_deref(), Some(&*format!("{ID}/02")));
    let mut profile = Profile::from_bytes(b"");
    menus
        .write_to_profile(&mut profile, Some(&name_of), None)
        .unwrap();
    assert_eq!(
        profile.get("CustMenu", "nCMIF[03][01]"),
        Some(&*format!("{ID}/02"))
    );
}

#[gpui::test]
fn the_main_menu_and_the_toolbar_read_a_plugin_command_by_its_name(cx: &mut TestAppContext) {
    let place = Place::new();
    let extra = format!(
        "[MainMenu]\r\nnMainMenuVer=2\r\nnMainMenuNum=3\r\nbKeyParentheses=0\r\n\
         MM[000]=0,0,34052,F,\r\nMM[001]=1,1,{ID}/01,P,\r\nMM[002]=1,1,{ID}/07,Q,\r\n\
         [Toolbar]\r\nnToolBarButtonNum=3\r\nnTBB[000]={ID}/02\r\nnTBB[001]={ID}/08\r\nnTBB[002]=3\r\n"
    );
    let user = install(
        cx,
        &place,
        &Setup {
            extra: &extra,
            ..Setup::ON
        },
    );
    let menu = user.main_menu();
    assert_eq!(menu.rows[1].function, 20201);
    assert!(menu.rows[2].is_unknown(), "{:?}", menu.rows[2]);
    let toolbar = crate::bars::toolbar::ToolbarButtons::from_profile(user.profile().unwrap());
    assert_eq!(toolbar.0, vec![20202, -1, 3]);

    // The menu shows the label of the command, and it can run.
    let bar = crate::menu::main_menu_for(
        &menu,
        &matsu_commands::keymap::KeyMap::default(),
        &crate::menu::Registered,
        &crate::menu::NoLists,
        &CustomMenus::default(),
    );
    let crate::menu::Entry::Item(item) = &bar[0].entries[0] else {
        panic!("{:?}", bar[0].entries);
    };
    assert!(
        item.label.starts_with("Insert the number of this command"),
        "{}",
        item.label
    );
    assert!(item.enabled);
    assert_eq!(item.action, crate::menu::MenuAction::Function(20201));
    // The command that is not there is a grayed-out row.
    let crate::menu::Entry::Item(gone) = &bar[0].entries[1] else {
        panic!();
    };
    assert!(!gone.enabled);

    // The plugin is not enabled: the same file gives the rows as texts.
    let user = install(
        cx,
        &place,
        &Setup {
            agree: false,
            extra: &extra,
            ..Setup::ON
        },
    );
    assert!(user.main_menu().rows[1].is_unknown());
    let toolbar = crate::bars::toolbar::ToolbarButtons::from_profile(user.profile().unwrap());
    assert_eq!(toolbar.0, vec![-1, -1, 3]);
}

#[gpui::test]
fn the_list_of_the_main_menu_has_a_submenu_for_each_plugin_with_its_commands(
    cx: &mut TestAppContext,
) {
    let place = Place::new();
    install(cx, &place, &Setup::ON);
    use crate::menu::{Entry, Lists as _, MenuAction};
    let entries = PluginList.entries(F_PLUGIN_LIST);
    let [Entry::Submenu { label, entries }] = entries.as_slice() else {
        panic!("{entries:?}");
    };
    assert_eq!(label, "Commands (JavaScript)");
    assert_eq!(entries.len(), 2);
    let Entry::Item(second) = &entries[1] else {
        panic!();
    };
    assert_eq!(second.label, "Add a command");
    assert_eq!(second.action, MenuAction::Function(20202));
    // Another row of the menu gets nothing from it, and neither does a menu without commands.
    assert!(PluginList.entries(29005).is_empty());
    install(
        cx,
        &place,
        &Setup {
            enabled: false,
            ..Setup::ON
        },
    );
    assert!(PluginList.entries(F_PLUGIN_LIST).is_empty());
}

#[gpui::test]
fn a_key_binding_of_a_plugin_command_is_read_and_written_by_its_name(cx: &mut TestAppContext) {
    let place = Place::new();
    let extra = format!("[KeyBind]\r\nKeyBind[000]=0041,0,0,{ID}/01,0,0,0,0,0,A\r\n");
    let user = install(
        cx,
        &place,
        &Setup {
            extra: &extra,
            ..Setup::ON
        },
    );
    let ctrl_a = Chord::new(Key::Letter('A'), Modifiers::CTRL);
    let map = user.key_map(Platform::Other);
    assert_eq!(map.resolve(ctrl_a), Resolved::Code(20201));
    assert_eq!(map.shortcuts_for(20201), vec![ctrl_a]);
    let rows =
        matsu_commands::keymap::user::parse_entries(user.profile().unwrap().entries("KeyBind")).0;
    let written = matsu_commands::keymap::user::format_entries(&rows);
    assert_eq!(written[0].1, format!("0041,0,0,{ID}/01,0,0,0,0,0,A"));
    // The plugin is not enabled: the binding is not read (it was not before plugins existed either).
    let user = install(
        cx,
        &place,
        &Setup {
            agree: false,
            extra: &extra,
            ..Setup::ON
        },
    );
    assert_eq!(
        user.key_map(Platform::Other).resolve(ctrl_a),
        Resolved::Unbound
    );
}

fn run_and_wait(
    cx: &mut VisualTestContext,
    view: &gpui::Entity<crate::view::EditorView>,
    code: u16,
    needle: &str,
) -> String {
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    wait_for(view, cx, needle)
}

#[gpui::test]
fn running_a_plugin_command_runs_its_handler_and_one_that_was_added_runs_too(
    cx: &mut TestAppContext,
) {
    let Some(host) = script_host() else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    crate::macros::plugin_call::set_test_interpreter(Some(host));
    let place = Place::new();
    let (view, cx) = setup(cx, "");
    let user = user_of(&place, &Setup::ON);
    cx.update(|_, app| {
        app.set_global(user);
        refresh(app);
    });

    // Command 1 inserts its own number.
    let output = run_and_wait(cx, &view, 20201, "正常に終了しました");
    assert!(output.contains("insert.js"), "{output}");
    assert_eq!(text_of(&view, cx), "command 1");

    // Command 2 adds a command; the new one is listed, and runs its own handler.
    run_and_wait(cx, &view, 20202, "add.js");
    for _ in 0..600 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if pc::info(20203).is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(pc::label(20203).as_deref(), Some("Command added by add.js"));
    let code = pc::by_name(&format!("{ID}/03")).unwrap();
    // The two calls do not overlap: the next waits for the end of the first.
    for _ in 0..200 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if !view.read_with(cx, |v, _| v.macro_output_text().is_empty())
            && !view.read_with(cx, |v, cx| v.plugin_in_call(cx))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    view.update_in(cx, |view, window, cx| {
        view.run_function(code, &[], window, cx)
    });
    for _ in 0..600 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if text_of(&view, cx).contains("added 3") {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(text_of(&view, cx), "command 1added 3");
    crate::macros::plugin_call::set_test_interpreter(None);
    cx.update(|_, app| crate::macros::plugin_call::shutdown(app, ID));
}

#[gpui::test]
fn a_command_of_a_plugin_that_is_not_enabled_says_so_in_the_status_bar(cx: &mut TestAppContext) {
    let place = Place::new();
    let (view, cx) = setup(cx, "");
    let user = user_of(
        &place,
        &Setup {
            agree: false,
            ..Setup::ON
        },
    );
    cx.update(|_, app| {
        app.set_global(user);
        refresh(app);
    });
    view.update_in(cx, |view, window, cx| {
        view.run_function(20201, &[], window, cx)
    });
    cx.run_until_parked();
    let notice = view
        .read_with(cx, |v, _| v.notice.clone())
        .unwrap_or_default();
    assert!(notice.contains("プラグイン"), "{notice}");
    assert_eq!(text_of(&view, cx), "");
}
