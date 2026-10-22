//! The lists of recent paths ([`Extra::Recent`]) are left out where the dialog cannot use them.

use super::*;

fn recent(kind: RecentKind) -> Extra {
    Extra::Recent {
        label: "最近のファイル".to_owned(),
        items: vec!["/a/b.txt".to_owned()],
        kind,
    }
}

fn request(extras: Vec<Extra>) -> Request {
    Request {
        kind: Kind::SaveFile,
        start: None,
        name: None,
        title: String::new(),
        filters: Vec::new(),
        selected_filter: 0,
        default_extension: None,
        extras,
        follow: None,
    }
}

fn combo() -> Extra {
    Extra::Combo {
        label: "c".to_owned(),
        items: vec!["x".to_owned()],
        selected: 0,
    }
}

fn check() -> Extra {
    Extra::Check {
        label: "k".to_owned(),
        checked: false,
        enabled: true,
    }
}

#[test]
fn a_request_without_recent_lists_is_unchanged() {
    let original = request(vec![combo(), check()]);
    let (stripped, shown) = without_recent(original.clone());
    assert_eq!(stripped, original);
    assert_eq!(shown, [true, true]);
}

#[test]
fn recent_lists_are_left_out_and_their_places_are_kept() {
    let (stripped, shown) = without_recent(request(vec![
        combo(),
        check(),
        recent(RecentKind::Files),
        recent(RecentKind::Folders),
    ]));
    assert_eq!(stripped.extras, [combo(), check()]);
    assert_eq!(shown, [true, true, false, false]);
    let chosen = Chosen {
        paths: vec![PathBuf::from("/x")],
        extras: vec![ExtraValue::Combo(0), ExtraValue::Check(true)],
    };
    assert_eq!(
        with_recent_places(chosen, &shown).extras,
        [
            ExtraValue::Combo(0),
            ExtraValue::Check(true),
            ExtraValue::Nothing,
            ExtraValue::Nothing
        ]
    );
}

#[test]
fn a_dialog_that_showed_no_fields_gives_no_values() {
    let shown = [false, false];
    let chosen = Chosen {
        paths: vec![PathBuf::from("/x")],
        extras: Vec::new(),
    };
    assert!(with_recent_places(chosen, &shown).extras.is_empty());
}

#[test]
fn the_follow_indexes_move_with_the_fields() {
    let mut r = request(vec![recent(RecentKind::Files), combo(), check()]);
    r.follow = Some(Follow::new(1, 2, |_| (true, false)));
    let (stripped, _) = without_recent(r);
    let follow = stripped.follow.unwrap();
    assert_eq!((follow.combo, follow.check), (0, 1));
}
