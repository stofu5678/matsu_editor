use std::fs;
use std::io::Read;
use std::time::{Duration, SystemTime};

use super::*;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("unsaved")).unwrap();
    (dir, store)
}

fn info() -> DocInfo {
    DocInfo {
        untitled_number: None,
        path: Some("C:\\Users\\山田 太郎\\メモ=1.txt".to_owned()),
        file_modified_secs: Some(1_790_000_000),
        file_modified_nanos: Some(123_456_789),
        file_size: Some(4096),
        encoding: "Shift_JIS".to_owned(),
        bom: true,
        line_ending: "CRLF".to_owned(),
        doc_type: Some(3),
        caret_line: 12,
        caret_column: 7,
        top_line: 4,
        bookmarks: vec![1, 5, 9],
    }
}

fn write(store: &Store, id: &str, info: &DocInfo, text: &str) {
    store
        .write_doc(id, info, &mut |w| w.write_all(text.as_bytes()))
        .unwrap();
}

fn read_text(store: &Store, id: &str) -> (DocInfo, String) {
    let mut doc = store.open_doc(id).unwrap();
    let mut text = String::new();
    doc.text.read_to_string(&mut text).unwrap();
    assert_eq!(text.len() as u64, doc.text_len);
    (doc.info, text)
}

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn a_document_comes_back_with_its_text_and_information() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    let text = "日本語の行\r\n二行目\r\n\r\n";
    write(&store, &id, &info(), text);
    let (got_info, got_text) = read_text(&store, &id);
    assert_eq!(got_info, info());
    assert_eq!(got_text, text);
}

#[test]
fn an_untitled_document_and_an_empty_text() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    let i = DocInfo {
        untitled_number: Some(3),
        encoding: "UTF-8".to_owned(),
        line_ending: "LF".to_owned(),
        ..DocInfo::default()
    };
    write(&store, &id, &i, "");
    let (got, text) = read_text(&store, &id);
    assert_eq!(got.untitled_number, Some(3));
    assert_eq!(got.path, None);
    assert!(got.bookmarks.is_empty());
    assert_eq!(text, "");
}

#[test]
fn text_that_looks_like_a_header_is_kept() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    let text = "MATSU-UNSAVED 1\n12\n[Doc]\nPath=x\n";
    write(&store, &id, &info(), text);
    assert_eq!(read_text(&store, &id).1, text);
}

#[test]
fn the_text_can_be_written_in_pieces() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    store
        .write_doc(&id, &info(), &mut |w| {
            for _ in 0..1000 {
                w.write_all("あいう\n".as_bytes())?;
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(read_text(&store, &id).1.lines().count(), 1000);
}

#[test]
fn a_new_write_replaces_the_text_and_the_information_together() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "old");
    let mut utf8 = info();
    utf8.encoding = "UTF-8".to_owned();
    write(&store, &id, &utf8, "new text");
    let (got, text) = read_text(&store, &id);
    assert_eq!(
        (got.encoding.as_str(), text.as_str()),
        ("UTF-8", "new text")
    );
}

#[test]
fn a_failed_write_keeps_the_old_file_and_leaves_no_temporary_file() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "old");
    let result = store.write_doc(&id, &info(), &mut |w| {
        w.write_all(b"half")?;
        Err(std::io::Error::other("disk full"))
    });
    assert!(result.is_err());
    assert_eq!(read_text(&store, &id).1, "old");
    assert!(names(store.dir()).iter().all(|n| !n.ends_with(".tmp")));
}

#[test]
fn a_write_cut_off_before_the_rename_leaves_only_a_temporary_file() {
    // The process died after the temporary file was written and before the rename.
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "complete");
    fs::write(store.dir().join(format!("{id}.doc.7.tmp")), b"MATSU-UNS").unwrap();
    // The real file is still the complete one, and the scan does not call the temporary file a document.
    assert_eq!(read_text(&store, &id).1, "complete");
    let session = Session {
        windows: vec![SessionWindow {
            docs: vec![id.clone()],
            active: 0,
        }],
        active_window: 0,
    };
    assert_eq!(store.scan(&session).unwrap(), Scan::default());
    assert_eq!(store.remove_tmp().unwrap(), 1);
    assert_eq!(read_text(&store, &id).1, "complete");
}

#[test]
fn a_new_document_cut_off_before_the_session_is_an_orphan() {
    let (_tmp, store) = store();
    let (listed, orphan) = (Store::new_id(), Store::new_id());
    write(&store, &listed, &info(), "a");
    write(&store, &orphan, &info(), "b");
    let session = Session {
        windows: vec![SessionWindow {
            docs: vec![listed],
            active: 0,
        }],
        active_window: 0,
    };
    let scan = store.scan(&session).unwrap();
    assert_eq!(scan.orphans, vec![orphan]);
    assert!(scan.dangling.is_empty());
}

#[test]
fn a_removal_cut_off_before_the_session_leaves_a_dangling_entry_not_an_orphan() {
    // discard_doc deletes the file first. If the process died then, the session still lists the id.
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "a");
    let session = Session {
        windows: vec![SessionWindow {
            docs: vec![id.clone()],
            active: 0,
        }],
        active_window: 0,
    };
    store.write_session(&session).unwrap();
    store.remove_doc(&id).unwrap();
    let scan = store.scan(&session).unwrap();
    assert_eq!(scan.dangling, vec![id]);
    assert!(scan.orphans.is_empty());
}

#[test]
fn discarding_a_document_removes_its_file_and_its_entry() {
    let (_tmp, store) = store();
    let (a, b) = (Store::new_id(), Store::new_id());
    write(&store, &a, &info(), "a");
    write(&store, &b, &info(), "b");
    let session = Session {
        windows: vec![SessionWindow {
            docs: vec![a.clone(), b.clone()],
            active: 1,
        }],
        active_window: 0,
    };
    store.write_session(&session).unwrap();
    let next = store.discard_doc(&a, &session).unwrap();
    assert_eq!(next.windows[0].docs, vec![b.clone()]);
    assert_eq!(store.read_session(), next);
    assert!(!store.doc_path(&a).exists());
    assert!(store.doc_path(&b).exists());
    // Removing again is not an error.
    store.remove_doc(&a).unwrap();
}

#[test]
fn the_session_keeps_windows_tabs_and_the_front_ones() {
    let (_tmp, store) = store();
    let ids: Vec<String> = (0..4).map(|_| Store::new_id()).collect();
    let session = Session {
        windows: vec![
            SessionWindow {
                docs: vec![ids[0].clone(), ids[1].clone()],
                active: 1,
            },
            SessionWindow {
                docs: vec![ids[2].clone(), ids[3].clone()],
                active: 0,
            },
        ],
        active_window: 1,
    };
    store.write_session(&session).unwrap();
    assert_eq!(store.read_session(), session);
}

#[test]
fn removing_the_last_document_of_a_window_removes_the_window() {
    let (a, b, c) = (Store::new_id(), Store::new_id(), Store::new_id());
    let session = Session {
        windows: vec![
            SessionWindow {
                docs: vec![a.clone()],
                active: 0,
            },
            SessionWindow {
                docs: vec![b.clone(), c.clone()],
                active: 1,
            },
        ],
        active_window: 1,
    };
    let next = session.without(&a);
    assert_eq!(next.windows.len(), 1);
    assert_eq!(next.active_window, 0);
    assert_eq!(next.windows[0].active, 1);
    let next = next.without(&b);
    assert_eq!(next.windows[0].docs, vec![c]);
    assert_eq!(next.windows[0].active, 0);
}

#[test]
fn a_damaged_session_cannot_name_other_files() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    fs::write(
        store.dir().join("session.ini"),
        format!("[Session]\nWindows=99999999999\nActiveWindow=x\nWindow0.Docs=..\\x,../../etc/passwd,{id},ABC\nWindow0.Active=40\n"),
    )
    .unwrap();
    let session = store.read_session();
    assert_eq!(session.windows.len(), 1);
    assert_eq!(session.windows[0].docs, vec![id]);
    assert_eq!(session.windows[0].active, 0);
    // A missing file is an empty session.
    fs::remove_file(store.dir().join("session.ini")).unwrap();
    assert_eq!(store.read_session(), Session::default());
}

#[test]
fn a_bad_id_is_refused() {
    let (_tmp, store) = store();
    assert!(store.write_doc("../x", &info(), &mut |_| Ok(())).is_err());
    assert!(store.remove_doc("x").is_err());
    assert!(store.open_doc("..").is_err());
}

#[test]
fn ids_are_valid_and_different() {
    let a = Store::new_id();
    let b = Store::new_id();
    assert!(is_valid_id(&a) && is_valid_id(&b));
    assert_ne!(a, b);
}

#[test]
fn a_damaged_document_file_is_an_error_not_a_panic() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    let path = store.doc_path(&id);
    for bytes in [
        &b""[..],
        b"hello\n",
        b"MATSU-UNSAVED 1\n",
        b"MATSU-UNSAVED 1\nabc\n",
        b"MATSU-UNSAVED 1\n99999999999\n",
        b"MATSU-UNSAVED 1\n50\n[Doc]\n",
        b"MATSU-UNSAVED 2\n0\n",
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(
            matches!(store.open_doc(&id), Err(FormatError::Damaged(_))),
            "{bytes:?}"
        );
    }
    assert!(matches!(
        store.open_doc(&Store::new_id()),
        Err(FormatError::Io(_))
    ));
}

#[test]
fn a_second_store_on_the_same_folder_is_refused_until_the_first_ends() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("unsaved");
    let first = Store::open(&dir).unwrap();
    assert!(matches!(Store::open(&dir), Err(StoreError::Locked)));
    drop(first);
    Store::open(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn the_folder_and_the_files_are_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt;
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "secret");
    store.write_session(&Session::default()).unwrap();
    let mode = |p: &std::path::Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(store.dir()), 0o700);
    assert_eq!(mode(&store.doc_path(&id)), 0o600);
    assert_eq!(mode(&store.dir().join("session.ini")), 0o600);
    assert_eq!(mode(&store.dir().join("lock")), 0o600);
}

#[test]
fn a_rejected_document_is_kept_for_thirty_days() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "damaged?");
    store.reject(&id).unwrap();
    assert!(!store.doc_path(&id).exists());
    let kept = store.dir().join("rejected").join(format!("{id}.doc"));
    assert!(kept.exists());
    let now = SystemTime::now();
    assert_eq!(store.purge_rejected(now, REJECTED_KEEP).unwrap(), 0);
    assert!(kept.exists());
    let later = now + REJECTED_KEEP + Duration::from_secs(60);
    assert_eq!(store.purge_rejected(later, REJECTED_KEEP).unwrap(), 1);
    assert!(!kept.exists());
    // No folder is no error.
    let (_tmp2, other) = self::store();
    assert_eq!(other.purge_rejected(later, REJECTED_KEEP).unwrap(), 0);
}

#[test]
fn clear_removes_ours_and_keeps_the_lock_and_other_files() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "a");
    store.write_session(&Session::default()).unwrap();
    store.reject(&Store::new_id()).ok();
    let other = Store::new_id();
    write(&store, &other, &info(), "b");
    store.reject(&other).unwrap();
    fs::write(store.dir().join("notes.txt"), b"not ours").unwrap();
    fs::write(store.dir().join(format!("{id}.doc.1.tmp")), b"x").unwrap();
    store.clear().unwrap();
    assert_eq!(names(store.dir()), vec!["lock", "notes.txt"]);
    // The lock is still held: nobody else can take the folder.
    assert!(matches!(Store::open(store.dir()), Err(StoreError::Locked)));
}

#[test]
fn the_folder_is_next_to_the_settings_file() {
    let dir = unsaved_dir(std::path::Path::new(
        "/home/u/.config/matsu-editor/matsu.ini",
    ));
    assert_eq!(
        dir,
        std::path::Path::new("/home/u/.config/matsu-editor/unsaved")
    );
}

#[test]
fn information_that_cannot_be_written_is_an_error() {
    let (_tmp, store) = store();
    let mut bad = info();
    bad.path = Some("a\nb".to_owned());
    let id = Store::new_id();
    assert!(store.write_doc(&id, &bad, &mut |_| Ok(())).is_err());
    assert!(!store.doc_path(&id).exists());
}

#[test]
fn unreadable_information_falls_back_to_defaults() {
    let info = DocInfo::from_bytes(b"[Doc]\nCaretLine=abc\nBookmarks=1,x,3\nEncoding=UTF-8\n");
    assert_eq!(info.caret_line, 0);
    assert_eq!(info.bookmarks, vec![1, 3]);
    assert_eq!(info.line_ending, "LF");
}

#[test]
fn an_old_document_is_kept_30_days_from_the_day_it_is_rejected() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "old");
    let long_ago = SystemTime::now() - Duration::from_secs(90 * 24 * 60 * 60);
    fs::File::options()
        .write(true)
        .open(store.doc_path(&id))
        .unwrap()
        .set_modified(long_ago)
        .unwrap();
    store.reject(&id).unwrap();
    assert_eq!(
        store
            .purge_rejected(SystemTime::now(), REJECTED_KEEP)
            .unwrap(),
        0
    );
}

#[test]
fn information_too_long_to_read_back_is_refused_before_the_old_file_is_replaced() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    write(&store, &id, &info(), "good");
    let mut huge = info();
    huge.bookmarks = (0..200_000).collect();
    assert!(store.write_doc(&id, &huge, &mut |_| Ok(())).is_err());
    assert_eq!(read_text(&store, &id).1, "good");
}

#[test]
fn a_header_that_is_not_text_or_has_no_line_break_is_damaged() {
    let (_tmp, store) = store();
    let id = Store::new_id();
    for bytes in [vec![0xff, 0xfe, b'\n'], vec![b'a'; 10_000]] {
        fs::write(store.doc_path(&id), bytes).unwrap();
        assert!(matches!(store.open_doc(&id), Err(FormatError::Damaged(_))));
    }
}
