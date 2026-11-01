//! Tests of the page 強調キーワード: the rules of the sets, the file, and the dialog in GPUI's headless test
//! windows.

use gpui::{AnyWindowHandle, TestAppContext};
use matsu_coloring::{ColorKind, ColorState, line_runs};
use matsu_config::Profile;
use matsu_config::settings::keyword_file::KeywordFile;
use matsu_config::settings::keywords::{KeywordSets, MAX_KEYWORD_SETS};
use matsu_config::settings::{DocumentTypes, ExtraSet};

use super::entry::{
    self, Edited, Entry, MAX_WORDS, Refused, can_add_word, drafts_of, entries_of, from_rows,
    renumber, rows_of, space,
};
use super::{CASE, CURRENT, NAME, SETS, WORD, WORD_SEL, store};
use crate::common_settings::Page;
use crate::common_settings::tests::{Ini, cancel, ok, run, setup, with_dialog};

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn set(name: &str, case: bool, words: &[&str]) -> Entry {
    Entry {
        origin: None,
        name: name.to_owned(),
        case_sensitive: case,
        words: words.iter().map(|w| (*w).to_owned()).collect(),
    }
}

fn words(entry: &Entry) -> Vec<&str> {
    entry.words.iter().map(String::as_str).collect()
}

// ---- the rules of the sets ----

#[test]
fn a_set_is_a_row_and_a_row_is_a_set() {
    let mut a = set("日本語 set", true, &["if", "while", "あ"]);
    a.origin = Some(3);
    let row = a.to_row();
    assert_eq!(row, ["3", "日本語 set", "1", "if\twhile\tあ"]);
    assert_eq!(Entry::from_row(&row), a);
    // A new set is in no file; an empty one has no keywords.
    let new = set("x", false, &[]);
    assert_eq!(new.to_row(), ["+", "x", "0", ""]);
    assert_eq!(Entry::from_row(&new.to_row()), new);
}

#[test]
fn a_set_is_added_with_a_name_up_to_32_units_and_not_beyond_100_sets() {
    let mut sets = vec![set("a", true, &[])];
    // `szKeyWord[0] != 0`: an empty name adds nothing.
    assert_eq!(entry::add_set(&mut sets, ""), Ok(None));
    assert_eq!(entry::add_set(&mut sets, "\r\n"), Ok(None));
    assert_eq!(entry::add_set(&mut sets, "Mine"), Ok(Some(1)));
    // The new set ignores the case (`AddKeyWordSet(name, false)`) and has no keyword.
    assert!(!sets[1].case_sensitive && sets[1].words.is_empty() && sets[1].origin.is_none());
    // `MAX_SETNAMELEN` units of UTF-16; a pair of units is not split.
    entry::add_set(&mut sets, &format!("{}😀", "a".repeat(31))).unwrap();
    assert_eq!(sets[2].name, "a".repeat(31));
    entry::add_set(&mut sets, &"あ".repeat(40)).unwrap();
    assert_eq!(sets[3].name.chars().count(), 32);
    // The same name twice is allowed.
    assert_eq!(entry::add_set(&mut sets, "Mine"), Ok(Some(4)));
    while sets.len() < MAX_KEYWORD_SETS {
        entry::add_set(&mut sets, "n").unwrap();
    }
    assert_eq!(entry::add_set(&mut sets, "n"), Err(Refused::TooManySets));
    assert_eq!(sets.len(), MAX_KEYWORD_SETS);
}

#[test]
fn a_set_is_renamed_unless_the_name_is_empty() {
    let mut sets = vec![set("a", true, &[]), set("b", true, &[])];
    assert!(entry::rename_set(&mut sets, 1, "Renamed"));
    assert_eq!(sets[1].name, "Renamed");
    assert!(!entry::rename_set(&mut sets, 1, ""));
    assert!(!entry::rename_set(&mut sets, 7, "x"));
    assert_eq!(sets[1].name, "Renamed");
    assert!(entry::remove_set(&mut sets, 0));
    assert!(!entry::remove_set(&mut sets, 5));
    assert_eq!(sets.len(), 1);
}

#[test]
fn a_keyword_is_added_unless_it_is_empty_or_a_duplicate() {
    let mut sets = vec![set("a", false, &["Alpha"])];
    assert_eq!(entry::add_word(&mut sets, 0, "beta"), Ok(true));
    assert_eq!(entry::add_word(&mut sets, 0, ""), Ok(false));
    // `wcscmp`: a duplicate is the same text, even when the set ignores the case.
    assert_eq!(entry::add_word(&mut sets, 0, "beta"), Ok(false));
    assert_eq!(entry::add_word(&mut sets, 0, "ALPHA"), Ok(true));
    // A tab and a line break cannot be in a keyword.
    assert_eq!(entry::add_word(&mut sets, 0, "a\tb"), Ok(false));
    assert_eq!(entry::add_word(&mut sets, 0, "a\nb"), Ok(false));
    assert_eq!(words(&sets[0]), ["Alpha", "beta", "ALPHA"]);
    // A keyword is cut to 63 units (`MAX_KEYWORDLEN`).
    assert_eq!(entry::add_word(&mut sets, 0, &"x".repeat(80)), Ok(true));
    assert_eq!(sets[0].words[3].len(), 63);
    // A duplicate of the cut keyword is not added.
    assert_eq!(entry::add_word(&mut sets, 0, &"x".repeat(70)), Ok(false));
}

#[test]
fn a_keyword_is_edited_deleted_or_left_when_it_is_a_duplicate() {
    let mut sets = vec![set("a", true, &["a", "b", "c"])];
    assert_eq!(entry::edit_word(&mut sets, 0, 1, "B"), Edited::Replaced);
    assert_eq!(words(&sets[0]), ["a", "B", "c"]);
    // The same text as another keyword, or as itself, changes nothing (`UpdateKeyWord`).
    assert_eq!(entry::edit_word(&mut sets, 0, 1, "c"), Edited::Unchanged);
    assert_eq!(entry::edit_word(&mut sets, 0, 1, "B"), Edited::Unchanged);
    // An empty text deletes it (`DelKeyWord`).
    assert_eq!(entry::edit_word(&mut sets, 0, 0, ""), Edited::Removed);
    assert_eq!(words(&sets[0]), ["B", "c"]);
    assert_eq!(entry::edit_word(&mut sets, 0, 9, "x"), Edited::Unchanged);
    assert!(entry::remove_word(&mut sets, 0, 1));
    assert!(!entry::remove_word(&mut sets, 0, 1));
    assert_eq!(words(&sets[0]), ["B"]);
}

#[test]
fn the_keywords_take_their_room_in_blocks_of_50_out_of_15000() {
    let many = |n: usize| -> Vec<String> { (0..n).map(|i| format!("w{i}")).collect() };
    let mut a = set("a", true, &[]);
    a.words = many(51);
    let b = set("b", true, &[]);
    let sets = vec![a, b];
    // 51 keywords have used up 100; the free room is what is left of all.
    assert_eq!(space(&sets, 0), 49 + (MAX_WORDS - 100));
    assert_eq!(space(&sets, 1), MAX_WORDS - 100);
    assert!(can_add_word(&sets, 0));
    // A set with all of its blocks used can take another keyword only if a whole block is free.
    let mut full = vec![set("a", true, &[])];
    full[0].words = many(MAX_WORDS - 25);
    assert!(can_add_word(&full, 0), "the last block has room");
    full[0].words = many(MAX_WORDS);
    assert!(!can_add_word(&full, 0));
    assert_eq!(entry::add_word(&mut full, 0, "new"), Err(Refused::NoRoom));
    // A set cannot be added when no block is free either (`AddKeyWordSet` reserves one).
    assert_eq!(entry::add_set(&mut full, "x"), Err(Refused::NoRoom));
    assert_eq!(space(&full, 0), 0);
    // The built-in sets leave room.
    let builtin = entries_of(&KeywordSets::builtin());
    assert!(space(&builtin, 0) > 10000);
}

#[test]
fn cleaning_sorts_the_keywords_and_removes_the_duplicates_of_the_case_rule() {
    // The set ignores the case: Int and INT are the same keyword (`wmemicmp`); the first is kept.
    let mut sets = vec![set(
        "a",
        false,
        &["zeta", "Int", "alpha", "INT", "int", "Alpha"],
    )];
    assert_eq!(entry::clean(&mut sets, 0), 3);
    assert_eq!(words(&sets[0]), ["alpha", "Int", "zeta"]);
    // A set that sees the case keeps them all, sorted by code unit (`wcscmp`).
    let mut sets = vec![set("a", true, &["b", "B", "a", "b"])];
    assert_eq!(entry::clean(&mut sets, 0), 1);
    assert_eq!(words(&sets[0]), ["B", "a", "b"]);
    assert_eq!(entry::clean(&mut sets, 0), 0);
}

#[test]
fn an_import_adds_the_keywords_and_takes_the_case_of_the_file() {
    let file = |case, lines: &[&str]| KeywordFile {
        case_sensitive: case,
        words: lines.iter().map(|w| (*w).to_owned()).collect(),
    };
    let mut sets = vec![set("a", true, &["x"])];
    // Duplicates are not added again; the case line of the file wins.
    assert!(!entry::import(
        &mut sets,
        0,
        &file(Some(false), &["x", "y", "z", "y"])
    ));
    assert_eq!(words(&sets[0]), ["x", "y", "z"]);
    assert!(!sets[0].case_sensitive);
    // A file without the case line makes the set ignore the case (`bCase` starts as false in Sakura).
    sets[0].case_sensitive = true;
    entry::import(&mut sets, 0, &file(None, &["w"]));
    assert!(!sets[0].case_sensitive);
    // When the room runs out the rest is left, and the answer says so (`STR_IMPEXP_KEYWORD`).
    let mut full = vec![set("a", true, &[])];
    full[0].words = (0..MAX_WORDS).map(|i| format!("w{i}")).collect();
    assert!(entry::import(
        &mut full,
        0,
        &file(Some(true), &["new1", "new2"])
    ));
    assert_eq!(full[0].words.len(), MAX_WORDS);
    assert!(full[0].case_sensitive);
}

#[test]
fn the_numbers_of_the_types_follow_the_deleted_sets() {
    let old = entries_of(&KeywordSets::builtin());
    let mut now = old.clone();
    entry::remove_set(&mut now, 2);
    let moved = renumber(&now, old.len());
    // `nIndex1 == index`: the set is gone; `nIndex1 < index`: the numbers after it move up by one.
    assert_eq!(moved(2), -1);
    assert_eq!(moved(0), 0);
    assert_eq!(moved(1), 1);
    assert_eq!(moved(3), 2);
    assert_eq!(moved(16), 15);
    assert_eq!(moved(-1), -1);
    // A number beyond the sets moves up by the number of the sets deleted.
    assert_eq!(moved(50), 49);
    // Another deletion on top: the sets that were added have no old number.
    entry::remove_set(&mut now, 0);
    now.push(set("new", false, &[]));
    let moved = renumber(&now, old.len());
    assert_eq!((moved(0), moved(1), moved(2), moved(3)), (-1, 0, -1, 1));
    assert_eq!(moved(60), 58);
}

// ---- the file ----

#[test]
fn the_sets_are_written_as_sakura_writes_them_and_read_back() {
    let mut p = profile("[Other]\r\nx=1\r\n[KeyWords]\r\nnCurrentKeyWordSetIdx=2\r\n");
    let drafts = drafts_of(&[
        set("C/C++", true, &["if", "while"]),
        set("日本語", false, &["あ", "い"]),
        set("empty", false, &[]),
    ]);
    assert!(matsu_config::settings::keywords::store_sets(&mut p, &drafts).unwrap());
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), Some("3"));
    assert_eq!(p.get("KeyWords", "szSN[01]"), Some("日本語"));
    assert_eq!(p.get("KeyWords", "nCASE[00]"), Some("1"));
    assert_eq!(p.get("KeyWords", "nCASE[01]"), Some("0"));
    assert_eq!(p.get("KeyWords", "nKWN[00]"), Some("2"));
    // Each keyword is followed by a tab; the size counts the end of the string too.
    assert_eq!(p.get("KeyWords", "szKW[00]"), Some("if\twhile\t"));
    assert_eq!(p.get("KeyWords", "szKW[00].Size"), Some("10"));
    assert_eq!(p.get("KeyWords", "szKW[01]"), Some("あ\tい\t"));
    assert_eq!(p.get("KeyWords", "szKW[01].Size"), Some("5"));
    assert_eq!(p.get("KeyWords", "szKW[02]"), Some(""));
    assert_eq!(p.get("KeyWords", "szKW[02].Size"), Some("1"));
    // The rest of the file stays.
    assert_eq!(p.get("KeyWords", "nCurrentKeyWordSetIdx"), Some("2"));
    assert_eq!(p.get("Other", "x"), Some("1"));
    // Reading gives the sets back, and MATSU Editor's own sets are not added a second time.
    let read = KeywordSets::from_profile(&p);
    assert_eq!(read.len(), 3);
    assert_eq!(read.get(1).unwrap().name, "日本語");
    assert!(read.get(0).unwrap().contains("while"));
    assert_eq!(read.extra_index(ExtraSet::Rust), -1);
    // The same sets again change nothing; fewer sets remove the keys of the ones that are gone.
    assert!(!matsu_config::settings::keywords::store_sets(&mut p, &drafts).unwrap());
    assert!(matsu_config::settings::keywords::store_sets(&mut p, &drafts[..1]).unwrap());
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), Some("1"));
    for key in [
        "szSN[01]",
        "nCASE[02]",
        "nKWN[01]",
        "szKW[02]",
        "szKW[01].Size",
    ] {
        assert_eq!(p.get("KeyWords", key), None, "{key}");
    }
}

#[test]
fn the_hidden_list_starts_with_the_sets_of_the_file() {
    let rows = store::SETS.load(&Profile::new());
    // Sakura's 17 sets, then MATSU Editor's own (ADR-0017).
    assert_eq!(rows.len(), 17 + ExtraSet::ALL.len());
    assert_eq!(&rows[0][..3], ["0", "C/C++", "1"]);
    assert!(rows[0][3].split('\t').any(|w| w == "while"));
    assert_eq!(&rows[26][..2], ["26", "YAML"]);
    let file = profile(
        "[KeyWords]\r\nnKeyWordSetNum=2\r\nszSN[00]=Mine\r\nnCASE[00]=1\r\nnKWN[00]=2\r\nszKW[00]=foo\tBar\t\r\n\
         szSN[01]=Other\r\nnCASE[01]=0\r\nnKWN[01]=0\r\n",
    );
    let rows = store::SETS.load(&file);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ["0", "Mine", "1", "Bar\tfoo"]);
    assert_eq!(rows[1], ["1", "Other", "0", ""]);
}

#[test]
fn nothing_is_written_while_the_sets_are_as_the_file_has_them() {
    let mut p = profile("[Other]\r\nx=1\r\n");
    let rows = store::SETS.load(&p);
    assert!(!store::SETS.store(&rows, &mut p).unwrap());
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), None);
}

#[test]
fn a_changed_set_writes_every_set_with_the_sets_of_matsu_editor_and_keeps_the_types() {
    let mut p = profile("[Other]\r\nx=1\r\n");
    let mut entries = from_rows(&store::SETS.load(&p));
    entries[1].words.push("<mine>".to_owned());
    assert!(store::SETS.store(&rows_of(&entries), &mut p).unwrap());
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), Some("33"));
    assert_eq!(p.get("KeyWords", "szSN[26]"), Some("YAML"));
    assert_eq!(p.get("KeyWords", "szSN[27]"), Some("SQL"));
    // No set was deleted: no type needs its numbers changed, so no type section was made.
    assert_eq!(p.get("Types(0)", "nInts"), None);
    // Reading gives the same 33 sets (MATSU Editor's own are found by name, not added again), with the change.
    let types = DocumentTypes::from_profile(&p);
    assert_eq!(types.keywords().len(), 33);
    assert!(types.keywords().get(1).unwrap().contains("<mine>"));
    assert_eq!(types.keywords().extra_index(ExtraSet::Rust), 22);
    let rust = types.iter().find(|t| t.type_name == "Rust").unwrap();
    assert_eq!(rust.keyword_set_idx[0], 22);
    // The same rows again change nothing.
    assert!(!store::SETS.store(&rows_of(&entries), &mut p).unwrap());
}

#[test]
fn deleting_a_set_moves_the_numbers_of_the_types_and_sets_of_matsu_editor_stay_found() {
    let mut p = profile("[Other]\r\nx=1\r\n");
    let mut entries = from_rows(&store::SETS.load(&p));
    // Delete "Java" (4) and MATSU Editor's "Go".
    let go = entries.iter().position(|e| e.name == "Go").unwrap();
    entry::remove_set(&mut entries, go);
    entry::remove_set(&mut entries, 4);
    assert!(store::SETS.store(&rows_of(&entries), &mut p).unwrap());
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), Some("31"));
    let types = DocumentTypes::from_profile(&p);
    let names: Vec<&str> = types.keywords().iter().map(|s| s.name.as_str()).collect();
    assert!(!names.contains(&"Java") && !names.contains(&"Go"));
    // The type that used "Java" has no set now; the types after it use the sets that moved up.
    let by_name = |n: &str| types.iter().find(|t| t.type_name == n).unwrap();
    let java = by_name("Java");
    assert_eq!(java.keyword_set_idx[0], -1);
    let cpp = by_name("C/C++");
    assert_eq!(cpp.keyword_set_idx[0], 0);
    let perl = by_name("Perl");
    assert_eq!(
        types.keywords().get(perl.keyword_set_idx[0]).unwrap().name,
        "Perl"
    );
    // The type of the deleted set of MATSU Editor's has none; the others still find theirs by name.
    assert_eq!(types.keywords().extra_index(ExtraSet::Go), -1);
    let go_type = by_name("Go");
    assert_eq!(go_type.keyword_set_idx[0], -1);
    let rust = by_name("Rust");
    assert_eq!(
        types.keywords().get(rust.keyword_set_idx[0]).unwrap().name,
        "Rust"
    );
    // Python's second set is "Python built-ins", wherever it is.
    let python = by_name("Python");
    assert_eq!(
        types
            .keywords()
            .get(python.keyword_set_idx[1])
            .unwrap()
            .name,
        "Python built-ins"
    );
    // The types the file did not have are not written: only the ones whose numbers changed are.
    let java_number = types.iter().position(|t| t.type_name == "Java").unwrap();
    assert_eq!(java_number, 6);
    assert!(p.get("Types(6)", "nInts").is_some());
    assert_eq!(p.get("Types(17)", "nInts"), None);
}

#[test]
fn renaming_a_set_of_matsu_editor_keeps_the_types_that_use_it() {
    let mut p = profile("[Other]\r\nx=1\r\n");
    let mut entries = from_rows(&store::SETS.load(&p));
    let rust = entries.iter().position(|e| e.name == "Rust").unwrap();
    let builtins = entries
        .iter()
        .position(|e| e.name == "Python built-ins")
        .unwrap();
    entry::rename_set(&mut entries, rust, "My Rust");
    entry::rename_set(&mut entries, builtins, "My builtins");
    assert!(store::SETS.store(&rows_of(&entries), &mut p).unwrap());
    let types = DocumentTypes::from_profile(&p);
    // The types are found by name no longer, so they were written with the number of the set.
    let by_name = |n: &str| types.iter().find(|t| t.type_name == n).unwrap();
    let rust_type = by_name("Rust");
    assert_eq!(
        types
            .keywords()
            .get(rust_type.keyword_set_idx[0])
            .unwrap()
            .name,
        "My Rust"
    );
    let python = by_name("Python");
    assert_eq!(
        types
            .keywords()
            .get(python.keyword_set_idx[1])
            .unwrap()
            .name,
        "My builtins"
    );
    // The types that did not need it are not written.
    assert_eq!(p.get("Types(4)", "nInts"), None);
    assert_eq!(p.get("Types(2)", "nInts"), None);
}

// ---- the dialog ----

fn open(cx: &mut TestAppContext, text: &str) -> (Ini, AnyWindowHandle) {
    let (ini, handle) = setup(cx, text);
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Keyword, dcx));
    (ini, handle)
}

fn entries_in(cx: &mut TestAppContext) -> Vec<Entry> {
    with_dialog(cx, |d, _, _| from_rows(d.values().rows(SETS)))
}

fn chosen_set(cx: &mut TestAppContext) -> Option<usize> {
    with_dialog(cx, |d, _, _| super::current(d.values()))
}

fn chosen_word(cx: &mut TestAppContext) -> Option<usize> {
    with_dialog(cx, |d, _, _| super::chosen_word(d.values()))
}

fn type_name(cx: &mut TestAppContext, text: &str) {
    with_dialog(cx, |d, w, dcx| d.set_input(NAME, text, w, dcx));
}

fn type_word(cx: &mut TestAppContext, text: &str) {
    with_dialog(cx, |d, w, dcx| d.set_input(WORD, text, w, dcx));
}

fn key(cx: &mut TestAppContext, letter: char) {
    assert!(with_dialog(cx, |d, w, dcx| d.use_access_key(letter, w, dcx)));
}

/// The key of a button that is off (greyed out) does nothing.
fn key_off(cx: &mut TestAppContext, letter: char) {
    assert!(!with_dialog(cx, |d, w, dcx| d.use_access_key(letter, w, dcx)));
}

fn box_text(cx: &mut TestAppContext, id: &str) -> String {
    with_dialog(cx, |d, _, dcx| d.input_text(id, dcx)).unwrap()
}

#[gpui::test]
fn the_page_shows_the_sets_and_chooses_the_first(cx: &mut TestAppContext) {
    let (_ini, _handle) = open(cx, "");
    assert!(with_dialog(cx, |d, _, _| d.page_controls()).len() > 10);
    let sets = entries_in(cx);
    assert_eq!(sets.len(), 17 + ExtraSet::ALL.len());
    assert_eq!(sets[0].name, "C/C++");
    assert_eq!(chosen_set(cx), Some(0));
    assert_eq!(chosen_word(cx), None);
    cancel(cx);
}

#[gpui::test]
fn the_page_starts_at_the_set_the_file_chose_last(cx: &mut TestAppContext) {
    let (ini, _handle) = open(cx, "[KeyWords]\r\nnCurrentKeyWordSetIdx=4\r\n");
    assert_eq!(chosen_set(cx), Some(4));
    cancel(cx);
    // The next window starts the same way; the test settings are one at a time.
    drop(ini);
    let (_ini, _handle) = open(cx, "[KeyWords]\r\nnCurrentKeyWordSetIdx=99\r\n");
    assert_eq!(
        chosen_set(cx),
        Some(32),
        "a number past the sets is the last set"
    );
    cancel(cx);
}

#[gpui::test]
fn a_file_without_sets_has_a_page_that_can_add_one(cx: &mut TestAppContext) {
    let (ini, _handle) = open(
        cx,
        "[Types(0)]\r\nnInts=0,10240,0,2,-1,-1,0,1,1,0,0,0\r\n[KeyWords]\r\nnKeyWordSetNum=0\r\n",
    );
    assert!(entries_in(cx).is_empty());
    assert_eq!(chosen_set(cx), None);
    // Nothing can be done to a set that is not there.
    key_off(cx, 'r');
    key_off(cx, 'a');
    assert!(entries_in(cx).is_empty());
    type_name(cx, "First");
    key(cx, 'm');
    assert_eq!(entries_in(cx)[0].name, "First");
    assert_eq!(chosen_set(cx), Some(0));
    ok(cx);
    let p = ini.profile();
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), Some("1"));
    assert_eq!(p.get("KeyWords", "szSN[00]"), Some("First"));
}

#[gpui::test]
fn a_set_is_added_renamed_and_chosen_with_the_boxes_and_the_buttons(cx: &mut TestAppContext) {
    let (_ini, _handle) = open(cx, "");
    let before = entries_in(cx).len();
    // An empty name adds nothing.
    key(cx, 'm');
    assert_eq!(entries_in(cx).len(), before);
    type_name(cx, "My set");
    key(cx, 'm');
    let sets = entries_in(cx);
    assert_eq!(sets.len(), before + 1);
    assert_eq!(sets[before].name, "My set");
    assert!(!sets[before].case_sensitive);
    // The new set is chosen, and the name box is empty for the next one.
    assert_eq!(chosen_set(cx), Some(before));
    assert_eq!(box_text(cx, NAME), "");
    // 変更 renames the chosen set; an empty box does nothing.
    key(cx, 'h');
    assert_eq!(entries_in(cx)[before].name, "My set");
    type_name(cx, "Renamed");
    key(cx, 'h');
    assert_eq!(entries_in(cx)[before].name, "Renamed");
    // Choosing another set empties the keyword box and the chosen keyword.
    with_dialog(cx, |d, w, dcx| d.keyword_select_set(Some(1), w, dcx));
    assert_eq!(chosen_set(cx), Some(1));
    cancel(cx);
}

#[gpui::test]
fn the_hundred_and_first_set_says_so(cx: &mut TestAppContext) {
    let (_ini, _handle) = open(cx, "");
    while entries_in(cx).len() < MAX_KEYWORD_SETS {
        type_name(cx, "n");
        key(cx, 'm');
    }
    assert!(!cx.has_pending_prompt());
    type_name(cx, "one more");
    key(cx, 'm');
    assert_eq!(entries_in(cx).len(), MAX_KEYWORD_SETS);
    let (message, _) = cx.pending_prompt().expect("the sets are full");
    assert_eq!(message, "セットは100個までしか登録できません。\n");
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    cancel(cx);
}

#[gpui::test]
fn deleting_a_set_asks_and_names_the_types_that_use_it(cx: &mut TestAppContext) {
    let (_ini, _handle) = open(cx, "");
    with_dialog(cx, |d, w, dcx| d.keyword_select_set(Some(4), w, dcx));
    key(cx, 'r');
    let (question, _) = cx.pending_prompt().expect("the question");
    assert!(
        question.starts_with("「Java」のセットを削除します。\nよろしいですか？"),
        "{question}"
    );
    // The Java type uses the set (`・name(extensions)`).
    assert!(question.contains("・Java(java,jav)\n"), "{question}");
    // キャンセル keeps the set.
    cx.simulate_prompt_answer("キャンセル");
    cx.run_until_parked();
    assert_eq!(entries_in(cx)[4].name, "Java");
    // OK deletes it; the set that took its place is chosen.
    key(cx, 'r');
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    let sets = entries_in(cx);
    assert_eq!(sets[4].name, "CORBA IDL");
    assert_eq!(chosen_set(cx), Some(4));
    cancel(cx);
}

#[gpui::test]
fn deleting_the_last_set_chooses_the_one_before_and_the_only_set_leaves_none(
    cx: &mut TestAppContext,
) {
    let (_ini, _handle) = open(
        cx,
        "[KeyWords]\r\nnKeyWordSetNum=2\r\nszSN[00]=A\r\nszSN[01]=B\r\n",
    );
    with_dialog(cx, |d, w, dcx| d.keyword_select_set(Some(1), w, dcx));
    with_dialog(cx, |d, w, dcx| d.keyword_remove_set(1, w, dcx));
    assert_eq!(chosen_set(cx), Some(0));
    with_dialog(cx, |d, w, dcx| d.keyword_remove_set(0, w, dcx));
    assert_eq!(chosen_set(cx), None);
    assert!(entries_in(cx).is_empty());
    cancel(cx);
}

#[gpui::test]
fn the_keywords_are_added_edited_and_deleted_with_the_box_and_the_list(cx: &mut TestAppContext) {
    let (_ini, _handle) = open(
        cx,
        "[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=A\r\nnCASE[00]=1\r\nnKWN[00]=2\r\nszKW[00]=a\tb\t\r\n",
    );
    let set = |cx: &mut TestAppContext| entries_in(cx).remove(0);
    // 追加: to the end; the box is emptied; nothing is chosen.
    type_word(cx, "c");
    key(cx, 'a');
    assert_eq!(words(&set(cx)), ["a", "b", "c"]);
    assert_eq!(box_text(cx, WORD), "");
    // A duplicate and an empty box add nothing.
    type_word(cx, "c");
    key(cx, 'a');
    key(cx, 'a');
    assert_eq!(set(cx).words.len(), 3);
    // Without a chosen keyword 編集 and 削除 do nothing (the buttons are off).
    type_word(cx, "zzz");
    key_off(cx, 'e');
    key_off(cx, 'd');
    assert_eq!(set(cx).words.len(), 3);
    // A click on a keyword chooses it and puts it in the box.
    with_dialog(cx, |d, w, dcx| d.keyword_select_word(Some(1), w, dcx));
    assert_eq!(chosen_word(cx), Some(1));
    assert_eq!(box_text(cx, WORD), "b");
    // 編集: the keyword takes the text of the box.
    type_word(cx, "B2");
    key(cx, 'e');
    assert_eq!(words(&set(cx)), ["a", "B2", "c"]);
    assert_eq!(chosen_word(cx), Some(1));
    // An empty box deletes it; the keyword that took its place is chosen.
    type_word(cx, "");
    key(cx, 'e');
    assert_eq!(words(&set(cx)), ["a", "c"]);
    assert_eq!(chosen_word(cx), Some(1));
    assert_eq!(box_text(cx, WORD), "c");
    // 削除: the last one went, so the one before it is chosen.
    key(cx, 'd');
    assert_eq!(words(&set(cx)), ["a"]);
    assert_eq!(chosen_word(cx), Some(0));
    key(cx, 'd');
    assert!(set(cx).words.is_empty());
    assert_eq!(chosen_word(cx), None);
    cancel(cx);
}

#[gpui::test]
fn the_case_box_flips_the_rule_of_the_chosen_set_only(cx: &mut TestAppContext) {
    let (_ini, _handle) = open(cx, "");
    // The first set is C/C++ (the case counts); the second is HTML (it does not).
    assert_eq!(
        entries_in(cx)
            .iter()
            .take(2)
            .map(|e| e.case_sensitive)
            .collect::<Vec<_>>(),
        [true, false]
    );
    key(cx, 'c');
    assert!(!entries_in(cx)[0].case_sensitive);
    assert!(!entries_in(cx)[1].case_sensitive);
    with_dialog(cx, |d, w, dcx| d.keyword_select_set(Some(1), w, dcx));
    with_dialog(cx, |d, _, dcx| d.set_bool(CASE, true, dcx));
    assert!(entries_in(cx)[1].case_sensitive);
    assert!(!entries_in(cx)[0].case_sensitive);
    cancel(cx);
}

#[gpui::test]
fn cleaning_asks_then_sorts_and_removes_the_duplicates(cx: &mut TestAppContext) {
    let (_ini, _handle) = open(
        cx,
        "[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=A\r\nnCASE[00]=0\r\nnKWN[00]=4\r\nszKW[00]=b\tB\ta\tb\t\r\n",
    );
    key(cx, 'o');
    let (question, _) = cx.pending_prompt().expect("the question");
    assert_eq!(
        question,
        "現在の設定では強調キーワードとして表示できないキーワードを削除しますか？"
    );
    cx.simulate_prompt_answer("いいえ(&N)");
    cx.run_until_parked();
    assert_eq!(entries_in(cx)[0].words.len(), 4);
    key(cx, 'o');
    cx.simulate_prompt_answer("はい(&Y)");
    cx.run_until_parked();
    assert_eq!(words(&entries_in(cx)[0]), ["a", "b"]);
    cancel(cx);
}

#[gpui::test]
fn ok_writes_the_sets_and_the_types_and_keeps_the_rest_of_the_file(cx: &mut TestAppContext) {
    let ini_text = "[Other]\r\nx=1\r\n\
[Types(0)]\r\nnInts=0,10240,0,2,0,-1,0,1,1,0,0,0\r\nszTypeName=C\r\n\
[Types(4)]\r\nnInts=4,10240,0,2,4,-1,0,1,1,0,0,0\r\nszTypeName=Java\r\n\
[Common]\r\nbDispSTATUSBAR=1\r\n";
    let (ini, _handle) = open(cx, ini_text);
    let before = ini.profile();
    // Delete C/C++ (the first set) and add a keyword to the second.
    with_dialog(cx, |d, w, dcx| d.keyword_remove_set(0, w, dcx));
    assert_eq!(chosen_set(cx), Some(0));
    type_word(cx, "<mine>");
    key(cx, 'a');
    ok(cx);
    let after = ini.profile();
    assert_eq!(after.get("KeyWords", "nKeyWordSetNum"), Some("32"));
    assert_eq!(after.get("KeyWords", "szSN[00]"), Some("HTML"));
    assert!(
        after
            .get("KeyWords", "szKW[00]")
            .unwrap()
            .contains("<mine>\t")
    );
    assert_eq!(after.get("KeyWords", "nCurrentKeyWordSetIdx"), Some("0"));
    assert_eq!(after.get("KeyWords", "bUmeExtraSetsInFile"), Some("16"));
    // The type that used C/C++ has no set; the Java type moved from 4 to 3.
    let types = DocumentTypes::from_profile(&after);
    assert_eq!(types.get(0).unwrap().keyword_set_idx[0], -1);
    assert_eq!(types.get(4).unwrap().keyword_set_idx[0], 3);
    assert_eq!(types.keywords().get(3).unwrap().name, "Java");
    // The rest of the file is as it was.
    let old: Vec<_> = before.entries("Other").collect();
    let new: Vec<_> = after.entries("Other").collect();
    assert_eq!(old, new);
    assert_eq!(after.get("Common", "bDispSTATUSBAR"), Some("1"));
    assert_eq!(after.get("Types(0)", "szTypeName"), Some("C"));
    // The next dialog starts from what was written.
    run(cx, _handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Keyword, dcx));
    assert_eq!(entries_in(cx).len(), 32);
    assert_eq!(entries_in(cx)[3].name, "Java");
    cancel(cx);
}

#[gpui::test]
fn ok_without_a_change_to_the_sets_leaves_them_out_of_the_file(cx: &mut TestAppContext) {
    let (ini, _handle) = open(cx, "[Other]\r\nx=1\r\n");
    ok(cx);
    let p = ini.profile();
    // Only the chosen set is remembered (`nCurrentKeyWordSetIdx`), as Sakura writes it.
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), None);
    assert_eq!(p.get("KeyWords", "szSN[00]"), None);
    assert_eq!(p.get("KeyWords", "nCurrentKeyWordSetIdx"), Some("0"));
}

#[gpui::test]
fn cancel_writes_nothing(cx: &mut TestAppContext) {
    let (ini, _handle) = open(cx, "[Other]\r\nx=1\r\n");
    let before = ini.bytes();
    with_dialog(cx, |d, w, dcx| d.keyword_remove_set(0, w, dcx));
    cancel(cx);
    assert_eq!(ini.bytes(), before);
}

#[gpui::test]
fn ok_colors_every_open_window_with_the_new_sets(cx: &mut TestAppContext) {
    use crate::file_ops::windows::workspace_of;
    let (ini, handle) = setup(cx, "");
    let editor = cx
        .update(|app| workspace_of(app, handle))
        .unwrap()
        .read_with(cx, |ws, _| ws.editor().clone());
    // A document of the C type.
    editor.update(cx, |view, cx| {
        let settings = crate::settings::UserSettings::none().view_settings(Some("a.c"));
        view.apply_settings(settings, cx);
    });
    let has_keyword = |cx: &mut TestAppContext| {
        editor.read_with(cx, |view, _| {
            let rules = &view.view_settings().coloring;
            line_runs(rules, "int a;", None, &ColorState::Normal, 6)
                .iter()
                .any(|run| run.kind == ColorKind::Keyword(0))
        })
    };
    assert!(has_keyword(cx), "int is a keyword of the C type");
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Keyword, dcx));
    // Taking "int" out of C/C++ takes the color off it at once, in the window that is open.
    type_word(cx, "int");
    with_dialog(cx, |d, w, dcx| {
        let words = &d.keyword_entries()[0].words;
        let at = words.iter().position(|w| w == "int").unwrap();
        d.keyword_select_word(Some(at), w, dcx);
    });
    key(cx, 'd');
    ok(cx);
    editor.read_with(cx, |view, _| {
        assert_eq!(view.type_index(), 0, "the window keeps its type");
    });
    assert!(!has_keyword(cx), "the sets were taken at once");
    assert!(ini.profile().get("KeyWords", "szKW[00]").is_some());
    // The file has the sets, so the next start colors the same.
    let sets = KeywordSets::from_profile(&ini.profile());
    assert!(!sets.get(0).unwrap().contains("int"));
}

#[gpui::test]
fn the_buttons_have_the_keys_of_sakuras_page(cx: &mut TestAppContext) {
    let (_ini, _handle) = open(cx, "");
    let labels: Vec<&str> = [
        "セット追加(&M)",
        "変更(&H)",
        "セット削除(&R)...",
        "追加(&A)",
        "編集(&E)",
        "削除(&D)",
        "整理(&O)",
        "インポート(&I)...",
        "エクスポート(&X)...",
        "英大文字小文字区別(&C)",
    ]
    .to_vec();
    let page: Vec<&str> = with_dialog(cx, |d, _, _| {
        d.page_controls().iter().map(|c| c.label).collect()
    });
    for label in labels {
        assert!(page.contains(&label), "{label}");
    }
    // The controls of the page keep their state under their own names (no key but the chosen set).
    let keys: Vec<&str> = with_dialog(cx, |d, _, _| {
        d.page_controls()
            .iter()
            .map(|c| c.key)
            .filter(|k| !k.is_empty())
            .collect()
    });
    assert_eq!(keys, ["nCurrentKeyWordSetIdx"]);
    for id in [SETS, CURRENT, WORD_SEL, NAME, WORD, CASE] {
        assert!(id.starts_with("keyword_"));
    }
    cancel(cx);
}

// ---- import and export ----

#[gpui::test]
fn an_import_adds_the_keywords_of_the_file_to_the_chosen_set(cx: &mut TestAppContext) {
    let (ini, _handle) = open(
        cx,
        "[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=A\r\nnCASE[00]=0\r\nnKWN[00]=1\r\nszKW[00]=old\t\r\n",
    );
    let path = ini.dir.0.join("in.kwd");
    std::fs::write(
        &path,
        "\u{feff}// A キーワード定義ファイル\r\n// CASE=True\r\n\r\nold\r\nnew1\r\n新2\r\n",
    )
    .unwrap();
    key(cx, 'i');
    assert!(cx.did_prompt_for_paths());
    let chosen = path.clone();
    cx.simulate_path_prompt_response(move |_| Some(vec![chosen]));
    cx.run_until_parked();
    let set = entries_in(cx).remove(0);
    assert_eq!(words(&set), ["old", "new1", "新2"]);
    assert!(set.case_sensitive);
    // A file that cannot be opened says so and changes nothing.
    let missing = ini.dir.0.join("none.kwd");
    assert!(!with_dialog(cx, |d, w, dcx| d.keyword_import_from(&missing, w, dcx)));
    let (message, _) = cx.pending_prompt().expect("the message");
    assert!(
        message.starts_with("ファイルを開けませんでした。\n\n"),
        "{message}"
    );
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    assert_eq!(entries_in(cx)[0].words.len(), 3);
    cancel(cx);
}

#[gpui::test]
fn an_export_writes_the_chosen_set_with_its_case_rule_and_sorted_keywords(cx: &mut TestAppContext) {
    let (ini, _handle) = open(
        cx,
        "[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=A/B\r\nnCASE[00]=1\r\nnKWN[00]=3\r\nszKW[00]=b\ta\tC\t\r\n",
    );
    let path = ini.dir.0.join("out.kwd");
    key(cx, 'x');
    assert!(cx.did_prompt_for_new_path());
    let target = path.clone();
    cx.simulate_new_path_selection(move |_| Some(target));
    cx.run_until_parked();
    let bytes = std::fs::read(&path).unwrap();
    let file = matsu_config::settings::keyword_file::parse(&bytes);
    assert_eq!(file.case_sensitive, Some(true));
    assert_eq!(file.words, ["C", "a", "b"]);
    let (message, _) = cx.pending_prompt().expect("the message");
    assert!(
        message.starts_with("ファイルをエクスポートしました。\n\n"),
        "{message}"
    );
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
    cancel(cx);
}

#[gpui::test]
fn ok_writes_nothing_when_the_file_got_other_sets_while_the_dialog_was_open(
    cx: &mut TestAppContext,
) {
    let three = "[KeyWords]\r\nnKeyWordSetNum=3\r\nszSN[00]=A\r\nszSN[01]=B\r\nszSN[02]=C\r\n\
[Types(0)]\r\nnInts=0,10240,0,2,2,-1,0,1,1,0,0,0\r\n";
    let (ini, _handle) = open(cx, three);
    // The file loses its first set, and the type that used "C" has 1 now (another program, or a hand edit).
    let two = "[KeyWords]\r\nnKeyWordSetNum=2\r\nszSN[00]=B\r\nszSN[01]=C\r\n\
[Types(0)]\r\nnInts=0,10240,0,2,1,-1,0,1,1,0,0,0\r\n";
    std::fs::write(&ini.path, crate::common_settings::tests::with_bom(two)).unwrap();
    type_word(cx, "w");
    key(cx, 'a');
    assert_eq!(entries_in(cx)[0].words, ["w"]);
    ok(cx);
    // Neither the sets nor the number of the type were written back; the user is told.
    let p = ini.profile();
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), Some("2"));
    assert_eq!(p.get("KeyWords", "szSN[00]"), Some("B"));
    assert_eq!(
        p.get("Types(0)", "nInts"),
        Some("0,10240,0,2,1,-1,0,1,1,0,0,0")
    );
    let (message, _) = cx.pending_prompt().expect("the message");
    assert!(message.contains("別の場所"), "{message}");
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
}

#[gpui::test]
fn ok_writes_nothing_when_only_a_keyword_of_the_file_changed_meanwhile(cx: &mut TestAppContext) {
    let before = "[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=A\r\nnCASE[00]=1\r\nnKWN[00]=1\r\nszKW[00]=x\t\r\n";
    let (ini, _handle) = open(cx, before);
    let after = "[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=A\r\nnCASE[00]=0\r\nnKWN[00]=2\r\nszKW[00]=x\ty\t\r\n";
    std::fs::write(&ini.path, crate::common_settings::tests::with_bom(after)).unwrap();
    type_word(cx, "w");
    key(cx, 'a');
    ok(cx);
    let p = ini.profile();
    assert_eq!(p.get("KeyWords", "szKW[00]"), Some("x\ty\t"));
    assert_eq!(p.get("KeyWords", "nCASE[00]"), Some("0"));
    let (message, _) = cx.pending_prompt().expect("the message");
    assert!(message.contains("別の場所"), "{message}");
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
}

#[gpui::test]
fn deleting_every_set_does_not_hide_a_change_of_the_file(cx: &mut TestAppContext) {
    let one = "[KeyWords]\r\nnKeyWordSetNum=1\r\nszSN[00]=A\r\n";
    let (ini, _handle) = open(cx, one);
    let two = "[KeyWords]\r\nnKeyWordSetNum=2\r\nszSN[00]=A\r\nszSN[01]=B\r\n";
    std::fs::write(&ini.path, crate::common_settings::tests::with_bom(two)).unwrap();
    with_dialog(cx, |d, w, dcx| d.keyword_remove_set(0, w, dcx));
    assert!(entries_in(cx).is_empty());
    ok(cx);
    assert_eq!(ini.profile().get("KeyWords", "nKeyWordSetNum"), Some("2"));
    assert!(cx.pending_prompt().is_some());
    cx.simulate_prompt_answer("OK");
    cx.run_until_parked();
}

#[test]
fn what_the_write_did_to_the_numbers_is_given_to_a_sheet_of_the_type_settings_once() {
    crate::common_settings::forget_keyword_renumbering();
    let mut p = profile("[Other]\r\nx=1\r\n");
    let mut entries = from_rows(&store::SETS.load(&p));
    // Nothing changed: nothing to give.
    assert!(!store::SETS.store(&rows_of(&entries), &mut p).unwrap());
    assert!(crate::common_settings::take_keyword_renumbering().is_none());
    // Set 0 is deleted and set 1 renamed with other words: its place is still known.
    entry::remove_set(&mut entries, 0);
    entries[0].name = "renamed".to_owned();
    entries[0].words = vec!["other".to_owned()];
    assert!(store::SETS.store(&rows_of(&entries), &mut p).unwrap());
    let moved = crate::common_settings::take_keyword_renumbering().expect("given");
    assert_eq!((moved(0), moved(1), moved(2), moved(-1)), (-1, 0, 1, -1));
    // Only the first to ask gets it.
    assert!(crate::common_settings::take_keyword_renumbering().is_none());
}
