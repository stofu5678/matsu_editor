//! The buttons of the type list that change the list, with a real settings file in a private
//! temporary folder, in GPUI's headless test windows.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use gpui::{AnyWindowHandle, Entity, TestAppContext, VisualTestContext};
use matsu_commands::command::Arg;
use matsu_config::Profile;
use matsu_config::history::HistoryStore;
use matsu_config::settings::DocumentTypes;

use super::list::TypeList;
use super::tests::{locked, run, type_index, view_of};
use crate::document::SharedDocument;
use crate::file_ops::windows::open_editor_window;
use crate::history::{HistoryOptions, HistoryState};
use crate::settings::UserSettings;
use crate::view::EditorView;

/// A folder that is removed at the end of the test.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("matsu-ui-type-list-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The settings file of a test.
pub(super) struct Ini {
    dir: TempDir,
    path: PathBuf,
}

impl Ini {
    pub(super) fn profile(&self) -> Profile {
        Profile::load(&self.path).unwrap()
    }

    pub(super) fn get(&self, section: &str, key: &str) -> Option<String> {
        self.profile().get(section, key).map(str::to_owned)
    }

    pub(super) fn folder(&self) -> &Path {
        &self.dir.0
    }
}

/// A settings file with `text`, read as `app::run` reads it, and an editor window.
pub(super) fn setup(cx: &mut TestAppContext, text: &str) -> (Ini, AnyWindowHandle) {
    cx.update(gpui_component::init);
    let dir = TempDir::new();
    let path = dir.0.join("matsu.ini");
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    std::fs::write(&path, bytes).unwrap();
    cx.update(|cx| {
        let user = UserSettings::load_from(std::slice::from_ref(&path));
        cx.set_global(HistoryState::on_disk(
            HistoryStore::new(path.clone(), false),
            HistoryOptions::default(),
        ));
        cx.set_global(user);
    });
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    (Ini { dir, path }, handle)
}

/// Opens the list over the view.
pub(super) fn open_list(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Entity<TypeList> {
    run(view, cx, "F_TYPE_LIST", &[]);
    view.read_with(cx, |v, _| v.doc_type.list.clone().expect("the list"))
}

pub(super) fn select(list: &Entity<TypeList>, cx: &mut VisualTestContext, index: usize) {
    list.update(cx, |l, _| l.selected = index);
}

pub(super) fn selected(list: &Entity<TypeList>, cx: &mut VisualTestContext) -> usize {
    list.read_with(cx, |l, _| l.selected)
}

pub(super) fn labels(list: &Entity<TypeList>, cx: &mut VisualTestContext) -> Vec<String> {
    list.read_with(cx, |l, _| l.labels.clone())
}

pub(super) fn types(ini: &Ini) -> DocumentTypes {
    DocumentTypes::from_profile(&ini.profile())
}

/// `n` types, the ones after the 17 built-in ones the user's own.
fn own_types(n: usize) -> String {
    let mut text = format!("[Other]\r\nnTypesCount={n}\r\n");
    for i in 17..n {
        text.push_str(&format!(
            "[Types({i})]\r\nszTypeName=自分{i}\r\nszTypeExts=x{i}\r\n"
        ));
    }
    text
}

#[gpui::test]
fn a_type_is_added_at_the_end_and_selected(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    assert_eq!(labels(&list, cx).len(), 20);
    list.update_in(cx, |l, window, lcx| l.add(window, lcx));
    cx.run_until_parked();
    let shown = labels(&list, cx);
    assert_eq!(shown.len(), 21);
    assert_eq!(shown[20], "設定21");
    assert_eq!(selected(&list, cx), 20);
    // The settings file has the type, and the count.
    assert_eq!(ini.get("Other", "nTypesCount").as_deref(), Some("21"));
    assert_eq!(
        ini.get("Types(20)", "szTypeName").as_deref(),
        Some("設定21")
    );
    // The application has it too: the list is open yet.
    assert!(view.read_with(cx, |v, _| v.doc_type.list.is_some()));
    assert_eq!(view.read_with(cx, |v, app| v.type_labels(app).0.len()), 21);
}

#[gpui::test]
fn a_type_is_copied_with_a_numbered_name(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    select(&list, cx, 18);
    list.update_in(cx, |l, window, lcx| l.copy(window, lcx));
    cx.run_until_parked();
    let shown = labels(&list, cx);
    assert_eq!(shown.len(), 21);
    assert_eq!(shown[20], "自分20 ( x18 )");
    assert_eq!(selected(&list, cx), 20);
    assert_eq!(types(&ini).get(20).unwrap().type_exts, "x18");
}

#[gpui::test]
fn the_buttons_follow_the_rules_of_the_selected_type(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    let rules = |list: &Entity<TypeList>, cx: &mut VisualTestContext, index: usize| {
        select(list, cx, index);
        list.read_with(cx, |l, _| {
            (
                l.can_move_up(),
                l.can_move_down(),
                l.can_delete(),
                l.has_room(),
            )
        })
    };
    // 基本: nothing but 追加 and 複製.
    assert_eq!(rules(&list, cx, 0), (false, false, false, true));
    // The second type cannot go up (it would pass 基本).
    assert_eq!(rules(&list, cx, 1), (false, true, true, true));
    assert_eq!(rules(&list, cx, 2), (true, true, true, true));
    assert_eq!(rules(&list, cx, 19), (true, false, true, true));
    // The buttons that are off do nothing.
    select(&list, cx, 0);
    list.update_in(cx, |l, window, lcx| {
        l.move_selected(true, window, lcx);
        l.move_selected(false, window, lcx);
        l.delete(window, lcx);
    });
    cx.run_until_parked();
    assert!(!cx.has_pending_prompt());
    assert_eq!(labels(&list, cx).len(), 20);
}

#[gpui::test]
fn there_are_at_most_60_types(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &own_types(60));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    assert!(!list.read_with(cx, |l, _| l.has_room()));
    list.update_in(cx, |l, window, lcx| {
        l.add(window, lcx);
        l.copy(window, lcx);
    });
    cx.run_until_parked();
    assert_eq!(labels(&list, cx).len(), 60);
    assert_eq!(ini.get("Other", "nTypesCount").as_deref(), Some("60"));
}

#[gpui::test]
fn a_type_moves_up_and_down_and_stays_selected(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    select(&list, cx, 18);
    list.update_in(cx, |l, window, lcx| l.move_selected(true, window, lcx));
    cx.run_until_parked();
    assert_eq!(selected(&list, cx), 17);
    assert_eq!(labels(&list, cx)[17], "自分18 ( x18 )");
    assert_eq!(types(&ini).get(17).unwrap().type_name, "自分18");
    list.update_in(cx, |l, window, lcx| l.move_selected(false, window, lcx));
    cx.run_until_parked();
    assert_eq!(selected(&list, cx), 18);
    assert_eq!(types(&ini).get(18).unwrap().type_name, "自分18");
}

#[gpui::test]
fn deleting_asks_first_and_selects_the_next_type(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    select(&list, cx, 18);
    list.update_in(cx, |l, window, lcx| l.delete(window, lcx));
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    // いいえ: nothing happens.
    cx.simulate_prompt_answer("いいえ(N)");
    cx.run_until_parked();
    assert_eq!(labels(&list, cx).len(), 20);
    list.update_in(cx, |l, window, lcx| l.delete(window, lcx));
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    let shown = labels(&list, cx);
    assert_eq!(shown.len(), 19);
    assert_eq!(shown[18], "自分19 ( x19 )");
    assert_eq!(selected(&list, cx), 18);
    assert_eq!(ini.get("Other", "nTypesCount").as_deref(), Some("19"));
    // Deleting the last type selects the new last one.
    select(&list, cx, 18);
    list.update_in(cx, |l, window, lcx| l.delete(window, lcx));
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert_eq!(selected(&list, cx), 17);
}

#[gpui::test]
fn a_type_is_reset_after_a_question_when_it_has_extensions(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    select(&list, cx, 18);
    list.update_in(cx, |l, window, lcx| l.reset(window, lcx));
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert_eq!(labels(&list, cx)[18], "設定19");
    assert_eq!(types(&ini).get(18).unwrap().type_exts, "");
    assert_eq!(selected(&list, cx), 18);
    // The report of the reset (an information, OK).
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    // A type without extensions is reset without a question.
    list.update_in(cx, |l, window, lcx| l.reset(window, lcx));
    cx.run_until_parked();
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("OK");
    assert!(!cx.has_pending_prompt());
}

#[gpui::test]
fn the_basic_type_is_reset_to_the_built_in_settings_after_a_question(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Types(0)]\r\nnInts=0,90,0,6,-1,-1,0,0,0,0,0,0\r\n");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    assert_eq!(types(&ini).get(0).unwrap().tab_space, 6);
    list.update_in(cx, |l, window, lcx| l.reset(window, lcx));
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert_eq!(
        types(&ini).get(0).unwrap().tab_space,
        DocumentTypes::builtin().get(0).unwrap().tab_space
    );
    // The window of the document has the settings again.
    assert_eq!(super::tests::tab_width(&view, cx), 4);
}

#[gpui::test]
fn a_document_keeps_its_chosen_type_when_the_types_move(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    // The document has type 18 by the user's choice (the argument is the number plus one).
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(19)]);
    assert_eq!((type_index(&view, cx), locked(&view, cx)), (18, true));
    let list = open_list(&view, cx);
    select(&list, cx, 18);
    list.update_in(cx, |l, window, lcx| l.move_selected(true, window, lcx));
    cx.run_until_parked();
    // The type moved to number 17: the document follows it (`GetDocumentTypeOfId`).
    assert_eq!(type_index(&view, cx), 17);
    assert!(locked(&view, cx));
    // A type before it is deleted: it moves up by one.
    select(&list, cx, 3);
    list.update_in(cx, |l, window, lcx| l.delete(window, lcx));
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert_eq!(type_index(&view, cx), 16);
    // Its own type is deleted: the type of the file name (an untitled document has the basic type).
    select(&list, cx, 16);
    list.update_in(cx, |l, window, lcx| l.delete(window, lcx));
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert_eq!(type_index(&view, cx), 0);
}

#[gpui::test]
fn the_zoom_stays_when_the_type_of_the_document_only_changes_its_number(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, &own_types(20));
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    run(&view, cx, "F_CHANGETYPE", &[Arg::Int(19)]);
    run(&view, cx, "F_SETFONTSIZEUP", &[Arg::Int(1)]);
    let px = view.read_with(cx, |v, _| v.font_px());
    let list = open_list(&view, cx);
    select(&list, cx, 18);
    list.update_in(cx, |l, window, lcx| l.move_selected(true, window, lcx));
    cx.run_until_parked();
    assert_eq!(type_index(&view, cx), 17);
    assert_eq!(view.read_with(cx, |v, _| v.font_px()), px);
}

#[gpui::test]
fn a_document_that_has_the_type_of_its_name_takes_the_type_of_its_name_again(
    cx: &mut TestAppContext,
) {
    let (ini, handle) = setup(cx, "");
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let file = ini.folder().join("a.cpp");
    std::fs::write(&file, b"x").unwrap();
    run(
        &view,
        cx,
        "F_FILEOPEN",
        &[Arg::Text(file.display().to_string())],
    );
    assert_eq!((type_index(&view, cx), locked(&view, cx)), (2, false));
    let list = open_list(&view, cx);
    // HTML (3) is deleted: C/C++ stays at 2. Then C/C++ goes up... it cannot pass number 1; delete
    // the type before it (テキスト, 1): C/C++ becomes number 1.
    select(&list, cx, 1);
    list.update_in(cx, |l, window, lcx| l.delete(window, lcx));
    cx.run_until_parked();
    cx.simulate_prompt_answer("はい(Y)");
    cx.run_until_parked();
    assert_eq!(type_index(&view, cx), 1);
    assert!(!locked(&view, cx));
}

#[gpui::test]
fn another_open_list_shows_the_change_and_keeps_its_type_selected(cx: &mut TestAppContext) {
    let (_ini, first) = setup(cx, &own_types(20));
    let second = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    cx.run_until_parked();
    let view_of_window = |cx: &mut TestAppContext, handle: AnyWindowHandle| {
        cx.update(|app| crate::file_ops::windows::workspace_of(app, handle))
            .unwrap()
            .read_with(cx, |ws, _| ws.editor().clone())
    };
    let (view_a, view_b) = (view_of_window(cx, first), view_of_window(cx, second));
    let cx_a = &mut VisualTestContext::from_window(first, cx);
    let list_a = open_list(&view_a, cx_a);
    let cx_b = &mut VisualTestContext::from_window(second, cx);
    let list_b = open_list(&view_b, cx_b);
    select(&list_b, cx_b, 19);
    // The first window deletes type 5 (its id is 5).
    let cx_a = &mut VisualTestContext::from_window(first, cx);
    select(&list_a, cx_a, 5);
    list_a.update_in(cx_a, |l, window, lcx| l.delete_now(5, window, lcx));
    cx_a.run_until_parked();
    let cx_b = &mut VisualTestContext::from_window(second, cx);
    // Type 19 is number 18 now, and still selected in the other list.
    assert_eq!(selected(&list_b, cx_b), 18);
    assert_eq!(labels(&list_b, cx_b).len(), 19);
    assert_eq!(labels(&list_b, cx_b)[18], "自分19 ( x19 )");
}

#[gpui::test]
fn without_a_settings_file_the_list_changes_for_the_session(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    cx.update(|cx| cx.set_global(UserSettings::none()));
    let handle = cx
        .update(|cx| open_editor_window(cx, SharedDocument::empty(), None))
        .unwrap();
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    list.update_in(cx, |l, window, lcx| l.add(window, lcx));
    cx.run_until_parked();
    assert_eq!(labels(&list, cx).len(), 37);
    assert_eq!(view.read_with(cx, |v, app| v.type_labels(app).0.len()), 37);
}

#[gpui::test]
fn the_outline_settings_of_a_type_follow_it_when_the_types_move(cx: &mut TestAppContext) {
    use crate::outline::dock::{DockSide, OutlineDock};
    let mut text = own_types(20);
    text.push_str("");
    // type 18 has its own place and sort in the file
    let text = text.replace(
        "[Types(18)]\r\n",
        "[Types(18)]\r\neOutlineDockSide=4\r\nnOutlineSortType=3\r\n",
    );
    let (ini, handle) = setup(cx, &text);
    cx.update(|cx| {
        let user = cx.global::<UserSettings>().clone();
        crate::saved_state::install(cx, &user);
    });
    let cx = &mut VisualTestContext::from_window(handle, cx);
    let view = view_of(cx);
    let list = open_list(&view, cx);
    select(&list, cx, 18);
    list.update_in(cx, |l, window, lcx| l.move_selected(true, window, lcx));
    cx.run_until_parked();
    assert_eq!(
        ini.get("Types(17)", "eOutlineDockSide").as_deref(),
        Some("4")
    );
    let dock = cx.update(|_, cx| cx.global::<OutlineDock>().clone());
    assert_eq!(dock.type_outline(17).place.side, DockSide::Bottom);
    assert_eq!(dock.type_outline(17).sort_type, 3);
    // and the type that took number 18 has nothing of it
    assert_eq!(dock.type_outline(18).place.side, DockSide::Right);
    assert_eq!(dock.type_outline(18).sort_type, 0);
}
