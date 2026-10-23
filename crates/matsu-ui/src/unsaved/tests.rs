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
