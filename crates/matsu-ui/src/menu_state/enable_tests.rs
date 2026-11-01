//! Tests of the `IsFuncEnable` rules beyond undo and the selection: view mode, the search, the
//! file path, the jump history, the tabs and the key macro. Each group has both states.

use gpui::TestAppContext;

use super::FuncStates;
use super::tests::{base, code, setup};

fn enabled(states: &FuncStates, name: &str) -> bool {
    states.is_enabled(code(name))
}

/// Both states of a function: gray in `off`, available in `on`.
fn flips(name: &str, off: &FuncStates, on: &FuncStates) {
    assert!(!enabled(off, name), "{name} should be gray");
    assert!(enabled(on, name), "{name} should be available");
}

#[test]
fn view_mode_grays_the_functions_that_edit() {
    let editable = FuncStates {
        can_undo: true,
        can_redo: true,
        text_selected: true,
        has_search_keys: true,
        ..base()
    };
    let view = FuncStates {
        view_mode: true,
        ..editable.clone()
    };
    for name in [
        "F_UNDO",
        "F_REDO",
        "F_DELETE",
        "F_WordDeleteToStart",
        "F_LineCutToEnd",
        "F_DUPLICATELINE",
        "F_INDENT_TAB",
        "F_SORT_ASC",
        "F_CUT",
        "F_PASTE",
        "F_INSTEXT_W",
        "F_INS_DATE",
        "F_TOLOWER",
        "F_CODECNV_SJIS2UTF8",
        "F_REPLACE_DIALOG",
        "F_REPLACE",
        "F_REPLACE_ALL",
        "F_CHGMOD_INS",
        "F_HOKAN",
    ] {
        assert!(!enabled(&view, name), "{name} in view mode");
    }
    // what only reads stays available
    for name in [
        "F_COPY",
        "F_COPYLINES",
        "F_VIEWMODE",
        "F_FILEOPEN",
        "F_SEARCH_NEXT",
    ] {
        assert!(enabled(&view, name), "{name}");
    }
    assert!(enabled(&editable, "F_UNDO") && enabled(&editable, "F_REPLACE"));
    // every name of the list is a function of Sakura's table
    for name in super::MODIFICATION_FORBIDDEN {
        code(name);
    }
}

#[test]
fn the_search_functions_need_a_search_string() {
    let some = FuncStates {
        has_search_keys: true,
        ..base()
    };
    for name in [
        "F_SEARCH_NEXT",
        "F_SEARCH_PREV",
        "F_SEARCH_CLEARMARK",
        "F_REPLACE",
        "F_REPLACE_ALL",
    ] {
        flips(name, &base(), &some);
    }
    let started = FuncStates {
        has_search_start: true,
        ..base()
    };
    flips("F_JUMP_SRCHSTARTPOS", &base(), &started);
}

#[test]
fn compare_needs_two_documents_and_the_diff_functions_a_diff() {
    let two = FuncStates {
        open_documents: 2,
        ..base()
    };
    flips("F_COMPARE", &base(), &two);
    let diff = FuncStates {
        diff_shown: true,
        ..base()
    };
    for name in ["F_DIFF_NEXT", "F_DIFF_PREV", "F_DIFF_RESET"] {
        flips(name, &base(), &diff);
    }
    assert!(enabled(&base(), "F_DIFF_DIALOG"));
}

#[test]
fn saving_needs_a_modified_document_outside_view_mode() {
    let modified = FuncStates {
        modified: true,
        ..base()
    };
    flips("F_FILESAVE", &base(), &modified);
    // a changed encoding or BOM (`IsChgCodeSet`) makes an unmodified document saveable too
    let recoded = FuncStates {
        format_changed: true,
        ..base()
    };
    flips("F_FILESAVE", &base(), &recoded);
    let view = FuncStates {
        view_mode: true,
        ..modified.clone()
    };
    assert!(!enabled(&view, "F_FILESAVE"));
    // a named file that is not on disk is created by a save, unmodified or not
    let dir = tempfile::tempdir().unwrap();
    let missing = FuncStates {
        has_path: true,
        path: Some(dir.path().join("new.txt")),
        ..base()
    };
    assert!(enabled(&missing, "F_FILESAVE"));
    std::fs::write(dir.path().join("new.txt"), "").unwrap();
    super::disk::forget();
    assert!(!enabled(&missing, "F_FILESAVE"));
    assert!(enabled(&base(), "F_FILESAVEAS_DIALOG"));
}

#[test]
fn the_functions_of_the_file_path_need_a_file() {
    let file = FuncStates {
        has_path: true,
        ..base()
    };
    for name in [
        "F_COPYPATH",
        "F_COPYDIRPATH",
        "F_COPYTAG",
        "F_COPYFNAME",
        "F_BROWSE",
        "F_OPEN_FOLDER_IN_EXPLORER",
        "F_PLSQL_COMPILE_ON_SQLPLUS",
        "F_PLSQL_COMPILE_ON_DATABASE_CHOICE",
        "F_OPEN_COMMAND_PROMPT",
        "F_OPEN_COMMAND_PROMPT_AS_ADMIN",
        "F_OPEN_POWERSHELL",
        "F_OPEN_POWERSHELL_AS_ADMIN",
    ] {
        flips(name, &base(), &file);
    }
    // a UNC path: the command prompt is gray, PowerShell is not
    let unc = FuncStates {
        unc_path: true,
        ..file.clone()
    };
    for name in ["F_OPEN_COMMAND_PROMPT", "F_OPEN_COMMAND_PROMPT_AS_ADMIN"] {
        assert!(!enabled(&unc, name), "{name}");
    }
    assert!(enabled(&unc, "F_OPEN_POWERSHELL"));
    // the header or source of the same name, looked for on the disk
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.c", "a.h", "b.c"] {
        std::fs::write(dir.path().join(name), "").unwrap();
    }
    let with = |name: &str| FuncStates {
        path: Some(dir.path().join(name)),
        ..file.clone()
    };
    flips("F_OPEN_HfromtoC", &with("b.c"), &with("a.c"));
    assert!(enabled(&with("a.h"), "F_OPEN_HfromtoC"));
    assert!(!enabled(&base(), "F_OPEN_HfromtoC"));
}

#[test]
fn the_jump_history_buttons_follow_its_ends() {
    let back = FuncStates {
        can_jump_back: true,
        ..base()
    };
    let forward = FuncStates {
        can_jump_forward: true,
        ..base()
    };
    flips("F_JUMPHIST_PREV", &base(), &back);
    flips("F_JUMPHIST_NEXT", &base(), &forward);
    assert!(!enabled(&back, "F_JUMPHIST_NEXT"));
    assert!(enabled(&base(), "F_JUMPHIST_SET"));
}

#[test]
fn a_direct_tag_jump_needs_a_file_and_no_grep_window() {
    let file = FuncStates {
        has_path: true,
        ..base()
    };
    let grep = FuncStates {
        grep_window: true,
        ..file.clone()
    };
    for name in ["F_DIRECT_TAGJUMP", "F_TAGJUMP_KEYWORD"] {
        flips(name, &base(), &file);
        assert!(!enabled(&grep, name), "{name} in a Grep window");
    }
}

#[test]
fn the_tab_functions_follow_the_tab_mode() {
    let none = base();
    // the bar is shown and the grouping is off: the case that made the user ask for the marks
    let bar_only = FuncStates {
        tab_bar: true,
        grouped: false,
        ..base()
    };
    let tab_mode = FuncStates {
        tab_bar: true,
        grouped: true,
        ..base()
    };
    for name in [
        "F_BIND_WINDOW",
        "F_TAB_MOVERIGHT",
        "F_TAB_MOVELEFT",
        "F_TAB_CLOSELEFT",
        "F_TAB_CLOSERIGHT",
    ] {
        assert!(!enabled(&none, name), "{name} without the bar");
        assert!(enabled(&bar_only, name), "{name} with the bar");
        assert!(enabled(&tab_mode, name), "{name} in tab mode");
    }
    for name in [
        "F_GROUPCLOSE",
        "F_NEXTGROUP",
        "F_PREVGROUP",
        "F_TAB_SEPARATE",
        "F_TAB_JOINTNEXT",
        "F_TAB_JOINTPREV",
        "F_FILENEW_NEWWINDOW",
    ] {
        assert!(!enabled(&none, name), "{name} without the bar");
        assert!(
            !enabled(&bar_only, name),
            "{name} with the bar, not grouped"
        );
        assert!(enabled(&tab_mode, name), "{name} in tab mode");
    }
}

#[test]
fn the_key_macro_functions_follow_the_recording_and_the_macro() {
    // nothing recorded or loaded
    assert!(enabled(&base(), "F_RECKEYMACRO"));
    assert!(enabled(&base(), "F_LOADKEYMACRO"));
    assert!(!enabled(&base(), "F_SAVEKEYMACRO"));
    assert!(!enabled(&base(), "F_EXECKEYMACRO"));
    // a key macro can be saved and played
    let loaded = FuncStates {
        key_macro_saveable: true,
        key_macro_loaded: true,
        ..base()
    };
    assert!(enabled(&loaded, "F_SAVEKEYMACRO") && enabled(&loaded, "F_EXECKEYMACRO"));
    // a script is played but cannot be saved
    let script = FuncStates {
        key_macro_loaded: true,
        ..base()
    };
    assert!(!enabled(&script, "F_SAVEKEYMACRO") && enabled(&script, "F_EXECKEYMACRO"));
    // while this window records, all four are available
    let recording = FuncStates {
        recording_key_macro: true,
        ..base()
    };
    for name in [
        "F_RECKEYMACRO",
        "F_LOADKEYMACRO",
        "F_SAVEKEYMACRO",
        "F_EXECKEYMACRO",
    ] {
        assert!(enabled(&recording, name), "{name}");
    }
}

// ---- the state read from real windows ----

#[gpui::test]
fn a_window_without_a_file_has_gray_file_functions(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    let states = f.states(cx);
    assert!(!states.has_path && !states.modified);
    assert!(!states.has_search_start && !states.diff_shown);
    assert!(!states.can_jump_back && !states.can_jump_forward && !states.grep_window);
    assert!(!states.key_macro_saveable && !states.key_macro_loaded);
    assert!(!states.is_enabled(code("F_COPYPATH")));
    assert!(!states.is_enabled(code("F_FILESAVE")));
    // typing makes the document modified; view mode then forbids the edit functions
    cx.simulate_input("x");
    assert!(f.states(cx).is_enabled(code("F_FILESAVE")));
    f.run(cx, "F_VIEWMODE");
    let states = f.states(cx);
    assert!(!states.is_enabled(code("F_FILESAVE")));
    assert!(!states.is_enabled(code("F_UNDO")));
    assert!(!states.is_enabled(code("F_PASTE")));
}

#[gpui::test]
fn a_recorded_key_macro_can_be_played_and_saved(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, "F_RECKEYMACRO");
    cx.simulate_input("x");
    f.run(cx, "F_RECKEYMACRO");
    let states = f.states(cx);
    assert!(states.key_macro_saveable && states.key_macro_loaded);
    assert!(states.is_enabled(code("F_SAVEKEYMACRO")));
    assert!(states.is_enabled(code("F_EXECKEYMACRO")));
}

#[gpui::test]
fn the_search_functions_follow_the_search_history_of_the_application(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    assert!(!f.states(cx).is_enabled(code("F_SEARCH_NEXT")));
    cx.update(|_, app| {
        crate::search::history::install(
            app,
            crate::search::history::SearchHistory::load(None, None),
        );
        crate::search::history::remember_search(app, "abc");
    });
    let states = f.states(cx);
    assert!(states.has_search_keys);
    assert!(states.is_enabled(code("F_SEARCH_NEXT")));
}

#[gpui::test]
fn the_jump_history_shows_in_the_state(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef\nghi");
    f.run(cx, "F_JUMPHIST_SET");
    cx.simulate_keystrokes("down down");
    f.run(cx, "F_JUMPHIST_SET");
    f.run(cx, "F_JUMPHIST_PREV");
    let states = f.states(cx);
    assert!(states.can_jump_forward);
    assert!(states.is_enabled(code("F_JUMPHIST_NEXT")));
}

// ---- the toolbar and the function key bar ----

#[test]
fn the_bars_are_checked_while_shown() {
    let shown = FuncStates {
        toolbar: true,
        funckeys: true,
        ..base()
    };
    let hidden = FuncStates {
        toolbar: false,
        funckeys: false,
        ..base()
    };
    for name in ["F_SHOWTOOLBAR", "F_SHOWFUNCKEY"] {
        assert!(shown.is_checked(code(name)), "{name} shown");
        assert!(!hidden.is_checked(code(name)), "{name} hidden");
        // the items are always available
        assert!(enabled(&hidden, name));
    }
    // each bar has its own mark
    let toolbar_only = FuncStates {
        toolbar: true,
        funckeys: false,
        ..base()
    };
    assert!(toolbar_only.is_checked(code("F_SHOWTOOLBAR")));
    assert!(!toolbar_only.is_checked(code("F_SHOWFUNCKEY")));
}

#[test]
fn the_minimap_is_checked_while_shown() {
    // `IsFuncChecked`: `GetMiniMap().GetHwnd() != nullptr`; the item is always available
    let shown = FuncStates {
        minimap: true,
        ..base()
    };
    assert!(shown.is_checked(code("F_SHOWMINIMAP")));
    assert!(!base().is_checked(code("F_SHOWMINIMAP")));
    assert!(enabled(&base(), "F_SHOWMINIMAP"));
    use crate::bars::toolbar::look;
    assert!(look(code("F_SHOWMINIMAP"), &shown).pressed);
    assert!(!look(code("F_SHOWMINIMAP"), &base()).pressed);
}

#[test]
fn saving_an_unmodified_file_follows_the_setting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.txt");
    std::fs::write(&path, "").unwrap();
    let on_disk = FuncStates {
        has_path: true,
        path: Some(path),
        ..base()
    };
    let overwrite = FuncStates {
        unmodified_overwrite: true,
        ..on_disk.clone()
    };
    flips("F_FILESAVE", &on_disk, &overwrite);
    // view mode still refuses
    let view = FuncStates {
        view_mode: true,
        ..overwrite
    };
    assert!(!enabled(&view, "F_FILESAVE"));
}

// The toolbar shows a button gray when the function has no command or `IsFuncEnable` refuses it,
// and pressed while `IsFuncChecked` says so.
#[test]
fn a_toolbar_button_is_gray_or_pressed_by_the_state() {
    use crate::bars::toolbar::{Look, look};
    let undo = code("F_UNDO");
    let view_mode = code("F_VIEWMODE");
    let can_undo = FuncStates {
        can_undo: true,
        ..base()
    };
    let look_of = |enabled, pressed| Look { enabled, pressed };
    assert_eq!(look(undo, &base()), look_of(false, false));
    assert_eq!(look(undo, &can_undo), look_of(true, false));
    let view = FuncStates {
        view_mode: true,
        can_undo: true,
        ..base()
    };
    assert_eq!(look(undo, &view), look_of(false, false));
    assert_eq!(look(view_mode, &view), look_of(true, true));
    assert_eq!(look(view_mode, &base()), look_of(true, false));
    // the empty button stays gray
    assert!(!look(0, &base()).enabled);
}

fn open_window(cx: &mut TestAppContext) -> gpui::AnyWindowHandle {
    let handle = cx
        .update(|app| {
            crate::file_ops::windows::open_editor_window(
                app,
                crate::document::SharedDocument::empty(),
                None,
            )
        })
        .unwrap();
    cx.run_until_parked();
    handle
}

fn states_of(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) -> FuncStates {
    let ws = cx
        .update(|app| crate::file_ops::windows::workspace_of(app, handle))
        .unwrap();
    let id = gpui::Entity::entity_id(&ws);
    ws.read_with(cx, |w, cx| w.func_states(false, id, cx))
}

// The toolbar reads the state of its own window; the bars are shared by every window.
#[gpui::test]
fn each_window_has_its_own_state_and_the_bars_are_shared(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let first = open_window(cx);
    let second = open_window(cx);
    let ws = cx
        .update(|app| crate::file_ops::windows::workspace_of(app, first))
        .unwrap();
    let view = ws.read_with(cx, |w, _| w.editor().clone());
    first
        .update(cx, |_, window, app| {
            view.update(app, |view, cx| {
                view.run_function(code("F_VIEWMODE"), &[], window, cx)
            })
        })
        .unwrap();
    cx.run_until_parked();
    assert!(states_of(cx, first).is_checked(code("F_VIEWMODE")));
    assert!(!states_of(cx, second).is_checked(code("F_VIEWMODE")));
    // two documents are open now, counted once each
    assert_eq!(states_of(cx, first).open_documents, 2);
    assert_eq!(states_of(cx, second).open_documents, 2);
    assert_eq!(states_of(cx, first).toolbar, states_of(cx, second).toolbar);
}

// The toolbar asks on every repaint: the disk is looked at once a second at most.
#[test]
fn what_the_disk_said_is_kept_for_a_moment() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.txt");
    super::disk::forget();
    assert!(!super::disk::exists(&path));
    std::fs::write(&path, "").unwrap();
    assert!(!super::disk::exists(&path), "kept");
    super::disk::forget();
    assert!(super::disk::exists(&path));
}
