//! Tests of the docking settings of the outline pane: the keys of Sakura's settings file are read
//! and written as Sakura does, and a file that is not changed is not touched.

use matsu_config::Profile;

use super::dock::{
    DockPlace, DockSide, OutlineDock, Target, TypeOutline, write_common, write_place, write_sort,
};

fn profile(text: &str) -> Profile {
    Profile::from_bytes(text.as_bytes())
}

const SAKURA: &str = "[Common]\r\nnOutlineDockSet=1\r\nbOutlineDockSync=0\r\nbOutlineDockDisp=1\r\n\
     eOutlineDockSide=1\r\nxyOutlineDock=100,200,300,400\r\n\
     [Types(2)]\r\nbOutlineDockDisp=1\r\neOutlineDockSide=4\r\nxyOutlineDock=0,0,0,150\r\n\
     nOutlineSortCol=2\r\nbOutlineSortDesc=1\r\nnOutlineSortType=3\r\n";

#[test]
fn a_file_without_the_keys_gives_the_defaults() {
    let dock = OutlineDock::from_profile(&Profile::new());
    // CShareData::InitShareData: not inherited from the types, in sync, closed; MATSU docks at
    // the right where Sakura floats
    assert_eq!(dock.dock_set, 0);
    assert!(dock.sync);
    assert_eq!(dock.common, DockPlace::default());
    assert_eq!(dock.common.side, DockSide::Right);
    assert!(!dock.common.disp);
    assert!(dock.types.is_empty());
    assert_eq!(dock.type_outline(5), TypeOutline::default());
}

#[test]
fn the_keys_of_common_and_of_a_type_are_read() {
    let dock = OutlineDock::from_profile(&profile(SAKURA));
    assert_eq!(dock.dock_set, 1);
    assert!(!dock.sync);
    assert_eq!(
        dock.common,
        DockPlace {
            disp: true,
            side: DockSide::Left,
            left: 100,
            top: 200,
            right: 300,
            bottom: 400,
        }
    );
    let t = dock.type_outline(2);
    assert_eq!(t.place.side, DockSide::Bottom);
    assert_eq!((t.place.bottom, t.place.right), (150, 0));
    assert_eq!((t.sort_col, t.sort_desc, t.sort_type), (2, true, 3));
    // the types the file does not have keep the defaults
    assert_eq!(dock.type_outline(3), TypeOutline::default());
}

#[test]
fn the_set_in_use_decides_where_the_place_comes_from() {
    let mut dock = OutlineDock::from_profile(&profile(SAKURA));
    // nOutlineDockSet=1: the type
    assert_eq!(dock.target(2), Target::Type(2));
    assert_eq!(dock.place(2).side, DockSide::Bottom);
    // a type with no section has the defaults, not [Common]
    assert_eq!(dock.place(7).side, DockSide::Right);
    dock.dock_set = 0;
    assert_eq!(dock.target(2), Target::Common);
    assert_eq!(dock.place(2).side, DockSide::Left);
}

#[test]
fn a_side_or_a_rect_that_sakura_would_not_read_is_the_default() {
    let dock = OutlineDock::from_profile(&profile(
        "[Common]\r\neOutlineDockSide=9\r\nxyOutlineDock=1,2,3\r\n[Types(1)]\r\nxyOutlineDock=a,b,c,d\r\n",
    ));
    assert_eq!(dock.common, DockPlace::default());
    assert_eq!(dock.type_outline(1).place, DockPlace::default());
    // `IOProfileData<RECT>` reads %d,%d,%d,%d with spaces allowed
    let dock = OutlineDock::from_profile(&profile("[Common]\r\nxyOutlineDock=1, 2 ,3,4\r\n"));
    assert_eq!(
        (
            dock.common.left,
            dock.common.top,
            dock.common.right,
            dock.common.bottom
        ),
        (1, 2, 3, 4)
    );
}

#[test]
fn every_side_has_the_number_of_sakura() {
    for (side, code) in [
        (DockSide::Undockable, -1),
        (DockSide::Float, 0),
        (DockSide::Left, 1),
        (DockSide::Top, 2),
        (DockSide::Right, 3),
        (DockSide::Bottom, 4),
    ] {
        assert_eq!(side.code(), code);
        assert_eq!(DockSide::from_code(code), side);
    }
    // IsDocking
    assert!(!DockSide::Float.is_docked() && !DockSide::Undockable.is_docked());
    assert!(DockSide::Bottom.is_docked());
}

#[test]
fn what_was_read_is_written_back_without_touching_the_file() {
    let mut p = profile(SAKURA);
    let before = p.to_bytes();
    let dock = OutlineDock::from_profile(&p);
    assert!(!write_common(&mut p, &dock).unwrap());
    assert!(!write_place(&mut p, Target::Common, &dock.common).unwrap());
    assert!(!write_place(&mut p, Target::Type(2), &dock.type_outline(2).place).unwrap());
    assert!(!write_sort(&mut p, 2, &dock.type_outline(2)).unwrap());
    assert_eq!(p.to_bytes(), before);
}

#[test]
fn a_changed_place_is_written_with_the_keys_of_sakura() {
    let mut p = Profile::new();
    let place = DockPlace {
        disp: true,
        side: DockSide::Top,
        left: 0,
        top: 123,
        right: 0,
        bottom: 0,
    };
    assert!(write_place(&mut p, Target::Common, &place).unwrap());
    assert_eq!(p.get("Common", "bOutlineDockDisp"), Some("1"));
    assert_eq!(p.get("Common", "eOutlineDockSide"), Some("2"));
    assert_eq!(p.get("Common", "xyOutlineDock"), Some("0,123,0,0"));
    // a type has the same keys in its own section
    assert!(write_place(&mut p, Target::Type(4), &place).unwrap());
    assert_eq!(p.get("Types(4)", "eOutlineDockSide"), Some("2"));
    // and the file reads back as it was written
    let dock = OutlineDock::from_profile(&p);
    assert_eq!(dock.common, place);
    assert_eq!(dock.type_outline(4).place, place);
}

#[test]
fn the_sort_and_the_choices_of_common_are_written() {
    let mut p = Profile::new();
    let mut dock = OutlineDock::default();
    dock.set_sort(3, 2, true, 1);
    assert!(write_sort(&mut p, 3, &dock.type_outline(3)).unwrap());
    assert_eq!(p.get("Types(3)", "nOutlineSortCol"), Some("2"));
    assert_eq!(p.get("Types(3)", "bOutlineSortDesc"), Some("1"));
    assert_eq!(p.get("Types(3)", "nOutlineSortType"), Some("1"));
    dock.dock_set = 1;
    dock.sync = false;
    assert!(write_common(&mut p, &dock).unwrap());
    assert_eq!(p.get("Common", "nOutlineDockSet"), Some("1"));
    assert_eq!(p.get("Common", "bOutlineDockSync"), Some("0"));
    assert_eq!(OutlineDock::from_profile(&p), dock);
}

#[test]
fn unifying_copies_the_place_to_common_and_to_every_type() {
    let mut dock = OutlineDock::from_profile(&profile(SAKURA));
    let place = DockPlace {
        disp: true,
        side: DockSide::Left,
        left: 50,
        ..DockPlace::default()
    };
    dock.unify(place, 3);
    assert_eq!(dock.common, place);
    for index in 0..3 {
        assert_eq!(dock.type_outline(index).place, place);
    }
    // the sort of the type stays
    assert_eq!(dock.type_outline(2).sort_col, 2);
    assert!(!dock.types.contains_key(&3));
}

#[test]
fn a_floating_pane_uses_the_width_of_the_right_side() {
    let mut place = DockPlace::default();
    place.set_size(DockSide::Float.size_side(), 222);
    assert_eq!(place.right, 222);
    assert_eq!(place.size_of(DockSide::Undockable.size_side()), 222);
    place.set_size(DockSide::Top, 80);
    assert_eq!((place.top, place.left, place.bottom), (80, 0, 0));
}
