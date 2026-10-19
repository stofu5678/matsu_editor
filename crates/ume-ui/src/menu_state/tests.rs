//! Tests of the check marks and the gray items: the rules of `IsFuncChecked` / `IsFuncEnable` on a
//! plain state, then the state read from a real window, and the items the menu bar and a custom
//! menu build from it.

use gpui::{
    AppContext as _, Entity, OwnedMenuItem, TestAppContext, VisualTestContext, Window, actions,
};
use gpui_component::Root;
use ume_commands::sakura::FUNCTIONS;
use ume_core::buffer::{Document, Eol};
use ume_core::encoding::Encoding;
use ume_core::model::DocumentModel;

use super::{FuncStates, apply};
use crate::app::Workspace;
use crate::document::SharedDocument;
use crate::menu_bar::MenuBar;
use crate::search::isearch::ISearchMode;
use crate::settings::WrapMethod;
use crate::tabs::settings::TabSettings;
use crate::view::RunFunction;

actions!(menu_state_tests, [Nothing]);

pub(super) fn code(name: &str) -> u16 {
    FUNCTIONS
        .iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("{name} is not a Sakura function"))
        .code
}

/// A new, untitled UTF-8 document with CRLF, nothing selected, nothing to undo.
pub(super) fn base() -> FuncStates {
    FuncStates {
        encoding: Encoding::Utf8,
        new_line: Eol::CrLf,
        view_mode: false,
        overwrite: false,
        wrap: WrapMethod::None,
        recording_key_macro: false,
        status_bar: true,
        tab_bar: false,
        grouped: true,
        topmost: false,
        isearch: None,
        outline: false,
        can_undo: false,
        can_redo: false,
        text_selected: false,
        has_path: false,
        unc_path: false,
        path: None,
        modified: false,
        format_changed: false,
        has_search_keys: false,
        has_search_start: false,
        open_documents: 1,
        toolbar: true,
        funckeys: false,
        unmodified_overwrite: false,
        diff_shown: false,
        can_jump_back: false,
        can_jump_forward: false,
        grep_window: false,
        key_macro_saveable: false,
        key_macro_loaded: false,
        select_count_by_byte: false,
    }
}

fn checked(states: &FuncStates, name: &str) -> bool {
    states.is_checked(code(name))
}

// ---- the rules ----

// `IsFuncChecked`: `F_FILE_REOPEN_*` by the encoding of the document, and only that one.
#[test]
fn the_reopen_item_of_the_encoding_is_checked() {
    let cases = [
        ("F_FILE_REOPEN_SJIS", Encoding::ShiftJis),
        ("F_FILE_REOPEN_JIS", Encoding::Jis),
        ("F_FILE_REOPEN_EUC", Encoding::EucJp),
        ("F_FILE_REOPEN_LATIN1", Encoding::Latin1),
        ("F_FILE_REOPEN_UNICODE", Encoding::Utf16Le),
        ("F_FILE_REOPEN_UNICODEBE", Encoding::Utf16Be),
        ("F_FILE_REOPEN_UTF8", Encoding::Utf8),
        ("F_FILE_REOPEN_CESU8", Encoding::Cesu8),
        ("F_FILE_REOPEN_UTF7", Encoding::Utf7),
    ];
    for (name, encoding) in cases {
        let states = FuncStates { encoding, ..base() };
        for (other, _) in cases {
            assert_eq!(checked(&states, other), other == name, "{other} for {name}");
        }
    }
    // a code page has no item of its own
    let gbk = Encoding::choices()
        .into_iter()
        .find(|e| matches!(e, Encoding::CodePage(_)));
    if let Some(encoding) = gbk {
        let states = FuncStates { encoding, ..base() };
        assert!(cases.iter().all(|(name, _)| !checked(&states, name)));
    }
}

#[test]
fn the_line_ending_of_enter_is_checked() {
    for (eol, name) in [
        (Eol::CrLf, "F_CHGMOD_EOL_CRLF"),
        (Eol::Lf, "F_CHGMOD_EOL_LF"),
        (Eol::Cr, "F_CHGMOD_EOL_CR"),
    ] {
        let states = FuncStates {
            new_line: eol,
            ..base()
        };
        for other in ["F_CHGMOD_EOL_CRLF", "F_CHGMOD_EOL_LF", "F_CHGMOD_EOL_CR"] {
            assert_eq!(checked(&states, other), other == name, "{other} for {name}");
        }
    }
}

#[test]
fn the_wrap_method_is_checked() {
    for (wrap, name) in [
        (WrapMethod::None, "F_TMPWRAPNOWRAP"),
        (WrapMethod::SettingWidth, "F_TMPWRAPSETTING"),
        (WrapMethod::WindowWidth, "F_TMPWRAPWINDOW"),
    ] {
        let states = FuncStates { wrap, ..base() };
        for other in ["F_TMPWRAPNOWRAP", "F_TMPWRAPSETTING", "F_TMPWRAPWINDOW"] {
            assert_eq!(checked(&states, other), other == name, "{other} for {name}");
        }
    }
}

// Sakura: `m_bDispTabWnd && !m_bDispTabWndMultiWin`. A bar with the grouping off is what made a
// dropped file open a window; the mark has to say so.
#[test]
fn grouping_is_checked_only_while_the_bar_is_shown_and_grouped() {
    for (tab_bar, grouped, mark) in [
        (true, true, true),
        (true, false, false),
        (false, true, false),
        (false, false, false),
    ] {
        let states = FuncStates {
            tab_bar,
            grouped,
            ..base()
        };
        assert_eq!(
            checked(&states, "F_BIND_WINDOW"),
            mark,
            "{tab_bar} {grouped}"
        );
        assert_eq!(checked(&states, "F_SHOWTAB"), tab_bar);
    }
}

#[test]
fn the_switches_are_checked_while_on() {
    for (name, on, off) in [
        (
            "F_VIEWMODE",
            FuncStates {
                view_mode: true,
                ..base()
            },
            base(),
        ),
        (
            "F_RECKEYMACRO",
            FuncStates {
                recording_key_macro: true,
                ..base()
            },
            base(),
        ),
        (
            "F_TOPMOST",
            FuncStates {
                topmost: true,
                ..base()
            },
            base(),
        ),
        (
            "F_OUTLINE_TOGGLE",
            FuncStates {
                outline: true,
                ..base()
            },
            base(),
        ),
        (
            "F_SHOWSTATUSBAR",
            base(),
            FuncStates {
                status_bar: false,
                ..base()
            },
        ),
        // insert mode is the mark; overwrite mode clears it
        (
            "F_CHGMOD_INS",
            base(),
            FuncStates {
                overwrite: true,
                ..base()
            },
        ),
    ] {
        assert!(checked(&on, name), "{name} on");
        assert!(!checked(&off, name), "{name} off");
    }
}

#[test]
fn the_incremental_search_is_checked_by_its_mode() {
    let normal = FuncStates {
        isearch: Some(ISearchMode::Normal),
        ..base()
    };
    let regexp = FuncStates {
        isearch: Some(ISearchMode::Regexp),
        ..base()
    };
    for name in ["F_ISEARCH_NEXT", "F_ISEARCH_PREV"] {
        assert!(checked(&normal, name) && !checked(&regexp, name) && !checked(&base(), name));
    }
    for name in ["F_ISEARCH_REGEXP_NEXT", "F_ISEARCH_REGEXP_PREV"] {
        assert!(checked(&regexp, name) && !checked(&normal, name) && !checked(&base(), name));
    }
    // no Migemo search here
    assert!(!checked(&normal, "F_ISEARCH_MIGEMO_NEXT"));
}

#[test]
fn the_other_functions_are_never_checked() {
    let states = FuncStates {
        view_mode: true,
        tab_bar: true,
        text_selected: true,
        ..base()
    };
    // no minimap; the toolbar and the function key bar come in another change
    for name in ["F_UNDO", "F_FILEOPEN", "F_SHOWMINIMAP"] {
        assert!(!checked(&states, name), "{name}");
    }
    assert!(!states.is_checked(0xFFFF));
}

#[test]
fn undo_and_redo_are_gray_with_nothing_to_do() {
    let none = base();
    assert!(!none.is_enabled(code("F_UNDO")));
    assert!(!none.is_enabled(code("F_REDO")));
    let some = FuncStates {
        can_undo: true,
        can_redo: true,
        ..base()
    };
    assert!(some.is_enabled(code("F_UNDO")));
    assert!(some.is_enabled(code("F_REDO")));
}

#[test]
fn the_functions_on_a_selection_need_one() {
    let names = [
        "F_COPYLINES",
        "F_COPY_COLOR_HTML",
        "F_TOLOWER",
        "F_TOUPPER",
        "F_TABTOSPACE",
        "F_CODECNV_SJIS2UTF8",
        "F_UUDECODE",
    ];
    let selected = FuncStates {
        text_selected: true,
        ..base()
    };
    for name in names {
        assert!(!base().is_enabled(code(name)), "{name} without a selection");
        assert!(selected.is_enabled(code(name)), "{name} with a selection");
    }
    // the reverse: a whole row is cut or deleted only without a selection
    for name in ["F_CUT_LINE", "F_DELETE_LINE"] {
        assert!(base().is_enabled(code(name)), "{name}");
        assert!(!selected.is_enabled(code(name)), "{name} with a selection");
    }
}

#[test]
fn a_function_with_no_rule_is_enabled() {
    assert!(base().is_enabled(code("F_FILEOPEN")));
    assert!(base().is_enabled(code("F_VIEWMODE")));
    assert!(base().is_enabled(0xFFFF));
}

// ---- the items ----

fn action(name: &str, function: &str) -> OwnedMenuItem {
    OwnedMenuItem::Action {
        name: name.to_owned(),
        action: Box::new(RunFunction {
            code: code(function),
        }),
        os_action: None,
        checked: false,
        disabled: false,
    }
}

/// `(checked, disabled)` of the first item that runs `function`, looking into the submenus.
fn marks(items: &[OwnedMenuItem], function: &str) -> Option<(bool, bool)> {
    let wanted = code(function);
    items.iter().find_map(|item| match item {
        OwnedMenuItem::Action {
            action,
            checked,
            disabled,
            ..
        } if action
            .as_any()
            .downcast_ref::<RunFunction>()
            .is_some_and(|a| a.code == wanted) =>
        {
            Some((*checked, *disabled))
        }
        OwnedMenuItem::Submenu(menu) => marks(&menu.items, function),
        _ => None,
    })
}

#[test]
fn the_marks_go_on_the_items_that_run_a_function() {
    let states = FuncStates {
        view_mode: true,
        text_selected: true,
        ..base()
    };
    let mut items = vec![
        action("view", "F_VIEWMODE"),
        OwnedMenuItem::Separator,
        OwnedMenuItem::Submenu(gpui::Menu::new("sub").owned()),
    ];
    let OwnedMenuItem::Submenu(sub) = &mut items[2] else {
        unreachable!()
    };
    sub.items = vec![action("undo", "F_UNDO"), action("lower", "F_TOLOWER")];
    // an item that was gray stays gray, and an action of another kind is left alone
    items.push(OwnedMenuItem::Action {
        name: "other".to_owned(),
        action: Box::new(Nothing),
        os_action: None,
        checked: false,
        disabled: false,
    });
    apply(&mut items, &states);
    assert_eq!(marks(&items, "F_VIEWMODE"), Some((true, false)));
    assert_eq!(marks(&items, "F_UNDO"), Some((false, true)));
    // selected, but view mode forbids the conversion
    assert_eq!(marks(&items, "F_TOLOWER"), Some((false, true)));
    let OwnedMenuItem::Action {
        checked, disabled, ..
    } = &items[3]
    else {
        unreachable!()
    };
    assert!(!checked && !disabled);

    let mut gray = vec![action("lower", "F_TOLOWER")];
    let OwnedMenuItem::Action { disabled, .. } = &mut gray[0] else {
        unreachable!()
    };
    *disabled = true;
    apply(&mut gray, &states);
    assert_eq!(marks(&gray, "F_TOLOWER"), Some((false, true)));
}

// ---- a window ----

pub(super) struct Fixture {
    workspace: Entity<Workspace>,
}

pub(super) fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Fixture, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    let made: std::rc::Rc<std::cell::RefCell<Option<Entity<Workspace>>>> = Default::default();
    let slot = made.clone();
    let (_root, vcx) = cx.add_window_view(move |window, cx| {
        let workspace = cx.new(|cx| Workspace::new(document, window, cx));
        *slot.borrow_mut() = Some(workspace.clone());
        Root::new(workspace, window, cx)
    });
    vcx.update(|window, _| window.activate_window());
    vcx.run_until_parked();
    let workspace = made.borrow().clone().unwrap();
    (Fixture { workspace }, vcx)
}

impl Fixture {
    pub(super) fn run(&self, cx: &mut VisualTestContext, function: &str) {
        cx.dispatch_action(RunFunction {
            code: code(function),
        });
        cx.run_until_parked();
    }

    pub(super) fn states(&self, cx: &mut VisualTestContext) -> FuncStates {
        self.workspace.read_with(cx, |w, cx| {
            w.func_states(false, gpui::Entity::entity_id(&self.workspace), cx)
        })
    }

    /// The items of Sakura's main menu as the menu bar of this window builds them when it opens.
    fn bar_items(&self, cx: &mut VisualTestContext) -> Vec<OwnedMenuItem> {
        let keymap =
            ume_commands::keymap::KeyMap::new(ume_commands::keymap::KeyMapOptions::for_platform(
                ume_commands::keymap::Platform::Other,
            ));
        let mut items: Vec<OwnedMenuItem> =
            crate::menu::main_menu(&keymap, &crate::menu::Registered)
                .into_iter()
                .flat_map(|top| {
                    crate::app::gpui_items(top.entries, ume_commands::keymap::DisplayStyle::Sakura)
                })
                .map(gpui::MenuItem::owned)
                .collect();
        let bar = self.workspace.read_with(cx, |w, _| w.menu_bar.clone());
        cx.update(|window: &mut Window, app| MenuBar::mark_items(&bar, &mut items, window, app));
        items
    }
}

fn set_tabs(cx: &mut VisualTestContext, show_bar: bool, grouped: bool) {
    cx.update(|_, app| {
        app.set_global(TabSettings {
            show_bar,
            grouped,
            new_window: false,
        });
    });
}

#[gpui::test]
fn a_new_window_shows_sakuras_defaults(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    let states = f.states(cx);
    assert_eq!(states.encoding, Encoding::Utf8);
    assert!(!states.view_mode && !states.overwrite && !states.outline && !states.topmost);
    assert!(!states.can_undo && !states.can_redo && !states.text_selected);
    assert_eq!(states.wrap, WrapMethod::None);
    assert!(checked(&states, "F_FILE_REOPEN_UTF8"));
    assert!(checked(&states, "F_CHGMOD_INS"));
    assert!(!states.is_enabled(code("F_UNDO")));
}

#[gpui::test]
fn the_state_follows_the_functions_that_change_it(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef");
    f.run(cx, "F_VIEWMODE");
    assert!(checked(&f.states(cx), "F_VIEWMODE"));
    f.run(cx, "F_VIEWMODE");
    assert!(!checked(&f.states(cx), "F_VIEWMODE"));

    f.run(cx, "F_TMPWRAPWINDOW");
    let states = f.states(cx);
    assert!(checked(&states, "F_TMPWRAPWINDOW") && !checked(&states, "F_TMPWRAPNOWRAP"));

    f.run(cx, "F_CHGMOD_EOL_LF");
    let states = f.states(cx);
    assert!(checked(&states, "F_CHGMOD_EOL_LF") && !checked(&states, "F_CHGMOD_EOL_CRLF"));
    f.run(cx, "F_CHGMOD_EOL_CRLF");
    assert!(checked(&f.states(cx), "F_CHGMOD_EOL_CRLF"));

    f.run(cx, "F_CHGMOD_INS");
    assert!(!checked(&f.states(cx), "F_CHGMOD_INS"));

    f.run(cx, "F_SHOWSTATUSBAR");
    assert!(!checked(&f.states(cx), "F_SHOWSTATUSBAR"));

    f.run(cx, "F_RECKEYMACRO");
    assert!(checked(&f.states(cx), "F_RECKEYMACRO"));
    f.run(cx, "F_RECKEYMACRO");
    assert!(!checked(&f.states(cx), "F_RECKEYMACRO"));
}

#[gpui::test]
fn undo_is_available_after_an_edit_and_a_selection_enables_the_conversions(
    cx: &mut TestAppContext,
) {
    let (f, cx) = setup(cx, "abc\ndef");
    assert!(!f.states(cx).is_enabled(code("F_UNDO")));
    cx.simulate_input("x");
    let states = f.states(cx);
    assert!(states.is_enabled(code("F_UNDO")));
    assert!(!states.is_enabled(code("F_REDO")));
    f.run(cx, "F_UNDO");
    let states = f.states(cx);
    assert!(!states.is_enabled(code("F_UNDO")));
    assert!(states.is_enabled(code("F_REDO")));

    assert!(!f.states(cx).is_enabled(code("F_TOLOWER")));
    f.run(cx, "F_SELECTALL");
    assert!(f.states(cx).is_enabled(code("F_TOLOWER")));
    assert!(!f.states(cx).is_enabled(code("F_DELETE_LINE")));
}

#[gpui::test]
fn the_incremental_search_and_the_outline_show_in_the_state(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc\ndef");
    f.run(cx, "F_ISEARCH_NEXT");
    assert!(checked(&f.states(cx), "F_ISEARCH_NEXT"));
    assert!(!checked(&f.states(cx), "F_ISEARCH_REGEXP_NEXT"));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(!checked(&f.states(cx), "F_ISEARCH_NEXT"));
}

// The case that made the user ask for this: the bar is shown and the grouping is off.
#[gpui::test]
fn the_menu_bar_checks_grouping_by_the_settings(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    for (show_bar, grouped, tab_mark, bind_mark) in [
        (true, true, true, true),
        (true, false, true, false),
        (false, true, false, false),
    ] {
        set_tabs(cx, show_bar, grouped);
        let items = f.bar_items(cx);
        assert_eq!(
            marks(&items, "F_SHOWTAB").map(|m| m.0),
            Some(tab_mark),
            "{show_bar} {grouped}"
        );
        assert_eq!(
            marks(&items, "F_BIND_WINDOW").map(|m| m.0),
            Some(bind_mark),
            "{show_bar} {grouped}"
        );
    }
}

#[gpui::test]
fn the_menu_bar_marks_the_window_it_belongs_to(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    let items = f.bar_items(cx);
    assert_eq!(marks(&items, "F_FILE_REOPEN_UTF8"), Some((true, false)));
    assert_eq!(
        marks(&items, "F_FILE_REOPEN_SJIS").map(|m| m.0),
        Some(false)
    );
    assert_eq!(marks(&items, "F_VIEWMODE").map(|m| m.0), Some(false));
    // nothing to undo: gray
    assert_eq!(marks(&items, "F_UNDO").map(|m| m.1), Some(true));

    f.run(cx, "F_VIEWMODE");
    cx.simulate_input("x");
    f.run(cx, "F_VIEWMODE");
    cx.simulate_input("y");
    let items = f.bar_items(cx);
    assert_eq!(marks(&items, "F_VIEWMODE").map(|m| m.0), Some(false));
    assert_eq!(marks(&items, "F_UNDO").map(|m| m.1), Some(false));
    f.run(cx, "F_VIEWMODE");
    assert_eq!(
        marks(&f.bar_items(cx), "F_VIEWMODE").map(|m| m.0),
        Some(true)
    );
}

#[gpui::test]
fn a_custom_menu_carries_the_same_marks(cx: &mut TestAppContext) {
    let (f, cx) = setup(cx, "abc");
    f.run(cx, "F_VIEWMODE");
    let view = f.workspace.read_with(cx, |w, _| w.editor().clone());
    let mut items = vec![
        action("view", "F_VIEWMODE"),
        action("undo", "F_UNDO"),
        action("utf8", "F_FILE_REOPEN_UTF8"),
    ];
    view.update_in(cx, |view, window, cx| {
        view.mark_menu_items(&mut items, window, cx);
    });
    assert_eq!(marks(&items, "F_VIEWMODE"), Some((true, false)));
    assert_eq!(marks(&items, "F_UNDO"), Some((false, true)));
    assert_eq!(marks(&items, "F_FILE_REOPEN_UTF8"), Some((true, false)));
}

// macOS reads the active window for its native menu. The root of a window is `Root`, not the
// workspace, so the state has to be found through the list of the open windows.
#[gpui::test]
fn the_state_of_the_active_window_is_found_for_the_native_menu(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let handle = cx.update(|app| {
        crate::file_ops::windows::open_editor_window(app, SharedDocument::empty(), None).unwrap()
    });
    cx.run_until_parked();
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    let states = cx
        .update(crate::app::active_states)
        .expect("an active window");
    assert_eq!(states.encoding, Encoding::Utf8);
    assert!(!states.can_undo);
}

// The refresh runs from `App::defer`, outside the update of the window that asked: reading the
// active window works there (a window's own `defer` runs inside its update, where it would not).
#[gpui::test]
fn the_state_can_be_read_from_the_applications_deferred_call(cx: &mut TestAppContext) {
    cx.update(gpui_component::init);
    let handle = cx.update(|app| {
        crate::file_ops::windows::open_editor_window(app, SharedDocument::empty(), None).unwrap()
    });
    cx.run_until_parked();
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    let read = std::rc::Rc::new(std::cell::RefCell::new(None));
    let slot = read.clone();
    handle
        .update(cx, move |_, _, app| {
            app.defer(move |app| *slot.borrow_mut() = Some(crate::app::active_states(app)));
        })
        .unwrap();
    cx.run_until_parked();
    assert!(read.borrow().clone().flatten().is_some());
}

// The native menu of macOS asks whether the action can be handled, so a function that is gray now
// gets the action that nobody handles.
#[test]
fn a_gray_item_of_the_native_menu_cannot_be_handled() {
    let mut items = vec![
        gpui::MenuItem::action(
            "undo",
            RunFunction {
                code: code("F_UNDO"),
            },
        ),
        gpui::MenuItem::action(
            "view",
            RunFunction {
                code: code("F_VIEWMODE"),
            },
        ),
    ];
    let states = FuncStates {
        view_mode: true,
        ..base()
    };
    super::apply_to_menu_items(&mut items, &states);
    let gpui::MenuItem::Action {
        action, disabled, ..
    } = &items[0]
    else {
        unreachable!()
    };
    assert!(*disabled);
    assert!(action.as_any().is::<crate::app::Unavailable>());
    let gpui::MenuItem::Action {
        action,
        checked,
        disabled,
        ..
    } = &items[1]
    else {
        unreachable!()
    };
    assert!(*checked && !*disabled);
    assert!(action.as_any().is::<RunFunction>());
}
