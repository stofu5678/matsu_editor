//! The order of the tabs of a window: which tab comes after a close, and where the tab commands
//! move a tab. Plain numbers, so that it is tested without a window.

/// The tab that becomes active when the active tab `closing` closes. Sakura goes to the tab that
/// was active before ("1つ前のアクティブなタブ", `CAppNodeManager::GetNextTab`,
/// `sakura_core/env/CAppNodeManager.cpp`): `recent` lists the tabs, the most recently active
/// first. `ids` are the tabs of the window in the order of the bar; a tab that is not in the
/// window any more is skipped. Without a history the neighbour on the right, else on the left.
///
/// Returns `None` when `closing` is the only tab.
pub(crate) fn after_close<T: Copy + Eq>(ids: &[T], closing: T, recent: &[T]) -> Option<T> {
    let at = ids.iter().position(|id| *id == closing)?;
    if ids.len() < 2 {
        return None;
    }
    recent
        .iter()
        .copied()
        .find(|id| *id != closing && ids.contains(id))
        .or_else(|| {
            ids.get(at + 1)
                .or_else(|| ids.get(at.wrapping_sub(1)))
                .copied()
        })
}

/// The position a tab at `at` moves to with `F_TAB_MOVERIGHT`; `None` at the right end
/// (`CTabWnd::MoveRight`: `nCount - 1 > nIndex`).
pub(crate) fn right_of(at: usize, count: usize) -> Option<usize> {
    (at + 1 < count).then_some(at + 1)
}

/// The position a tab at `at` moves to with `F_TAB_MOVELEFT`; `None` at the left end
/// (`CTabWnd::MoveLeft`: `0 < nIndex`).
pub(crate) fn left_of(at: usize) -> Option<usize> {
    at.checked_sub(1)
}

/// The tabs `F_TAB_CLOSELEFT` closes: every tab before `at`
/// (`CViewCommander::Command_TAB_CLOSELEFT`).
pub(crate) fn left_range(at: usize) -> std::ops::Range<usize> {
    0..at
}

/// The tabs `F_TAB_CLOSERIGHT` closes: every tab after `at`.
pub(crate) fn right_range(at: usize, count: usize) -> std::ops::Range<usize> {
    (at + 1).min(count)..count
}

/// The window `F_NEXTGROUP` / `F_TAB_JOINTNEXT` goes to: the next of `count` in the order of the
/// windows, the first after the last (`CTabWnd::GetNextGroupWnd`); `None` if there is no other.
pub(crate) fn next_group(at: usize, count: usize) -> Option<usize> {
    (count > 1).then(|| (at + 1) % count)
}

/// The window `F_PREVGROUP` / `F_TAB_JOINTPREV` goes to (`CTabWnd::GetPrevGroupWnd`).
pub(crate) fn prev_group(at: usize, count: usize) -> Option<usize> {
    (count > 1).then(|| (at + count - 1) % count)
}

/// The tab that `F_NEXTWINDOW` goes to inside a window with tabs (`CControlTray::ActiveNextWindow`
/// walks the windows of the group, the first after the last).
pub(crate) fn next_tab(at: usize, count: usize) -> usize {
    (at + 1) % count.max(1)
}

/// The tab that `F_PREVWINDOW` goes to (`CControlTray::ActivePrevWindow`).
pub(crate) fn prev_tab(at: usize, count: usize) -> usize {
    (at + count.max(1) - 1) % count.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_goes_back_to_the_tab_that_was_active_before() {
        // tabs a b c d; d is active, c was active before it, then a
        let ids = ['a', 'b', 'c', 'd'];
        assert_eq!(after_close(&ids, 'd', &['d', 'c', 'a']), Some('c'));
        assert_eq!(after_close(&ids, 'c', &['c', 'a']), Some('a'));
        // a tab that has gone is skipped
        assert_eq!(after_close(&ids, 'd', &['d', 'z', 'b']), Some('b'));
    }

    #[test]
    fn without_a_history_the_right_neighbour_then_the_left() {
        let ids = ['a', 'b', 'c'];
        assert_eq!(after_close(&ids, 'a', &[]), Some('b'));
        assert_eq!(after_close(&ids, 'b', &[]), Some('c'));
        assert_eq!(after_close(&ids, 'c', &[]), Some('b'));
        assert_eq!(after_close(&['a'], 'a', &['a']), None);
        assert_eq!(after_close(&ids, 'x', &[]), None);
    }

    #[test]
    fn moving_a_tab_stops_at_the_ends() {
        assert_eq!(right_of(0, 3), Some(1));
        assert_eq!(right_of(2, 3), None);
        assert_eq!(right_of(0, 1), None);
        assert_eq!(left_of(2), Some(1));
        assert_eq!(left_of(0), None);
    }

    #[test]
    fn close_left_and_right_leave_the_tab_itself() {
        assert_eq!(left_range(2).collect::<Vec<_>>(), [0, 1]);
        assert!(left_range(0).next().is_none());
        assert_eq!(right_range(1, 4).collect::<Vec<_>>(), [2, 3]);
        assert!(right_range(3, 4).next().is_none());
        assert!(right_range(0, 0).next().is_none());
    }

    #[test]
    fn groups_wrap_around() {
        assert_eq!(next_group(2, 3), Some(0));
        assert_eq!(next_group(0, 3), Some(1));
        assert_eq!(prev_group(0, 3), Some(2));
        assert_eq!(prev_group(2, 3), Some(1));
        assert_eq!(next_group(0, 1), None);
        assert_eq!(prev_group(0, 1), None);
        assert_eq!(next_tab(2, 3), 0);
        assert_eq!(prev_tab(0, 3), 2);
        assert_eq!(next_tab(0, 1), 0);
    }
}
