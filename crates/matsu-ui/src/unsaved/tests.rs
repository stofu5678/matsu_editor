use std::io::Read;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use matsu_config::unsaved::{DocInfo, Session, SessionWindow, Store, StoreError};
use matsu_core::buffer::{Document, Eol, Pos};
use matsu_core::encoding::Encoding;

use super::keeper::{DEBOUNCE, Entry, MAX_WAIT, Plan, RETRY, Want, decide};
use super::writer::{Revision, Snapshot, Writer, write_text};

fn rev(version: u64) -> Revision {
    Revision {
        version,
        encoding: Encoding::Utf8,
        bom: false,
        eol: Eol::Lf,
    }
}

fn secs(n: u64) -> Duration {
    Duration::from_secs(n)
}

fn write_plan() -> Plan {
    Plan {
        write: true,
        ..Plan::default()
    }
}

#[test]
fn a_change_is_written_two_seconds_after_the_last_change() {
    let t0 = Instant::now();
    let mut e = Entry::new(t0);
    // The first tick that sees the change starts the count.
    assert_eq!(decide(&mut e, Want::Keep, rev(1), t0), Plan::default());
    assert_eq!(
        decide(
            &mut e,
            Want::Keep,
            rev(1),
            t0 + DEBOUNCE - Duration::from_millis(1)
        ),
        Plan::default()
    );
    assert_eq!(
        decide(&mut e, Want::Keep, rev(1), t0 + DEBOUNCE),
        write_plan()
    );
}

#[test]
fn typing_on_moves_the_two_seconds_but_not_the_thirty() {
    let t0 = Instant::now();
    let mut e = Entry::new(t0);
    let mut now = t0;
    // A new change every second: never still for two seconds.
    for version in 1..=29 {
        assert_eq!(
            decide(&mut e, Want::Keep, rev(version), now),
            Plan::default(),
            "at {now:?}"
        );
        now += secs(1);
    }
    // 30 seconds after the first change it is written although the text is still changing.
    assert_eq!(
        decide(&mut e, Want::Keep, rev(30), t0 + MAX_WAIT),
        write_plan()
    );
}

#[test]
fn a_document_that_is_written_is_left_alone_until_it_changes() {
    let t0 = Instant::now();
    let mut e = Entry::new(t0);
    decide(&mut e, Want::Keep, rev(1), t0);
    assert!(decide(&mut e, Want::Keep, rev(1), t0 + secs(3)).write);
    // The thread wrote it.
    e.shared.in_flight.store(false, Ordering::SeqCst);
    *e.shared.written.lock().unwrap() = Some(rev(1));
    for s in 4..40 {
        assert_eq!(
            decide(&mut e, Want::Keep, rev(1), t0 + secs(s)),
            Plan::default()
        );
    }
    // A new change starts a new count.
    assert_eq!(
        decide(&mut e, Want::Keep, rev(2), t0 + secs(40)),
        Plan::default()
    );
    assert!(decide(&mut e, Want::Keep, rev(2), t0 + secs(42)).write);
}

#[test]
fn a_document_is_not_queued_twice_while_a_write_is_running() {
    let t0 = Instant::now();
    let mut e = Entry::new(t0);
    decide(&mut e, Want::Keep, rev(1), t0);
    assert!(decide(&mut e, Want::Keep, rev(1), t0 + secs(2)).write);
    e.shared.in_flight.store(true, Ordering::SeqCst);
    for s in 3..10 {
        assert_eq!(
            decide(&mut e, Want::Keep, rev(2), t0 + secs(s)),
            Plan::default()
        );
    }
    // When the write is over, the later change is written at once: it has been still long enough.
    e.shared.in_flight.store(false, Ordering::SeqCst);
    *e.shared.written.lock().unwrap() = Some(rev(1));
    assert!(decide(&mut e, Want::Keep, rev(2), t0 + secs(10)).write);
}

#[test]
fn a_failed_write_is_tried_again_after_five_seconds() {
    let t0 = Instant::now();
    let mut e = Entry::new(t0);
    decide(&mut e, Want::Keep, rev(1), t0);
    assert!(decide(&mut e, Want::Keep, rev(1), t0 + secs(2)).write);
    e.shared.in_flight.store(false, Ordering::SeqCst);
    e.shared.failed.store(true, Ordering::SeqCst);
    assert_eq!(
        decide(
            &mut e,
            Want::Keep,
            rev(1),
            t0 + secs(2) + RETRY - Duration::from_millis(1)
        ),
        Plan::default()
    );
    assert!(decide(&mut e, Want::Keep, rev(1), t0 + secs(2) + RETRY).write);
}

#[test]
fn a_document_that_is_no_longer_kept_has_its_copy_discarded_once() {
    let t0 = Instant::now();
    let mut e = Entry::new(t0);
    // Nothing to discard before a copy exists.
    assert_eq!(decide(&mut e, Want::Drop, rev(0), t0), Plan::default());
    e.id = Some("a".repeat(32));
    let plan = decide(&mut e, Want::Drop, rev(2), t0 + secs(1));
    assert!(plan.discard && !plan.write);
    e.forget_copy();
    assert_eq!(
        decide(&mut e, Want::Drop, rev(2), t0 + secs(2)),
        Plan::default()
    );
}

#[test]
fn a_document_that_is_too_big_is_discarded_and_the_user_is_told_once() {
    let t0 = Instant::now();
    let mut e = Entry::new(t0);
    e.id = Some("b".repeat(32));
    let plan = decide(&mut e, Want::TooBig, rev(1), t0);
    assert!(plan.discard && plan.warn_too_big && !plan.write);
    e.forget_copy();
    assert_eq!(
        decide(&mut e, Want::TooBig, rev(2), t0 + secs(1)),
        Plan::default()
    );
    // Saved, then big and unsaved again: told again.
    assert_eq!(
        decide(&mut e, Want::Drop, rev(3), t0 + secs(2)),
        Plan::default()
    );
    assert!(decide(&mut e, Want::TooBig, rev(4), t0 + secs(3)).warn_too_big);
}

#[test]
fn a_change_of_the_encoding_alone_is_a_new_revision() {
    let t0 = Instant::now();
    let mut e = Entry::new(t0);
    decide(&mut e, Want::Keep, rev(1), t0);
    assert!(decide(&mut e, Want::Keep, rev(1), t0 + secs(2)).write);
    e.shared.in_flight.store(false, Ordering::SeqCst);
    *e.shared.written.lock().unwrap() = Some(rev(1));
    let mut sjis = rev(1);
    sjis.encoding = Encoding::ShiftJis;
    assert_eq!(
        decide(&mut e, Want::Keep, sjis, t0 + secs(3)),
        Plan::default()
    );
    assert!(decide(&mut e, Want::Keep, sjis, t0 + secs(5)).write);
}

fn info() -> DocInfo {
    DocInfo {
        encoding: "UTF-8".to_owned(),
        line_ending: "LF".to_owned(),
        ..DocInfo::default()
    }
}

fn snapshot(text: &str) -> Snapshot {
    Snapshot {
        doc: Document::from_utf8(text.as_bytes().to_vec()).unwrap(),
        info: info(),
    }
}

fn read_back(store_dir: &std::path::Path, id: &str) -> String {
    let store = Store::open(store_dir).unwrap();
    let mut doc = store.open_doc(id).unwrap();
    let mut text = String::new();
    doc.text.read_to_string(&mut text).unwrap();
    text
}

#[test]
fn the_writer_writes_a_document_and_the_session() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start(&dir).unwrap();
    let shared = std::sync::Arc::new(super::writer::Shared::default());
    let id = Store::new_id();
    writer.write(id.clone(), &shared, rev(1), snapshot("日本語\r\nabc"));
    writer.session(Session {
        windows: vec![SessionWindow {
            docs: vec![id.clone()],
            active: 0,
        }],
        active_window: 0,
    });
    writer.flush();
    assert_eq!(shared.written(), Some(rev(1)));
    assert!(!shared.in_flight.load(Ordering::SeqCst));
    assert!(!writer.failed());
    drop(writer);
    assert_eq!(read_back(&dir, &id), "日本語\r\nabc");
}

#[test]
fn a_discard_queued_after_a_write_leaves_no_file_and_no_entry() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start(&dir).unwrap();
    let shared = std::sync::Arc::new(super::writer::Shared::default());
    let id = Store::new_id();
    writer.write(id.clone(), &shared, rev(1), snapshot("old text"));
    writer.session(Session {
        windows: vec![SessionWindow {
            docs: vec![id.clone()],
            active: 0,
        }],
        active_window: 0,
    });
    // The user saves: the discard is queued behind the write that may still be running.
    writer.discard(id.clone());
    writer.flush();
    drop(writer);
    let store = Store::open(&dir).unwrap();
    assert!(!store.doc_path(&id).exists());
    assert_eq!(store.read_session(), Session::default());
}

#[test]
fn the_writer_holds_the_lock_until_it_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start(&dir).unwrap();
    assert!(matches!(Writer::start(&dir), Err(StoreError::Locked)));
    drop(writer);
    Writer::start(&dir).unwrap();
}

#[test]
fn a_failed_write_is_reported_and_keeps_the_old_file() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start(&dir).unwrap();
    let shared = std::sync::Arc::new(super::writer::Shared::default());
    let id = Store::new_id();
    writer.write(id.clone(), &shared, rev(1), snapshot("first"));
    writer.flush();
    // An information that cannot be written (a line break in the path) fails the write.
    let mut bad = snapshot("second");
    bad.info.path = Some("a\nb".to_owned());
    writer.write(id.clone(), &shared, rev(2), bad);
    writer.flush();
    assert!(shared.failed.load(Ordering::SeqCst));
    assert!(writer.failed());
    assert_eq!(shared.written(), Some(rev(1)));
    assert!(!shared.in_flight.load(Ordering::SeqCst));
    drop(writer);
    assert_eq!(read_back(&dir, &id), "first");
}

#[test]
fn clear_removes_the_copies() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start(&dir).unwrap();
    let shared = std::sync::Arc::new(super::writer::Shared::default());
    let id = Store::new_id();
    writer.write(id.clone(), &shared, rev(1), snapshot("x"));
    writer.clear();
    writer.flush();
    drop(writer);
    let store = Store::open(&dir).unwrap();
    assert!(!store.doc_path(&id).exists());
}

fn text_of(doc: &Document) -> Vec<u8> {
    let mut out = Vec::new();
    write_text(doc, &mut out).unwrap();
    out
}

#[test]
fn the_text_keeps_every_line_ending_the_document_has() {
    let text = "a\r\nb\nc\rd\u{2028}e\u{85}f\u{2029}g";
    let doc = Document::from_utf8_with(text.as_bytes().to_vec(), true).unwrap();
    assert_eq!(text_of(&doc), text.as_bytes());
    assert_eq!(text_of(&Document::empty()), b"");
}

#[test]
fn an_edited_document_is_written_as_it_is_now() {
    let mut doc = Document::from_utf8(b"hello\nworld".to_vec()).unwrap();
    doc.insert(Pos::new(0, 5), " there").unwrap();
    doc.delete(Pos::new(1, 0), Pos::new(1, 1)).unwrap();
    let snapshot = doc.clone();
    // The live document goes on; the snapshot does not change.
    doc.insert(Pos::new(0, 0), ">> ").unwrap();
    assert_eq!(text_of(&snapshot), b"hello there\norld");
}

#[test]
fn bytes_that_are_not_valid_in_the_encoding_are_kept_as_the_characters_the_editor_holds() {
    // The editor holds the byte 0xFF, which is not UTF-8, as a stand-in character (U+10FF00..U+10FFFF). The
    // copy has that character as ordinary 4-byte UTF-8 (not the raw byte the saver writes), so the file is
    // valid UTF-8 and the text comes back as it was held.
    let doc = Document::from_utf8(b"a\xffb".to_vec()).unwrap();
    let bytes = text_of(&doc);
    let text = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(text, "a\u{10FFFF}b");
}

#[test]
fn a_failed_write_of_the_session_is_reported_and_works_again_later() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start(&dir).unwrap();
    // A folder where the file should be makes the rename fail.
    std::fs::create_dir(dir.join("session.ini")).unwrap();
    writer.session(Session::default());
    writer.flush();
    assert!(writer.session_failed());
    assert!(writer.failed());
    std::fs::remove_dir(dir.join("session.ini")).unwrap();
    writer.session(Session::default());
    writer.flush();
    assert!(!writer.session_failed());
    assert!(!writer.failed());
}

#[test]
fn a_removal_that_failed_is_tried_again_until_it_works() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start_retrying(&dir, Duration::from_millis(30)).unwrap();
    let id = Store::new_id();
    let path = dir.join(format!("{id}.doc"));
    // A folder with a file in it where the copy should be: it cannot be removed as a file.
    std::fs::create_dir_all(path.join("x")).unwrap();
    writer.discard(id.clone());
    writer.flush();
    assert!(writer.failed(), "the removal failed");
    // The obstacle goes; the writer finds the copy gone at its next try.
    std::fs::remove_dir_all(&path).unwrap();
    let start = Instant::now();
    while writer.failed() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!writer.failed());
}

#[test]
fn a_removal_is_tried_again_also_while_the_queue_is_never_idle() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start_retrying(&dir, Duration::from_millis(50)).unwrap();
    let id = Store::new_id();
    let path = dir.join(format!("{id}.doc"));
    std::fs::create_dir_all(path.join("x")).unwrap();
    writer.discard(id);
    writer.flush();
    assert!(writer.failed());
    std::fs::remove_dir_all(&path).unwrap();
    // A job every 10 ms: the wait for a job never times out.
    let start = Instant::now();
    while writer.failed() && start.elapsed() < Duration::from_secs(5) {
        writer.session(Session::default());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!writer.failed());
}

#[test]
fn a_failure_is_not_reported_as_over_by_a_wait_that_retried_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start_retrying(&dir, Duration::from_millis(20)).unwrap();
    std::fs::create_dir(dir.join("session.ini")).unwrap();
    writer.session(Session::default());
    writer.flush();
    assert!(writer.failed());
    // Several waits of the writer pass without a job; nothing was retried, so nothing has recovered.
    std::thread::sleep(Duration::from_millis(200));
    assert!(writer.failed());
}

#[test]
fn the_files_a_clear_removes_are_listed_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("unsaved")).unwrap();
    let id = Store::new_id();
    store
        .write_doc(&id, &info(), &mut |w| w.write_all(b"x"))
        .unwrap();
    std::fs::write(store.dir().join("notes.doc"), b"not ours").unwrap();
    std::fs::write(store.dir().join(format!("{id}.doc.1.tmp")), b"x").unwrap();
    std::fs::create_dir(store.dir().join("rejected")).unwrap();
    std::fs::write(store.dir().join("rejected").join("anything"), b"x").unwrap();
    let (targets, unreadable) = super::writer::clear_targets(&store);
    assert!(unreadable.is_empty());
    let mut names: Vec<String> = targets
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut want = vec![
        format!("{id}.doc"),
        format!("{id}.doc.1.tmp"),
        "anything".to_owned(),
    ];
    want.sort();
    assert_eq!(names, want);
}

#[cfg(unix)]
#[test]
fn a_clear_that_cannot_remove_a_rejected_file_goes_on_trying_for_it_alone() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start_retrying(&dir, Duration::from_millis(30)).unwrap();
    let rejected = dir.join("rejected");
    std::fs::create_dir_all(&rejected).unwrap();
    std::fs::write(rejected.join("old.doc"), b"x").unwrap();
    // A folder that cannot be changed: its files cannot be removed.
    std::fs::set_permissions(&rejected, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(rejected.join("probe"), b"x").is_ok() {
        // The permissions are not enforced (running as root): nothing to test.
        std::fs::set_permissions(&rejected, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    writer.clear();
    writer.flush();
    assert!(writer.failed(), "the clear is not complete");
    assert!(rejected.join("old.doc").exists());
    // A copy that is written after the clear is not touched by the tries.
    let shared = std::sync::Arc::new(super::writer::Shared::default());
    let id = Store::new_id();
    writer.write(id.clone(), &shared, rev(1), snapshot("new copy"));
    writer.flush();
    std::fs::set_permissions(&rejected, std::fs::Permissions::from_mode(0o755)).unwrap();
    let start = Instant::now();
    while writer.failed() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!writer.failed());
    assert!(!rejected.join("old.doc").exists());
    drop(writer);
    assert_eq!(read_back(&dir, &id), "new copy");
}

#[cfg(unix)]
#[test]
fn a_clear_that_cannot_read_rejected_goes_on_trying() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().join("unsaved");
    let writer = Writer::start_retrying(&dir, Duration::from_millis(30)).unwrap();
    let rejected = dir.join("rejected");
    std::fs::create_dir_all(&rejected).unwrap();
    std::fs::write(rejected.join("old.doc"), b"x").unwrap();
    std::fs::set_permissions(&rejected, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read_dir(&rejected).is_ok() {
        // Not enforced (running as root): nothing to test.
        std::fs::set_permissions(&rejected, std::fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }
    writer.clear();
    writer.flush();
    assert!(writer.failed(), "an unreadable folder is not a clean clear");
    std::fs::set_permissions(&rejected, std::fs::Permissions::from_mode(0o755)).unwrap();
    let start = Instant::now();
    while writer.failed() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!writer.failed());
    assert!(!rejected.join("old.doc").exists());
}

#[cfg(unix)]
fn session_stuck_setup(dir: &std::path::Path) -> Option<Writer> {
    use std::os::unix::fs::PermissionsExt;
    let writer = Writer::start_retrying(dir, Duration::from_millis(30)).unwrap();
    writer.session(Session::default());
    writer.flush();
    assert!(dir.join("session.ini").exists());
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(dir.join("probe"), b"x").is_ok() {
        // Not enforced (running as root): nothing to test.
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        return None;
    }
    writer.clear();
    writer.flush();
    assert!(writer.failed(), "session.ini could not be removed");
    Some(writer)
}

#[cfg(unix)]
#[test]
fn a_session_that_a_clear_could_not_remove_is_removed_at_a_later_try() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("unsaved");
    let Some(writer) = session_stuck_setup(&dir) else {
        return;
    };
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let start = Instant::now();
    while writer.failed() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(!writer.failed());
    assert!(!dir.join("session.ini").exists());
}

#[cfg(unix)]
#[test]
fn a_session_written_after_the_feature_is_on_again_is_not_removed_by_the_retry() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("unsaved");
    let Some(writer) = session_stuck_setup(&dir) else {
        return;
    };
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    // The feature is on again and the keeper writes its session before the retry comes.
    writer.session(Session::default());
    writer.flush();
    std::thread::sleep(Duration::from_millis(200));
    assert!(dir.join("session.ini").exists());
}
