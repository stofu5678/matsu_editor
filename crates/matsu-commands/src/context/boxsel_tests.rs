//! Tests of the setting 「矩形選択移動で選択をロックする」 (`m_bBoxSelectLock`): `Sub_BoxSelectLock`
//! (`sakura_core/cmd/CViewCommander.cpp`) begins a box with `Command_BEGIN_BOXSELECT(bSelLock)`,
//! where `bSelLock` is the setting; a plain move reads the lock (`HandleCommand`, `F_RIGHT`:
//! `Command_RIGHT(m_bSelectingLock, ...)`), so with the lock the plain moves go on extending the
//! box, and without it the first plain move ends it.

use crate::edit_options::{self, EditOptions};
use crate::testing::Editor;

/// The box commands of the commands crate. The paging ones (`F_1PageUp_BOX`, ...) need the
/// window and are tested in `matsu-ui`; they begin the box with the same `ensure_box`.
const BOX_MOVES: &[&str] = &[
    "cursor_up_box",
    "cursor_down_box",
    "cursor_left_box",
    "cursor_right_box",
    "cursor_up2_box",
    "cursor_down2_box",
    "word_left_box",
    "word_right_box",
    "go_line_top_box",
    "go_line_end_box",
    "go_logical_line_top_box",
    "go_file_top_box",
    "go_file_end_box",
];

fn with_lock<R>(lock: bool, f: impl FnOnce() -> R) -> R {
    edit_options::with_options(
        EditOptions {
            box_select_lock: lock,
            ..EditOptions::default()
        },
        f,
    )
}

/// A text where every box move above leaves the caret on a row from which a plain move down
/// still moves.
const TEXT: &str = "abc def\nab|c def\nabc def\nabc def\nabc def\nabc def\n";

#[test]
fn the_default_locks_the_box() {
    assert!(EditOptions::default().box_select_lock);
}

#[test]
fn with_the_lock_a_plain_move_extends_the_box_begun_by_every_box_move() {
    with_lock(true, || {
        for id in BOX_MOVES {
            let mut e = Editor::new(TEXT);
            e.run(id);
            assert!(e.cursor().is_box() && e.cursor().selecting_lock(), "{id}");
            let row = e.cursor().caret().line;
            e.run("cursor_down");
            assert!(e.cursor().is_box(), "{id}: the box goes on");
            // at the end of the file the plain move has nowhere to go
            if *id != "go_file_end_box" {
                assert!(e.cursor().caret().line > row, "{id}: the plain move moved");
            }
        }
    });
}

#[test]
fn without_the_lock_a_plain_move_ends_the_box_begun_by_every_box_move() {
    with_lock(false, || {
        for id in BOX_MOVES {
            let mut e = Editor::new(TEXT);
            e.run(id);
            assert!(e.cursor().is_box() && !e.cursor().selecting_lock(), "{id}");
            let row = e.cursor().caret().line;
            e.run("cursor_down");
            assert!(!e.cursor().is_box(), "{id}: the box is gone");
            assert!(!e.cursor().has_selection(), "{id}: nothing is selected");
            // at the end of the file the plain move has nowhere to go
            if *id != "go_file_end_box" {
                assert!(e.cursor().caret().line > row, "{id}: the plain move moved");
            }
        }
    });
}

#[test]
fn without_the_lock_the_box_moves_still_extend_the_box() {
    with_lock(false, || {
        let mut e = Editor::new("abcd\nef|gh\nijkl\n");
        e.run("cursor_down_box").assert_box((1, 2), (2, 2));
        e.run("cursor_right_box").assert_box((1, 2), (2, 3));
        e.run("cursor_up_box").assert_box((1, 2), (1, 3));
    });
}

#[test]
fn begin_box_always_locks_whatever_the_setting_is() {
    // `F_BEGIN_BOX` is `Command_BEGIN_BOXSELECT(true)`
    for lock in [true, false] {
        with_lock(lock, || {
            let mut e = Editor::new("ab|cd\nefgh\n");
            e.run("begin_box");
            assert!(e.cursor().is_box() && e.cursor().selecting_lock(), "{lock}");
            e.run("cursor_down");
            e.assert_box((0, 2), (1, 2));
        });
    }
}

#[test]
fn a_box_that_is_going_on_keeps_its_lock_when_the_setting_changes() {
    // `Sub_BoxSelectLock` begins a box only when there is none
    let mut e = Editor::new("abcd\nef|gh\nijkl\nmnop\n");
    with_lock(true, || {
        e.run("cursor_down_box");
    });
    with_lock(false, || {
        e.run("cursor_right_box");
        assert!(e.cursor().selecting_lock());
        e.run("cursor_down");
        e.assert_box((1, 2), (3, 3));
    });
}

#[test]
fn a_box_that_was_begun_unlocked_stays_unlocked_when_the_setting_changes() {
    let mut e = Editor::new("abcd\nef|gh\nijkl\nmnop\n");
    with_lock(false, || {
        e.run("cursor_down_box");
    });
    with_lock(true, || {
        e.run("cursor_right_box");
        assert!(!e.cursor().selecting_lock());
        e.run("cursor_down");
        assert!(!e.cursor().is_box());
    });
}

#[test]
fn a_normal_selection_is_dropped_and_the_box_gets_the_setting() {
    for lock in [true, false] {
        with_lock(lock, || {
            let mut e = Editor::new("ab\ncd\ne[f|g");
            e.run("cursor_up_box");
            assert!(e.cursor().is_box());
            assert_eq!(e.cursor().selecting_lock(), lock);
        });
    }
}
