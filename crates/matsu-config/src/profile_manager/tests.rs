use std::path::PathBuf;

use super::*;

/// The text as a file that Sakura wrote: UTF-8 with a BOM (a file without one is Shift_JIS).
fn with_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn read_text(text: &str) -> ProfileSettings {
    ProfileSettings::from_profile(&Profile::from_bytes(&with_bom(text)))
}

fn settings(names: &[&str], default_index: i32) -> ProfileSettings {
    ProfileSettings {
        profiles: names.iter().map(|n| (*n).to_owned()).collect(),
        default_index,
        ..ProfileSettings::default()
    }
}

/// The file of `test-cdlgprofilemgr.cpp`, `TrySelectProfile_004`.
const SAKURA_FILE: &str = "[Profile]\nszDllLanguage=\nnDefaultIndex=3\nnCount=3\nP[1]=保存用\nP[2]=鑑賞用\nP[3]=使用用\nbDefaultSelect=1\n";

#[test]
fn reads_the_keys_of_sakuras_file() {
    let s = read_text(SAKURA_FILE);
    assert_eq!(s.profiles, ["保存用", "鑑賞用", "使用用"]);
    assert_eq!(s.default_index, 3);
    assert!(s.default_select);
    assert_eq!(s.dll_language, "");
}

#[test]
fn an_empty_file_has_the_defaults_of_read_prof_settings() {
    // `ReadProfSettings` starts with `nDefaultIndex = 0` and the keys that are missing keep it
    let s = read_text("");
    assert_eq!(s, ProfileSettings::default());
    assert_eq!(s.default_index, 0);
}

#[test]
fn a_default_index_out_of_range_is_none() {
    // `IOProfSettings`: above `nCount` or below -1 is -1
    let index = |text: &str| read_text(text).default_index;
    assert_eq!(index("[Profile]\nnCount=3\nnDefaultIndex=4\n"), -1);
    assert_eq!(index("[Profile]\nnCount=3\nnDefaultIndex=3\n"), 3);
    assert_eq!(index("[Profile]\nnCount=3\nnDefaultIndex=-2\n"), -1);
    assert_eq!(index("[Profile]\nnCount=3\nnDefaultIndex=-1\n"), -1);
    assert_eq!(index("[Profile]\nnCount=0\nnDefaultIndex=0\n"), 0);
}

#[test]
fn a_missing_name_is_an_empty_one_and_a_bad_count_reads_nothing() {
    let s = read_text("[Profile]\nnCount=2\nP[2]=b\n");
    assert_eq!(s.profiles, ["", "b"]);
    assert!(
        read_text("[Profile]\nnCount=-3\nP[1]=a\n")
            .profiles
            .is_empty()
    );
    assert!(
        read_text("[Profile]\nnCount=x\nP[1]=a\n")
            .profiles
            .is_empty()
    );
    // a count that is absurd does not make the reader allocate without end
    let many = read_text("[Profile]\nnCount=2000000000\n");
    assert_eq!(many.profiles.len(), MAX_PROFILES);
}

#[test]
fn the_bool_is_any_number_that_is_not_zero() {
    assert!(read_text("[Profile]\nbDefaultSelect=2\n").default_select);
    assert!(!read_text("[Profile]\nbDefaultSelect=0\n").default_select);
}

#[test]
fn writes_the_keys_of_sakuras_file_and_reads_them_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(FILE_NAME);
    let mut want = settings(&["保存用", "work"], 2);
    want.default_select = true;
    want.dll_language = "sakura_lang_en_US.dll".to_owned();
    write(&path, &want).unwrap();
    assert_eq!(read(&path), Some(want));
    // the keys are Sakura's, in the section `[Profile]`
    let file = Profile::load(&path).unwrap();
    assert_eq!(file.get("Profile", "nCount"), Some("2"));
    assert_eq!(file.get("Profile", "P[1]"), Some("保存用"));
    assert_eq!(file.get("Profile", "P[2]"), Some("work"));
    assert_eq!(file.get("Profile", "nDefaultIndex"), Some("2"));
    assert_eq!(
        file.get("Profile", "szDllLanguage"),
        Some("sakura_lang_en_US.dll")
    );
    assert_eq!(file.get("Profile", "bDefaultSelect"), Some("1"));
    // a new file has the comment on its first line, like Sakura's header
    let text = std::fs::read(&path).unwrap();
    assert!(String::from_utf8_lossy(&text).contains(";MATSU Profile ini"));
}

#[test]
fn writing_a_shorter_list_leaves_no_stale_names_and_keeps_unknown_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(FILE_NAME);
    std::fs::write(&path, with_bom(&format!("{SAKURA_FILE}\n[Other]\nx=1\n"))).unwrap();
    let mut s = read(&path).unwrap();
    s.remove(2);
    write(&path, &s).unwrap();
    let file = Profile::load(&path).unwrap();
    assert_eq!(file.get("Profile", "nCount"), Some("2"));
    assert_eq!(file.get("Profile", "P[2]"), Some("使用用"));
    assert_eq!(file.get("Profile", "P[3]"), None);
    assert_eq!(file.get("Other", "x"), Some("1"));
    assert_eq!(read(&path).unwrap().profiles, ["保存用", "使用用"]);
}

#[test]
fn a_file_that_is_not_there_reads_as_none() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read(&dir.path().join(FILE_NAME)), None);
}

#[test]
fn the_rows_are_the_default_profile_and_the_names() {
    let s = settings(&["a", "b"], 1);
    assert_eq!(s.rows(), 3);
    assert_eq!(s.row_label(0), "(default)");
    assert_eq!(s.row_label(2), "b");
    assert_eq!(s.profile_of(0), "");
    assert_eq!(s.profile_of(2), "b");
    assert!(s.is_default_row(1));
    assert!(!s.is_default_row(0));
    assert!(!settings(&["a"], -1).is_default_row(0));
}

#[test]
fn names_are_checked_in_the_order_of_sakura() {
    let s = settings(&["Work", "memo"], -1);
    assert_eq!(s.check_name("", None), Err(NameError::Empty));
    for bad in [
        "a/b", "a\\b", "a*", "a?", "a<", "a>", "a&", "a|", "a:", "a\"", "a'", "a\tb", "..",
    ] {
        assert_eq!(
            s.check_name(bad, None),
            Err(NameError::InvalidChar),
            "{bad:?}"
        );
    }
    // `.` alone and a name with a dot are not in Sakura's list
    assert_eq!(s.check_name("a.b", None), Ok(()));
    assert_eq!(s.check_name("執筆用 2", None), Ok(()));
    // the same name in another case is the same folder
    assert_eq!(s.check_name("work", None), Err(NameError::Duplicate));
    assert_eq!(s.check_name("MEMO", None), Err(NameError::Duplicate));
    // 「(default)」 is a row of the list
    assert_eq!(s.check_name("(default)", None), Err(NameError::Duplicate));
    assert_eq!(s.check_name("(DEFAULT)", None), Err(NameError::Duplicate));
    // the row that is renamed may take its own name in another case
    assert_eq!(s.check_name("work", Some(1)), Ok(()));
    assert_eq!(s.check_name("memo", Some(1)), Err(NameError::Duplicate));
    // an invalid character is reported before a duplicate
    assert_eq!(s.check_name("work:", None), Err(NameError::InvalidChar));
}

#[test]
fn add_rename_and_remove_move_the_star_with_the_row() {
    let mut s = settings(&["a", "b", "c"], 2);
    assert_eq!(s.add("d"), 4);
    assert_eq!(s.profiles, ["a", "b", "c", "d"]);
    s.rename(2, "bb");
    assert_eq!(s.profile_of(2), "bb");
    assert!(s.is_default_row(2));
    // a row above the star goes: the star stays with `bb`
    assert_eq!(s.remove(1), 1);
    assert_eq!(s.profiles, ["bb", "c", "d"]);
    assert!(s.is_default_row(1));
    // the row with the star goes: no default (`UpdateIni` finds no star)
    assert_eq!(s.remove(1), 1);
    assert_eq!(s.default_index, -1);
    // the last row goes: the row above is chosen
    assert_eq!(s.remove(2), 1);
    assert_eq!(s.profiles, ["c"]);
    // 「(default)」 cannot be removed or renamed
    assert_eq!(s.remove(0), 0);
    s.rename(0, "x");
    assert_eq!(s.profiles, ["c"]);
}

#[test]
fn set_default_and_clear_default() {
    let mut s = settings(&["a"], -1);
    s.set_default(1);
    assert!(s.is_default_row(1));
    s.set_default(0);
    assert!(s.is_default_row(0) && !s.is_default_row(1));
    s.clear_default();
    assert_eq!(s.default_index, -1);
}

// the decision table of `TrySelectProfile`

#[test]
fn profmgr_always_shows_the_dialog() {
    // `TrySelectProfile_001`, and with everything else given
    let file = settings(&["a"], 1);
    assert_eq!(startup(true, None, None), Startup::Dialog);
    assert_eq!(startup(true, Some("x"), Some(&file)), Startup::Dialog);
}

#[test]
fn prof_settles_the_profile_even_when_empty() {
    // `TrySelectProfile_002`
    let file = settings(&["a"], -1);
    assert_eq!(
        startup(false, Some("執筆用"), Some(&file)),
        Startup::Profile("執筆用".into())
    );
    assert_eq!(
        startup(false, Some(""), Some(&file)),
        Startup::Profile(String::new())
    );
}

#[test]
fn no_file_settles_the_profile_without_a_name() {
    // `TrySelectProfile_003`
    assert_eq!(startup(false, None, None), Startup::Profile(String::new()));
}

#[test]
fn the_default_index_picks_the_row() {
    // `TrySelectProfile_004`: index 3 of 3 names
    let file = read_text(SAKURA_FILE);
    assert_eq!(
        startup(false, None, Some(&file)),
        Startup::Profile("使用用".into())
    );
    assert_eq!(
        startup(false, None, Some(&settings(&["a", "b"], 1))),
        Startup::Profile("a".into())
    );
}

#[test]
fn a_default_index_that_is_wrong_or_none_shows_the_dialog() {
    // `TrySelectProfile_005`: 4 of 3 names is read as -1
    let file = read_text("[Profile]\nnCount=3\nnDefaultIndex=4\nP[1]=a\nP[2]=b\nP[3]=c\n");
    assert_eq!(startup(false, None, Some(&file)), Startup::Dialog);
    // `TrySelectProfile_006`: an empty definition
    let file = read_text("[Profile]\nszDllLanguage=\nnDefaultIndex=-1\nbDefaultSelect=0\n");
    assert_eq!(startup(false, None, Some(&file)), Startup::Dialog);
}

#[test]
fn the_default_profile_chosen_as_the_default_starts_without_the_dialog() {
    // the one rule that differs from Sakura (see `startup`)
    let file = read_text("[Profile]\nnCount=1\nnDefaultIndex=0\nP[1]=a\n");
    assert_eq!(
        startup(false, None, Some(&file)),
        Startup::Profile(String::new())
    );
}

fn env() -> Environment {
    Environment {
        exe_dir: Some(PathBuf::from("/app")),
        home: Some(PathBuf::from("/home/u")),
        app_data: Some(PathBuf::from("/appdata")),
        xdg_config_home: None,
        profile_name: None,
    }
}

#[test]
fn the_file_is_next_to_the_settings_file_and_a_profile_goes_up_one_folder() {
    let mut e = env();
    let linux = settings_path(Os::Linux, &e).unwrap();
    assert_eq!(
        linux,
        PathBuf::from("/home/u/.config/matsu-editor/matsu_prof.ini")
    );
    // with `-PROF` the settings file is in a sub folder, the manager's file is not
    e.profile_name = Some("work".to_owned());
    assert_eq!(settings_path(Os::Linux, &e).unwrap(), linux);
    assert_eq!(
        profile_dir(Os::Linux, &e, "memo").unwrap(),
        PathBuf::from("/home/u/.config/matsu-editor/memo")
    );
    assert_eq!(
        settings_path(Os::MacOs, &e).unwrap(),
        PathBuf::from("/home/u/Library/Application Support/matsu-editor/matsu_prof.ini")
    );
    // Windows: the user's own folder unless a `matsu.ini` is next to the program (nothing is there)
    assert_eq!(
        settings_path(Os::Windows, &e).unwrap(),
        PathBuf::from("/appdata/matsu-editor/matsu_prof.ini")
    );
}

#[test]
fn the_folder_of_an_existing_settings_file_is_the_one_used() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("matsu.ini"), "").unwrap();
    let e = Environment {
        exe_dir: Some(dir.path().to_path_buf()),
        app_data: Some(PathBuf::from("/appdata")),
        ..Environment::default()
    };
    assert_eq!(
        settings_path(Os::Windows, &e).unwrap(),
        dir.path().join(FILE_NAME)
    );
    // a profile that has no `matsu.ini` yet starts in the user's own folder, as `UserSettings` does,
    // and the manager looks there (the folder next to the program has none of it)
    assert_eq!(
        profile_dir(Os::Windows, &e, "p").unwrap(),
        PathBuf::from("/appdata/matsu-editor/p")
    );
    // a profile that has its `matsu.ini` next to the program is there
    std::fs::create_dir(dir.path().join("p")).unwrap();
    std::fs::write(dir.path().join("p").join("matsu.ini"), "").unwrap();
    assert_eq!(
        profile_dir(Os::Windows, &e, "p").unwrap(),
        dir.path().join("p")
    );
    // and it is the folder of the file `UserSettings` reads for that profile
    let with_profile = Environment {
        profile_name: Some("p".to_owned()),
        ..e.clone()
    };
    assert_eq!(
        crate::location::settings_file(Os::Windows, &with_profile).unwrap(),
        dir.path().join("p").join("matsu.ini")
    );
}

#[test]
fn one_rule_for_the_names_of_the_manager_and_of_prof() {
    let s = settings(&[], -1);
    for name in [".", "..", "a\u{7}b", "a/b", "", "x".repeat(260).as_str()] {
        assert!(!valid_folder_name(name), "{name:?}");
    }
    // the manager refuses what `-PROF=` cannot select
    assert_eq!(s.check_name(".", None), Err(NameError::InvalidChar));
    assert_eq!(s.check_name("a\u{7}b", None), Err(NameError::InvalidChar));
    for name in ["work", "a.b", "執筆用 2", "x y"] {
        assert!(valid_folder_name(name), "{name:?}");
        assert_eq!(s.check_name(name, None), Ok(()), "{name:?}");
    }
}
