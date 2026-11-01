# GENERATED from crates/matsu-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: MATSU_UPDATE_GENERATED=1 cargo test -p matsu-plugin generated_files

# The names of the editor's macro API (the names and the arguments of Sakura's macros).
$global:MatsuApiNames = @(
    # GetLineCount(reserved: int) -> int
    # The number of logical lines (lines separated by line breaks). The text after the last line
    # break is not a line unless it is not empty. The argument is reserved and must be 0.
    'GetLineCount'
    # GetLineStr(line: int) -> string
    # A logical line with its line break ("\n"; the last line has none unless the text ends with
    # one). 0 is the line of the cursor, 1 is the first line. Past the last line it is an empty
    # string.
    'GetLineStr'
    # InsText(text: string) -> null
    # Inserts the text at the cursor, replacing the selection. The cursor ends after the text.
    'InsText'
    # AddTail(text: string) -> null
    # Moves to the end of the file, drops the selection, and inserts the text there.
    'AddTail'
    # GoFileTop() -> null
    # Moves the cursor to the start of the file and drops the selection.
    'GoFileTop'
    # GoFileEnd() -> null
    # Moves the cursor to the end of the file and drops the selection.
    'GoFileEnd'
    # Up() -> null
    # Moves the cursor up one line and drops the selection.
    'Up'
    # Down() -> null
    # Moves the cursor down one line and drops the selection.
    'Down'
    # SelectAll() -> null
    # Selects the whole text. The cursor goes to the end.
    'SelectAll'
    # GetSelectedString(reserved: int) -> string
    # The selected text with its line breaks ("\n"), or an empty string when nothing is
    # selected. The argument is reserved and ignored.
    'GetSelectedString'
    # IsTextSelected() -> int
    # 0 when nothing is selected, 1 when text is selected, 2 when a box selection is selected.
    'IsTextSelected'
    # MoveCursor(line: int, column: int, option: int) -> null
    # Moves the cursor to a line and a column, both counted from 1. The column counts UTF-16
    # code units. Option: 0 does not select, 1 selects from the old position, 2 selects a box
    # with the old and the new position as corners (2 wins over 1). Past the end of the text it
    # goes to the end of the text.
    'MoveCursor'
    # Jump(line: int, flag: int?) -> null
    # Moves to the start of a line, counted from 1 in logical lines. Lines before 1 and after
    # the last one are limited to the first and the last line. Flag bit 0 (1, the default) is
    # the line break unit, which is the only unit supported yet; bit 1 (PL/SQL) is not
    # supported.
    'Jump'
    # GetFilename() -> string
    # The full path of the file, or an empty string when the text has no file.
    'GetFilename'
    # GetSelectLineFrom() -> int
    # The line where the selection starts, counted from 1 in layout lines (wrapped rows count as
    # lines). 0 when nothing is selected.
    'GetSelectLineFrom'
    # GetSelectColmFrom() -> int
    # The column where the selection starts, counted from 1 in layout columns (cells: a
    # full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected. Same
    # as GetSelectColumnFrom.
    'GetSelectColmFrom'
    # GetSelectColumnFrom() -> int
    # The column where the selection starts, counted from 1 in layout columns (cells: a
    # full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected.
    'GetSelectColumnFrom'
    # GetSelectLineTo() -> int
    # The line where the selection ends, counted from 1 in layout lines. 0 when nothing is
    # selected.
    'GetSelectLineTo'
    # GetSelectColmTo() -> int
    # The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
    # selected. Same as GetSelectColumnTo.
    'GetSelectColmTo'
    # GetSelectColumnTo() -> int
    # The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
    # selected.
    'GetSelectColumnTo'
    # IsInsMode() -> int
    # 1 in insert mode, 0 in overwrite mode.
    'IsInsMode'
    # GetCharCode() -> int
    # The encoding of the file: 0 Shift_JIS, 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, 5 UTF-7, 6
    # UTF-16 BE, 7 CESU-8, 8 Latin1.
    'GetCharCode'
    # GetLineCode() -> int
    # The line ending that new line breaks get: 0 CR LF, 1 CR, 2 LF (3 NEL, 4 LS, 5 PS).
    'GetLineCode'
    # IsPossibleUndo() -> int
    # 1 when there is something to undo, else 0.
    'IsPossibleUndo'
    # IsPossibleRedo() -> int
    # 1 when there is something to redo, else 0.
    'IsPossibleRedo'
    # GetLineAttribute(line: int, type: int) -> int
    # An attribute of a logical line (0 is the line of the cursor, 1 the first line). Type 0 is
    # 1 when the line changed since the last save, type 1 the number of the undo step that last
    # touched it, type 2 the bookmark (1 or 0). Types 3 (diff mark) and 4 (function list mark)
    # answer the error -32001 until the editor has them. A negative line, a line past the end
    # and any other type are errors.
    'GetLineAttribute'
    # IsTextSelectingLock() -> int
    # 0 when the selection is not locked, 1 when it is locked (plain moves extend the
    # selection), 2 when it is locked and a box selection.
    'IsTextSelectingLock'
    # GetSaveFilename() -> string
    # The full path that a save writes to, or an empty string when the text has no file. The
    # same as GetFilename, except in the macro that runs before a save (the registered macro of
    # \u{4FDD}\u{5B58}\u{524D}), where it is the file being saved.
    'GetSaveFilename'
    # ChangeInsertSpace(value: int) -> int
    # Whether the Tab key inserts spaces instead of a tab character, before the call (1 or 0). 0
    # turns it off, 1 turns it on for this window (not saved); any other value only reads. Only
    # the low 16 bits of the value count, as in Sakura.
    'ChangeInsertSpace'
    # LayoutToLogicLineNum(layoutLine: int) -> int
    # The logical line (up to a line break) that a layout line (a row on the screen) belongs to,
    # both counted from 1. The line after the last layout line gives the line after the last
    # logical line. Any other line outside the text is an error.
    'LayoutToLogicLineNum'
    # LogicToLayoutLineNum(logicLine: int, logicIndex: int) -> int
    # The layout line (a row on the screen) of a position in a logical line: the line and the
    # index in it (UTF-16 code units), both counted from 1. A line past the end gives the line
    # after the last layout line.
    'LogicToLayoutLineNum'
    # LineColumnToIndex(layoutLine: int, layoutColumn: int) -> int
    # The index (UTF-16 code units, from 1) in the logical line of a layout line and a layout
    # column (cells), both from 1. A column past the text gives the end of the text of the row.
    'LineColumnToIndex'
    # LineIndexToColumn(logicLine: int, logicIndex: int) -> int
    # The layout column (cells, from 1) of a position in a logical line: the line and the index
    # in it (UTF-16 code units), both from 1. Tabs and full-width characters take their width.
    'LineIndexToColumn'
    # GetStrWidth(text: string, column: int?) -> int
    # The width of a string in cells, starting at layout column `column` (from 1, default 1; it
    # matters for tabs). A CR LF at the end counts as the CR alone.
    'GetStrWidth'
    # GetStrLayoutLength(text: string, column: int?) -> int
    # The width of a string as a line of the document in cells, starting at layout column
    # `column` (from 1, default 1). A line ending at the end takes one cell.
    'GetStrLayoutLength'
    # GetDefaultCharLength() -> int
    # The layout width of a half-width character. This editor lays out in cells, so it is 1.
    'GetDefaultCharLength'
    # GetViewLines() -> int
    # How many rows the text area shows (at least 1).
    'GetViewLines'
    # GetViewColumns() -> int
    # How many columns the text area shows (at least 1).
    'GetViewColumns'
    # GetViewTop() -> int
    # The layout line at the top of the text area, counted from 1.
    'GetViewTop'
    # ChangeTabWidth(width: int) -> int
    # The tab width before the call. A width from 1 to 64 that differs from it becomes the tab
    # width of this window (not saved). Any other value, such as 0, only reads.
    'ChangeTabWidth'
    # ChangeWrapColm(column: int) -> int
    # The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
    # column (not saved). Any other value only reads. Same as ChangeWrapColumn.
    'ChangeWrapColm'
    # ChangeWrapColumn(column: int) -> int
    # The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
    # column (not saved). Any other value only reads.
    'ChangeWrapColumn'
    # GetClipboard(option: int?) -> string
    # The text on the clipboard, or an empty string when there is none. The line endings are
    # those of the source. The option is reserved.
    'GetClipboard'
    # SetClipboard(option: int?, text: string?) -> int
    # Puts the text on the clipboard. Option bit 0 marks a box copy (a paste puts it in as a
    # box), bit 1 a row copy (a paste puts it in front of the row); with both, the box wins.
    # Returns 1 when it worked, 0 when not.
    'SetClipboard'
    # IsIncludeClipboardFormat(format: string) -> int
    # 1 when the clipboard has the format. Only text is known: CF_TEXT, CF_OEMTEXT and
    # CF_UNICODETEXT (or 1, 7, 13) are 1 when the clipboard has text; every other format is 0.
    'IsIncludeClipboardFormat'
    # GetClipboardByFormat(format: string, mode: int, endMode: int?) -> string
    # The text of the clipboard in a text format (CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT), with the
    # line endings of the document. Mode -2 is the usual way and 3 is UTF-16 (the text itself);
    # the other modes and formats are errors (-32001). The end mode is not used.
    'GetClipboardByFormat'
    # SetClipboardByFormat(text: string, format: string, mode: int, endMode: int?) -> int
    # Puts the text on the clipboard as text. The formats and modes are those of
    # GetClipboardByFormat. Returns 1 when it worked.
    'SetClipboardByFormat'
    # GetCookie(scope: string, name: string) -> string
    # The value of a cookie, or an empty string when there is none. The scope is "window" or
    # "document".
    'GetCookie'
    # GetCookieDefault(scope: string, name: string, defaultValue: string) -> string
    # The value of a cookie, or the default when there is none (an unknown scope still gives an
    # empty string). The scope is "window" or "document".
    'GetCookieDefault'
    # SetCookie(scope: string, name: string, value: string) -> int
    # Sets a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a name with a
    # character that a name cannot have (Sakura lets in digits, letters and every character from
    # "_" up in the character table). The scope is "window" or "document".
    'SetCookie'
    # DeleteCookie(scope: string, name: string) -> int
    # Deletes a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a bad name, 5
    # when there is no such cookie. The scope is "window" or "document".
    'DeleteCookie'
    # GetCookieNames(scope: string) -> string
    # The names of the cookies of a scope, joined with commas, in the order of the names (UTF-16
    # code units). An unknown scope gives an empty string. The scope is "window" or "document".
    'GetCookieNames'
    # CompareVersion(versionA: string, versionB: string) -> int
    # Compares two version strings: 0 when equal, positive when A is newer, negative when B is.
    # Four parts at most; a part has two digits at most; "alpha", "beta", "rc" and "pl" put a
    # part before or after the release (like PHP's version_compare).
    'CompareVersion'
    # SetDrawSwitch(on: int) -> int
    # Turns the draw switch on (non-zero) or off (0) and returns the state before (1 or 0). The
    # editor keeps the flag for GetDrawSwitch but still paints the text when it likes.
    'SetDrawSwitch'
    # GetDrawSwitch() -> int
    # 1 when the draw switch is on (the start state), 0 when a macro turned it off.
    'GetDrawSwitch'
    # IsShownStatus() -> int
    # 1 when the status bar is shown, else 0.
    'IsShownStatus'
    # IsCurTypeExt(extension: string) -> int
    # 1 when the document type of this file is the type of the extension (looked up in the
    # extension lists of the types in the settings file, as a file name). Without type settings
    # every extension is the basic type.
    'IsCurTypeExt'
    # IsSameTypeExt(extension1: string, extension2: string) -> int
    # 1 when the two extensions belong to the same document type (see IsCurTypeExt).
    'IsSameTypeExt'
    # ExpandParameter(format: string) -> string
    # Replaces the codes that start with $ in a string: $$ ($), $F (full path), $f (file name),
    # $g (file name without the extension), $/ (full path with /), $N (as $F), $b (extension),
    # $e and $E (folder), $C (the selected text or the word at the caret), $x and $y (caret
    # column in UTF-16 units and line, from 1), $S (the editor), $M (the macro file), $V
    # (version), $A (name), and conditions ${R?a$:b$:c$} (R, w, M, U, N, I). A text without a
    # file is (\u{7121}\u{984C}). $d $t $D $T $B $Q $I are not supported and stay as written. At most 2047
    # UTF-16 units.
    'ExpandParameter'
    # MessageBox(message: string, type: int?) -> int
    # Shows a message box and waits. The type is Windows' MB_ value: the low nibble chooses the
    # buttons (0 OK, 1 OK/Cancel, 2 Abort/Retry/Ignore, 3 Yes/No/Cancel, 4 Yes/No, 5
    # Retry/Cancel, 6 Cancel/Try again/Continue), the icon bits (0x10 stop, 0x20 question, 0x30
    # exclamation, 0x40 information) the icon; the default button and the modality are ignored.
    # Returns the button as Windows numbers it: 1 OK, 2 Cancel, 3 Abort, 4 Retry, 5 Ignore, 6
    # Yes, 7 No, 10 Try again, 11 Continue. A box that is closed without an answer gives Cancel
    # (the last button when there is none).
    'MessageBox'
    # ErrorMsg(message: string) -> int
    # Shows an error message with an OK button and waits. Returns 1.
    'ErrorMsg'
    # WarnMsg(message: string) -> int
    # Shows a warning with an OK button and waits. Returns 1.
    'WarnMsg'
    # InfoMsg(message: string) -> int
    # Shows an information message with an OK button and waits. Returns 1.
    'InfoMsg'
    # OkCancelBox(message: string) -> int
    # Asks a question with OK and Cancel and waits. Returns 1 (OK) or 2 (Cancel).
    'OkCancelBox'
    # YesNoBox(message: string) -> int
    # Asks a question with Yes and No and waits. Returns 6 (Yes) or 7 (No).
    'YesNoBox'
    # FileOpenDialog(defaultName: string?, filter: string?) -> string
    # Lets the user choose a file to open and returns its full path, or an empty string when the
    # dialog is cancelled. The default name and the filter are not used: the dialog of the
    # platform takes neither.
    'FileOpenDialog'
    # FileSaveDialog(defaultName: string?, filter: string?) -> string
    # Lets the user choose a file name to save under and returns the full path, or an empty
    # string when the dialog is cancelled. The dialog starts with the default name. The filter
    # is not used.
    'FileSaveDialog'
    # FolderDialog(message: string?, defaultFolder: string?) -> string
    # Lets the user choose a folder and returns its full path, or an empty string when the
    # dialog is cancelled. The message becomes the prompt text of the platform dialog where it
    # has one; the default folder is not used.
    'FolderDialog'
    # InputBox(message: string, defaultValue: string?, maxLength: int?) -> string
    # Shows a dialog with a text field and waits. The message is shown above the field, the
    # default value is in the field, and the text cannot be longer than the maximum length (in
    # UTF-16 code units; 0 or less, or no value, is 260). Returns the text of the field when OK
    # is pressed, or an empty string when the dialog is cancelled.
    'InputBox'
    # CreateMenu(position: int, menu: string) -> int
    # Shows a pop-up menu and waits. The position is 1 for the caret (the pointer when the caret
    # is out of the view) and anything else for the pointer. The menu is a list of items
    # separated by commas; an item can start with flags in brackets: C checked, D disabled, R
    # radio, S opens a submenu, E ends a submenu (one for each E), B (a new column) is ignored.
    # A lone - is a separator. Write [][text] for a text that starts with a bracket. Returns the
    # number of the item chosen, counted from 1 over the items only (separators and submenus are
    # not counted), or 0 when the menu is closed without a choice or has no item.
    'CreateMenu'
    # Sleep(milliseconds: int) -> int
    # Waits for the given number of milliseconds (the window stays alive, and the macro can be
    # cancelled). Returns 0.
    'Sleep'
    # FileNew() -> null
    # Sakura's F_FILENEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'FileNew'
    # FileOpen(arg1: string?, arg2: int?, arg3: int?, arg4: string?) -> null
    # MATSU Editor's own F_FILEOPEN2 (not in Sakura). Runs the command that a key, the menu and
    # a key macro run.
    'FileOpen'
    # FileSave() -> null
    # Sakura's F_FILESAVE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'FileSave'
    # FileSaveAll() -> null
    # Sakura's F_FILESAVEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileSaveAll'
    # FileSaveAsDialog(arg1: string?, arg2: int?, arg3: int?) -> null
    # Sakura's F_FILESAVEAS_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileSaveAsDialog'
    # FileSaveAs(arg1: string?, arg2: int?, arg3: int?) -> null
    # Sakura's F_FILESAVEAS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'FileSaveAs'
    # FileClose() -> null
    # Sakura's F_FILECLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'FileClose'
    # FileCloseOpen() -> null
    # Sakura's F_FILECLOSE_OPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileCloseOpen'
    # FileReopen(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopen'
    # FileReopenSJIS(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopenSJIS'
    # FileReopenJIS(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopenJIS'
    # FileReopenEUC(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopenEUC'
    # FileReopenLatin1(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_LATIN1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopenLatin1'
    # FileReopenUNICODE(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_UNICODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopenUNICODE'
    # FileReopenUNICODEBE(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_UNICODEBE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
    # a key, the menu and a key macro run. The last arguments may be left out. A command that
    # this editor does not have yet answers the error -32001 (not implemented).
    'FileReopenUNICODEBE'
    # FileReopenUTF8(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopenUTF8'
    # FileReopenCESU8(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_CESU8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopenCESU8'
    # FileReopenUTF7(arg1: int?) -> null
    # Sakura's F_FILE_REOPEN_UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FileReopenUTF7'
    # Print() -> null
    # Sakura's F_PRINT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Print'
    # PrintPreview() -> null
    # Sakura's F_PRINT_PREVIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'PrintPreview'
    # PrintPageSetup() -> null
    # Sakura's F_PRINT_PAGESETUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'PrintPageSetup'
    # OpenHfromtoC() -> null
    # Sakura's F_OPEN_HfromtoC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'OpenHfromtoC'
    # ActivateSQLPLUS() -> null
    # Sakura's F_ACTIVATE_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ActivateSQLPLUS'
    # ExecSQLPLUS() -> null
    # Sakura's F_PLSQL_COMPILE_ON_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    'ExecSQLPLUS'
    # ExecDatabaseChoice() -> null
    # MATSU Editor's own F_PLSQL_COMPILE_ON_DATABASE_CHOICE (not in Sakura). Runs the command
    # that a key, the menu and a key macro run.
    'ExecDatabaseChoice'
    # Browse() -> null
    # Sakura's F_BROWSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Browse'
    # ViewMode() -> null
    # Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ViewMode'
    # ReadOnly() -> null
    # Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ReadOnly'
    # PropertyFile() -> null
    # Sakura's F_PROPERTY_FILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'PropertyFile'
    # ExitAllEditors() -> null
    # Sakura's F_EXITALLEDITORS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ExitAllEditors'
    # ExitAll() -> null
    # Sakura's F_EXITALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ExitAll'
    # PutFile(arg1: string?, arg2: int?, arg3: int?) -> null
    # MATSU Editor's own F_PUTFILE (not in Sakura). Runs the command that a key, the menu and a
    # key macro run.
    'PutFile'
    # InsFile(arg1: string?, arg2: int?, arg3: int?) -> null
    # MATSU Editor's own F_INSFILE (not in Sakura). Runs the command that a key, the menu and a
    # key macro run.
    'InsFile'
    # Char(arg1: int?) -> null
    # Sakura's F_WCHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Char'
    # CharIme(arg1: int?) -> null
    # Sakura's F_IME_CHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CharIme'
    # Undo() -> null
    # Sakura's F_UNDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Undo'
    # Redo() -> null
    # Sakura's F_REDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Redo'
    # Delete() -> null
    # Sakura's F_DELETE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Delete'
    # DeleteBack() -> null
    # Sakura's F_DELETE_BACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'DeleteBack'
    # WordDeleteToStart() -> null
    # Sakura's F_WordDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WordDeleteToStart'
    # WordDeleteToEnd() -> null
    # Sakura's F_WordDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WordDeleteToEnd'
    # WordCut() -> null
    # Sakura's F_WordCut (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WordCut'
    # WordDelete() -> null
    # Sakura's F_WordDelete (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WordDelete'
    # LineCutToStart() -> null
    # Sakura's F_LineCutToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'LineCutToStart'
    # LineCutToEnd() -> null
    # Sakura's F_LineCutToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'LineCutToEnd'
    # LineDeleteToStart() -> null
    # Sakura's F_LineDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'LineDeleteToStart'
    # LineDeleteToEnd() -> null
    # Sakura's F_LineDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'LineDeleteToEnd'
    # CutLine() -> null
    # Sakura's F_CUT_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CutLine'
    # DeleteLine() -> null
    # Sakura's F_DELETE_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'DeleteLine'
    # DuplicateLine() -> null
    # Sakura's F_DUPLICATELINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'DuplicateLine'
    # IndentTab() -> null
    # Sakura's F_INDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'IndentTab'
    # UnindentTab() -> null
    # Sakura's F_UNINDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'UnindentTab'
    # IndentSpace() -> null
    # Sakura's F_INDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'IndentSpace'
    # UnindentSpace() -> null
    # Sakura's F_UNINDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'UnindentSpace'
    # LTrim() -> null
    # Sakura's F_LTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'LTrim'
    # RTrim() -> null
    # Sakura's F_RTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'RTrim'
    # SortAsc() -> null
    # Sakura's F_SORT_ASC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'SortAsc'
    # SortDesc() -> null
    # Sakura's F_SORT_DESC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'SortDesc'
    # Merge() -> null
    # Sakura's F_MERGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Merge'
    # Left() -> null
    # Sakura's F_LEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Left'
    # Right() -> null
    # Sakura's F_RIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Right'
    # Up2() -> null
    # Sakura's F_UP2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Up2'
    # Down2() -> null
    # Sakura's F_DOWN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Down2'
    # WordLeft() -> null
    # Sakura's F_WORDLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WordLeft'
    # WordRight() -> null
    # Sakura's F_WORDRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WordRight'
    # GoLineTop(arg1: int?) -> null
    # Sakura's F_GOLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'GoLineTop'
    # GoLineEnd(arg1: int?) -> null
    # Sakura's F_GOLINEEND (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'GoLineEnd'
    # HalfPageUp(arg1: int?) -> null
    # Sakura's F_HalfPageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'HalfPageUp'
    # HalfPageDown(arg1: int?) -> null
    # Sakura's F_HalfPageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HalfPageDown'
    # PageUp(arg1: int?) -> null
    # Sakura's F_1PageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'PageUp'
    # PageDown(arg1: int?) -> null
    # Sakura's F_1PageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'PageDown'
    # CurLineCenter() -> null
    # Sakura's F_CURLINECENTER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CurLineCenter'
    # CurLineTop() -> null
    # Sakura's F_CURLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CurLineTop'
    # CurLineBottom() -> null
    # Sakura's F_CURLINEBOTTOM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CurLineBottom'
    # MoveHistPrev() -> null
    # Sakura's F_JUMPHIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'MoveHistPrev'
    # MoveHistNext() -> null
    # Sakura's F_JUMPHIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'MoveHistNext'
    # MoveHistSet() -> null
    # Sakura's F_JUMPHIST_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'MoveHistSet'
    # F_WndScrollDown() -> null
    # Sakura's F_WndScrollDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'F_WndScrollDown'
    # F_WndScrollUp() -> null
    # Sakura's F_WndScrollUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'F_WndScrollUp'
    # GoNextParagraph() -> null
    # Sakura's F_GONEXTPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoNextParagraph'
    # GoPrevParagraph() -> null
    # Sakura's F_GOPREVPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoPrevParagraph'
    # GoModifyLineNext() -> null
    # Sakura's F_MODIFYLINE_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoModifyLineNext'
    # GoModifyLinePrev() -> null
    # Sakura's F_MODIFYLINE_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoModifyLinePrev'
    # MoveCursorLayout(arg1: int?, arg2: int?, arg3: int?) -> null
    # MATSU Editor's own F_MOVECURSORLAYOUT (not in Sakura). Runs the command that a key, the
    # menu and a key macro run.
    'MoveCursorLayout'
    # WheelUp(arg1: int?) -> null
    # Sakura's F_WHEELUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WheelUp'
    # WheelDown(arg1: int?) -> null
    # Sakura's F_WHEELDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WheelDown'
    # WheelLeft(arg1: int?) -> null
    # Sakura's F_WHEELLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WheelLeft'
    # WheelRight(arg1: int?) -> null
    # Sakura's F_WHEELRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WheelRight'
    # WheelPageUp(arg1: int?) -> null
    # Sakura's F_WHEELPAGEUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WheelPageUp'
    # WheelPageDown(arg1: int?) -> null
    # Sakura's F_WHEELPAGEDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WheelPageDown'
    # WheelPageLeft(arg1: int?) -> null
    # Sakura's F_WHEELPAGELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WheelPageLeft'
    # WheelPageRight(arg1: int?) -> null
    # Sakura's F_WHEELPAGERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WheelPageRight'
    # SelectWord() -> null
    # Sakura's F_SELECTWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'SelectWord'
    # SelectLine(arg1: int?) -> null
    # Sakura's F_SELECTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'SelectLine'
    # BeginSelect() -> null
    # Sakura's F_BEGIN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'BeginSelect'
    # Up_Sel() -> null
    # Sakura's F_UP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Up_Sel'
    # Down_Sel() -> null
    # Sakura's F_DOWN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Down_Sel'
    # Left_Sel() -> null
    # Sakura's F_LEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Left_Sel'
    # Right_Sel() -> null
    # Sakura's F_RIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Right_Sel'
    # Up2_Sel() -> null
    # Sakura's F_UP2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Up2_Sel'
    # Down2_Sel() -> null
    # Sakura's F_DOWN2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Down2_Sel'
    # WordLeft_Sel() -> null
    # Sakura's F_WORDLEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WordLeft_Sel'
    # WordRight_Sel() -> null
    # Sakura's F_WORDRIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WordRight_Sel'
    # GoLineTop_Sel(arg1: int?) -> null
    # Sakura's F_GOLINETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoLineTop_Sel'
    # GoLineEnd_Sel(arg1: int?) -> null
    # Sakura's F_GOLINEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoLineEnd_Sel'
    # HalfPageUp_Sel(arg1: int?) -> null
    # Sakura's F_HalfPageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HalfPageUp_Sel'
    # HalfPageDown_Sel(arg1: int?) -> null
    # Sakura's F_HalfPageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HalfPageDown_Sel'
    # PageUp_Sel(arg1: int?) -> null
    # Sakura's F_1PageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'PageUp_Sel'
    # PageDown_Sel(arg1: int?) -> null
    # Sakura's F_1PageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'PageDown_Sel'
    # GoFileTop_Sel() -> null
    # Sakura's F_GOFILETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoFileTop_Sel'
    # GoFileEnd_Sel() -> null
    # Sakura's F_GOFILEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoFileEnd_Sel'
    # GoNextParagraph_Sel() -> null
    # Sakura's F_GONEXTPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoNextParagraph_Sel'
    # GoPrevParagraph_Sel() -> null
    # Sakura's F_GOPREVPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoPrevParagraph_Sel'
    # GoModifyLineNext_Sel() -> null
    # Sakura's F_MODIFYLINE_NEXT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoModifyLineNext_Sel'
    # GoModifyLinePrev_Sel() -> null
    # Sakura's F_MODIFYLINE_PREV_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoModifyLinePrev_Sel'
    # BeginBoxSelect() -> null
    # Sakura's F_BEGIN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'BeginBoxSelect'
    # Up_BoxSel(arg1: int?) -> null
    # Sakura's F_UP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Up_BoxSel'
    # Down_BoxSel(arg1: int?) -> null
    # Sakura's F_DOWN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Down_BoxSel'
    # Left_BoxSel(arg1: int?) -> null
    # Sakura's F_LEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Left_BoxSel'
    # Right_BoxSel(arg1: int?) -> null
    # Sakura's F_RIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Right_BoxSel'
    # Up2_BoxSel(arg1: int?) -> null
    # Sakura's F_UP2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Up2_BoxSel'
    # Down2_BoxSel(arg1: int?) -> null
    # Sakura's F_DOWN2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Down2_BoxSel'
    # WordLeft_BoxSel(arg1: int?) -> null
    # Sakura's F_WORDLEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WordLeft_BoxSel'
    # WordRight_BoxSel(arg1: int?) -> null
    # Sakura's F_WORDRIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WordRight_BoxSel'
    # GoLogicalLineTop_BoxSel(arg1: int?, arg2: int?) -> null
    # Sakura's F_GOLOGICALLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
    # a key, the menu and a key macro run. The last arguments may be left out. A command that
    # this editor does not have yet answers the error -32001 (not implemented).
    'GoLogicalLineTop_BoxSel'
    # GoLineTop_BoxSel(arg1: int?, arg2: int?) -> null
    # Sakura's F_GOLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoLineTop_BoxSel'
    # GoLineEnd_BoxSel(arg1: int?, arg2: int?) -> null
    # Sakura's F_GOLINEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoLineEnd_BoxSel'
    # HalfPageUp_BoxSel(arg1: int?, arg2: int?) -> null
    # Sakura's F_HalfPageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HalfPageUp_BoxSel'
    # HalfPageDown_BoxSel(arg1: int?, arg2: int?) -> null
    # Sakura's F_HalfPageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HalfPageDown_BoxSel'
    # PageUp_BoxSel(arg1: int?, arg2: int?) -> null
    # Sakura's F_1PageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'PageUp_BoxSel'
    # PageDown_BoxSel(arg1: int?, arg2: int?) -> null
    # Sakura's F_1PageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'PageDown_BoxSel'
    # GoFileTop_BoxSel(arg1: int?) -> null
    # Sakura's F_GOFILETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoFileTop_BoxSel'
    # GoFileEnd_BoxSel(arg1: int?) -> null
    # Sakura's F_GOFILEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GoFileEnd_BoxSel'
    # Cut() -> null
    # Sakura's F_CUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Cut'
    # Copy() -> null
    # Sakura's F_COPY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Copy'
    # Paste(arg1: int?) -> null
    # Sakura's F_PASTE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Paste'
    # CopyAddCRLF() -> null
    # Sakura's F_COPY_ADDCRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CopyAddCRLF'
    # CopyCRLF() -> null
    # Sakura's F_COPY_CRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CopyCRLF'
    # CopyLF() -> null
    # MATSU Editor's own F_COPY_LF (not in Sakura). Runs the command that a key, the menu and a
    # key macro run.
    'CopyLF'
    # CopyCR() -> null
    # MATSU Editor's own F_COPY_CR (not in Sakura). Runs the command that a key, the menu and a
    # key macro run.
    'CopyCR'
    # PasteBox(arg1: int?) -> null
    # Sakura's F_PASTEBOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'PasteBox'
    # InsBoxText(arg1: string?) -> null
    # MATSU Editor's own F_INSBOXTEXT (not in Sakura). Runs the command that a key, the menu and
    # a key macro run.
    'InsBoxText'
    # CopyLines() -> null
    # Sakura's F_COPYLINES (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CopyLines'
    # CopyLinesAsPassage() -> null
    # Sakura's F_COPYLINESASPASSAGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CopyLinesAsPassage'
    # CopyLinesWithLineNumber() -> null
    # Sakura's F_COPYLINESWITHLINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    'CopyLinesWithLineNumber'
    # CopyColorHtml() -> null
    # Sakura's F_COPY_COLOR_HTML (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CopyColorHtml'
    # CopyColorHtmlWithLineNumber() -> null
    # Sakura's F_COPY_COLOR_HTML_LINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    'CopyColorHtmlWithLineNumber'
    # CopyPath() -> null
    # Sakura's F_COPYPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CopyPath'
    # CopyDirPath() -> null
    # Sakura's F_COPYDIRPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CopyDirPath'
    # CopyFilename() -> null
    # Sakura's F_COPYFNAME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CopyFilename'
    # CopyTag() -> null
    # Sakura's F_COPYTAG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CopyTag'
    # CopyKeyBindList() -> null
    # Sakura's F_CREATEKEYBINDLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CopyKeyBindList'
    # InsertDate() -> null
    # Sakura's F_INS_DATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'InsertDate'
    # InsertTime() -> null
    # Sakura's F_INS_TIME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'InsertTime'
    # CtrlCodeDialog() -> null
    # Sakura's F_CTRL_CODE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CtrlCodeDialog'
    # CtrlCode(arg1: int?) -> null
    # Sakura's F_CTRL_CODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CtrlCode'
    # InsertFileUsedRecently() -> null
    # Sakura's F_INS_FILE_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    'InsertFileUsedRecently'
    # InsertFolderUsedRecently() -> null
    # Sakura's F_INS_FOLDER_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    'InsertFolderUsedRecently'
    # ToLower() -> null
    # Sakura's F_TOLOWER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ToLower'
    # ToUpper() -> null
    # Sakura's F_TOUPPER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ToUpper'
    # ToHankaku() -> null
    # Sakura's F_TOHANKAKU (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ToHankaku'
    # ToHankata() -> null
    # Sakura's F_TOHANKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ToHankata'
    # ToZenEi() -> null
    # Sakura's F_TOZENEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ToZenEi'
    # ToHanEi() -> null
    # Sakura's F_TOHANEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ToHanEi'
    # ToZenKata() -> null
    # Sakura's F_TOZENKAKUKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ToZenKata'
    # ToZenHira() -> null
    # Sakura's F_TOZENKAKUHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ToZenHira'
    # HanKataToZenKata() -> null
    # Sakura's F_HANKATATOZENKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HanKataToZenKata'
    # HanKataToZenHira() -> null
    # Sakura's F_HANKATATOZENHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HanKataToZenHira'
    # TABToSPACE() -> null
    # Sakura's F_TABTOSPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'TABToSPACE'
    # SPACEToTAB() -> null
    # Sakura's F_SPACETOTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'SPACEToTAB'
    # AutoToSJIS() -> null
    # Sakura's F_CODECNV_AUTO2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'AutoToSJIS'
    # JIStoSJIS() -> null
    # Sakura's F_CODECNV_EMAIL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'JIStoSJIS'
    # EUCtoSJIS() -> null
    # Sakura's F_CODECNV_EUC2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'EUCtoSJIS'
    # CodeCnvUNICODEtoSJIS() -> null
    # Sakura's F_CODECNV_UNICODE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
    # a key, the menu and a key macro run. The last arguments may be left out. A command that
    # this editor does not have yet answers the error -32001 (not implemented).
    'CodeCnvUNICODEtoSJIS'
    # CodeCnvUNICODEBEtoSJIS() -> null
    # Sakura's F_CODECNV_UNICODEBE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    'CodeCnvUNICODEBEtoSJIS'
    # UTF8toSJIS() -> null
    # Sakura's F_CODECNV_UTF82SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'UTF8toSJIS'
    # UTF7toSJIS() -> null
    # Sakura's F_CODECNV_UTF72SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'UTF7toSJIS'
    # SJIStoJIS() -> null
    # Sakura's F_CODECNV_SJIS2JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SJIStoJIS'
    # SJIStoEUC() -> null
    # Sakura's F_CODECNV_SJIS2EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SJIStoEUC'
    # SJIStoUTF8() -> null
    # Sakura's F_CODECNV_SJIS2UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SJIStoUTF8'
    # SJIStoUTF7() -> null
    # Sakura's F_CODECNV_SJIS2UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SJIStoUTF7'
    # Base64Decode() -> null
    # Sakura's F_BASE64DECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'Base64Decode'
    # Uudecode() -> null
    # Sakura's F_UUDECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Uudecode'
    # SearchDialog() -> null
    # Sakura's F_SEARCH_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SearchDialog'
    # SearchNext(arg1: string?, arg2: int?) -> null
    # Sakura's F_SEARCH_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SearchNext'
    # SearchPrev(arg1: string?, arg2: int?) -> null
    # Sakura's F_SEARCH_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SearchPrev'
    # ReplaceDialog() -> null
    # Sakura's F_REPLACE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ReplaceDialog'
    # Replace(arg1: string?, arg2: string?, arg3: int?) -> null
    # Sakura's F_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Replace'
    # ReplaceAll(arg1: string?, arg2: string?, arg3: int?) -> null
    # Sakura's F_REPLACE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ReplaceAll'
    # SearchClearMark() -> null
    # Sakura's F_SEARCH_CLEARMARK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SearchClearMark'
    # SearchStartPos() -> null
    # Sakura's F_JUMP_SRCHSTARTPOS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SearchStartPos'
    # Grep(arg1: string?, arg2: string?, arg3: string?, arg4: int?, arg5: int?) -> null
    # Sakura's F_GREP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Grep'
    # GrepReplace(arg1: string?, arg2: string?, arg3: string?, arg4: string?, arg5: int?, arg6: int?) -> null
    # Sakura's F_GREP_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'GrepReplace'
    # Outline(arg1: int?) -> null
    # Sakura's F_OUTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Outline'
    # TagJump() -> null
    # Sakura's F_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'TagJump'
    # TagJumpBack() -> null
    # Sakura's F_TAGJUMPBACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TagJumpBack'
    # TagMake() -> null
    # Sakura's F_TAGS_MAKE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'TagMake'
    # DirectTagJump() -> null
    # Sakura's F_DIRECT_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'DirectTagJump'
    # KeywordTagJump(arg1: string?) -> null
    # Sakura's F_TAGJUMP_KEYWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'KeywordTagJump'
    # Compare() -> null
    # Sakura's F_COMPARE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'Compare'
    # DiffDialog() -> null
    # Sakura's F_DIFF_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'DiffDialog'
    # Diff(arg1: string?, arg2: int?) -> null
    # Sakura's F_DIFF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Diff'
    # DiffNext() -> null
    # Sakura's F_DIFF_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'DiffNext'
    # DiffPrev() -> null
    # Sakura's F_DIFF_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'DiffPrev'
    # DiffReset() -> null
    # Sakura's F_DIFF_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'DiffReset'
    # BracketPair() -> null
    # Sakura's F_BRACKETPAIR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'BracketPair'
    # BookmarkSet() -> null
    # Sakura's F_BOOKMARK_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'BookmarkSet'
    # BookmarkNext() -> null
    # Sakura's F_BOOKMARK_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'BookmarkNext'
    # BookmarkPrev() -> null
    # Sakura's F_BOOKMARK_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'BookmarkPrev'
    # BookmarkReset() -> null
    # Sakura's F_BOOKMARK_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'BookmarkReset'
    # BookmarkView() -> null
    # Sakura's F_BOOKMARK_VIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'BookmarkView'
    # BookmarkPattern(arg1: string?, arg2: int?) -> null
    # Sakura's F_BOOKMARK_PATTERN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'BookmarkPattern'
    # FuncListNext() -> null
    # Sakura's F_FUNCLIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FuncListNext'
    # FuncListPrev() -> null
    # Sakura's F_FUNCLIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'FuncListPrev'
    # ChgmodINS() -> null
    # Sakura's F_CHGMOD_INS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ChgmodINS'
    # ChgCharSet(arg1: int?, arg2: int?) -> null
    # Sakura's F_CHG_CHARSET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ChgCharSet'
    # ChgmodEOL(arg1: int?) -> null
    # MATSU Editor's own F_CHGMOD_EOL (not in Sakura). Runs the command that a key, the menu and
    # a key macro run.
    'ChgmodEOL'
    # CancelMode() -> null
    # Sakura's F_CANCEL_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CancelMode'
    # ExecExternalMacro(arg1: string?, arg2: string?) -> null
    # Sakura's F_EXECEXTMACRO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ExecExternalMacro'
    # ShowToolbar() -> null
    # Sakura's F_SHOWTOOLBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ShowToolbar'
    # ShowFunckey() -> null
    # Sakura's F_SHOWFUNCKEY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ShowFunckey'
    # ShowTab() -> null
    # Sakura's F_SHOWTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ShowTab'
    # ShowStatusbar() -> null
    # Sakura's F_SHOWSTATUSBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ShowStatusbar'
    # ShowMiniMap() -> null
    # Sakura's F_SHOWMINIMAP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ShowMiniMap'
    # TypeList() -> null
    # Sakura's F_TYPE_LIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'TypeList'
    # ChangeType(arg1: int?) -> null
    # MATSU Editor's own F_CHANGETYPE (not in Sakura). Runs the command that a key, the menu and
    # a key macro run.
    'ChangeType'
    # OptionType() -> null
    # Sakura's F_OPTION_TYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'OptionType'
    # OptionCommon() -> null
    # Sakura's F_OPTION (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'OptionCommon'
    # SelectFont() -> null
    # Sakura's F_FONT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'SelectFont'
    # SetFontSize(arg1: int?, arg2: int?, arg3: int?) -> null
    # Sakura's F_SETFONTSIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SetFontSize'
    # WrapWindowWidth() -> null
    # Sakura's F_WRAPWINDOWWIDTH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WrapWindowWidth'
    # OptionFavorite() -> null
    # Sakura's F_FAVORITE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'OptionFavorite'
    # SetMsgQuoteStr(arg1: string?) -> null
    # MATSU Editor's own F_SET_QUOTESTRING (not in Sakura). Runs the command that a key, the
    # menu and a key macro run.
    'SetMsgQuoteStr'
    # TextWrapMethod(arg1: int?) -> null
    # MATSU Editor's own F_TEXTWRAPMETHOD (not in Sakura). Runs the command that a key, the menu
    # and a key macro run.
    'TextWrapMethod'
    # SelectCountMode(arg1: int?) -> null
    # Sakura's F_SELECT_COUNT_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'SelectCountMode'
    # ExecCommand(arg1: string?, arg2: int?, arg3: string?) -> null
    # Sakura's F_EXECMD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ExecCommand'
    # ExecCommandDialog() -> null
    # Sakura's F_EXECMD_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ExecCommandDialog'
    # RMenu() -> null
    # Sakura's F_MENU_RBUTTON (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'RMenu'
    # CustMenu1() -> null
    # Sakura's F_CUSTMENU_1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu1'
    # CustMenu2() -> null
    # Sakura's F_CUSTMENU_2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu2'
    # CustMenu3() -> null
    # Sakura's F_CUSTMENU_3 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu3'
    # CustMenu4() -> null
    # Sakura's F_CUSTMENU_4 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu4'
    # CustMenu5() -> null
    # Sakura's F_CUSTMENU_5 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu5'
    # CustMenu6() -> null
    # Sakura's F_CUSTMENU_6 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu6'
    # CustMenu7() -> null
    # Sakura's F_CUSTMENU_7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu7'
    # CustMenu8() -> null
    # Sakura's F_CUSTMENU_8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu8'
    # CustMenu9() -> null
    # Sakura's F_CUSTMENU_9 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CustMenu9'
    # CustMenu10() -> null
    # Sakura's F_CUSTMENU_10 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu10'
    # CustMenu11() -> null
    # Sakura's F_CUSTMENU_11 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu11'
    # CustMenu12() -> null
    # Sakura's F_CUSTMENU_12 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu12'
    # CustMenu13() -> null
    # Sakura's F_CUSTMENU_13 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu13'
    # CustMenu14() -> null
    # Sakura's F_CUSTMENU_14 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu14'
    # CustMenu15() -> null
    # Sakura's F_CUSTMENU_15 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu15'
    # CustMenu16() -> null
    # Sakura's F_CUSTMENU_16 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu16'
    # CustMenu17() -> null
    # Sakura's F_CUSTMENU_17 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu17'
    # CustMenu18() -> null
    # Sakura's F_CUSTMENU_18 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu18'
    # CustMenu19() -> null
    # Sakura's F_CUSTMENU_19 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu19'
    # CustMenu20() -> null
    # Sakura's F_CUSTMENU_20 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu20'
    # CustMenu21() -> null
    # Sakura's F_CUSTMENU_21 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu21'
    # CustMenu22() -> null
    # Sakura's F_CUSTMENU_22 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu22'
    # CustMenu23() -> null
    # Sakura's F_CUSTMENU_23 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu23'
    # CustMenu24() -> null
    # Sakura's F_CUSTMENU_24 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CustMenu24'
    # SplitWinV() -> null
    # Sakura's F_SPLIT_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'SplitWinV'
    # SplitWinH() -> null
    # Sakura's F_SPLIT_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'SplitWinH'
    # SplitWinVH() -> null
    # Sakura's F_SPLIT_VH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'SplitWinVH'
    # WinMaximize() -> null
    # MATSU Editor's own F_WINMAXIMIZE (not in Sakura). Runs the command that a key, the menu
    # and a key macro run.
    'WinMaximize'
    # WinMinimize() -> null
    # MATSU Editor's own F_WINMINIMIZE (not in Sakura). Runs the command that a key, the menu
    # and a key macro run.
    'WinMinimize'
    # WinRestore() -> null
    # MATSU Editor's own F_WINRESTORE (not in Sakura). Runs the command that a key, the menu and
    # a key macro run.
    'WinRestore'
    # WinClose() -> null
    # Sakura's F_WINCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WinClose'
    # WinCloseAll() -> null
    # Sakura's F_WIN_CLOSEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'WinCloseAll'
    # CascadeWin() -> null
    # Sakura's F_CASCADE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'CascadeWin'
    # TileWinV() -> null
    # Sakura's F_TILE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'TileWinV'
    # TileWinH() -> null
    # Sakura's F_TILE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'TileWinH'
    # NextWindow() -> null
    # Sakura's F_NEXTWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'NextWindow'
    # PrevWindow() -> null
    # Sakura's F_PREVWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'PrevWindow'
    # WindowList() -> null
    # Sakura's F_WINLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WindowList'
    # MaximizeV() -> null
    # Sakura's F_MAXIMIZE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'MaximizeV'
    # MaximizeH() -> null
    # Sakura's F_MAXIMIZE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'MaximizeH'
    # MinimizeAll() -> null
    # Sakura's F_MINIMIZE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'MinimizeAll'
    # ReDraw() -> null
    # Sakura's F_REDRAW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ReDraw'
    # ActivateWinOutput() -> null
    # Sakura's F_WIN_OUTPUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ActivateWinOutput'
    # PreviewTab() -> null
    # MATSU Editor's own F_PREVIEW_TAB (not in Sakura). Runs the command that a key, the menu
    # and a key macro run.
    'PreviewTab'
    # TraceOut(arg1: string?, arg2: int?) -> null
    # MATSU Editor's own F_TRACEOUT (not in Sakura). Runs the command that a key, the menu and a
    # key macro run.
    'TraceOut'
    # WindowTopMost(arg1: int?) -> null
    # Sakura's F_TOPMOST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'WindowTopMost'
    # GroupClose() -> null
    # Sakura's F_GROUPCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'GroupClose'
    # NextGroup() -> null
    # Sakura's F_NEXTGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'NextGroup'
    # PrevGroup() -> null
    # Sakura's F_PREVGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'PrevGroup'
    # TabMoveRight() -> null
    # Sakura's F_TAB_MOVERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TabMoveRight'
    # TabMoveLeft() -> null
    # Sakura's F_TAB_MOVELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TabMoveLeft'
    # TabSeparate() -> null
    # Sakura's F_TAB_SEPARATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TabSeparate'
    # TabJointNext() -> null
    # Sakura's F_TAB_JOINTNEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TabJointNext'
    # TabJointPrev() -> null
    # Sakura's F_TAB_JOINTPREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TabJointPrev'
    # TabCloseOther() -> null
    # Sakura's F_TAB_CLOSEOTHER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TabCloseOther'
    # TabCloseLeft() -> null
    # Sakura's F_TAB_CLOSELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TabCloseLeft'
    # TabCloseRight() -> null
    # Sakura's F_TAB_CLOSERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'TabCloseRight'
    # Complete() -> null
    # Sakura's F_HOKAN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'Complete'
    # ToggleKeyHelpSearch(arg1: int?) -> null
    # Sakura's F_TOGGLE_KEY_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ToggleKeyHelpSearch'
    # HelpContents() -> null
    # Sakura's F_HELP_CONTENTS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HelpContents'
    # HelpSearch() -> null
    # Sakura's F_HELP_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'HelpSearch'
    # CommandList() -> null
    # Sakura's F_MENU_ALLFUNC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'CommandList'
    # ExtHelp1() -> null
    # Sakura's F_EXTHELP1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    'ExtHelp1'
    # ExtHtmlHelp(arg1: string?, arg2: string?) -> null
    # Sakura's F_EXTHTMLHELP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    'ExtHtmlHelp'
    # About() -> null
    # Sakura's F_ABOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    'About'
    # StatusMsg(arg1: string?, arg2: int?) -> null
    # MATSU Editor's own F_STATUSMSG (not in Sakura). Runs the command that a key, the menu and
    # a key macro run.
    'StatusMsg'
    # MsgBeep(arg1: int?) -> null
    # MATSU Editor's own F_MSGBEEP (not in Sakura). Runs the command that a key, the menu and a
    # key macro run.
    'MsgBeep'
    # CommitUndoBuffer() -> null
    # MATSU Editor's own F_COMMITUNDOBUFFER (not in Sakura). Runs the command that a key, the
    # menu and a key macro run.
    'CommitUndoBuffer'
    # AddRefUndoBuffer() -> null
    # MATSU Editor's own F_ADDREFUNDOBUFFER (not in Sakura). Runs the command that a key, the
    # menu and a key macro run.
    'AddRefUndoBuffer'
    # SetUndoBuffer() -> null
    # MATSU Editor's own F_SETUNDOBUFFER (not in Sakura). Runs the command that a key, the menu
    # and a key macro run.
    'SetUndoBuffer'
    # AppendUndoBufferCursor() -> null
    # MATSU Editor's own F_APPENDUNDOBUFFERCURSOR (not in Sakura). Runs the command that a key,
    # the menu and a key macro run.
    'AppendUndoBufferCursor'
    # ClipboardEmpty() -> null
    # MATSU Editor's own F_CLIPBOARDEMPTY (not in Sakura). Runs the command that a key, the menu
    # and a key macro run.
    'ClipboardEmpty'
    # SetViewTop(arg1: int?) -> null
    # MATSU Editor's own F_SETVIEWTOP (not in Sakura). Runs the command that a key, the menu and
    # a key macro run.
    'SetViewTop'
    # SetViewLeft(arg1: int?) -> null
    # MATSU Editor's own F_SETVIEWLEFT (not in Sakura). Runs the command that a key, the menu
    # and a key macro run.
    'SetViewLeft'
)

function global:New-MatsuEditor {
    $editor = [System.Management.Automation.PSObject]::new()
    foreach ($name in $global:MatsuApiNames) {
        # The body is made from text, so that each method knows its own name.
        $body = [scriptblock]::Create("Invoke-MatsuCall '$name' `$args")
        $editor.PSObject.Members.Add([System.Management.Automation.PSScriptMethod]::new($name, $body))
    }
    # Any other name of Sakura's macro functions, including those that start with a digit
    # ('1PageUp'): $Editor.Call('1PageUp'). The editor answers a name it does not know with an
    # error.
    $call = [scriptblock]::Create('param($name) Invoke-MatsuCall $name $args')
    $editor.PSObject.Members.Add([System.Management.Automation.PSScriptMethod]::new('Call', $call))
    $editor.PSObject.Members.Add([System.Management.Automation.PSNoteProperty]::new('Info', $null))
    $editor
}
