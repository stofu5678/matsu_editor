use super::*;

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn text(profile: &Profile) -> String {
    String::from_utf8(profile.to_bytes()).unwrap()
}

#[test]
fn the_eight_settings_start_as_the_base_with_sakuras_names() {
    let s = PrintSettings::new(&PrintSetting::default());
    assert_eq!(s.slots().len(), MAX_PRINT_SETTINGS);
    assert_eq!(s.get(0).name, "印刷設定 1");
    assert_eq!(s.get(7).name, "印刷設定 8");
    assert_eq!(
        PrintSetting {
            name: String::new(),
            ..s.get(3).clone()
        },
        PrintSetting::default()
    );
    // a number out of range is the first
    assert_eq!(s.get(99).name, "印刷設定 1");
}

#[test]
fn an_empty_file_gives_the_base() {
    let base = PrintSetting::default();
    let from_file = PrintSettings::from_profile(&base, &Profile::new());
    assert_eq!(from_file, PrintSettings::new(&base));
}

#[test]
fn reads_sakuras_keys() {
    let p = profile(
        "[Print]\r\n\
         PS[01].nInts=10,20,2,80,40,110,120,130,140,2,8,0,1,0,1,1,0,1,1\r\n\
         PS[01].szSName=議事録\r\n\
         PS[01].szFF=Courier\r\n\
         PS[01].szFFZ=ＭＳ ゴシック\r\n\
         PS[01].szHF[0]=$f\r\n\
         PS[01].szHF[1]=$y/$m/$d\r\n\
         PS[01].szHF[2]=\r\n\
         PS[01].szFTF[0]=\r\n\
         PS[01].szFTF[1]=- $p / $P -\r\n\
         PS[01].szFTF[2]=$N\r\n\
         PS[01].lfHeader=-16,0,0,0,400,0,0,0,128,3,2,1,49\r\n\
         PS[01].nHeaderPointSize=120\r\n\
         PS[01].lfHeaderFaceName=Arial\r\n\
         PS[01].bKinsokuHead=1\r\n\
         PS[01].bKinsokuTail=0\r\n\
         PS[01].bKinsokuRet=1\r\n\
         PS[01].bKinsokuKuto=1\r\n\
         PS[01].bColorPrint=1\r\n",
    );
    let s = PrintSettings::from_profile(&PrintSetting::default(), &p);
    let one = s.get(1);
    assert_eq!(one.name, "議事録");
    assert_eq!(
        (
            one.print_font_width,
            one.print_font_height,
            one.print_dansuu
        ),
        (10, 20, 2)
    );
    assert_eq!((one.print_dan_space, one.print_line_spacing), (80, 40));
    assert_eq!(
        (
            one.print_margin_ty,
            one.print_margin_by,
            one.print_margin_lx,
            one.print_margin_rx
        ),
        (110, 120, 130, 140)
    );
    assert_eq!(one.print_paper_orientation, Orientation::Landscape);
    assert_eq!(one.print_paper_size, 8);
    assert!(!one.print_word_wrap && one.print_line_number);
    assert_eq!(one.print_font_face_han, "Courier");
    assert_eq!(one.print_font_face_zen, "ＭＳ ゴシック");
    assert_eq!(one.header_form, ["$f", "$y/$m/$d", ""]);
    assert_eq!(one.footer_form, ["", "- $p / $P -", "$N"]);
    assert_eq!(
        one.header_font,
        Some(HeaderFooterFont {
            face: "Arial".into(),
            point_size: 120
        })
    );
    // the footer font is not in the file
    assert_eq!(one.footer_font, None);
    assert!(one.print_kinsoku_head && !one.print_kinsoku_tail && one.print_kinsoku_ret);
    assert!(one.print_kinsoku_kuto && one.color_print);
    // the others are untouched
    assert_eq!(
        s.get(0),
        &PrintSettings::new(&PrintSetting::default()).slots()[0]
    );
}

#[test]
fn fewer_than_19_numbers_are_ignored_as_a_whole() {
    let p = profile("[Print]\r\nPS[00].nInts=10,20,2,80\r\nPS[00].szSName=x\r\n");
    let s = PrintSettings::from_profile(&PrintSetting::default(), &p);
    assert_eq!(s.get(0).print_font_height, 24);
    assert_eq!(s.get(0).print_dansuu, 1);
    // the other entries are still read
    assert_eq!(s.get(0).name, "x");
}

#[test]
fn an_empty_font_face_is_no_font() {
    let p = profile(
        "[Print]\r\nPS[00].lfFooter=0,0,0,0,0,0,0,0,0,0,0,0,0\r\nPS[00].nFooterPointSize=0\r\nPS[00].lfFooterFaceName=\r\n",
    );
    let base = PrintSetting {
        footer_font: Some(HeaderFooterFont {
            face: "Old".into(),
            point_size: 100,
        }),
        ..PrintSetting::default()
    };
    let s = PrintSettings::from_profile(&base, &p);
    assert_eq!(s.get(0).footer_font, None);
}

#[test]
fn the_old_default_forms_are_converted() {
    // 2002.02.16 hor
    let p =
        profile("[Print]\r\nPS[00].szHF[0]=&f\r\nPS[00].szFTF[0]=&C- &P -\r\nPS[00].szFTF[1]=\r\n");
    let s = PrintSettings::from_profile(&PrintSetting::default(), &p);
    assert_eq!(s.get(0).header_form[0], "$f");
    assert_eq!(s.get(0).footer_form, ["", "- $p -", ""]);
}

#[test]
fn a_long_name_and_form_are_cut_to_sakuras_buffers() {
    let long = "あ".repeat(60);
    let form = "x".repeat(150);
    let p = profile(&format!(
        "[Print]\r\nPS[00].szSName={long}\r\nPS[00].szHF[1]={form}\r\n"
    ));
    let s = PrintSettings::from_profile(&PrintSetting::default(), &p);
    assert_eq!(s.get(0).name.chars().count(), NAME_MAX);
    assert_eq!(s.get(0).header_form[1].chars().count(), 99);
}

#[test]
fn writing_then_reading_gives_the_same_settings() {
    let base = PrintSetting::default();
    let original = PrintSettings::new(&base);
    let mut changed = original.clone();
    let mut s = changed.get(2).clone();
    s.name = "A4 横".into();
    s.print_font_height = 20;
    s.print_font_width = 10;
    s.print_dansuu = 2;
    s.print_paper_orientation = Orientation::Landscape;
    s.print_paper_size = matsu_print::DMPAPER_A3;
    s.print_line_number = true;
    s.print_word_wrap = false;
    s.print_kinsoku_head = true;
    s.print_kinsoku_kuto = true;
    s.color_print = true;
    s.print_font_face_han = "Courier New".into();
    s.print_font_face_zen = "游明朝".into();
    s.header_form = ["$f".into(), "中央".into(), "$y-$m-$d".into()];
    s.footer_form = [String::new(), "- $p -".into(), "$N".into()];
    s.header_font = Some(HeaderFooterFont {
        face: "Arial".into(),
        point_size: 90,
    });
    s.footer_font = None;
    changed.set(2, s);
    let mut p = Profile::new();
    changed.write_changed(&original, &mut p).unwrap();
    let back = PrintSettings::from_profile(&base, &p);
    assert_eq!(back, changed);
}

#[test]
fn only_the_changed_settings_are_written_in_sakuras_keys() {
    let base = PrintSetting::default();
    let original = PrintSettings::new(&base);
    let mut changed = original.clone();
    let mut s = changed.get(0).clone();
    s.print_dansuu = 2;
    s.print_paper_orientation = Orientation::Landscape;
    changed.set(0, s);
    let mut p = Profile::new();
    changed.write_changed(&original, &mut p).unwrap();
    let out = text(&p);
    assert!(
        out.contains("PS[00].nInts=12,24,2,70,30,100,200,200,100,2,9,1,0,1,0,0,1,0,0"),
        "{out}"
    );
    assert!(out.contains("PS[00].szSName=印刷設定 1"), "{out}");
    assert!(out.contains("PS[00].szHF[0]=$f"), "{out}");
    assert!(out.contains("PS[00].szFTF[1]=- $p -"), "{out}");
    assert!(out.contains("PS[00].bKinsokuHead=0"), "{out}");
    assert!(out.contains("PS[00].bColorPrint=0"), "{out}");
    assert!(
        out.contains("PS[00].lfHeader=0,0,0,0,0,0,0,0,0,0,0,0,0"),
        "{out}"
    );
    // the second setting was not changed and is not written
    assert!(!out.contains("PS[01]"), "{out}");
}

#[test]
fn nothing_else_in_the_file_is_touched() {
    let src = "[Common]\r\nbX=1\r\n\r\n[Print]\r\nPS[00].nInts=12, 24, 1, 70, 30, 100, 200, 200, 100, 1, 9, 1, 0, 0, 1, 1, 0, 0, 1\r\nPS[00].szDriver=drv\r\nPS[00].szDevice=dev\r\nPS[00].szOutput=out\r\nPS[00].szSName=印刷設定 1\r\n\r\n[Other]\r\nnY=2\r\n";
    let mut p = profile(src);
    let base = PrintSetting::default();
    let original = PrintSettings::from_profile(&base, &p);
    let mut changed = original.clone();
    let mut s = changed.get(0).clone();
    s.print_margin_lx = 150;
    changed.set(0, s);
    changed.write_changed(&original, &mut p).unwrap();
    let out = text(&p);
    // the printer and the header / footer flags (0,1,1,0,0,1) are kept
    assert!(out.contains("PS[00].szDriver=drv"), "{out}");
    assert!(out.contains("PS[00].szDevice=dev"), "{out}");
    assert!(out.contains("PS[00].szOutput=out"), "{out}");
    assert!(
        out.contains("PS[00].nInts=12,24,1,70,30,100,200,150,100,1,9,1,0,0,1,1,0,0,1"),
        "{out}"
    );
    assert!(out.contains("[Common]\r\nbX=1\r\n"), "{out}");
    assert!(out.contains("[Other]\r\nnY=2\r\n"), "{out}");
}

#[test]
fn writing_what_the_file_has_leaves_the_file_as_it_is() {
    let src = "[Print]\r\nPS[00].nInts=12, 24, 1, 70, 30, 100, 200, 200, 100, 1, 9, 1, 0, 1, 0, 0, 1, 0, 0\r\nPS[00].szSName=印刷設定 1\r\nPS[00].szFF=ＭＳ 明朝\r\nPS[00].szFFZ=ＭＳ 明朝\r\nPS[00].szHF[0]=$f\r\nPS[00].szHF[1]=\r\nPS[00].szHF[2]=\r\nPS[00].szFTF[0]=\r\nPS[00].szFTF[1]=- $p -\r\nPS[00].szFTF[2]=\r\nPS[00].lfHeader=0,0,0,0,0,0,0,0,0,0,0,0,0\r\nPS[00].nHeaderPointSize=0\r\nPS[00].lfHeaderFaceName=\r\nPS[00].lfFooter=0,0,0,0,0,0,0,0,0,0,0,0,0\r\nPS[00].nFooterPointSize=0\r\nPS[00].lfFooterFaceName=\r\nPS[00].bKinsokuHead=0\r\nPS[00].bKinsokuTail=0\r\nPS[00].bKinsokuRet=0\r\nPS[00].bKinsokuKuto=0\r\nPS[00].bColorPrint=0\r\n";
    let mut p = profile(src);
    let before = p.to_bytes();
    let base = PrintSetting::default();
    let original = PrintSettings::from_profile(&base, &p);
    // a setting the dialog "changed" back to the same values
    let mut same = original.clone();
    same.set(0, original.get(0).clone());
    same.write_changed(&PrintSettings::new(&base), &mut p)
        .unwrap();
    assert_eq!(p.to_bytes(), before);
}

#[test]
fn a_header_font_keeps_its_logfont_while_its_face_and_size_stay() {
    let src = "[Print]\r\nPS[00].lfHeader=-16,0,0,0,700,1,0,0,128,3,2,1,49\r\nPS[00].nHeaderPointSize=120\r\nPS[00].lfHeaderFaceName=Arial\r\n";
    let mut p = profile(src);
    let base = PrintSetting::default();
    let original = PrintSettings::from_profile(&base, &p);
    let mut changed = original.clone();
    let mut s = changed.get(0).clone();
    s.print_dansuu = 2;
    changed.set(0, s);
    changed.write_changed(&original, &mut p).unwrap();
    let out = text(&p);
    // bold and italic stay
    assert!(
        out.contains("PS[00].lfHeader=-16,0,0,0,700,1,0,0,128,3,2,1,49"),
        "{out}"
    );
    // a new size changes the height (9 pt at 96 dpi is 12 pixels)
    let mut again = original.clone();
    let mut s = again.get(0).clone();
    s.header_font = Some(HeaderFooterFont {
        face: "Arial".into(),
        point_size: 90,
    });
    again.set(0, s);
    let mut p = profile(src);
    again.write_changed(&original, &mut p).unwrap();
    let out = text(&p);
    assert!(
        out.contains("PS[00].lfHeader=-12,0,0,0,700,1,0,0,128,3,2,1,49"),
        "{out}"
    );
    assert!(out.contains("PS[00].nHeaderPointSize=90"), "{out}");
}

#[test]
fn removing_a_font_writes_sakuras_empty_logfont() {
    let src = "[Print]\r\nPS[00].lfFooter=-16,0,0,0,400,0,0,0,128,3,2,1,49\r\nPS[00].nFooterPointSize=120\r\nPS[00].lfFooterFaceName=Arial\r\n";
    let mut p = profile(src);
    let original = PrintSettings::from_profile(&PrintSetting::default(), &p);
    assert!(original.get(0).footer_font.is_some());
    let mut changed = original.clone();
    let mut s = changed.get(0).clone();
    s.footer_font = None;
    changed.set(0, s);
    changed.write_changed(&original, &mut p).unwrap();
    let out = text(&p);
    assert!(
        out.contains("PS[00].lfFooter=0,0,0,0,0,0,0,0,0,0,0,0,0"),
        "{out}"
    );
    assert!(out.contains("PS[00].nFooterPointSize=0"), "{out}");
    assert!(out.contains("PS[00].lfFooterFaceName=\r\n"), "{out}");
    let back = PrintSettings::from_profile(&PrintSetting::default(), &p);
    assert_eq!(back.get(0).footer_font, None);
}

#[test]
fn the_current_setting_is_the_number_in_the_types_nints() {
    let mut t = TypeSettings::default();
    assert_eq!(current_of(&t), 0);
    t.current_print_setting = 5;
    assert_eq!(current_of(&t), 5);
    t.current_print_setting = 8;
    assert_eq!(current_of(&t), 0);
    t.current_print_setting = -1;
    assert_eq!(current_of(&t), 0);
}

#[test]
fn storing_the_current_setting_changes_only_that_number() {
    let mut p = profile(
        "[Types(1)]\r\nnInts=1, 120, 0, 4, 3, 2, 0, 1, 1, 0, 0, 0\r\nszTypeName=テキスト\r\n",
    );
    assert!(store_current(&mut p, 1, 3).unwrap());
    let out = text(&p);
    assert!(out.contains("nInts=1,120,0,4,3,2,0,1,1,0,3,0"), "{out}");
    assert!(out.contains("szTypeName=テキスト"), "{out}");
    // the same number again changes nothing
    assert!(!store_current(&mut p, 1, 3).unwrap());
    let types = DocumentTypes::from_profile(&p);
    assert_eq!(types.get(1).unwrap().current_print_setting, 3);
}

#[test]
fn storing_for_a_type_without_a_section_writes_the_type() {
    let mut p = Profile::new();
    assert!(store_current(&mut p, 0, 2).unwrap());
    let types = DocumentTypes::from_profile(&p);
    assert_eq!(types.get(0).unwrap().current_print_setting, 2);
    // no such type: nothing
    assert!(!store_current(&mut p, 9999, 2).unwrap());
}
