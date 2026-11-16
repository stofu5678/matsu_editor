//! The move at start (ADR-0035, 6): the one-liners that `[OneLiner]` marks `AI[nnn]=1` become destinations of `[AI]`.
//!
//! [`move_at_start`] runs once when the program starts, after the profile is chosen and before the settings are read
//! (`crate::launch`), so the first read already sees `[AI]`. The change itself is `matsu_config::settings::ai::migrate`,
//! a pure change of the profile in memory; here it is put in the file the way the program writes other sections
//! (`matsu_config::history::update_profile`: under the lock of the file, read again, written once with the unknown
//! keys kept).
//!
//! * Nothing marked: nothing is written, not even the lock file.
//! * The marked slots fit in `[AI]`: they move, in one write, and the user is told once.
//! * They do not fit: the file is not changed, the slots stay and do not run (`crate::oneliner::flow::run_slot`),
//!   and the user is told. The start after a move finds nothing marked.
//!
//! The user hears of it on the status bar, in the place of the message about the move from the old name
//! (`crate::settings::set_startup_notice`): it needs no window, and a message there stays 20 seconds in every window
//! that opens at the start. The same text goes to the error output, as that message does. MATSU does not rewrite key
//! assignments, menus, toolbars or macros that called a moved `F_ONELINER_n` (they are the user's data and the new
//! number is not known to them); the message says that they no longer reach it.

use std::io;
use std::path::{Path, PathBuf};

use matsu_config::Profile;
use matsu_config::history::update_profile;
use matsu_config::location::{Environment, Os, candidate_paths, find_existing};
use matsu_config::settings::ai::{AiMigration, migrate};
use matsu_config::settings::oneliner::OneLiners;

/// Moves the AI one-liners of the settings file of this process, and keeps the message for the status bar.
pub(crate) fn move_at_start() {
    let candidates = candidate_paths(Os::current(), &Environment::from_process());
    if let Some(notice) = move_at(&candidates) {
        eprintln!("matsu: {notice}");
        crate::settings::set_startup_notice(&notice);
    }
}

/// [`move_at_start`] for the first of `candidates` that is a file: returns the message for the user, if there is
/// one. A file that does not exist or cannot be read has nothing to move.
pub(crate) fn move_at(candidates: &[PathBuf]) -> Option<String> {
    let path = find_existing(candidates)?;
    // Look first, without the lock and without writing: most starts have nothing marked.
    let profile = Profile::load(path).ok()?;
    if OneLiners::from_profile(&profile).ai_marked().is_empty() {
        return None;
    }
    move_in(path)
}

/// The move in the file `path`, which has marked slots when it is read first. `None` when the file turned out to have
/// none by the time it was locked (another window moved them).
fn move_in(path: &Path) -> Option<String> {
    let mut outcome = AiMigration::NothingToMove;
    let result = update_profile(path, |profile| {
        outcome = migrate(profile)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(outcome.changed())
    });
    match result {
        Err(error) => Some(format!(
            "ワンライナーの生成 AI の枠を、生成 AI の送り先（[AI]）に移せませんでした: {error}。\
             その枠は、移せるまで動きません。"
        )),
        Ok(()) => notice(outcome),
    }
}

/// What the user is told about what the move did.
fn notice(outcome: AiMigration) -> Option<String> {
    match outcome {
        AiMigration::NothingToMove => None,
        AiMigration::Moved { count } => Some(format!(
            "ワンライナーの生成 AI の枠 {count} 個を、生成 AI の送り先（[AI]）に移しました。\
             移した枠を呼んでいたキー割り当て・カスタムメニュー・ツールバー・マクロ\
             （F_ONELINER_n、OneLinern）は、もう届きません。"
        )),
        AiMigration::Blocked { count } => Some(format!(
            "生成 AI の送り先（[AI]）が満杯で、ワンライナーの生成 AI の枠 {count} 個を移せませんでした。\
             空きができるまで、その枠は動きません。"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::UserSettings;
    use matsu_config::settings::ai::{AiDestinations, MAX_AI};

    /// A settings file with the one-liners `(name, command, ai)` in the slots in a row, an unknown key in each of the
    /// sections and another section, and `ai_slots` destinations in `[AI]`.
    fn file(one_liners: &[(&str, &str, bool)], ai_slots: usize) -> String {
        let mut text =
            String::from("[Common]\r\nbDispTabWnd=1\r\n\r\n[OneLiner]\r\nFuture=keep\r\n");
        for (i, (name, command, ai)) in one_liners.iter().enumerate() {
            text += &format!(
                "Name[{i:03}]={name}\r\nCommand[{i:03}]={command}\r\nOutput[{i:03}]=1\r\n"
            );
            if *ai {
                text += &format!("AI[{i:03}]=1\r\nAskBeforeSend[{i:03}]=0\r\n");
            }
        }
        text += "\r\n[AI]\r\nFuture=keep\r\n";
        for i in 0..ai_slots {
            text += &format!("Name[{i:03}]=old{i}\r\nCommand[{i:03}]=cat\r\n");
        }
        text
    }

    fn folder_with(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("matsu.ini");
        std::fs::write(&path, text).unwrap();
        (dir, path)
    }

    fn names_of_dir(dir: &tempfile::TempDir) -> Vec<String> {
        let mut names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn the_marked_one_liners_move_in_one_write_and_the_user_hears_of_it_once() {
        let (dir, path) = folder_with(&file(
            &[
                ("up", "tr a-z A-Z", false),
                ("ask", "claude -p", true),
                ("ask2", "codex exec -", true),
            ],
            0,
        ));
        let notice = move_at(std::slice::from_ref(&path)).expect("a message");
        assert!(notice.contains("2 個"), "{notice}");
        assert!(
            notice.contains("キー割り当て") && notice.contains("マクロ"),
            "{notice}"
        );
        // the settings read at the start have the destinations, and the one-liner that was not marked stays
        let settings = UserSettings::load_from(std::slice::from_ref(&path));
        let ai = settings.shared_ai_destinations();
        assert_eq!(
            (
                ai.slots[1].name.as_str(),
                ai.slots[1].command.as_str(),
                ai.slots[1].ask_before_send
            ),
            ("ask", "claude -p", false)
        );
        assert_eq!(ai.slots[2].command, "codex exec -");
        let one = settings.shared_oneliners();
        assert!(one.slots[0].is_registered() && !one.slots[0].is_ai_marked());
        assert!(!one.slots[1].is_registered() && !one.slots[2].is_registered());
        // the keys that this program does not know and the other sections stay; the marks are gone
        let profile = Profile::load(&path).unwrap();
        assert_eq!(profile.get("OneLiner", "Future"), Some("keep"));
        assert_eq!(profile.get("AI", "Future"), Some("keep"));
        assert_eq!(profile.get("Common", "bDispTabWnd"), Some("1"));
        assert_eq!(profile.get("OneLiner", "AI[001]"), None);
        assert_eq!(profile.get("OneLiner", "Command[001]"), None);
        // nothing but the settings file is left in the folder (the temporary file of the write and the lock are gone)
        assert_eq!(names_of_dir(&dir), ["matsu.ini"]);
        // the next start finds nothing marked: no message, and the file is not written
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(move_at(std::slice::from_ref(&path)), None);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(names_of_dir(&dir), ["matsu.ini"]);
    }

    #[test]
    fn nothing_is_written_when_nothing_is_marked() {
        let (dir, path) = folder_with(&file(&[("up", "tr a-z A-Z", false)], 3));
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(move_at(std::slice::from_ref(&path)), None);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(names_of_dir(&dir), ["matsu.ini"], "not even the lock file");
        // no file at all
        let missing = dir.path().join("none.ini");
        assert_eq!(move_at(std::slice::from_ref(&missing)), None);
        assert!(!missing.exists());
        assert_eq!(move_at(&[]), None);
    }

    #[test]
    fn a_full_ai_section_changes_nothing_and_the_marked_slots_stay_and_do_not_run() {
        let (dir, path) = folder_with(&file(
            &[("a", "claude -p", true), ("b", "codex exec -", true)],
            MAX_AI - 1,
        ));
        let bytes = std::fs::read(&path).unwrap();
        let notice = move_at(std::slice::from_ref(&path)).expect("a message");
        assert!(
            notice.contains("満杯") && notice.contains("2 個"),
            "{notice}"
        );
        assert!(notice.contains("動きません"), "{notice}");
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "no part is moved");
        assert_eq!(names_of_dir(&dir), ["matsu.ini"]);
        let settings = UserSettings::load_from(std::slice::from_ref(&path));
        let one = settings.shared_oneliners();
        assert!(one.slots[0].is_ai_marked() && one.slots[1].is_ai_marked());
        assert_eq!(
            settings
                .shared_ai_destinations()
                .slots
                .iter()
                .filter(|d| d.is_registered())
                .count(),
            MAX_AI - 1
        );
        // every start says it again, until there is room
        assert_eq!(
            move_at(std::slice::from_ref(&path)).as_deref(),
            Some(notice.as_str())
        );
        // and once there is room, the next start moves them
        let mut profile = Profile::load(&path).unwrap();
        let mut table = AiDestinations::from_profile(&profile);
        table.slots[10] = Default::default();
        table.slots[11] = Default::default();
        table.store(&mut profile).unwrap();
        profile.save(&path).unwrap();
        assert!(
            move_at(std::slice::from_ref(&path))
                .unwrap()
                .contains("2 個を")
        );
    }

    #[test]
    fn a_file_with_nothing_in_it_has_nothing_to_move() {
        let (_dir, path) = folder_with("");
        assert_eq!(move_at(&[path]), None);
    }

    #[test]
    fn the_notice_of_a_second_migration_follows_the_first() {
        crate::settings::set_startup_notice("first");
        crate::settings::set_startup_notice("second");
        assert_eq!(
            crate::settings::startup_notice().as_deref(),
            Some("first second")
        );
        crate::settings::expire_startup_notice();
    }
}
