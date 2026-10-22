//! How the fields of a save dialog ([`Request::extras`](super::Request::extras)) are grouped for the
//! panel of macOS, which lays the groups out side by side. Apart from AppKit so that it is tested
//! everywhere.

use super::Extra;

/// The indices of `extras` that go together, in the order of the request: a list starts a group,
/// the check boxes after it join that group (Sakura's BOM box belongs to the encoding list, as
/// on Windows), a check box before any list is a group of its own, and a [`Extra::Recent`] is in
/// none (the panel does not show it).
pub(super) fn groups(extras: &[Extra]) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (i, extra) in extras.iter().enumerate() {
        match extra {
            Extra::Combo { .. } => groups.push(vec![i]),
            Extra::Check { .. } => match groups.last_mut() {
                Some(group) => group.push(i),
                None => groups.push(vec![i]),
            },
            Extra::Recent { .. } => {}
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::super::RecentKind;
    use super::*;

    fn combo() -> Extra {
        Extra::Combo {
            label: "list".to_owned(),
            items: vec!["a".to_owned()],
            selected: 0,
        }
    }

    fn check() -> Extra {
        Extra::Check {
            label: "box".to_owned(),
            checked: false,
            enabled: true,
        }
    }

    fn recent() -> Extra {
        Extra::Recent {
            label: "recent".to_owned(),
            items: Vec::new(),
            kind: RecentKind::Files,
        }
    }

    #[test]
    fn the_box_stays_with_the_encoding_and_the_line_ending_is_beside_it() {
        // 文字コードセット, BOM, 改行コード
        assert_eq!(groups(&[combo(), check(), combo()]), [vec![0, 1], vec![2]]);
    }

    #[test]
    fn a_box_before_any_list_is_alone_and_nothing_gives_no_group() {
        assert_eq!(groups(&[check(), combo()]), [vec![0], vec![1]]);
        assert!(groups(&[]).is_empty());
    }

    #[test]
    fn recent_lists_are_in_no_group() {
        assert_eq!(
            groups(&[recent(), combo(), check(), recent()]),
            [vec![1, 2]]
        );
    }
}
