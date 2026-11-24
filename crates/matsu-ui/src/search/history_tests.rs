//! Tests of the search and replace history: the order and the limit, the file, two windows, and
//! the drop-down of the panel. The lists themselves are tested in
//! `matsu_config::settings::search_keys`.
//!
//! Expectations cite `sakura_core/env/CSearchKeywordManager.cpp`, `sakura_core/dlg/CDlgFind.cpp`,
//! `CDlgReplace.cpp` and `sakura_core/view/CEditView_Search.cpp`.

use std::path::PathBuf;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::{Document, Pos};
use matsu_core::model::DocumentModel;

use super::PanelMode;
use super::history::{self, SearchHistory};
use crate::document::SharedDocument;
use crate::view::EditorView;

struct Fixture {
    view: Entity<EditorView>,
}

fn setup<'a>(cx: &'a mut TestAppContext, text: &str) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (Fixture { view }, vcx)
}

impl Fixture {
    fn pattern(&self, cx: &mut VisualTestContext) -> String {
        self.view
            .read_with(cx, |v, _| v.search.pattern().to_owned())
    }

    fn replacement(&self, cx: &mut VisualTestContext) -> String {
        self.view
            .read_with(cx, |v, _| v.search.replace.text.clone())
    }

    fn selection(&self, cx: &mut VisualTestContext) -> Option<(Pos, Pos)> {
        self.view.read_with(cx, |v, _| v.cursor.selection())
    }

    fn panel_mode(&self, cx: &mut VisualTestContext) -> Option<PanelMode> {
        self.view.read_with(cx, |v, cx| v.search_panel_mode(cx))
    }

    fn regex(&self, cx: &mut VisualTestContext) {
        self.view.update(cx, |v, _| {
            let mut o = v.search.options();
            o.regex = true;
            v.search.set_options(o);
        });
        // A dialog starts with the options the last search left, so the box was ticked there.
        cx.update(|_, app| {
            let o = crate::search::state::SearchOptions {
                regex: true,
                ..Default::default()
            };
            crate::saved_state::search::remember(app, o, None);
        });
    }
}

fn search_keys(cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, app| history::search_keys(app))
}

fn replace_keys(cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, app| history::replace_keys(app))
}

/// Another window searched (or replaced) for these strings.
fn other_window(cx: &mut VisualTestContext, search: &str, replace: Option<&str>) {
    cx.update(|_, app| {
        history::remember_search(app, search);
        if let Some(replace) = replace {
            history::remember_replace(app, replace);
        }
    });
}

fn temp_ini(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "matsu-ui-search-history-{}-{name}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("matsu.ini")
}

#[gpui::test]
fn the_newest_is_first_and_a_repeat_moves_up(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "");
    for s in ["a", "b", "c"] {
        other_window(cx, s, None);
    }
    assert_eq!(search_keys(cx), ["c", "b", "a"]);
    other_window(cx, "a", None);
    assert_eq!(search_keys(cx), ["a", "c", "b"]);
}

#[gpui::test]
fn the_history_keeps_thirty_strings(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "");
    for i in 0..35 {
        other_window(cx, &format!("s{i}"), Some(&format!("r{i}")));
    }
    let keys = search_keys(cx);
    assert_eq!(keys.len(), 30);
    assert_eq!((keys[0].as_str(), keys[29].as_str()), ("s34", "s5"));
    assert_eq!(replace_keys(cx).len(), 30);
}

#[gpui::test]
fn an_empty_search_string_is_not_kept(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "");
    other_window(cx, "", None);
    assert!(search_keys(cx).is_empty());
}

#[gpui::test]
fn the_history_is_written_to_the_file_and_read_back(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "");
    let path = temp_ini("file");
    // a file that has other sections already
    std::fs::write(&path, "\u{feff}[Other]\r\nkey=value\r\n").unwrap();
    cx.update(|_, app| {
        history::install(app, SearchHistory::load(Some(&path), Some(path.clone())));
    });
    other_window(cx, "b+", Some(""));
    other_window(cx, "検索 文字列", Some("置換\\1"));
    other_window(cx, "^a=b;c$", None);
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("[Other]\r\nkey=value\r\n"), "{written}");
    assert!(written.contains("SEARCHKEY[00]=^a=b;c$"), "{written}");
    // a new run reads the same strings, newest first
    let again = SearchHistory::load(Some(&path), None);
    assert_eq!(
        again.keys().search.items(),
        ["^a=b;c$", "検索 文字列", "b+"]
    );
    assert_eq!(again.keys().replace.items(), ["置換\\1", ""]);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn two_processes_with_the_same_file_do_not_lose_each_others_strings(cx: &mut TestAppContext) {
    let (_f, _cx) = setup(cx, "");
    let path = temp_ini("two");
    // both started with an empty file
    let mut a = SearchHistory::load(None, Some(path.clone()));
    let mut b = SearchHistory::load(None, Some(path.clone()));
    a.persist(|keys| {
        keys.add_search("from a");
    });
    b.persist(|keys| {
        keys.add_replace("from b");
    });
    let file = SearchHistory::load(Some(&path), None);
    assert_eq!(file.keys().search.items(), ["from a"]);
    assert_eq!(file.keys().replace.items(), ["from b"]);
    // and b now knows what a saved
    assert_eq!(b.keys().search.items(), ["from a"]);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn a_string_whose_save_failed_is_saved_with_the_next_one(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "");
    let path = temp_ini("failed");
    // the first save cannot work: the place of the file is a folder
    let folder = path.parent().unwrap().join("folder");
    std::fs::create_dir_all(&folder).unwrap();
    cx.update(|_, app| history::install(app, SearchHistory::load(None, Some(folder.clone()))));
    other_window(cx, "first", None);
    assert_eq!(search_keys(cx), ["first"]);
    cx.update(|_, app| {
        app.global_mut::<SearchHistory>()
            .set_store(Some(path.clone()))
    });
    other_window(cx, "second", None);
    assert_eq!(search_keys(cx), ["second", "first"]);
    let file = SearchHistory::load(Some(&path), None);
    assert_eq!(file.keys().search.items(), ["second", "first"]);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn a_string_the_file_cannot_hold_stays_in_memory(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "");
    let path = temp_ini("multiline");
    cx.update(|_, app| history::install(app, SearchHistory::load(None, Some(path.clone()))));
    other_window(cx, "a\nb", None);
    other_window(cx, "c", None);
    assert_eq!(search_keys(cx), ["c", "a\nb"]);
    let file = SearchHistory::load(Some(&path), None);
    assert_eq!(file.keys().search.items(), ["c"]);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn what_another_process_saved_is_news_to_the_windows(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    let path = temp_ini("news");
    cx.update(|_, app| history::install(app, SearchHistory::load(None, Some(path.clone()))));
    // this window searched, so nothing is news to it
    f.view.update(cx, |v, cx| {
        v.search.set_pattern("mine");
        v.remember_search_key(cx);
    });
    // another process saves a search string
    SearchHistory::load(None, Some(path.clone())).persist(|keys| {
        keys.add_search("theirs");
    });
    let before = cx.update(|_, app| app.global::<SearchHistory>().search_sequence());
    // this process then remembers a replacement: the file's search string comes in
    cx.update(|_, app| {
        history::remember_replace(app, "r");
    });
    let after = cx.update(|_, app| app.global::<SearchHistory>().search_sequence());
    assert!(after > before, "{before} {after}");
    assert_eq!(search_keys(cx), ["theirs", "mine"]);
    // and the window that had the old sequence takes it when Find opens
    cx.simulate_keystrokes("ctrl-f");
    assert_eq!(f.pattern(cx), "theirs");
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn searching_again_for_the_newest_string_puts_it_on_top_in_the_file(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "");
    let path = temp_ini("again");
    cx.update(|_, app| history::install(app, SearchHistory::load(None, Some(path.clone()))));
    other_window(cx, "x", None);
    // another process saves "y" after "x"
    SearchHistory::load(Some(&path), Some(path.clone())).persist(|keys| {
        keys.add_search("y");
    });
    // this process searches for "x" again, which it has as the newest
    other_window(cx, "x", None);
    let file = SearchHistory::load(Some(&path), None);
    assert_eq!(file.keys().search.items(), ["x", "y"]);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[gpui::test]
fn a_file_that_does_not_exist_yet_is_made(cx: &mut TestAppContext) {
    let (_f, cx) = setup(cx, "");
    let path = temp_ini("new");
    cx.update(|_, app| history::install(app, SearchHistory::load(None, Some(path.clone()))));
    other_window(cx, "x", None);
    let again = SearchHistory::load(Some(&path), None);
    assert_eq!(again.keys().search.items(), ["x"]);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

/// The report that started this: after `b+` found `bbbb` the find panel starts with `bbbb`
/// (the selection, as in Sakura), and `b+` can be picked from the drop-down.
#[gpui::test]
fn b_plus_is_in_the_history_after_it_found_bbbb(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abbbb");
    f.regex(cx);
    cx.simulate_keystrokes("ctrl-f");
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_input("b+");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 1), Pos::new(0, 5))));
    assert_eq!(search_keys(cx), ["b+"]);
    // the panel starts with the selection...
    cx.simulate_keystrokes("ctrl-f");
    assert_eq!(f.pattern(cx), "bbbb");
    // ...and the list has `b+`: "bbbb" is not a row, so Down takes the first
    cx.simulate_keystrokes("alt-down");
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("enter");
    assert_eq!(f.pattern(cx), "b+");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
}

#[gpui::test]
fn the_search_string_is_remembered_by_every_kind_of_search(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "foo bar");
    // the down-search button / Enter
    cx.simulate_keystrokes("ctrl-f secondary-a");
    cx.simulate_input("bar");
    cx.simulate_keystrokes("enter");
    assert_eq!(search_keys(cx), ["bar"]);
    // the search mark of a selection (Ctrl+F3)
    f.view
        .update(cx, |v, _| v.cursor.select(Pos::new(0, 0), Pos::new(0, 3)));
    cx.simulate_keystrokes("ctrl-f3");
    assert_eq!(search_keys(cx), ["foo", "bar"]);
}

#[gpui::test]
fn a_regular_expression_that_does_not_compile_is_not_kept(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.regex(cx);
    cx.simulate_keystrokes("ctrl-f secondary-a");
    cx.simulate_input("(");
    cx.simulate_keystrokes("enter");
    assert!(search_keys(cx).is_empty());
}

#[gpui::test]
fn a_replacement_is_not_kept_when_the_pattern_does_not_compile(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.regex(cx);
    cx.simulate_keystrokes("ctrl-r secondary-a");
    cx.simulate_input("(");
    let panel = f.view.read_with(cx, |v, _| v.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.focus_replacement(window, cx));
    cx.simulate_input("zzz");
    cx.simulate_keystrokes("enter");
    assert!(search_keys(cx).is_empty() && replace_keys(cx).is_empty());
}

#[gpui::test]
fn the_replace_dialog_keeps_both_strings(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("ctrl-r secondary-a");
    cx.simulate_input("a");
    let panel = f.view.read_with(cx, |v, _| v.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.focus_replacement(window, cx));
    cx.simulate_input("bb");
    assert!(search_keys(cx).is_empty());
    cx.simulate_keystrokes("enter"); // 置換 in the replacement input
    assert_eq!(search_keys(cx), ["a"]);
    assert_eq!(replace_keys(cx), ["bb"]);
}

#[gpui::test]
fn an_empty_replacement_is_kept_as_sakura_does(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "aa");
    cx.simulate_keystrokes("ctrl-r secondary-a");
    cx.simulate_input("a");
    let panel = f.view.read_with(cx, |v, _| v.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.focus_replacement(window, cx));
    cx.simulate_keystrokes("enter");
    assert_eq!(replace_keys(cx), [""]);
}

#[gpui::test]
fn without_a_word_the_panel_starts_with_the_newest_string(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    other_window(cx, "older", None);
    other_window(cx, "newest", None);
    cx.simulate_keystrokes("ctrl-f");
    assert_eq!(f.pattern(cx), "newest");
}

#[gpui::test]
fn a_word_at_the_caret_comes_before_the_history(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "word");
    other_window(cx, "newest", None);
    cx.simulate_keystrokes("ctrl-f");
    assert_eq!(f.pattern(cx), "word");
}

#[gpui::test]
fn a_window_takes_what_another_window_searched_for(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    // this window searched for "mine"; nothing is news to it
    f.view.update(cx, |v, cx| {
        v.search.set_pattern("mine");
        v.remember_search_key(cx);
    });
    cx.simulate_keystrokes("ctrl-f escape");
    assert_eq!(f.pattern(cx), "mine");
    // another window searches for "theirs": now it is news (`m_nCurSearchKeySequence`)
    other_window(cx, "theirs", None);
    cx.simulate_keystrokes("ctrl-f");
    assert_eq!(f.pattern(cx), "theirs");
}

fn inherit(cx: &mut VisualTestContext, on: bool) {
    cx.update(|_, app| {
        app.set_global(crate::search::prefs::SearchPrefs {
            inherit_key_other_view: on,
            ..Default::default()
        });
    });
}

#[gpui::test]
fn f3_takes_what_another_view_searched_for(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "mine theirs mine");
    f.view.update(cx, |v, cx| {
        v.search.set_pattern("mine");
        v.remember_search_key(cx);
    });
    other_window(cx, "theirs", None);
    // `ChangeCurRegexp`: the view has not seen the newer search, so it takes it.
    cx.simulate_keystrokes("f3");
    assert_eq!(f.pattern(cx), "theirs");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 5), Pos::new(0, 11))));
}

#[gpui::test]
fn f3_keeps_its_own_string_when_the_switch_is_off(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "mine theirs mine");
    inherit(cx, false);
    f.view.update(cx, |v, cx| {
        v.search.set_pattern("mine");
        v.remember_search_key(cx);
    });
    other_window(cx, "theirs", None);
    cx.simulate_keystrokes("f3");
    assert_eq!(f.pattern(cx), "mine");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 4))));
}

#[gpui::test]
fn f3_of_a_view_with_no_string_takes_the_newest_even_when_the_switch_is_off(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx, "mine theirs mine");
    inherit(cx, false);
    other_window(cx, "theirs", None);
    // `0 == m_strCurSearchKey.size()`
    cx.simulate_keystrokes("f3");
    assert_eq!(f.pattern(cx), "theirs");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 5), Pos::new(0, 11))));
}

/// Runs a Sakura function by name, the way a key or a menu item does.
fn run(f: &Fixture, cx: &mut VisualTestContext, name: &str) {
    let code = FUNCTIONS.iter().find(|f| f.name == name).unwrap().code;
    f.view
        .update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
}

fn text(f: &Fixture, cx: &mut VisualTestContext) -> String {
    f.view.read_with(cx, |v, cx| {
        let doc = v.document.read(cx).doc();
        doc.text_between(Pos::new(0, 0), Pos::new(doc.line_count(), 0))
            .unwrap()
    })
}

fn seen_mine(f: &Fixture, cx: &mut VisualTestContext) {
    f.view.update(cx, |v, cx| {
        v.search.set_pattern("mine");
        v.search.replace.text = "X".to_owned();
        v.remember_search_key(cx);
    });
}

#[gpui::test]
fn replace_starts_with_the_search_of_another_view(cx: &mut TestAppContext) {
    // `Command_REPLACE` begins with `ChangeCurRegexp(false)` (`CViewCommander_Search.cpp`).
    let (f, cx) = setup(cx, "mine theirs mine");
    seen_mine(&f, cx);
    other_window(cx, "theirs", None);
    run(&f, cx, "F_REPLACE");
    assert_eq!(f.pattern(cx), "theirs");
    assert_eq!(text(&f, cx), "mine X mine");
    // 次を検索 follows the same pattern, and the matches are marked
    assert!(f.view.read_with(cx, |v, _| v.search.mark()));
    cx.simulate_keystrokes("f3");
    assert_eq!(f.pattern(cx), "theirs");
}

#[gpui::test]
fn replace_all_starts_with_the_search_of_another_view(cx: &mut TestAppContext) {
    // `Command_REPLACE_ALL` begins with `ChangeCurRegexp()`.
    let (f, cx) = setup(cx, "mine theirs theirs");
    seen_mine(&f, cx);
    other_window(cx, "theirs", None);
    run(&f, cx, "F_REPLACE_ALL");
    assert_eq!(f.pattern(cx), "theirs");
    assert_eq!(text(&f, cx), "mine X X");
    assert!(f.view.read_with(cx, |v, _| v.search.mark()));
}

#[gpui::test]
fn replace_keeps_its_own_string_when_the_switch_is_off(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "mine theirs mine");
    inherit(cx, false);
    seen_mine(&f, cx);
    other_window(cx, "theirs", None);
    run(&f, cx, "F_REPLACE");
    assert_eq!(f.pattern(cx), "mine");
    assert_eq!(text(&f, cx), "X theirs mine");
    run(&f, cx, "F_REPLACE_ALL");
    assert_eq!(text(&f, cx), "X theirs X");
}

#[gpui::test]
fn replace_of_a_view_with_no_string_takes_the_newest(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "mine theirs mine");
    inherit(cx, false);
    other_window(cx, "theirs", None);
    f.view
        .update(cx, |v, _| v.search.replace.text = "X".to_owned());
    run(&f, cx, "F_REPLACE");
    // `0 == m_strCurSearchKey.size()`
    assert_eq!(f.pattern(cx), "theirs");
    assert_eq!(text(&f, cx), "mine X mine");
}

#[gpui::test]
fn the_replace_dialog_starts_with_the_previous_replacement(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    other_window(cx, "x", Some("old"));
    other_window(cx, "x", Some("new"));
    cx.simulate_keystrokes("ctrl-r");
    assert_eq!(f.replacement(cx), "new");
}

#[gpui::test]
fn alt_down_opens_the_list_and_enter_picks_from_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    for s in ["a", "b", "c"] {
        other_window(cx, s, None);
    }
    cx.simulate_keystrokes("ctrl-f secondary-a");
    cx.simulate_input("zzz");
    cx.simulate_keystrokes("alt-down down down down enter");
    // c, b, a: the list had no "zzz", so nothing is highlighted; three Downs reach a
    assert_eq!(f.pattern(cx), "a");
    // Enter closed the list and did not search
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    assert_eq!(f.selection(cx), None);
}

#[gpui::test]
fn escape_closes_the_list_first_and_then_the_panel(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    other_window(cx, "a", None);
    cx.simulate_keystrokes("ctrl-f alt-down escape");
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    cx.simulate_keystrokes("escape");
    assert_eq!(f.panel_mode(cx), None);
}

#[gpui::test]
fn up_and_down_go_through_the_strings_with_the_list_closed(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "");
    for s in ["a", "b", "c"] {
        other_window(cx, s, None);
    }
    cx.simulate_keystrokes("ctrl-f");
    // the panel started with the newest, "c"
    assert_eq!(f.pattern(cx), "c");
    cx.simulate_keystrokes("down");
    assert_eq!(f.pattern(cx), "b");
    cx.simulate_keystrokes("down down");
    assert_eq!(f.pattern(cx), "a");
    cx.simulate_keystrokes("up");
    assert_eq!(f.pattern(cx), "b");
}

#[gpui::test]
fn the_replacement_input_has_its_own_list(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    other_window(cx, "s", Some("r1"));
    other_window(cx, "s", Some("r2"));
    cx.simulate_keystrokes("ctrl-r");
    let panel = f.view.read_with(cx, |v, _| v.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.focus_replacement(window, cx));
    cx.simulate_keystrokes("alt-down down enter");
    assert_eq!(f.replacement(cx), "r1");
    // the search string is not touched by the replacement's list
    assert_eq!(f.pattern(cx), "x");
}

#[gpui::test]
fn the_list_of_a_panel_without_history_does_not_open(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    // End drops the selection of the opening word: the headless keys put the character of Enter
    // in before the Enter handler runs, which would replace a selection with nothing
    cx.simulate_keystrokes("ctrl-f end alt-down enter");
    // Enter went to the input: 下検索
    assert_eq!(f.selection(cx), Some((Pos::new(0, 0), Pos::new(0, 1))));
}

// ---- the ▼ of a combo box is not a Tab stop ----

#[gpui::test]
fn tab_from_the_search_string_goes_to_the_replacement_and_not_to_the_arrow(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx, "x");
    // `IDD_REPLACE`: 置換前 and 置換後 are `CBS_DROPDOWN` combo boxes; Tab goes from one to the next
    cx.simulate_keystrokes("ctrl-r tab");
    cx.simulate_input("zz");
    assert_eq!(f.replacement(cx), "zz");
}

#[gpui::test]
fn the_arrow_opens_the_list_by_click_and_the_keys_still_work(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    for s in ["a", "b"] {
        other_window(cx, s, Some(s));
    }
    cx.simulate_keystrokes("ctrl-r");
    // the search box has the word at the caret ("x", not a row): Down takes "b", the next "a";
    // the replacement box has "b" (a row): one Down reaches "a"
    for (toggle, keys) in [
        ("search-history-toggle", "down down enter"),
        ("replace-history-toggle", "down enter"),
    ] {
        let arrow = cx.debug_bounds(toggle).expect("the arrow");
        cx.simulate_click(arrow.center(), gpui::Modifiers::none());
        cx.simulate_keystrokes(keys);
    }
    assert_eq!(f.pattern(cx), "a");
    assert_eq!(f.replacement(cx), "a");
}

#[gpui::test]
fn f4_still_opens_the_list_of_the_replacement(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    other_window(cx, "s", Some("r1"));
    other_window(cx, "s", Some("r2"));
    cx.simulate_keystrokes("ctrl-r");
    let panel = f.view.read_with(cx, |v, _| v.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.focus_replacement(window, cx));
    // the box has "r2" (the newest); the list opens on it and Down reaches "r1"
    cx.simulate_keystrokes("f4 down enter");
    assert_eq!(f.replacement(cx), "r1");
}

#[gpui::test]
fn enter_in_an_open_list_only_takes_the_row_and_does_not_search(cx: &mut TestAppContext) {
    // `CBS_DROPDOWN`: with the list dropped down, Enter takes the item and closes the list; the
    // default button of the dialog is not pressed
    let (f, cx) = setup(cx, " bbb ccc");
    for s in ["aaa", "bbb", "ccc"] {
        other_window(cx, s, None);
    }
    // the panel starts with the word at the caret; Down from a string that is not in the list
    // takes the first (the newest)
    cx.simulate_keystrokes("ctrl-f down");
    assert_eq!(f.pattern(cx), "ccc");
    let before = f.selection(cx);
    // the list opens on "ccc"; Down goes to "bbb"
    cx.simulate_keystrokes("f4 down enter");
    assert_eq!(f.pattern(cx), "bbb");
    assert_eq!(f.selection(cx), before, "the caret did not move");
    // the list is closed: a second Enter searches
    cx.simulate_keystrokes("enter");
    assert_eq!(f.selection(cx), Some((Pos::new(0, 1), Pos::new(0, 4))));
}

#[gpui::test]
fn enter_in_an_open_list_of_the_replacement_does_not_replace(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    other_window(cx, "x", Some("r1"));
    other_window(cx, "x", Some("r2"));
    cx.simulate_keystrokes("ctrl-r");
    let panel = f.view.read_with(cx, |v, _| v.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.focus_replacement(window, cx));
    cx.simulate_keystrokes("f4 down enter");
    assert_eq!(f.replacement(cx), "r1");
    assert_eq!(text(&f, cx), "x", "nothing was replaced");
    // the list is closed: the second Enter replaces
    cx.simulate_keystrokes("enter");
    assert_eq!(text(&f, cx), "r1");
}

fn list_open(f: &Fixture, cx: &mut VisualTestContext) -> bool {
    f.view.read_with(cx, |v, app| {
        v.panel
            .as_ref()
            .is_some_and(|panel| panel.read(app).open_list().is_some())
    })
}

/// Measured on a Windows combo box: Down in the open list puts the row in the box at once; Escape
/// or Alt+Up closes the list and keeps that text; Up right after opening, from a text that is not
/// a row, takes the first row; Down stops at the last row.
#[gpui::test]
fn the_arrows_of_an_open_list_put_the_row_in_the_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    for s in ["aaa", "bbb", "ccc"] {
        other_window(cx, s, None);
    }
    cx.simulate_keystrokes("ctrl-f secondary-a");
    cx.simulate_input("typed");
    let before = f.selection(cx);
    cx.simulate_keystrokes("f4");
    assert_eq!(
        (f.pattern(cx), list_open(&f, cx)),
        ("typed".to_owned(), true)
    );
    cx.simulate_keystrokes("down");
    assert_eq!((f.pattern(cx), list_open(&f, cx)), ("ccc".to_owned(), true));
    cx.simulate_keystrokes("down");
    assert_eq!((f.pattern(cx), list_open(&f, cx)), ("bbb".to_owned(), true));
    // Escape keeps the text the arrows put there, and nothing is searched
    cx.simulate_keystrokes("escape");
    assert_eq!(
        (f.pattern(cx), list_open(&f, cx)),
        ("bbb".to_owned(), false)
    );
    assert_eq!(f.panel_mode(cx), Some(PanelMode::Find));
    assert_eq!(f.selection(cx), before);
    // Alt+Up closes too
    cx.simulate_keystrokes("f4 down alt-up");
    assert_eq!(
        (f.pattern(cx), list_open(&f, cx)),
        ("aaa".to_owned(), false)
    );
    // Down stops at the last row
    cx.simulate_keystrokes("f4 down down down");
    assert_eq!((f.pattern(cx), list_open(&f, cx)), ("aaa".to_owned(), true));
}

#[gpui::test]
fn up_right_after_opening_takes_the_first_row(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    for s in ["aaa", "bbb"] {
        other_window(cx, s, None);
    }
    cx.simulate_keystrokes("ctrl-f secondary-a");
    cx.simulate_input("typed");
    cx.simulate_keystrokes("f4 up");
    assert_eq!((f.pattern(cx), list_open(&f, cx)), ("bbb".to_owned(), true));
    // Up at the first row stays
    cx.simulate_keystrokes("up");
    assert_eq!((f.pattern(cx), list_open(&f, cx)), ("bbb".to_owned(), true));
}

#[gpui::test]
fn the_arrows_of_the_open_list_of_the_replacement_put_the_row_in_its_box(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    other_window(cx, "s", Some("r1"));
    other_window(cx, "s", Some("r2"));
    cx.simulate_keystrokes("ctrl-r");
    let pattern = f.pattern(cx);
    let panel = f.view.read_with(cx, |v, _| v.panel.clone().unwrap());
    panel.update_in(cx, |p, window, cx| p.focus_replacement(window, cx));
    cx.simulate_keystrokes("f4 down");
    assert_eq!(f.replacement(cx), "r1");
    cx.simulate_keystrokes("escape");
    assert_eq!(
        (f.replacement(cx), list_open(&f, cx)),
        ("r1".to_owned(), false)
    );
    // the search box was not touched
    assert_eq!(f.pattern(cx), pattern);
}

/// The highlighted row is the row that holds the text of the box, not a number kept when the list
/// opened: after the text is typed over, the next arrow starts from the first row.
#[gpui::test]
fn an_arrow_after_typing_in_the_open_list_starts_from_the_text(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "x");
    for s in ["aaa", "bbb", "ccc"] {
        other_window(cx, s, None);
    }
    cx.simulate_keystrokes("ctrl-f secondary-a");
    cx.simulate_input("typed");
    cx.simulate_keystrokes("f4 down down");
    assert_eq!(f.pattern(cx), "bbb");
    cx.simulate_input("x");
    assert_eq!(f.pattern(cx), "bbbx");
    cx.simulate_keystrokes("down");
    assert_eq!(f.pattern(cx), "ccc");
    assert!(list_open(&f, cx));
}
