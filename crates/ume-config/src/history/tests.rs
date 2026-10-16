use std::path::PathBuf;

use super::*;

/// What Sakura writes (`ShareData_IO_Mru`), with Japanese text, a Windows path with a space and a
/// UNC path.
const SAKURA_INI: &str = "\
[Common]
nMRUArrNum_MAX=3
nOPENFOLDERArrNum_MAX=2
[MRU]
_MRU_Counts=3
MRU[00].nViewTopLine=4
MRU[00].nViewLeftCol=1
MRU[00].nX=7
MRU[00].nY=12
MRU[00].nCharCode=4
MRU[00].szPath=C:\\Users\\山田 太郎\\Documents\\メモ.txt
MRU[00].szMark2=:0a
MRU[00].nType=2
MRU[00].bFavorite=1
MRU[01].nViewTopLine=0
MRU[01].nViewLeftCol=0
MRU[01].nX=0
MRU[01].nY=0
MRU[01].nCharCode=0
MRU[01].szPath=\\\\server\\share\\日報=2026.log
MRU[01].szMark2=:0
MRU[01].nType=-1
MRU[01].bFavorite=0
MRU[02].nViewTopLine=0
MRU[02].nViewLeftCol=0
MRU[02].nX=3
MRU[02].nY=1
MRU[02].nCharCode=99
MRU[02].szPath=D:\\work\\old.txt
MRU[02].szMark2=:0
MRU[02].nType=-1
MRU[02].bFavorite=0
_MRUFOLDER_Counts=2
MRUFOLDER[00]=C:\\Users\\山田 太郎\\Documents\\
MRUFOLDER[00].bFavorite=0
MRUFOLDER[01]=\\\\server\\share\\
MRUFOLDER[01].bFavorite=1
_ExceptMRU_Counts=1
ExceptMRU[00]=\\Temp\\
[Other]
x=1
";

/// The file as Sakura writes it: UTF-8 with a BOM.
fn sakura_bytes() -> Vec<u8> {
    [b"\xEF\xBB\xBF".as_slice(), SAKURA_INI.as_bytes()].concat()
}

fn sakura() -> History {
    History::from_profile(&Profile::from_bytes(&sakura_bytes()), true)
}

fn paths(h: &History) -> Vec<&str> {
    h.files.items().iter().map(|f| f.path.as_str()).collect()
}

#[test]
fn what_sakura_wrote_is_read() {
    let h = sakura();
    assert_eq!(h.files.view_count(), 3);
    assert_eq!(h.folders.view_count(), 2);
    assert_eq!(
        paths(&h),
        [
            "C:\\Users\\山田 太郎\\Documents\\メモ.txt",
            "\\\\server\\share\\日報=2026.log",
            "D:\\work\\old.txt"
        ]
    );
    let first = h.file("c:\\users\\山田 太郎\\documents\\メモ.txt").unwrap();
    assert_eq!(
        (
            first.caret_x,
            first.caret_y,
            first.view_top,
            first.view_left
        ),
        (7, 12, 4, 1)
    );
    assert_eq!(first.encoding_code, 4);
    assert_eq!(first.marks, ":0a");
    assert_eq!(first.type_id, 2);
    assert!(first.favorite);
    assert_eq!(h.file("D:\\work\\old.txt").unwrap().encoding_code, 99);
    assert_eq!(h.folder_paths().len(), 2);
    assert!(h.folders.is_favorite(1));
    assert_eq!(h.except, ["\\Temp\\"]);
}

#[test]
fn writing_what_was_read_changes_no_byte() {
    let mut profile = Profile::from_bytes(&sakura_bytes());
    let h = History::from_profile(&profile, true);
    h.write_to(&mut profile).unwrap();
    assert_eq!(profile.to_bytes(), sakura_bytes());
}

#[test]
fn a_changed_history_is_written_and_read_back() {
    let mut profile = Profile::from_bytes(&sakura_bytes());
    let mut h = History::from_profile(&profile, true);
    let mut entry = FileEntry::new("E:\\新しい フォルダー\\テスト.txt");
    entry.encoding_code = 3;
    entry.caret_x = 2;
    entry.caret_y = 40;
    entry.view_top = 30;
    entry.marks = marks::encode_marks([1, 5, 40]);
    h.add_file(entry.clone());
    h.write_to(&mut profile).unwrap();
    let back = History::from_profile(&Profile::from_bytes(&profile.to_bytes()), true);
    assert_eq!(back, h);
    assert_eq!(back.file(&entry.path), Some(&entry));
    // The part of the file that is not the history is as it was.
    assert_eq!(profile.get("Other", "x"), Some("1"));
    assert_eq!(profile.get_i32("Common", "nMRUArrNum_MAX"), Some(3));
}

#[test]
fn entries_that_are_gone_leave_no_keys() {
    let mut profile = Profile::from_bytes(&sakura_bytes());
    let mut h = History::from_profile(&profile, true);
    h.files.clear();
    h.folders.delete(1);
    h.write_to(&mut profile).unwrap();
    let keys: Vec<_> = profile.entries("MRU").map(|(k, _)| k).collect();
    assert_eq!(
        keys,
        [
            "_MRU_Counts",
            "_MRUFOLDER_Counts",
            "MRUFOLDER[00]",
            "MRUFOLDER[00].bFavorite",
            "_ExceptMRU_Counts",
            "ExceptMRU[00]"
        ]
    );
    let back = History::from_profile(&profile, true);
    assert!(back.files.is_empty());
    assert_eq!(back.folder_paths(), ["C:\\Users\\山田 太郎\\Documents\\"]);
}

#[test]
fn without_settings_15_are_shown_and_36_at_most() {
    let h = History::from_profile(&Profile::new(), true);
    assert_eq!(h.files.view_count(), 15);
    assert_eq!(h.folders.view_count(), 15);
    let mut p = Profile::new();
    p.set("Common", "nMRUArrNum_MAX", "99").unwrap();
    p.set("Common", "nOPENFOLDERArrNum_MAX", "-4").unwrap();
    let h = History::from_profile(&p, true);
    assert_eq!(h.files.view_count(), 36);
    assert_eq!(h.folders.view_count(), 0);
}

#[test]
fn the_shown_files_are_the_first_n() {
    let mut h = History::new(2, 2, true);
    for n in 1..=4 {
        h.add_file(FileEntry::new(format!("/home/a/{n}.txt")));
    }
    assert_eq!(h.file_paths(), ["/home/a/4.txt", "/home/a/3.txt"]);
    assert_eq!(h.files.len(), 4);
    // The text that is inserted has the line ending after each, the last one too.
    assert_eq!(h.files_text("\r\n"), "/home/a/4.txt\r\n/home/a/3.txt\r\n");
    assert_eq!(h.folders_text("\n"), "/home/a/\n");
    assert_eq!(History::new(5, 5, true).files_text("\n"), "");
}

#[test]
fn the_most_recent_36_are_kept() {
    let mut h = History::new(36, 36, true);
    for n in 0..50 {
        h.add_file(FileEntry::new(format!("/d{n}/f.txt")));
    }
    assert_eq!(h.files.len(), 36);
    assert_eq!(h.folders.len(), 36);
    assert_eq!(h.files.get(0).unwrap().path, "/d49/f.txt");
    assert_eq!(h.files.get(35).unwrap().path, "/d14/f.txt");
}

#[test]
fn a_file_also_adds_its_folder() {
    let mut h = History::new(5, 5, true);
    h.add_file(FileEntry::new("C:\\a\\b\\c.txt"));
    h.add_file(FileEntry::new("\\\\srv\\share\\d.txt"));
    h.add_file(FileEntry::new("/home/x/e.txt"));
    h.add_file(FileEntry::new("memo.txt"));
    h.add_file(FileEntry::new("D:memo.txt"));
    h.add_file(FileEntry::new(""));
    assert_eq!(
        h.folder_paths(),
        ["D:", "/home/x/", "\\\\srv\\share\\", "C:\\a\\b\\"]
    );
    assert_eq!(h.files.len(), 5);
}

#[test]
fn folder_of_keeps_the_last_separator() {
    assert_eq!(folder_of("C:\\a\\b.txt"), "C:\\a\\");
    assert_eq!(folder_of("/a/b.txt"), "/a/");
    assert_eq!(folder_of("C:/a\\b.txt"), "C:/a\\");
    assert_eq!(folder_of("b.txt"), "");
    assert_eq!(folder_of("C:b.txt"), "C:");
    assert_eq!(folder_of("日本語/メモ.txt"), "日本語/");
}

#[test]
fn excluded_paths_are_not_added_unless_they_are_there() {
    let mut h = History::new(5, 5, true);
    h.except = vec!["\\TEMP\\".to_owned(), String::new()];
    h.add_file(FileEntry::new("C:\\Users\\a\\AppData\\Local\\Temp\\x.txt"));
    assert!(h.files.is_empty());
    assert!(h.folders.is_empty());
    h.add_file(FileEntry::new("C:\\work\\y.txt"));
    assert_eq!(h.files.len(), 1);
    // A file that is in the history stays when the list gets a new entry.
    let mut h = History::new(5, 5, true);
    h.add_file(FileEntry::new("C:\\temp\\z.txt"));
    h.except = vec!["\\temp\\".to_owned()];
    h.add_file(FileEntry::new("C:\\temp\\z.txt"));
    assert_eq!(h.files.len(), 1);
}

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("ume-config-history-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_store_creates_the_file_and_reads_it_back() {
    let dir = temp_dir("create");
    let store = HistoryStore::new(dir.join("sub").join("ume.ini"), true);
    assert!(store.load().files.is_empty());
    store
        .update(|h| h.add_file(FileEntry::new("C:\\メモ\\a.txt")))
        .unwrap();
    let h = store.load();
    assert_eq!(h.file_paths(), ["C:\\メモ\\a.txt"]);
    assert_eq!(h.folder_paths(), ["C:\\メモ\\"]);
    assert!(!store.lock_path().exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_store_keeps_the_rest_of_the_file_and_writes_nothing_for_no_change() {
    let dir = temp_dir("rest");
    let path = dir.join("ume.ini");
    std::fs::write(&path, sakura_bytes()).unwrap();
    let store = HistoryStore::new(&path, true);
    store.update(|_| {}).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), sakura_bytes());
    store
        .update(|h| h.add_file(FileEntry::new("D:\\new.txt")))
        .unwrap();
    let text = String::from_utf8(std::fs::read(&path).unwrap()).unwrap();
    assert!(text.contains("[Other]"));
    assert!(text.contains("nMRUArrNum_MAX=3"));
    assert!(text.contains("szPath=D:\\new.txt"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn windows_that_add_at_the_same_time_all_get_in() {
    let dir = temp_dir("threads");
    let path = dir.join("ume.ini");
    let threads: Vec<_> = (0..8)
        .map(|t| {
            let path = path.clone();
            std::thread::spawn(move || {
                // Each window has a store of its own, as each process would.
                let store = HistoryStore::new(path, true);
                for n in 0..4 {
                    store
                        .update(|h| h.add_file(FileEntry::new(format!("/t{t}/f{n}.txt"))))
                        .unwrap();
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
    let h = HistoryStore::new(&path, true).load();
    assert_eq!(h.files.len(), 32);
    assert_eq!(h.folders.len(), 8);
    for t in 0..8 {
        for n in 0..4 {
            assert!(h.file(&format!("/t{t}/f{n}.txt")).is_some(), "{t} {n}");
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_lock_left_by_a_dead_process_is_taken_over() {
    let dir = temp_dir("stale");
    let path = dir.join("ume.ini");
    let store = HistoryStore::new(&path, true);
    let lock = store.lock_path();
    let file = std::fs::File::create(&lock).unwrap();
    file.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(120))
        .unwrap();
    drop(file);
    store
        .update(|h| h.add_file(FileEntry::new("/a/b.txt")))
        .unwrap();
    assert_eq!(store.load().files.len(), 1);
    assert!(!lock.exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn recent_files_and_search_keys_written_at_the_same_time_are_both_kept() {
    use crate::settings::SearchKeys;
    let dir = temp_dir("sections");
    let path = dir.join("ume.ini");
    let store = HistoryStore::new(&path, true);
    let files = std::thread::spawn({
        let store = store.clone();
        move || {
            for n in 0..12 {
                store
                    .update(|h| h.add_file(FileEntry::new(format!("/mru/f{n}.txt"))))
                    .unwrap();
            }
        }
    });
    let keys = std::thread::spawn({
        let path = path.clone();
        move || {
            for n in 0..25 {
                update_profile(&path, |profile| {
                    let mut keys = SearchKeys::from_profile(profile);
                    keys.add_search(&format!("検索{n}"));
                    keys.write_to(profile).unwrap();
                    Ok(true)
                })
                .unwrap();
            }
        }
    });
    files.join().unwrap();
    keys.join().unwrap();
    let h = store.load();
    for n in 0..12 {
        assert!(h.file(&format!("/mru/f{n}.txt")).is_some(), "file {n}");
    }
    let profile = Profile::load(&path).unwrap();
    let keys = SearchKeys::from_profile(&profile);
    assert_eq!(keys.search.len(), 25);
    assert_eq!(keys.search.newest(), Some("検索24"));
    assert!(!store.lock_path().exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_old_bookmark_key_is_read_when_there_is_no_new_one() {
    let p =
        Profile::from_bytes(b"[MRU]\n_MRU_Counts=1\nMRU[00].szPath=a.txt\nMRU[00].szMark=1,5, 9\n");
    let h = History::from_profile(&p, true);
    assert_eq!(h.file("a.txt").unwrap().marks, "1,5, 9");
    assert_eq!(h.file("a.txt").unwrap().encoding_code, CODE_AUTODETECT);
    assert_eq!(h.file("a.txt").unwrap().type_id, -1);
}
