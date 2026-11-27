//! The keys and values of the association, and the flows on a fake registry. The paths in these
//! tests are written as text; nothing here reads the file system or the real registry.

use std::collections::BTreeMap;
use std::io;

use super::*;

/// The registry in memory. Key names and value names do not depend on case, as in Windows: they
/// are kept in lower case, with the name as written next to the value.
#[derive(Default)]
struct Fake {
    user: BTreeMap<String, BTreeMap<String, (String, String)>>,
    machine: BTreeMap<String, BTreeMap<String, (String, String)>>,
    choices: BTreeMap<String, String>,
    /// How many times the shell was told that associations changed.
    notified: u32,
    /// A path whose `set_value` fails, to test a failure half way.
    fail_on: Option<String>,
    /// `copy_machine_tree` fails (the machine's ProgID is missing or cannot be read).
    fail_copy: bool,
    /// `delete_value` fails for this key (an access denied that must not be taken for "missing").
    fail_delete_value: Option<String>,
}

fn key(path: &str) -> String {
    path.to_lowercase()
}

impl Fake {
    fn put_machine(&mut self, path: &str, name: &str, data: &str) {
        self.machine
            .entry(key(path))
            .or_default()
            .insert(key(name), (name.to_owned(), data.to_owned()));
    }

    fn put_user(&mut self, path: &str, name: &str, data: &str) {
        self.set_value(path, name, data).unwrap();
    }

    fn keys(&self) -> Vec<String> {
        self.user.keys().cloned().collect()
    }

    /// Everything the fake holds, to compare before and after.
    fn snapshot(&self) -> Vec<(String, Vec<(String, String)>)> {
        self.user
            .iter()
            .map(|(k, v)| (k.clone(), v.values().cloned().collect()))
            .collect()
    }

    fn has_below(&self, path: &str) -> bool {
        let prefix = format!("{}\\", key(path));
        self.user.keys().any(|k| k.starts_with(&prefix))
    }
}

impl Registry for Fake {
    fn key_exists(&self, path: &str) -> bool {
        self.user.contains_key(&key(path)) || self.has_below(path)
    }

    fn value(&self, path: &str, name: &str) -> Option<String> {
        self.user
            .get(&key(path))?
            .get(&key(name))
            .map(|(_, data)| data.clone())
    }

    fn set_value(&mut self, path: &str, name: &str, data: &str) -> io::Result<()> {
        if self.fail_on.as_deref() == Some(path) {
            return Err(io::Error::other("denied"));
        }
        self.user
            .entry(key(path))
            .or_default()
            .insert(key(name), (name.to_owned(), data.to_owned()));
        Ok(())
    }

    fn delete_value(&mut self, path: &str, name: &str) -> io::Result<()> {
        if self.fail_delete_value.as_deref() == Some(path) {
            return Err(io::Error::from(io::ErrorKind::PermissionDenied));
        }
        if let Some(values) = self.user.get_mut(&key(path)) {
            values.remove(&key(name));
        }
        Ok(())
    }

    fn delete_tree(&mut self, path: &str) -> io::Result<()> {
        let own = key(path);
        let prefix = format!("{own}\\");
        self.user
            .retain(|k, _| *k != own && !k.starts_with(&prefix));
        Ok(())
    }

    fn delete_key_if_empty(&mut self, path: &str) -> io::Result<()> {
        let empty = self.user.get(&key(path)).is_some_and(BTreeMap::is_empty);
        if empty && !self.has_below(path) {
            self.user.remove(&key(path));
        }
        Ok(())
    }

    fn machine_default(&self, path: &str) -> Option<String> {
        self.machine
            .get(&key(path))?
            .get("")
            .map(|(_, data)| data.clone())
    }

    fn copy_machine_tree(&mut self, from: &str, to: &str) -> io::Result<()> {
        if self.fail_copy {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        let from = key(from);
        let prefix = format!("{from}\\");
        let copies: Vec<_> = self
            .machine
            .iter()
            .filter(|(k, _)| **k == from || k.starts_with(&prefix))
            .map(|(k, v)| (format!("{}{}", key(to), &k[from.len()..]), v.clone()))
            .collect();
        for (path, values) in copies {
            self.user.entry(path).or_default().extend(values);
        }
        Ok(())
    }

    fn notify_changed(&mut self) {
        self.notified += 1;
    }

    fn user_choice(&self, ext: &Ext) -> Option<String> {
        self.choices.get(&ext.dot_key()).cloned()
    }
}

const EXE: &str = r"C:\Program Files\MATSU Editor\matsu.exe";

fn ext(token: &str) -> Ext {
    Ext::from_token(token).unwrap()
}

#[test]
fn an_extension_is_lower_case_and_has_the_names_of_matsu() {
    let e = ext("TXT");
    assert_eq!(e.as_str(), "txt");
    assert_eq!(e.dot_key(), ".txt");
    assert_eq!(e.prog_id(), "MatsuEditor_txt");
    assert_eq!(
        e.command_key(),
        r"MatsuEditor_txt\shell\MatsuEditor\command"
    );
    assert_eq!(
        e.shell_backup_key(),
        r"MatsuEditor_txt\shell\MatsuEditor\ShellBackup"
    );
}

#[test]
fn a_token_with_a_wildcard_or_a_path_is_not_an_extension() {
    for token in ["", "*.txt", "te?t", r"a\b", "a/b", "a:b", "a|b", "a\u{7}b"] {
        assert_eq!(Ext::from_token(token), None, "{token:?}");
    }
    // Sakura registers a token as it is (`.makefile`); so does this.
    assert_eq!(ext("Makefile").dot_key(), ".makefile");
    assert_eq!(ext("日本語").dot_key(), ".日本語");
}

#[test]
fn the_extension_list_is_split_like_the_type_list_does() {
    let names: Vec<_> = extensions_of("c cpp;h,H *.x  txt")
        .iter()
        .map(|e| e.as_str().to_owned())
        .collect();
    assert_eq!(names, ["c", "cpp", "h", "txt"]);
    assert!(extensions_of("").is_empty());
    assert!(extensions_of("*.* a*").is_empty());
}

#[test]
fn the_command_quotes_the_program_and_the_argument() {
    assert_eq!(
        command_line(EXE),
        r#""C:\Program Files\MATSU Editor\matsu.exe" "%1""#
    );
    assert_eq!(
        command_line(r"D:\ツール\マツ エディタ\matsu.exe"),
        r#""D:\ツール\マツ エディタ\matsu.exe" "%1""#
    );
}

#[test]
fn the_choice_of_the_user_is_ours_for_the_progid_or_the_program() {
    let e = ext("txt");
    assert!(user_choice_is_ours("MatsuEditor_txt", &e, "matsu.exe"));
    assert!(user_choice_is_ours("matsueditor_TXT", &e, "matsu.exe"));
    assert!(user_choice_is_ours(
        r"Applications\matsu.exe",
        &e,
        "matsu.exe"
    ));
    assert!(user_choice_is_ours(
        r"applications\MATSU.EXE",
        &e,
        "matsu.exe"
    ));
    // The ProgID of the installer (`packaging/windows/matsu.iss`).
    assert!(user_choice_is_ours("MATSU.Editor.txt", &e, "matsu.exe"));
    assert!(user_choice_is_ours("matsu.editor.TXT", &e, "matsu.exe"));
    assert!(!user_choice_is_ours("MATSU.Editor.log", &e, "matsu.exe"));
    assert!(!user_choice_is_ours("txtfile", &e, "matsu.exe"));
    assert!(!user_choice_is_ours("MatsuEditor_log", &e, "matsu.exe"));
    assert!(!user_choice_is_ours(
        r"Applications\notepad.exe",
        &e,
        "matsu.exe"
    ));
    assert!(!user_choice_is_ours("", &e, "matsu.exe"));
}

#[test]
fn registering_writes_the_item_and_makes_it_the_default() {
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("MatsuEditor_mtest"));
    assert_eq!(r.value(".mtest", PROGID_BACKUP_NAME), None);
    assert_eq!(
        r.value(r"MatsuEditor_mtest\shell\MatsuEditor", "")
            .as_deref(),
        Some("MATSU &Editor")
    );
    assert_eq!(
        r.value(r"MatsuEditor_mtest\shell\MatsuEditor\command", "")
            .as_deref(),
        Some(r#""C:\Program Files\MATSU Editor\matsu.exe" "%1""#)
    );
    assert_eq!(
        r.value(r"MatsuEditor_mtest\shell", "").as_deref(),
        Some("MatsuEditor")
    );
    assert_eq!(
        check(&r, &ext("mtest")),
        Status {
            menu: true,
            default: true
        }
    );
    // The item for every program, whatever the default is.
    assert_eq!(
        r.value(r"SystemFileAssociations\.mtest\shell\MatsuEditor", "")
            .as_deref(),
        Some("MATSU &Editor")
    );
    assert_eq!(
        r.value(
            r"SystemFileAssociations\.mtest\shell\MatsuEditor\command",
            ""
        )
        .as_deref(),
        Some(r#""C:\Program Files\MATSU Editor\matsu.exe" "%1""#)
    );
}

#[test]
fn the_menu_item_stays_when_another_program_took_the_extension() {
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    r.put_user(".mtest", "", "newowner");
    assert_eq!(
        check(&r, &ext("mtest")),
        Status {
            menu: true,
            default: false
        }
    );
    unregister(&mut r, &ext("mtest")).unwrap();
    assert!(!r.key_exists("SystemFileAssociations"));
    assert_eq!(r.value(".mtest", "").as_deref(), Some("newowner"));
}

#[test]
fn removing_keeps_what_else_is_under_system_file_associations() {
    let mut r = Fake::default();
    r.put_user(
        r"SystemFileAssociations\.mtest\shell\other\command",
        "",
        "o.exe",
    );
    register(&mut r, &ext("mtest"), EXE, false).unwrap();
    unregister(&mut r, &ext("mtest")).unwrap();
    assert!(r.key_exists(r"SystemFileAssociations\.mtest\shell\other\command"));
    assert!(!r.key_exists(r"SystemFileAssociations\.mtest\shell\MatsuEditor"));
}

#[test]
fn only_the_menu_item_without_the_default() {
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, false).unwrap();
    assert_eq!(
        check(&r, &ext("mtest")),
        Status {
            menu: true,
            default: false
        }
    );
    assert_eq!(r.value(r"MatsuEditor_mtest\shell", ""), None);
}

#[test]
fn an_unregistered_extension_shows_nothing() {
    let r = Fake::default();
    assert_eq!(check(&r, &ext("mtest")), Status::default());
    let mut r = Fake::default();
    // A ProgID of someone else, with a verb of its own: not ours.
    r.put_user(".mtest", "", "other");
    r.put_user(r"other\shell\open\command", "", "other.exe");
    assert_eq!(check(&r, &ext("mtest")), Status::default());
}

#[test]
fn registering_twice_changes_nothing() {
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    let once = r.snapshot();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert_eq!(r.snapshot(), once);
}

#[test]
fn the_previous_default_is_kept_and_comes_back() {
    let mut r = Fake::default();
    r.put_user(".mtest", "", "othertype");
    r.put_user(".mtest", "OpenWithProgids", "x");
    r.put_user(r".mtest\OpenWithProgids", "app.mtest", "");
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert_eq!(
        r.value(".mtest", PROGID_BACKUP_NAME).as_deref(),
        Some("othertype")
    );
    unregister(&mut r, &ext("mtest")).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("othertype"));
    assert_eq!(r.value(".mtest", PROGID_BACKUP_NAME), None);
    assert!(!r.key_exists("MatsuEditor_mtest"));
    // What MATSU did not write stays.
    assert_eq!(r.value(".mtest", "OpenWithProgids").as_deref(), Some("x"));
    assert!(r.key_exists(r".mtest\OpenWithProgids"));
}

#[test]
fn the_verbs_the_machine_had_are_copied_and_the_old_default_verb_comes_back() {
    let mut r = Fake::default();
    r.put_machine(".mtest", "", "othertype");
    r.put_machine("othertype", "", "Other file");
    r.put_machine(r"othertype\shell", "", "open");
    r.put_machine(r"othertype\shell\open\command", "", "other.exe %1");
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    // The machine's ProgID is only copied, not changed.
    assert_eq!(
        r.value(r"MatsuEditor_mtest\shell\open\command", "")
            .as_deref(),
        Some("other.exe %1")
    );
    assert_eq!(
        r.value(r"MatsuEditor_mtest\shell", "").as_deref(),
        Some("MatsuEditor")
    );
    assert_eq!(
        r.value(r"MatsuEditor_mtest\shell\MatsuEditor\ShellBackup", "")
            .as_deref(),
        Some("open")
    );
    // Without the double click `.mtest` is not ours any more: the machine's ProgID (and its
    // `open`) is the default again.
    register(&mut r, &ext("mtest"), EXE, false).unwrap();
    assert_eq!(r.value(".mtest", ""), None);
    assert_eq!(r.machine_default(".mtest").as_deref(), Some("othertype"));
    assert_eq!(
        check(&r, &ext("mtest")),
        Status {
            menu: true,
            default: false
        }
    );
    // And with it again.
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert!(check(&r, &ext("mtest")).default);
}

#[test]
fn removing_leaves_a_clean_registry() {
    let mut r = Fake::default();
    r.put_machine(".mtest", "", "othertype");
    r.put_machine(r"othertype\shell\open\command", "", "other.exe %1");
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    unregister(&mut r, &ext("mtest")).unwrap();
    assert_eq!(r.keys(), Vec::<String>::new());
    // Removing again is not an error.
    unregister(&mut r, &ext("mtest")).unwrap();
    assert_eq!(r.keys(), Vec::<String>::new());
}

#[test]
fn removing_does_not_touch_another_program() {
    let mut r = Fake::default();
    // Another program's ProgID and extension, and a ProgID of ours for another extension.
    r.put_user(".other", "", "otherapp");
    r.put_user(r"otherapp\shell\open\command", "", "other.exe");
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    register(&mut r, &ext("mlog"), EXE, true).unwrap();
    let before_mlog: Vec<_> = r
        .snapshot()
        .into_iter()
        .filter(|(k, _)| k.contains("mlog"))
        .collect();
    unregister(&mut r, &ext("mtest")).unwrap();
    assert_eq!(r.value(".other", "").as_deref(), Some("otherapp"));
    assert!(r.key_exists(r"otherapp\shell\open\command"));
    let after_mlog: Vec<_> = r
        .snapshot()
        .into_iter()
        .filter(|(k, _)| k.contains("mlog"))
        .collect();
    assert_eq!(before_mlog, after_mlog);
    assert!(!r.key_exists(".mtest"));
}

#[test]
fn when_the_extension_was_given_to_another_program_the_default_stays() {
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    // Another program takes `.mtest` afterwards.
    r.put_user(".mtest", "", "newowner");
    r.put_user(r"newowner\shell\open\command", "", "new.exe");
    unregister(&mut r, &ext("mtest")).unwrap();
    // The new owner keeps `.mtest` and its ProgID; ours is gone.
    assert_eq!(r.value(".mtest", "").as_deref(), Some("newowner"));
    assert!(r.key_exists(r"newowner\shell\open\command"));
    assert!(!r.key_exists("MatsuEditor_mtest"));
    assert_eq!(r.value(".mtest", PROGID_BACKUP_NAME), None);
}

#[test]
fn the_default_verb_that_is_not_ours_is_not_restored_over() {
    let mut r = Fake::default();
    r.put_machine(".mtest", "", "othertype");
    r.put_machine(r"othertype\shell", "", "open");
    r.put_machine(r"othertype\shell\open\command", "", "other.exe %1");
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    // Somebody sets the default verb to `edit` by hand.
    r.put_user(r"MatsuEditor_mtest\shell", "", "edit");
    register(&mut r, &ext("mtest"), EXE, false).unwrap();
    assert_eq!(
        r.value(r"MatsuEditor_mtest\shell", "").as_deref(),
        Some("edit")
    );
}

#[test]
fn a_type_shows_the_and_of_its_extensions() {
    let mut r = Fake::default();
    register(&mut r, &ext("c"), EXE, true).unwrap();
    register(&mut r, &ext("h"), EXE, false).unwrap();
    let state = type_state(&r, "c h *.x", "matsu.exe");
    assert!(state.available);
    assert!(state.menu);
    assert!(!state.default);
    let state = type_state(&r, "c cpp", "matsu.exe");
    assert!(!state.menu);
    assert!(!state.default);
    // A type with nothing to associate has grey check boxes.
    assert_eq!(type_state(&r, "", "matsu.exe"), TypeState::default());
    assert_eq!(type_state(&r, "*.x", "matsu.exe"), TypeState::default());
}

#[test]
fn the_choice_of_the_user_is_shown_when_it_beats_the_double_click() {
    let mut r = Fake::default();
    register(&mut r, &ext("c"), EXE, true).unwrap();
    register(&mut r, &ext("h"), EXE, true).unwrap();
    r.choices.insert(".c".into(), "VisualStudio.c.1".into());
    r.choices
        .insert(".h".into(), r"Applications\matsu.exe".into());
    let state = type_state(&r, "c h", "matsu.exe");
    assert!(state.default);
    assert_eq!(state.overridden, ["c"]);
    // Without the double click there is nothing to override.
    register(&mut r, &ext("c"), EXE, false).unwrap();
    assert!(type_state(&r, "c h", "matsu.exe").overridden.is_empty());
}

#[test]
fn the_check_boxes_do_what_sakura_does() {
    let mut r = Fake::default();
    // Checking the menu also makes the double click (`RegistExt(ext, true)`).
    apply(&mut r, "c h", EXE, Change::Menu(true)).unwrap();
    let state = type_state(&r, "c h", "matsu.exe");
    assert!(state.menu && state.default);
    // Unchecking the double click keeps the menu.
    apply(&mut r, "c h", EXE, Change::Default(false)).unwrap();
    let state = type_state(&r, "c h", "matsu.exe");
    assert!(state.menu && !state.default);
    apply(&mut r, "c h", EXE, Change::Default(true)).unwrap();
    assert!(type_state(&r, "c h", "matsu.exe").default);
    // Unchecking the menu removes all of it.
    apply(&mut r, "c h", EXE, Change::Menu(false)).unwrap();
    assert!(!type_state(&r, "c h", "matsu.exe").menu);
    assert_eq!(r.keys(), Vec::<String>::new());
}

#[test]
fn a_failure_stops_at_that_extension_and_the_state_tells_the_truth() {
    let mut r = Fake {
        fail_on: Some(".h".into()),
        ..Fake::default()
    };
    let error = apply(&mut r, "c h cpp", EXE, Change::Menu(true)).unwrap_err();
    assert_eq!(error.to_string(), "denied");
    assert!(check(&r, &ext("c")).menu);
    // `.h` failed at the last step (`.h` itself): its item was written, but it is not the default.
    assert!(check(&r, &ext("h")).menu);
    assert!(!check(&r, &ext("h")).default);
    assert!(!check(&r, &ext("cpp")).menu);
    assert!(!type_state(&r, "c h cpp", "matsu.exe").menu);
}

#[test]
fn only_windows_associates() {
    assert_eq!(supported(), cfg!(windows));
    #[cfg(not(windows))]
    {
        assert!(current_type_state("txt").is_err());
        assert!(apply_to_system("txt", Change::Menu(true)).is_err());
    }
}

#[test]
fn the_icon_is_the_program_in_our_progid_only() {
    assert_eq!(
        icon_value(r"C:\Program Files\MATSU Editor\matsu.exe"),
        r"C:\Program Files\MATSU Editor\matsu.exe,0"
    );
    let mut r = Fake::default();
    // The machine's ProgID has an icon of the other program: our copy gets ours, the machine's stays.
    r.put_machine(".mtest", "", "othertype");
    r.put_machine(r"othertype\DefaultIcon", "", "other.exe,3");
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert_eq!(
        r.value(r"MatsuEditor_mtest\DefaultIcon", "").as_deref(),
        Some(r"C:\Program Files\MATSU Editor\matsu.exe,0")
    );
    assert_eq!(
        r.machine[&key(r"othertype\DefaultIcon")][""].1,
        "other.exe,3"
    );
    // Another program's ProgID is never written.
    r.put_user(".other", "", "otherapp");
    r.put_user(r"otherapp\DefaultIcon", "", "o.exe,1");
    unregister(&mut r, &ext("mtest")).unwrap();
    assert_eq!(
        r.value(r"otherapp\DefaultIcon", "").as_deref(),
        Some("o.exe,1")
    );
    assert!(!r.key_exists("MatsuEditor_mtest"));
}

#[test]
fn the_shell_is_told_once_per_change_and_not_when_nothing_was_asked() {
    let mut r = Fake::default();
    apply(&mut r, "c h cpp", EXE, Change::Menu(true)).unwrap();
    assert_eq!(r.notified, 1);
    apply(&mut r, "c h cpp", EXE, Change::Default(false)).unwrap();
    assert_eq!(r.notified, 2);
    apply(&mut r, "c h cpp", EXE, Change::Menu(false)).unwrap();
    assert_eq!(r.notified, 3);
    // Nothing to associate: nothing changed.
    apply(&mut r, "*.x", EXE, Change::Menu(true)).unwrap();
    assert_eq!(r.notified, 3);
    // A failure half way still tells the shell about what was written.
    r.fail_on = Some(".h".into());
    assert!(apply(&mut r, "c h", EXE, Change::Menu(true)).is_err());
    assert_eq!(r.notified, 4);
}

#[test]
fn double_click_off_with_nothing_to_fall_back_on_leaves_mtest_without_our_default() {
    // No previous default and no `open` to copy: only our verb is in our ProgID.
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    register(&mut r, &ext("mtest"), EXE, false).unwrap();
    // `.mtest` does not name our ProgID, so Windows cannot fall back on our only verb.
    assert_eq!(r.value(".mtest", ""), None);
    assert!(!r.key_exists(".mtest"));
    assert_eq!(
        check(&r, &ext("mtest")),
        Status {
            menu: true,
            default: false
        }
    );
    // The right-click item is still there, under every program.
    assert_eq!(
        r.value(
            r"SystemFileAssociations\.mtest\shell\MatsuEditor\command",
            ""
        )
        .as_deref(),
        Some(r#""C:\Program Files\MATSU Editor\matsu.exe" "%1""#)
    );
}

#[test]
fn double_click_off_gives_back_the_previous_progid_and_on_takes_it_again() {
    let mut r = Fake::default();
    r.put_user(".mtest", "", "othertype");
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("MatsuEditor_mtest"));
    // on -> off
    register(&mut r, &ext("mtest"), EXE, false).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("othertype"));
    assert_eq!(r.value(".mtest", PROGID_BACKUP_NAME), None);
    assert!(check(&r, &ext("mtest")).menu);
    assert!(!check(&r, &ext("mtest")).default);
    // off -> on: MATSU is the default again, and the previous one is kept again.
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("MatsuEditor_mtest"));
    assert_eq!(
        r.value(".mtest", PROGID_BACKUP_NAME).as_deref(),
        Some("othertype")
    );
    assert!(check(&r, &ext("mtest")).default);
    // And removing everything gives the previous one back.
    unregister(&mut r, &ext("mtest")).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("othertype"));
}

#[test]
fn double_click_off_does_not_take_back_an_extension_that_another_program_owns() {
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    r.put_user(".mtest", "", "newowner");
    register(&mut r, &ext("mtest"), EXE, false).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("newowner"));
}

#[test]
fn the_check_boxes_report_the_real_state_with_the_double_click_off() {
    let mut r = Fake::default();
    apply(&mut r, "c h", EXE, Change::Menu(true)).unwrap();
    apply(&mut r, "c h", EXE, Change::Default(false)).unwrap();
    let state = type_state(&r, "c h", "matsu.exe");
    assert!(state.menu);
    assert!(!state.default);
    apply(&mut r, "c h", EXE, Change::Default(true)).unwrap();
    let state = type_state(&r, "c h", "matsu.exe");
    assert!(state.menu && state.default);
}

#[test]
fn a_choice_of_the_user_for_matsu_is_shown_when_the_double_click_is_off() {
    let mut r = Fake::default();
    apply(&mut r, "c h", EXE, Change::Menu(true)).unwrap();
    // The user chose MATSU in the default apps for `.c` (our ProgID) and `.h` (the installer's).
    r.choices.insert(".c".into(), "MatsuEditor_c".into());
    r.choices.insert(".h".into(), "MATSU.Editor.h".into());
    // The double click is on: nothing to say.
    let state = type_state(&r, "c h", "matsu.exe");
    assert!(state.default);
    assert!(state.chosen.is_empty());
    assert!(state.overridden.is_empty());
    // Off: Windows still follows the choice to MATSU, and the state says so.
    apply(&mut r, "c h", EXE, Change::Default(false)).unwrap();
    let state = type_state(&r, "c h", "matsu.exe");
    assert!(!state.default);
    assert!(state.menu);
    assert_eq!(state.chosen, ["c", "h"]);
    // A choice for another program is not a reason to say that.
    r.choices.insert(".c".into(), "txtfile".into());
    assert_eq!(type_state(&r, "c h", "matsu.exe").chosen, ["h"]);
    // Nothing is registered: nothing to say either.
    let r = Fake::default();
    assert!(type_state(&r, "c", "matsu.exe").chosen.is_empty());
}

#[test]
fn the_progid_is_ours_whatever_its_letter_case() {
    let mut r = Fake::default();
    // `.mtest` names our ProgID with other letters, and no previous default exists.
    r.put_user(".mtest", "", "matsueditor_MTEST");
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    // It is not taken for a previous default of another program.
    assert_eq!(r.value(".mtest", PROGID_BACKUP_NAME), None);
    assert_eq!(r.value(".mtest", "").as_deref(), Some("MatsuEditor_mtest"));
    // Removing it does not restore a self-reference.
    r.put_user(".mtest", "", "MATSUEDITOR_mtest");
    r.put_user(".mtest", PROGID_BACKUP_NAME, "othertype");
    unregister(&mut r, &ext("mtest")).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("othertype"));
    assert!(!r.key_exists("MatsuEditor_mtest"));
}

#[test]
fn the_verb_and_the_default_are_compared_without_case() {
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    r.put_user(r"MatsuEditor_mtest\shell", "", "MATSUEDITOR");
    assert!(check(&r, &ext("mtest")).default);
    // Registering again does not back the verb up as if it were another program's.
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert_eq!(
        r.value(r"MatsuEditor_mtest\shell\MatsuEditor\ShellBackup", ""),
        None
    );
    // Turning the double click off with the `.ext` in other letters gives the previous one back.
    r.put_user(".mtest", "", "MATSUEDITOR_MTEST");
    r.put_user(".mtest", PROGID_BACKUP_NAME, "othertype");
    register(&mut r, &ext("mtest"), EXE, false).unwrap();
    assert_eq!(r.value(".mtest", "").as_deref(), Some("othertype"));
}

#[test]
fn a_failure_while_the_progid_is_built_leaves_the_extension_as_it_was() {
    let mut r = Fake {
        fail_copy: true,
        ..Fake::default()
    };
    // The machine's `.mtest` names a ProgID that cannot be copied; the user had a default too.
    r.put_machine(".mtest", "", "missingtype");
    r.put_user(".mtest", "", "othertype");
    assert!(register(&mut r, &ext("mtest"), EXE, true).is_err());
    assert_eq!(r.value(".mtest", "").as_deref(), Some("othertype"));
    assert_eq!(r.value(".mtest", PROGID_BACKUP_NAME), None);
    // Nothing says MATSU is associated, so the dialog can try again.
    assert_eq!(check(&r, &ext("mtest")), Status::default());
    // And once the copy works, the same call completes.
    r.fail_copy = false;
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    assert!(check(&r, &ext("mtest")).default);
}

#[test]
fn a_delete_that_fails_stops_the_removal_before_the_progid_goes() {
    let mut r = Fake::default();
    register(&mut r, &ext("mtest"), EXE, true).unwrap();
    r.fail_delete_value = Some(".mtest".into());
    let error = unregister(&mut r, &ext("mtest")).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    // `.mtest` still names MATSU's ProgID, and the ProgID is still there: no dangling association.
    assert_eq!(r.value(".mtest", "").as_deref(), Some("MatsuEditor_mtest"));
    assert!(r.key_exists(r"MatsuEditor_mtest\shell\MatsuEditor\command"));
    // It can be removed once the key can be written.
    r.fail_delete_value = None;
    unregister(&mut r, &ext("mtest")).unwrap();
    assert!(!r.key_exists(".mtest"));
    assert!(!r.key_exists("MatsuEditor_mtest"));
}
