use super::*;
use crate::settings::DocumentTypes;

fn profile(text: &str) -> Profile {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    Profile::from_bytes(&bytes)
}

fn names(profile: &Profile) -> Vec<String> {
    DocumentTypes::from_profile(profile)
        .iter()
        .map(|t| t.type_name.clone())
        .collect()
}

fn count(profile: &Profile) -> i32 {
    profile.get_i32("Other", "nTypesCount").unwrap()
}

/// A file with `n` types where every type past the 17 built-in ones is the user's own, so that the
/// numbers of the list are the numbers of the sections.
fn own_types(n: usize) -> String {
    let mut text = format!("[Other]\r\nnTypesCount={n}\r\n");
    for i in 17..n {
        text.push_str(&format!(
            "[Types({i})]\r\nszTypeName=自分{i}\r\nszTypeExts=x{i}\r\nid={}\r\n",
            1000 + i
        ));
    }
    text
}

#[test]
fn the_names_follow_sakuras_rules() {
    // OnAddTypeSetting: 設定(insert + 1), the next number while a type from number 1 has the name.
    assert_eq!(name_for_added(&["基本", "A"], 2), "設定3");
    assert_eq!(name_for_added(&["基本", "設定3", "設定4"], 2), "設定5");
    // The name of 基本 is not compared.
    assert_eq!(name_for_added(&["設定3", "A"], 2), "設定3");
    // CopyType: a number is added, or the number at the end is made one larger.
    assert_eq!(name_for_copy("Python", &["基本", "Python"]), "Python2");
    assert_eq!(name_for_copy("Python2", &["基本", "Python2"]), "Python3");
    assert_eq!(
        name_for_copy("Python", &["基本", "Python", "Python2", "Python3"]),
        "Python4"
    );
    assert_eq!(name_for_copy("日本語9", &["日本語9"]), "日本語10");
    assert_eq!(name_for_copy("123", &["123"]), "124");
    // The name and the number fit in 63 UTF-16 units.
    let long = "あ".repeat(70);
    let copy = name_for_copy(&long, &[&long]);
    assert_eq!(copy.encode_utf16().count(), 63);
    assert!(copy.ends_with("あ2"));
    // InitializeType: 設定(index + 1), and the next while another type has the name.
    assert_eq!(name_for_initialized(&["基本", "A", "B"], 2), "設定3");
    assert_eq!(name_for_initialized(&["基本", "設定3", "B"], 2), "設定4");
    // The type itself is not compared.
    assert_eq!(name_for_initialized(&["基本", "A", "設定3"], 2), "設定3");
}

#[test]
fn a_type_is_added_at_the_end_as_a_copy_of_the_basic_type() {
    let mut p = profile("[Types(0)]\r\nnInts=0,80,0,8,-1,-1,0,0,0,0,0,0\r\n");
    let before = names(&p);
    assert_eq!(before.len(), 36);
    let index = add_type(&mut p).unwrap();
    assert_eq!(index, 36);
    assert_eq!(count(&p), 37);
    let types = DocumentTypes::from_profile(&p);
    assert_eq!(types.len(), 37);
    let added = types.get(36).unwrap();
    assert_eq!(added.type_name, "設定37");
    assert_eq!(added.type_exts, "");
    assert_eq!(added.idx, 36);
    // A copy of the basic type as it is now.
    assert_eq!(added.tab_space, 8);
    assert_eq!(added.max_line_ketas, 80);
    // The other types are as they were.
    assert_eq!(names(&p)[..36], before[..]);
    // Every id is its own.
    let mut ids: Vec<_> = types.iter().map(|t| t.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 37);
    assert_eq!(p.get("Types(36)", "szTypeName"), Some("設定37"));
}

#[test]
fn the_names_of_added_types_do_not_repeat() {
    let mut p = profile(&own_types(20));
    p.set("Types(18)", "szTypeName", "設定21").unwrap();
    assert_eq!(add_type(&mut p).unwrap(), 20);
    assert_eq!(names(&p)[20], "設定22");
}

#[test]
fn the_limit_is_60_types() {
    let mut p = profile(&own_types(60));
    assert_eq!(add_type(&mut p), Err(ListError::Full));
    assert_eq!(copy_type(&mut p, 2), Err(ListError::Full));
    assert_eq!(count(&p), 60);
    let mut p = profile(&own_types(59));
    assert_eq!(add_type(&mut p), Ok(59));
    assert_eq!(count(&p), 60);
}

#[test]
fn a_list_of_exactly_30_types_cannot_be_written() {
    // nTypesCount=30 is read as 36.
    let mut p = profile(&own_types(31));
    assert_eq!(delete_type(&mut p, 20), Err(ListError::ThirtyTypes));
    assert_eq!(count(&p), 31);
    let mut p = profile(&own_types(29));
    assert_eq!(add_type(&mut p), Err(ListError::ThirtyTypes));
    assert_eq!(copy_type(&mut p, 2), Err(ListError::ThirtyTypes));
}

#[test]
fn a_copy_keeps_the_extensions_and_gets_a_numbered_name_and_its_own_id() {
    let mut p = profile(&own_types(20));
    let index = copy_type(&mut p, 18).unwrap();
    assert_eq!(index, 20);
    let types = DocumentTypes::from_profile(&p);
    let copy = types.get(20).unwrap();
    // 自分19 is taken: the number grows until the name is free.
    assert_eq!(copy.type_name, "自分20");
    assert_eq!(copy.type_exts, "x18");
    assert_ne!(copy.id, types.get(18).unwrap().id);
    copy_type(&mut p, 18).unwrap();
    assert_eq!(names(&p)[21], "自分21");
    copy_type(&mut p, 20).unwrap();
    assert_eq!(names(&p)[22], "自分22");
}

#[test]
fn a_copy_of_a_builtin_type_has_all_its_values() {
    let mut p = Profile::new();
    let index = copy_type(&mut p, 2).unwrap();
    let types = DocumentTypes::from_profile(&p);
    let (cpp, copy) = (types.get(2).unwrap(), types.get(index).unwrap());
    assert_eq!(copy.type_name, "C/C++2");
    assert_eq!(copy.type_exts, cpp.type_exts);
    assert_eq!(copy.block_comments, cpp.block_comments);
    assert_eq!(copy.line_comments, cpp.line_comments);
    assert_eq!(copy.keyword_set_idx, cpp.keyword_set_idx);
}

#[test]
fn deleting_a_type_moves_the_ones_after_it_up() {
    let mut p = profile(&own_types(22));
    let before = DocumentTypes::from_profile(&p);
    delete_type(&mut p, 18).unwrap();
    assert_eq!(count(&p), 21);
    let after = DocumentTypes::from_profile(&p);
    assert_eq!(after.len(), 21);
    for i in 0..18 {
        assert_eq!(after.get(i), before.get(i), "{i}");
    }
    for i in 18..21 {
        let (a, b) = (after.get(i).unwrap(), before.get(i + 1).unwrap());
        assert_eq!(
            (&a.type_name, &a.type_exts, a.id),
            (&b.type_name, &b.type_exts, b.id)
        );
        assert_eq!(a.idx, i32::try_from(i).unwrap());
    }
    // The section of the last number is gone.
    assert_eq!(p.entries("Types(21)").count(), 0);
    assert_eq!(p.get("Types(20)", "szTypeName"), Some("自分21"));
}

#[test]
fn deleting_a_builtin_type_keeps_the_types_after_it_as_they_were() {
    let mut p = Profile::new();
    let before = DocumentTypes::from_profile(&p);
    delete_type(&mut p, 3).unwrap();
    let after = DocumentTypes::from_profile(&p);
    assert_eq!(after.len(), before.len() - 1);
    assert_eq!(after.get(3).unwrap().type_name, "PL/SQL");
    assert_eq!(
        after.get(3).unwrap().type_exts,
        before.get(4).unwrap().type_exts
    );
    assert_eq!(after.get(15).unwrap().type_name, "設定ファイル");
    assert_eq!(
        after.type_index_for_file_name("a.pls"),
        before.type_index_for_file_name("a.pls") - 1
    );
    // The HTML type is gone, so its files are the basic type's.
    assert_eq!(after.type_index_for_file_name("a.html"), 0);
}

#[test]
fn the_basic_type_cannot_be_deleted_or_moved() {
    let mut p = profile(&own_types(20));
    assert_eq!(delete_type(&mut p, 0), Err(ListError::NotAllowed));
    assert_eq!(move_type(&mut p, 0, false), Err(ListError::NotAllowed));
    assert_eq!(move_type(&mut p, 0, true), Err(ListError::NotAllowed));
    // Nothing passes it: the button 上へ is for numbers from 2.
    assert_eq!(move_type(&mut p, 1, true), Err(ListError::NotAllowed));
    assert_eq!(move_type(&mut p, 19, false), Err(ListError::NotAllowed));
    assert_eq!(delete_type(&mut p, 20), Err(ListError::NoSuchType));
    assert_eq!(names(&p)[0], "基本");
}

#[test]
fn a_type_moves_up_and_down_with_its_settings_and_id() {
    let mut p = profile(&own_types(20));
    let before = DocumentTypes::from_profile(&p);
    assert_eq!(move_type(&mut p, 18, true), Ok(17));
    let moved = DocumentTypes::from_profile(&p);
    assert_eq!(moved.get(17).unwrap().type_name, "自分18");
    assert_eq!(moved.get(17).unwrap().id, before.get(18).unwrap().id);
    assert_eq!(moved.get(17).unwrap().idx, 17);
    assert_eq!(moved.get(18).unwrap().type_name, "自分17");
    assert_eq!(moved.get(18).unwrap().idx, 18);
    assert_eq!(count(&p), 20);
    // The other way brings it back.
    assert_eq!(move_type(&mut p, 17, false), Ok(18));
    let back = DocumentTypes::from_profile(&p);
    for i in 0..20 {
        let (a, b) = (back.get(i).unwrap(), before.get(i).unwrap());
        assert_eq!(
            (&a.type_name, &a.type_exts, a.id),
            (&b.type_name, &b.type_exts, b.id)
        );
    }
}

#[test]
fn a_builtin_type_moves_with_its_built_in_values() {
    let mut p = Profile::new();
    assert_eq!(move_type(&mut p, 3, true), Ok(2));
    let types = DocumentTypes::from_profile(&p);
    assert_eq!(types.get(2).unwrap().type_name, "HTML");
    assert_eq!(types.get(3).unwrap().type_name, "C/C++");
    assert_eq!(types.get(3).unwrap().block_comments[1].from, "#if 0");
    assert_eq!(types.type_index_for_file_name("a.cpp"), 3);
    assert_eq!(types.type_index_for_file_name("a.html"), 2);
    // The first type that can move up is number 2.
    assert_eq!(move_type(&mut p, 2, true), Ok(1));
    assert_eq!(names(&p)[1], "HTML");
}

#[test]
fn a_moved_type_keeps_its_own_keys() {
    let mut p = profile(
        "[Other]\r\nnTypesCount=20\r\n\
         [Types(18)]\r\nszTypeName=A\r\nszTypeExts=a\r\nsomethingElse=keep\r\neDefaultEoltype=2\r\n\
         [Types(19)]\r\nszTypeName=B\r\nszTypeExts=b\r\n",
    );
    assert_eq!(move_type(&mut p, 19, true), Ok(18));
    // B is at 18 now, with none of A's keys; A is at 19 with all of its keys.
    assert_eq!(p.get("Types(18)", "szTypeName"), Some("B"));
    assert_eq!(p.get("Types(19)", "szTypeName"), Some("A"));
    assert_eq!(p.get("Types(19)", "somethingElse"), Some("keep"));
    assert_eq!(p.get("Types(19)", "eDefaultEoltype"), Some("2"));
    assert_eq!(p.get("Types(18)", "somethingElse"), None);
    // B did not set the new-file keys, so it still does not.
    for key in ["eDefaultCodetype", "eDefaultEoltype", "bDefaultBom"] {
        assert_eq!(p.get("Types(18)", key), None, "{key}");
    }
    assert_eq!(
        TypeSettings::new_file_keys(&p, 18),
        crate::settings::NewFileKeys::default()
    );
}

#[test]
fn a_type_is_reset_to_a_copy_of_the_basic_type_with_a_numbered_name() {
    let mut p = profile(&own_types(20));
    p.set("Types(0)", "nInts", "0,90,0,6,-1,-1,0,0,0,0,0,0")
        .unwrap();
    let old_id = DocumentTypes::from_profile(&p).get(18).unwrap().id;
    assert_eq!(reset_type(&mut p, 18).unwrap(), "設定19");
    let types = DocumentTypes::from_profile(&p);
    let t = types.get(18).unwrap();
    assert_eq!((t.type_name.as_str(), t.type_exts.as_str()), ("設定19", ""));
    assert_eq!(t.tab_space, 6);
    assert_ne!(t.id, old_id);
    assert_eq!(types.len(), 20);
    assert_eq!(names(&p)[19], "自分19");
    // A builtin type too: it is not its built-in self any more.
    assert_eq!(reset_type(&mut p, 2).unwrap(), "設定3");
    let t = DocumentTypes::from_profile(&p);
    assert_eq!(t.get(2).unwrap().type_exts, "");
    assert!(t.get(2).unwrap().block_comments[1].from.is_empty());
    assert_eq!(t.type_index_for_file_name("a.cpp"), 0);
}

#[test]
fn the_basic_type_goes_back_to_the_built_in_settings() {
    let mut p = profile("[Types(0)]\r\nnInts=0,90,0,6,-1,-1,0,0,0,0,0,0\r\nfoo=bar\r\n");
    assert_eq!(reset_type(&mut p, 0).unwrap(), "基本");
    let basic = DocumentTypes::builtin().get(0).unwrap().clone();
    let now = DocumentTypes::from_profile(&p);
    assert_eq!(now.get(0).unwrap().tab_space, basic.tab_space);
    assert_eq!(now.get(0).unwrap().id, 0);
    assert_eq!(p.get("Types(0)", "foo"), None);
    assert_eq!(reset_type(&mut p, 99), Err(ListError::NoSuchType));
}

#[test]
fn a_file_without_a_count_gets_the_count_of_the_list() {
    let mut p = Profile::new();
    delete_type(&mut p, 20).unwrap();
    assert_eq!(count(&p), 35);
    assert_eq!(DocumentTypes::from_profile(&p).len(), 35);
}
