//! Tests of the tags file search. The expectations follow `CDlgTagJumpList::find_key_core` and the
//! functions it calls (`sakura_core/dlg/CDlgTagJumpList.cpp`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::*;

/// A tags file in the format that `ctags --excmd=n` writes.
fn tag(keyword: &str, file: &str, line: usize, kind: char) -> String {
    format!("{keyword}\t{file}\t{line};\"\t{kind}\tclass:X\n")
}

fn write(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

fn sorted_header() -> &'static str {
    "!_TAG_FILE_FORMAT\t2\t/extended format/\n!_TAG_FILE_SORTED\t1\t/0=unsorted, 1=sorted, 2=foldcase/\n"
}

fn keywords(page: &TagPage) -> Vec<&str> {
    page.items.iter().map(|i| i.keyword.as_str()).collect()
}

fn direct(document: &Path, keyword: &str) -> TagPage {
    find_tags(document, &TagQuery::direct(keyword, 1))
}

fn keyword_query<'a>(keyword: &'a str, partial: bool, ignore_case: bool) -> TagQuery<'a> {
    TagQuery {
        keyword,
        partial,
        exact: false,
        ignore_case,
        ignore_case_by_tags: false,
        next_mode: 3,
        top: 0,
    }
}

#[test]
fn an_exact_search_finds_the_tag_with_its_columns() {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = sorted_header().to_owned();
    tags += &tag("alpha", "a.c", 3, 'f');
    tags += &tag("beta", "src/b.c", 12, 'c');
    tags += &tag("gamma", "g.c", 7, 'v');
    write(dir.path(), "tags", tags);
    let doc = dir.path().join("main.c");
    let page = direct(&doc, "beta");
    assert_eq!(page.matched, 1);
    assert!(!page.has_next);
    let item = &page.items[0];
    assert_eq!(item.keyword, "beta");
    assert_eq!(item.file, "src/b.c");
    assert_eq!(item.line, 12);
    assert_eq!(item.kind, Some('c'));
    assert_eq!(item.note, "class:X");
    assert_eq!(item.depth, 0);
    assert_eq!(item.path, dir.path().join("src/b.c"));
}

#[test]
fn the_plain_format_and_lines_that_are_not_tags() {
    let dir = tempfile::tempdir().unwrap();
    // format 1 has no `;"`; a line without a number, with zero, or with a regex address is no tag
    let tags = "!_TAG_FILE_FORMAT\t1\n!_TAG_FILE_SORTED\t0\n\
        one\ta.c\t5\n\
        two\ta.c\t/^two$/\n\
        three\ta.c\t0\n\
        \tb.c\t3\n\
        four\ta.c\n\
        five\ta.c\t9;\"\tf\n";
    write(dir.path(), "tags", tags);
    let doc = dir.path().join("x.c");
    for (keyword, line) in [("one", Some(5)), ("five", Some(9))] {
        let page = direct(&doc, keyword);
        assert_eq!(page.items.first().map(|i| i.line), line, "{keyword}");
    }
    for keyword in ["two", "three", "four"] {
        assert_eq!(direct(&doc, keyword).matched, 0, "{keyword}");
    }
    // in format 1 the extension fields are not read
    let one = &direct(&doc, "five").items[0];
    assert_eq!(one.kind, None);
}

#[test]
fn no_tags_file_and_an_empty_keyword_find_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let doc = dir.path().join("main.c");
    assert_eq!(direct(&doc, "x"), TagPage::default());
    write(dir.path(), "tags", tag("x", "a.c", 1, 'f'));
    assert_eq!(direct(&doc, ""), TagPage::default());
}

#[test]
fn the_tags_file_of_a_folder_above_is_found_with_its_depth() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "tags", tag("top", "lib.c", 1, 'f'));
    write(&dir.path().join("a"), "tags", tag("mid", "m.c", 2, 'f'));
    let doc = dir.path().join("a/b/c/deep.c");
    let page = direct(&doc, "top");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].depth, 3);
    // the file name is relative to the folder of the tags file
    assert_eq!(page.items[0].path, dir.path().join("lib.c"));
    assert_eq!(direct(&doc, "mid").items[0].depth, 2);
}

#[test]
fn the_next_mode_decides_whether_a_folder_above_is_searched_after_a_hit() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "tags", tag("f", "outer.c", 1, 'f'));
    write(&dir.path().join("sub"), "tags", tag("f", "inner.c", 2, 'f'));
    let doc = dir.path().join("sub/x.c");
    let run = |mode: u8| find_tags(&doc, &TagQuery::direct("f", mode));
    // 1: not after a hit
    assert_eq!(run(1).matched, 1);
    // 2: not after an exact hit, and the direct jump is exact
    assert_eq!(run(2).matched, 1);
    // 3: always
    let both = run(3);
    assert_eq!(both.matched, 2);
    assert_eq!(
        both.items.iter().map(|i| i.depth).collect::<Vec<_>>(),
        [0, 1]
    );
    // 0: the first tags file is read, the next is not
    let doc_one = dir.path().join("sub/x.c");
    write(&dir.path().join("sub"), "tags", tag("g", "inner.c", 2, 'f'));
    assert_eq!(find_tags(&doc_one, &TagQuery::direct("f", 0)).matched, 0);
    assert_eq!(find_tags(&doc_one, &TagQuery::direct("f", 3)).matched, 1);
}

#[test]
fn a_tags_file_can_say_what_to_do_about_the_next_one() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "tags", tag("f", "outer.c", 1, 'f'));
    // "do search the next file" in the file itself beats the setting
    let mut inner = String::from("!_TAG_S_SEARCH_NEXT\t3\n");
    inner += &tag("f", "inner.c", 2, 'f');
    write(&dir.path().join("sub"), "tags", inner);
    let doc = dir.path().join("sub/x.c");
    assert_eq!(find_tags(&doc, &TagQuery::direct("f", 1)).matched, 2);
}

#[test]
fn a_tags_file_can_send_the_search_to_another_folder() {
    let dir = tempfile::tempdir().unwrap();
    let other = dir.path().join("other");
    write(&other, "tags", tag("f", "o.c", 4, 'f'));
    let mut inner = format!("!_TAG_S_SEARCH_NEXT\t3\t{}\n", other.display());
    inner += &tag("f", "i.c", 1, 'f');
    write(&dir.path().join("proj"), "tags", inner);
    let page = find_tags(&dir.path().join("proj/x.c"), &TagQuery::direct("f", 1));
    assert_eq!(page.matched, 2);
    let far = page.items.iter().find(|i| i.file == "o.c").unwrap();
    assert!(far.via_next);
    // names in a tags file reached this way are relative to its folder
    assert_eq!(far.path, other.join("o.c"));
}

#[test]
fn the_base_folder_of_a_tags_file_is_where_its_names_start() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("elsewhere");
    let mut tags = format!("!_TAG_S_FILE_BASEDIR\t{}\n", base.display());
    tags += &tag("f", "sub/a.c", 1, 'f');
    tags += &tag("g", "/abs/b.c", 1, 'f');
    write(dir.path(), "tags", tags);
    let doc = dir.path().join("x.c");
    let page = find_tags(&doc, &TagQuery::direct("f", 3));
    assert_eq!(page.items[0].path, base.join("sub/a.c"));
    let page = find_tags(&doc, &TagQuery::direct("g", 3));
    // a name that starts at the root is not put under the base folder (on Windows it gets the drive)
    assert!(page.items[0].path.ends_with("abs/b.c") && !page.items[0].path.starts_with(&base));
}

#[test]
fn the_search_by_the_beginning_ignores_case_when_asked() {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = String::from("!_TAG_FILE_SORTED\t0\n");
    for name in ["Parse", "parser", "ParseError", "reparse", "other"] {
        tags += &tag(name, "p.c", 1, 'f');
    }
    write(dir.path(), "tags", tags);
    let doc = dir.path().join("x.c");
    let find = |query: &TagQuery<'_>| {
        let page = find_tags(&doc, query);
        let mut names: Vec<_> = keywords(&page).into_iter().map(str::to_owned).collect();
        names.sort();
        names
    };
    assert_eq!(
        find(&keyword_query("Parse", false, false)),
        ["Parse", "ParseError"]
    );
    assert_eq!(
        find(&keyword_query("parse", false, true)),
        ["Parse", "ParseError", "parser"]
    );
    // in the middle
    assert_eq!(
        find(&keyword_query("parse", true, false)),
        ["parser", "reparse"]
    );
    assert_eq!(
        find(&keyword_query("parse", true, true)),
        ["Parse", "ParseError", "parser", "reparse"]
    );
}

#[test]
fn a_foldcase_tags_file_makes_the_direct_jump_ignore_case() {
    let dir = tempfile::tempdir().unwrap();
    // sorted ignoring case, as `ctags --sort=foldcase` writes
    let mut tags = String::from("!_TAG_FILE_SORTED\t2\n");
    tags += &tag("alpha", "a.c", 1, 'f');
    tags += &tag("Beta", "b.c", 2, 'f');
    tags += &tag("gamma", "g.c", 3, 'f');
    write(dir.path(), "tags", tags);
    let doc = dir.path().join("x.c");
    // The rule of the first file keeps the setting from before its header: case-sensitive. The
    // hits of the next files would be case-insensitive.
    assert_eq!(direct(&doc, "Beta").matched, 1);
    assert_eq!(direct(&doc, "beta").matched, 0);
}

#[test]
fn a_page_holds_fifty_hits_and_the_next_page_goes_on() {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = String::from("!_TAG_FILE_SORTED\t0\n");
    for i in 0..120 {
        tags += &tag(&format!("item{i:03}"), "a.c", i + 1, 'f');
    }
    write(dir.path(), "tags", tags);
    let doc = dir.path().join("x.c");
    let mut query = keyword_query("item", false, false);
    let first = find_tags(&doc, &query);
    assert_eq!(first.items.len(), PAGE_SIZE);
    assert!(first.has_next);
    // the hit that did not fit is counted: Sakura's m_nMatchAll
    assert_eq!(first.matched, PAGE_SIZE + 1);
    assert_eq!(first.items[0].keyword, "item000");

    query.top = 50;
    let second = find_tags(&doc, &query);
    assert_eq!(second.items.len(), PAGE_SIZE);
    assert!(second.has_next);
    assert_eq!(second.items[0].keyword, "item050");

    query.top = 100;
    let last = find_tags(&doc, &query);
    assert_eq!(last.items.len(), 20);
    assert!(!last.has_next);
    assert_eq!(last.items[19].keyword, "item119");
    assert_eq!(last.matched, 120);
}

#[test]
fn a_page_is_sorted_by_keyword() {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = String::from("!_TAG_FILE_SORTED\t0\n");
    for name in ["zeta", "alpha", "Mid", "beta"] {
        tags += &tag(name, "a.c", 1, 'f');
    }
    write(dir.path(), "tags", tags);
    let page = find_tags(&dir.path().join("x.c"), &keyword_query("a", true, false));
    assert_eq!(keywords(&page), ["alpha", "beta", "zeta"]);
}

#[test]
fn a_big_sorted_file_is_searched_by_bisection() {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = String::from(sorted_header());
    // 60,000 tags, a few of them with the same name; about 3 MB
    for i in 0..60_000 {
        let name = format!("name{i:06}");
        tags += &tag(&name, "some/dir/file.c", i + 1, 'f');
        if i % 10_000 == 0 {
            tags += &tag(&name, "other/file.c", i + 2, 'v');
        }
    }
    let path = write(dir.path(), "tags", tags);
    assert!(std::fs::metadata(path).unwrap().len() > 2_000_000);
    let doc = dir.path().join("x.c");
    for i in [0, 1, 777, 10_000, 30_001, 59_999] {
        let name = format!("name{i:06}");
        let page = direct(&doc, &name);
        let expected = if i % 10_000 == 0 { 2 } else { 1 };
        assert_eq!(page.matched, expected, "{name}");
        assert!(page.items.iter().all(|t| t.line > i), "{name}");
    }
    assert_eq!(direct(&doc, "name999999").matched, 0);
    assert_eq!(direct(&doc, "a").matched, 0);
    assert_eq!(direct(&doc, "zzz").matched, 0);
    assert_eq!(direct(&doc, "name00000").matched, 0);
}

#[test]
fn bisection_and_a_scan_agree() {
    let dir = tempfile::tempdir().unwrap();
    // names of different lengths with many equal neighbours, so the step back has work to do
    let mut names: Vec<String> = Vec::new();
    for i in 0..400 {
        for copy in 0..(i % 7 + 1) {
            names.push(format!("k{:03}", i / 3 * 3 + copy % 2));
        }
    }
    names.sort();
    let mut sorted = String::from(sorted_header());
    let mut unsorted = String::from("!_TAG_FILE_SORTED\t0\n");
    for (n, name) in names.iter().enumerate() {
        let line = tag(name, "f.c", n + 1, 'f');
        sorted += &line;
        unsorted += &line;
    }
    let a = write(&dir.path().join("a"), "tags", sorted);
    let b = write(&dir.path().join("b"), "tags", unsorted);
    for i in 0..410 {
        let name = format!("k{i:03}");
        let by_bisection = direct(&a.parent().unwrap().join("x.c"), &name);
        let by_scan = direct(&b.parent().unwrap().join("x.c"), &name);
        assert_eq!(by_bisection.matched, by_scan.matched, "{name}");
        let lines = |p: &TagPage| {
            let mut v: Vec<_> = p.items.iter().map(|i| i.line).collect();
            v.sort_unstable();
            v
        };
        assert_eq!(lines(&by_bisection), lines(&by_scan), "{name}");
    }
}

#[test]
fn a_huge_file_is_not_read_whole() {
    // The hit is at the middle of 20 MB of tags; the cost of a bisection does not depend on the
    // size. The file is made once and read by ranges only.
    let dir = tempfile::tempdir().unwrap();
    let mut text = String::from(sorted_header());
    let line = tag("padding", "p.c", 1, 'f');
    let mut count = 0;
    while text.len() < 20_000_000 {
        write!(text, "{}", line.replace("padding", &format!("n{count:09}"))).unwrap();
        count += 1;
    }
    write(dir.path(), "tags", text);
    let start = std::time::Instant::now();
    let page = direct(&dir.path().join("x.c"), &format!("n{:09}", count / 2));
    assert_eq!(page.matched, 1);
    assert!(
        start.elapsed() < std::time::Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn a_long_line_is_cut_and_the_next_line_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let mut tags = String::from("!_TAG_FILE_SORTED\t0\n");
    // the number of this line is past the 1023rd byte, so it is not a tag
    tags += &format!("long\t{}\t5;\"\tf\n", "d/".repeat(600));
    tags += &tag("after", "a.c", 8, 'f');
    // a line that is exactly as long as can be read stays whole
    let name = "n".repeat(900);
    tags += &tag(&name, "a.c", 9, 'f');
    write(dir.path(), "tags", tags);
    let doc = dir.path().join("x.c");
    assert_eq!(direct(&doc, "long").matched, 0);
    assert_eq!(direct(&doc, "after").items[0].line, 8);
    assert_eq!(direct(&doc, &name).items.len(), 1);
}

#[test]
fn an_etags_file_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "tags",
        "\x0c\nmain.c,100\nint main()\x7fmain\x011,0\n",
    );
    let page = direct(&dir.path().join("x.c"), "main");
    assert_eq!(page.matched, 0);
}

#[test]
fn a_file_that_is_not_utf8_is_read_as_shift_jis() {
    let dir = tempfile::tempdir().unwrap();
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode("関数\t日本語.c\t4;\"\tf\n");
    let mut tags = b"!_TAG_FILE_SORTED\t0\n".to_vec();
    tags.extend_from_slice(&bytes);
    write(dir.path(), "tags", tags);
    let page = direct(&dir.path().join("x.c"), "関数");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].file, "日本語.c");
    // UTF-8 as ctags writes it
    write(dir.path(), "tags", "関数\t日本語.c\t4;\"\tf\n");
    assert_eq!(direct(&dir.path().join("x.c"), "関数").items.len(), 1);
}

#[test]
fn crlf_lines_and_windows_names_are_read() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "tags",
        "!_TAG_FILE_SORTED\t0\r\nf\tsub\\a.c\t3;\"\tf\r\n",
    );
    let page = direct(&dir.path().join("x.c"), "f");
    assert_eq!(page.items[0].line, 3);
    assert_eq!(page.items[0].file, "sub\\a.c");
    #[cfg(not(windows))]
    assert_eq!(page.items[0].path, dir.path().join("sub/a.c"));
}

#[test]
fn the_kind_is_named_by_the_extension_of_the_file() {
    assert_eq!(kind_name(Some('f'), "a.c"), "function");
    assert_eq!(kind_name(Some('c'), "a.CPP"), "class");
    assert_eq!(kind_name(Some('f'), "a.rb"), "method");
    assert_eq!(kind_name(Some('F'), "a.rb"), "singleton method");
    // without an extension: C
    assert_eq!(kind_name(Some('s'), "Makefile"), "struct");
    assert_eq!(kind_name(Some('z'), "a.c"), "");
    assert_eq!(kind_name(Some('f'), "a.unknown"), "");
    assert_eq!(kind_name(None, "a.c"), "");
}

fn item(path: &str) -> TagItem {
    TagItem {
        keyword: "k".into(),
        file: path.into(),
        line: 1,
        kind: None,
        note: String::new(),
        depth: 0,
        via_next: false,
        path: PathBuf::from(path),
    }
}

#[test]
fn the_first_choice_is_the_hit_in_the_file_being_edited() {
    let items = [
        item("/p/other.c"),
        item("/p/src/main.h"),
        item("/p/src/main.c"),
    ];
    // the same file
    assert_eq!(best_item(&items, Path::new("/p/src/main.c")), 2);
    // the same folder and name, another extension
    assert_eq!(best_item(&items, Path::new("/p/src/main.cpp")), 1);
    // the same name and extension elsewhere
    assert_eq!(best_item(&items, Path::new("/q/main.c")), 2);
    // nothing alike: the first
    assert_eq!(best_item(&items, Path::new("/z/none.txt")), 0);
    assert_eq!(best_item(&[], Path::new("/z/none.txt")), 0);
}

#[test]
fn a_sorted_shift_jis_file_is_read_through_not_bisected() {
    // In Shift_JIS the order of these is 亜 愛 悪, which is not the order of their code points
    let dir = tempfile::tempdir().unwrap();
    let mut tags = b"!_TAG_FILE_SORTED\t1\n".to_vec();
    for name in ["亜", "愛", "悪"] {
        let line = format!("{name}\tf.c\t1;\"\tf\n");
        tags.extend_from_slice(&encoding_rs::SHIFT_JIS.encode(&line).0);
    }
    write(dir.path(), "tags", tags);
    for name in ["亜", "愛", "悪"] {
        assert_eq!(direct(&dir.path().join("x.c"), name).matched, 1, "{name}");
    }
}
