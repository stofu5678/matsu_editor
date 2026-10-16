# GENERATED from crates/ume-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files

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
            """0 when nothing is selected, 1 when text is selected. (Sakura returns 2 for a box
            selection, which is not supported yet.)"""
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

        def FileNew(self) -> None:
            """Sakura's F_FILENEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("FileNew", _pack())

        def FileOpen(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET, arg4: str = _UNSET) -> None:
            """Sakura's F_FILEOPEN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
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
            """Sakura's F_PUTFILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("PutFile", _pack(arg1, arg2, arg3))

        def InsFile(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET) -> None:
            """Sakura's F_INSFILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
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
            """Sakura's F_MOVECURSORLAYOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
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

        def PasteBox(self, arg1: int = _UNSET) -> None:
            """Sakura's F_PASTEBOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("PasteBox", _pack(arg1))

        def InsBoxText(self, arg1: str = _UNSET) -> None:
            """Sakura's F_INSBOXTEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
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
            """Sakura's F_CHGMOD_EOL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
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
            """Sakura's F_CHANGETYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
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
            """Sakura's F_SET_QUOTESTRING (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SetMsgQuoteStr", _pack(arg1))

        def TextWrapMethod(self, arg1: int = _UNSET) -> None:
            """Sakura's F_TEXTWRAPMETHOD (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
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
            """Sakura's F_WINMAXIMIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WinMaximize", _pack())

        def WinMinimize(self) -> None:
            """Sakura's F_WINMINIMIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("WinMinimize", _pack())

        def WinRestore(self) -> None:
            """Sakura's F_WINRESTORE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
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
            """Sakura's F_TRACEOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
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
            """Sakura's F_STATUSMSG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("StatusMsg", _pack(arg1, arg2))

        def MsgBeep(self, arg1: int = _UNSET) -> None:
            """Sakura's F_MSGBEEP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
            key, the menu and a key macro run. The last arguments may be left out. A command
            that this editor does not have yet answers the error -32001 (not implemented)."""
            return call("MsgBeep", _pack(arg1))

        def CommitUndoBuffer(self) -> None:
            """Sakura's F_COMMITUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("CommitUndoBuffer", _pack())

        def AddRefUndoBuffer(self) -> None:
            """Sakura's F_ADDREFUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("AddRefUndoBuffer", _pack())

        def SetUndoBuffer(self) -> None:
            """Sakura's F_SETUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SetUndoBuffer", _pack())

        def AppendUndoBufferCursor(self) -> None:
            """Sakura's F_APPENDUNDOBUFFERCURSOR (sakura_core/macro/CSMacroMgr.cpp). Runs the
            command that a key, the menu and a key macro run. The last arguments may be left
            out. A command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("AppendUndoBufferCursor", _pack())

        def ClipboardEmpty(self) -> None:
            """Sakura's F_CLIPBOARDEMPTY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
            that a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("ClipboardEmpty", _pack())

        def SetViewTop(self, arg1: int = _UNSET) -> None:
            """Sakura's F_SETVIEWTOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SetViewTop", _pack(arg1))

        def SetViewLeft(self, arg1: int = _UNSET) -> None:
            """Sakura's F_SETVIEWLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
            a key, the menu and a key macro run. The last arguments may be left out. A
            command that this editor does not have yet answers the error -32001 (not
            implemented)."""
            return call("SetViewLeft", _pack(arg1))

        def GetSaveFilename(self) -> str:
            """Sakura's F_GETSAVEFILENAME (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetSaveFilename", _pack())

        def ExpandParameter(self, arg1: str = _UNSET) -> str:
            """Sakura's F_EXPANDPARAMETER (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("ExpandParameter", _pack(arg1))

        def ChangeTabWidth(self, arg1: int = _UNSET) -> int:
            """Sakura's F_CHGTABWIDTH (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("ChangeTabWidth", _pack(arg1))

        def GetSelectLineFrom(self) -> int:
            """Sakura's F_GETSELLINEFROM (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetSelectLineFrom", _pack())

        def GetSelectColmFrom(self) -> int:
            """Sakura's F_GETSELCOLUMNFROM (sakura_core/macro/CSMacroMgr.cpp). A function with
            a result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetSelectColmFrom", _pack())

        def GetSelectColumnFrom(self) -> int:
            """Sakura's F_GETSELCOLUMNFROM (sakura_core/macro/CSMacroMgr.cpp). A function with
            a result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetSelectColumnFrom", _pack())

        def GetSelectLineTo(self) -> int:
            """Sakura's F_GETSELLINETO (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetSelectLineTo", _pack())

        def GetSelectColmTo(self) -> int:
            """Sakura's F_GETSELCOLUMNTO (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetSelectColmTo", _pack())

        def GetSelectColumnTo(self) -> int:
            """Sakura's F_GETSELCOLUMNTO (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetSelectColumnTo", _pack())

        def IsInsMode(self) -> int:
            """Sakura's F_ISINSMODE (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("IsInsMode", _pack())

        def GetCharCode(self) -> int:
            """Sakura's F_GETCHARCODE (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetCharCode", _pack())

        def GetLineCode(self) -> int:
            """Sakura's F_GETLINECODE (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetLineCode", _pack())

        def IsPossibleUndo(self) -> int:
            """Sakura's F_ISPOSSIBLEUNDO (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("IsPossibleUndo", _pack())

        def IsPossibleRedo(self) -> int:
            """Sakura's F_ISPOSSIBLEREDO (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("IsPossibleRedo", _pack())

        def ChangeWrapColm(self, arg1: int = _UNSET) -> int:
            """Sakura's F_CHGWRAPCOLUMN (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("ChangeWrapColm", _pack(arg1))

        def ChangeWrapColumn(self, arg1: int = _UNSET) -> int:
            """Sakura's F_CHGWRAPCOLUMN (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("ChangeWrapColumn", _pack(arg1))

        def IsCurTypeExt(self, arg1: str = _UNSET) -> int:
            """Sakura's F_ISCURTYPEEXT (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("IsCurTypeExt", _pack(arg1))

        def IsSameTypeExt(self, arg1: str = _UNSET, arg2: str = _UNSET) -> int:
            """Sakura's F_ISSAMETYPEEXT (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("IsSameTypeExt", _pack(arg1, arg2))

        def InputBox(self, arg1: str = _UNSET, arg2: str = _UNSET, arg3: int = _UNSET) -> str:
            """Sakura's F_INPUTBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("InputBox", _pack(arg1, arg2, arg3))

        def MessageBox(self, arg1: str = _UNSET, arg2: int = _UNSET) -> int:
            """Sakura's F_MESSAGEBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("MessageBox", _pack(arg1, arg2))

        def ErrorMsg(self, arg1: str = _UNSET) -> int:
            """Sakura's F_ERRORMSG (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("ErrorMsg", _pack(arg1))

        def WarnMsg(self, arg1: str = _UNSET) -> int:
            """Sakura's F_WARNMSG (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
            This editor answers the error -32001 (not implemented) while it cannot answer
            it."""
            return call("WarnMsg", _pack(arg1))

        def InfoMsg(self, arg1: str = _UNSET) -> int:
            """Sakura's F_INFOMSG (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
            This editor answers the error -32001 (not implemented) while it cannot answer
            it."""
            return call("InfoMsg", _pack(arg1))

        def OkCancelBox(self, arg1: str = _UNSET) -> int:
            """Sakura's F_OKCANCELBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("OkCancelBox", _pack(arg1))

        def YesNoBox(self, arg1: str = _UNSET) -> int:
            """Sakura's F_YESNOBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("YesNoBox", _pack(arg1))

        def CompareVersion(self, arg1: str = _UNSET, arg2: str = _UNSET) -> int:
            """Sakura's F_COMPAREVERSION (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("CompareVersion", _pack(arg1, arg2))

        def Sleep(self, arg1: int = _UNSET) -> int:
            """Sakura's F_MACROSLEEP (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("Sleep", _pack(arg1))

        def FileOpenDialog(self, arg1: str = _UNSET, arg2: str = _UNSET) -> str:
            """Sakura's F_FILEOPENDIALOG (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("FileOpenDialog", _pack(arg1, arg2))

        def FileSaveDialog(self, arg1: str = _UNSET, arg2: str = _UNSET) -> str:
            """Sakura's F_FILESAVEDIALOG (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("FileSaveDialog", _pack(arg1, arg2))

        def FolderDialog(self, arg1: str = _UNSET, arg2: str = _UNSET) -> str:
            """Sakura's F_FOLDERDIALOG (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("FolderDialog", _pack(arg1, arg2))

        def GetClipboard(self, arg1: int = _UNSET) -> str:
            """Sakura's F_GETCLIPBOARD (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetClipboard", _pack(arg1))

        def SetClipboard(self, arg1: int = _UNSET, arg2: str = _UNSET) -> int:
            """Sakura's F_SETCLIPBOARD (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("SetClipboard", _pack(arg1, arg2))

        def LayoutToLogicLineNum(self, arg1: int = _UNSET) -> int:
            """Sakura's F_LAYOUTTOLOGICLINENUM (sakura_core/macro/CSMacroMgr.cpp). A function
            with a result. This editor answers the error -32001 (not implemented) while it
            cannot answer it."""
            return call("LayoutToLogicLineNum", _pack(arg1))

        def LogicToLayoutLineNum(self, arg1: int = _UNSET, arg2: int = _UNSET) -> int:
            """Sakura's F_LOGICTOLAYOUTLINENUM (sakura_core/macro/CSMacroMgr.cpp). A function
            with a result. This editor answers the error -32001 (not implemented) while it
            cannot answer it."""
            return call("LogicToLayoutLineNum", _pack(arg1, arg2))

        def LineColumnToIndex(self, arg1: int = _UNSET, arg2: int = _UNSET) -> int:
            """Sakura's F_LINECOLUMNTOINDEX (sakura_core/macro/CSMacroMgr.cpp). A function with
            a result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("LineColumnToIndex", _pack(arg1, arg2))

        def LineIndexToColumn(self, arg1: int = _UNSET, arg2: int = _UNSET) -> int:
            """Sakura's F_LINEINDEXTOCOLUMN (sakura_core/macro/CSMacroMgr.cpp). A function with
            a result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("LineIndexToColumn", _pack(arg1, arg2))

        def GetCookie(self, arg1: str = _UNSET, arg2: str = _UNSET) -> str:
            """Sakura's F_GETCOOKIE (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetCookie", _pack(arg1, arg2))

        def GetCookieDefault(self, arg1: str = _UNSET, arg2: str = _UNSET, arg3: str = _UNSET) -> str:
            """Sakura's F_GETCOOKIEDEFAULT (sakura_core/macro/CSMacroMgr.cpp). A function with
            a result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetCookieDefault", _pack(arg1, arg2, arg3))

        def SetCookie(self, arg1: str = _UNSET, arg2: str = _UNSET, arg3: str = _UNSET) -> int:
            """Sakura's F_SETCOOKIE (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("SetCookie", _pack(arg1, arg2, arg3))

        def DeleteCookie(self, arg1: str = _UNSET, arg2: str = _UNSET) -> int:
            """Sakura's F_DELETECOOKIE (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("DeleteCookie", _pack(arg1, arg2))

        def GetCookieNames(self, arg1: str = _UNSET) -> str:
            """Sakura's F_GETCOOKIENAMES (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetCookieNames", _pack(arg1))

        def SetDrawSwitch(self, arg1: int = _UNSET) -> int:
            """Sakura's F_SETDRAWSWITCH (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("SetDrawSwitch", _pack(arg1))

        def GetDrawSwitch(self) -> int:
            """Sakura's F_GETDRAWSWITCH (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetDrawSwitch", _pack())

        def IsShownStatus(self) -> int:
            """Sakura's F_ISSHOWNSTATUS (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("IsShownStatus", _pack())

        def GetStrWidth(self, arg1: str = _UNSET, arg2: int = _UNSET) -> int:
            """Sakura's F_GETSTRWIDTH (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetStrWidth", _pack(arg1, arg2))

        def GetStrLayoutLength(self, arg1: str = _UNSET, arg2: int = _UNSET) -> int:
            """Sakura's F_GETSTRLAYOUTLENGTH (sakura_core/macro/CSMacroMgr.cpp). A function
            with a result. This editor answers the error -32001 (not implemented) while it
            cannot answer it."""
            return call("GetStrLayoutLength", _pack(arg1, arg2))

        def GetDefaultCharLength(self) -> int:
            """Sakura's F_GETDEFAULTCHARLENGTH (sakura_core/macro/CSMacroMgr.cpp). A function
            with a result. This editor answers the error -32001 (not implemented) while it
            cannot answer it."""
            return call("GetDefaultCharLength", _pack())

        def IsIncludeClipboardFormat(self, arg1: str = _UNSET) -> int:
            """Sakura's F_ISINCLUDECLIPBOARDFORMAT (sakura_core/macro/CSMacroMgr.cpp). A
            function with a result. This editor answers the error -32001 (not implemented)
            while it cannot answer it."""
            return call("IsIncludeClipboardFormat", _pack(arg1))

        def GetClipboardByFormat(self, arg1: str = _UNSET, arg2: int = _UNSET, arg3: int = _UNSET) -> str:
            """Sakura's F_GETCLIPBOARDBYFORMAT (sakura_core/macro/CSMacroMgr.cpp). A function
            with a result. This editor answers the error -32001 (not implemented) while it
            cannot answer it."""
            return call("GetClipboardByFormat", _pack(arg1, arg2, arg3))

        def SetClipboardByFormat(self, arg1: str = _UNSET, arg2: str = _UNSET, arg3: int = _UNSET, arg4: int = _UNSET) -> int:
            """Sakura's F_SETCLIPBOARDBYFORMAT (sakura_core/macro/CSMacroMgr.cpp). A function
            with a result. This editor answers the error -32001 (not implemented) while it
            cannot answer it."""
            return call("SetClipboardByFormat", _pack(arg1, arg2, arg3, arg4))

        def GetLineAttribute(self, arg1: int = _UNSET, arg2: int = _UNSET) -> int:
            """Sakura's F_GETLINEATTRIBUTE (sakura_core/macro/CSMacroMgr.cpp). A function with
            a result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetLineAttribute", _pack(arg1, arg2))

        def IsTextSelectingLock(self) -> int:
            """Sakura's F_ISTEXTSELECTINGLOCK (sakura_core/macro/CSMacroMgr.cpp). A function
            with a result. This editor answers the error -32001 (not implemented) while it
            cannot answer it."""
            return call("IsTextSelectingLock", _pack())

        def GetViewLines(self) -> int:
            """Sakura's F_GETVIEWLINES (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetViewLines", _pack())

        def GetViewColumns(self) -> int:
            """Sakura's F_GETVIEWCOLUMNS (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetViewColumns", _pack())

        def CreateMenu(self, arg1: int = _UNSET, arg2: str = _UNSET) -> int:
            """Sakura's F_CREATEMENU (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("CreateMenu", _pack(arg1, arg2))

        def GetViewTop(self) -> int:
            """Sakura's F_GETVIEWTOP (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("GetViewTop", _pack())

        def ChangeInsertSpace(self, arg1: int = _UNSET) -> int:
            """Sakura's F_CHGINSSPACE (sakura_core/macro/CSMacroMgr.cpp). A function with a
            result. This editor answers the error -32001 (not implemented) while it cannot
            answer it."""
            return call("ChangeInsertSpace", _pack(arg1))

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
