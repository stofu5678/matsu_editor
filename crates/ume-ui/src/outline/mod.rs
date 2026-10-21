// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for UME Editor.
//
// Original files: sakura_core/cmd/CViewCommander_Outline.cpp, sakura_core/cmd/CViewCommander_Bookmark.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2014, the authors named in the original files (genta, zenryaku, Moca,
//     aroka, hor, YAZAKI, little YOSHI, ryoji, syat and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a UTF-16 buffer
// walked with `CDocLineMgr`, the results put in `CFuncInfoArr`, a Windows dialog) was rewritten in
// Rust on the lines of `LineSource`, the positions in the layout are left to the window, and the
// tests are new.
//
// zlib License
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Sakura's アウトライン解析 (`F_OUTLINE`, `F_OUTLINE_TOGGLE`), ブックマークの一覧 (`F_BOOKMARK_VIEW`)
//! and 次 / 前の関数リストマーク (`F_FUNCLIST_NEXT`, `F_FUNCLIST_PREV`).
//!
//! * The analyzers are the crate `ume-outline` (no window; it reads the document line by line).
//! * [`pane`] is the dialog of Sakura (`CDlgFuncList`) as a pane docked at the right of the
//!   window, one per window. It follows the active tab. Sakura's dialog can float or dock at any
//!   side; the floating form and the other sides are left out (the settings that choose them,
//!   `m_eOutlineDockSide` and friends, belong to the common settings dialog, which comes
//!   separately). The check boxes 自動的に閉じる and フォーカスを移す only matter for the floating
//!   form (a docked dialog gives the focus back to the text), so they are not here either;
//!   空行を無視する is, for the bookmark list.
//! * [`file_tree`] is Sakura's file tree (`F_FILETREE`, ファイルツリー), which the same dialog shows
//!   in place of an analysis: the pane builds it from the settings instead of the document.
//! * [`file_tree_dialog`] is the dialog 「ファイルツリー設定」 (`CDlgFileTree`) that the 設定 button of the
//!   pane opens while the pane shows the file tree.
//! * Here: what the commands do (`Command_FUNCLIST`), and the jumps between the function list marks.
//!
//! Sakura: `sakura_core/cmd/CViewCommander_Outline.cpp`, `sakura_core/outline/CDlgFuncList.cpp`,
//! `sakura_core/cmd/CViewCommander_Bookmark.cpp`.

pub(crate) mod file_tree;
pub(crate) mod file_tree_dialog;
#[cfg(test)]
mod file_tree_tests;
pub(crate) mod pane;
#[cfg(test)]
mod tests;

use gpui::{AppContext as _, Context, Entity, Global, Window};
use ume_commands::context::FuncListJump;
use ume_core::search::Direction;
use ume_outline::OutlineKind;

use crate::app::Workspace;
use crate::search::state::{not_found_status, wrapped_message};
use crate::view::EditorView;
use pane::OutlinePane;

/// What was asked for: the outline of the document type (`OUTLINE_DEFAULT`), the bookmarks
/// (`OUTLINE_BOOKMARK`) or the file tree (`OUTLINE_FILETREE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Requested {
    Default,
    Bookmark,
    FileTree,
}

/// What a command does when the pane is open (`SHOW_NORMAL`, `SHOW_RELOAD`, `SHOW_TOGGLE`; the
/// argument of the macro `Outline`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Show {
    /// Show it; the same kind only gets the focus.
    Normal,
    /// Show it and analyze again.
    Reload,
    /// Close it if it shows the same kind.
    Toggle,
}

impl Show {
    /// The argument of `F_OUTLINE` and `F_BOOKMARK_VIEW` (`lparam1`).
    pub(crate) fn from_argument(value: i64) -> Self {
        match value {
            1 => Self::Reload,
            2 => Self::Toggle,
            _ => Self::Normal,
        }
    }
}

/// The check boxes of the dialog that are kept for the whole program (`m_sOutline`). They are not
/// written to the settings file yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct OutlineOptions {
    /// 空行を無視する (`m_bMarkUpBlankLineEnable`) of the bookmark list.
    pub ignore_blank_bookmarks: bool,
}

impl Global for OutlineOptions {}

/// 見出し記号 of the common settings page 書式 (`m_szMidashiKigou`): the characters a line of a
/// text starts with to be a topic of the outline. Without it the outline uses Sakura's default.
#[derive(Clone, Debug)]
pub(crate) struct HeadingMarks(pub(crate) String);

impl Global for HeadingMarks {}

/// The kind of outline for `requested` in the view: the one of the document type, with C/C++ settled
/// by the file name (`Command_FUNCLIST`).
pub(crate) fn resolve_kind<T: 'static>(
    requested: Requested,
    view: &Entity<EditorView>,
    cx: &mut Context<T>,
) -> OutlineKind {
    match requested {
        Requested::Bookmark => return OutlineKind::Bookmark,
        Requested::FileTree => return OutlineKind::FileTree,
        Requested::Default => {}
    }
    let view = view.read(cx);
    let kind = OutlineKind::from_setting(view.view_settings().outline.kind);
    if kind == OutlineKind::CCpp {
        let path = view
            .document()
            .read(cx)
            .path()
            .map(|p| p.to_string_lossy().into_owned());
        return match ume_outline::cpp::kind_of_file_name(path.as_deref()) {
            ume_outline::cpp::CKind::C => OutlineKind::C,
            ume_outline::cpp::CKind::Cpp => OutlineKind::Cpp,
            ume_outline::cpp::CKind::CCpp => OutlineKind::CCpp,
        };
    }
    kind
}

impl Workspace {
    /// `Command_FUNCLIST`: shows, reloads or closes the pane.
    pub(crate) fn outline_command(
        &mut self,
        requested: Requested,
        show: Show,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = self.panes.active().clone();
        let kind = resolve_kind(requested, &view, cx);
        if let Some(pane) = self.outline.clone()
            && show != Show::Reload
        {
            let same = pane.read(cx).outline_type() == kind;
            match show {
                // activate it: the same kind is only given the focus
                Show::Normal if same => {
                    pane.update(cx, |pane, cx| pane.focus(window, cx));
                    return;
                }
                // close it: the same kind is closed
                Show::Toggle if same => {
                    self.close_outline(window, cx);
                    return;
                }
                _ => {}
            }
        }
        let pane = match self.outline.clone() {
            Some(pane) => {
                pane.update(cx, |pane, cx| pane.follow(&view, cx));
                pane
            }
            None => {
                let host = cx.weak_entity();
                let pane = cx.new(|cx| OutlinePane::new(host, &view, cx));
                self.outline = Some(pane.clone());
                pane
            }
        };
        pane.update(cx, |pane, cx| {
            pane.analyze(requested, cx);
            pane.focus(window, cx);
        });
        cx.notify();
    }

    /// Closes the pane and gives the focus back to the text.
    pub(crate) fn close_outline(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.outline.take().is_some() {
            let active = self.panes.active().clone();
            active.update(cx, |view, cx| window.focus(&view.focus_handle, cx));
            cx.notify();
        }
    }

    /// The pane follows the pane of the text the user works in: another tab is analyzed
    /// (`ChangeView`, `Refresh`). Called when the window is drawn.
    pub(crate) fn sync_outline(&mut self, cx: &mut Context<Self>) {
        let Some(pane) = self.outline.clone() else {
            return;
        };
        let active = self.panes.active().clone();
        if pane.read(cx).view_id() != active.entity_id() {
            pane.update(cx, |pane, cx| pane.follow(&active, cx));
        }
    }
}

impl EditorView {
    /// `F_OUTLINE`, `F_OUTLINE_TOGGLE` and `F_BOOKMARK_VIEW`: the window does it after the command
    /// ends, because it changes the panes.
    pub(crate) fn run_outline(
        &self,
        requested: Requested,
        show: Show,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ume_commands::command::Result {
        self.with_workspace(window, cx, move |workspace, window, cx| {
            workspace.outline_command(requested, show, window, cx);
        })
    }

    /// `F_FUNCLIST_NEXT` / `F_FUNCLIST_PREV`: moves to the next (previous) line of a function. A
    /// search that went on from the other end says so in the status bar; finding nothing says so
    /// there and in a box (if the option 見つからないときにメッセージを表示 is on).
    pub fn jump_to_funclist_mark(
        &mut self,
        direction: Direction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let options = self.search.options();
        let jump = self.run_with(cx, |c| c.jump_to_funclist_mark(direction, options.wrap));
        match jump {
            Ok(FuncListJump::Found { wrapped }) => {
                self.notice = wrapped.then(|| wrapped_message(direction).to_owned());
                cx.notify();
            }
            Ok(FuncListJump::NotFound) => {
                self.notice = Some(not_found_status(direction).to_owned());
                cx.notify();
                if options.notify_not_found {
                    self.info_box(funclist_not_found_message(direction), window, cx);
                }
            }
            Err(e) => {
                self.notice = Some(format!("funclist: {e}"));
                cx.notify();
            }
        }
    }
}

/// `STR_FUCLIST_NEXT_NOT_FOUND` / `STR_FUCLIST_PREV_NOT_FOUND` (`sakura_core/sakura_rc.rc`).
pub fn funclist_not_found_message(direction: Direction) -> &'static str {
    match direction {
        Direction::Forward => "前方(↓) に関数が見つかりません。",
        Direction::Backward => "後方(↑) に関数が見つかりません。",
    }
}
