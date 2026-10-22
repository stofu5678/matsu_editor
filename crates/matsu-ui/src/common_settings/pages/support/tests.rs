//! Tests of the page 支援 in GPUI's headless test windows: the controls, the keys OK writes, what
//! the windows take, and what is not available.

use gpui::TestAppContext;

use super::{EXT_HELP, EXT_HTML_HELP};
use crate::common_settings::model::{Kind, controls};
use crate::common_settings::tests::{Ini, check, ok, run, setup, type_in, with_dialog};
use crate::common_settings::{Page, all_controls};
use crate::support::prefs;

const INI: &str = "[Other]\r\nx=1\r\n\
[Common]\r\nbHokanKey_RETURN=0\r\nbHokanKey_TAB=1\r\nbHokanKey_RIGHT=0\r\n\
szExtHelp=a.chm\r\nszExtHtmlHelp=b.chm\r\nbHtmlHelpIsSingle=0\r\n\
szMigemoDll=migemo.dll\r\nszMigemoDict=dict\r\nkhlf=-12,0,0,0,400,0,0,0,128,3,2,1,49\r\nkhlfFaceName=Meiryo\r\nkhps=90\r\n";

fn open_page(cx: &mut TestAppContext, handle: gpui::AnyWindowHandle) {
    run(cx, handle, "F_OPTION");
    with_dialog(cx, |d, _, dcx| d.select_page(Page::Support, dcx));
}

fn text(cx: &mut TestAppContext, id: &str) -> String {
    with_dialog(cx, |d, _, dcx| d.input_text(id, dcx).unwrap_or_default())
}

fn current(cx: &mut TestAppContext) -> prefs::HelperPrefs {
    cx.update(|app| prefs::current(app))
}

fn get(ini: &Ini, key: &str) -> Option<String> {
    ini.get(key)
}

#[test]
fn the_page_has_the_controls_of_the_resource() {
    // `IDD_PROP_HELPER`: three keys, two files with their buttons, the check box, the font with its
    // button, and the two files of migemo with their buttons
    let list = controls(&Page::Support.items().unwrap());
    assert_eq!(list.len(), 3 + 4 + 1 + 2 + 4);
    let keys: Vec<&str> = list
        .iter()
        .map(|c| c.key)
        .filter(|k| !k.is_empty())
        .collect();
    // `ShareData_IO_Common`
    for key in [
        "bHokanKey_RETURN",
        "bHokanKey_TAB",
        "bHokanKey_RIGHT",
        "szExtHelp",
        "szExtHtmlHelp",
        "bHtmlHelpIsSingle",
        "szMigemoDll",
        "szMigemoDict",
    ] {
        assert!(keys.contains(&key), "{key}");
    }
    // Only the migemo is not available; the font and the buttons of the files are.
    for c in &list {
        assert_eq!(c.live, !c.id.starts_with("helper_migemo"), "{}", c.id);
    }
    assert!(
        list.iter()
            .any(|c| matches!(c.kind, Kind::Font { key_lf: "khlf", .. }))
    );
    assert!(all_controls().iter().any(|c| c.id == EXT_HELP));
    assert!(matches!(
        list.iter().find(|c| c.id == EXT_HTML_HELP).unwrap().kind,
        Kind::Text
    ));
}

#[gpui::test]
fn the_page_shows_the_file_and_the_defaults_are_sakuras(cx: &mut TestAppContext) {
    let (_ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    assert_eq!(text(cx, EXT_HELP), "a.chm");
    assert_eq!(text(cx, EXT_HTML_HELP), "b.chm");
    assert_eq!(text(cx, "helper_migemo_dll"), "migemo.dll");
    let font = with_dialog(cx, |d, _, _| d.values().font(super::FONT));
    assert_eq!(
        (font.log_font.face_name.as_str(), font.point_size),
        ("Meiryo", 90)
    );
    let v = |cx: &mut TestAppContext, id| with_dialog(cx, |d, _, _| d.values().bool(id));
    assert!(!v(cx, "helper_hokan_return") && v(cx, "helper_hokan_tab"));
    assert!(!v(cx, "helper_hokan_right") && !v(cx, "helper_html_help_single"));
}

#[gpui::test]
fn a_file_without_the_keys_gives_the_defaults_of_sakura(cx: &mut TestAppContext) {
    // `CShareData::InitShareData`
    let (_ini, handle) = setup(cx, "[Other]\r\nx=1\r\n");
    open_page(cx, handle);
    let v = |cx: &mut TestAppContext, id| with_dialog(cx, |d, _, _| d.values().bool(id));
    assert!(v(cx, "helper_hokan_return") && !v(cx, "helper_hokan_tab"));
    assert!(v(cx, "helper_hokan_right") && v(cx, "helper_html_help_single"));
    assert_eq!(text(cx, EXT_HELP), "");
    assert_eq!(current(cx), prefs::HelperPrefs::default());
}

#[gpui::test]
fn the_windows_start_with_the_settings_of_the_file(cx: &mut TestAppContext) {
    let (_ini, _handle) = setup(cx, INI);
    let p = current(cx);
    assert!(!p.hokan_key_return && p.hokan_key_tab && !p.hokan_key_right);
    assert_eq!(
        (p.ext_help.as_str(), p.ext_html_help.as_str()),
        ("a.chm", "b.chm")
    );
    assert!(!p.html_help_is_single);
}

#[gpui::test]
fn ok_writes_the_keys_and_the_windows_take_them(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Other]\r\nx=1\r\n");
    open_page(cx, handle);
    check(cx, "helper_hokan_return", false);
    check(cx, "helper_hokan_tab", true);
    check(cx, "helper_html_help_single", false);
    type_in(cx, EXT_HELP, "docs/help.hlp");
    type_in(cx, EXT_HTML_HELP, "C:\\help\\main.chm");
    ok(cx);
    assert_eq!(get(&ini, "bHokanKey_RETURN").as_deref(), Some("0"));
    assert_eq!(get(&ini, "bHokanKey_TAB").as_deref(), Some("1"));
    assert_eq!(get(&ini, "bHokanKey_RIGHT").as_deref(), Some("1"));
    assert_eq!(get(&ini, "bHtmlHelpIsSingle").as_deref(), Some("0"));
    assert_eq!(get(&ini, "szExtHelp").as_deref(), Some("docs/help.hlp"));
    assert_eq!(
        get(&ini, "szExtHtmlHelp").as_deref(),
        Some("C:\\help\\main.chm")
    );
    // the rest of the file stays
    assert_eq!(ini.profile().get("Other", "x"), Some("1"));
    let p = current(cx);
    assert!(!p.hokan_key_return && p.hokan_key_tab && p.hokan_key_right);
    assert_eq!(p.ext_help, "docs/help.hlp");
    assert_eq!(p.ext_html_help, "C:\\help\\main.chm");
    assert!(!p.html_help_is_single);
}

#[gpui::test]
fn what_is_not_available_stays_as_the_file_has_it(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, INI);
    open_page(cx, handle);
    // a change of an unavailable box does nothing
    type_in(cx, "helper_migemo_dll", "other.dll");
    check(cx, "helper_hokan_tab", false);
    ok(cx);
    assert_eq!(get(&ini, "szMigemoDll").as_deref(), Some("migemo.dll"));
    assert_eq!(get(&ini, "szMigemoDict").as_deref(), Some("dict"));
    // the font is not changed either
    assert_eq!(get(&ini, "khlfFaceName").as_deref(), Some("Meiryo"));
    assert_eq!(get(&ini, "khps").as_deref(), Some("90"));
    assert_eq!(get(&ini, "bHokanKey_TAB").as_deref(), Some("0"));
}

#[gpui::test]
fn the_chosen_font_is_written_as_khlf_and_the_tip_takes_it_at_once(cx: &mut TestAppContext) {
    let (ini, handle) = setup(cx, "[Other]\r\nx=1\r\n");
    assert_eq!(current(cx).key_help_font, None);
    open_page(cx, handle);
    let mut font = matsu_config::settings::FontSetting::default();
    font.log_font.face_name = "Meiryo".to_owned();
    font.point_size = 120;
    // what the font dialog gives (`IDC_BUTTON_KEYWORDHELPFONT`)
    with_dialog(cx, |d, _, dcx| d.set_font(super::FONT, font.clone(), dcx));
    ok(cx);
    assert_eq!(get(&ini, "khps").as_deref(), Some("120"));
    assert_eq!(get(&ini, "khlfFaceName").as_deref(), Some("Meiryo"));
    assert!(get(&ini, "khlf").is_some());
    assert_eq!(current(cx).key_help_font, Some(font));
}
