// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/func/Funccode.cpp (IsFuncChecked, IsFuncEnable)
//
// Original copyright notices of that file:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2001, jepro, genta
//   Copyright (C) 2002, YAZAKI, aroka, hor, MIK, Moca, minfu, ai, genta, Azumaiya, novice
//   Copyright (C) 2003, MIK, genta, Moca, zenryaku, ryoji, Kazika, KEITA
//   Copyright (C) 2004, genta, Moca, novice, MIK, Kazika
//   Copyright (C) 2005, genta, aroka, Moca
//   Copyright (C) 2006, aroka, ryoji, genta
//   Copyright (C) 2007, ryoji, genta, nasukoji
//   Copyright (C) 2008, nasukoji, ryoji, Uchi
//   Copyright (C) 2009, ryoji, nasukoji, syat
//   Copyright (C) 2018-2022, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The check marks and the gray items of the menus: what Sakura's `IsFuncChecked` and
//! `IsFuncEnable` answer for a function code in a window.
//!
//! [`FuncStates`] is a snapshot of the state of one window (its active view and its document),
//! taken when a menu opens. It knows nothing of GPUI menus: [`apply`] puts the answers on the
//! items of a menu, and the toolbar can ask the same snapshot for its pressed and gray buttons.
//!
//! Sakura uses them for the main menu (`CEditWnd::InitMenu`, `WM_INITMENUPOPUP`), the right-click
//! and custom menus (`CEditView::CreatePopUpMenuSub`) and the toolbar (`CMainToolBar`).
//!
//! Not here yet:
//! * The `IsFuncEnable` cases that need state MATSU does not have: the clipboard (`F_PASTE`, whose
//!   read is slow on Wayland), the box functions (a fixed-pitch font), the limit of windows
//!   (there is none), and whether PowerShell is installed. Another
//!   window that records a key macro is not known either.

use gpui::{Action, App, MenuItem, OwnedMenuItem};
use matsu_commands::sakura::FUNCTIONS;
use matsu_core::buffer::Eol;
use matsu_core::encoding::Encoding;

use crate::search::isearch::ISearchMode;
use crate::settings::WrapMethod;
use crate::view::{EditorView, RunFunction};

/// The state of a window that decides the marks. Every field is what Sakura reads in
/// `IsFuncChecked` / `IsFuncEnable`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FuncStates {
    /// `GetDocumentEncoding`.
    pub encoding: Encoding,
    /// `GetNewLineCode`: the line ending that Enter inserts.
    pub new_line: Eol,
    /// `CAppMode::IsViewMode`.
    pub view_mode: bool,
    /// `!IsInsMode`.
    pub overwrite: bool,
    /// `m_nTextWrapMethodCur`.
    pub wrap: WrapMethod,
    /// This window records the key macro (`m_hwndRecordingKeyMacro`).
    pub recording_key_macro: bool,
    /// The status bar is shown.
    pub status_bar: bool,
    /// `m_bDispTabWnd`: the tab bar is shown.
    pub tab_bar: bool,
    /// `!m_bDispTabWndMultiWin`: the windows are grouped.
    pub grouped: bool,
    /// The window is on top of the others (`WS_EX_TOPMOST`); false when the OS cannot tell.
    pub topmost: bool,
    /// The incremental search mode that is on in the view, if any (`IsISearchEnabled`).
    pub isearch: Option<ISearchMode>,
    /// The outline pane is open.
    pub outline: bool,
    /// `IsEnableUndo`.
    pub can_undo: bool,
    /// `IsEnableRedo`.
    pub can_redo: bool,
    /// `IsTextSelected`.
    pub text_selected: bool,
    /// The document has a file (`GetFilePathClass().IsValidPath()`).
    pub has_path: bool,
    /// The file is on a UNC path (`PathIsUNCW`).
    pub unc_path: bool,
    /// The path of the file. Only `F_FILESAVE` and `F_OPEN_HfromtoC` look at the disk with it,
    /// when their items are asked for (not when the snapshot is taken: the macOS menu takes one
    /// on every repaint), and what they find is kept for a second ([`disk`]) because the toolbar
    /// asks on every repaint.
    pub path: Option<std::path::PathBuf>,
    /// `IsModified`.
    pub modified: bool,
    /// `IsChgCodeSet`: the encoding or the BOM was changed (`F_CHG_CHARSET`).
    pub format_changed: bool,
    /// The search history has a string (`m_aSearchKeys`).
    pub has_search_keys: bool,
    /// The view remembers where the search began (`m_ptSrchStartPos_PHY`).
    pub has_search_start: bool,
    /// How many documents are open in all the windows (`m_nEditArrNum`).
    pub open_documents: usize,
    /// The toolbar and the function key bar are shown (`F_SHOWTOOLBAR`, `F_SHOWFUNCKEY`).
    pub toolbar: bool,
    pub funckeys: bool,
    /// The minimap is shown (`F_SHOWMINIMAP`: `GetMiniMap().GetHwnd() != nullptr`).
    pub minimap: bool,
    /// `m_bEnableUnmodifiedOverwrite`: a save writes the file even when it is unmodified.
    pub unmodified_overwrite: bool,
    /// A DIFF is shown in the document (`CDiffManager::IsDiffUse`).
    pub diff_shown: bool,
    /// The jump history has an entry before (after) the current one.
    pub can_jump_back: bool,
    pub can_jump_forward: bool,
    /// The window shows the result of a Grep (`m_bGrepMode`).
    pub grep_window: bool,
    /// The key macro can be saved (`IsSaveOk`) and there is one to play.
    pub key_macro_saveable: bool,
    pub key_macro_loaded: bool,
    /// The window counts the selection in bytes (`m_nSelectCountMode`, else the common setting).
    pub select_count_by_byte: bool,
    /// The keyword help looks up the word at the caret (`m_bUseCaretKeyWord`).
    pub key_search: bool,
}

/// The name of the function with this code (`F_UNDO`), if it is in Sakura's table.
fn name_of(code: u16) -> Option<&'static str> {
    FUNCTIONS.iter().find(|f| f.code == code).map(|f| f.name)
}

/// The functions that `is_checked` answers for: the ones that have an on and an off state (a
/// check mark in a menu, a pressed toolbar button). The accessibility tree reports them as
/// toggles, also while they are off. Keep it in step with the arms of `is_checked`.
const CHECKABLE: &[&str] = &[
    "F_FILE_REOPEN_SJIS",
    "F_FILE_REOPEN_JIS",
    "F_FILE_REOPEN_EUC",
    "F_FILE_REOPEN_LATIN1",
    "F_FILE_REOPEN_UNICODE",
    "F_FILE_REOPEN_UNICODEBE",
    "F_FILE_REOPEN_UTF8",
    "F_FILE_REOPEN_CESU8",
    "F_FILE_REOPEN_UTF7",
    "F_RECKEYMACRO",
    "F_SHOWTOOLBAR",
    "F_SHOWFUNCKEY",
    "F_SHOWMINIMAP",
    "F_SHOWTAB",
    "F_SHOWSTATUSBAR",
    "F_TMPWRAPNOWRAP",
    "F_TMPWRAPSETTING",
    "F_TMPWRAPWINDOW",
    "F_VIEWMODE",
    "F_CHGMOD_EOL_CRLF",
    "F_CHGMOD_EOL_LF",
    "F_CHGMOD_EOL_CR",
    "F_CHGMOD_INS",
    "F_BIND_WINDOW",
    "F_TOPMOST",
    "F_ISEARCH_NEXT",
    "F_ISEARCH_PREV",
    "F_ISEARCH_REGEXP_NEXT",
    "F_ISEARCH_REGEXP_PREV",
    "F_OUTLINE_TOGGLE",
    "F_SELECT_COUNT_MODE",
    "F_TOGGLE_KEY_SEARCH",
];

/// Whether the function of `code` has a check mark that can be on or off (see [`CHECKABLE`]).
pub(crate) fn is_checkable(code: u16) -> bool {
    name_of(code).is_some_and(|name| CHECKABLE.contains(&name))
}

impl FuncStates {
    /// The state of the window of `view`. `outline` is whether the window shows the outline pane
    /// (the window keeps it, not the view) and `topmost` whether the OS says the window is on top.
    pub(crate) fn of_view(
        view: &EditorView,
        outline: bool,
        topmost: bool,
        open_documents: usize,
        cx: &App,
    ) -> Self {
        let shared = view.document().read(cx);
        let tabs = crate::tabs::settings::current(cx);
        let path = shared.path();
        let bars = crate::bars::current(cx);
        Self {
            encoding: shared.file().encoding,
            new_line: shared.new_line(),
            view_mode: shared.view_mode(),
            overwrite: shared.overwrite(),
            wrap: view.prefs.wrap_method,
            recording_key_macro: view.is_recording_key_macro(),
            status_bar: view.status_bar_shown(),
            tab_bar: tabs.show_bar,
            grouped: tabs.grouped,
            topmost,
            isearch: view.isearch().map(crate::search::isearch::ISearch::mode),
            outline,
            can_undo: shared.model().history().can_undo(),
            can_redo: shared.model().history().can_redo(),
            text_selected: view.cursor.has_selection(),
            has_path: path.is_some(),
            unc_path: path
                .and_then(std::path::Path::to_str)
                .is_some_and(matsu_platform::shell::is_unc_path),
            path: path.map(std::path::Path::to_path_buf),
            modified: shared.model().is_modified(),
            format_changed: shared.format_changed(),
            has_search_keys: !crate::search::history::search_keys(cx).is_empty(),
            has_search_start: view.search.start_pos().is_some(),
            open_documents,
            toolbar: bars.toolbar,
            funckeys: bars.funckeys,
            minimap: crate::minimap::current(cx).shown,
            unmodified_overwrite: crate::file_ops::prefs::prefs(cx).enable_unmodified_overwrite,
            diff_shown: shared.model().diff_marks().in_use(),
            can_jump_back: view.cursor.jump_history().has_prev(),
            can_jump_forward: view.cursor.jump_history().has_next(),
            grep_window: view.is_grep_window(),
            key_macro_saveable: view.macros.can_save_key_macro(cx),
            key_macro_loaded: view.macros.has_key_macro(cx),
            select_count_by_byte: view
                .prefs
                .select_count
                .by_byte(crate::status_options::current(cx).select_count_by_byte),
            key_search: crate::key_help::caret_key_word(cx),
        }
    }

    /// `IsFuncChecked`: whether the item of the function shows a check mark.
    pub(crate) fn is_checked(&self, code: u16) -> bool {
        let Some(name) = name_of(code) else {
            return false;
        };
        match name {
            "F_FILE_REOPEN_SJIS" => self.encoding == Encoding::ShiftJis,
            "F_FILE_REOPEN_JIS" => self.encoding == Encoding::Jis,
            "F_FILE_REOPEN_EUC" => self.encoding == Encoding::EucJp,
            "F_FILE_REOPEN_LATIN1" => self.encoding == Encoding::Latin1,
            "F_FILE_REOPEN_UNICODE" => self.encoding == Encoding::Utf16Le,
            "F_FILE_REOPEN_UNICODEBE" => self.encoding == Encoding::Utf16Be,
            "F_FILE_REOPEN_UTF8" => self.encoding == Encoding::Utf8,
            "F_FILE_REOPEN_CESU8" => self.encoding == Encoding::Cesu8,
            "F_FILE_REOPEN_UTF7" => self.encoding == Encoding::Utf7,
            "F_RECKEYMACRO" => self.recording_key_macro,
            "F_SHOWTOOLBAR" => self.toolbar,
            "F_SHOWFUNCKEY" => self.funckeys,
            "F_SHOWMINIMAP" => self.minimap,
            "F_SHOWTAB" => self.tab_bar,
            "F_SHOWSTATUSBAR" => self.status_bar,
            "F_TMPWRAPNOWRAP" => self.wrap == WrapMethod::None,
            "F_TMPWRAPSETTING" => self.wrap == WrapMethod::SettingWidth,
            "F_TMPWRAPWINDOW" => self.wrap == WrapMethod::WindowWidth,
            "F_VIEWMODE" => self.view_mode,
            "F_CHGMOD_EOL_CRLF" => self.new_line == Eol::CrLf,
            "F_CHGMOD_EOL_LF" => self.new_line == Eol::Lf,
            "F_CHGMOD_EOL_CR" => self.new_line == Eol::Cr,
            // `IsInsMode`: the mark is on in insert mode.
            "F_CHGMOD_INS" => !self.overwrite,
            // Sakura: the bar is shown and the windows are grouped.
            "F_BIND_WINDOW" => self.tab_bar && self.grouped,
            "F_TOPMOST" => self.topmost,
            // `IsISearchEnabled( nId )`: the mode of the function is the one that is on. The
            // direction is not looked at. The Migemo search is not built.
            "F_ISEARCH_NEXT" | "F_ISEARCH_PREV" => self.isearch == Some(ISearchMode::Normal),
            "F_ISEARCH_REGEXP_NEXT" | "F_ISEARCH_REGEXP_PREV" => {
                self.isearch == Some(ISearchMode::Regexp)
            }
            "F_OUTLINE_TOGGLE" => self.outline,
            "F_SELECT_COUNT_MODE" => self.select_count_by_byte,
            "F_TOGGLE_KEY_SEARCH" => self.key_search,
            _ => false,
        }
    }

    /// `IsFuncEnable`: whether the item of the function can be chosen. `true` for the functions
    /// whose rule is not kept here (a function without a command is grayed out elsewhere).
    pub(crate) fn is_enabled(&self, code: u16) -> bool {
        let Some(name) = name_of(code) else {
            return true;
        };
        // `IsModificationForbidden`, checked first.
        if self.view_mode && MODIFICATION_FORBIDDEN.contains(&name) {
            return false;
        }
        let tab_mode = self.tab_bar && self.grouped;
        match name {
            "F_RECKEYMACRO" | "F_LOADKEYMACRO" => true,
            // Another window that records is not known here.
            "F_SAVEKEYMACRO" => self.recording_key_macro || self.key_macro_saveable,
            "F_EXECKEYMACRO" => self.recording_key_macro || self.key_macro_loaded,
            "F_SEARCH_NEXT" | "F_SEARCH_PREV" | "F_SEARCH_CLEARMARK" | "F_REPLACE"
            | "F_REPLACE_ALL" => self.has_search_keys,
            "F_JUMP_SRCHSTARTPOS" => self.has_search_start,
            "F_COMPARE" => self.open_documents >= 2,
            "F_DIFF_NEXT" | "F_DIFF_PREV" | "F_DIFF_RESET" => self.diff_shown,
            // View mode is refused above. Then a save is possible when the document is modified,
            // its encoding or BOM was changed, or `m_bEnableUnmodifiedOverwrite` is on. A named
            // file that is not on disk yet is created by a save even when unmodified (`on_disk`
            // in `file_ops::save`).
            "F_FILESAVE" => {
                !self.view_mode
                    && (self.modified
                        || self.format_changed
                        || self.unmodified_overwrite
                        || self.path.as_deref().is_some_and(|p| !disk::exists(p)))
            }
            "F_UNDO" => self.can_undo,
            "F_REDO" => self.can_redo,
            "F_COPYLINES"
            | "F_COPYLINESASPASSAGE"
            | "F_COPYLINESWITHLINENUMBER"
            | "F_COPY_COLOR_HTML"
            | "F_COPY_COLOR_HTML_LINENUMBER"
            | "F_TOLOWER"
            | "F_TOUPPER"
            | "F_TOHANKAKU"
            | "F_TOHANKATA"
            | "F_TOZENEI"
            | "F_TOHANEI"
            | "F_TOZENKAKUKATA"
            | "F_TOZENKAKUHIRA"
            | "F_HANKATATOZENKATA"
            | "F_HANKATATOZENHIRA"
            | "F_TABTOSPACE"
            | "F_SPACETOTAB"
            | "F_CODECNV_AUTO2SJIS"
            | "F_CODECNV_EMAIL"
            | "F_CODECNV_EUC2SJIS"
            | "F_CODECNV_UNICODE2SJIS"
            | "F_CODECNV_UNICODEBE2SJIS"
            | "F_CODECNV_UTF82SJIS"
            | "F_CODECNV_UTF72SJIS"
            | "F_CODECNV_SJIS2JIS"
            | "F_CODECNV_SJIS2EUC"
            | "F_CODECNV_SJIS2UTF8"
            | "F_CODECNV_SJIS2UTF7"
            | "F_BASE64DECODE"
            | "F_UUDECODE" => self.text_selected,
            "F_CUT_LINE" | "F_DELETE_LINE" => !self.text_selected,
            // `Command_OPEN_HfromtoC( TRUE )`: the file has a header or source of the same name.
            "F_OPEN_HfromtoC" => self.path.as_deref().is_some_and(|path| {
                crate::file_ops::header_source::counterpart(path, disk::exists).is_some()
            }),
            // `F_VIEWMODE` is always allowed (commented out in Sakura).
            "F_COPYPATH"
            | "F_COPYDIRPATH"
            | "F_COPYTAG"
            | "F_COPYFNAME"
            | "F_PLSQL_COMPILE_ON_SQLPLUS"
            | "F_BROWSE"
            | "F_OPEN_FOLDER_IN_EXPLORER"
            | "F_OPEN_POWERSHELL"
            | "F_OPEN_POWERSHELL_AS_ADMIN" => self.has_path,
            // `cmd.exe` cannot start in a UNC path.
            "F_OPEN_COMMAND_PROMPT" | "F_OPEN_COMMAND_PROMPT_AS_ADMIN" => {
                self.has_path && !self.unc_path
            }
            "F_JUMPHIST_PREV" => self.can_jump_back,
            "F_JUMPHIST_NEXT" => self.can_jump_forward,
            "F_DIRECT_TAGJUMP" | "F_TAGJUMP_KEYWORD" | "F_TAGJUMP_LIST" => {
                !self.grep_window && self.has_path
            }
            // Windows cannot be bound without the tab bar.
            "F_BIND_WINDOW" | "F_TAB_MOVERIGHT" | "F_TAB_MOVELEFT" | "F_TAB_CLOSELEFT"
            | "F_TAB_CLOSERIGHT" => self.tab_bar,
            "F_GROUPCLOSE"
            | "F_NEXTGROUP"
            | "F_PREVGROUP"
            | "F_TAB_SEPARATE"
            | "F_TAB_JOINTNEXT"
            | "F_TAB_JOINTPREV"
            | "F_FILENEW_NEWWINDOW" => tab_mode,
            _ => true,
        }
    }
}

/// What the disk says about a path, kept for a second: the toolbar asks `IsFuncEnable` each time
/// the window is drawn, and a `stat` on a slow or network drive would stall the drawing.
mod disk {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    const KEEP: Duration = Duration::from_secs(1);

    thread_local! {
        static SEEN: RefCell<HashMap<PathBuf, (Instant, bool)>> = RefCell::new(HashMap::new());
    }

    /// `path.exists()`, as it was less than a second ago if it was asked then.
    pub(super) fn exists(path: &Path) -> bool {
        let now = Instant::now();
        SEEN.with(|seen| {
            let mut seen = seen.borrow_mut();
            if let Some(&(at, found)) = seen.get(path)
                && now.duration_since(at) < KEEP
            {
                return found;
            }
            let found = path.exists();
            if seen.len() > 64 {
                seen.clear();
            }
            seen.insert(path.to_path_buf(), (now, found));
            found
        })
    }

    /// Forgets what was seen (the tests change the disk within a second).
    #[cfg(test)]
    pub(super) fn forget() {
        SEEN.with(|seen| seen.borrow_mut().clear());
    }
}

/// `EIsModificationForbidden` (`sakura_core/doc/CEditDoc.cpp`): the functions that are grayed out
/// while the document cannot be edited. The command context refuses the edits themselves
/// (`CommandContext::with_view_mode`) at the one place every edit goes through, so it has no list
/// of names to reuse; this is the list of what the menus gray out.
const MODIFICATION_FORBIDDEN: &[&str] = &[
    "F_WCHAR",
    "F_IME_CHAR",
    "F_UNDO",
    "F_REDO",
    "F_DELETE",
    "F_DELETE_BACK",
    "F_WordDeleteToStart",
    "F_WordDeleteToEnd",
    "F_WordCut",
    "F_WordDelete",
    "F_LineCutToStart",
    "F_LineCutToEnd",
    "F_LineDeleteToStart",
    "F_LineDeleteToEnd",
    "F_CUT_LINE",
    "F_DELETE_LINE",
    "F_DUPLICATELINE",
    "F_INDENT_TAB",
    "F_UNINDENT_TAB",
    "F_INDENT_SPACE",
    "F_UNINDENT_SPACE",
    "F_LTRIM",
    "F_RTRIM",
    "F_SORT_ASC",
    "F_SORT_DESC",
    "F_MERGE",
    "F_CUT",
    "F_PASTE",
    "F_PASTEBOX",
    "F_INSTEXT_W",
    "F_ADDTAIL_W",
    "F_INS_DATE",
    "F_INS_TIME",
    "F_CTRL_CODE_DIALOG",
    "F_INS_FILE_USED_RECENTLY",
    "F_INS_FOLDER_USED_RECENTLY",
    "F_TOLOWER",
    "F_TOUPPER",
    "F_TOHANKAKU",
    "F_TOZENKAKUKATA",
    "F_TOZENKAKUHIRA",
    "F_HANKATATOZENKATA",
    "F_HANKATATOZENHIRA",
    "F_TOZENEI",
    "F_TOHANEI",
    "F_TOHANKATA",
    "F_TABTOSPACE",
    "F_SPACETOTAB",
    "F_CODECNV_AUTO2SJIS",
    "F_CODECNV_EMAIL",
    "F_CODECNV_EUC2SJIS",
    "F_CODECNV_UNICODE2SJIS",
    "F_CODECNV_UTF82SJIS",
    "F_CODECNV_UTF72SJIS",
    "F_CODECNV_UNICODEBE2SJIS",
    "F_CODECNV_SJIS2JIS",
    "F_CODECNV_SJIS2EUC",
    "F_CODECNV_SJIS2UTF8",
    "F_CODECNV_SJIS2UTF7",
    "F_REPLACE_DIALOG",
    "F_REPLACE",
    "F_REPLACE_ALL",
    "F_CHGMOD_INS",
    "F_HOKAN",
];

/// Sets the marks of one item that runs a function ([`RunFunction`]): `checked` from
/// [`FuncStates::is_checked`], and `disabled` when [`FuncStates::is_enabled`] says no. An item
/// that was gray stays gray.
fn mark(action: &dyn Action, checked: &mut bool, disabled: &mut bool, states: &FuncStates) {
    if let Some(run) = action.as_any().downcast_ref::<RunFunction>() {
        *checked |= states.is_checked(run.code);
        *disabled |= !states.is_enabled(run.code);
    }
}

/// Puts the marks on the items of a menu (and its submenus) that GPUI draws itself.
pub(crate) fn apply(items: &mut [OwnedMenuItem], states: &FuncStates) {
    for item in items {
        match item {
            OwnedMenuItem::Action {
                action,
                checked,
                disabled,
                ..
            } => mark(action.as_ref(), checked, disabled, states),
            OwnedMenuItem::Submenu(menu) => apply(&mut menu.items, states),
            OwnedMenuItem::Separator | OwnedMenuItem::SystemMenu(_) => {}
        }
    }
}

/// As [`apply`], for the items that are set on the native menu (macOS).
pub(crate) fn apply_to_menu_items(items: &mut [MenuItem], states: &FuncStates) {
    for item in items {
        match item {
            MenuItem::Action {
                action,
                checked,
                disabled,
                ..
            } => {
                let was_disabled = *disabled;
                mark(action.as_ref(), checked, disabled, states);
                if *disabled && !was_disabled {
                    // The native menu asks whether the action can be handled, and the editor
                    // handles `RunFunction`: nobody handles `Unavailable` (see `gpui_items`).
                    *action = Box::new(crate::app::Unavailable);
                }
            }
            MenuItem::Submenu(menu) => apply_to_menu_items(&mut menu.items, states),
            MenuItem::Separator | MenuItem::SystemMenu(_) => {}
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod enable_tests;
