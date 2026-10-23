//! Tests of what the settings pages share: the functions that open the sheet at a page, the names
//! of the functions, and the dialogs of the インポート and エクスポート (the file types and the import
//! folder).

use gpui::TestAppContext;

use super::model::Kind;
use super::tests::{Ini, cancel, run, setup, with_dialog};
use super::{Page, open};

/// Presses the button `id` of the page the dialog shows.
fn press(cx: &mut TestAppContext, id: &'static str) {
    with_dialog(cx, |d, window, dcx| {
        let control = d
            .page_controls()
            .into_iter()
            .find(|c| c.id == id)
            .expect("the button");
        assert!(d.is_usable(&control), "{id} is usable");
        let Kind::Button { run } = control.kind else {
            panic!("{id} is not a button");
        };
        run(d, window, dcx);
    });
}

fn open_page(cx: &mut TestAppContext, page: Page) -> Ini {
    let (ini, first) = setup(cx, "");
    cx.update(|app| open(app, Some(page), Some(first)));
    cx.run_until_parked();
    ini
}

/// The dialog of the OS that the last button asked for.
fn asked() -> matsu_platform::dialog::Request {
    crate::file_dialog::tests::asked()
        .pop()
        .expect("a dialog was asked for")
}

#[gpui::test]
fn each_function_of_a_page_opens_the_sheet_at_that_page(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, "");
    for (function, page) in [
        ("F_OPTION_GENERAL", Page::General),
        ("F_OPTION_KEYBIND", Page::Keys),
        ("F_OPTION_CUSTMENU", Page::CustomMenu),
        ("F_OPTION_TOOLBAR", Page::Toolbar),
        ("F_OPTION_KEYWORD", Page::Keyword),
        ("F_OPTION_HELPER", Page::Support),
        ("F_OPTION_MACRO", Page::Macro),
        ("F_OPTION_PLUGIN", Page::Plugin),
        ("F_OPTION_MAINMENU", Page::MainMenu),
    ] {
        run(cx, handle, function);
        assert_eq!(with_dialog(cx, |d, _, _| d.page()), page, "{function}");
    }
    cancel(cx);
}

#[gpui::test]
fn the_custom_menu_dialogs_list_the_mnu_files(cx: &mut TestAppContext) {
    let ini = open_page(cx, Page::CustomMenu);
    crate::file_dialog::tests::asked();
    for id in ["custmenu_import", "custmenu_export"] {
        press(cx, id);
        let request = asked();
        let names: Vec<_> = request.filters.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            ["ユーザー指定", "テキストファイル", "すべてのファイル"]
        );
        assert_eq!(request.filters[0].patterns, ["*.mnu"], "{id}");
        assert_eq!(request.start.as_deref(), Some(ini.dir.0.as_path()), "{id}");
    }
    cancel(cx);
}

#[gpui::test]
fn the_keyword_dialogs_list_the_kwd_files(cx: &mut TestAppContext) {
    let ini = open_page(cx, Page::Keyword);
    crate::file_dialog::tests::asked();
    for id in ["keyword_import", "keyword_export"] {
        press(cx, id);
        let request = asked();
        assert_eq!(request.filters[0].patterns, ["*.kwd"], "{id}");
        assert_eq!(request.start.as_deref(), Some(ini.dir.0.as_path()), "{id}");
    }
    cancel(cx);
}

#[gpui::test]
fn the_folder_of_a_chosen_file_is_the_import_folder_of_every_dialog(cx: &mut TestAppContext) {
    let ini = open_page(cx, Page::CustomMenu);
    crate::file_dialog::tests::asked();
    let other = tempfile::tempdir().unwrap();
    let target = other.path().join("menus.mnu");
    press(cx, "custmenu_export");
    asked();
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert!(target.exists());
    // The settings file says so (`szIMPORTFOLDER` of `[Folders]`), with a separator at the end.
    let want = format!("{}{}", other.path().display(), std::path::MAIN_SEPARATOR);
    assert_eq!(
        ini.profile().get("Folders", "szIMPORTFOLDER"),
        Some(want.as_str())
    );
    // Another page's dialog starts there too.
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Keyword, dcx));
    press(cx, "keyword_import");
    assert_eq!(asked().start.as_deref(), Some(other.path()));
    cancel(cx);
}
