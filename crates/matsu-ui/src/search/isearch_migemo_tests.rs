//! Tests of the Migemo search (`isearch.rs`, `migemo.rs`) with a library that answers from a
//! table: CI has no library of C/Migemo and no dictionary.
//!
//! The rules are those of `ISearchEnter` and `ISearchWordMake` in
//! `sakura_core/view/CEditView_Cmdisrch.cpp`. None of the tests touches a path or a line ending
//! of the OS (the texts use `\r\n` in the document itself, and the library is a fake).

use std::rc::Rc;

use gpui::{TestAppContext, VisualTestContext};
use matsu_core::buffer::Pos;

use super::isearch_tests::{Fixture, range, setup};
use super::migemo::{MigemoEngine, MigemoHost, MigemoPaths, Unavailable};

/// What the fake library answers with.
type Answer = Box<dyn Fn(&str) -> Option<String>>;

/// A library that answers from a function.
struct FakeMigemo(Answer);

impl MigemoEngine for FakeMigemo {
    fn query(&self, text: &str) -> Option<String> {
        (self.0)(text)
    }
}

/// Puts a fake library in place of the real one. `open` is what opening it gives.
fn install(cx: &mut TestAppContext, open: impl Fn() -> Result<FakeMigemo, Unavailable> + 'static) {
    cx.update(|app| {
        app.set_global(MigemoHost::new(Box::new(move |_| {
            open().map(|engine| Rc::new(engine) as Rc<dyn MigemoEngine>)
        })));
    });
}

/// A small dictionary, as C/Migemo writes it: a group of the typed text and its kana and kanji.
fn words(key: &str) -> Option<String> {
    let pattern = match key {
        "k" => "(k|\u{ff4b}|か|き|柿)",
        "ka" => "(ka|\u{ff4b}\u{ff41}|か|カ|ｶ|柿)",
        "kaki" => "(kaki|かき|カキ|柿|牡蠣)",
        _ => return None,
    };
    Some(pattern.to_owned())
}

fn dictionary() -> Result<FakeMigemo, Unavailable> {
    Ok(FakeMigemo(Box::new(words)))
}

impl Fixture {
    /// The text of the selection.
    fn selected(&self, cx: &mut VisualTestContext) -> Option<String> {
        let (from, to) = self.selection(cx)?;
        self.document
            .read_with(cx, |shared, _| shared.doc().text_between(from, to).ok())
    }
}

#[gpui::test]
fn the_migemo_search_finds_what_the_library_makes_of_the_text(cx: &mut TestAppContext) {
    install(cx, dictionary);
    let (f, cx) = setup(cx, "abc ka\r\nか 柿 かき");
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert_eq!(f.message(cx).as_deref(), Some("[Migemo] I-Search: "));
    cx.simulate_input("ka");
    assert_eq!(f.message(cx).as_deref(), Some("[Migemo] I-Search: ka"));
    assert_eq!(f.selected(cx).as_deref(), Some("ka"));
    assert_eq!(f.selection(cx), range((0, 4), (0, 6)));
    // the text typed is the key, the pattern of the search is the library's
    assert_eq!(f.key(cx).as_deref(), Some("ka"));
    assert_eq!(f.pattern(cx), "(ka|\u{ff4b}\u{ff41}|か|カ|ｶ|柿)");
    let options = f.view.read_with(cx, |v, _| v.search_state().options());
    assert!(options.regex && !options.match_case);
    // the command again goes to the next match
    for found in ["か", "柿", "か"] {
        f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
        assert_eq!(f.selected(cx).as_deref(), Some(found));
    }
    // the text is not touched
    assert_eq!(f.text(cx), "abc ka\r\nか 柿 かき");
}

#[gpui::test]
fn the_migemo_search_wraps_after_a_miss(cx: &mut TestAppContext) {
    install(cx, dictionary);
    let (f, cx) = setup(cx, "ka あ か");
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    cx.simulate_input("ka");
    assert_eq!(f.selected(cx).as_deref(), Some("ka"));
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert_eq!(f.selected(cx).as_deref(), Some("か"));
    // nothing more: the search stays and says so
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert_eq!(f.selected(cx).as_deref(), Some("か"));
    assert_eq!(
        f.message(cx).as_deref(),
        Some("[Migemo] I-Search: ka (見つかりません)")
    );
    // and the next one begins at the top
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert_eq!(f.selected(cx).as_deref(), Some("ka"));
    assert_eq!(f.selection(cx), range((0, 0), (0, 2)));
}

#[gpui::test]
fn the_backward_migemo_search_goes_up_and_wraps_to_the_end(cx: &mut TestAppContext) {
    install(cx, dictionary);
    let (f, cx) = setup(cx, "か ka\r\n柿");
    f.caret_to(cx, Pos::new(1, 3));
    f.run(cx, "F_ISEARCH_MIGEMO_PREV");
    assert_eq!(
        f.message(cx).as_deref(),
        Some("[Migemo] I-Search Backward: ")
    );
    cx.simulate_input("ka");
    assert_eq!(f.selected(cx).as_deref(), Some("柿"));
    f.run(cx, "F_ISEARCH_MIGEMO_PREV");
    assert_eq!(f.selected(cx).as_deref(), Some("ka"));
    f.run(cx, "F_ISEARCH_MIGEMO_PREV");
    assert_eq!(f.selected(cx).as_deref(), Some("か"));
    f.run(cx, "F_ISEARCH_MIGEMO_PREV");
    assert!(f.message(cx).unwrap().ends_with("(見つかりません)"));
    f.run(cx, "F_ISEARCH_MIGEMO_PREV");
    assert_eq!(f.selected(cx).as_deref(), Some("柿"));
}

#[gpui::test]
fn backspace_asks_the_library_again(cx: &mut TestAppContext) {
    install(cx, dictionary);
    let (f, cx) = setup(cx, "ka あ か");
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    cx.simulate_input("ka");
    assert_eq!(f.selected(cx).as_deref(), Some("ka"));
    cx.simulate_keystrokes("backspace");
    assert_eq!(f.key(cx).as_deref(), Some("k"));
    assert_eq!(f.pattern(cx), "(k|\u{ff4b}|か|き|柿)");
    // the library does not know "kx": the text itself is searched for
    cx.simulate_input("x");
    assert_eq!(f.pattern(cx), "kx");
    assert!(f.message(cx).unwrap().ends_with("(見つかりません)"));
}

#[gpui::test]
fn leaving_keeps_the_typed_text_as_the_search_like_sakura(cx: &mut TestAppContext) {
    install(cx, dictionary);
    let (f, cx) = setup(cx, "ka かき");
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    cx.simulate_input("kaki");
    assert_eq!(f.selected(cx).as_deref(), Some("かき"));
    cx.simulate_keystrokes("enter");
    assert!(!f.active(cx));
    // `ISearchExit` keeps `m_strCurSearchKey` and the option 正規表現
    assert_eq!(f.pattern(cx), "kaki");
    let options = f.view.read_with(cx, |v, _| v.search_state().options());
    assert!(options.regex);
}

#[gpui::test]
fn the_mode_switches_between_normal_and_migemo_keeping_the_text(cx: &mut TestAppContext) {
    install(cx, dictionary);
    let (f, cx) = setup(cx, "ka かき");
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("ka");
    assert_eq!(f.selection(cx), range((0, 0), (0, 2)));
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert_eq!(f.key(cx).as_deref(), Some("ka"));
    assert!(f.message(cx).unwrap().starts_with("[Migemo] I-Search"));
    // the next typed char begins a new pattern, as after any start of a mode
    cx.simulate_input("k");
    assert_eq!(f.key(cx).as_deref(), Some("k"));
    assert_eq!(f.pattern(cx), "(k|\u{ff4b}|か|き|柿)");
    // back to the plain search: the library is not asked
    f.run(cx, "F_ISEARCH_NEXT");
    cx.simulate_input("ka");
    assert_eq!(f.pattern(cx), "ka");
    let options = f.view.read_with(cx, |v, _| v.search_state().options());
    assert!(!options.regex);
}

#[gpui::test]
fn a_library_that_cannot_be_loaded_says_so_and_the_mode_does_not_begin(cx: &mut TestAppContext) {
    install(cx, || Err(Unavailable::Library));
    let (f, cx) = setup(cx, "ka");
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert!(!f.active(cx));
    let notice = f.view.read_with(cx, |v, cx| v.status(cx).notice);
    assert_eq!(notice.as_deref(), Some("MIGEMO.DLLが使用できません。"));
    // a key typed afterwards is typed into the text, not into a search
    cx.simulate_input("x");
    assert_eq!(f.text(cx), "xka");
}

#[gpui::test]
fn a_missing_dictionary_says_so_and_the_mode_does_not_begin(cx: &mut TestAppContext) {
    install(cx, || Err(Unavailable::Dictionary));
    let (f, cx) = setup(cx, "ka");
    f.run(cx, "F_ISEARCH_MIGEMO_PREV");
    assert!(!f.active(cx));
    let notice = f.view.read_with(cx, |v, cx| v.status(cx).notice);
    assert_eq!(notice.as_deref(), Some("MIGEMOは使用できません。"));
}

#[gpui::test]
fn a_library_that_gives_nothing_ends_the_search_and_says_so(cx: &mut TestAppContext) {
    // no silent search for the typed romaji as a regular expression: a text like `a.b(` could
    // even be an invalid pattern
    install(cx, || Ok(FakeMigemo(Box::new(|_| None))));
    let (f, cx) = setup(cx, "a.c abc");
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert!(f.active(cx));
    cx.simulate_input("a");
    assert!(!f.active(cx));
    let notice = f.view.read_with(cx, |v, cx| v.status(cx).notice);
    assert_eq!(notice.as_deref(), Some("MIGEMOは使用できません。"));
}

#[gpui::test]
fn another_windows_settings_do_not_change_a_running_search(cx: &mut TestAppContext) {
    // The library is opened per paths: the paths "broken" give none, the others the dictionary.
    cx.update(|app| {
        app.set_global(MigemoHost::new(Box::new(|paths: &MigemoPaths| {
            if paths.dictionary == "broken" {
                return Err(Unavailable::Dictionary);
            }
            Ok(Rc::new(FakeMigemo(Box::new(words))) as Rc<dyn MigemoEngine>)
        })));
    });
    let (f, cx) = setup(cx, "ka あ か");
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    cx.simulate_input("k");
    assert_eq!(f.selected(cx).as_deref(), Some("か"));
    // Another window changes the setting and begins its own search with it: that fails, and
    // must not touch the library of the search that is running.
    cx.update(|_, app| {
        app.set_global(crate::support::prefs::HelperPrefs {
            migemo_dict: "broken".to_owned(),
            ..Default::default()
        });
        let paths = MigemoPaths::current(app);
        assert!(app.default_global::<MigemoHost>().open(&paths).is_err());
    });
    cx.simulate_input("a");
    assert_eq!(f.key(cx).as_deref(), Some("ka"));
    assert_eq!(f.pattern(cx), "(ka|\u{ff4b}\u{ff41}|か|カ|ｶ|柿)");
    assert_eq!(f.selected(cx).as_deref(), Some("ka"));
    assert!(f.active(cx));
}

/// The syntax C/Migemo writes: groups, alternatives, classes and the backslash before each
/// character of `pcre_int2char` (`\ . * ^ $ / [ ] | ( ) + ? { } : - &`). The engine of MATSU
/// reads every one of them as the character itself.
#[gpui::test]
fn the_backslash_escapes_of_migemo_are_read_as_the_character(cx: &mut TestAppContext) {
    for c in "\\.*^$/[]|()+?{}:-&".chars() {
        let text = format!("ab{c}cd");
        let escaped = format!("(zz|\\{c})");
        install(cx, move || {
            let escaped = escaped.clone();
            Ok(FakeMigemo(Box::new(move |_| Some(escaped.clone()))))
        });
        let (f, vcx) = setup(cx, &text);
        f.run(vcx, "F_ISEARCH_MIGEMO_NEXT");
        vcx.simulate_input("q");
        assert_eq!(
            f.selected(vcx).as_deref(),
            Some(c.to_string().as_str()),
            "{c}"
        );
    }
}

#[gpui::test]
fn migemo_groups_classes_and_wide_letters_work_together(cx: &mut TestAppContext) {
    install(cx, || {
        Ok(FakeMigemo(Box::new(|_| {
            Some("(q|ｑ|[あいうえおｱｲｳｴｵ]|(か|カ)(き|キ)|\\[[\\-\\]]|\\&)".to_owned())
        })))
    });
    let (f, cx) = setup(cx, "xx ｵ yy カキ [- zz");
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    cx.simulate_input("q");
    assert_eq!(f.selected(cx).as_deref(), Some("ｵ"));
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert_eq!(f.selected(cx).as_deref(), Some("カキ"));
    f.run(cx, "F_ISEARCH_MIGEMO_NEXT");
    assert_eq!(f.selected(cx).as_deref(), Some("[-"));
}
