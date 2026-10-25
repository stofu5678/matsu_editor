use super::*;
use crate::settings::DocumentTypes;
use crate::settings::type_list::{add_type, delete_type};

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn no_files(_: &str) -> Option<Vec<u8>> {
    None
}

/// A type file as Sakura writes it, by hand.
const SAKURA_FILE: &str = "; タイプ別設定 Ver1\r\n\r\n\
    [Info]\r\nszVersion=2.4.2.0\r\nvStructureVersion=183\r\n\r\n\
    [TypeEx]\r\nszKeyword[1]=日本語の単語\r\nszKeywordCase[1]=1\r\nszKeywordFile[1]=日本語の単語.kwd\r\n\r\n\
    [Types]\r\nid=7\r\nnInts=9,90,0,2,-1,-1,0,0,0,0,0,0\r\nszTypeName=日本語タイプ\r\nszTypeExts=jp,nihongo\r\n\
    szLineComment=＃\r\nnLineCommentColumn=-1\r\nC[TXT]=1,0,0000ff,ffffff,0\r\n";

/// Sakura writes UTF-8 with a BOM.
fn utf8(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

fn sakura_file() -> TypeFile {
    TypeFile::parse(&utf8(SAKURA_FILE)).unwrap()
}

#[test]
fn a_file_without_the_structure_version_is_not_a_type_file() {
    assert!(TypeFile::parse(b"[Types]\r\nszTypeName=A\r\n").is_none());
    let file = sakura_file();
    assert_eq!(file.structure_version(), 183);
    assert_eq!(file.version(), "2.4.2.0");
    assert_eq!(file.type_name(), "日本語タイプ");
}

#[test]
fn a_file_sakura_wrote_is_added_as_a_new_type() {
    let mut p = Profile::new();
    let words =
        "// 日本語の単語 キーワード定義ファイル\r\n// CASE=True\r\n\r\nりんご\r\nみかん\r\n";
    let mut asked = Vec::new();
    let imported = import_type(
        &mut p,
        &sakura_file(),
        Target::Add,
        Colors::AsFile,
        &mut |name| {
            asked.push(name.to_owned());
            let mut bytes = vec![0xEF, 0xBB, 0xBF];
            bytes.extend_from_slice(words.as_bytes());
            Some(bytes)
        },
    )
    .unwrap();
    assert_eq!(imported.index, 36);
    assert_eq!(
        imported.keyword_files,
        [("日本語の単語.kwd".to_owned(), true)]
    );
    assert_eq!(asked, ["日本語の単語.kwd"]);
    let types = DocumentTypes::from_profile(&p);
    assert_eq!(types.len(), 37);
    let t = types.get(36).unwrap();
    assert_eq!(t.type_name, "日本語タイプ");
    assert_eq!(t.type_exts, "jp,nihongo");
    assert_eq!(t.max_line_ketas, 90);
    assert_eq!(t.line_comments[0].delimiter, "＃");
    assert_eq!(t.idx, 36);
    assert_eq!(t.colors.get("TXT").unwrap().text, 0xff);
    // The id is new, not the one in the file.
    assert_ne!(t.id, 7);
    // The keyword set was added by name, with the case rule and the words of its file.
    let set = types.keyword_sets_of(36);
    assert_eq!(set.len(), 1);
    assert_eq!(set[0].1.name, "日本語の単語");
    assert!(set[0].1.case_sensitive);
    assert!(set[0].1.contains("りんご") && set[0].1.contains("みかん"));
    assert_eq!(p.get_i32("Other", "nTypesCount"), Some(37));
}

#[test]
fn a_set_the_user_has_is_found_by_name() {
    let mut p = Profile::new();
    let text = "[Info]\r\nvStructureVersion=183\r\n[TypeEx]\r\nszKeyword[1]=Python\r\n\
                szKeyword[2]=HTML\r\n[Types]\r\nszTypeName=A\r\nszTypeExts=a\r\n";
    let file = TypeFile::parse(text.as_bytes()).unwrap();
    let before = KeywordSets::from_profile(&p);
    let imported = import_type(&mut p, &file, Target::Add, Colors::AsFile, &mut no_files).unwrap();
    assert!(imported.keyword_files.is_empty());
    let types = DocumentTypes::from_profile(&p);
    let t = types.get(imported.index).unwrap();
    assert_eq!(
        t.keyword_set_idx[0],
        i32::try_from(before.index_of("Python").unwrap()).unwrap()
    );
    assert_eq!(
        t.keyword_set_idx[1],
        i32::try_from(before.index_of("HTML").unwrap()).unwrap()
    );
    // The slots the file names nothing for have no set.
    assert!(t.keyword_set_idx[2..].iter().all(|&i| i == -1));
    // No set was added, so the sets were not written.
    assert_eq!(p.get("KeyWords", "nKeyWordSetNum"), None);
}

#[test]
fn a_type_file_can_replace_a_type() {
    let mut p = Profile::new();
    let old_id = DocumentTypes::from_profile(&p).get(2).unwrap().id;
    let imported = import_type(
        &mut p,
        &sakura_file(),
        Target::Overwrite(2),
        Colors::AsFile,
        &mut no_files,
    )
    .unwrap();
    assert_eq!(imported.index, 2);
    let types = DocumentTypes::from_profile(&p);
    assert_eq!(types.len(), 36);
    let t = types.get(2).unwrap();
    assert_eq!(t.type_name, "日本語タイプ");
    assert_eq!(t.idx, 2);
    assert_ne!(t.id, old_id);
    // The built-in C/C++ is gone: everything not in the file is the basic type's.
    assert!(t.block_comments[1].from.is_empty());
    assert_eq!(types.type_index_for_file_name("a.nihongo"), 2);
    assert_eq!(types.type_index_for_file_name("a.cpp"), 0);
}

#[test]
fn the_basic_type_keeps_its_name_extensions_and_id() {
    let mut p = Profile::new();
    import_type(
        &mut p,
        &sakura_file(),
        Target::Overwrite(0),
        Colors::AsFile,
        &mut no_files,
    )
    .unwrap();
    let basic = DocumentTypes::from_profile(&p).get(0).unwrap().clone();
    assert_eq!(
        (basic.type_name.as_str(), basic.type_exts.as_str(), basic.id),
        ("基本", "", 0)
    );
    assert_eq!(basic.max_line_ketas, 90);
}

#[test]
fn the_colors_come_from_the_file_a_type_or_a_color_file_but_the_show_flag_stays() {
    let file = TypeFile::parse(
        "[Info]\r\nvStructureVersion=183\r\n[Types]\r\nszTypeName=A\r\nC[CMT]=0,0,111111,ffffff,0\r\n\
         C[URL]=1,0,222222,ffffff,0\r\n"
            .as_bytes(),
    )
    .unwrap();
    let mut p = Profile::new();
    // The colors of type 2 (C/C++).
    let cpp = DocumentTypes::builtin().get(2).unwrap().colors.clone();
    let i = import_type(&mut p, &file, Target::Add, Colors::OfType(2), &mut no_files)
        .unwrap()
        .index;
    let t = DocumentTypes::from_profile(&p).get(i).unwrap().clone();
    assert_eq!(
        t.colors.get("CMT").unwrap().text,
        cpp.get("CMT").unwrap().text
    );
    // `disp` is the file's.
    assert!(!t.colors.get("CMT").unwrap().disp);
    assert!(t.colors.get("URL").unwrap().disp);
    // As the file has them.
    let i = import_type(&mut p, &file, Target::Add, Colors::AsFile, &mut no_files)
        .unwrap()
        .index;
    let t = DocumentTypes::from_profile(&p).get(i).unwrap().clone();
    assert_eq!(t.colors.get("CMT").unwrap().text, 0x11_1111);
    // A color set that was read.
    let mut given = ColorSet::default();
    given.get_mut("CMT").unwrap().text = 0x33_3333;
    let i = import_type(
        &mut p,
        &file,
        Target::Add,
        Colors::Given(Box::new(given)),
        &mut no_files,
    )
    .unwrap()
    .index;
    let t = DocumentTypes::from_profile(&p).get(i).unwrap().clone();
    assert_eq!(t.colors.get("CMT").unwrap().text, 0x33_3333);
    assert!(!t.colors.get("CMT").unwrap().disp);
}

#[test]
fn a_full_list_has_no_room_for_a_new_type_but_can_replace() {
    let mut text = String::from("[Other]\r\nnTypesCount=60\r\n");
    for i in 17..60 {
        text.push_str(&format!(
            "[Types({i})]\r\nszTypeName=T{i}\r\nszTypeExts=t{i}\r\n"
        ));
    }
    let mut p = profile(&text);
    let file = sakura_file();
    assert_eq!(
        import_type(&mut p, &file, Target::Add, Colors::AsFile, &mut no_files).unwrap_err(),
        ListError::Full
    );
    assert!(
        import_type(
            &mut p,
            &file,
            Target::Overwrite(59),
            Colors::AsFile,
            &mut no_files
        )
        .is_ok()
    );
    assert_eq!(
        import_type(
            &mut p,
            &file,
            Target::Overwrite(60),
            Colors::AsFile,
            &mut no_files
        )
        .unwrap_err(),
        ListError::NoSuchType
    );
}

#[test]
fn a_type_goes_out_and_comes_back_the_same() {
    let p = Profile::new();
    let exported = export_type(&p, 2, "0.3.0").unwrap();
    // The file is UTF-8 with a BOM and starts with the comment line.
    assert!(exported.bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
    let text = String::from_utf8(exported.bytes[3..].to_vec()).unwrap();
    assert!(text.starts_with("; タイプ別設定 Ver1"), "{text}");
    for needle in [
        "[Info]",
        "[Types]",
        "szTypeName=C/C++",
        "vStructureVersion=183",
        "szVersion=0.3.0",
        "[TypeEx]",
        "szKeyword[1]=C/C++",
        "szKeywordFile[1]=C／C++.kwd",
        "szKeywordCase[1]=1",
    ] {
        assert!(text.contains(needle), "{needle}\n{text}");
    }
    assert_eq!(exported.keyword_files.len(), 1);
    assert_eq!(exported.keyword_files[0].file_name, "C／C++.kwd");
    // The new-file keys that the settings file does not set are not exported.
    assert!(!text.contains("eDefaultEoltype"), "{text}");
    let file = TypeFile::parse(&exported.bytes).unwrap();
    assert_eq!(file.type_name(), "C/C++");
    let mut other = Profile::new();
    let kwd = exported.keyword_files[0].bytes.clone();
    let imported = import_type(
        &mut other,
        &file,
        Target::Add,
        Colors::AsFile,
        &mut |name| (name == "C／C++.kwd").then(|| kwd.clone()),
    )
    .unwrap();
    assert_eq!(imported.keyword_files, [("C／C++.kwd".to_owned(), true)]);
    let (a, b) = (
        DocumentTypes::from_profile(&p).get(2).unwrap().clone(),
        DocumentTypes::from_profile(&other)
            .get(imported.index)
            .unwrap()
            .clone(),
    );
    // Everything but the number and the id (the set was found by name, so its number is the same).
    let (mut x, mut y) = (a, b);
    x.idx = 0;
    y.idx = 0;
    x.id = 0;
    y.id = 0;
    assert_eq!(x, y);
}

#[test]
fn keyword_files_of_names_that_differ_only_in_case_get_different_files() {
    use crate::settings::keywords::{SetDraft, store_sets};
    let mut p = Profile::new();
    let set = |name: &str, word: &str| SetDraft {
        name: name.to_owned(),
        case_sensitive: true,
        words: vec![word.to_owned()],
    };
    store_sets(&mut p, &[set("Foo", "a"), set("foo", "b")]).unwrap();
    let i = add_type(&mut p).unwrap();
    p.set(
        &format!("Types({i})"),
        "nInts",
        &format!("{i},10240,0,4,0,1,0,1,1,0,0,0"),
    )
    .unwrap();
    let out = export_type(&p, i, "0.3.0").unwrap();
    assert_eq!(out.keyword_files.len(), 2);
    let lower: Vec<_> = out
        .keyword_files
        .iter()
        .map(|k| k.file_name.to_lowercase())
        .collect();
    assert_ne!(lower[0], lower[1]);
}

#[test]
fn an_exported_type_without_regex_keywords_does_not_get_those_of_the_basic_type() {
    // C/C++ has none; the importer's basic type has one.
    let out = export_type(&Profile::new(), 2, "0.3.0").unwrap();
    let file = TypeFile::parse(&out.bytes).unwrap();
    let mut other = profile("[Types(0)]\r\nRxKey[000]=URL,/x/k\r\n");
    let n = import_type(
        &mut other,
        &file,
        Target::Add,
        Colors::AsFile,
        &mut no_files,
    )
    .unwrap()
    .index;
    let t = DocumentTypes::from_profile(&other).get(n).unwrap().clone();
    assert!(t.regex_keywords.iter().all(|k| k.pattern.is_empty()));
}

#[test]
fn a_type_with_japanese_text_goes_out_and_comes_back() {
    let mut p = Profile::new();
    let i = add_type(&mut p).unwrap();
    let section = format!("Types({i})");
    p.set(&section, "szTypeName", "日本語 タイプ").unwrap();
    p.set(&section, "szTypeExts", "にほん").unwrap();
    p.set(&section, "szLineComment", "※").unwrap();
    let exported = export_type(&p, i, "0.3.0").unwrap();
    assert_eq!(export_file_name("日本語 タイプ"), "日本語 タイプ.ini");
    let file = TypeFile::parse(&exported.bytes).unwrap();
    let mut other = Profile::new();
    let n = import_type(
        &mut other,
        &file,
        Target::Add,
        Colors::AsFile,
        &mut no_files,
    )
    .unwrap()
    .index;
    let t = DocumentTypes::from_profile(&other).get(n).unwrap().clone();
    assert_eq!(t.type_name, "日本語 タイプ");
    assert_eq!(t.type_exts, "にほん");
    assert_eq!(t.line_comments[0].delimiter, "※");
}

#[test]
fn a_file_in_shift_jis_is_read() {
    let (bytes, _, _) = encoding_rs::SHIFT_JIS.encode(
        "; タイプ別設定 Ver1\r\n[Info]\r\nvStructureVersion=183\r\n[Types]\r\nszTypeName=和文\r\nszTypeExts=wa\r\n",
    );
    let file = TypeFile::parse(&bytes).unwrap();
    assert_eq!(file.type_name(), "和文");
    let mut p = Profile::new();
    let n = import_type(&mut p, &file, Target::Add, Colors::AsFile, &mut no_files)
        .unwrap()
        .index;
    assert_eq!(
        DocumentTypes::from_profile(&p).get(n).unwrap().type_exts,
        "wa"
    );
}

#[test]
fn the_new_file_keys_travel_with_the_type_when_the_file_sets_them() {
    let file = TypeFile::parse(
        "[Info]\r\nvStructureVersion=183\r\n[Types]\r\nszTypeName=A\r\neDefaultEoltype=2\r\n"
            .as_bytes(),
    )
    .unwrap();
    let mut p = Profile::new();
    let n = import_type(&mut p, &file, Target::Add, Colors::AsFile, &mut no_files)
        .unwrap()
        .index;
    let keys = TypeSettings::new_file_keys(&p, i32::try_from(n).unwrap());
    assert_eq!(keys.eoltype, Some(2));
    assert_eq!(keys.codetype, None);
    let out = export_type(&p, n, "0.3.0").unwrap();
    assert!(
        String::from_utf8(out.bytes)
            .unwrap()
            .contains("eDefaultEoltype=2")
    );
    // The list is still a list of the same types after a delete.
    delete_type(&mut p, 5).unwrap();
    assert_eq!(DocumentTypes::from_profile(&p).len(), 36);
}

#[test]
fn colors_go_to_a_file_and_come_back() {
    let mut colors = ColorSet::default();
    colors.get_mut("CMT").unwrap().text = 0x12_3456;
    colors.get_mut("KW1").unwrap().bold = true;
    colors.get_mut("URL").unwrap().underline = true;
    let bytes = render_colors(&colors);
    let text = String::from_utf8(bytes[3..].to_vec()).unwrap();
    assert!(text.starts_with("; テキストエディタ色設定 Ver3"), "{text}");
    assert!(text.contains("[SakuraColor]"));
    assert!(text.contains("C[CMT]=1,0,123456,"), "{text}");
    assert_eq!(
        parse_colors(&bytes, &ColorSet::default()),
        Some(colors.clone())
    );
    // Shift_JIS, and the comment mark `//` of the old files.
    let (sjis, _, _) = encoding_rs::SHIFT_JIS.encode(&text);
    assert_eq!(
        parse_colors(&sjis, &ColorSet::default()),
        Some(colors.clone())
    );
    let slashes = text.replacen("; テキスト", "// テキスト", 1);
    assert_eq!(
        parse_colors(&utf8(&slashes), &ColorSet::default()),
        Some(colors)
    );
}

#[test]
fn a_color_file_of_another_format_is_not_read() {
    let base = ColorSet::default();
    assert_eq!(
        parse_colors(b"[SakuraColor]\r\nC[CMT]=1,0,0,0,0\r\n", &base),
        None
    );
    assert_eq!(
        parse_colors("; テキストエディタ色設定 Ver2\r\n".as_bytes(), &base),
        None
    );
    assert_eq!(parse_colors(b"", &base), None);
    assert_eq!(parse_colors(b"x", &base), None);
}

#[test]
fn a_color_that_the_file_lacks_keeps_the_value_it_had() {
    let mut base = ColorSet::default();
    base.get_mut("NUM").unwrap().text = 0x0a_0b0c;
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(
        "; テキストエディタ色設定 Ver3\r\n[SakuraColor]\r\nC[CMT]=1,0,010203,ffffff,0\r\n"
            .as_bytes(),
    );
    let read = parse_colors(&bytes, &base).unwrap();
    assert_eq!(read.get("CMT").unwrap().text, 0x01_0203);
    assert_eq!(read.get("NUM").unwrap().text, 0x0a_0b0c);
}
