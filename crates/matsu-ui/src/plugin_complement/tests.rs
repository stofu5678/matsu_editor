//! Tests of the completion jacks on a window, with the real `matsu-script` (skipped when that program
//! is not built next to the tests). A plugin `Comp` has a plug of `ComplementGlobal` and one of
//! `Complement`; its handlers add candidates, and the tests look at the list that the window shows.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use gpui::{AppContext as _, Entity, EntityInputHandler as _, TestAppContext, VisualTestContext};
use matsu_config::Profile;
use matsu_config::plugins::consent::{ConsentFile, fingerprint};
use matsu_config::plugins::{DEFAULT_LANGUAGE, PluginRec, PluginSettings, PluginState, SearchDirs};
use matsu_core::buffer::Document;
use matsu_core::model::DocumentModel;

use super::*;
use crate::document::SharedDocument;
use crate::macros::plugin_call::tests::script_host;
use crate::macros::plugin_call::{set_test_interpreter, shutdown};
use crate::settings::UserSettings;

const ID: &str = "test.complement";
const F_HOKAN: u16 = 31430;

/// What a test sets: the handlers of the two plugs, whether the plugin stays, and the lines of
/// `[Types(0)]`.
struct Setup<'a> {
    global: &'a str,
    typed: &'a str,
    resident: bool,
    types: &'a str,
}

impl Default for Setup<'_> {
    fn default() -> Self {
        Self {
            global: "Complement.AddList(Complement.GetCurrentWord() + '_g');",
            typed: "Complement.AddList(Complement.GetCurrentWord() + '_t'); \
                    Complement.AddList('opt' + Complement.GetOption());",
            resident: true,
            // The plugin of the slot 0 completes, the case is ignored, the words of the document count.
            types: "nHokanType=20000\r\nbHokanLoHiCase=1\r\nbUseHokanByFile=1\r\n",
        }
    }
}

struct Place {
    dir: tempfile::TempDir,
}

impl Place {
    fn new(setup: &Setup) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let plugin = dir.path().join("plugins").join("Comp");
        fs::create_dir_all(&plugin).unwrap();
        let def = format!(
            "[Plugin]\nId={ID}\nName=Comp\nType=wsh\n\n[Plug]\nComplementGlobal=global.js\n\
             Complement=typed.js\nComplement.Label=Typed words\n\n[Wsh]\nUseCache={}\n",
            u8::from(setup.resident)
        );
        fs::write(plugin.join("plugin.def"), def).unwrap();
        fs::write(plugin.join("global.js"), setup.global).unwrap();
        fs::write(plugin.join("typed.js"), setup.typed).unwrap();
        Self { dir }
    }

    fn settings(&self) -> PathBuf {
        self.dir.path().join("matsu.ini")
    }

    fn dirs(&self) -> SearchDirs {
        SearchDirs::new(&self.settings(), None)
    }

    /// The settings of a user who turned plugins on and enabled `Comp`.
    fn user(&self, setup: &Setup, enabled: bool) -> UserSettings {
        let dirs = self.dirs();
        let def = dirs.load_def("Comp", DEFAULT_LANGUAGE).unwrap();
        let mut settings = PluginSettings {
            enabled: true,
            ..PluginSettings::default()
        };
        settings.table[0] = PluginRec {
            id: def.id.clone(),
            name: "Comp".into(),
            cmd_num: 0,
            state: PluginState::Stopped,
        };
        let mut profile = Profile::from_bytes(format!("[Types(0)]\r\n{}", setup.types).as_bytes());
        settings.store(&mut profile).unwrap();
        profile.save(&self.settings()).unwrap();
        if enabled {
            let files = fingerprint(&dirs.plugin_dir("Comp").unwrap()).unwrap();
            let mut consent = ConsentFile::default();
            consent
                .enable(&def.id, "Comp", &files, "2026-11-02")
                .unwrap();
            consent.save(&dirs.consent_file()).unwrap();
        }
        UserSettings::from_profile(profile, Some(self.settings()))
    }
}

/// A window over `text` with the caret at its end, in an application with the plugin set up.
fn window<'a>(
    cx: &'a mut TestAppContext,
    place: &Place,
    setup: &Setup,
    text: &str,
) -> Option<(Entity<EditorView>, &'a mut VisualTestContext)> {
    let host = script_host()?;
    set_test_interpreter(Some(host));
    let user = place.user(setup, true);
    cx.update(gpui_component::init);
    cx.update(|app| app.set_global(user));
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let (view, vcx) = cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    });
    vcx.executor().allow_parking();
    vcx.simulate_keystrokes("ctrl-end");
    Some((view, vcx))
}

fn finish(cx: &mut VisualTestContext) {
    set_test_interpreter(None);
    cx.update(|_, app| shutdown(app, ID));
}

fn hokan(view: &Entity<EditorView>, cx: &mut VisualTestContext) {
    view.update_in(cx, |v, window, cx| v.run_function(F_HOKAN, &[], window, cx));
    cx.run_until_parked();
}

/// The list as the window has it (shown or not), and whether the plugins are still asked.
fn state(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
) -> (Option<(Vec<String>, bool)>, bool) {
    view.read_with(cx, |v, _| {
        (
            v.support
                .completion
                .as_ref()
                .map(|c| (c.items.clone(), c.is_shown())),
            v.support.complement.waiting(),
        )
    })
}

/// Lets the window and the plugin process run until the plugins are not asked any more.
fn wait_done(view: &Entity<EditorView>, cx: &mut VisualTestContext) {
    for _ in 0..3000 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        if !state(view, cx).1 {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "the plugins did not end: {:?}\n{}",
        state(view, cx),
        view.read_with(cx, |v, _| v.macro_output_text())
    );
}

fn words(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

fn text(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |v, app| {
        let shared = v.document().read(app);
        let doc = shared.doc();
        doc.text_between(
            matsu_core::buffer::Pos::new(0, 0),
            matsu_core::buffer::Pos::new(doc.line_count(), 0),
        )
        .unwrap()
    })
}

#[gpui::test]
fn the_candidates_of_both_jacks_are_added_after_the_other_sources(cx: &mut TestAppContext) {
    let setup = Setup::default();
    let place = Place::new(&setup);
    // `foobar` is the word of the document; `fo` is the word before the caret.
    let Some((view, cx)) = window(cx, &place, &setup, "foobar fo") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    hokan(&view, cx);
    // One candidate from the document, and the plugins are asked: nothing is shown yet.
    let (list, waiting) = state(&view, cx);
    assert_eq!(list, Some((words(&["foobar"]), false)));
    assert!(waiting);
    wait_done(&view, cx);
    // ComplementGlobal first, then the Complement of the type; the option is 0x01 | 0x02.
    assert_eq!(
        state(&view, cx).0,
        Some((words(&["foobar", "fo_g", "fo_t", "opt3"]), true))
    );
    finish(cx);
}

#[gpui::test]
fn the_complement_of_a_type_that_did_not_choose_it_is_not_called(cx: &mut TestAppContext) {
    let setup = Setup {
        types: "nHokanType=0\r\n",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "foobar fo") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    hokan(&view, cx);
    wait_done(&view, cx);
    // Only ComplementGlobal, which is for every type.
    assert_eq!(state(&view, cx).0, Some((words(&["foobar", "fo_g"]), true)));
    finish(cx);
}

#[gpui::test]
fn a_plugin_that_does_not_stay_has_no_completion_jack(cx: &mut TestAppContext) {
    let setup = Setup {
        resident: false,
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "foobar fo") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    hokan(&view, cx);
    // No call: the only candidate is inserted at once, as before the jacks.
    assert!(!state(&view, cx).1);
    assert_eq!(state(&view, cx).0, None);
    assert_eq!(text(&view, cx), "foobar foobar");
    // The type settings do not offer it either.
    assert!(cx.update(|_, app| choices(app)).is_empty());
    finish(cx);
}

#[gpui::test]
fn a_candidate_that_comes_after_the_time_limit_is_dropped_and_the_other_sources_stay(
    cx: &mut TestAppContext,
) {
    // The first candidate is added at once; the loop passes the limit of one second.
    let setup = Setup {
        typed: "Complement.AddList('early'); var t = Date.now(); \
                while (Date.now() - t < 5000) {} Complement.AddList('late');",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "apex apple\nap") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let started = std::time::Instant::now();
    hokan(&view, cx);
    wait_done(&view, cx);
    assert!(
        started.elapsed() < Duration::from_secs(4),
        "it was not stopped"
    );
    let (list, _) = state(&view, cx);
    let (items, shown) = list.expect("the list is there");
    assert!(shown);
    assert_eq!(items, words(&["apex", "apple", "ap_g", "early"]));
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(output.contains("時間切れ"), "{output}");
    finish(cx);
}

#[gpui::test]
fn narrowing_uses_the_collected_candidates_and_does_not_call_the_plugin_again(
    cx: &mut TestAppContext,
) {
    let setup = Setup {
        global: "console.log('[call]'); var w = Complement.GetCurrentWord(); \
                 Complement.AddList(w + '_g'); Complement.AddList('foo_x');",
        typed: "",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "foobar fo") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    hokan(&view, cx);
    wait_done(&view, cx);
    assert_eq!(
        state(&view, cx).0,
        Some((words(&["foobar", "fo_g", "foo_x"]), true))
    );
    // A typed char narrows the list to the new word `foo`: `fo_g` goes, and the plugin is not asked.
    cx.simulate_input("o");
    wait_done(&view, cx);
    assert_eq!(
        state(&view, cx).0,
        Some((words(&["foobar", "foo_x"]), true))
    );
    // Backspace goes back to `fo`: the candidates that were collected come back.
    cx.simulate_keystrokes("backspace");
    assert_eq!(
        state(&view, cx).0,
        Some((words(&["foobar", "fo_g", "foo_x"]), true))
    );
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert_eq!(output.matches("[call]").count(), 1, "{output}");
    finish(cx);
}

#[gpui::test]
fn a_list_kept_back_for_the_plugins_follows_the_rules_of_sakura_at_the_end(
    cx: &mut TestAppContext,
) {
    // The plugins add nothing: the one candidate of the document is inserted, as without them.
    let setup = Setup {
        global: "",
        typed: "",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "foobar fo") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    hokan(&view, cx);
    wait_done(&view, cx);
    assert_eq!(state(&view, cx).0, None);
    assert_eq!(text(&view, cx), "foobar foobar");
    finish(cx);
}

#[gpui::test]
fn typing_while_the_plugins_are_asked_drops_what_they_add(cx: &mut TestAppContext) {
    let setup = Setup {
        global: "var t = Date.now(); while (Date.now() - t < 600) {} \
                 Complement.AddList(Complement.GetCurrentWord() + '_slow');",
        typed: "",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "foobar fo") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    hokan(&view, cx);
    // Kept back (one candidate), and the plugin is slow: the user types.
    let (list, waiting) = state(&view, cx);
    assert_eq!(list, Some((words(&["foobar"]), false)));
    assert!(waiting);
    cx.simulate_input("o");
    // The list that was never on the screen is closed and the plugin is not waited for.
    assert_eq!(state(&view, cx), (None, false));
    for _ in 0..80 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(state(&view, cx), (None, false));
    assert_eq!(text(&view, cx), "foobar foo");
    finish(cx);
}

#[test]
fn the_list_for_a_plugin_is_unique_and_counts_like_sakura() {
    // `CHokanMgr::AddKouhoUnique`: the size of the whole list, or -1 when the word is there.
    let mut list = Collector::new(vec!["a".into(), "b".into()]);
    assert_eq!(list.add("c"), 3);
    assert_eq!(list.add("b"), -1);
    assert_eq!(list.add("c"), -1);
    assert_eq!(list.add("d"), 4);
    assert_eq!(list.take_new(), ["c", "d"]);
    assert!(list.take_new().is_empty());
}

#[test]
fn a_plugin_cannot_fill_the_list_without_end() {
    let mut list = Collector::new(vec![]);
    for i in 0..MAX_ADDED {
        assert!(list.add(&format!("w{i}")) > 0);
    }
    assert_eq!(list.add("one more"), -1);
}

#[test]
fn the_stored_number_names_a_plugin_by_the_code_of_its_plug_0() {
    assert_eq!(chosen_slot(20_000), Some(0));
    assert_eq!(chosen_slot(20_100), Some(1));
    assert_eq!(chosen_slot(20_050), Some(20));
    // A command of a plugin (plug 1) is not a completion; neither is 0 nor another number.
    assert_eq!(chosen_slot(20_001), None);
    assert_eq!(chosen_slot(0), None);
    assert_eq!(chosen_slot(-1), None);
    assert_eq!(chosen_slot(5), None);
}

#[gpui::test]
fn the_choice_lists_the_enabled_plugins_that_complete_by_the_label_of_the_plug(
    cx: &mut TestAppContext,
) {
    let setup = Setup::default();
    let place = Place::new(&setup);
    let enabled = place.user(&setup, true);
    cx.update(|app| app.set_global(enabled));
    assert_eq!(
        cx.update(|app| choices(app)),
        [("Typed words".to_owned(), 20_000)]
    );
    // The user has not agreed to the files (or took the agreement back): the plugin is not listed.
    let _ = fs::remove_file(place.dirs().consent_file());
    assert!(cx.update(|app| choices(app)).is_empty());
}

#[test]
fn a_sample_plugin_def_with_japanese_text_starts_with_the_bom() {
    // A `plugin.def` without a BOM is read as the ANSI code page (as Sakura does), so Japanese in it
    // would be garbled on Windows.
    let samples =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../matsu-plugin/samples/plugins");
    let mut checked = 0;
    for entry in fs::read_dir(samples).unwrap().flatten() {
        let def = entry.path().join("plugin.def");
        let Ok(bytes) = fs::read(&def) else { continue };
        checked += 1;
        if !bytes.is_ascii() {
            assert!(
                bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
                "{} has Japanese text and no BOM",
                def.display()
            );
        }
    }
    assert!(checked >= 4, "{checked} samples");
}

#[gpui::test]
fn the_choices_are_made_once_and_not_at_every_draw(cx: &mut TestAppContext) {
    let setup = Setup::default();
    let place = Place::new(&setup);
    let user = place.user(&setup, true);
    cx.update(|app| app.set_global(user));
    forget_choices();
    assert_eq!(cx.update(|app| choices(app)).len(), 1);
    // A file of the plugin changes: the agreement would no longer hold, which only hashing the files
    // can tell. The draws do not hash, so the list stays until it is made again.
    let handler = place.dir.path().join("plugins/Comp/typed.js");
    fs::write(&handler, "// changed").unwrap();
    assert_eq!(cx.update(|app| choices(app)).len(), 1);
    forget_choices();
    assert!(cx.update(|app| choices(app)).is_empty());
}

#[test]
fn the_calls_of_the_complement_object_are_told_apart_and_answered_without_the_editor() {
    use matsu_plugin::plugin_host::PluginApi;
    struct Api(Vec<String>);
    impl PluginApi for Api {
        fn call(
            &mut self,
            _: &str,
            params: &[matsu_plugin::Value],
        ) -> Result<matsu_plugin::Value, matsu_plugin::protocol::RpcError> {
            self.0.push(params[0].as_str().unwrap().to_owned());
            Ok(matsu_plugin::json!(self.0.len()))
        }
    }
    use crate::macros::plugin_call::{answer_plugin_object_call, is_plugin_object_call};
    let add = r#"{"jsonrpc":"2.0","id":1,"method":"Complement.AddList","params":["w"]}"#;
    let insert = r#"{"jsonrpc":"2.0","id":2,"method":"InsText","params":["w"]}"#;
    // The `Plugin` object (the definition and the options) is the plugin's own too.
    let option = r#"{"jsonrpc":"2.0","id":3,"method":"Plugin.GetOption","params":["S"]}"#;
    assert!(is_plugin_object_call(option));
    assert!(is_plugin_object_call(add));
    assert!(!is_plugin_object_call(insert));
    let mut api = Api(Vec::new());
    let line = answer_plugin_object_call(add, &mut api).unwrap();
    assert!(line.contains("\"result\":1"), "{line}");
    // Any other call is refused here: it would need the window.
    let line = answer_plugin_object_call(insert, &mut api).unwrap();
    assert!(line.contains("error"), "{line}");
}

#[gpui::test]
fn a_composition_goes_on_while_a_completion_handler_reads_its_options(cx: &mut TestAppContext) {
    // The handler waits a little, then asks for an option of the plugin (`Plugin.GetOption`) and adds a
    // candidate: neither may commit the composition the user starts meanwhile.
    let setup = Setup {
        global: "var t = Date.now(); while (Date.now() - t < 300) {} \
                 Plugin.GetOption('S', 'K'); Complement.AddList('late');",
        typed: "",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "apex apple\nap") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    hokan(&view, cx);
    view.update_in(cx, |v, window, cx| {
        v.replace_and_mark_text_in_range(None, "に", Some(1..1), window, cx);
    });
    wait_done(&view, cx);
    let marked = view.update_in(cx, |v, window, cx| v.marked_text_range(window, cx));
    assert!(marked.is_some(), "the composition was committed");
    finish(cx);
}

#[gpui::test]
fn an_undo_block_that_a_completion_handler_left_open_is_closed_when_it_ends(
    cx: &mut TestAppContext,
) {
    // The handler opens a block of the undo buffer and throws before it closes it.
    let setup = Setup {
        global: "Editor.AddRefUndoBuffer(); throw new Error('boom');",
        typed: "",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "apex apple\nap") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let depth = |view: &Entity<EditorView>, cx: &mut VisualTestContext| {
        view.read_with(cx, |v, app| {
            v.document().read(app).model().history().depth()
        })
    };
    let before = depth(&view, cx);
    hokan(&view, cx);
    wait_done(&view, cx);
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(output.contains("boom"), "{output}");
    assert_eq!(depth(&view, cx), before, "the block stayed open");
    // The edit of the user is a step of its own.
    cx.simulate_input("x");
    cx.simulate_keystrokes("ctrl-z");
    assert_eq!(text(&view, cx), "apex apple\nap");
    finish(cx);
}

#[gpui::test]
fn a_plugin_that_is_busy_for_another_window_is_left_out_without_a_word(cx: &mut TestAppContext) {
    // The first window's call keeps the host of the plugin busy for a while.
    let setup = Setup {
        global: "var t = Date.now(); while (Date.now() - t < 600) {} \
                 Complement.AddList(Complement.GetCurrentWord() + '_slow');",
        typed: "",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((first, cx)) = window(cx, &place, &setup, "apex apple\nap") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    let doc = Document::from_utf8(b"apex apple\nap".to_vec()).unwrap();
    let shared = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let second = cx.new(|cx| EditorView::new(shared, cx));
    let to_end = matsu_commands::sakura::FUNCTIONS
        .iter()
        .find(|f| f.name == "F_GOFILEEND")
        .unwrap()
        .code;
    second.update_in(cx, |v, window, cx| v.run_function(to_end, &[], window, cx));
    hokan(&first, cx);
    hokan(&second, cx);
    wait_done(&second, cx);
    // The other sources show, and nothing was said about the busy plugin.
    assert_eq!(
        state(&second, cx).0,
        Some((words(&["apex", "apple"]), true))
    );
    let output = second.read_with(cx, |v, _| v.macro_output_text());
    assert!(output.trim().is_empty(), "{output}");
    wait_done(&first, cx);
    finish(cx);
}

#[gpui::test]
fn a_list_the_user_closed_does_not_start_the_calls_that_are_left(cx: &mut TestAppContext) {
    // Two candidates of the document: the list is on the screen at once. The first call is slow.
    let setup = Setup {
        global: "var t = Date.now(); while (Date.now() - t < 500) {} console.log('[global]');",
        typed: "console.log('[typed]');",
        ..Setup::default()
    };
    let place = Place::new(&setup);
    let Some((view, cx)) = window(cx, &place, &setup, "apex apple\nap") else {
        eprintln!("SKIP: matsu-script is not built next to the tests");
        return;
    };
    hokan(&view, cx);
    assert_eq!(state(&view, cx).0.map(|(_, shown)| shown), Some(true));
    cx.simulate_keystrokes("escape");
    assert_eq!(state(&view, cx).0, None);
    for _ in 0..100 {
        cx.executor().advance_clock(Duration::from_millis(20));
        cx.run_until_parked();
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = view.read_with(cx, |v, _| v.macro_output_text());
    assert!(output.contains("[global]"), "{output}");
    assert!(!output.contains("[typed]"), "{output}");
    finish(cx);
}
