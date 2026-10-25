//! OK of a sheet whose type moved or was deleted in the list of types while the sheet was open.

use gpui::TestAppContext;
use matsu_config::settings::type_list::{delete_type, move_type};

use super::*;

/// 20 types: the three after the 17 built-in ones are the user's own, each with its id.
fn own_types() -> String {
    let mut text = String::from("[Other]\r\nnTypesCount=20\r\n");
    for i in 17..20 {
        text.push_str(&format!(
            "[Types({i})]\r\nszTypeName=自分{i}\r\nszTypeExts=x{i}\r\nid={}\r\n\
             nInts={i},10240,0,4,-1,-1,0,1,1,0,0,0\r\n",
            1000 + i
        ));
    }
    text
}

/// What another window does to the list while the sheet is open.
fn change_list(ini: &Ini, change: impl FnOnce(&mut Profile)) {
    let mut profile = ini.profile();
    change(&mut profile);
    profile.save(&ini.path).unwrap();
}

#[gpui::test]
fn ok_writes_to_the_type_even_when_it_moved_in_the_list(cx: &mut TestAppContext) {
    let (ini, _a) = setup(cx, &own_types());
    cx.update(|app| open(app, 19, None, None)).unwrap();
    type_in(cx, TAB_SPACE, "8");
    change_list(&ini, |p| {
        move_type(p, 19, true).unwrap();
    });
    ok(cx);
    // The type (id 1019) is number 18 now and has the tab width; the one that took its number is untouched.
    assert_eq!(ini.get("Types(18)", "id").as_deref(), Some("1019"));
    assert_eq!(ints(&ini, 18)[3], 8);
    assert_eq!(ints(&ini, 19)[3], 4);
}

#[gpui::test]
fn ok_writes_nothing_when_the_type_was_deleted(cx: &mut TestAppContext) {
    let (ini, _a) = setup(cx, &own_types());
    cx.update(|app| open(app, 18, None, None)).unwrap();
    type_in(cx, TAB_SPACE, "8");
    change_list(&ini, |p| {
        delete_type(p, 18).unwrap();
    });
    let before = ini.profile().to_bytes();
    ok(cx);
    assert_eq!(ini.profile().to_bytes(), before);
}
