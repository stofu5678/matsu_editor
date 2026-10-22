use std::collections::{HashMap, HashSet};

use super::*;

#[derive(Default)]
struct Source {
    program: Option<String>,
    settings: Option<String>,
    shell: HashMap<&'static str, String>,
    env: HashMap<&'static str, String>,
}

impl MetaSource for Source {
    fn program_dir(&self) -> Option<String> {
        self.program.clone()
    }
    fn settings_dir(&self) -> Option<String> {
        self.settings.clone()
    }
    fn shell_folder(&self, name: &str) -> Option<String> {
        self.shell
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.clone())
    }
    fn env_var(&self, name: &str) -> Option<String> {
        self.env.get(name).cloned()
    }
}

fn source() -> Source {
    let mut source = Source {
        program: Some(r"C:\Program Files\MATSU".to_owned()),
        settings: Some(r"C:\Users\a\AppData\Roaming\matsu\".to_owned()),
        ..Source::default()
    };
    source
        .shell
        .insert("Personal", r"C:\Users\a\Documents".to_owned());
    source
        .shell
        .insert("My Pictures", r"C:\Users\a\Pictures\".to_owned());
    source
        .shell
        .insert("Video", r"C:\Users\a\Videos".to_owned());
    source.env.insert("WORK", r"D:\work".to_owned());
    source
}

fn expand(text: &str) -> (String, bool) {
    let result = expand_meta(text, &source());
    (result.text, result.complete)
}

#[test]
fn text_without_meta_stays() {
    assert_eq!(
        expand("C:\\data\\メモ"),
        ("C:\\data\\メモ".to_owned(), true)
    );
    assert_eq!(expand(""), (String::new(), true));
}

#[test]
fn percent_percent_is_a_percent() {
    assert_eq!(expand("100%%"), ("100%".to_owned(), true));
    assert_eq!(expand("%%MYDOC%%"), ("%MYDOC%".to_owned(), true));
}

#[test]
fn the_aliases_and_the_names_of_the_registry_are_folders() {
    assert_eq!(
        expand(r"%MYDOC%\a"),
        (r"C:\Users\a\Documents\a".to_owned(), true)
    );
    assert_eq!(
        expand(r"%Personal%\a"),
        (r"C:\Users\a\Documents\a".to_owned(), true)
    );
    assert_eq!(
        expand(r"%mydoc%"),
        (r"C:\Users\a\Documents".to_owned(), true)
    );
    assert_eq!(expand("%MYVIDEO%"), (r"C:\Users\a\Videos".to_owned(), true));
}

#[test]
fn the_last_backslash_of_the_folder_is_cut() {
    // `%Personal%\` is Sakura's default: the folder gets its own backslash from the text
    assert_eq!(
        expand(r"%Personal%\"),
        (r"C:\Users\a\Documents\".to_owned(), true)
    );
    assert_eq!(
        expand("%MYPICT%"),
        (r"C:\Users\a\Pictures".to_owned(), true)
    );
}

#[test]
fn sakura_and_sakuradata_are_the_program_and_the_settings() {
    assert_eq!(
        expand(r"%SAKURA%\x"),
        (r"C:\Program Files\MATSU\x".to_owned(), true)
    );
    assert_eq!(
        expand("%sakuradata%"),
        (r"C:\Users\a\AppData\Roaming\matsu".to_owned(), true)
    );
}

#[test]
fn an_environment_variable_is_the_next_choice() {
    assert_eq!(expand(r"%WORK%\src"), (r"D:\work\src".to_owned(), true));
}

#[test]
fn an_unknown_name_stays_as_typed() {
    assert_eq!(expand(r"%NOPE%\a"), (r"%NOPE%\a".to_owned(), true));
}

#[test]
fn a_percent_without_a_closing_one_is_a_percent() {
    assert_eq!(expand("50% done"), ("50% done".to_owned(), true));
    assert_eq!(expand("a%"), ("a%".to_owned(), true));
    assert_eq!(expand("%MYDOC"), ("%MYDOC".to_owned(), true));
}

#[test]
fn a_meta_folder_that_is_empty_stops_the_expansion() {
    let mut empty = source();
    empty.program = None;
    let result = expand_meta(r"x\%SAKURA%\y", &empty);
    assert_eq!(result.text, r"x\");
    assert!(!result.complete);
}

#[test]
fn the_setting_is_read_as_sakura_names_it() {
    let profile = Profile::from_bytes(
        "[Common]\neOpenDialogDir=2\nszOpenDialogSelDir=%MYDOC%\\memo\n".as_bytes(),
    );
    let setting = DialogDirSetting::from_profile(&profile);
    assert_eq!(setting.mode, OpenDialogDir::Selected);
    assert_eq!(setting.selected, "%MYDOC%\\memo");
}

#[test]
fn the_default_setting_is_the_current_folder_and_personal() {
    let setting = DialogDirSetting::from_profile(&Profile::new());
    assert_eq!(setting.mode, OpenDialogDir::Current);
    assert_eq!(setting.selected, r"%Personal%\");
    // a number Sakura does not know is the default too
    let profile = Profile::from_bytes(b"[Common]\neOpenDialogDir=7\n");
    assert_eq!(
        DialogDirSetting::from_profile(&profile).mode,
        OpenDialogDir::Current
    );
    for mode in [
        OpenDialogDir::Current,
        OpenDialogDir::Recent,
        OpenDialogDir::Selected,
    ] {
        assert_eq!(OpenDialogDir::from_code(mode.code()), Some(mode));
    }
}

/// The folders that exist in the tests below.
fn folders(list: &[&str]) -> HashSet<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

fn start(
    setting: &DialogDirSetting,
    document: Option<&str>,
    recent: &[&str],
    current: Option<&str>,
    existing: &[&str],
) -> Option<PathBuf> {
    let existing = folders(existing);
    let recent: Vec<String> = recent.iter().map(|s| (*s).to_owned()).collect();
    let is_dir = |path: &Path| existing.contains(path);
    let source = source();
    let document = document.map(PathBuf::from);
    setting.initial_dir(&Surroundings {
        document: document.as_deref(),
        recent_folders: &recent,
        current_dir: current.map(PathBuf::from),
        meta: &source,
        is_dir: &is_dir,
    })
}

fn setting(mode: OpenDialogDir, selected: &str) -> DialogDirSetting {
    DialogDirSetting {
        mode,
        selected: selected.to_owned(),
    }
}

#[test]
fn a_document_with_a_file_starts_in_its_folder_whatever_the_setting() {
    for mode in [
        OpenDialogDir::Current,
        OpenDialogDir::Recent,
        OpenDialogDir::Selected,
    ] {
        let got = start(
            &setting(mode, "/sel"),
            Some("/work/memo/a.txt"),
            &["/recent"],
            Some("/cwd"),
            &["/work/memo", "/recent", "/cwd", "/sel"],
        );
        assert_eq!(got, Some(PathBuf::from("/work/memo")), "{mode:?}");
    }
}

#[test]
fn a_folder_of_the_file_that_is_gone_leaves_the_choice_to_the_os() {
    let got = start(
        &setting(OpenDialogDir::Current, ""),
        Some("/gone/a.txt"),
        &[],
        Some("/cwd"),
        &["/cwd"],
    );
    assert_eq!(got, None);
}

#[test]
fn current_is_the_current_folder() {
    let setting = setting(OpenDialogDir::Current, "/sel");
    assert_eq!(
        start(
            &setting,
            None,
            &["/recent"],
            Some("/cwd"),
            &["/cwd", "/recent"]
        ),
        Some(PathBuf::from("/cwd"))
    );
    assert_eq!(start(&setting, None, &[], None, &[]), None);
}

#[test]
fn recent_is_the_first_recent_folder_that_exists() {
    let setting = setting(OpenDialogDir::Recent, "/sel");
    assert_eq!(
        start(
            &setting,
            None,
            &["/gone", "/b", "/c"],
            Some("/cwd"),
            &["/b", "/c", "/cwd"]
        ),
        Some(PathBuf::from("/b"))
    );
}

#[test]
fn recent_without_a_folder_that_exists_is_the_current_folder() {
    let setting = setting(OpenDialogDir::Recent, "/sel");
    assert_eq!(
        start(&setting, None, &["/gone"], Some("/cwd"), &["/cwd"]),
        Some(PathBuf::from("/cwd"))
    );
    assert_eq!(
        start(&setting, None, &[], Some("/cwd"), &["/cwd"]),
        Some(PathBuf::from("/cwd"))
    );
}

#[test]
fn selected_is_the_given_folder_with_its_meta_folders_expanded() {
    let setting = setting(OpenDialogDir::Selected, "%WORK%/src");
    // the environment variable of the test source is a Windows path, so use a plain one
    let mut with_env = source();
    with_env.env.insert("WORK", "/home/a/work".to_owned());
    let existing = folders(&["/home/a/work/src", "/cwd"]);
    let is_dir = |path: &Path| existing.contains(path);
    let got = setting.initial_dir(&Surroundings {
        document: None,
        recent_folders: &[],
        current_dir: Some(PathBuf::from("/cwd")),
        meta: &with_env,
        is_dir: &is_dir,
    });
    assert_eq!(got, Some(PathBuf::from("/home/a/work/src")));
}

#[test]
fn selected_that_does_not_exist_is_left_to_the_os() {
    let setting = setting(OpenDialogDir::Selected, "/gone");
    assert_eq!(
        start(&setting, None, &["/r"], Some("/cwd"), &["/r", "/cwd"]),
        None
    );
}

#[cfg(not(windows))]
#[test]
fn backslashes_of_the_setting_are_separators_on_other_systems() {
    let mut meta = source();
    meta.shell
        .insert("Personal", "/home/a/Documents".to_owned());
    let existing = folders(&["/home/a/Documents/メモ"]);
    let is_dir = |path: &Path| existing.contains(path);
    let setting = setting(OpenDialogDir::Selected, r"%Personal%\メモ");
    let got = setting.initial_dir(&Surroundings {
        document: None,
        recent_folders: &[],
        current_dir: None,
        meta: &meta,
        is_dir: &is_dir,
    });
    assert_eq!(got, Some(PathBuf::from("/home/a/Documents/メモ")));
}

#[test]
fn the_default_name_of_fileopen_gives_the_folder_and_the_file() {
    let existing = folders(&["/work/", "/work/sub/"]);
    let is_dir = |path: &Path| existing.contains(path);
    // a folder
    assert_eq!(
        split_default_name("/work/sub/", &is_dir),
        (Some(PathBuf::from("/work/sub/")), None)
    );
    // a file in a folder that exists
    assert_eq!(
        split_default_name("/work/メモ.txt", &is_dir),
        (Some(PathBuf::from("/work/")), Some("メモ.txt".to_owned()))
    );
    // a folder that does not exist: neither
    assert_eq!(split_default_name("/gone/a.txt", &is_dir), (None, None));
    // a name alone
    assert_eq!(
        split_default_name("a.txt", &is_dir),
        (None, Some("a.txt".to_owned()))
    );
    assert_eq!(split_default_name("", &is_dir), (None, None));
}

#[cfg(not(windows))]
#[test]
fn the_default_name_may_use_backslashes() {
    let existing = folders(&["C:/work/"]);
    let is_dir = |path: &Path| existing.contains(path);
    assert_eq!(
        split_default_name(r"C:\work\a.txt", &is_dir),
        (Some(PathBuf::from("C:/work/")), Some("a.txt".to_owned()))
    );
}

#[cfg(not(windows))]
#[test]
fn the_macro_folder_is_taken_from_the_settings_then_the_program() {
    let settings = Path::new("/home/a/.config/matsu");
    let program = Path::new("/opt/matsu");
    let existing = folders(&["/home/a/.config/matsu/macros", "/opt/matsu/macro"]);
    let is_dir = |path: &Path| existing.contains(path);
    let folder =
        |setting: Option<&str>| macro_folder(setting, Some(settings), Some(program), &is_dir);
    // not written: the folder of the settings file
    assert_eq!(folder(None), Some(settings.to_owned()));
    // an empty value is the same folder
    assert_eq!(folder(Some("")), Some(settings.to_owned()));
    // an absolute folder as it is, whether it exists or not
    assert_eq!(
        folder(Some("/srv/macros")),
        Some(PathBuf::from("/srv/macros"))
    );
    // relative: the settings first, then the program
    assert_eq!(
        folder(Some("macros")),
        Some(PathBuf::from("/home/a/.config/matsu/macros"))
    );
    assert_eq!(
        folder(Some("macro")),
        Some(PathBuf::from("/opt/matsu/macro"))
    );
    // neither: the settings again
    assert_eq!(
        folder(Some("none\\here")),
        Some(PathBuf::from("/home/a/.config/matsu/none/here"))
    );
    assert_eq!(macro_folder(None, None, Some(program), &is_dir), None);
}

#[cfg(not(windows))]
#[test]
fn the_key_macro_file_is_reckey_mac_in_the_macro_folder() {
    let settings = Path::new("/home/a/.config/matsu");
    let program = Path::new("/opt/matsu");
    let existing = folders(&["/home/a/.config/matsu/macros", "/opt/matsu/macro"]);
    let is_dir = |path: &Path| existing.contains(path);
    let file =
        |setting: Option<&str>| key_macro_file(setting, Some(settings), Some(program), &is_dir);
    // an empty or missing folder: the folder of the settings file
    let in_settings = Some(PathBuf::from("/home/a/.config/matsu/RecKey.mac"));
    assert_eq!(file(None), in_settings);
    assert_eq!(file(Some("")), in_settings);
    // absolute
    assert_eq!(
        file(Some("/srv/macros")),
        Some(PathBuf::from("/srv/macros/RecKey.mac"))
    );
    // relative: from the settings, else from the program
    assert_eq!(
        file(Some("macros")),
        Some(PathBuf::from("/home/a/.config/matsu/macros/RecKey.mac"))
    );
    assert_eq!(
        file(Some("macro")),
        Some(PathBuf::from("/opt/matsu/macro/RecKey.mac"))
    );
    assert_eq!(key_macro_file(None, None, Some(program), &is_dir), None);
}
