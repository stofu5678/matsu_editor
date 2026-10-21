//! Tests of the print setting dialog in GPUI's headless test window: what it shows, Sakura's
//! checks of the values, OK and Cancel, the preview laying the document out again, and the
//! settings file. The rules without a window are tested in `model`, the file in `ume_config`.

use std::path::{Path, PathBuf};

use gpui::{AppContext as _, Entity, Focusable as _, TestAppContext, VisualTestContext};
use ume_config::Profile;
use ume_config::settings::FontSetting;
use ume_core::buffer::Document;
use ume_core::model::DocumentModel;
use ume_print::{DMPAPER_A3, DMPAPER_A5, Orientation};

use super::dialog::{Part, PrintSettingDialog};
use super::model::Number;
use super::prefs::{self, PrintPrefs};
use crate::document::SharedDocument;
use crate::view::EditorView;

/// Sakura's function codes (`sakura_core/Funccode_enum.h`).
const F_PRINT_PREVIEW: u16 = 30151;
const F_PRINT_PAGESETUP: u16 = 30152;

fn setup<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
) -> (Entity<EditorView>, &'a mut VisualTestContext) {
    cx.update(gpui_component::init);
    let doc = Document::from_utf8(text.as_bytes().to_vec()).unwrap();
    let document = cx.new(|_| SharedDocument::new(DocumentModel::new(doc), None));
    cx.add_window_view(move |window, cx| {
        let view = EditorView::new(document, cx);
        window.focus(&view.focus_handle, cx);
        view
    })
}

fn many_lines(n: usize) -> String {
    (0..n).map(|i| format!("line {i}\r\n")).collect()
}

fn run(view: &Entity<EditorView>, code: u16, cx: &mut VisualTestContext) {
    view.update_in(cx, |v, window, cx| v.run_function(code, &[], window, cx));
}

fn dialog_of(
    view: &Entity<EditorView>,
    cx: &mut VisualTestContext,
) -> Option<Entity<PrintSettingDialog>> {
    view.read_with(cx, |v, _| v.print.setup_dialog.clone())
}

/// Opens the dialog with the command and returns it.
fn open(view: &Entity<EditorView>, cx: &mut VisualTestContext) -> Entity<PrintSettingDialog> {
    run(view, F_PRINT_PAGESETUP, cx);
    dialog_of(view, cx).expect("the dialog is open")
}

fn type_number(
    dialog: &Entity<PrintSettingDialog>,
    number: Number,
    text: &str,
    cx: &mut VisualTestContext,
) {
    let text = text.to_owned();
    dialog.update_in(cx, |d, window, cx| {
        let at = Number::ALL.iter().position(|n| *n == number).unwrap();
        d.numbers[at].update(cx, |state, cx| state.set_value(text, window, cx));
    });
}

fn shown(
    dialog: &Entity<PrintSettingDialog>,
    number: Number,
    cx: &mut VisualTestContext,
) -> String {
    dialog.read_with(cx, |d, cx| {
        let at = Number::ALL.iter().position(|n| *n == number).unwrap();
        d.numbers[at].read(cx).value().to_string()
    })
}

fn press_ok(dialog: &Entity<PrintSettingDialog>, cx: &mut VisualTestContext) {
    dialog.update_in(cx, |d, window, cx| d.ok(window, cx));
    cx.run_until_parked();
}

fn saved(cx: &mut VisualTestContext) -> ume_config::settings::print::PrintSettings {
    cx.update(|_, app| prefs::settings(app))
}

fn install_file(cx: &mut VisualTestContext, file: &Path) {
    cx.update(|_, app| prefs::install(app, PrintPrefs::load(None, Some(file.to_owned()))));
}

fn file_in(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("ume.ini")
}

#[gpui::test]
fn the_command_opens_the_dialog_and_not_the_preview(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    assert!(!view.read_with(cx, |v, _| v.is_print_preview_open()));
    // Sakura's defaults: 24 (1/10 mm), 30 %, 1 column, 7 mm, margins 10 / 20 / 20 / 10 mm
    let values: Vec<String> = Number::ALL.iter().map(|n| shown(&dialog, *n, cx)).collect();
    assert_eq!(values, ["24", "30", "1", "7", "10", "20", "20", "10"]);
    assert_eq!(dialog.read_with(cx, |d, _| d.current()), 0);
    assert_eq!(dialog.read_with(cx, |d, _| d.name_of(0)), "印刷設定 1");
    assert_eq!(dialog.read_with(cx, |d, _| d.name_of(7)), "印刷設定 8");
    // the dialog is not opened twice
    run(&view, F_PRINT_PAGESETUP, cx);
    assert_eq!(dialog_of(&view, cx), Some(dialog));
}

#[gpui::test]
fn the_width_of_the_line_numbers_is_the_least_without_the_preview(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &many_lines(2000));
    let dialog = open(&view, cx);
    assert_eq!(dialog.read_with(cx, |d, _| d.number_columns), 3);
    press_cancel(&dialog, cx);
    // with the preview, the digits of the document: 2000 lines need 4 and the space after them
    run(&view, F_PRINT_PREVIEW, cx);
    let dialog = open(&view, cx);
    assert_eq!(dialog.read_with(cx, |d, _| d.number_columns), 5);
}

fn press_cancel(dialog: &Entity<PrintSettingDialog>, cx: &mut VisualTestContext) {
    dialog.update_in(cx, |d, window, cx| d.cancel(window, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn ok_keeps_the_values_checked_into_their_ranges(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    type_number(&dialog, Number::FontHeight, "3", cx);
    type_number(&dialog, Number::LineSpacing, "400", cx);
    type_number(&dialog, Number::Dansuu, "9", cx);
    type_number(&dialog, Number::DanSpace, "99", cx);
    type_number(&dialog, Number::MarginTop, "x", cx);
    type_number(&dialog, Number::MarginBottom, "51", cx);
    press_ok(&dialog, cx);
    assert_eq!(dialog_of(&view, cx), None);
    let s = saved(cx);
    let one = s.get(0);
    // DataCheckAndCorrect: 7 .. 200, 0 .. 150, 1 .. 4, 0 .. 30 mm, 0 .. 50 mm
    assert_eq!((one.print_font_height, one.print_font_width), (7, 4));
    assert_eq!((one.print_line_spacing, one.print_dansuu), (150, 4));
    assert_eq!(one.print_dan_space, 300);
    assert_eq!((one.print_margin_ty, one.print_margin_by), (0, 500));
}

#[gpui::test]
fn the_boxes_show_the_corrected_values_when_ok_is_pressed(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    type_number(&dialog, Number::Dansuu, "9", cx);
    // `GetData` writes the corrected value back with SetDlgItemInt
    dialog.update_in(cx, |d, window, cx| d.select_setting(0, window, cx));
    assert_eq!(shown(&dialog, Number::Dansuu, cx), "4");
}

#[gpui::test]
fn ok_changes_the_preview(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &many_lines(300));
    run(&view, F_PRINT_PREVIEW, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    let before = preview.read_with(cx, |p, _| p.page_count());
    assert!(before >= 2);
    let dialog = open(&view, cx);
    // two columns on a landscape A3: more room, fewer pages
    dialog.update_in(cx, |d, _, _| {
        let draft = d.draft_mut();
        draft.print_paper_size = DMPAPER_A3;
        draft.print_paper_orientation = Orientation::Landscape;
    });
    type_number(&dialog, Number::Dansuu, "2", cx);
    press_ok(&dialog, cx);
    let after = preview.read_with(cx, |p, _| p.page_count());
    assert!(after < before, "{after} < {before}");
    let job = preview.read_with(cx, |p, _| p.job().setting.clone());
    assert_eq!(job.print_dansuu, 2);
    assert_eq!(job.print_paper_size, DMPAPER_A3);
    assert_eq!(
        preview.read_with(cx, |p, _| p.paper_name()),
        "A3 (297 x 420 mm)  横"
    );
}

#[gpui::test]
fn cancel_changes_nothing(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &many_lines(300));
    run(&view, F_PRINT_PREVIEW, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    let before = preview.read_with(cx, |p, _| p.page_count());
    let dialog = open(&view, cx);
    type_number(&dialog, Number::FontHeight, "100", cx);
    dialog.update_in(cx, |d, _, _| d.draft_mut().print_line_number = true);
    press_cancel(&dialog, cx);
    assert_eq!(dialog_of(&view, cx), None);
    assert_eq!(preview.read_with(cx, |p, _| p.page_count()), before);
    assert_eq!(saved(cx).get(0).print_font_height, 24);
    // the preview has the keyboard again
    assert!(view.read_with(cx, |v, _| v.is_print_preview_open()));
}

#[gpui::test]
fn the_preview_buttons_key_opens_the_dialog(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, &many_lines(10));
    run(&view, F_PRINT_PREVIEW, cx);
    assert_eq!(dialog_of(&view, cx), None);
    // ページ設定(S)...
    cx.simulate_keystrokes("s");
    assert!(dialog_of(&view, cx).is_some());
    // the preview stays under it
    assert!(view.read_with(cx, |v, _| v.is_print_preview_open()));
}

#[gpui::test]
fn another_setting_keeps_what_was_edited_in_the_first(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    type_number(&dialog, Number::FontHeight, "30", cx);
    dialog.update_in(cx, |d, window, cx| {
        d.draft_mut().print_kinsoku_head = true;
        d.select_setting(2, window, cx);
    });
    assert_eq!(dialog.read_with(cx, |d, _| d.current()), 2);
    // the third setting is Sakura's default
    assert_eq!(shown(&dialog, Number::FontHeight, cx), "24");
    assert!(!dialog.read_with(cx, |d, _| d.draft.print_kinsoku_head));
    type_number(&dialog, Number::Dansuu, "3", cx);
    dialog.update_in(cx, |d, window, cx| d.select_setting(0, window, cx));
    assert_eq!(shown(&dialog, Number::FontHeight, cx), "30");
    assert!(dialog.read_with(cx, |d, _| d.draft.print_kinsoku_head));
    press_ok(&dialog, cx);
    let s = saved(cx);
    assert_eq!(s.get(0).print_font_height, 30);
    assert_eq!(s.get(0).print_font_width, 15);
    assert!(s.get(0).print_kinsoku_head);
    // the one that was left is kept, too
    assert_eq!(s.get(2).print_dansuu, 3);
    // OK keeps the one that was shown last
    assert!(view.read_with(cx, |v, cx| v.current_print_setting(cx).print_font_height) == 30);
}

#[gpui::test]
fn the_chosen_setting_is_the_one_printing_uses(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    dialog.update_in(cx, |d, window, cx| d.select_setting(3, window, cx));
    type_number(&dialog, Number::Dansuu, "2", cx);
    press_ok(&dialog, cx);
    let job = view.read_with(cx, |v, cx| v.print_job(cx));
    assert_eq!(job.setting.name, "印刷設定 4");
    assert_eq!(job.setting.print_dansuu, 2);
    // the dialog starts at it the next time
    let dialog = open(&view, cx);
    assert_eq!(dialog.read_with(cx, |d, _| d.current()), 3);
    assert_eq!(shown(&dialog, Number::Dansuu, cx), "2");
}

#[gpui::test]
fn ok_cannot_be_pressed_without_room_to_print(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    assert!(dialog.read_with(cx, |d, cx| d.can_ok(cx)));
    // 4 columns of 20 mm characters with 30 mm between them do not fit on an A5 paper
    dialog.update_in(cx, |d, _, _| d.draft_mut().print_paper_size = DMPAPER_A5);
    type_number(&dialog, Number::FontHeight, "200", cx);
    type_number(&dialog, Number::Dansuu, "4", cx);
    type_number(&dialog, Number::DanSpace, "30", cx);
    assert!(!dialog.read_with(cx, |d, cx| d.can_ok(cx)));
    press_ok(&dialog, cx);
    // nothing happened: the dialog is open and nothing was kept
    assert!(dialog_of(&view, cx).is_some());
    assert_eq!(saved(cx).get(0).print_dansuu, 1);
    // back to a size that fits
    type_number(&dialog, Number::Dansuu, "1", cx);
    assert!(dialog.read_with(cx, |d, cx| d.can_ok(cx)));
    press_ok(&dialog, cx);
    assert_eq!(dialog_of(&view, cx), None);
    assert_eq!(saved(cx).get(0).print_paper_size, DMPAPER_A5);
}

#[gpui::test]
fn a_name_is_taken_unless_it_is_empty_and_is_cut_to_32_characters(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    dialog.update_in(cx, |d, window, cx| d.set_name("議事録", window, cx));
    assert_eq!(dialog.read_with(cx, |d, _| d.name_of(0)), "議事録");
    dialog.update_in(cx, |d, window, cx| d.set_name("", window, cx));
    assert_eq!(dialog.read_with(cx, |d, _| d.name_of(0)), "議事録");
    dialog.update_in(cx, |d, window, cx| d.set_name(&"あ".repeat(40), window, cx));
    assert_eq!(
        dialog.read_with(cx, |d, _| d.name_of(0)).chars().count(),
        32
    );
    press_ok(&dialog, cx);
    assert_eq!(saved(cx).get(0).name.chars().count(), 32);
}

#[gpui::test]
fn the_name_is_asked_with_the_one_line_dialog(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    dialog.update_in(cx, |d, window, cx| d.rename(window, cx));
    // Sakura's STR_DLGPRNST2
    let input = view
        .read_with(cx, |v, _| v.input_dialog.clone())
        .expect("the input dialog is open");
    assert_eq!(
        input.read_with(cx, |d, _| d.message().to_owned()),
        "設定の名称を入力してください。"
    );
}

#[gpui::test]
fn the_fonts_of_the_header_and_the_footer(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    let mut font = FontSetting::default();
    font.log_font.face_name = "Arial".into();
    font.point_size = 120;
    dialog.update_in(cx, |d, _, cx| d.set_font(Part::Header, &font, cx));
    let header = dialog
        .read_with(cx, |d, _| d.draft.header_font.clone())
        .unwrap();
    assert_eq!((header.face.as_str(), header.point_size), ("Arial", 120));
    // a font without a face is not taken
    let mut none = FontSetting::default();
    none.log_font.face_name = String::new();
    dialog.update_in(cx, |d, _, cx| d.set_font(Part::Footer, &none, cx));
    assert_eq!(
        dialog.read_with(cx, |d, _| d.draft.footer_font.clone()),
        None
    );
    // the check box next to the font drops it
    dialog.update_in(cx, |d, _, cx| d.set_font(Part::Footer, &font, cx));
    dialog.update_in(cx, |d, _, cx| d.drop_font(Part::Footer, cx));
    assert_eq!(
        dialog.read_with(cx, |d, _| d.draft.footer_font.clone()),
        None
    );
    press_ok(&dialog, cx);
    let one = saved(cx).get(0).clone();
    assert_eq!(one.header_font.map(|f| f.face), Some("Arial".into()));
    assert_eq!(one.footer_font, None);
}

#[gpui::test]
fn the_header_and_the_footer_are_kept_and_cut_to_99_characters(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    dialog.update_in(cx, |d, window, cx| {
        d.header[1].update(cx, |s, cx| s.set_value("$y/$m/$d", window, cx));
        d.header[0].update(cx, |s, cx| s.set_value("", window, cx));
        d.footer[2].update(cx, |s, cx| s.set_value("あ".repeat(120), window, cx));
    });
    press_ok(&dialog, cx);
    let one = saved(cx).get(0).clone();
    assert_eq!(one.header_form, ["", "$y/$m/$d", ""]);
    assert_eq!(one.footer_form[1], "- $p -");
    assert_eq!(one.footer_form[2].chars().count(), 99);
}

#[gpui::test]
fn ok_saves_print_of_the_file_in_sakuras_keys(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    std::fs::write(&file, "\u{feff}[Common]\r\nbX=1\r\n").unwrap();
    let (view, cx) = setup(cx, "abc");
    install_file(cx, &file);
    let dialog = open(&view, cx);
    type_number(&dialog, Number::Dansuu, "2", cx);
    dialog.update_in(cx, |d, window, cx| {
        d.draft_mut().print_paper_orientation = Orientation::Landscape;
        d.select_setting(1, window, cx);
    });
    dialog.update_in(cx, |d, window, cx| d.select_setting(0, window, cx));
    press_ok(&dialog, cx);
    let text = String::from_utf8(std::fs::read(&file).unwrap()).unwrap();
    assert!(text.contains("bX=1"), "{text}");
    assert!(text.contains("[Print]"), "{text}");
    assert!(
        text.contains("PS[00].nInts=12,24,2,70,30,100,200,200,100,2,9,1,0,1,0,0,1,0,0"),
        "{text}"
    );
    // the settings that were not changed are not written
    assert!(!text.contains("PS[01]"), "{text}");
    // and the next start reads them
    let profile = Profile::load(&file).unwrap();
    let back = ume_config::settings::print::PrintSettings::from_profile(
        &ume_platform::print::default_setting(),
        &profile,
    );
    assert_eq!(back.get(0).print_dansuu, 2);
    assert_eq!(back.get(0).print_paper_orientation, Orientation::Landscape);
}

#[gpui::test]
fn the_settings_of_the_file_are_what_the_dialog_starts_with(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    std::fs::write(
        &file,
        "\u{feff}[Print]\r\nPS[05].nInts=10,20,3,50,40,110,120,130,140,2,8,0,1,1,0,0,1,0,0\r\nPS[05].szSName=縦書き用\r\n"
            .as_bytes(),
    )
    .unwrap();
    let (view, cx) = setup(cx, "abc");
    cx.update(|_, app| prefs::install(app, PrintPrefs::load(Some(&file), None)));
    let dialog = open(&view, cx);
    assert_eq!(dialog.read_with(cx, |d, _| d.name_of(5)), "縦書き用");
    dialog.update_in(cx, |d, window, cx| d.select_setting(5, window, cx));
    let values: Vec<String> = Number::ALL.iter().map(|n| shown(&dialog, *n, cx)).collect();
    assert_eq!(values, ["20", "40", "3", "5", "11", "12", "13", "14"]);
    assert_eq!(
        dialog.read_with(cx, |d, _| d.draft.print_paper_orientation),
        Orientation::Landscape
    );
    assert!(dialog.read_with(cx, |d, _| d.draft.print_line_number));
}

#[gpui::test]
fn the_chosen_number_is_written_to_the_type_and_read_back(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let file = file_in(&dir);
    std::fs::write(&file, "\u{feff}[Common]\r\nbX=1\r\n").unwrap();
    let (view, cx) = setup(cx, "abc");
    install_file(cx, &file);
    let dialog = open(&view, cx);
    dialog.update_in(cx, |d, window, cx| d.select_setting(4, window, cx));
    press_ok(&dialog, cx);
    let profile = Profile::load(&file).unwrap();
    let types = ume_config::settings::DocumentTypes::from_profile(&profile);
    let index = view.read_with(cx, |v, _| v.type_index());
    assert_eq!(types.get(index).unwrap().current_print_setting, 4);
}

#[gpui::test]
fn a_second_window_sees_the_settings_after_ok(cx: &mut TestAppContext) {
    // `PM_PRINTSETTING` goes to every editor window: the other previews lay out again.
    let (view, cx) = setup(cx, &many_lines(300));
    run(&view, F_PRINT_PREVIEW, cx);
    let preview = view
        .read_with(cx, |v, _| v.print_preview().cloned())
        .unwrap();
    let before = preview.read_with(cx, |p, _| p.page_count());
    view.update_in(cx, |v, _, cx| {
        let original = prefs::settings(cx);
        let mut s = original.clone();
        let mut one = s.get(0).clone();
        one.print_font_height = 60;
        one.print_font_width = 30;
        s.set(0, one);
        prefs::commit(cx, &s, &original, v.type_index(), 0);
        v.print_setting_changed(cx);
    });
    assert!(preview.read_with(cx, |p, _| p.page_count()) > before);
}

#[gpui::test]
fn a_dialog_with_old_settings_keeps_what_another_one_saved(cx: &mut TestAppContext) {
    // Two windows open the dialog before either presses OK.
    let (view, cx) = setup(cx, "abc");
    let first = open(&view, cx);
    let second_original = saved(cx);
    type_number(&first, Number::Dansuu, "3", cx);
    press_ok(&first, cx);
    assert_eq!(saved(cx).get(0).print_dansuu, 3);
    // the second dialog changes only the second setting
    view.update_in(cx, |v, _, cx| {
        let mut s = second_original.clone();
        let mut two = s.get(1).clone();
        two.print_font_height = 40;
        two.print_font_width = 20;
        s.set(1, two);
        prefs::commit(cx, &s, &second_original, v.type_index(), 0);
    });
    let s = saved(cx);
    assert_eq!(s.get(0).print_dansuu, 3);
    assert_eq!(s.get(1).print_font_height, 40);
}

#[gpui::test]
fn cancelling_the_rename_gives_the_focus_back_to_the_dialog(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    dialog.update_in(cx, |d, window, cx| d.rename(window, cx));
    assert!(view.read_with(cx, |v, _| v.input_dialog.is_some()));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(view.read_with(cx, |v, _| v.input_dialog.is_none()));
    assert_eq!(dialog.read_with(cx, |d, _| d.name_of(0)), "印刷設定 1");
    let focused = cx.update(|window, app| dialog.read(app).focus_handle(app).is_focused(window));
    assert!(focused);
    // Esc closes the dialog, as before
    cx.simulate_keystrokes("escape");
    assert_eq!(dialog_of(&view, cx), None);
}

#[gpui::test]
fn the_font_dialog_starts_from_the_height_shown(cx: &mut TestAppContext) {
    let (view, cx) = setup(cx, "abc");
    let dialog = open(&view, cx);
    let old = dialog.update_in(cx, |d, _, cx| d.font_start(Part::Header, cx));
    type_number(&dialog, Number::FontHeight, "50", cx);
    let start = dialog.update_in(cx, |d, _, cx| d.font_start(Part::Header, cx));
    // -(50 * 96 / 254) pixels
    assert_eq!(start.log_font.height, -18);
    assert_ne!(start.log_font.height, old.log_font.height);
    // accepting it unchanged keeps the size of the new height
    dialog.update_in(cx, |d, _, cx| d.set_font(Part::Header, &start, cx));
    press_ok(&dialog, cx);
    let kept = saved(cx).get(0).header_font.clone().unwrap();
    assert_eq!(kept.point_size, start.point_size_for_dpi(96));
    assert_ne!(kept.point_size, old.point_size_for_dpi(96));
}
