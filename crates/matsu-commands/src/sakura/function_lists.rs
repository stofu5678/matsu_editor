// The order of the functions in each group of the command list (コマンド一覧), taken from the
// arrays of the groups (File ... Others) in Sakura Editor.
//
// Source: Sakura Editor, commit 48db6c7c25e2d98bdf47baab201826b4da8c7303
//   https://github.com/sakura-editor/sakura
//   sakura_core/func/Funccode.cpp
//
// This file is derived from the file above and was modified: the names of the functions were
// converted to Sakura's numeric codes (a comment names each one), and the arrays were collected in
// one table, in the order of `FUNCTION_GROUPS`.
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2000, jepro
// Copyright (C) 2001, jepro, MIK, Stonee, asa-o, Misaka, genta, hor
// Copyright (C) 2002, aroka, minfu, MIK, ai, genta
// Copyright (C) 2002, YAZAKI, aroka
// Copyright (C) 2003, genta, MIK, Moca
// Copyright (C) 2004, genta, isearch
// Copyright (C) 2005, genta, MIK
// Copyright (C) 2006, aroka, ryoji, fon
// Copyright (C) 2007, ryoji
// Copyright (C) 2008, nasukoji
// Copyright (C) 2009, ryoji
// Copyright (C) 2018-2022, Sakura Editor Organization
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

//! The functions of each group of Sakura's "コマンド一覧" (command list), in the order of the
//! list. `FUNCTIONS` holds the codes in the order of `Funccode_x.hsrc`; the order of the command
//! list is the one of the arrays of `sakura_core/func/Funccode.cpp`, which differs.

/// For each group of [`FUNCTION_GROUPS`](super::FUNCTION_GROUPS), the function codes in the order
/// of Sakura's command list. The three groups that Sakura adds at run time (registered macros,
/// custom menus, plugin commands) are not here.
#[rustfmt::skip]
pub static FUNCTION_LISTS: [&[u16]; 15] = [
    // ファイル操作系
    &[
        30101, // F_FILENEW
        30110, // F_FILENEW_NEWWINDOW
        30102, // F_FILEOPEN
        30108, // F_FILEOPEN_DROPDOWN
        30103, // F_FILESAVE
        30104, // F_FILESAVEAS_DIALOG
        30120, // F_FILESAVEALL
        30105, // F_FILECLOSE
        30107, // F_FILECLOSE_OPEN
        31320, // F_WINCLOSE
        30109, // F_FILESAVECLOSE
        30119, // F_FILE_REOPEN
        30111, // F_FILE_REOPEN_SJIS
        30112, // F_FILE_REOPEN_JIS
        30113, // F_FILE_REOPEN_EUC
        30122, // F_FILE_REOPEN_LATIN1
        30114, // F_FILE_REOPEN_UNICODE
        30117, // F_FILE_REOPEN_UNICODEBE
        30115, // F_FILE_REOPEN_UTF8
        30118, // F_FILE_REOPEN_CESU8
        30116, // F_FILE_REOPEN_UTF7
        30150, // F_PRINT
        30151, // F_PRINT_PREVIEW
        30152, // F_PRINT_PAGESETUP
        30162, // F_OPEN_HfromtoC
        30170, // F_ACTIVATE_SQLPLUS
        30171, // F_PLSQL_COMPILE_ON_SQLPLUS
        32703, // F_PLSQL_COMPILE_ON_DATABASE_CHOICE (MATSU-only)
        30180, // F_BROWSE
        30185, // F_VIEWMODE
        30190, // F_PROPERTY_FILE
        30196, // F_OPEN_FOLDER_IN_EXPLORER
        30197, // F_OPEN_COMMAND_PROMPT
        30198, // F_OPEN_COMMAND_PROMPT_AS_ADMIN
        31456, // F_OPEN_POWERSHELL
        31457, // F_OPEN_POWERSHELL_AS_ADMIN
        30191, // F_PROFILEMGR
        30194, // F_EXITALLEDITORS
        30195, // F_EXITALL
    ],
    // 編集系
    &[
        30210, // F_UNDO
        30211, // F_REDO
        30221, // F_DELETE
        30222, // F_DELETE_BACK
        30230, // F_WordDeleteToStart
        30231, // F_WordDeleteToEnd
        30232, // F_WordCut
        30233, // F_WordDelete
        30240, // F_LineCutToStart
        30241, // F_LineCutToEnd
        30242, // F_LineDeleteToStart
        30243, // F_LineDeleteToEnd
        30244, // F_CUT_LINE
        30245, // F_DELETE_LINE
        30250, // F_DUPLICATELINE
        30260, // F_INDENT_TAB
        30261, // F_UNINDENT_TAB
        30262, // F_INDENT_SPACE
        30263, // F_UNINDENT_SPACE
        30280, // F_LTRIM
        30281, // F_RTRIM
        30282, // F_SORT_ASC
        30283, // F_SORT_DESC
        30284, // F_MERGE
        30285, // F_RECONVERT
    ],
    // カーソル移動系
    &[
        30311, // F_UP
        30312, // F_DOWN
        30313, // F_LEFT
        30314, // F_RIGHT
        30315, // F_UP2
        30316, // F_DOWN2
        30320, // F_WORDLEFT
        30321, // F_WORDRIGHT
        30332, // F_GOLINETOP
        30333, // F_GOLINEEND
        30340, // F_HalfPageUp
        30341, // F_HalfPageDown
        30342, // F_1PageUp
        30343, // F_1PageDown
        30350, // F_GOFILETOP
        30351, // F_GOFILEEND
        30360, // F_CURLINECENTER
        30361, // F_CURLINETOP
        30362, // F_CURLINEBOTTOM
        30920, // F_JUMP_DIALOG
        30909, // F_JUMP_SRCHSTARTPOS
        30370, // F_JUMPHIST_PREV
        30371, // F_JUMPHIST_NEXT
        30372, // F_JUMPHIST_SET
        30380, // F_WndScrollDown
        30381, // F_WndScrollUp
        30382, // F_GONEXTPARAGRAPH
        30383, // F_GOPREVPARAGRAPH
        30384, // F_AUTOSCROLL
        30385, // F_WHEELUP
        30386, // F_WHEELDOWN
        30387, // F_WHEELLEFT
        30388, // F_WHEELRIGHT
        30389, // F_WHEELPAGEUP
        30390, // F_WHEELPAGEDOWN
        30391, // F_WHEELPAGELEFT
        30392, // F_WHEELPAGERIGHT
        30393, // F_MODIFYLINE_NEXT
        30394, // F_MODIFYLINE_PREV
    ],
    // 選択系
    &[
        30400, // F_SELECTWORD
        30401, // F_SELECTALL
        30403, // F_SELECTLINE
        30410, // F_BEGIN_SEL
        30411, // F_UP_SEL
        30412, // F_DOWN_SEL
        30413, // F_LEFT_SEL
        30414, // F_RIGHT_SEL
        30415, // F_UP2_SEL
        30416, // F_DOWN2_SEL
        30420, // F_WORDLEFT_SEL
        30421, // F_WORDRIGHT_SEL
        30432, // F_GOLINETOP_SEL
        30433, // F_GOLINEEND_SEL
        30440, // F_HalfPageUp_Sel
        30441, // F_HalfPageDown_Sel
        30442, // F_1PageUp_Sel
        30443, // F_1PageDown_Sel
        30450, // F_GOFILETOP_SEL
        30451, // F_GOFILEEND_SEL
        30482, // F_GONEXTPARAGRAPH_SEL
        30483, // F_GOPREVPARAGRAPH_SEL
        30484, // F_MODIFYLINE_NEXT_SEL
        30485, // F_MODIFYLINE_PREV_SEL
    ],
    // 矩形選択系
    &[
        30510, // F_BEGIN_BOX
        30511, // F_UP_BOX
        30512, // F_DOWN_BOX
        30513, // F_LEFT_BOX
        30514, // F_RIGHT_BOX
        30515, // F_UP2_BOX
        30516, // F_DOWN2_BOX
        30520, // F_WORDLEFT_BOX
        30521, // F_WORDRIGHT_BOX
        30530, // F_GOLOGICALLINETOP_BOX
        30532, // F_GOLINETOP_BOX
        30533, // F_GOLINEEND_BOX
        30540, // F_HalfPageUp_BOX
        30541, // F_HalfPageDown_BOX
        30542, // F_1PageUp_BOX
        30543, // F_1PageDown_BOX
        30550, // F_GOFILETOP_BOX
        30551, // F_GOFILEEND_BOX
    ],
    // クリップボード系
    &[
        30601, // F_CUT
        30602, // F_COPY
        30608, // F_COPY_ADDCRLF
        30603, // F_COPY_CRLF
        32700, // F_COPY_LF (MATSU-only)
        32701, // F_COPY_CR (MATSU-only)
        30604, // F_PASTE
        30605, // F_PASTEBOX
        30610, // F_COPYLINES
        30611, // F_COPYLINESASPASSAGE
        30612, // F_COPYLINESWITHLINENUMBER
        30613, // F_COPY_COLOR_HTML
        30614, // F_COPY_COLOR_HTML_LINENUMBER
        30622, // F_COPYFNAME
        30620, // F_COPYPATH
        30631, // F_COPYDIRPATH
        30621, // F_COPYTAG
        30630, // F_CREATEKEYBINDLIST
    ],
    // 挿入系
    &[
        30790, // F_INS_DATE
        30791, // F_INS_TIME
        30792, // F_CTRL_CODE_DIALOG
        30794, // F_INS_FILE_USED_RECENTLY
        30795, // F_INS_FOLDER_USED_RECENTLY
    ],
    // 変換系
    &[
        30800, // F_TOLOWER
        30801, // F_TOUPPER
        30810, // F_TOHANKAKU
        30811, // F_TOZENKAKUKATA
        30812, // F_TOZENKAKUHIRA
        30815, // F_TOZENEI
        30816, // F_TOHANEI
        30817, // F_TOHANKATA
        30813, // F_HANKATATOZENKATA
        30814, // F_HANKATATOZENHIRA
        30830, // F_TABTOSPACE
        30831, // F_SPACETOTAB
        30850, // F_CODECNV_AUTO2SJIS
        30851, // F_CODECNV_EMAIL
        30852, // F_CODECNV_EUC2SJIS
        30853, // F_CODECNV_UNICODE2SJIS
        30856, // F_CODECNV_UNICODEBE2SJIS
        30854, // F_CODECNV_UTF82SJIS
        30855, // F_CODECNV_UTF72SJIS
        30860, // F_CODECNV_SJIS2JIS
        30861, // F_CODECNV_SJIS2EUC
        30862, // F_CODECNV_SJIS2UTF8
        30863, // F_CODECNV_SJIS2UTF7
        30870, // F_BASE64DECODE
        30880, // F_UUDECODE
    ],
    // 検索系
    &[
        30901, // F_SEARCH_DIALOG
        30908, // F_SEARCH_BOX
        30902, // F_SEARCH_NEXT
        30903, // F_SEARCH_PREV
        30904, // F_REPLACE_DIALOG
        30905, // F_SEARCH_CLEARMARK
        30909, // F_JUMP_SRCHSTARTPOS
        30910, // F_GREP_DIALOG
        30912, // F_GREP_REPLACE_DLG
        30920, // F_JUMP_DIALOG
        30930, // F_OUTLINE
        30931, // F_OUTLINE_TOGGLE
        30990, // F_FILETREE
        30940, // F_TAGJUMP
        30945, // F_TAGJUMP_CLOSE
        30941, // F_TAGJUMPBACK
        30943, // F_TAGS_MAKE
        30944, // F_DIRECT_TAGJUMP
        30946, // F_TAGJUMP_KEYWORD
        30950, // F_COMPARE
        30976, // F_DIFF_DIALOG
        30978, // F_DIFF_NEXT
        30979, // F_DIFF_PREV
        30980, // F_DIFF_RESET
        30960, // F_BRACKETPAIR
        30970, // F_BOOKMARK_SET
        30971, // F_BOOKMARK_NEXT
        30972, // F_BOOKMARK_PREV
        30973, // F_BOOKMARK_RESET
        30974, // F_BOOKMARK_VIEW
        30981, // F_ISEARCH_NEXT
        30982, // F_ISEARCH_PREV
        30983, // F_ISEARCH_REGEXP_NEXT
        30984, // F_ISEARCH_REGEXP_PREV
        30985, // F_ISEARCH_MIGEMO_NEXT
        30986, // F_ISEARCH_MIGEMO_PREV
        30988, // F_FUNCLIST_NEXT
        30989, // F_FUNCLIST_PREV
    ],
    // モード切り替え系
    &[
        31001, // F_CHGMOD_INS
        31010, // F_CHG_CHARSET
        31081, // F_CHGMOD_EOL_CRLF
        31082, // F_CHGMOD_EOL_LF
        31083, // F_CHGMOD_EOL_CR
        31099, // F_CANCEL_MODE
    ],
    // 設定系
    &[
        31100, // F_SHOWTOOLBAR
        31101, // F_SHOWFUNCKEY
        31103, // F_SHOWTAB
        31102, // F_SHOWSTATUSBAR
        31104, // F_SHOWMINIMAP
        31110, // F_TYPE_LIST
        31111, // F_OPTION_TYPE
        31112, // F_OPTION
        31120, // F_FONT
        31122, // F_SETFONTSIZEUP
        31123, // F_SETFONTSIZEDOWN
        31140, // F_WRAPWINDOWWIDTH
        30152, // F_PRINT_PAGESETUP
        31113, // F_FAVORITE
        31141, // F_TMPWRAPNOWRAP
        31142, // F_TMPWRAPSETTING
        31143, // F_TMPWRAPWINDOW
        31144, // F_SELECT_COUNT_MODE
    ],
    // マクロ系
    &[
        31250, // F_RECKEYMACRO
        31251, // F_SAVEKEYMACRO
        31252, // F_LOADKEYMACRO
        31253, // F_EXECKEYMACRO
        31254, // F_EXECEXTMACRO
        31270, // F_EXECMD_DIALOG
    ],
    // ウィンドウ系
    &[
        31310, // F_SPLIT_V
        31311, // F_SPLIT_H
        31312, // F_SPLIT_VH
        31320, // F_WINCLOSE
        31321, // F_WIN_CLOSEALL
        31388, // F_TAB_CLOSEOTHER
        31340, // F_NEXTWINDOW
        31341, // F_PREVWINDOW
        31336, // F_WINLIST
        31337, // F_DLGWINLIST
        31330, // F_CASCADE
        31331, // F_TILE_V
        31332, // F_TILE_H
        31334, // F_TOPMOST
        31333, // F_BIND_WINDOW
        31380, // F_GROUPCLOSE
        31381, // F_NEXTGROUP
        31382, // F_PREVGROUP
        31383, // F_TAB_MOVERIGHT
        31384, // F_TAB_MOVELEFT
        31385, // F_TAB_SEPARATE
        31386, // F_TAB_JOINTNEXT
        31387, // F_TAB_JOINTPREV
        31389, // F_TAB_CLOSELEFT
        31390, // F_TAB_CLOSERIGHT
        31350, // F_MAXIMIZE_V
        31352, // F_MAXIMIZE_H
        31351, // F_MINIMIZE_ALL
        31360, // F_REDRAW
        31370, // F_WIN_OUTPUT
        32702, // F_PREVIEW_TAB (MATSU-only)
    ],
    // 支援
    &[
        31430, // F_HOKAN
        31431, // F_TOGGLE_KEY_SEARCH
        31440, // F_HELP_CONTENTS
        31441, // F_HELP_SEARCH
        31445, // F_MENU_ALLFUNC
        31450, // F_EXTHELP1
        31451, // F_EXTHTMLHELP
        31455, // F_ABOUT
    ],
    // その他 (only a dummy, which the list skips: `Command_MENU_ALLFUNC` leaves out code 0)
    &[
        0, // F_DISABLE
    ],
];

#[cfg(test)]
mod tests {
    use super::FUNCTION_LISTS;
    use crate::sakura::{FUNCTION_GROUPS, FUNCTIONS};

    #[test]
    fn every_listed_code_is_a_function_of_its_group() {
        assert_eq!(FUNCTION_LISTS.len(), FUNCTION_GROUPS.len());
        for (group, codes) in FUNCTION_LISTS.iter().enumerate() {
            for &code in codes.iter().filter(|&&c| c != 0) {
                let listed = FUNCTIONS
                    .iter()
                    .any(|f| f.code == code && f.groups.contains(&u8::try_from(group).unwrap()));
                assert!(
                    listed,
                    "{code} is not in the group {}",
                    FUNCTION_GROUPS[group]
                );
            }
        }
    }

    #[test]
    fn the_order_is_the_one_of_funccode_cpp_not_of_the_function_table() {
        // the File array of Funccode.cpp has F_FILENEW_NEWWINDOW (30110) right after F_FILENEW (30101);
        // the function table has it after F_FILESAVECLOSE.
        assert_eq!(&FUNCTION_LISTS[0][..3], &[30101, 30110, 30102]);
        assert_eq!(FUNCTION_LISTS[13][0], 31430); // F_HOKAN
        assert_eq!(FUNCTION_LISTS[13][4], 31445); // F_MENU_ALLFUNC
        assert_eq!(*FUNCTION_LISTS[13].last().unwrap(), 31455); // F_ABOUT
    }
}
