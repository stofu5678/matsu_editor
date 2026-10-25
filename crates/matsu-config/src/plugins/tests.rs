use std::fs;
use std::path::Path;

use super::options::PluginOptions;
use super::*;

const DEF: &str = "[Plugin]\r\nId=jp.example.sample\r\nName=サンプル\r\nDescription=説明\r\nAuthor=作者\r\n\
Version=1.2.3.4\r\nUrl=https://example.com/\r\nType=wsh\r\n\
[Plug]\r\nOutline=outline.js\r\nOutline.Label=見出し\r\nDocumentOpen=open.js\r\nDocumentOpen[1]=open2.js\r\n\
[Command]\r\nC[1]=cmd1.js\r\nC[1].Label=一つ目\r\nC[1].Icon=a.bmp\r\nC[2]=cmd2.js\r\n\
[Option]\r\nO[1].Section=Main\r\nO[1].Key=Level\r\nO[1].Label=レベル\r\nO[1].Type=Int\r\nO[1].Default=3\r\n\
O[2].Key=Mode\r\nO[2].Type=SEL\r\nO[2].Select=自動:0|固定:1|x\r\n\
O[4].Section=Other\r\nO[4].Key=Dir\r\nO[4].Type=dir\r\n\
[String]\r\nS[1]=one\r\nS[3]=three\r\n\
[Wsh]\r\nUseCache=1\r\n";

fn utf8_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn def(text: &str) -> PluginDef {
    PluginDef::from_bytes(&utf8_bom(text), None)
}

fn profile(text: &str) -> Profile {
    Profile::from_bytes(&utf8_bom(text))
}

// ---- plugin.def ----

#[test]
fn the_common_section_and_the_kind_are_read() {
    let d = def(DEF);
    assert_eq!(d.id, "jp.example.sample");
    assert_eq!(d.name, "サンプル");
    assert_eq!(d.description, "説明");
    assert_eq!(d.author, "作者");
    assert_eq!(d.version, "1.2.3.4");
    assert_eq!(d.url, "https://example.com/");
    assert_eq!(d.kind, PluginKind::Wsh);
    assert!(d.use_cache);
    assert_eq!(d.unsupported(), None);
}

#[test]
fn the_kind_ignores_case_and_script_is_the_same_as_wsh() {
    assert_eq!(def("[Plugin]\r\nType=WSH\r\n").kind, PluginKind::Wsh);
    assert_eq!(def("[Plugin]\r\nType=Script\r\n").kind, PluginKind::Script);
    assert_eq!(def("[Plugin]\r\nType=script\r\n").unsupported(), None);
    assert_eq!(def("[Plugin]\r\nType=Dll\r\n").kind, PluginKind::Dll);
}

#[test]
fn a_dll_plugin_a_vbscript_handler_and_an_unknown_kind_are_not_supported() {
    assert_eq!(
        def("[Plugin]\r\nType=dll\r\n[DLL]\r\nName=x.dll\r\n").unsupported(),
        Some(Unsupported::Dll)
    );
    let vbs = def("[Plugin]\r\nType=wsh\r\n[Plug]\r\nOutline=out.VBS\r\n");
    assert_eq!(vbs.unsupported(), Some(Unsupported::VbScript));
    let cmd = def("[Plugin]\r\nType=wsh\r\n[Command]\r\nC[1]=a.js\r\nC[2]=b.vbe\r\n");
    assert_eq!(cmd.unsupported(), Some(Unsupported::VbScript));
    assert_eq!(
        def("[Plugin]\r\nType=ruby\r\n").unsupported(),
        Some(Unsupported::UnknownType("ruby".into()))
    );
    // No type at all is an unknown type too (`LoadPlugin` returns nothing for it).
    assert_eq!(
        def("[Plugin]\r\nId=x\r\n").unsupported(),
        Some(Unsupported::UnknownType(String::new()))
    );
    // A name that only ends like VBScript is not.
    let js = def("[Plugin]\r\nType=wsh\r\n[Plug]\r\nOutline=vbs.js\r\n");
    assert_eq!(js.unsupported(), None);
}

#[test]
fn the_plugs_run_through_each_jack_until_a_key_is_missing() {
    let d = def(DEF);
    let plugs: Vec<_> = d
        .plugs
        .iter()
        .map(|p| (p.jack, p.index, p.handler.as_str(), p.label.as_str()))
        .collect();
    // The order is the jacks' order, and a plug with no label is called by its handler.
    assert_eq!(
        plugs,
        [
            (Jack::DocumentOpen, 0, "open.js", "open.js"),
            (Jack::DocumentOpen, 1, "open2.js", "open2.js"),
            (Jack::Outline, 0, "outline.js", "見出し"),
        ]
    );
    // `Outline[1]` is not read when `Outline` is missing.
    let d = def("[Plug]\r\nOutline[1]=a.js\r\n");
    assert!(d.plugs.is_empty());
}

#[test]
fn the_commands_start_at_one_and_stop_at_a_missing_key() {
    let d = def(DEF);
    assert_eq!(d.command_count(), 2);
    assert_eq!(d.commands[0].index, 1);
    assert_eq!(d.commands[0].label, "一つ目");
    assert_eq!(d.commands[0].icon, "a.bmp");
    assert_eq!(d.commands[1].label, "cmd2.js");
    let gap = def("[Command]\r\nC[1]=a.js\r\nC[3]=c.js\r\n");
    assert_eq!(gap.command_count(), 1);
    // At most 49, and `C[0]` is not one.
    let mut text = String::from("[Command]\r\nC[0]=zero.js\r\n");
    for i in 1..=60 {
        text += &format!("C[{i}]=c{i}.js\r\n");
    }
    assert_eq!(def(&text).command_count(), 49);
    assert_eq!(def(&text).commands[0].handler, "c1.js");
}

#[test]
fn the_install_count_stops_at_an_empty_command_too() {
    let p = profile("[Command]\r\nC[1]=a.js\r\nC[2]=\r\nC[3]=c.js\r\n");
    assert_eq!(PluginDef::install_command_count(&p), 1);
    // `ReadPluginDefCommand` keeps an empty handler and goes on.
    assert_eq!(
        def("[Command]\r\nC[1]=a.js\r\nC[2]=\r\nC[3]=c.js\r\n").command_count(),
        3
    );
}

#[test]
fn the_options_inherit_the_section_and_skip_what_has_no_key() {
    let d = def(DEF);
    let list: Vec<_> = d
        .options
        .iter()
        .map(|o| (o.index, o.section.as_str(), o.key.as_str(), o.kind))
        .collect();
    assert_eq!(
        list,
        [
            (1, "Main", "Level", OptionType::Int),
            // No section: the one before it. The type is read without regard to case.
            (2, "Main", "Mode", OptionType::Sel),
            // O[3] is missing; the numbers go on.
            (4, "Other", "Dir", OptionType::Dir),
        ]
    );
    assert_eq!(d.options[0].label, "レベル");
    // No label: the key.
    assert_eq!(d.options[1].label, "Mode");
    assert_eq!(d.options[0].default, "3");
    assert_eq!(
        d.options[1].selects(),
        [
            ("自動".to_owned(), "0".to_owned()),
            ("固定".to_owned(), "1".to_owned()),
            ("x".to_owned(), "x".to_owned()),
        ]
    );
}

#[test]
fn an_option_without_a_section_or_a_key_is_ignored_and_a_type_that_is_unknown_is_a_string() {
    let d = def(
        "[Option]\r\nO[1].Key=A\r\nO[2].Section=S\r\nO[2].Key=\r\nO[3].Key=C\r\nO[3].Type=zzz\r\n",
    );
    // O[1] has no section, O[2] no key; O[3] takes the section of O[2] (it was read).
    assert_eq!(d.options.len(), 1);
    assert_eq!(d.options[0].key, "C");
    assert_eq!(d.options[0].section, "S");
    assert_eq!(d.options[0].kind, OptionType::Str);
}

#[test]
fn the_choices_of_an_option_are_not_taken_from_the_one_before() {
    // Sakura keeps `sSelect` from one option to the next (an old bug), so an option with no
    // `Select` shows the choices of the previous one. Here it has none (ADR-0026, PR).
    let d = def(
        "[Option]\r\nO[1].Section=S\r\nO[1].Key=A\r\nO[1].Type=Sel\r\nO[1].Select=a|b\r\nO[2].Key=B\r\nO[2].Type=Sel\r\n",
    );
    assert_eq!(d.options[0].selects().len(), 2);
    assert!(d.options[1].selects().is_empty());
}

#[test]
fn the_strings_start_with_a_dummy_and_a_missing_one_is_empty() {
    let d = def(DEF);
    assert_eq!(d.strings.len(), MAX_STRINGS);
    assert_eq!(d.strings[0], "");
    assert_eq!(d.strings[1], "one");
    assert_eq!(d.strings[2], "");
    assert_eq!(d.strings[3], "three");
}

const MAX_STRINGS: usize = def::MAX_PLUG_STRING;

#[test]
fn the_language_file_replaces_the_texts_it_has() {
    let local = "[Plugin]\r\nName=Sample\r\nDescription=desc\r\n\
[Plug]\r\nOutline.Label=Headings\r\n[Command]\r\nC[1].Label=First\r\nC[1].Icon=b.bmp\r\n\
[Option]\r\nO[1].Label=Level\r\nO[2].Select=auto:0|fixed:1\r\n[String]\r\nS[1]=uno\r\n";
    let d = PluginDef::from_bytes(&utf8_bom(DEF), Some(&utf8_bom(local)));
    assert_eq!(d.name, "Sample");
    assert_eq!(d.description, "desc");
    // Not in the language file: the main file's.
    assert_eq!(d.author, "作者");
    assert_eq!(d.url, "https://example.com/");
    let outline = d.plugs.iter().find(|p| p.jack == Jack::Outline).unwrap();
    assert_eq!(outline.label, "Headings");
    assert_eq!(d.commands[0].label, "First");
    assert_eq!(d.commands[0].icon, "b.bmp");
    assert_eq!(d.options[0].label, "Level");
    assert_eq!(d.options[1].selects()[0].0, "auto");
    assert_eq!(d.strings[1], "uno");
    assert_eq!(d.strings[3], "three");
}

#[test]
fn a_definition_in_shift_jis_is_read() {
    let (bytes, _, _) =
        encoding_rs::SHIFT_JIS.encode("[Plugin]\r\nId=a.b\r\nName=日本語の名前\r\nType=wsh\r\n");
    let d = PluginDef::from_bytes(&bytes, None);
    assert_eq!(d.name, "日本語の名前");
    assert_eq!(d.id, "a.b");
}

#[test]
fn a_file_that_is_not_a_definition_gives_an_empty_one() {
    let d = PluginDef::from_bytes(b"\x00\xff garbage \x80\r\nnot=ini", None);
    assert_eq!(d.id, "");
    assert!(d.plugs.is_empty() && d.commands.is_empty() && d.options.is_empty());
    assert_eq!(PluginDef::from_bytes(b"", None).name, "");
}

// ---- [Plugin] of the settings file ----

#[test]
fn the_table_round_trips() {
    let ini = "[Plugin]\r\nEnablePlugin=1\r\nP[00].Name=Sample\r\nP[00].Id=jp.example.sample\r\nP[00].CmdNum=2\r\n\
P[03].Name=Other\r\nP[03].Id=other\r\nP[03].CmdNum=0\r\n[Common]\r\nx=1\r\n";
    let settings = PluginSettings::from_profile(&profile(ini));
    assert!(settings.enabled);
    assert_eq!(settings.table.len(), MAX_PLUGIN);
    assert_eq!(settings.table[0].name, "Sample");
    assert_eq!(settings.table[0].id, "jp.example.sample");
    assert_eq!(settings.table[0].cmd_num, 2);
    assert_eq!(settings.table[0].state, PluginState::Stopped);
    assert_eq!(settings.table[3].state, PluginState::Stopped);
    assert_eq!(settings.table[1].state, PluginState::None);

    // Writing what was read changes nothing.
    let mut again = profile(ini);
    assert!(!settings.store(&mut again).unwrap());
    assert_eq!(again.to_bytes(), profile(ini).to_bytes());

    // A changed table is written and read back.
    let mut changed = settings.clone();
    changed.enabled = false;
    changed.table[1].id = "new".into();
    changed.table[1].name = "New".into();
    changed.table[1].cmd_num = 4;
    changed.table[1].state = PluginState::Installed;
    let mut out = profile(ini);
    assert!(changed.store(&mut out).unwrap());
    let back = PluginSettings::from_profile(&out);
    assert!(!back.enabled);
    assert_eq!(back.table[1].name, "New");
    assert_eq!(back.table[1].cmd_num, 4);
    assert_eq!(back.table[0], settings.table[0]);
    assert_eq!(out.get("Common", "x"), Some("1"));
}

#[test]
fn a_new_file_does_not_use_plugins_and_writes_no_empty_slots() {
    let settings = PluginSettings::from_profile(&Profile::new());
    assert!(!settings.enabled);
    assert!(settings.table.iter().all(|r| r.state == PluginState::None));
    let mut p = Profile::new();
    settings.store(&mut p).unwrap();
    assert_eq!(p.get("Plugin", "EnablePlugin"), Some("0"));
    assert_eq!(p.get("Plugin", "P[00].Id"), None);
}

#[test]
fn a_slot_is_taken_only_when_all_three_keys_are_there() {
    let ini = "[Plugin]\r\nP[00].Name=A\r\nP[00].Id=a\r\n\
P[01].Name=B\r\nP[01].Id=b\r\nP[01].CmdNum=1\r\nP[02].Name=C\r\nP[02].Id=\r\nP[02].CmdNum=3\r\n";
    let s = PluginSettings::from_profile(&profile(ini));
    // No CmdNum: nothing of the slot is taken (`IOProfileData<PluginRec>` returns early).
    assert_eq!(s.table[0], PluginRec::default());
    assert_eq!(s.table[1].id, "b");
    // An empty ID is an empty slot.
    assert_eq!(s.table[2].state, PluginState::None);
}

#[test]
fn a_removed_slot_is_written_empty_and_keeps_its_id_until_then() {
    let ini = "[Plugin]\r\nP[00].Name=A\r\nP[00].Id=a\r\nP[00].CmdNum=1\r\n";
    let mut s = PluginSettings::from_profile(&profile(ini));
    s.uninstall(0);
    assert_eq!(s.table[0].state, PluginState::Deleted);
    assert_eq!(s.table[0].id, "a");
    assert_eq!(s.table[0].name, "");
    let mut p = profile(ini);
    assert!(s.store(&mut p).unwrap());
    assert_eq!(p.get("Plugin", "P[00].Id"), Some(""));
    assert_eq!(p.get("Plugin", "P[00].Name"), Some(""));
    assert_eq!(
        PluginSettings::from_profile(&p).table[0].state,
        PluginState::None
    );
}

#[test]
fn an_id_longer_than_the_slot_is_cut_when_read() {
    let long = "a".repeat(80);
    let ini = format!("[Plugin]\r\nP[00].Name=A\r\nP[00].Id={long}\r\nP[00].CmdNum=0\r\n");
    let s = PluginSettings::from_profile(&profile(&ini));
    assert_eq!(s.table[0].id.chars().count(), ID_MAX_CHARS);
}

// ---- the rules for an ID ----

#[test]
fn the_id_rules() {
    assert_eq!(check_id("jp.example.Bar_Plugin-2"), Ok(()));
    assert_eq!(check_id("{A8E7CC0A-032D-11DF-9E9A-333D56D89593}"), Ok(()));
    for c in RESERVED_CHARS.chars() {
        let id = format!("a{c}b");
        assert_eq!(check_id(&id), Err(InstallError::ReservedChar(c)), "{id:?}");
    }
    assert_eq!(check_id("1abc"), Err(InstallError::IdStartsWithDigit));
    assert_eq!(check_id("a1"), Ok(()));
    assert_eq!(check_id(&"a".repeat(63)), Ok(()));
    assert_eq!(check_id(&"a".repeat(64)), Err(InstallError::IdTooLong));
}

// ---- the folders ----

/// A folder `name` with a `plugin.def` of the id `id`, in `dir`.
fn plugin(dir: &Path, name: &str, id: &str, commands: usize) {
    let folder = dir.join(name);
    fs::create_dir_all(&folder).unwrap();
    let mut text = format!("[Plugin]\r\nId={id}\r\nName={name}\r\nType=wsh\r\n[Command]\r\n");
    for i in 1..=commands {
        text += &format!("C[{i}]=c{i}.js\r\n");
    }
    fs::write(folder.join("plugin.def"), utf8_bom(&text)).unwrap();
}

fn dirs(root: &Path) -> SearchDirs {
    let exe = root.join("exe");
    fs::create_dir_all(&exe).unwrap();
    SearchDirs::new(&root.join("cfg").join("matsu.ini"), Some(&exe))
}

#[test]
fn the_places_are_next_to_the_settings_and_the_executable() {
    let d = SearchDirs::new(Path::new("/a/cfg/matsu.ini"), Some(Path::new("/b/exe")));
    assert_eq!(d.user, Path::new("/a/cfg/plugins"));
    assert_eq!(d.exe.as_deref(), Some(Path::new("/b/exe/plugins")));
    assert_eq!(
        d.option_file("Sample"),
        Path::new("/a/cfg/plugins/Sample.ini")
    );
    // The same folder is looked at once.
    let same = SearchDirs::new(Path::new("/a/matsu.ini"), Some(Path::new("/a")));
    assert_eq!(same.exe, None);
    assert_eq!(same.all().count(), 1);
}

#[test]
fn new_plugins_are_found_in_both_places_without_the_ones_to_skip() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    plugin(&d.user, "Zeta", "z", 0);
    plugin(&d.user, "alpha", "a", 0);
    plugin(&d.user, "unuse", "u", 0);
    plugin(&d.user, ".hidden", "h", 0);
    plugin(&d.user, "Registered", "r", 0);
    fs::create_dir_all(d.user.join("no_def")).unwrap();
    fs::write(d.user.join("file.txt"), "x").unwrap();
    plugin(d.exe.as_deref().unwrap(), "FromExe", "e", 0);
    // The same folder name in both places counts once.
    plugin(d.exe.as_deref().unwrap(), "ZETA", "z2", 0);
    let mut settings = PluginSettings::default();
    settings.table[0] = PluginRec {
        id: "r".into(),
        name: "registered".into(),
        cmd_num: 0,
        state: PluginState::Stopped,
    };
    // The user's folders come first, in name order without regard to case.
    let expected: &[&str] = if cfg!(windows) {
        // A name with a dot is not hidden on Windows (only the attribute hides).
        &[".hidden", "alpha", "Zeta", "FromExe"]
    } else {
        &["alpha", "Zeta", "FromExe"]
    };
    assert_eq!(d.find_new(&settings), expected);
}

#[test]
fn no_plugins_folder_finds_nothing_and_is_made_on_request() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    assert!(d.find_new(&PluginSettings::default()).is_empty());
    assert!(!d.user.exists());
    assert!(d.ensure_user_dir());
    assert!(d.user.is_dir());
}

#[test]
fn a_plugin_is_registered_in_the_first_empty_slot_with_its_commands_counted() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    plugin(&d.user, "One", "id.one", 3);
    plugin(&d.user, "Two", "id.two", 0);
    let mut s = PluginSettings::default();
    assert_eq!(s.install(&d, "One", false), Ok(0));
    assert_eq!(s.install(&d, "Two", false), Ok(1));
    assert_eq!(s.table[0].name, "One");
    assert_eq!(s.table[0].id, "id.one");
    assert_eq!(s.table[0].cmd_num, 3);
    assert_eq!(s.table[0].state, PluginState::Installed);
    assert_eq!(s.find_folder("ONE"), Some(0));
    assert_eq!(s.find_folder("three"), None);
}

#[test]
fn a_plugin_with_the_same_id_asks_and_then_takes_the_slot_of_the_old_one() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    plugin(&d.user, "Old", "same", 1);
    plugin(&d.user, "New", "same", 2);
    let mut s = PluginSettings::default();
    s.install(&d, "Old", false).unwrap();
    assert_eq!(
        s.install(&d, "New", false),
        Err(InstallError::SameId {
            folder: "Old".into()
        })
    );
    assert_eq!(s.install(&d, "New", true), Ok(0));
    assert_eq!(s.table[0].name, "New");
    assert_eq!(s.table[0].cmd_num, 2);
    assert_eq!(s.table[0].state, PluginState::Updated);
}

#[test]
fn a_removed_plugin_comes_back_to_its_slot_without_asking() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    plugin(&d.user, "A", "a", 1);
    plugin(&d.user, "B", "b", 1);
    let mut s = PluginSettings::default();
    s.install(&d, "A", false).unwrap();
    s.install(&d, "B", false).unwrap();
    s.uninstall(0);
    // The slot 0 is not empty yet (`PLS_DELETED`), so a new plugin goes after B.
    plugin(&d.user, "C", "c", 0);
    assert_eq!(s.install(&d, "C", false), Ok(2));
    // The same ID returns to slot 0.
    assert_eq!(s.install(&d, "A", false), Ok(0));
    assert_eq!(s.table[0].state, PluginState::Installed);
}

#[test]
fn the_errors_of_a_registration() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    let mut s = PluginSettings::default();
    assert_eq!(
        s.install(&d, "Missing", false),
        Err(InstallError::NoDefinition)
    );
    plugin(&d.user, "NoId", "", 0);
    assert_eq!(s.install(&d, "NoId", false), Err(InstallError::NoId));
    plugin(&d.user, "Digit", "9x", 0);
    assert_eq!(
        s.install(&d, "Digit", false),
        Err(InstallError::IdStartsWithDigit)
    );
    plugin(&d.user, "Bad", "a b", 0);
    assert_eq!(
        s.install(&d, "Bad", false),
        Err(InstallError::ReservedChar(' '))
    );
    // A folder name that the slot cannot hold is refused, not cut.
    let long = "f".repeat(64);
    plugin(&d.user, &long, "long", 0);
    assert_eq!(
        s.install(&d, &long, false),
        Err(InstallError::FolderNameTooLong)
    );
    // The table is full at 40.
    for i in 0..MAX_PLUGIN {
        plugin(&d.user, &format!("P{i}"), &format!("id{i}"), 0);
        s.install(&d, &format!("P{i}"), false).unwrap();
    }
    plugin(&d.user, "OneMore", "more", 0);
    assert_eq!(
        s.install(&d, "OneMore", false),
        Err(InstallError::TableFull)
    );
}

#[test]
fn the_definition_is_read_with_its_language_file() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    plugin(&d.user, "Sample", "s", 0);
    let local = d.user.join("Sample").join("local");
    fs::create_dir_all(&local).unwrap();
    fs::write(
        local.join("plugin_ja_JP.def"),
        utf8_bom("[Plugin]\r\nName=日本語\r\n"),
    )
    .unwrap();
    fs::write(
        local.join("plugin_en_US.def"),
        utf8_bom("[Plugin]\r\nName=English\r\n"),
    )
    .unwrap();
    assert_eq!(d.load_def("Sample", "ja_JP").unwrap().name, "日本語");
    assert_eq!(d.load_def("Sample", "en_US").unwrap().name, "English");
    assert_eq!(d.load_def("Sample", "fr_FR").unwrap().name, "Sample");
    assert!(d.load_def("Nobody", "ja_JP").is_none());
}

#[test]
fn the_user_folder_wins_over_the_executable_folder() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    plugin(d.exe.as_deref().unwrap(), "Both", "exe", 0);
    plugin(&d.user, "Both", "user", 0);
    assert_eq!(d.load_def("Both", "ja_JP").unwrap().id, "user");
    assert_eq!(d.plugin_dir("Both").unwrap(), d.user.join("Both"));
}

#[test]
fn this_thread_remembers_where_the_plugins_are() {
    assert_eq!(search_dirs(), None);
    let d = SearchDirs::new(Path::new("/x/matsu.ini"), None);
    set_search_dirs(Some(d.clone()));
    assert_eq!(search_dirs(), Some(d));
    set_search_dirs(None);
    assert_eq!(search_dirs(), None);
}

// ---- the options ----

#[test]
fn the_options_are_read_and_written_in_the_plugins_own_file() {
    let root = tempfile::tempdir().unwrap();
    let d = dirs(root.path());
    plugin(&d.user, "Sample", "s", 0);
    let defs = def(DEF).options;
    let file = d.option_file("Sample");
    let mut opts = PluginOptions::load(&file);
    // Nothing in the file: the defaults of the definition.
    assert_eq!(opts.value(&defs[0]), "3");
    assert_eq!(opts.get("Main", "Level"), None);
    opts.set_option(&defs[0], " 12 ").unwrap();
    opts.set("Free", "Key", "値").unwrap();
    opts.save().unwrap();
    let again = PluginOptions::load(&file);
    assert_eq!(again.value(&defs[0]), "12");
    assert_eq!(again.get("Free", "Key"), Some("値"));
    // A number that is not one becomes 0, and a bool is 0 or 1.
    let mut opts = again;
    opts.set_option(&defs[0], "abc").unwrap();
    assert_eq!(opts.value(&defs[0]), "0");
    let mut flag = defs[0].clone();
    flag.kind = OptionType::Bool;
    opts.set_option(&flag, "true").unwrap();
    assert_eq!(opts.value(&flag), "1");
    opts.set_option(&flag, "no").unwrap();
    assert_eq!(opts.value(&flag), "0");
}
