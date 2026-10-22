use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

/// A new empty folder for one test, removed at the end.
struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "matsu-config-migrate-{}-{}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).unwrap();
        Sandbox(dir)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap()
    }

    fn env(&self) -> Environment {
        Environment {
            exe_dir: Some(self.path("app")),
            home: Some(self.path("home")),
            app_data: Some(self.path("appdata")),
            xdg_config_home: None,
            profile_name: None,
        }
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

#[test]
fn windows_copies_the_appdata_folder_and_renames_the_ini() {
    let sb = Sandbox::new();
    sb.write("appdata/ume-editor/ume.ini", "[Common]\nx=1\n");
    sb.write("appdata/ume-editor/extra/notes.txt", "keep");
    sb.write("appdata/ume-editor/work/ume.ini", "[Common]\nx=2\n");
    let outcome = migrate(Os::Windows, &sb.env());
    assert_eq!(
        outcome,
        Outcome::Migrated {
            from: sb.path("appdata/ume-editor"),
            to: sb.path("appdata/matsu-editor"),
            skipped: 0,
        }
    );
    assert_eq!(sb.read("appdata/matsu-editor/matsu.ini"), "[Common]\nx=1\n");
    assert_eq!(
        sb.read("appdata/matsu-editor/work/matsu.ini"),
        "[Common]\nx=2\n"
    );
    assert_eq!(sb.read("appdata/matsu-editor/extra/notes.txt"), "keep");
    assert!(!sb.path("appdata/matsu-editor/ume.ini").exists());
    // The old files are as they were.
    assert_eq!(sb.read("appdata/ume-editor/ume.ini"), "[Common]\nx=1\n");
    assert_eq!(
        sb.read("appdata/ume-editor/work/ume.ini"),
        "[Common]\nx=2\n"
    );
    assert_eq!(sb.read("appdata/ume-editor/extra/notes.txt"), "keep");
    assert!(outcome.notice().unwrap().contains("引き継ぎました"));
}

#[test]
fn windows_portable_ini_next_to_the_exe_is_copied_alone() {
    let sb = Sandbox::new();
    sb.write("app/ume.ini", "portable");
    sb.write("app/matsu.exe", "program");
    sb.write("appdata/ume-editor/ume.ini", "roaming");
    let outcome = migrate(Os::Windows, &sb.env());
    assert_eq!(
        outcome,
        Outcome::Migrated {
            from: sb.path("app"),
            to: sb.path("app"),
            skipped: 0,
        }
    );
    assert_eq!(sb.read("app/matsu.ini"), "portable");
    assert_eq!(sb.read("app/ume.ini"), "portable");
    // The folder of the program is not copied, and the roaming folder is left for later.
    assert!(!sb.path("appdata/matsu-editor").exists());
    assert!(!sb.path("app/matsu.ini.migrating").exists());
    // The next start finds matsu.ini.
    assert_eq!(migrate(Os::Windows, &sb.env()), Outcome::NotNeeded);
}

#[test]
fn windows_portable_ini_of_a_profile_goes_to_the_same_sub_folder() {
    let sb = Sandbox::new();
    sb.write("app/work/ume.ini", "work");
    let mut env = sb.env();
    env.profile_name = Some("work".to_owned());
    migrate(Os::Windows, &env);
    assert_eq!(sb.read("app/work/matsu.ini"), "work");
}

#[test]
fn macos_copies_the_application_support_folder() {
    let sb = Sandbox::new();
    sb.write("home/Library/Application Support/ume-editor/ume.ini", "mac");
    let outcome = migrate(Os::MacOs, &sb.env());
    assert!(matches!(outcome, Outcome::Migrated { .. }));
    assert_eq!(
        sb.read("home/Library/Application Support/matsu-editor/matsu.ini"),
        "mac"
    );
    assert_eq!(
        sb.read("home/Library/Application Support/ume-editor/ume.ini"),
        "mac"
    );
}

#[test]
fn linux_copies_the_xdg_folder_or_dot_config() {
    let sb = Sandbox::new();
    sb.write("home/.config/ume-editor/ume.ini", "dot");
    assert!(matches!(
        migrate(Os::Linux, &sb.env()),
        Outcome::Migrated { .. }
    ));
    assert_eq!(sb.read("home/.config/matsu-editor/matsu.ini"), "dot");

    let sb = Sandbox::new();
    sb.write("xdg/ume-editor/ume.ini", "xdg");
    let mut env = sb.env();
    env.xdg_config_home = Some(sb.path("xdg"));
    assert!(matches!(migrate(Os::Linux, &env), Outcome::Migrated { .. }));
    assert_eq!(sb.read("xdg/matsu-editor/matsu.ini"), "xdg");
    assert_eq!(sb.read("xdg/ume-editor/ume.ini"), "xdg");
}

#[test]
fn an_existing_matsu_ini_means_no_copy() {
    let sb = Sandbox::new();
    sb.write("home/.config/ume-editor/ume.ini", "old");
    sb.write("home/.config/matsu-editor/matsu.ini", "new");
    assert_eq!(migrate(Os::Linux, &sb.env()), Outcome::NotNeeded);
    assert_eq!(sb.read("home/.config/matsu-editor/matsu.ini"), "new");
    assert!(!sb.path("home/.config/matsu-editor/ume.ini").exists());
}

#[test]
fn a_matsu_ini_in_any_place_counts_on_windows() {
    let sb = Sandbox::new();
    sb.write("app/matsu.ini", "portable new");
    sb.write("appdata/ume-editor/ume.ini", "old");
    assert_eq!(migrate(Os::Windows, &sb.env()), Outcome::NotNeeded);
    assert!(!sb.path("appdata/matsu-editor").exists());
}

#[test]
fn nothing_old_means_nothing_happens() {
    let sb = Sandbox::new();
    assert_eq!(migrate(Os::Linux, &sb.env()), Outcome::NothingToMigrate);
    assert!(!sb.path("home/.config/matsu-editor").exists());
    // An old folder without ume.ini is not a settings folder.
    sb.write("home/.config/ume-editor/other.txt", "x");
    assert_eq!(migrate(Os::Linux, &sb.env()), Outcome::NothingToMigrate);
    assert!(!sb.path("home/.config/matsu-editor").exists());
    assert_eq!(Outcome::NothingToMigrate.notice(), None);
    assert_eq!(Outcome::NotNeeded.notice(), None);
    // No places at all.
    assert_eq!(
        migrate(Os::Linux, &Environment::default()),
        Outcome::NotNeeded
    );
}

#[test]
fn a_failed_copy_leaves_the_old_files_alone_and_writes_no_matsu_ini() {
    let sb = Sandbox::new();
    sb.write("home/.config/ume-editor/ume.ini", "old");
    sb.write("home/.config/ume-editor/sub/data.txt", "data");
    // The new folder cannot be made: a file is in its way.
    sb.write("home/.config/matsu-editor", "a file, not a folder");
    let outcome = migrate(Os::Linux, &sb.env());
    let Outcome::Failed { from, .. } = &outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(*from, sb.path("home/.config/ume-editor"));
    assert!(outcome.notice().unwrap().contains("引き継げませんでした"));
    assert_eq!(sb.read("home/.config/ume-editor/ume.ini"), "old");
    assert_eq!(sb.read("home/.config/ume-editor/sub/data.txt"), "data");
    assert_eq!(sb.read("home/.config/matsu-editor"), "a file, not a folder");
}

#[test]
fn a_copy_that_stopped_half_way_is_tried_again_and_files_there_stay() {
    let sb = Sandbox::new();
    sb.write("home/.config/ume-editor/ume.ini", "old");
    sb.write("home/.config/ume-editor/a.txt", "from old");
    sb.write("home/.config/ume-editor/b.txt", "from old");
    // A leftover of an earlier try: the data file is there, matsu.ini is not.
    sb.write("home/.config/matsu-editor/a.txt", "already new");
    let outcome = migrate(Os::Linux, &sb.env());
    assert!(matches!(outcome, Outcome::Migrated { .. }));
    assert_eq!(sb.read("home/.config/matsu-editor/a.txt"), "already new");
    assert_eq!(sb.read("home/.config/matsu-editor/b.txt"), "from old");
    assert_eq!(sb.read("home/.config/matsu-editor/matsu.ini"), "old");
}

#[test]
fn the_migration_happens_once() {
    let sb = Sandbox::new();
    sb.write("home/.config/ume-editor/ume.ini", "old");
    assert!(matches!(
        migrate(Os::Linux, &sb.env()),
        Outcome::Migrated { .. }
    ));
    // The user changes the new file; the next start does not copy over it.
    sb.write("home/.config/matsu-editor/matsu.ini", "changed");
    assert_eq!(migrate(Os::Linux, &sb.env()), Outcome::NotNeeded);
    assert_eq!(sb.read("home/.config/matsu-editor/matsu.ini"), "changed");
    assert_eq!(sb.read("home/.config/ume-editor/ume.ini"), "old");
}

#[cfg(unix)]
#[test]
fn symbolic_links_in_the_old_folder_are_not_followed() {
    let sb = Sandbox::new();
    sb.write("home/.config/ume-editor/ume.ini", "old");
    sb.write("outside/secret.txt", "secret");
    std::os::unix::fs::symlink(
        sb.path("outside/secret.txt"),
        sb.path("home/.config/ume-editor/link.txt"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        sb.path("outside"),
        sb.path("home/.config/ume-editor/linkdir"),
    )
    .unwrap();
    assert!(matches!(
        migrate(Os::Linux, &sb.env()),
        Outcome::Migrated { .. }
    ));
    assert!(!sb.path("home/.config/matsu-editor/link.txt").exists());
    assert!(!sb.path("home/.config/matsu-editor/linkdir").exists());
}

#[test]
fn the_own_section_is_renamed_and_nothing_else_changes() {
    let text =
        "\u{feff}[Common]\r\nx=[UME]\r\n[UME]\r\nnMacKeyLayout=1\r\n [UME]\r\n[UMEX]\r\n[UME]";
    let sb = Sandbox::new();
    sb.write("home/.config/ume-editor/ume.ini", text);
    migrate(Os::Linux, &sb.env());
    assert_eq!(
        sb.read("home/.config/matsu-editor/matsu.ini"),
        "\u{feff}[Common]\r\nx=[UME]\r\n[MATSU]\r\nnMacKeyLayout=1\r\n [UME]\r\n[UMEX]\r\n[MATSU]"
    );
    assert_eq!(sb.read("home/.config/ume-editor/ume.ini"), text);
    // Shift_JIS bytes pass through.
    assert_eq!(
        rename_own_section(b"[UME]\n\x93\xfa\x96\x7b=1\n"),
        b"[MATSU]\n\x93\xfa\x96\x7b=1\n"
    );
}

#[test]
fn the_own_section_is_found_in_a_file_with_cr_only_lines() {
    assert_eq!(
        rename_own_section(b"[Common]\rx=1\r[UME]\ry=2\r"),
        b"[Common]\rx=1\r[MATSU]\ry=2\r"
    );
}

#[test]
fn a_file_that_cannot_be_copied_is_skipped_and_counted() {
    let sb = Sandbox::new();
    sb.write("home/.config/ume-editor/ume.ini", "old");
    sb.write("home/.config/ume-editor/sub/data.txt", "data");
    sb.write("home/.config/ume-editor/ok.txt", "ok");
    // A file is in the way of the folder `sub`.
    sb.write("home/.config/matsu-editor/sub", "a file");
    let outcome = migrate(Os::Linux, &sb.env());
    let Outcome::Migrated { skipped, .. } = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(skipped, 1);
    assert_eq!(sb.read("home/.config/matsu-editor/matsu.ini"), "old");
    assert_eq!(sb.read("home/.config/matsu-editor/ok.txt"), "ok");
    assert!(
        outcome
            .notice()
            .unwrap()
            .contains("1 個はコピーできませんでした")
    );
}
