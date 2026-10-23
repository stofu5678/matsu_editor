//! Tests of the help functions in GPUI's headless test window: which file or page each function
//! opens, with which word, and the question when no file is set. The rules of the choice are
//! tested in [`super::help`]; nothing is started, the tests read what would have been opened.
//!
//! Expectations cite `sakura_core/cmd/CViewCommander_Support.cpp` and
//! `sakura_core/env/CHelpManager.cpp`.

use std::path::PathBuf;

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::Document;
use matsu_core::model::DocumentModel;

use super::help::{CONTENTS_URL, INDEX_URL, NO_KEYWORD_LOOKUP, OPENED, Opening};
use super::prefs::HelperPrefs;
use crate::document::SharedDocument;
use crate::settings::UserSettings;
use crate::view::EditorView;

fn code_of(name: &str) -> u16 {
    FUNCTIONS.iter().find(|f| f.name == name).unwrap().code
}

struct Fixture {
    view: Entity<EditorView>,
    dir: tempfile::TempDir,
}

/// A view over `text` with the settings `types_0` (the type of an untitled document) in a private
/// folder, and `prefs` as the common settings of the page 支援.
fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    types_0: &str,
    prefs: HelperPrefs,
) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("matsu.ini");
    let profile = matsu_config::Profile::from_bytes(format!("[Types(0)]\r\n{types_0}").as_bytes());
    let user = UserSettings::from_profile(profile, Some(path));
    cx.update(|app| {
        app.set_global(user);
        app.set_global(prefs);
    });
    OPENED.with(|opened| opened.borrow_mut().clear());
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    (Fixture { view, dir }, vcx)
}

impl Fixture {
    fn run(&self, cx: &mut VisualTestContext, function: &str, args: &[Arg]) {
        let code = code_of(function);
        self.view
            .update_in(cx, |v, window, cx| v.run_function(code, args, window, cx));
        cx.run_until_parked();
    }

    fn notice(&self, cx: &mut VisualTestContext) -> Option<String> {
        self.view.read_with(cx, |v, _| v.notice.clone())
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
}

fn opened() -> Vec<Opening> {
    OPENED.with(|opened| opened.borrow().clone())
}

fn keyword(file: PathBuf, word: &str, single: bool) -> Opening {
    Opening::Keyword {
        file,
        keyword: word.to_owned(),
        single,
    }
}

// ---- F_EXTHELP1 ----

#[gpui::test]
fn exthelp1_opens_the_html_help_of_the_type_at_the_word_under_the_caret(cx: &mut TestAppContext) {
    // a relative path starts at the folder of the settings file; the word is the one at the caret
    let (f, cx) = setup(
        cx,
        "hello world",
        "szExtHelp=docs/a.chm\r\nszExtHtmlHelp=x.chm\r\nbTypeHtmlHelpIsSingle=0\r\n",
        HelperPrefs::default(),
    );
    f.run(cx, "F_EXTHELP1", &[]);
    assert_eq!(opened(), [keyword(f.path("docs/a.chm"), "hello", false)]);
    assert_eq!(f.notice(cx).as_deref(), Some(NO_KEYWORD_LOOKUP));
}

#[gpui::test]
fn exthelp1_takes_the_selection_as_the_word(cx: &mut TestAppContext) {
    let (f, cx) = setup(
        cx,
        "hello world",
        "szExtHelp=a.chm\r\n",
        HelperPrefs::default(),
    );
    cx.simulate_keystrokes(
        "shift-right shift-right shift-right shift-right shift-right shift-right shift-right",
    );
    f.run(cx, "F_EXTHELP1", &[]);
    // `GetCurrentTextForSearch`: the selection up to the end of its first line
    assert_eq!(opened(), [keyword(f.path("a.chm"), "hello w", true)]);
}

#[gpui::test]
fn exthelp1_uses_the_common_file_when_the_type_has_none_and_opens_other_files_as_files(
    cx: &mut TestAppContext,
) {
    // `GetExtWinHelp`; a file that is not HTML help is a WinHelp file
    let prefs = HelperPrefs {
        ext_help: "common.hlp".to_owned(),
        ..HelperPrefs::default()
    };
    let (f, cx) = setup(cx, "hello", "", prefs);
    f.run(cx, "F_EXTHELP1", &[]);
    assert_eq!(opened(), [Opening::File(f.path("common.hlp"))]);
}

#[gpui::test]
fn the_file_of_the_type_comes_before_the_common_one(cx: &mut TestAppContext) {
    let prefs = HelperPrefs {
        ext_help: "common.chm".to_owned(),
        html_help_is_single: false,
        ..HelperPrefs::default()
    };
    let (f, cx) = setup(cx, "hello", "szExtHelp=type.CHM\r\n", prefs);
    f.run(cx, "F_EXTHELP1", &[]);
    // the viewer is the common one: the type has no HTML help file of its own
    assert_eq!(opened(), [keyword(f.path("type.CHM"), "hello", false)]);
}

#[gpui::test]
fn without_a_file_exthelp1_asks_and_yes_shows_the_page_of_the_common_settings(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx, "hello", "", HelperPrefs::default());
    f.run(cx, "F_EXTHELP1", &[]);
    assert!(opened().is_empty());
    // `STR_ERR_CEDITVIEW_CMD01`
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    let page =
        cx.update(|_, app| crate::common_settings::current_dialog(app).map(|d| d.read(app).page()));
    assert_eq!(page, Some(crate::common_settings::Page::Support));
}

#[gpui::test]
fn no_answers_to_the_question_open_nothing(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "hello", "", HelperPrefs::default());
    f.run(cx, "F_EXTHELP1", &[]);
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("いいえ(N)");
    cx.run_until_parked();
    assert!(opened().is_empty());
    assert!(cx.update(|_, app| crate::common_settings::current_dialog(app).is_none()));
}

// ---- F_EXTHTMLHELP ----

#[gpui::test]
fn exthtmlhelp_opens_the_file_of_the_settings_at_the_word(cx: &mut TestAppContext) {
    let prefs = HelperPrefs {
        ext_html_help: "common.chm".to_owned(),
        html_help_is_single: false,
        ..HelperPrefs::default()
    };
    let (f, cx) = setup(cx, "hello world", "", prefs);
    f.run(cx, "F_EXTHTMLHELP", &[]);
    assert_eq!(opened(), [keyword(f.path("common.chm"), "hello", false)]);
}

#[gpui::test]
fn the_type_decides_the_file_and_the_viewer_when_it_has_a_file(cx: &mut TestAppContext) {
    // `HTMLHelpIsSingle`: the flag of the type counts with its file
    let prefs = HelperPrefs {
        ext_html_help: "common.chm".to_owned(),
        html_help_is_single: true,
        ..HelperPrefs::default()
    };
    let (f, cx) = setup(
        cx,
        "hello",
        "szExtHtmlHelp=type.chm\r\nbTypeHtmlHelpIsSingle=0\r\n",
        prefs,
    );
    f.run(cx, "F_EXTHTMLHELP", &[]);
    assert_eq!(opened(), [keyword(f.path("type.chm"), "hello", false)]);
}

#[gpui::test]
fn a_macro_can_give_the_file_and_the_word(cx: &mut TestAppContext) {
    // `Command_EXTHTMLHELP( helpfile, kwd )`: given texts win over the settings and the caret
    let (f, cx) = setup(
        cx,
        "hello",
        "szExtHtmlHelp=type.chm\r\n",
        HelperPrefs::default(),
    );
    let abs = f.path("abs.chm");
    f.run(
        cx,
        "F_EXTHTMLHELP",
        &[
            Arg::Text(abs.to_string_lossy().into_owned()),
            Arg::Text("macro word".to_owned()),
        ],
    );
    assert_eq!(opened(), [keyword(abs, "macro word", true)]);
    // an empty text counts as missing
    OPENED.with(|o| o.borrow_mut().clear());
    f.run(
        cx,
        "F_EXTHTMLHELP",
        &[Arg::Text(String::new()), Arg::Text(String::new())],
    );
    assert_eq!(opened(), [keyword(f.path("type.chm"), "hello", true)]);
}

#[gpui::test]
fn exthtmlhelp_without_a_file_asks(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "hello", "", HelperPrefs::default());
    f.run(cx, "F_EXTHTMLHELP", &[]);
    assert!(opened().is_empty());
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("キャンセル");
    cx.run_until_parked();
    assert!(opened().is_empty());
}

// ---- F_HELP_CONTENTS and F_HELP_SEARCH ----

#[gpui::test]
fn the_help_of_the_program_is_the_documents_of_the_project(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "hello", "", HelperPrefs::default());
    f.run(cx, "F_HELP_CONTENTS", &[]);
    f.run(cx, "F_HELP_SEARCH", &[]);
    assert_eq!(
        opened(),
        [
            Opening::Url(CONTENTS_URL.to_owned()),
            Opening::Url(INDEX_URL.to_owned())
        ]
    );
    // nothing is lost: no notice
    assert_eq!(f.notice(cx), None);
}
