//! インポート and エクスポート of the colors (`.col`) on the page カラー.

use gpui::TestAppContext;

use super::*;
use crate::common_settings::model::Kind;

/// The sheet of the basic type, opened at the page カラー.
fn open_color_page(cx: &mut TestAppContext, text: &str) -> Ini {
    let (ini, a) = setup(cx, text);
    run(cx, a, "F_TYPE_COLOR", &[]);
    ini
}

/// Presses the button `id` of the page the sheet shows.
fn press_button(cx: &mut TestAppContext, id: &'static str) {
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

/// The text color and the background color of the item `name` as the sheet holds them.
fn item_colors(cx: &mut TestAppContext, name: &str) -> (i32, i32) {
    let item = ColorSet::all_names().position(|n| n == name).unwrap();
    with_dialog(cx, |d, _, _| {
        (
            d.values().int(item_key(item, Field::Text)),
            d.values().int(item_key(item, Field::Back)),
        )
    })
}

#[gpui::test]
fn the_colors_go_to_a_file_and_come_back(cx: &mut TestAppContext) {
    let ini = open_color_page(
        cx,
        "[Types(0)]\r\nC[CMT]=1,1,0000ff,00ff00,0\r\nC[URL]=0,0,123456,654321,1\r\n",
    );
    let dir = ini.path.parent().unwrap().to_owned();
    let file = dir.join("mine.col");
    assert!(with_dialog(cx, |d, window, dcx| d.colors_export_to(&file, window, dcx)));
    let bytes = std::fs::read(&file).unwrap();
    assert!(bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    let text = String::from_utf8(bytes[3..].to_vec()).unwrap();
    assert!(text.starts_with("; テキストエディタ色設定 Ver3"), "{text}");
    assert!(text.contains("[SakuraColor]"), "{text}");
    assert!(text.contains("C[CMT]=1,1,0000ff,00ff00,0"), "{text}");
    assert!(text.contains("C[URL]=0,0,123456,654321,1"), "{text}");
    // Other colors in the page, then the file brings the colors back.
    let item = ColorSet::all_names().position(|n| n == "CMT").unwrap();
    with_dialog(cx, |d, _, dcx| {
        d.edit_values(
            |v| {
                v.set(item_key(item, Field::Text), Value::Int(0x0a_0b0c));
                v.set(item_key(item, Field::Bold), Value::Bool(false));
            },
            dcx,
        );
    });
    assert_eq!(item_colors(cx, "CMT").0, 0x0a_0b0c);
    assert!(with_dialog(cx, |d, window, dcx| d
        .colors_import_from(&file, window, dcx)));
    assert_eq!(item_colors(cx, "CMT"), (0xff, 0x00ff00));
    assert_eq!(item_colors(cx, "URL"), (0x123456, 0x654321));
    assert!(with_dialog(cx, |d, _, _| d
        .values()
        .bool(item_key(item, Field::Bold))));
    // Nothing is written until OK.
    assert_eq!(
        ini.get("Types(0)", "C[CMT]").as_deref(),
        Some("1,1,0000ff,00ff00,0")
    );
    cancel(cx);
}

#[gpui::test]
fn a_color_file_from_sakura_is_read_and_the_chosen_item_shows_it(cx: &mut TestAppContext) {
    let ini = open_color_page(cx, "");
    let file = ini.path.parent().unwrap().join("sakura.col");
    // Shift_JIS, with the comment `;` and the header of Sakura; only some colors.
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(
        "; テキストエディタ色設定 Ver3\r\n\r\n[SakuraColor]\r\nC[TXT]=1,0,00ff00,ffffff,0\r\nC[KW1]=1,1,0000ff,ffffff,1\r\n",
    );
    std::fs::write(&file, &bytes).unwrap();
    let before_url = item_colors(cx, "URL");
    // The chosen item is the first (テキスト): its controls show the file's colors.
    assert!(with_dialog(cx, |d, window, dcx| d
        .colors_import_from(&file, window, dcx)));
    assert_eq!(item_colors(cx, "TXT").0, 0x00ff00);
    assert_eq!(
        with_dialog(cx, |d, _, _| d.values().int(COLOR_TEXT)),
        0x00ff00
    );
    // A color the file lacks keeps the page's value.
    assert_eq!(item_colors(cx, "URL"), before_url);
    let kw1 = ColorSet::all_names().position(|n| n == "KW1").unwrap();
    assert!(with_dialog(cx, |d, _, _| d
        .values()
        .bool(item_key(kw1, Field::Underline))));
    // OK writes them to the type.
    ok(cx);
    assert_eq!(
        ini.get("Types(0)", "C[KW1]").as_deref(),
        Some("1,1,0000ff,ffffff,1")
    );
}

#[gpui::test]
fn a_file_of_another_format_does_not_change_the_colors(cx: &mut TestAppContext) {
    let ini = open_color_page(cx, "");
    let dir = ini.path.parent().unwrap().to_owned();
    let before = item_colors(cx, "CMT");
    let old = dir.join("old.col");
    std::fs::write(
        &old,
        "\u{feff}; テキスト\r\n[SakuraColor]\r\nC[CMT]=1,0,0,0,0\r\n".as_bytes(),
    )
    .unwrap();
    assert!(!with_dialog(cx, |d, window, dcx| d.colors_import_from(&old, window, dcx)));
    assert!(!with_dialog(cx, |d, window, dcx| d.colors_import_from(
        &dir.join("missing.col"),
        window,
        dcx
    )));
    assert_eq!(item_colors(cx, "CMT"), before);
    cancel(cx);
}

#[gpui::test]
fn the_color_buttons_list_the_col_files_and_have_no_name_to_offer(cx: &mut TestAppContext) {
    let ini = open_color_page(cx, "");
    crate::file_dialog::tests::asked();
    press_button(cx, COLORS_EXPORT);
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    assert_eq!(request.filters[0].patterns, ["*.col"]);
    // Sakura does not suggest a name for the colors.
    assert_eq!(request.name, None);
    assert_eq!(request.start.as_deref(), Some(ini.path.parent().unwrap()));
    let target = ini.path.parent().unwrap().join("out.col");
    let chosen = target.clone();
    cx.simulate_new_path_selection(move |_| Some(chosen));
    cx.run_until_parked();
    assert!(target.exists());
    press_button(cx, COLORS_IMPORT);
    let request = crate::file_dialog::tests::asked().pop().expect("a dialog");
    assert_eq!(request.filters[0].patterns, ["*.col"]);
    cancel(cx);
}
