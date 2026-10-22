# GENERATED from crates/matsu-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: MATSU_UPDATE_GENERATED=1 cargo test -p matsu-plugin generated_files

_UNSET = object()


def _pack(*args):
    args = list(args)
    while args and args[-1] is _UNSET:
        args.pop()
    return args


def install(call):
    class _Editor:
        """The editor's macro API (the names and the arguments of Sakura's macros)."""

        def GetLineCount(self, reserved: int) -> int:
            """The number of logical lines (lines separated by line breaks). The text after the
            last line break is not a line unless it is not empty. The argument is reserved
            and must be 0."""
            return call("GetLineCount", _pack(reserved))

        def GetLineStr(self, line: int) -> str:
            """A logical line with its line break ("\n"; the last line has none unless the text
            ends with one). 0 is the line of the cursor, 1 is the first line. Past the last
            line it is an empty string."""
            return call("GetLineStr", _pack(line))

        def InsText(self, text: str) -> None:
            """Inserts the text at the cursor, replacing the selection. The cursor ends after
            the text."""
            return call("InsText", _pack(text))

        def AddTail(self, text: str) -> None:
            """Moves to the end of the file, drops the selection, and inserts the text there."""
            return call("AddTail", _pack(text))

        def GoFileTop(self) -> None:
            """Moves the cursor to the start of the file and drops the selection."""
            return call("GoFileTop", _pack())

        def GoFileEnd(self) -> None:
            """Moves the cursor to the end of the file and drops the selection."""
            return call("GoFileEnd", _pack())

        def Up(self) -> None:
            """Moves the cursor up one line and drops the selection."""
            return call("Up", _pack())

        def Down(self) -> None:
            """Moves the cursor down one line and drops the selection."""
            return call("Down", _pack())

        def SelectAll(self) -> None:
            """Selects the whole text. The cursor goes to the end."""
            return call("SelectAll", _pack())

        def GetSelectedString(self, reserved: int) -> str:
            """The selected text with its line breaks ("\n"), or an empty string when nothing
            is selected. The argument is reserved and ignored."""
            return call("GetSelectedString", _pack(reserved))

        def IsTextSelected(self) -> int:
            """0 when nothing is selected, 1 when text is selected, 2 when a box selection is
            selected."""
            return call("IsTextSelected", _pack())

        def MoveCursor(self, line: int, column: int, option: int) -> None:
            """Moves the cursor to a line and a column, both counted from 1. The column counts
            UTF-16 code units. Option: 0 does not select, 1 selects from the old position.
            (2, the box selection, is not supported.) Past the end of the text it goes to
            the end of the text."""
            return call("MoveCursor", _pack(line, column, option))

        def Jump(self, line: int, flag: int = _UNSET) -> None:
            """Moves to the start of a line, counted from 1 in logical lines. Lines before 1
            and after the last one are limited to the first and the last line. Flag bit 0
            (1, the default) is the line break unit, which is the only unit supported yet;
            bit 1 (PL/SQL) is not supported."""
            return call("Jump", _pack(line, flag))

        def GetFilename(self) -> str:
            """The full path of the file, or an empty string when the text has no file."""
            return call("GetFilename", _pack())

        def GetSelectLineFrom(self) -> int:
            """The line where the selection starts, counted from 1 in layout lines (wrapped
            rows count as lines). 0 when nothing is selected."""
            return call("GetSelectLineFrom", _pack())

        def GetSelectColmFrom(self) -> int:
            """The column where the selection starts, counted from 1 in layout columns (cells:
            a full-width character is 2, a tab goes to the tab stop). 0 when nothing is
            selected. Same as GetSelectColumnFrom."""
            return call("GetSelectColmFrom", _pack())

        def GetSelectColumnFrom(self) -> int:
            """The column where the selection starts, counted from 1 in layout columns (cells:
            a full-width character is 2, a tab goes to the tab stop). 0 when nothing is
            selected."""
            return call("GetSelectColumnFrom", _pack())

        def GetSelectLineTo(self) -> int:
            """The line where the selection ends, counted from 1 in layout lines. 0 when
            nothing is selected."""
            return call("GetSelectLineTo", _pack())

        def GetSelectColmTo(self) -> int:
            """The column where the selection ends, counted from 1 in layout columns. 0 when
            nothing is selected. Same as GetSelectColumnTo."""
            return call("GetSelectColmTo", _pack())

        def GetSelectColumnTo(self) -> int:
            """The column where the selection ends, counted from 1 in layout columns. 0 when
            nothing is selected."""
            return call("GetSelectColumnTo", _pack())

        def IsInsMode(self) -> int:
            """1 in insert mode, 0 in overwrite mode."""
            return call("IsInsMode", _pack())

        def GetCharCode(self) -> int:
            """The encoding of the file: 0 Shift_JIS, 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, 5
            UTF-7, 6 UTF-16 BE, 7 CESU-8, 8 Latin1."""
            return call("GetCharCode", _pack())

        def GetLineCode(self) -> int:
            """The line ending that new line breaks get: 0 CR LF, 1 CR, 2 LF (3 NEL, 4 LS, 5
            PS)."""
            return call("GetLineCode", _pack())

        def IsPossibleUndo(self) -> int:
            """1 when there is something to undo, else 0."""
            return call("IsPossibleUndo", _pack())

        def IsPossibleRedo(self) -> int:
            """1 when there is something to redo, else 0."""
            return call("IsPossibleRedo", _pack())

        def GetLineAttribute(self, line: int, type: int) -> int:
            """An attribute of a logical line (0 is the line of the cursor, 1 the first line).
            Type 0 is 1 when the line changed since the last save, type 1 the number of the
            undo step that last touched it, type 2 the bookmark (1 or 0). Types 3 (diff
            mark) and 4 (function list mark) answer the error -32001 until the editor has
            them. A negative line, a line past the end and any other type are errors."""
            return call("GetLineAttribute", _pack(line, type))

        def IsTextSelectingLock(self) -> int:
            """0 when the selection is not locked, 1 when it is locked (plain moves extend the
            selection), 2 when it is locked and a box selection."""
            return call("IsTextSelectingLock", _pack())

        def GetSaveFilename(self) -> str:
            """The full path that a save writes to, or an empty string when the text has no
            file. The same as GetFilename, except in the macro that runs before a save (the
            registered macro of 保存前), where it is the file being saved."""
            return call("GetSaveFilename", _pack())

        def ChangeInsertSpace(self, value: int) -> int:
            """Whether the Tab key inserts spaces instead of a tab character, before the call
            (1 or 0). 0 turns it off, 1 turns it on for this window (not saved); any other
            value only reads. Only the low 16 bits of the value count, as in Sakura."""
            return call("ChangeInsertSpace", _pack(value))

        def LayoutToLogicLineNum(self, layoutLine: int) -> int:
            """The logical line (up to a line break) that a layout line (a row on the screen)
            belongs to, both counted from 1. The line after the last layout line gives the
            line after the last logical line. Any other line outside the text is an error."""
            return call("LayoutToLogicLineNum", _pack(layoutLine))

        def LogicToLayoutLineNum(self, logicLine: int, logicIndex: int) -> int:
            """The layout line (a row on the screen) of a position in a logical line: the line
            and the index in it (UTF-16 code units), both counted from 1. A line past the
            end gives the line after the last layout line."""
            return call("LogicToLayoutLineNum", _pack(logicLine, logicIndex))

        def LineColumnToIndex(self, layoutLine: int, layoutColumn: int) -> int:
            """The index (UTF-16 code units, from 1) in the logical line of a layout line and a
            layout column (cells), both from 1. A column past the text gives the end of the
            text of the row."""
            return call("LineColumnToIndex", _pack(layoutLine, layoutColumn))

        def LineIndexToColumn(self, logicLine: int, logicIndex: int) -> int:
            """The layout column (cells, from 1) of a position in a logical line: the line and
            the index in it (UTF-16 code units), both from 1. Tabs and full-width characters
            take their width."""
            return call("LineIndexToColumn", _pack(logicLine, logicIndex))

        def GetStrWidth(self, text: str, column: int = _UNSET) -> int:
            """The width of a string in cells, starting at layout column `column` (from 1,
            default 1; it matters for tabs). A CR LF at the end counts as the CR alone."""
            return call("GetStrWidth", _pack(text, column))

        def GetStrLayoutLength(self, text: str, column: int = _UNSET) -> int:
            """The width of a string as a line of the document in cells, starting at layout
            column `column` (from 1, default 1). A line ending at the end takes one cell."""
            return call("GetStrLayoutLength", _pack(text, column))

        def GetDefaultCharLength(self) -> int:
            """The layout width of a half-width character. This editor lays out in cells, so it
            is 1."""
            return call("GetDefaultCharLength", _pack())

        def GetViewLines(self) -> int:
            """How many rows the text area shows (at least 1)."""
            return call("GetViewLines", _pack())

        def GetViewColumns(self) -> int:
            """How many columns the text area shows (at least 1)."""
            return call("GetViewColumns", _pack())

        def GetViewTop(self) -> int:
            """The layout line at the top of the text area, counted from 1."""
            return call("GetViewTop", _pack())

        def ChangeTabWidth(self, width: int) -> int:
            """The tab width before the call. A width from 1 to 64 that differs from it becomes
            the tab width of this window (not saved). Any other value, such as 0, only
            reads."""
            return call("ChangeTabWidth", _pack(width))

        def ChangeWrapColm(self, column: int) -> int:
            """The wrap column before the call. A column from 10 to 10240 makes this window
            wrap at that column (not saved). Any other value only reads. Same as
            ChangeWrapColumn."""
            return call("ChangeWrapColm", _pack(column))

        def ChangeWrapColumn(self, column: int) -> int:
            """The wrap column before the call. A column from 10 to 10240 makes this window
            wrap at that column (not saved). Any other value only reads."""
            return call("ChangeWrapColumn", _pack(column))

        def GetClipboard(self, option: int = _UNSET) -> str:
            """The text on the clipboard, or an empty string when there is none. The line
            endings are those of the source. The option is reserved."""
            return call("GetClipboard", _pack(option))

        def SetClipboard(self, option: int = _UNSET, text: str = _UNSET) -> int:
            """Puts the text on the clipboard. Option bit 0 marks a box copy (a paste puts it
            in as a box), bit 1 a row copy (a paste puts it in front of the row); with both,
            the box wins. Returns 1 when it worked, 0 when not."""
            return call("SetClipboard", _pack(option, text))

        def IsIncludeClipboardFormat(self, format: str) -> int:
            """1 when the clipboard has the format. Only text is known: CF_TEXT, CF_OEMTEXT and
            CF_UNICODETEXT (or 1, 7, 13) are 1 when the clipboard has text; every other
            format is 0."""
            return call("IsIncludeClipboardFormat", _pack(format))

        def GetClipboardByFormat(self, format: str, mode: int, endMode: int = _UNSET) -> str:
            """The text of the clipboard in a text format (CF_TEXT, CF_OEMTEXT,
            CF_UNICODETEXT), with the line endings of the document. Mode -2 is the usual way
            and 3 is UTF-16 (the text itself); the other modes and formats are errors
            (-32001). The end mode is not used."""
            return call("GetClipboardByFormat", _pack(format, mode, endMode))

        def SetClipboardByFormat(self, text: str, format: str, mode: int, endMode: int = _UNSET) -> int:
            """Puts the text on the clipboard as text. The formats and modes are those of
            GetClipboardByFormat. Returns 1 when it worked."""
            return call("SetClipboardByFormat", _pack(text, format, mode, endMode))

        def GetCookie(self, scope: str, name: str) -> str:
            """The value of a cookie, or an empty string when there is none. The scope is
            "window" or "document"."""
            return call("GetCookie", _pack(scope, name))

        def GetCookieDefault(self, scope: str, name: str, defaultValue: str) -> str:
            """The value of a cookie, or the default when there is none (an unknown scope still
            gives an empty string). The scope is "window" or "document"."""
            return call("GetCookieDefault", _pack(scope, name, defaultValue))

        def SetCookie(self, scope: str, name: str, value: str) -> int:
            """Sets a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a name
            with a character that a name cannot have (Sakura lets in digits, letters and
            every character from "_" up in the character table). The scope is "window" or
            "document"."""
            return call("SetCookie", _pack(scope, name, value))

        def DeleteCookie(self, scope: str, name: str) -> int:
            """Deletes a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a bad
            name, 5 when there is no such cookie. The scope is "window" or "document"."""
            return call("DeleteCookie", _pack(scope, name))

        def GetCookieNames(self, scope: str) -> str:
            """The names of the cookies of a scope, joined with commas, in the order of the
            names (UTF-16 code units). An unknown scope gives an empty string. The scope is
            "window" or "document"."""
            return call("GetCookieNames", _pack(scope))

        def CompareVersion(self, versionA: str, versionB: str) -> int:
            """Compares two version strings: 0 when equal, positive when A is newer, negative
            when B is. Four parts at most; a part has two digits at most; "alpha", "beta",
            "rc" and "pl" put a part before or after the release (like PHP's
            version_compare)."""
            return call("CompareVersion", _pack(versionA, versionB))

        def SetDrawSwitch(self, on: int) -> int:
            """Turns the draw switch on (non-zero) or off (0) and returns the state before (1
            or 0). The editor keeps the flag for GetDrawSwitch but still paints the text
            when it likes."""
            return call("SetDrawSwitch", _pack(on))

        def GetDrawSwitch(self) -> int:
            """1 when the draw switch is on (the start state), 0 when a macro turned it off."""
            return call("GetDrawSwitch", _pack())

        def IsShownStatus(self) -> int:
            """1 when the status bar is shown, else 0."""
            return call("IsShownStatus", _pack())

        def IsCurTypeExt(self, extension: str) -> int:
            """1 when the document type of this file is the type of the extension (looked up in
            the extension lists of the types in the settings file, as a file name). Without
            type settings every extension is the basic type."""
            return call("IsCurTypeExt", _pack(extension))

        def IsSameTypeExt(self, extension1: str, extension2: str) -> int:
            """1 when the two extensions belong to the same document type (see IsCurTypeExt)."""
            return call("IsSameTypeExt", _pack(extension1, extension2))

        def ExpandParameter(self, format: str) -> str:
            """Replaces the codes that start with $ in a string: $$ ($), $F (full path), $f
            (file name), $g (file name without the extension), $/ (full path with /), $N (as
            $F), $b (extension), $e and $E (folder), $C (the selected text or the word at
            the caret), $x and $y (caret column in UTF-16 units and line, from 1), $S (the
            editor), $M (the macro file), $V (version), $A (name), and conditions
            ${R?a$:b$:c$} (R, w, M, U, N, I). A text without a file is (無題). $d $t $D $T $B
            $Q $I are not supported and stay as written. At most 2047 UTF-16 units."""
            return call("ExpandParameter", _pack(format))

        def MessageBox(self, message: str, type: int = _UNSET) -> int:
            """Shows a message box and waits. The type is Windows' MB_ value: the low nibble
            chooses the buttons (0 OK, 1 OK/Cancel, 2 Abort/Retry/Ignore, 3 Yes/No/Cancel, 4
            Yes/No, 5 Retry/Cancel, 6 Cancel/Try again/Continue), the icon bits (0x10 stop,
            0x20 question, 0x30 exclamation, 0x40 information) the icon; the default button
            and the modality are ignored. Returns the button as Windows numbers it: 1 OK, 2
            Cancel, 3 Abort, 4 Retry, 5 Ignore, 6 Yes, 7 No, 10 Try again, 11 Continue. A
            box that is closed without an answer gives Cancel (the last button when there is
            none)."""
            return call("MessageBox", _pack(message, type))

        def ErrorMsg(self, message: str) -> int:
            """Shows an error message with an OK button and waits. Returns 1."""
            return call("ErrorMsg", _pack(message))

        def WarnMsg(self, message: str) -> int:
            """Shows a warning with an OK button and waits. Returns 1."""
            return call("WarnMsg", _pack(message))

        def InfoMsg(self, message: str) -> int:
            """Shows an information message with an OK button and waits. Returns 1."""
            return call("InfoMsg", _pack(message))

        def OkCancelBox(self, message: str) -> int:
            """Asks a question with OK and Cancel and waits. Returns 1 (OK) or 2 (Cancel)."""
            return call("OkCancelBox", _pack(message))

        def YesNoBox(self, message: str) -> int:
            """Asks a question with Yes and No and waits. Returns 6 (Yes) or 7 (No)."""
            return call("YesNoBox", _pack(message))

        def FileOpenDialog(self, defaultName: str = _UNSET, filter: str = _UNSET) -> str:
            """Lets the user choose a file to open and returns its full path, or an empty
            string when the dialog is cancelled. The default name and the filter are not
            used: the dialog of the platform takes neither."""
            return call("FileOpenDialog", _pack(defaultName, filter))

        def FileSaveDialog(self, defaultName: str = _UNSET, filter: str = _UNSET) -> str:
            """Lets the user choose a file name to save under and returns the full path, or an
            empty string when the dialog is cancelled. The dialog starts with the default
            name. The filter is not used."""
            return call("FileSaveDialog", _pack(defaultName, filter))

        def FolderDialog(self, message: str = _UNSET, defaultFolder: str = _UNSET) -> str:
            """Lets the user choose a folder and returns its full path, or an empty string when
            the dialog is cancelled. The message becomes the prompt text of the platform
            dialog where it has one; the default folder is not used."""
            return call("FolderDialog", _pack(message, defaultFolder))

        def InputBox(self, message: str, defaultValue: str = _UNSET, maxLength: int = _UNSET) -> str:
            """Shows a dialog with a text field and waits. The message is shown above the
            field, the default value is in the field, and the text cannot be longer than the
            maximum length (in UTF-16 code units; 0 or less, or no value, is 260). Returns
            the text of the field when OK is pressed, or an empty string when the dialog is
            cancelled."""
            return call("InputBox", _pack(message, defaultValue, maxLength))

        def CreateMenu(self, position: int, menu: str) -> int:
            """Shows a pop-up menu and waits. The position is 1 for the caret (the pointer when
            the caret is out of the view) and anything else for the pointer. The menu is a
            list of items separated by commas; an item can start with flags in brackets: C
            checked, D disabled, R radio, S opens a submenu, E ends a submenu (one for each
            E), B (a new column) is ignored. A lone - is a separator. Write [][text] for a
            text that starts with a bracket. Returns the number of the item chosen, counted
            from 1 over the items only (separators and submenus are not counted), or 0 when
            the menu is closed without a choice or has no item."""
            return call("CreateMenu", _pack(position, menu))

        def Sleep(self, milliseconds: int) -> int:
            """Waits for the given number of milliseconds (the window stays alive, and the
            macro can be cancelled). Returns 0."""
            return call("Sleep", _pack(milliseconds))

        def FileNew(self) -> None:
            """Sakura's F_FILENEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("FileNew", _pack())

        def FileOpen(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET, arg4: str = _UNSET) -> None:
            """MATSU Editor's own F_FILEOPEN2 (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("FileOpen", _pack(arg1, arg2, arg3, arg4))

        def FileSave(self) -> None:
            """Sakura's F_FILESAVE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("FileSave", _pack())

        def FileSaveAll(self) -> None:
            """Sakura's F_FILESAVEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileSaveAll", _pack())

        def FileSaveAsDialog(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET) -> None:
            """Sakura's F_FILESAVEAS_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileSaveAsDialog", _pack(arg1, arg2, arg3))

        def FileSaveAs(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET) -> None:
            """Sakura's F_FILESAVEAS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileSaveAs", _pack(arg1, arg2, arg3))

        def FileClose(self) -> None:
            """Sakura's F_FILECLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("FileClose", _pack())

        def FileCloseOpen(self) -> None:
            """Sakura's F_FILECLOSE_OPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileCloseOpen", _pack())

        def FileReopen(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopen", _pack(arg1))

        def FileReopenSJIS(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenSJIS", _pack(arg1))

        def FileReopenJIS(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenJIS", _pack(arg1))

        def FileReopenEUC(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenEUC", _pack(arg1))

        def FileReopenLatin1(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_LATIN1 (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenLatin1", _pack(arg1))

        def FileReopenUNICODE(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_UNICODE (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenUNICODE", _pack(arg1))

        def FileReopenUNICODEBE(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_UNICODEBE (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenUNICODEBE", _pack(arg1))

        def FileReopenUTF8(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenUTF8", _pack(arg1))

        def FileReopenCESU8(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_CESU8 (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenCESU8", _pack(arg1))

        def FileReopenUTF7(self, arg1: int = _UNSET) -> None:
            """Sakura's F_FILE_REOPEN_UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FileReopenUTF7", _pack(arg1))

        def Print(self) -> None:
            """Sakura's F_PRINT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Print", _pack())

        def PrintPreview(self) -> None:
            """Sakura's F_PRINT_PREVIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("PrintPreview", _pack())

        def PrintPageSetup(self) -> None:
            """Sakura's F_PRINT_PAGESETUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("PrintPageSetup", _pack())

        def OpenHfromtoC(self) -> None:
            """Sakura's F_OPEN_HfromtoC (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("OpenHfromtoC", _pack())

        def ActivateSQLPLUS(self) -> None:
            """Sakura's F_ACTIVATE_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ActivateSQLPLUS", _pack())

        def ExecSQLPLUS(self) -> None:
            """Sakura's F_PLSQL_COMPILE_ON_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ExecSQLPLUS", _pack())

        def Browse(self) -> None:
            """Sakura's F_BROWSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Browse", _pack())

        def ViewMode(self) -> None:
            """Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ViewMode", _pack())

        def ReadOnly(self) -> None:
            """Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ReadOnly", _pack())

        def PropertyFile(self) -> None:
            """Sakura's F_PROPERTY_FILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("PropertyFile", _pack())

        def ExitAllEditors(self) -> None:
            """Sakura's F_EXITALLEDITORS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ExitAllEditors", _pack())

        def ExitAll(self) -> None:
            """Sakura's F_EXITALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ExitAll", _pack())

        def PutFile(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET) -> None:
            """MATSU Editor's own F_PUTFILE (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("PutFile", _pack(arg1, arg2, arg3))

        def InsFile(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET) -> None:
            """MATSU Editor's own F_INSFILE (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("InsFile", _pack(arg1, arg2, arg3))

        def Char(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WCHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Char", _pack(arg1))

        def CharIme(self, arg1: int = _UNSET) -> None:
            """Sakura's F_IME_CHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CharIme", _pack(arg1))

        def Undo(self) -> None:
            """Sakura's F_UNDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Undo", _pack())

        def Redo(self) -> None:
            """Sakura's F_REDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Redo", _pack())

        def Delete(self) -> None:
            """Sakura's F_DELETE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Delete", _pack())

        def DeleteBack(self) -> None:
            """Sakura's F_DELETE_BACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("DeleteBack", _pack())

        def WordDeleteToStart(self) -> None:
            """Sakura's F_WordDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WordDeleteToStart", _pack())

        def WordDeleteToEnd(self) -> None:
            """Sakura's F_WordDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WordDeleteToEnd", _pack())

        def WordCut(self) -> None:
            """Sakura's F_WordCut (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WordCut", _pack())

        def WordDelete(self) -> None:
            """Sakura's F_WordDelete (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WordDelete", _pack())

        def LineCutToStart(self) -> None:
            """Sakura's F_LineCutToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("LineCutToStart", _pack())

        def LineCutToEnd(self) -> None:
            """Sakura's F_LineCutToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("LineCutToEnd", _pack())

        def LineDeleteToStart(self) -> None:
            """Sakura's F_LineDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("LineDeleteToStart", _pack())

        def LineDeleteToEnd(self) -> None:
            """Sakura's F_LineDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("LineDeleteToEnd", _pack())

        def CutLine(self) -> None:
            """Sakura's F_CUT_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CutLine", _pack())

        def DeleteLine(self) -> None:
            """Sakura's F_DELETE_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("DeleteLine", _pack())

        def DuplicateLine(self) -> None:
            """Sakura's F_DUPLICATELINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("DuplicateLine", _pack())

        def IndentTab(self) -> None:
            """Sakura's F_INDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("IndentTab", _pack())

        def UnindentTab(self) -> None:
            """Sakura's F_UNINDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("UnindentTab", _pack())

        def IndentSpace(self) -> None:
            """Sakura's F_INDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("IndentSpace", _pack())

        def UnindentSpace(self) -> None:
            """Sakura's F_UNINDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("UnindentSpace", _pack())

        def LTrim(self) -> None:
            """Sakura's F_LTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("LTrim", _pack())

        def RTrim(self) -> None:
            """Sakura's F_RTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("RTrim", _pack())

        def SortAsc(self) -> None:
            """Sakura's F_SORT_ASC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("SortAsc", _pack())

        def SortDesc(self) -> None:
            """Sakura's F_SORT_DESC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("SortDesc", _pack())

        def Merge(self) -> None:
            """Sakura's F_MERGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Merge", _pack())

        def Left(self) -> None:
            """Sakura's F_LEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Left", _pack())

        def Right(self) -> None:
            """Sakura's F_RIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Right", _pack())

        def Up2(self) -> None:
            """Sakura's F_UP2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Up2", _pack())

        def Down2(self) -> None:
            """Sakura's F_DOWN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Down2", _pack())

        def WordLeft(self) -> None:
            """Sakura's F_WORDLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WordLeft", _pack())

        def WordRight(self) -> None:
            """Sakura's F_WORDRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WordRight", _pack())

        def GoLineTop(self, arg1: int = _UNSET) -> None:
            """Sakura's F_GOLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("GoLineTop", _pack(arg1))

        def GoLineEnd(self, arg1: int = _UNSET) -> None:
            """Sakura's F_GOLINEEND (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("GoLineEnd", _pack(arg1))

        def HalfPageUp(self, arg1: int = _UNSET) -> None:
            """Sakura's F_HalfPageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HalfPageUp", _pack(arg1))

        def HalfPageDown(self, arg1: int = _UNSET) -> None:
            """Sakura's F_HalfPageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HalfPageDown", _pack(arg1))

        def PageUp(self, arg1: int = _UNSET) -> None:
            """Sakura's F_1PageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("PageUp", _pack(arg1))

        def PageDown(self, arg1: int = _UNSET) -> None:
            """Sakura's F_1PageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("PageDown", _pack(arg1))

        def CurLineCenter(self) -> None:
            """Sakura's F_CURLINECENTER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CurLineCenter", _pack())

        def CurLineTop(self) -> None:
            """Sakura's F_CURLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CurLineTop", _pack())

        def CurLineBottom(self) -> None:
            """Sakura's F_CURLINEBOTTOM (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CurLineBottom", _pack())

        def MoveHistPrev(self) -> None:
            """Sakura's F_JUMPHIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("MoveHistPrev", _pack())

        def MoveHistNext(self) -> None:
            """Sakura's F_JUMPHIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("MoveHistNext", _pack())

        def MoveHistSet(self) -> None:
            """Sakura's F_JUMPHIST_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("MoveHistSet", _pack())

        def F_WndScrollDown(self) -> None:
            """Sakura's F_WndScrollDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("F_WndScrollDown", _pack())

        def F_WndScrollUp(self) -> None:
            """Sakura's F_WndScrollUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("F_WndScrollUp", _pack())

        def GoNextParagraph(self) -> None:
            """Sakura's F_GONEXTPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoNextParagraph", _pack())

        def GoPrevParagraph(self) -> None:
            """Sakura's F_GOPREVPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoPrevParagraph", _pack())

        def GoModifyLineNext(self) -> None:
            """Sakura's F_MODIFYLINE_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoModifyLineNext", _pack())

        def GoModifyLinePrev(self) -> None:
            """Sakura's F_MODIFYLINE_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoModifyLinePrev", _pack())

        def MoveCursorLayout(self, arg1: int = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET) -> None:
            """MATSU Editor's own F_MOVECURSORLAYOUT (not in Sakura). Runs the command that a
            key, the menu and a key macro run."""
            return call("MoveCursorLayout", _pack(arg1, arg2, arg3))

        def WheelUp(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WHEELUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WheelUp", _pack(arg1))

        def WheelDown(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WHEELDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WheelDown", _pack(arg1))

        def WheelLeft(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WHEELLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WheelLeft", _pack(arg1))

        def WheelRight(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WHEELRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WheelRight", _pack(arg1))

        def WheelPageUp(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WHEELPAGEUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WheelPageUp", _pack(arg1))

        def WheelPageDown(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WHEELPAGEDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WheelPageDown", _pack(arg1))

        def WheelPageLeft(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WHEELPAGELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WheelPageLeft", _pack(arg1))

        def WheelPageRight(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WHEELPAGERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WheelPageRight", _pack(arg1))

        def SelectWord(self) -> None:
            """Sakura's F_SELECTWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SelectWord", _pack())

        def SelectLine(self, arg1: int = _UNSET) -> None:
            """Sakura's F_SELECTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SelectLine", _pack(arg1))

        def BeginSelect(self) -> None:
            """Sakura's F_BEGIN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("BeginSelect", _pack())

        def Up_Sel(self) -> None:
            """Sakura's F_UP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Up_Sel", _pack())

        def Down_Sel(self) -> None:
            """Sakura's F_DOWN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Down_Sel", _pack())

        def Left_Sel(self) -> None:
            """Sakura's F_LEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Left_Sel", _pack())

        def Right_Sel(self) -> None:
            """Sakura's F_RIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Right_Sel", _pack())

        def Up2_Sel(self) -> None:
            """Sakura's F_UP2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Up2_Sel", _pack())

        def Down2_Sel(self) -> None:
            """Sakura's F_DOWN2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Down2_Sel", _pack())

        def WordLeft_Sel(self) -> None:
            """Sakura's F_WORDLEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WordLeft_Sel", _pack())

        def WordRight_Sel(self) -> None:
            """Sakura's F_WORDRIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WordRight_Sel", _pack())

        def GoLineTop_Sel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_GOLINETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoLineTop_Sel", _pack(arg1))

        def GoLineEnd_Sel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_GOLINEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoLineEnd_Sel", _pack(arg1))

        def HalfPageUp_Sel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_HalfPageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HalfPageUp_Sel", _pack(arg1))

        def HalfPageDown_Sel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_HalfPageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HalfPageDown_Sel", _pack(arg1))

        def PageUp_Sel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_1PageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("PageUp_Sel", _pack(arg1))

        def PageDown_Sel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_1PageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("PageDown_Sel", _pack(arg1))

        def GoFileTop_Sel(self) -> None:
            """Sakura's F_GOFILETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoFileTop_Sel", _pack())

        def GoFileEnd_Sel(self) -> None:
            """Sakura's F_GOFILEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoFileEnd_Sel", _pack())

        def GoNextParagraph_Sel(self) -> None:
            """Sakura's F_GONEXTPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoNextParagraph_Sel", _pack())

        def GoPrevParagraph_Sel(self) -> None:
            """Sakura's F_GOPREVPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoPrevParagraph_Sel", _pack())

        def GoModifyLineNext_Sel(self) -> None:
            """Sakura's F_MODIFYLINE_NEXT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoModifyLineNext_Sel", _pack())

        def GoModifyLinePrev_Sel(self) -> None:
            """Sakura's F_MODIFYLINE_PREV_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoModifyLinePrev_Sel", _pack())

        def BeginBoxSelect(self) -> None:
            """Sakura's F_BEGIN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("BeginBoxSelect", _pack())

        def Up_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_UP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Up_BoxSel", _pack(arg1))

        def Down_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_DOWN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Down_BoxSel", _pack(arg1))

        def Left_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_LEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Left_BoxSel", _pack(arg1))

        def Right_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_RIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Right_BoxSel", _pack(arg1))

        def Up2_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_UP2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Up2_BoxSel", _pack(arg1))

        def Down2_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_DOWN2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Down2_BoxSel", _pack(arg1))

        def WordLeft_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WORDLEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WordLeft_BoxSel", _pack(arg1))

        def WordRight_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_WORDRIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WordRight_BoxSel", _pack(arg1))

        def GoLogicalLineTop_BoxSel(self, arg1: int = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_GOLOGICALLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoLogicalLineTop_BoxSel", _pack(arg1, arg2))

        def GoLineTop_BoxSel(self, arg1: int = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_GOLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoLineTop_BoxSel", _pack(arg1, arg2))

        def GoLineEnd_BoxSel(self, arg1: int = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_GOLINEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoLineEnd_BoxSel", _pack(arg1, arg2))

        def HalfPageUp_BoxSel(self, arg1: int = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_HalfPageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HalfPageUp_BoxSel", _pack(arg1, arg2))

        def HalfPageDown_BoxSel(self, arg1: int = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_HalfPageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HalfPageDown_BoxSel", _pack(arg1, arg2))

        def PageUp_BoxSel(self, arg1: int = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_1PageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("PageUp_BoxSel", _pack(arg1, arg2))

        def PageDown_BoxSel(self, arg1: int = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_1PageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("PageDown_BoxSel", _pack(arg1, arg2))

        def GoFileTop_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_GOFILETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoFileTop_BoxSel", _pack(arg1))

        def GoFileEnd_BoxSel(self, arg1: int = _UNSET) -> None:
            """Sakura's F_GOFILEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GoFileEnd_BoxSel", _pack(arg1))

        def Cut(self) -> None:
            """Sakura's F_CUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Cut", _pack())

        def Copy(self) -> None:
            """Sakura's F_COPY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Copy", _pack())

        def Paste(self, arg1: int = _UNSET) -> None:
            """Sakura's F_PASTE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Paste", _pack(arg1))

        def CopyAddCRLF(self) -> None:
            """Sakura's F_COPY_ADDCRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CopyAddCRLF", _pack())

        def CopyCRLF(self) -> None:
            """Sakura's F_COPY_CRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CopyCRLF", _pack())

        def CopyLF(self) -> None:
            """MATSU Editor's own F_COPY_LF (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("CopyLF", _pack())

        def CopyCR(self) -> None:
            """MATSU Editor's own F_COPY_CR (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("CopyCR", _pack())

        def PasteBox(self, arg1: int = _UNSET) -> None:
            """Sakura's F_PASTEBOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("PasteBox", _pack(arg1))

        def InsBoxText(self, arg1: str = _UNSET) -> None:
            """MATSU Editor's own F_INSBOXTEXT (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("InsBoxText", _pack(arg1))

        def CopyLines(self) -> None:
            """Sakura's F_COPYLINES (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CopyLines", _pack())

        def CopyLinesAsPassage(self) -> None:
            """Sakura's F_COPYLINESASPASSAGE (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CopyLinesAsPassage", _pack())

        def CopyLinesWithLineNumber(self) -> None:
            """Sakura's F_COPYLINESWITHLINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CopyLinesWithLineNumber", _pack())

        def CopyColorHtml(self) -> None:
            """Sakura's F_COPY_COLOR_HTML (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CopyColorHtml", _pack())

        def CopyColorHtmlWithLineNumber(self) -> None:
            """Sakura's F_COPY_COLOR_HTML_LINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs
            the command that a key, the menu and a key macro run. The last arguments may be
            left out. A command that this editor does not have yet answers the error -32001
            (not implemented)."""
            return call("CopyColorHtmlWithLineNumber", _pack())

        def CopyPath(self) -> None:
            """Sakura's F_COPYPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CopyPath", _pack())

        def CopyDirPath(self) -> None:
            """Sakura's F_COPYDIRPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CopyDirPath", _pack())

        def CopyFilename(self) -> None:
            """Sakura's F_COPYFNAME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CopyFilename", _pack())

        def CopyTag(self) -> None:
            """Sakura's F_COPYTAG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CopyTag", _pack())

        def CopyKeyBindList(self) -> None:
            """Sakura's F_CREATEKEYBINDLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CopyKeyBindList", _pack())

        def InsertDate(self) -> None:
            """Sakura's F_INS_DATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("InsertDate", _pack())

        def InsertTime(self) -> None:
            """Sakura's F_INS_TIME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("InsertTime", _pack())

        def CtrlCodeDialog(self) -> None:
            """Sakura's F_CTRL_CODE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CtrlCodeDialog", _pack())

        def CtrlCode(self, arg1: int = _UNSET) -> None:
            """Sakura's F_CTRL_CODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CtrlCode", _pack(arg1))

        def InsertFileUsedRecently(self) -> None:
            """Sakura's F_INS_FILE_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("InsertFileUsedRecently", _pack())

        def InsertFolderUsedRecently(self) -> None:
            """Sakura's F_INS_FOLDER_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("InsertFolderUsedRecently", _pack())

        def ToLower(self) -> None:
            """Sakura's F_TOLOWER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ToLower", _pack())

        def ToUpper(self) -> None:
            """Sakura's F_TOUPPER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ToUpper", _pack())

        def ToHankaku(self) -> None:
            """Sakura's F_TOHANKAKU (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ToHankaku", _pack())

        def ToHankata(self) -> None:
            """Sakura's F_TOHANKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ToHankata", _pack())

        def ToZenEi(self) -> None:
            """Sakura's F_TOZENEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ToZenEi", _pack())

        def ToHanEi(self) -> None:
            """Sakura's F_TOHANEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ToHanEi", _pack())

        def ToZenKata(self) -> None:
            """Sakura's F_TOZENKAKUKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ToZenKata", _pack())

        def ToZenHira(self) -> None:
            """Sakura's F_TOZENKAKUHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ToZenHira", _pack())

        def HanKataToZenKata(self) -> None:
            """Sakura's F_HANKATATOZENKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HanKataToZenKata", _pack())

        def HanKataToZenHira(self) -> None:
            """Sakura's F_HANKATATOZENHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HanKataToZenHira", _pack())

        def TABToSPACE(self) -> None:
            """Sakura's F_TABTOSPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TABToSPACE", _pack())

        def SPACEToTAB(self) -> None:
            """Sakura's F_SPACETOTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SPACEToTAB", _pack())

        def AutoToSJIS(self) -> None:
            """Sakura's F_CODECNV_AUTO2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("AutoToSJIS", _pack())

        def JIStoSJIS(self) -> None:
            """Sakura's F_CODECNV_EMAIL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("JIStoSJIS", _pack())

        def EUCtoSJIS(self) -> None:
            """Sakura's F_CODECNV_EUC2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("EUCtoSJIS", _pack())

        def CodeCnvUNICODEtoSJIS(self) -> None:
            """Sakura's F_CODECNV_UNICODE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CodeCnvUNICODEtoSJIS", _pack())

        def CodeCnvUNICODEBEtoSJIS(self) -> None:
            """Sakura's F_CODECNV_UNICODEBE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CodeCnvUNICODEBEtoSJIS", _pack())

        def UTF8toSJIS(self) -> None:
            """Sakura's F_CODECNV_UTF82SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("UTF8toSJIS", _pack())

        def UTF7toSJIS(self) -> None:
            """Sakura's F_CODECNV_UTF72SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("UTF7toSJIS", _pack())

        def SJIStoJIS(self) -> None:
            """Sakura's F_CODECNV_SJIS2JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SJIStoJIS", _pack())

        def SJIStoEUC(self) -> None:
            """Sakura's F_CODECNV_SJIS2EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SJIStoEUC", _pack())

        def SJIStoUTF8(self) -> None:
            """Sakura's F_CODECNV_SJIS2UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SJIStoUTF8", _pack())

        def SJIStoUTF7(self) -> None:
            """Sakura's F_CODECNV_SJIS2UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SJIStoUTF7", _pack())

        def Base64Decode(self) -> None:
            """Sakura's F_BASE64DECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("Base64Decode", _pack())

        def Uudecode(self) -> None:
            """Sakura's F_UUDECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Uudecode", _pack())

        def SearchDialog(self) -> None:
            """Sakura's F_SEARCH_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SearchDialog", _pack())

        def SearchNext(self, arg1: str = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_SEARCH_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SearchNext", _pack(arg1, arg2))

        def SearchPrev(self, arg1: str = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_SEARCH_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SearchPrev", _pack(arg1, arg2))

        def ReplaceDialog(self) -> None:
            """Sakura's F_REPLACE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ReplaceDialog", _pack())

        def Replace(self, arg1: str = _UNSET, arg2: str = _UNSET, arg3: int = _UNSET) -> None:
            """Sakura's F_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Replace", _pack(arg1, arg2, arg3))

        def ReplaceAll(self, arg1: str = _UNSET, arg2: str = _UNSET, arg3: int = _UNSET) -> None:
            """Sakura's F_REPLACE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ReplaceAll", _pack(arg1, arg2, arg3))

        def SearchClearMark(self) -> None:
            """Sakura's F_SEARCH_CLEARMARK (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SearchClearMark", _pack())

        def SearchStartPos(self) -> None:
            """Sakura's F_JUMP_SRCHSTARTPOS (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SearchStartPos", _pack())

        def Grep(self, arg1: str = _UNSET, arg2: str = _UNSET, arg3: str = _UNSET, arg4: int = _UNSET, arg5: int = _UNSET) -> None:
            """Sakura's F_GREP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Grep", _pack(arg1, arg2, arg3, arg4, arg5))

        def GrepReplace(self, arg1: str = _UNSET, arg2: str = _UNSET, arg3: str = _UNSET, arg4: str = _UNSET, arg5: int = _UNSET, arg6: int = _UNSET) -> None:
            """Sakura's F_GREP_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GrepReplace", _pack(arg1, arg2, arg3, arg4, arg5, arg6))

        def Outline(self, arg1: int = _UNSET) -> None:
            """Sakura's F_OUTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Outline", _pack(arg1))

        def TagJump(self) -> None:
            """Sakura's F_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("TagJump", _pack())

        def TagJumpBack(self) -> None:
            """Sakura's F_TAGJUMPBACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TagJumpBack", _pack())

        def TagMake(self) -> None:
            """Sakura's F_TAGS_MAKE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("TagMake", _pack())

        def DirectTagJump(self) -> None:
            """Sakura's F_DIRECT_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("DirectTagJump", _pack())

        def KeywordTagJump(self, arg1: str = _UNSET) -> None:
            """Sakura's F_TAGJUMP_KEYWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("KeywordTagJump", _pack(arg1))

        def Compare(self) -> None:
            """Sakura's F_COMPARE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Compare", _pack())

        def DiffDialog(self) -> None:
            """Sakura's F_DIFF_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("DiffDialog", _pack())

        def Diff(self, arg1: str = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_DIFF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Diff", _pack(arg1, arg2))

        def DiffNext(self) -> None:
            """Sakura's F_DIFF_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("DiffNext", _pack())

        def DiffPrev(self) -> None:
            """Sakura's F_DIFF_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("DiffPrev", _pack())

        def DiffReset(self) -> None:
            """Sakura's F_DIFF_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("DiffReset", _pack())

        def BracketPair(self) -> None:
            """Sakura's F_BRACKETPAIR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("BracketPair", _pack())

        def BookmarkSet(self) -> None:
            """Sakura's F_BOOKMARK_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("BookmarkSet", _pack())

        def BookmarkNext(self) -> None:
            """Sakura's F_BOOKMARK_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("BookmarkNext", _pack())

        def BookmarkPrev(self) -> None:
            """Sakura's F_BOOKMARK_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("BookmarkPrev", _pack())

        def BookmarkReset(self) -> None:
            """Sakura's F_BOOKMARK_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("BookmarkReset", _pack())

        def BookmarkView(self) -> None:
            """Sakura's F_BOOKMARK_VIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("BookmarkView", _pack())

        def BookmarkPattern(self, arg1: str = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_BOOKMARK_PATTERN (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("BookmarkPattern", _pack(arg1, arg2))

        def FuncListNext(self) -> None:
            """Sakura's F_FUNCLIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FuncListNext", _pack())

        def FuncListPrev(self) -> None:
            """Sakura's F_FUNCLIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("FuncListPrev", _pack())

        def ChgmodINS(self) -> None:
            """Sakura's F_CHGMOD_INS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ChgmodINS", _pack())

        def ChgCharSet(self, arg1: int = _UNSET, arg2: int = _UNSET) -> None:
            """Sakura's F_CHG_CHARSET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ChgCharSet", _pack(arg1, arg2))

        def ChgmodEOL(self, arg1: int = _UNSET) -> None:
            """MATSU Editor's own F_CHGMOD_EOL (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("ChgmodEOL", _pack(arg1))

        def CancelMode(self) -> None:
            """Sakura's F_CANCEL_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CancelMode", _pack())

        def ExecExternalMacro(self, arg1: str = _UNSET, arg2: str = _UNSET) -> None:
            """Sakura's F_EXECEXTMACRO (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ExecExternalMacro", _pack(arg1, arg2))

        def ShowToolbar(self) -> None:
            """Sakura's F_SHOWTOOLBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ShowToolbar", _pack())

        def ShowFunckey(self) -> None:
            """Sakura's F_SHOWFUNCKEY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ShowFunckey", _pack())

        def ShowTab(self) -> None:
            """Sakura's F_SHOWTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ShowTab", _pack())

        def ShowStatusbar(self) -> None:
            """Sakura's F_SHOWSTATUSBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ShowStatusbar", _pack())

        def ShowMiniMap(self) -> None:
            """Sakura's F_SHOWMINIMAP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ShowMiniMap", _pack())

        def TypeList(self) -> None:
            """Sakura's F_TYPE_LIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("TypeList", _pack())

        def ChangeType(self, arg1: int = _UNSET) -> None:
            """MATSU Editor's own F_CHANGETYPE (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("ChangeType", _pack(arg1))

        def OptionType(self) -> None:
            """Sakura's F_OPTION_TYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("OptionType", _pack())

        def OptionCommon(self) -> None:
            """Sakura's F_OPTION (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("OptionCommon", _pack())

        def SelectFont(self) -> None:
            """Sakura's F_FONT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
            the menu and a key macro run. The last arguments may be left out. A command that
            this editor does not have yet answers the error -32001 (not implemented)."""
            return call("SelectFont", _pack())

        def SetFontSize(self, arg1: int = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET) -> None:
            """Sakura's F_SETFONTSIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SetFontSize", _pack(arg1, arg2, arg3))

        def WrapWindowWidth(self) -> None:
            """Sakura's F_WRAPWINDOWWIDTH (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WrapWindowWidth", _pack())

        def OptionFavorite(self) -> None:
            """Sakura's F_FAVORITE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("OptionFavorite", _pack())

        def SetMsgQuoteStr(self, arg1: str = _UNSET) -> None:
            """MATSU Editor's own F_SET_QUOTESTRING (not in Sakura). Runs the command that a key,
            the menu and a key macro run."""
            return call("SetMsgQuoteStr", _pack(arg1))

        def TextWrapMethod(self, arg1: int = _UNSET) -> None:
            """MATSU Editor's own F_TEXTWRAPMETHOD (not in Sakura). Runs the command that a key,
            the menu and a key macro run."""
            return call("TextWrapMethod", _pack(arg1))

        def SelectCountMode(self, arg1: int = _UNSET) -> None:
            """Sakura's F_SELECT_COUNT_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SelectCountMode", _pack(arg1))

        def ExecCommand(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: str = _UNSET) -> None:
            """Sakura's F_EXECMD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ExecCommand", _pack(arg1, arg2, arg3))

        def ExecCommandDialog(self) -> None:
            """Sakura's F_EXECMD_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ExecCommandDialog", _pack())

        def RMenu(self) -> None:
            """Sakura's F_MENU_RBUTTON (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("RMenu", _pack())

        def CustMenu1(self) -> None:
            """Sakura's F_CUSTMENU_1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu1", _pack())

        def CustMenu2(self) -> None:
            """Sakura's F_CUSTMENU_2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu2", _pack())

        def CustMenu3(self) -> None:
            """Sakura's F_CUSTMENU_3 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu3", _pack())

        def CustMenu4(self) -> None:
            """Sakura's F_CUSTMENU_4 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu4", _pack())

        def CustMenu5(self) -> None:
            """Sakura's F_CUSTMENU_5 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu5", _pack())

        def CustMenu6(self) -> None:
            """Sakura's F_CUSTMENU_6 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu6", _pack())

        def CustMenu7(self) -> None:
            """Sakura's F_CUSTMENU_7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu7", _pack())

        def CustMenu8(self) -> None:
            """Sakura's F_CUSTMENU_8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu8", _pack())

        def CustMenu9(self) -> None:
            """Sakura's F_CUSTMENU_9 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu9", _pack())

        def CustMenu10(self) -> None:
            """Sakura's F_CUSTMENU_10 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu10", _pack())

        def CustMenu11(self) -> None:
            """Sakura's F_CUSTMENU_11 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu11", _pack())

        def CustMenu12(self) -> None:
            """Sakura's F_CUSTMENU_12 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu12", _pack())

        def CustMenu13(self) -> None:
            """Sakura's F_CUSTMENU_13 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu13", _pack())

        def CustMenu14(self) -> None:
            """Sakura's F_CUSTMENU_14 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu14", _pack())

        def CustMenu15(self) -> None:
            """Sakura's F_CUSTMENU_15 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu15", _pack())

        def CustMenu16(self) -> None:
            """Sakura's F_CUSTMENU_16 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu16", _pack())

        def CustMenu17(self) -> None:
            """Sakura's F_CUSTMENU_17 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu17", _pack())

        def CustMenu18(self) -> None:
            """Sakura's F_CUSTMENU_18 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu18", _pack())

        def CustMenu19(self) -> None:
            """Sakura's F_CUSTMENU_19 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu19", _pack())

        def CustMenu20(self) -> None:
            """Sakura's F_CUSTMENU_20 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu20", _pack())

        def CustMenu21(self) -> None:
            """Sakura's F_CUSTMENU_21 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu21", _pack())

        def CustMenu22(self) -> None:
            """Sakura's F_CUSTMENU_22 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu22", _pack())

        def CustMenu23(self) -> None:
            """Sakura's F_CUSTMENU_23 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu23", _pack())

        def CustMenu24(self) -> None:
            """Sakura's F_CUSTMENU_24 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CustMenu24", _pack())

        def SplitWinV(self) -> None:
            """Sakura's F_SPLIT_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("SplitWinV", _pack())

        def SplitWinH(self) -> None:
            """Sakura's F_SPLIT_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("SplitWinH", _pack())

        def SplitWinVH(self) -> None:
            """Sakura's F_SPLIT_VH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("SplitWinVH", _pack())

        def WinMaximize(self) -> None:
            """MATSU Editor's own F_WINMAXIMIZE (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("WinMaximize", _pack())

        def WinMinimize(self) -> None:
            """MATSU Editor's own F_WINMINIMIZE (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("WinMinimize", _pack())

        def WinRestore(self) -> None:
            """MATSU Editor's own F_WINRESTORE (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("WinRestore", _pack())

        def WinClose(self) -> None:
            """Sakura's F_WINCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WinClose", _pack())

        def WinCloseAll(self) -> None:
            """Sakura's F_WIN_CLOSEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WinCloseAll", _pack())

        def CascadeWin(self) -> None:
            """Sakura's F_CASCADE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("CascadeWin", _pack())

        def TileWinV(self) -> None:
            """Sakura's F_TILE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("TileWinV", _pack())

        def TileWinH(self) -> None:
            """Sakura's F_TILE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("TileWinH", _pack())

        def NextWindow(self) -> None:
            """Sakura's F_NEXTWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("NextWindow", _pack())

        def PrevWindow(self) -> None:
            """Sakura's F_PREVWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("PrevWindow", _pack())

        def WindowList(self) -> None:
            """Sakura's F_WINLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WindowList", _pack())

        def MaximizeV(self) -> None:
            """Sakura's F_MAXIMIZE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("MaximizeV", _pack())

        def MaximizeH(self) -> None:
            """Sakura's F_MAXIMIZE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("MaximizeH", _pack())

        def MinimizeAll(self) -> None:
            """Sakura's F_MINIMIZE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("MinimizeAll", _pack())

        def ReDraw(self) -> None:
            """Sakura's F_REDRAW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ReDraw", _pack())

        def ActivateWinOutput(self) -> None:
            """Sakura's F_WIN_OUTPUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ActivateWinOutput", _pack())

        def TraceOut(self, arg1: str = _UNSET, arg2: int = _UNSET) -> None:
            """MATSU Editor's own F_TRACEOUT (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("TraceOut", _pack(arg1, arg2))

        def WindowTopMost(self, arg1: int = _UNSET) -> None:
            """Sakura's F_TOPMOST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("WindowTopMost", _pack(arg1))

        def GroupClose(self) -> None:
            """Sakura's F_GROUPCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("GroupClose", _pack())

        def NextGroup(self) -> None:
            """Sakura's F_NEXTGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("NextGroup", _pack())

        def PrevGroup(self) -> None:
            """Sakura's F_PREVGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("PrevGroup", _pack())

        def TabMoveRight(self) -> None:
            """Sakura's F_TAB_MOVERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TabMoveRight", _pack())

        def TabMoveLeft(self) -> None:
            """Sakura's F_TAB_MOVELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TabMoveLeft", _pack())

        def TabSeparate(self) -> None:
            """Sakura's F_TAB_SEPARATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TabSeparate", _pack())

        def TabJointNext(self) -> None:
            """Sakura's F_TAB_JOINTNEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TabJointNext", _pack())

        def TabJointPrev(self) -> None:
            """Sakura's F_TAB_JOINTPREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TabJointPrev", _pack())

        def TabCloseOther(self) -> None:
            """Sakura's F_TAB_CLOSEOTHER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TabCloseOther", _pack())

        def TabCloseLeft(self) -> None:
            """Sakura's F_TAB_CLOSELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TabCloseLeft", _pack())

        def TabCloseRight(self) -> None:
            """Sakura's F_TAB_CLOSERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("TabCloseRight", _pack())

        def Complete(self) -> None:
            """Sakura's F_HOKAN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("Complete", _pack())

        def ToggleKeyHelpSearch(self, arg1: int = _UNSET) -> None:
            """Sakura's F_TOGGLE_KEY_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ToggleKeyHelpSearch", _pack(arg1))

        def HelpContents(self) -> None:
            """Sakura's F_HELP_CONTENTS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HelpContents", _pack())

        def HelpSearch(self) -> None:
            """Sakura's F_HELP_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("HelpSearch", _pack())

        def CommandList(self) -> None:
            """Sakura's F_MENU_ALLFUNC (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CommandList", _pack())

        def ExtHelp1(self) -> None:
            """Sakura's F_EXTHELP1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("ExtHelp1", _pack())

        def ExtHtmlHelp(self, arg1: str = _UNSET, arg2: str = _UNSET) -> None:
            """Sakura's F_EXTHTMLHELP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ExtHtmlHelp", _pack(arg1, arg2))

        def About(self) -> None:
            """Sakura's F_ABOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("About", _pack())

        def StatusMsg(self, arg1: str = _UNSET, arg2: int = _UNSET) -> None:
            """MATSU Editor's own F_STATUSMSG (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("StatusMsg", _pack(arg1, arg2))

        def MsgBeep(self, arg1: int = _UNSET) -> None:
            """MATSU Editor's own F_MSGBEEP (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("MsgBeep", _pack(arg1))

        def CommitUndoBuffer(self) -> None:
            """MATSU Editor's own F_COMMITUNDOBUFFER (not in Sakura). Runs the command that a
            key, the menu and a key macro run."""
            return call("CommitUndoBuffer", _pack())

        def AddRefUndoBuffer(self) -> None:
            """MATSU Editor's own F_ADDREFUNDOBUFFER (not in Sakura). Runs the command that a
            key, the menu and a key macro run."""
            return call("AddRefUndoBuffer", _pack())

        def SetUndoBuffer(self) -> None:
            """MATSU Editor's own F_SETUNDOBUFFER (not in Sakura). Runs the command that a key,
            the menu and a key macro run."""
            return call("SetUndoBuffer", _pack())

        def AppendUndoBufferCursor(self) -> None:
            """MATSU Editor's own F_APPENDUNDOBUFFERCURSOR (not in Sakura). Runs the command that
            a key, the menu and a key macro run."""
            return call("AppendUndoBufferCursor", _pack())

        def ClipboardEmpty(self) -> None:
            """MATSU Editor's own F_CLIPBOARDEMPTY (not in Sakura). Runs the command that a key,
            the menu and a key macro run."""
            return call("ClipboardEmpty", _pack())

        def SetViewTop(self, arg1: int = _UNSET) -> None:
            """MATSU Editor's own F_SETVIEWTOP (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("SetViewTop", _pack(arg1))

        def SetViewLeft(self, arg1: int = _UNSET) -> None:
            """MATSU Editor's own F_SETVIEWLEFT (not in Sakura). Runs the command that a key, the
            menu and a key macro run."""
            return call("SetViewLeft", _pack(arg1))

    def _fallback(self, name):
        # Any other name that starts like a Sakura macro name (an upper case letter or a digit)
        # is sent to the editor as it is. The editor answers a name it does not know with an
        # error (EditorError).
        if not name[:1].isupper() and not name[:1].isdigit():
            raise AttributeError(name)

        def method(*args):
            return call(name, _pack(*args))

        return method

    _Editor.__getattr__ = _fallback

    return _Editor()
