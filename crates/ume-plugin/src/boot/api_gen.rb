# GENERATED from crates/ume-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files

module UmeEditorApi
  UNSET = Object.new.freeze

  # The editor's macro API (the names and the arguments of Sakura's macros).
  class EditorObject
    attr_accessor :info

    def initialize(call)
      @call = call
      @info = nil
    end

    # GetLineCount(reserved: int) -> int
    #
    # The number of logical lines (lines separated by line breaks). The text after the last line
    # break is not a line unless it is not empty. The argument is reserved and must be 0.
    def GetLineCount(reserved)
      @call.call('GetLineCount', _pack(reserved))
    end

    # GetLineStr(line: int) -> string
    #
    # A logical line with its line break ("\n"; the last line has none unless the text ends with
    # one). 0 is the line of the cursor, 1 is the first line. Past the last line it is an empty
    # string.
    def GetLineStr(line)
      @call.call('GetLineStr', _pack(line))
    end

    # InsText(text: string) -> null
    #
    # Inserts the text at the cursor, replacing the selection. The cursor ends after the text.
    def InsText(text)
      @call.call('InsText', _pack(text))
    end

    # AddTail(text: string) -> null
    #
    # Moves to the end of the file, drops the selection, and inserts the text there.
    def AddTail(text)
      @call.call('AddTail', _pack(text))
    end

    # GoFileTop() -> null
    #
    # Moves the cursor to the start of the file and drops the selection.
    def GoFileTop
      @call.call('GoFileTop', _pack())
    end

    # GoFileEnd() -> null
    #
    # Moves the cursor to the end of the file and drops the selection.
    def GoFileEnd
      @call.call('GoFileEnd', _pack())
    end

    # Up() -> null
    #
    # Moves the cursor up one line and drops the selection.
    def Up
      @call.call('Up', _pack())
    end

    # Down() -> null
    #
    # Moves the cursor down one line and drops the selection.
    def Down
      @call.call('Down', _pack())
    end

    # SelectAll() -> null
    #
    # Selects the whole text. The cursor goes to the end.
    def SelectAll
      @call.call('SelectAll', _pack())
    end

    # GetSelectedString(reserved: int) -> string
    #
    # The selected text with its line breaks ("\n"), or an empty string when nothing is
    # selected. The argument is reserved and ignored.
    def GetSelectedString(reserved)
      @call.call('GetSelectedString', _pack(reserved))
    end

    # IsTextSelected() -> int
    #
    # 0 when nothing is selected, 1 when text is selected, 2 when a box selection is selected.
    def IsTextSelected
      @call.call('IsTextSelected', _pack())
    end

    # MoveCursor(line: int, column: int, option: int) -> null
    #
    # Moves the cursor to a line and a column, both counted from 1. The column counts UTF-16
    # code units. Option: 0 does not select, 1 selects from the old position. (2, the box
    # selection, is not supported.) Past the end of the text it goes to the end of the text.
    def MoveCursor(line, column, option)
      @call.call('MoveCursor', _pack(line, column, option))
    end

    # Jump(line: int, flag: int?) -> null
    #
    # Moves to the start of a line, counted from 1 in logical lines. Lines before 1 and after
    # the last one are limited to the first and the last line. Flag bit 0 (1, the default) is
    # the line break unit, which is the only unit supported yet; bit 1 (PL/SQL) is not
    # supported.
    def Jump(line, flag = UNSET)
      @call.call('Jump', _pack(line, flag))
    end

    # GetFilename() -> string
    #
    # The full path of the file, or an empty string when the text has no file.
    def GetFilename
      @call.call('GetFilename', _pack())
    end

    # GetSelectLineFrom() -> int
    #
    # The line where the selection starts, counted from 1 in layout lines (wrapped rows count as
    # lines). 0 when nothing is selected.
    def GetSelectLineFrom
      @call.call('GetSelectLineFrom', _pack())
    end

    # GetSelectColmFrom() -> int
    #
    # The column where the selection starts, counted from 1 in layout columns (cells: a
    # full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected. Same
    # as GetSelectColumnFrom.
    def GetSelectColmFrom
      @call.call('GetSelectColmFrom', _pack())
    end

    # GetSelectColumnFrom() -> int
    #
    # The column where the selection starts, counted from 1 in layout columns (cells: a
    # full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected.
    def GetSelectColumnFrom
      @call.call('GetSelectColumnFrom', _pack())
    end

    # GetSelectLineTo() -> int
    #
    # The line where the selection ends, counted from 1 in layout lines. 0 when nothing is
    # selected.
    def GetSelectLineTo
      @call.call('GetSelectLineTo', _pack())
    end

    # GetSelectColmTo() -> int
    #
    # The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
    # selected. Same as GetSelectColumnTo.
    def GetSelectColmTo
      @call.call('GetSelectColmTo', _pack())
    end

    # GetSelectColumnTo() -> int
    #
    # The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
    # selected.
    def GetSelectColumnTo
      @call.call('GetSelectColumnTo', _pack())
    end

    # IsInsMode() -> int
    #
    # 1 in insert mode, 0 in overwrite mode.
    def IsInsMode
      @call.call('IsInsMode', _pack())
    end

    # GetCharCode() -> int
    #
    # The encoding of the file: 0 Shift_JIS, 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, 5 UTF-7, 6
    # UTF-16 BE, 7 CESU-8, 8 Latin1.
    def GetCharCode
      @call.call('GetCharCode', _pack())
    end

    # GetLineCode() -> int
    #
    # The line ending that new line breaks get: 0 CR LF, 1 CR, 2 LF (3 NEL, 4 LS, 5 PS).
    def GetLineCode
      @call.call('GetLineCode', _pack())
    end

    # IsPossibleUndo() -> int
    #
    # 1 when there is something to undo, else 0.
    def IsPossibleUndo
      @call.call('IsPossibleUndo', _pack())
    end

    # IsPossibleRedo() -> int
    #
    # 1 when there is something to redo, else 0.
    def IsPossibleRedo
      @call.call('IsPossibleRedo', _pack())
    end

    # GetLineAttribute(line: int, type: int) -> int
    #
    # An attribute of a logical line (0 is the line of the cursor, 1 the first line). Type 2 is
    # the bookmark (1 or 0). The other types (0 modified, 1 modification number, 3 diff mark, 4
    # function list mark) answer the error -32001 until the editor keeps them. A negative line,
    # a line past the end and any other type are errors.
    def GetLineAttribute(line, type)
      @call.call('GetLineAttribute', _pack(line, type))
    end

    # IsTextSelectingLock() -> int
    #
    # 0 when the selection is not locked, 1 when it is locked (plain moves extend the
    # selection), 2 when it is locked and a box selection.
    def IsTextSelectingLock
      @call.call('IsTextSelectingLock', _pack())
    end

    # GetSaveFilename() -> string
    #
    # The full path that a save writes to, or an empty string when the text has no file. The
    # same as GetFilename here.
    def GetSaveFilename
      @call.call('GetSaveFilename', _pack())
    end

    # LayoutToLogicLineNum(layoutLine: int) -> int
    #
    # The logical line (up to a line break) that a layout line (a row on the screen) belongs to,
    # both counted from 1. The line after the last layout line gives the line after the last
    # logical line. Any other line outside the text is an error.
    def LayoutToLogicLineNum(layoutLine)
      @call.call('LayoutToLogicLineNum', _pack(layoutLine))
    end

    # LogicToLayoutLineNum(logicLine: int, logicIndex: int) -> int
    #
    # The layout line (a row on the screen) of a position in a logical line: the line and the
    # index in it (UTF-16 code units), both counted from 1. A line past the end gives the line
    # after the last layout line.
    def LogicToLayoutLineNum(logicLine, logicIndex)
      @call.call('LogicToLayoutLineNum', _pack(logicLine, logicIndex))
    end

    # LineColumnToIndex(layoutLine: int, layoutColumn: int) -> int
    #
    # The index (UTF-16 code units, from 1) in the logical line of a layout line and a layout
    # column (cells), both from 1. A column past the text gives the end of the text of the row.
    def LineColumnToIndex(layoutLine, layoutColumn)
      @call.call('LineColumnToIndex', _pack(layoutLine, layoutColumn))
    end

    # LineIndexToColumn(logicLine: int, logicIndex: int) -> int
    #
    # The layout column (cells, from 1) of a position in a logical line: the line and the index
    # in it (UTF-16 code units), both from 1. Tabs and full-width characters take their width.
    def LineIndexToColumn(logicLine, logicIndex)
      @call.call('LineIndexToColumn', _pack(logicLine, logicIndex))
    end

    # GetStrWidth(text: string, column: int?) -> int
    #
    # The width of a string in cells, starting at layout column `column` (from 1, default 1; it
    # matters for tabs). A CR LF at the end counts as the CR alone.
    def GetStrWidth(text, column = UNSET)
      @call.call('GetStrWidth', _pack(text, column))
    end

    # GetStrLayoutLength(text: string, column: int?) -> int
    #
    # The width of a string as a line of the document in cells, starting at layout column
    # `column` (from 1, default 1). A line ending at the end takes one cell.
    def GetStrLayoutLength(text, column = UNSET)
      @call.call('GetStrLayoutLength', _pack(text, column))
    end

    # GetDefaultCharLength() -> int
    #
    # The layout width of a half-width character. This editor lays out in cells, so it is 1.
    def GetDefaultCharLength
      @call.call('GetDefaultCharLength', _pack())
    end

    # GetViewLines() -> int
    #
    # How many rows the text area shows (at least 1).
    def GetViewLines
      @call.call('GetViewLines', _pack())
    end

    # GetViewColumns() -> int
    #
    # How many columns the text area shows (at least 1).
    def GetViewColumns
      @call.call('GetViewColumns', _pack())
    end

    # GetViewTop() -> int
    #
    # The layout line at the top of the text area, counted from 1.
    def GetViewTop
      @call.call('GetViewTop', _pack())
    end

    # ChangeTabWidth(width: int) -> int
    #
    # The tab width before the call. A width from 1 to 64 that differs from it becomes the tab
    # width of this window (not saved). Any other value, such as 0, only reads.
    def ChangeTabWidth(width)
      @call.call('ChangeTabWidth', _pack(width))
    end

    # ChangeWrapColm(column: int) -> int
    #
    # The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
    # column (not saved). Any other value only reads. Same as ChangeWrapColumn.
    def ChangeWrapColm(column)
      @call.call('ChangeWrapColm', _pack(column))
    end

    # ChangeWrapColumn(column: int) -> int
    #
    # The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
    # column (not saved). Any other value only reads.
    def ChangeWrapColumn(column)
      @call.call('ChangeWrapColumn', _pack(column))
    end

    # GetClipboard(option: int?) -> string
    #
    # The text on the clipboard, or an empty string when there is none. The line endings are
    # those of the source. The option is reserved.
    def GetClipboard(option = UNSET)
      @call.call('GetClipboard', _pack(option))
    end

    # SetClipboard(option: int?, text: string?) -> int
    #
    # Puts the text on the clipboard. Option bit 0 marks a box copy (a paste puts it in as a
    # box), bit 1 a row copy (a paste puts it in front of the row); with both, the box wins.
    # Returns 1 when it worked, 0 when not.
    def SetClipboard(option = UNSET, text = UNSET)
      @call.call('SetClipboard', _pack(option, text))
    end

    # IsIncludeClipboardFormat(format: string) -> int
    #
    # 1 when the clipboard has the format. Only text is known: CF_TEXT, CF_OEMTEXT and
    # CF_UNICODETEXT (or 1, 7, 13) are 1 when the clipboard has text; every other format is 0.
    def IsIncludeClipboardFormat(format)
      @call.call('IsIncludeClipboardFormat', _pack(format))
    end

    # GetClipboardByFormat(format: string, mode: int, endMode: int?) -> string
    #
    # The text of the clipboard in a text format (CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT), with the
    # line endings of the document. Mode -2 is the usual way and 3 is UTF-16 (the text itself);
    # the other modes and formats are errors (-32001). The end mode is not used.
    def GetClipboardByFormat(format, mode, endMode = UNSET)
      @call.call('GetClipboardByFormat', _pack(format, mode, endMode))
    end

    # SetClipboardByFormat(text: string, format: string, mode: int, endMode: int?) -> int
    #
    # Puts the text on the clipboard as text. The formats and modes are those of
    # GetClipboardByFormat. Returns 1 when it worked.
    def SetClipboardByFormat(text, format, mode, endMode = UNSET)
      @call.call('SetClipboardByFormat', _pack(text, format, mode, endMode))
    end

    # GetCookie(scope: string, name: string) -> string
    #
    # The value of a cookie, or an empty string when there is none. The scope is "window" or
    # "document".
    def GetCookie(scope, name)
      @call.call('GetCookie', _pack(scope, name))
    end

    # GetCookieDefault(scope: string, name: string, defaultValue: string) -> string
    #
    # The value of a cookie, or the default when there is none (an unknown scope still gives an
    # empty string). The scope is "window" or "document".
    def GetCookieDefault(scope, name, defaultValue)
      @call.call('GetCookieDefault', _pack(scope, name, defaultValue))
    end

    # SetCookie(scope: string, name: string, value: string) -> int
    #
    # Sets a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a name with a
    # character that a name cannot have (Sakura lets in digits, letters and every character from
    # "_" up in the character table). The scope is "window" or "document".
    def SetCookie(scope, name, value)
      @call.call('SetCookie', _pack(scope, name, value))
    end

    # DeleteCookie(scope: string, name: string) -> int
    #
    # Deletes a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a bad name, 5
    # when there is no such cookie. The scope is "window" or "document".
    def DeleteCookie(scope, name)
      @call.call('DeleteCookie', _pack(scope, name))
    end

    # GetCookieNames(scope: string) -> string
    #
    # The names of the cookies of a scope, joined with commas, in the order of the names (UTF-16
    # code units). An unknown scope gives an empty string. The scope is "window" or "document".
    def GetCookieNames(scope)
      @call.call('GetCookieNames', _pack(scope))
    end

    # CompareVersion(versionA: string, versionB: string) -> int
    #
    # Compares two version strings: 0 when equal, positive when A is newer, negative when B is.
    # Four parts at most; a part has two digits at most; "alpha", "beta", "rc" and "pl" put a
    # part before or after the release (like PHP's version_compare).
    def CompareVersion(versionA, versionB)
      @call.call('CompareVersion', _pack(versionA, versionB))
    end

    # SetDrawSwitch(on: int) -> int
    #
    # Turns the draw switch on (non-zero) or off (0) and returns the state before (1 or 0). The
    # editor keeps the flag for GetDrawSwitch but still paints the text when it likes.
    def SetDrawSwitch(on)
      @call.call('SetDrawSwitch', _pack(on))
    end

    # GetDrawSwitch() -> int
    #
    # 1 when the draw switch is on (the start state), 0 when a macro turned it off.
    def GetDrawSwitch
      @call.call('GetDrawSwitch', _pack())
    end

    # IsShownStatus() -> int
    #
    # 1 when the status bar is shown, else 0.
    def IsShownStatus
      @call.call('IsShownStatus', _pack())
    end

    # IsCurTypeExt(extension: string) -> int
    #
    # 1 when the document type of this file is the type of the extension (looked up in the
    # extension lists of the types in the settings file, as a file name). Without type settings
    # every extension is the basic type.
    def IsCurTypeExt(extension)
      @call.call('IsCurTypeExt', _pack(extension))
    end

    # IsSameTypeExt(extension1: string, extension2: string) -> int
    #
    # 1 when the two extensions belong to the same document type (see IsCurTypeExt).
    def IsSameTypeExt(extension1, extension2)
      @call.call('IsSameTypeExt', _pack(extension1, extension2))
    end

    # ExpandParameter(format: string) -> string
    #
    # Replaces the codes that start with $ in a string: $$ ($), $F (full path), $f (file name),
    # $g (file name without the extension), $/ (full path with /), $N (as $F), $b (extension),
    # $e and $E (folder), $C (the selected text or the word at the caret), $x and $y (caret
    # column in UTF-16 units and line, from 1), $S (the editor), $M (the macro file), $V
    # (version), $A (name), and conditions ${R?a$:b$:c$} (R, w, M, U, N, I). A text without a
    # file is (無題). $d $t $D $T $B $Q $I are not supported and stay as written. At most 2047
    # UTF-16 units.
    def ExpandParameter(format)
      @call.call('ExpandParameter', _pack(format))
    end

    # MessageBox(message: string, type: int?) -> int
    #
    # Shows a message box and waits. The type is Windows' MB_ value: the low nibble chooses the
    # buttons (0 OK, 1 OK/Cancel, 2 Abort/Retry/Ignore, 3 Yes/No/Cancel, 4 Yes/No, 5
    # Retry/Cancel, 6 Cancel/Try again/Continue), the icon bits (0x10 stop, 0x20 question, 0x30
    # exclamation, 0x40 information) the icon; the default button and the modality are ignored.
    # Returns the button as Windows numbers it: 1 OK, 2 Cancel, 3 Abort, 4 Retry, 5 Ignore, 6
    # Yes, 7 No, 10 Try again, 11 Continue. A box that is closed without an answer gives Cancel
    # (the last button when there is none).
    def MessageBox(message, type = UNSET)
      @call.call('MessageBox', _pack(message, type))
    end

    # ErrorMsg(message: string) -> int
    #
    # Shows an error message with an OK button and waits. Returns 1.
    def ErrorMsg(message)
      @call.call('ErrorMsg', _pack(message))
    end

    # WarnMsg(message: string) -> int
    #
    # Shows a warning with an OK button and waits. Returns 1.
    def WarnMsg(message)
      @call.call('WarnMsg', _pack(message))
    end

    # InfoMsg(message: string) -> int
    #
    # Shows an information message with an OK button and waits. Returns 1.
    def InfoMsg(message)
      @call.call('InfoMsg', _pack(message))
    end

    # OkCancelBox(message: string) -> int
    #
    # Asks a question with OK and Cancel and waits. Returns 1 (OK) or 2 (Cancel).
    def OkCancelBox(message)
      @call.call('OkCancelBox', _pack(message))
    end

    # YesNoBox(message: string) -> int
    #
    # Asks a question with Yes and No and waits. Returns 6 (Yes) or 7 (No).
    def YesNoBox(message)
      @call.call('YesNoBox', _pack(message))
    end

    # FileOpenDialog(defaultName: string?, filter: string?) -> string
    #
    # Lets the user choose a file to open and returns its full path, or an empty string when the
    # dialog is cancelled. The dialog starts in the folder of the default name (or of the file
    # in the window). The filter is not used: the dialog of the platform does not take one.
    def FileOpenDialog(defaultName = UNSET, filter = UNSET)
      @call.call('FileOpenDialog', _pack(defaultName, filter))
    end

    # FileSaveDialog(defaultName: string?, filter: string?) -> string
    #
    # Lets the user choose a file name to save under and returns the full path, or an empty
    # string when the dialog is cancelled. The dialog starts with the default name. The filter
    # is not used.
    def FileSaveDialog(defaultName = UNSET, filter = UNSET)
      @call.call('FileSaveDialog', _pack(defaultName, filter))
    end

    # FolderDialog(message: string?, defaultFolder: string?) -> string
    #
    # Lets the user choose a folder and returns its full path, or an empty string when the
    # dialog is cancelled. The message is shown by the platform where it has room for one.
    def FolderDialog(message = UNSET, defaultFolder = UNSET)
      @call.call('FolderDialog', _pack(message, defaultFolder))
    end

    # Sleep(milliseconds: int) -> int
    #
    # Waits for the given number of milliseconds (the window stays alive, and the macro can be
    # cancelled). Returns 0.
    def Sleep(milliseconds)
      @call.call('Sleep', _pack(milliseconds))
    end

    # FileNew() -> null
    #
    # Sakura's F_FILENEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def FileNew
      @call.call('FileNew', _pack())
    end

    # FileOpen(arg1: string?, arg2: int?, arg3: int?, arg4: string?) -> null
    #
    # Sakura's F_FILEOPEN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def FileOpen(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET, arg4 = UNSET)
      @call.call('FileOpen', _pack(arg1, arg2, arg3, arg4))
    end

    # FileSave() -> null
    #
    # Sakura's F_FILESAVE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def FileSave
      @call.call('FileSave', _pack())
    end

    # FileSaveAll() -> null
    #
    # Sakura's F_FILESAVEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileSaveAll
      @call.call('FileSaveAll', _pack())
    end

    # FileSaveAsDialog(arg1: string?, arg2: int?, arg3: int?) -> null
    #
    # Sakura's F_FILESAVEAS_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileSaveAsDialog(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('FileSaveAsDialog', _pack(arg1, arg2, arg3))
    end

    # FileSaveAs(arg1: string?, arg2: int?, arg3: int?) -> null
    #
    # Sakura's F_FILESAVEAS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def FileSaveAs(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('FileSaveAs', _pack(arg1, arg2, arg3))
    end

    # FileClose() -> null
    #
    # Sakura's F_FILECLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def FileClose
      @call.call('FileClose', _pack())
    end

    # FileCloseOpen() -> null
    #
    # Sakura's F_FILECLOSE_OPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileCloseOpen
      @call.call('FileCloseOpen', _pack())
    end

    # FileReopen(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopen(arg1 = UNSET)
      @call.call('FileReopen', _pack(arg1))
    end

    # FileReopenSJIS(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopenSJIS(arg1 = UNSET)
      @call.call('FileReopenSJIS', _pack(arg1))
    end

    # FileReopenJIS(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopenJIS(arg1 = UNSET)
      @call.call('FileReopenJIS', _pack(arg1))
    end

    # FileReopenEUC(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopenEUC(arg1 = UNSET)
      @call.call('FileReopenEUC', _pack(arg1))
    end

    # FileReopenLatin1(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_LATIN1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopenLatin1(arg1 = UNSET)
      @call.call('FileReopenLatin1', _pack(arg1))
    end

    # FileReopenUNICODE(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_UNICODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopenUNICODE(arg1 = UNSET)
      @call.call('FileReopenUNICODE', _pack(arg1))
    end

    # FileReopenUNICODEBE(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_UNICODEBE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
    # a key, the menu and a key macro run. The last arguments may be left out. A command that
    # this editor does not have yet answers the error -32001 (not implemented).
    def FileReopenUNICODEBE(arg1 = UNSET)
      @call.call('FileReopenUNICODEBE', _pack(arg1))
    end

    # FileReopenUTF8(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopenUTF8(arg1 = UNSET)
      @call.call('FileReopenUTF8', _pack(arg1))
    end

    # FileReopenCESU8(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_CESU8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopenCESU8(arg1 = UNSET)
      @call.call('FileReopenCESU8', _pack(arg1))
    end

    # FileReopenUTF7(arg1: int?) -> null
    #
    # Sakura's F_FILE_REOPEN_UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FileReopenUTF7(arg1 = UNSET)
      @call.call('FileReopenUTF7', _pack(arg1))
    end

    # Print() -> null
    #
    # Sakura's F_PRINT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Print
      @call.call('Print', _pack())
    end

    # PrintPreview() -> null
    #
    # Sakura's F_PRINT_PREVIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def PrintPreview
      @call.call('PrintPreview', _pack())
    end

    # PrintPageSetup() -> null
    #
    # Sakura's F_PRINT_PAGESETUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def PrintPageSetup
      @call.call('PrintPageSetup', _pack())
    end

    # OpenHfromtoC() -> null
    #
    # Sakura's F_OPEN_HfromtoC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def OpenHfromtoC
      @call.call('OpenHfromtoC', _pack())
    end

    # ActivateSQLPLUS() -> null
    #
    # Sakura's F_ACTIVATE_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ActivateSQLPLUS
      @call.call('ActivateSQLPLUS', _pack())
    end

    # ExecSQLPLUS() -> null
    #
    # Sakura's F_PLSQL_COMPILE_ON_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    def ExecSQLPLUS
      @call.call('ExecSQLPLUS', _pack())
    end

    # Browse() -> null
    #
    # Sakura's F_BROWSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Browse
      @call.call('Browse', _pack())
    end

    # ViewMode() -> null
    #
    # Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ViewMode
      @call.call('ViewMode', _pack())
    end

    # ReadOnly() -> null
    #
    # Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ReadOnly
      @call.call('ReadOnly', _pack())
    end

    # PropertyFile() -> null
    #
    # Sakura's F_PROPERTY_FILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def PropertyFile
      @call.call('PropertyFile', _pack())
    end

    # ExitAllEditors() -> null
    #
    # Sakura's F_EXITALLEDITORS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ExitAllEditors
      @call.call('ExitAllEditors', _pack())
    end

    # ExitAll() -> null
    #
    # Sakura's F_EXITALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ExitAll
      @call.call('ExitAll', _pack())
    end

    # PutFile(arg1: string?, arg2: int?, arg3: int?) -> null
    #
    # Sakura's F_PUTFILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def PutFile(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('PutFile', _pack(arg1, arg2, arg3))
    end

    # InsFile(arg1: string?, arg2: int?, arg3: int?) -> null
    #
    # Sakura's F_INSFILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def InsFile(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('InsFile', _pack(arg1, arg2, arg3))
    end

    # Char(arg1: int?) -> null
    #
    # Sakura's F_WCHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Char(arg1 = UNSET)
      @call.call('Char', _pack(arg1))
    end

    # CharIme(arg1: int?) -> null
    #
    # Sakura's F_IME_CHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CharIme(arg1 = UNSET)
      @call.call('CharIme', _pack(arg1))
    end

    # Undo() -> null
    #
    # Sakura's F_UNDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Undo
      @call.call('Undo', _pack())
    end

    # Redo() -> null
    #
    # Sakura's F_REDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Redo
      @call.call('Redo', _pack())
    end

    # Delete() -> null
    #
    # Sakura's F_DELETE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Delete
      @call.call('Delete', _pack())
    end

    # DeleteBack() -> null
    #
    # Sakura's F_DELETE_BACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def DeleteBack
      @call.call('DeleteBack', _pack())
    end

    # WordDeleteToStart() -> null
    #
    # Sakura's F_WordDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WordDeleteToStart
      @call.call('WordDeleteToStart', _pack())
    end

    # WordDeleteToEnd() -> null
    #
    # Sakura's F_WordDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WordDeleteToEnd
      @call.call('WordDeleteToEnd', _pack())
    end

    # WordCut() -> null
    #
    # Sakura's F_WordCut (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WordCut
      @call.call('WordCut', _pack())
    end

    # WordDelete() -> null
    #
    # Sakura's F_WordDelete (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WordDelete
      @call.call('WordDelete', _pack())
    end

    # LineCutToStart() -> null
    #
    # Sakura's F_LineCutToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def LineCutToStart
      @call.call('LineCutToStart', _pack())
    end

    # LineCutToEnd() -> null
    #
    # Sakura's F_LineCutToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def LineCutToEnd
      @call.call('LineCutToEnd', _pack())
    end

    # LineDeleteToStart() -> null
    #
    # Sakura's F_LineDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def LineDeleteToStart
      @call.call('LineDeleteToStart', _pack())
    end

    # LineDeleteToEnd() -> null
    #
    # Sakura's F_LineDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def LineDeleteToEnd
      @call.call('LineDeleteToEnd', _pack())
    end

    # CutLine() -> null
    #
    # Sakura's F_CUT_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CutLine
      @call.call('CutLine', _pack())
    end

    # DeleteLine() -> null
    #
    # Sakura's F_DELETE_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def DeleteLine
      @call.call('DeleteLine', _pack())
    end

    # DuplicateLine() -> null
    #
    # Sakura's F_DUPLICATELINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def DuplicateLine
      @call.call('DuplicateLine', _pack())
    end

    # IndentTab() -> null
    #
    # Sakura's F_INDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def IndentTab
      @call.call('IndentTab', _pack())
    end

    # UnindentTab() -> null
    #
    # Sakura's F_UNINDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def UnindentTab
      @call.call('UnindentTab', _pack())
    end

    # IndentSpace() -> null
    #
    # Sakura's F_INDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def IndentSpace
      @call.call('IndentSpace', _pack())
    end

    # UnindentSpace() -> null
    #
    # Sakura's F_UNINDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def UnindentSpace
      @call.call('UnindentSpace', _pack())
    end

    # LTrim() -> null
    #
    # Sakura's F_LTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def LTrim
      @call.call('LTrim', _pack())
    end

    # RTrim() -> null
    #
    # Sakura's F_RTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def RTrim
      @call.call('RTrim', _pack())
    end

    # SortAsc() -> null
    #
    # Sakura's F_SORT_ASC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SortAsc
      @call.call('SortAsc', _pack())
    end

    # SortDesc() -> null
    #
    # Sakura's F_SORT_DESC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SortDesc
      @call.call('SortDesc', _pack())
    end

    # Merge() -> null
    #
    # Sakura's F_MERGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Merge
      @call.call('Merge', _pack())
    end

    # Left() -> null
    #
    # Sakura's F_LEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Left
      @call.call('Left', _pack())
    end

    # Right() -> null
    #
    # Sakura's F_RIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Right
      @call.call('Right', _pack())
    end

    # Up2() -> null
    #
    # Sakura's F_UP2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Up2
      @call.call('Up2', _pack())
    end

    # Down2() -> null
    #
    # Sakura's F_DOWN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Down2
      @call.call('Down2', _pack())
    end

    # WordLeft() -> null
    #
    # Sakura's F_WORDLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WordLeft
      @call.call('WordLeft', _pack())
    end

    # WordRight() -> null
    #
    # Sakura's F_WORDRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WordRight
      @call.call('WordRight', _pack())
    end

    # GoLineTop(arg1: int?) -> null
    #
    # Sakura's F_GOLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def GoLineTop(arg1 = UNSET)
      @call.call('GoLineTop', _pack(arg1))
    end

    # GoLineEnd(arg1: int?) -> null
    #
    # Sakura's F_GOLINEEND (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def GoLineEnd(arg1 = UNSET)
      @call.call('GoLineEnd', _pack(arg1))
    end

    # HalfPageUp(arg1: int?) -> null
    #
    # Sakura's F_HalfPageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def HalfPageUp(arg1 = UNSET)
      @call.call('HalfPageUp', _pack(arg1))
    end

    # HalfPageDown(arg1: int?) -> null
    #
    # Sakura's F_HalfPageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HalfPageDown(arg1 = UNSET)
      @call.call('HalfPageDown', _pack(arg1))
    end

    # PageUp(arg1: int?) -> null
    #
    # Sakura's F_1PageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def PageUp(arg1 = UNSET)
      @call.call('PageUp', _pack(arg1))
    end

    # PageDown(arg1: int?) -> null
    #
    # Sakura's F_1PageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def PageDown(arg1 = UNSET)
      @call.call('PageDown', _pack(arg1))
    end

    # CurLineCenter() -> null
    #
    # Sakura's F_CURLINECENTER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CurLineCenter
      @call.call('CurLineCenter', _pack())
    end

    # CurLineTop() -> null
    #
    # Sakura's F_CURLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CurLineTop
      @call.call('CurLineTop', _pack())
    end

    # CurLineBottom() -> null
    #
    # Sakura's F_CURLINEBOTTOM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CurLineBottom
      @call.call('CurLineBottom', _pack())
    end

    # MoveHistPrev() -> null
    #
    # Sakura's F_JUMPHIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def MoveHistPrev
      @call.call('MoveHistPrev', _pack())
    end

    # MoveHistNext() -> null
    #
    # Sakura's F_JUMPHIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def MoveHistNext
      @call.call('MoveHistNext', _pack())
    end

    # MoveHistSet() -> null
    #
    # Sakura's F_JUMPHIST_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def MoveHistSet
      @call.call('MoveHistSet', _pack())
    end

    # F_WndScrollDown() -> null
    #
    # Sakura's F_WndScrollDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def F_WndScrollDown
      @call.call('F_WndScrollDown', _pack())
    end

    # F_WndScrollUp() -> null
    #
    # Sakura's F_WndScrollUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def F_WndScrollUp
      @call.call('F_WndScrollUp', _pack())
    end

    # GoNextParagraph() -> null
    #
    # Sakura's F_GONEXTPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoNextParagraph
      @call.call('GoNextParagraph', _pack())
    end

    # GoPrevParagraph() -> null
    #
    # Sakura's F_GOPREVPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoPrevParagraph
      @call.call('GoPrevParagraph', _pack())
    end

    # GoModifyLineNext() -> null
    #
    # Sakura's F_MODIFYLINE_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoModifyLineNext
      @call.call('GoModifyLineNext', _pack())
    end

    # GoModifyLinePrev() -> null
    #
    # Sakura's F_MODIFYLINE_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoModifyLinePrev
      @call.call('GoModifyLinePrev', _pack())
    end

    # MoveCursorLayout(arg1: int?, arg2: int?, arg3: int?) -> null
    #
    # Sakura's F_MOVECURSORLAYOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def MoveCursorLayout(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('MoveCursorLayout', _pack(arg1, arg2, arg3))
    end

    # WheelUp(arg1: int?) -> null
    #
    # Sakura's F_WHEELUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WheelUp(arg1 = UNSET)
      @call.call('WheelUp', _pack(arg1))
    end

    # WheelDown(arg1: int?) -> null
    #
    # Sakura's F_WHEELDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WheelDown(arg1 = UNSET)
      @call.call('WheelDown', _pack(arg1))
    end

    # WheelLeft(arg1: int?) -> null
    #
    # Sakura's F_WHEELLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WheelLeft(arg1 = UNSET)
      @call.call('WheelLeft', _pack(arg1))
    end

    # WheelRight(arg1: int?) -> null
    #
    # Sakura's F_WHEELRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WheelRight(arg1 = UNSET)
      @call.call('WheelRight', _pack(arg1))
    end

    # WheelPageUp(arg1: int?) -> null
    #
    # Sakura's F_WHEELPAGEUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WheelPageUp(arg1 = UNSET)
      @call.call('WheelPageUp', _pack(arg1))
    end

    # WheelPageDown(arg1: int?) -> null
    #
    # Sakura's F_WHEELPAGEDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WheelPageDown(arg1 = UNSET)
      @call.call('WheelPageDown', _pack(arg1))
    end

    # WheelPageLeft(arg1: int?) -> null
    #
    # Sakura's F_WHEELPAGELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WheelPageLeft(arg1 = UNSET)
      @call.call('WheelPageLeft', _pack(arg1))
    end

    # WheelPageRight(arg1: int?) -> null
    #
    # Sakura's F_WHEELPAGERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WheelPageRight(arg1 = UNSET)
      @call.call('WheelPageRight', _pack(arg1))
    end

    # SelectWord() -> null
    #
    # Sakura's F_SELECTWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SelectWord
      @call.call('SelectWord', _pack())
    end

    # SelectLine(arg1: int?) -> null
    #
    # Sakura's F_SELECTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SelectLine(arg1 = UNSET)
      @call.call('SelectLine', _pack(arg1))
    end

    # BeginSelect() -> null
    #
    # Sakura's F_BEGIN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def BeginSelect
      @call.call('BeginSelect', _pack())
    end

    # Up_Sel() -> null
    #
    # Sakura's F_UP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Up_Sel
      @call.call('Up_Sel', _pack())
    end

    # Down_Sel() -> null
    #
    # Sakura's F_DOWN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Down_Sel
      @call.call('Down_Sel', _pack())
    end

    # Left_Sel() -> null
    #
    # Sakura's F_LEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Left_Sel
      @call.call('Left_Sel', _pack())
    end

    # Right_Sel() -> null
    #
    # Sakura's F_RIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Right_Sel
      @call.call('Right_Sel', _pack())
    end

    # Up2_Sel() -> null
    #
    # Sakura's F_UP2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Up2_Sel
      @call.call('Up2_Sel', _pack())
    end

    # Down2_Sel() -> null
    #
    # Sakura's F_DOWN2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Down2_Sel
      @call.call('Down2_Sel', _pack())
    end

    # WordLeft_Sel() -> null
    #
    # Sakura's F_WORDLEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WordLeft_Sel
      @call.call('WordLeft_Sel', _pack())
    end

    # WordRight_Sel() -> null
    #
    # Sakura's F_WORDRIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WordRight_Sel
      @call.call('WordRight_Sel', _pack())
    end

    # GoLineTop_Sel(arg1: int?) -> null
    #
    # Sakura's F_GOLINETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoLineTop_Sel(arg1 = UNSET)
      @call.call('GoLineTop_Sel', _pack(arg1))
    end

    # GoLineEnd_Sel(arg1: int?) -> null
    #
    # Sakura's F_GOLINEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoLineEnd_Sel(arg1 = UNSET)
      @call.call('GoLineEnd_Sel', _pack(arg1))
    end

    # HalfPageUp_Sel(arg1: int?) -> null
    #
    # Sakura's F_HalfPageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HalfPageUp_Sel(arg1 = UNSET)
      @call.call('HalfPageUp_Sel', _pack(arg1))
    end

    # HalfPageDown_Sel(arg1: int?) -> null
    #
    # Sakura's F_HalfPageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HalfPageDown_Sel(arg1 = UNSET)
      @call.call('HalfPageDown_Sel', _pack(arg1))
    end

    # PageUp_Sel(arg1: int?) -> null
    #
    # Sakura's F_1PageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def PageUp_Sel(arg1 = UNSET)
      @call.call('PageUp_Sel', _pack(arg1))
    end

    # PageDown_Sel(arg1: int?) -> null
    #
    # Sakura's F_1PageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def PageDown_Sel(arg1 = UNSET)
      @call.call('PageDown_Sel', _pack(arg1))
    end

    # GoFileTop_Sel() -> null
    #
    # Sakura's F_GOFILETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoFileTop_Sel
      @call.call('GoFileTop_Sel', _pack())
    end

    # GoFileEnd_Sel() -> null
    #
    # Sakura's F_GOFILEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoFileEnd_Sel
      @call.call('GoFileEnd_Sel', _pack())
    end

    # GoNextParagraph_Sel() -> null
    #
    # Sakura's F_GONEXTPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoNextParagraph_Sel
      @call.call('GoNextParagraph_Sel', _pack())
    end

    # GoPrevParagraph_Sel() -> null
    #
    # Sakura's F_GOPREVPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoPrevParagraph_Sel
      @call.call('GoPrevParagraph_Sel', _pack())
    end

    # GoModifyLineNext_Sel() -> null
    #
    # Sakura's F_MODIFYLINE_NEXT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoModifyLineNext_Sel
      @call.call('GoModifyLineNext_Sel', _pack())
    end

    # GoModifyLinePrev_Sel() -> null
    #
    # Sakura's F_MODIFYLINE_PREV_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoModifyLinePrev_Sel
      @call.call('GoModifyLinePrev_Sel', _pack())
    end

    # BeginBoxSelect() -> null
    #
    # Sakura's F_BEGIN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def BeginBoxSelect
      @call.call('BeginBoxSelect', _pack())
    end

    # Up_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_UP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Up_BoxSel(arg1 = UNSET)
      @call.call('Up_BoxSel', _pack(arg1))
    end

    # Down_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_DOWN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Down_BoxSel(arg1 = UNSET)
      @call.call('Down_BoxSel', _pack(arg1))
    end

    # Left_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_LEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Left_BoxSel(arg1 = UNSET)
      @call.call('Left_BoxSel', _pack(arg1))
    end

    # Right_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_RIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Right_BoxSel(arg1 = UNSET)
      @call.call('Right_BoxSel', _pack(arg1))
    end

    # Up2_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_UP2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Up2_BoxSel(arg1 = UNSET)
      @call.call('Up2_BoxSel', _pack(arg1))
    end

    # Down2_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_DOWN2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Down2_BoxSel(arg1 = UNSET)
      @call.call('Down2_BoxSel', _pack(arg1))
    end

    # WordLeft_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_WORDLEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WordLeft_BoxSel(arg1 = UNSET)
      @call.call('WordLeft_BoxSel', _pack(arg1))
    end

    # WordRight_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_WORDRIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WordRight_BoxSel(arg1 = UNSET)
      @call.call('WordRight_BoxSel', _pack(arg1))
    end

    # GoLogicalLineTop_BoxSel(arg1: int?, arg2: int?) -> null
    #
    # Sakura's F_GOLOGICALLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
    # a key, the menu and a key macro run. The last arguments may be left out. A command that
    # this editor does not have yet answers the error -32001 (not implemented).
    def GoLogicalLineTop_BoxSel(arg1 = UNSET, arg2 = UNSET)
      @call.call('GoLogicalLineTop_BoxSel', _pack(arg1, arg2))
    end

    # GoLineTop_BoxSel(arg1: int?, arg2: int?) -> null
    #
    # Sakura's F_GOLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoLineTop_BoxSel(arg1 = UNSET, arg2 = UNSET)
      @call.call('GoLineTop_BoxSel', _pack(arg1, arg2))
    end

    # GoLineEnd_BoxSel(arg1: int?, arg2: int?) -> null
    #
    # Sakura's F_GOLINEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoLineEnd_BoxSel(arg1 = UNSET, arg2 = UNSET)
      @call.call('GoLineEnd_BoxSel', _pack(arg1, arg2))
    end

    # HalfPageUp_BoxSel(arg1: int?, arg2: int?) -> null
    #
    # Sakura's F_HalfPageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HalfPageUp_BoxSel(arg1 = UNSET, arg2 = UNSET)
      @call.call('HalfPageUp_BoxSel', _pack(arg1, arg2))
    end

    # HalfPageDown_BoxSel(arg1: int?, arg2: int?) -> null
    #
    # Sakura's F_HalfPageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HalfPageDown_BoxSel(arg1 = UNSET, arg2 = UNSET)
      @call.call('HalfPageDown_BoxSel', _pack(arg1, arg2))
    end

    # PageUp_BoxSel(arg1: int?, arg2: int?) -> null
    #
    # Sakura's F_1PageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def PageUp_BoxSel(arg1 = UNSET, arg2 = UNSET)
      @call.call('PageUp_BoxSel', _pack(arg1, arg2))
    end

    # PageDown_BoxSel(arg1: int?, arg2: int?) -> null
    #
    # Sakura's F_1PageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def PageDown_BoxSel(arg1 = UNSET, arg2 = UNSET)
      @call.call('PageDown_BoxSel', _pack(arg1, arg2))
    end

    # GoFileTop_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_GOFILETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoFileTop_BoxSel(arg1 = UNSET)
      @call.call('GoFileTop_BoxSel', _pack(arg1))
    end

    # GoFileEnd_BoxSel(arg1: int?) -> null
    #
    # Sakura's F_GOFILEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GoFileEnd_BoxSel(arg1 = UNSET)
      @call.call('GoFileEnd_BoxSel', _pack(arg1))
    end

    # Cut() -> null
    #
    # Sakura's F_CUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Cut
      @call.call('Cut', _pack())
    end

    # Copy() -> null
    #
    # Sakura's F_COPY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Copy
      @call.call('Copy', _pack())
    end

    # Paste(arg1: int?) -> null
    #
    # Sakura's F_PASTE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Paste(arg1 = UNSET)
      @call.call('Paste', _pack(arg1))
    end

    # CopyAddCRLF() -> null
    #
    # Sakura's F_COPY_ADDCRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CopyAddCRLF
      @call.call('CopyAddCRLF', _pack())
    end

    # CopyCRLF() -> null
    #
    # Sakura's F_COPY_CRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CopyCRLF
      @call.call('CopyCRLF', _pack())
    end

    # PasteBox(arg1: int?) -> null
    #
    # Sakura's F_PASTEBOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def PasteBox(arg1 = UNSET)
      @call.call('PasteBox', _pack(arg1))
    end

    # InsBoxText(arg1: string?) -> null
    #
    # Sakura's F_INSBOXTEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def InsBoxText(arg1 = UNSET)
      @call.call('InsBoxText', _pack(arg1))
    end

    # CopyLines() -> null
    #
    # Sakura's F_COPYLINES (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CopyLines
      @call.call('CopyLines', _pack())
    end

    # CopyLinesAsPassage() -> null
    #
    # Sakura's F_COPYLINESASPASSAGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CopyLinesAsPassage
      @call.call('CopyLinesAsPassage', _pack())
    end

    # CopyLinesWithLineNumber() -> null
    #
    # Sakura's F_COPYLINESWITHLINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    def CopyLinesWithLineNumber
      @call.call('CopyLinesWithLineNumber', _pack())
    end

    # CopyColorHtml() -> null
    #
    # Sakura's F_COPY_COLOR_HTML (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CopyColorHtml
      @call.call('CopyColorHtml', _pack())
    end

    # CopyColorHtmlWithLineNumber() -> null
    #
    # Sakura's F_COPY_COLOR_HTML_LINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    def CopyColorHtmlWithLineNumber
      @call.call('CopyColorHtmlWithLineNumber', _pack())
    end

    # CopyPath() -> null
    #
    # Sakura's F_COPYPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CopyPath
      @call.call('CopyPath', _pack())
    end

    # CopyDirPath() -> null
    #
    # Sakura's F_COPYDIRPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CopyDirPath
      @call.call('CopyDirPath', _pack())
    end

    # CopyFilename() -> null
    #
    # Sakura's F_COPYFNAME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CopyFilename
      @call.call('CopyFilename', _pack())
    end

    # CopyTag() -> null
    #
    # Sakura's F_COPYTAG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CopyTag
      @call.call('CopyTag', _pack())
    end

    # CopyKeyBindList() -> null
    #
    # Sakura's F_CREATEKEYBINDLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CopyKeyBindList
      @call.call('CopyKeyBindList', _pack())
    end

    # InsertDate() -> null
    #
    # Sakura's F_INS_DATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def InsertDate
      @call.call('InsertDate', _pack())
    end

    # InsertTime() -> null
    #
    # Sakura's F_INS_TIME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def InsertTime
      @call.call('InsertTime', _pack())
    end

    # CtrlCodeDialog() -> null
    #
    # Sakura's F_CTRL_CODE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CtrlCodeDialog
      @call.call('CtrlCodeDialog', _pack())
    end

    # CtrlCode(arg1: int?) -> null
    #
    # Sakura's F_CTRL_CODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CtrlCode(arg1 = UNSET)
      @call.call('CtrlCode', _pack(arg1))
    end

    # InsertFileUsedRecently() -> null
    #
    # Sakura's F_INS_FILE_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    def InsertFileUsedRecently
      @call.call('InsertFileUsedRecently', _pack())
    end

    # InsertFolderUsedRecently() -> null
    #
    # Sakura's F_INS_FOLDER_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    def InsertFolderUsedRecently
      @call.call('InsertFolderUsedRecently', _pack())
    end

    # ToLower() -> null
    #
    # Sakura's F_TOLOWER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ToLower
      @call.call('ToLower', _pack())
    end

    # ToUpper() -> null
    #
    # Sakura's F_TOUPPER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ToUpper
      @call.call('ToUpper', _pack())
    end

    # ToHankaku() -> null
    #
    # Sakura's F_TOHANKAKU (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ToHankaku
      @call.call('ToHankaku', _pack())
    end

    # ToHankata() -> null
    #
    # Sakura's F_TOHANKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ToHankata
      @call.call('ToHankata', _pack())
    end

    # ToZenEi() -> null
    #
    # Sakura's F_TOZENEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ToZenEi
      @call.call('ToZenEi', _pack())
    end

    # ToHanEi() -> null
    #
    # Sakura's F_TOHANEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ToHanEi
      @call.call('ToHanEi', _pack())
    end

    # ToZenKata() -> null
    #
    # Sakura's F_TOZENKAKUKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ToZenKata
      @call.call('ToZenKata', _pack())
    end

    # ToZenHira() -> null
    #
    # Sakura's F_TOZENKAKUHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ToZenHira
      @call.call('ToZenHira', _pack())
    end

    # HanKataToZenKata() -> null
    #
    # Sakura's F_HANKATATOZENKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HanKataToZenKata
      @call.call('HanKataToZenKata', _pack())
    end

    # HanKataToZenHira() -> null
    #
    # Sakura's F_HANKATATOZENHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HanKataToZenHira
      @call.call('HanKataToZenHira', _pack())
    end

    # TABToSPACE() -> null
    #
    # Sakura's F_TABTOSPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def TABToSPACE
      @call.call('TABToSPACE', _pack())
    end

    # SPACEToTAB() -> null
    #
    # Sakura's F_SPACETOTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SPACEToTAB
      @call.call('SPACEToTAB', _pack())
    end

    # AutoToSJIS() -> null
    #
    # Sakura's F_CODECNV_AUTO2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def AutoToSJIS
      @call.call('AutoToSJIS', _pack())
    end

    # JIStoSJIS() -> null
    #
    # Sakura's F_CODECNV_EMAIL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def JIStoSJIS
      @call.call('JIStoSJIS', _pack())
    end

    # EUCtoSJIS() -> null
    #
    # Sakura's F_CODECNV_EUC2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def EUCtoSJIS
      @call.call('EUCtoSJIS', _pack())
    end

    # CodeCnvUNICODEtoSJIS() -> null
    #
    # Sakura's F_CODECNV_UNICODE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
    # a key, the menu and a key macro run. The last arguments may be left out. A command that
    # this editor does not have yet answers the error -32001 (not implemented).
    def CodeCnvUNICODEtoSJIS
      @call.call('CodeCnvUNICODEtoSJIS', _pack())
    end

    # CodeCnvUNICODEBEtoSJIS() -> null
    #
    # Sakura's F_CODECNV_UNICODEBE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    def CodeCnvUNICODEBEtoSJIS
      @call.call('CodeCnvUNICODEBEtoSJIS', _pack())
    end

    # UTF8toSJIS() -> null
    #
    # Sakura's F_CODECNV_UTF82SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def UTF8toSJIS
      @call.call('UTF8toSJIS', _pack())
    end

    # UTF7toSJIS() -> null
    #
    # Sakura's F_CODECNV_UTF72SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def UTF7toSJIS
      @call.call('UTF7toSJIS', _pack())
    end

    # SJIStoJIS() -> null
    #
    # Sakura's F_CODECNV_SJIS2JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SJIStoJIS
      @call.call('SJIStoJIS', _pack())
    end

    # SJIStoEUC() -> null
    #
    # Sakura's F_CODECNV_SJIS2EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SJIStoEUC
      @call.call('SJIStoEUC', _pack())
    end

    # SJIStoUTF8() -> null
    #
    # Sakura's F_CODECNV_SJIS2UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SJIStoUTF8
      @call.call('SJIStoUTF8', _pack())
    end

    # SJIStoUTF7() -> null
    #
    # Sakura's F_CODECNV_SJIS2UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SJIStoUTF7
      @call.call('SJIStoUTF7', _pack())
    end

    # Base64Decode() -> null
    #
    # Sakura's F_BASE64DECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def Base64Decode
      @call.call('Base64Decode', _pack())
    end

    # Uudecode() -> null
    #
    # Sakura's F_UUDECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Uudecode
      @call.call('Uudecode', _pack())
    end

    # SearchDialog() -> null
    #
    # Sakura's F_SEARCH_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SearchDialog
      @call.call('SearchDialog', _pack())
    end

    # SearchNext(arg1: string?, arg2: int?) -> null
    #
    # Sakura's F_SEARCH_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SearchNext(arg1 = UNSET, arg2 = UNSET)
      @call.call('SearchNext', _pack(arg1, arg2))
    end

    # SearchPrev(arg1: string?, arg2: int?) -> null
    #
    # Sakura's F_SEARCH_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SearchPrev(arg1 = UNSET, arg2 = UNSET)
      @call.call('SearchPrev', _pack(arg1, arg2))
    end

    # ReplaceDialog() -> null
    #
    # Sakura's F_REPLACE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ReplaceDialog
      @call.call('ReplaceDialog', _pack())
    end

    # Replace(arg1: string?, arg2: string?, arg3: int?) -> null
    #
    # Sakura's F_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Replace(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('Replace', _pack(arg1, arg2, arg3))
    end

    # ReplaceAll(arg1: string?, arg2: string?, arg3: int?) -> null
    #
    # Sakura's F_REPLACE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ReplaceAll(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('ReplaceAll', _pack(arg1, arg2, arg3))
    end

    # SearchClearMark() -> null
    #
    # Sakura's F_SEARCH_CLEARMARK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SearchClearMark
      @call.call('SearchClearMark', _pack())
    end

    # SearchStartPos() -> null
    #
    # Sakura's F_JUMP_SRCHSTARTPOS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SearchStartPos
      @call.call('SearchStartPos', _pack())
    end

    # Grep(arg1: string?, arg2: string?, arg3: string?, arg4: int?, arg5: int?) -> null
    #
    # Sakura's F_GREP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Grep(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET, arg4 = UNSET, arg5 = UNSET)
      @call.call('Grep', _pack(arg1, arg2, arg3, arg4, arg5))
    end

    # GrepReplace(arg1: string?, arg2: string?, arg3: string?, arg4: string?, arg5: int?, arg6: int?) -> null
    #
    # Sakura's F_GREP_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def GrepReplace(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET, arg4 = UNSET, arg5 = UNSET, arg6 = UNSET)
      @call.call('GrepReplace', _pack(arg1, arg2, arg3, arg4, arg5, arg6))
    end

    # Outline(arg1: int?) -> null
    #
    # Sakura's F_OUTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Outline(arg1 = UNSET)
      @call.call('Outline', _pack(arg1))
    end

    # TagJump() -> null
    #
    # Sakura's F_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def TagJump
      @call.call('TagJump', _pack())
    end

    # TagJumpBack() -> null
    #
    # Sakura's F_TAGJUMPBACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TagJumpBack
      @call.call('TagJumpBack', _pack())
    end

    # TagMake() -> null
    #
    # Sakura's F_TAGS_MAKE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def TagMake
      @call.call('TagMake', _pack())
    end

    # DirectTagJump() -> null
    #
    # Sakura's F_DIRECT_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def DirectTagJump
      @call.call('DirectTagJump', _pack())
    end

    # KeywordTagJump(arg1: string?) -> null
    #
    # Sakura's F_TAGJUMP_KEYWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def KeywordTagJump(arg1 = UNSET)
      @call.call('KeywordTagJump', _pack(arg1))
    end

    # Compare() -> null
    #
    # Sakura's F_COMPARE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def Compare
      @call.call('Compare', _pack())
    end

    # DiffDialog() -> null
    #
    # Sakura's F_DIFF_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def DiffDialog
      @call.call('DiffDialog', _pack())
    end

    # Diff(arg1: string?, arg2: int?) -> null
    #
    # Sakura's F_DIFF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Diff(arg1 = UNSET, arg2 = UNSET)
      @call.call('Diff', _pack(arg1, arg2))
    end

    # DiffNext() -> null
    #
    # Sakura's F_DIFF_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def DiffNext
      @call.call('DiffNext', _pack())
    end

    # DiffPrev() -> null
    #
    # Sakura's F_DIFF_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def DiffPrev
      @call.call('DiffPrev', _pack())
    end

    # DiffReset() -> null
    #
    # Sakura's F_DIFF_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def DiffReset
      @call.call('DiffReset', _pack())
    end

    # BracketPair() -> null
    #
    # Sakura's F_BRACKETPAIR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def BracketPair
      @call.call('BracketPair', _pack())
    end

    # BookmarkSet() -> null
    #
    # Sakura's F_BOOKMARK_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def BookmarkSet
      @call.call('BookmarkSet', _pack())
    end

    # BookmarkNext() -> null
    #
    # Sakura's F_BOOKMARK_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def BookmarkNext
      @call.call('BookmarkNext', _pack())
    end

    # BookmarkPrev() -> null
    #
    # Sakura's F_BOOKMARK_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def BookmarkPrev
      @call.call('BookmarkPrev', _pack())
    end

    # BookmarkReset() -> null
    #
    # Sakura's F_BOOKMARK_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def BookmarkReset
      @call.call('BookmarkReset', _pack())
    end

    # BookmarkView() -> null
    #
    # Sakura's F_BOOKMARK_VIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def BookmarkView
      @call.call('BookmarkView', _pack())
    end

    # BookmarkPattern(arg1: string?, arg2: int?) -> null
    #
    # Sakura's F_BOOKMARK_PATTERN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def BookmarkPattern(arg1 = UNSET, arg2 = UNSET)
      @call.call('BookmarkPattern', _pack(arg1, arg2))
    end

    # FuncListNext() -> null
    #
    # Sakura's F_FUNCLIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FuncListNext
      @call.call('FuncListNext', _pack())
    end

    # FuncListPrev() -> null
    #
    # Sakura's F_FUNCLIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def FuncListPrev
      @call.call('FuncListPrev', _pack())
    end

    # ChgmodINS() -> null
    #
    # Sakura's F_CHGMOD_INS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ChgmodINS
      @call.call('ChgmodINS', _pack())
    end

    # ChgCharSet(arg1: int?, arg2: int?) -> null
    #
    # Sakura's F_CHG_CHARSET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ChgCharSet(arg1 = UNSET, arg2 = UNSET)
      @call.call('ChgCharSet', _pack(arg1, arg2))
    end

    # ChgmodEOL(arg1: int?) -> null
    #
    # Sakura's F_CHGMOD_EOL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ChgmodEOL(arg1 = UNSET)
      @call.call('ChgmodEOL', _pack(arg1))
    end

    # CancelMode() -> null
    #
    # Sakura's F_CANCEL_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CancelMode
      @call.call('CancelMode', _pack())
    end

    # ExecExternalMacro(arg1: string?, arg2: string?) -> null
    #
    # Sakura's F_EXECEXTMACRO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ExecExternalMacro(arg1 = UNSET, arg2 = UNSET)
      @call.call('ExecExternalMacro', _pack(arg1, arg2))
    end

    # ShowToolbar() -> null
    #
    # Sakura's F_SHOWTOOLBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ShowToolbar
      @call.call('ShowToolbar', _pack())
    end

    # ShowFunckey() -> null
    #
    # Sakura's F_SHOWFUNCKEY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ShowFunckey
      @call.call('ShowFunckey', _pack())
    end

    # ShowTab() -> null
    #
    # Sakura's F_SHOWTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ShowTab
      @call.call('ShowTab', _pack())
    end

    # ShowStatusbar() -> null
    #
    # Sakura's F_SHOWSTATUSBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ShowStatusbar
      @call.call('ShowStatusbar', _pack())
    end

    # ShowMiniMap() -> null
    #
    # Sakura's F_SHOWMINIMAP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ShowMiniMap
      @call.call('ShowMiniMap', _pack())
    end

    # TypeList() -> null
    #
    # Sakura's F_TYPE_LIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def TypeList
      @call.call('TypeList', _pack())
    end

    # ChangeType(arg1: int?) -> null
    #
    # Sakura's F_CHANGETYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ChangeType(arg1 = UNSET)
      @call.call('ChangeType', _pack(arg1))
    end

    # OptionType() -> null
    #
    # Sakura's F_OPTION_TYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def OptionType
      @call.call('OptionType', _pack())
    end

    # OptionCommon() -> null
    #
    # Sakura's F_OPTION (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def OptionCommon
      @call.call('OptionCommon', _pack())
    end

    # SelectFont() -> null
    #
    # Sakura's F_FONT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def SelectFont
      @call.call('SelectFont', _pack())
    end

    # SetFontSize(arg1: int?, arg2: int?, arg3: int?) -> null
    #
    # Sakura's F_SETFONTSIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SetFontSize(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('SetFontSize', _pack(arg1, arg2, arg3))
    end

    # WrapWindowWidth() -> null
    #
    # Sakura's F_WRAPWINDOWWIDTH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WrapWindowWidth
      @call.call('WrapWindowWidth', _pack())
    end

    # OptionFavorite() -> null
    #
    # Sakura's F_FAVORITE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def OptionFavorite
      @call.call('OptionFavorite', _pack())
    end

    # SetMsgQuoteStr(arg1: string?) -> null
    #
    # Sakura's F_SET_QUOTESTRING (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SetMsgQuoteStr(arg1 = UNSET)
      @call.call('SetMsgQuoteStr', _pack(arg1))
    end

    # TextWrapMethod(arg1: int?) -> null
    #
    # Sakura's F_TEXTWRAPMETHOD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TextWrapMethod(arg1 = UNSET)
      @call.call('TextWrapMethod', _pack(arg1))
    end

    # SelectCountMode(arg1: int?) -> null
    #
    # Sakura's F_SELECT_COUNT_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SelectCountMode(arg1 = UNSET)
      @call.call('SelectCountMode', _pack(arg1))
    end

    # ExecCommand(arg1: string?, arg2: int?, arg3: string?) -> null
    #
    # Sakura's F_EXECMD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ExecCommand(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('ExecCommand', _pack(arg1, arg2, arg3))
    end

    # ExecCommandDialog() -> null
    #
    # Sakura's F_EXECMD_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ExecCommandDialog
      @call.call('ExecCommandDialog', _pack())
    end

    # RMenu() -> null
    #
    # Sakura's F_MENU_RBUTTON (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def RMenu
      @call.call('RMenu', _pack())
    end

    # CustMenu1() -> null
    #
    # Sakura's F_CUSTMENU_1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu1
      @call.call('CustMenu1', _pack())
    end

    # CustMenu2() -> null
    #
    # Sakura's F_CUSTMENU_2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu2
      @call.call('CustMenu2', _pack())
    end

    # CustMenu3() -> null
    #
    # Sakura's F_CUSTMENU_3 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu3
      @call.call('CustMenu3', _pack())
    end

    # CustMenu4() -> null
    #
    # Sakura's F_CUSTMENU_4 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu4
      @call.call('CustMenu4', _pack())
    end

    # CustMenu5() -> null
    #
    # Sakura's F_CUSTMENU_5 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu5
      @call.call('CustMenu5', _pack())
    end

    # CustMenu6() -> null
    #
    # Sakura's F_CUSTMENU_6 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu6
      @call.call('CustMenu6', _pack())
    end

    # CustMenu7() -> null
    #
    # Sakura's F_CUSTMENU_7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu7
      @call.call('CustMenu7', _pack())
    end

    # CustMenu8() -> null
    #
    # Sakura's F_CUSTMENU_8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu8
      @call.call('CustMenu8', _pack())
    end

    # CustMenu9() -> null
    #
    # Sakura's F_CUSTMENU_9 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CustMenu9
      @call.call('CustMenu9', _pack())
    end

    # CustMenu10() -> null
    #
    # Sakura's F_CUSTMENU_10 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu10
      @call.call('CustMenu10', _pack())
    end

    # CustMenu11() -> null
    #
    # Sakura's F_CUSTMENU_11 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu11
      @call.call('CustMenu11', _pack())
    end

    # CustMenu12() -> null
    #
    # Sakura's F_CUSTMENU_12 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu12
      @call.call('CustMenu12', _pack())
    end

    # CustMenu13() -> null
    #
    # Sakura's F_CUSTMENU_13 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu13
      @call.call('CustMenu13', _pack())
    end

    # CustMenu14() -> null
    #
    # Sakura's F_CUSTMENU_14 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu14
      @call.call('CustMenu14', _pack())
    end

    # CustMenu15() -> null
    #
    # Sakura's F_CUSTMENU_15 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu15
      @call.call('CustMenu15', _pack())
    end

    # CustMenu16() -> null
    #
    # Sakura's F_CUSTMENU_16 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu16
      @call.call('CustMenu16', _pack())
    end

    # CustMenu17() -> null
    #
    # Sakura's F_CUSTMENU_17 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu17
      @call.call('CustMenu17', _pack())
    end

    # CustMenu18() -> null
    #
    # Sakura's F_CUSTMENU_18 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu18
      @call.call('CustMenu18', _pack())
    end

    # CustMenu19() -> null
    #
    # Sakura's F_CUSTMENU_19 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu19
      @call.call('CustMenu19', _pack())
    end

    # CustMenu20() -> null
    #
    # Sakura's F_CUSTMENU_20 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu20
      @call.call('CustMenu20', _pack())
    end

    # CustMenu21() -> null
    #
    # Sakura's F_CUSTMENU_21 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu21
      @call.call('CustMenu21', _pack())
    end

    # CustMenu22() -> null
    #
    # Sakura's F_CUSTMENU_22 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu22
      @call.call('CustMenu22', _pack())
    end

    # CustMenu23() -> null
    #
    # Sakura's F_CUSTMENU_23 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu23
      @call.call('CustMenu23', _pack())
    end

    # CustMenu24() -> null
    #
    # Sakura's F_CUSTMENU_24 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CustMenu24
      @call.call('CustMenu24', _pack())
    end

    # SplitWinV() -> null
    #
    # Sakura's F_SPLIT_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SplitWinV
      @call.call('SplitWinV', _pack())
    end

    # SplitWinH() -> null
    #
    # Sakura's F_SPLIT_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SplitWinH
      @call.call('SplitWinH', _pack())
    end

    # SplitWinVH() -> null
    #
    # Sakura's F_SPLIT_VH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SplitWinVH
      @call.call('SplitWinVH', _pack())
    end

    # WinMaximize() -> null
    #
    # Sakura's F_WINMAXIMIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WinMaximize
      @call.call('WinMaximize', _pack())
    end

    # WinMinimize() -> null
    #
    # Sakura's F_WINMINIMIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WinMinimize
      @call.call('WinMinimize', _pack())
    end

    # WinRestore() -> null
    #
    # Sakura's F_WINRESTORE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WinRestore
      @call.call('WinRestore', _pack())
    end

    # WinClose() -> null
    #
    # Sakura's F_WINCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WinClose
      @call.call('WinClose', _pack())
    end

    # WinCloseAll() -> null
    #
    # Sakura's F_WIN_CLOSEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def WinCloseAll
      @call.call('WinCloseAll', _pack())
    end

    # CascadeWin() -> null
    #
    # Sakura's F_CASCADE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def CascadeWin
      @call.call('CascadeWin', _pack())
    end

    # TileWinV() -> null
    #
    # Sakura's F_TILE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def TileWinV
      @call.call('TileWinV', _pack())
    end

    # TileWinH() -> null
    #
    # Sakura's F_TILE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def TileWinH
      @call.call('TileWinH', _pack())
    end

    # NextWindow() -> null
    #
    # Sakura's F_NEXTWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def NextWindow
      @call.call('NextWindow', _pack())
    end

    # PrevWindow() -> null
    #
    # Sakura's F_PREVWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def PrevWindow
      @call.call('PrevWindow', _pack())
    end

    # WindowList() -> null
    #
    # Sakura's F_WINLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WindowList
      @call.call('WindowList', _pack())
    end

    # MaximizeV() -> null
    #
    # Sakura's F_MAXIMIZE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def MaximizeV
      @call.call('MaximizeV', _pack())
    end

    # MaximizeH() -> null
    #
    # Sakura's F_MAXIMIZE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def MaximizeH
      @call.call('MaximizeH', _pack())
    end

    # MinimizeAll() -> null
    #
    # Sakura's F_MINIMIZE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def MinimizeAll
      @call.call('MinimizeAll', _pack())
    end

    # ReDraw() -> null
    #
    # Sakura's F_REDRAW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ReDraw
      @call.call('ReDraw', _pack())
    end

    # ActivateWinOutput() -> null
    #
    # Sakura's F_WIN_OUTPUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ActivateWinOutput
      @call.call('ActivateWinOutput', _pack())
    end

    # TraceOut(arg1: string?, arg2: int?) -> null
    #
    # Sakura's F_TRACEOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def TraceOut(arg1 = UNSET, arg2 = UNSET)
      @call.call('TraceOut', _pack(arg1, arg2))
    end

    # WindowTopMost(arg1: int?) -> null
    #
    # Sakura's F_TOPMOST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def WindowTopMost(arg1 = UNSET)
      @call.call('WindowTopMost', _pack(arg1))
    end

    # GroupClose() -> null
    #
    # Sakura's F_GROUPCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def GroupClose
      @call.call('GroupClose', _pack())
    end

    # NextGroup() -> null
    #
    # Sakura's F_NEXTGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def NextGroup
      @call.call('NextGroup', _pack())
    end

    # PrevGroup() -> null
    #
    # Sakura's F_PREVGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def PrevGroup
      @call.call('PrevGroup', _pack())
    end

    # TabMoveRight() -> null
    #
    # Sakura's F_TAB_MOVERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TabMoveRight
      @call.call('TabMoveRight', _pack())
    end

    # TabMoveLeft() -> null
    #
    # Sakura's F_TAB_MOVELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TabMoveLeft
      @call.call('TabMoveLeft', _pack())
    end

    # TabSeparate() -> null
    #
    # Sakura's F_TAB_SEPARATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TabSeparate
      @call.call('TabSeparate', _pack())
    end

    # TabJointNext() -> null
    #
    # Sakura's F_TAB_JOINTNEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TabJointNext
      @call.call('TabJointNext', _pack())
    end

    # TabJointPrev() -> null
    #
    # Sakura's F_TAB_JOINTPREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TabJointPrev
      @call.call('TabJointPrev', _pack())
    end

    # TabCloseOther() -> null
    #
    # Sakura's F_TAB_CLOSEOTHER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TabCloseOther
      @call.call('TabCloseOther', _pack())
    end

    # TabCloseLeft() -> null
    #
    # Sakura's F_TAB_CLOSELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TabCloseLeft
      @call.call('TabCloseLeft', _pack())
    end

    # TabCloseRight() -> null
    #
    # Sakura's F_TAB_CLOSERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def TabCloseRight
      @call.call('TabCloseRight', _pack())
    end

    # Complete() -> null
    #
    # Sakura's F_HOKAN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def Complete
      @call.call('Complete', _pack())
    end

    # ToggleKeyHelpSearch(arg1: int?) -> null
    #
    # Sakura's F_TOGGLE_KEY_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ToggleKeyHelpSearch(arg1 = UNSET)
      @call.call('ToggleKeyHelpSearch', _pack(arg1))
    end

    # HelpContents() -> null
    #
    # Sakura's F_HELP_CONTENTS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HelpContents
      @call.call('HelpContents', _pack())
    end

    # HelpSearch() -> null
    #
    # Sakura's F_HELP_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def HelpSearch
      @call.call('HelpSearch', _pack())
    end

    # CommandList() -> null
    #
    # Sakura's F_MENU_ALLFUNC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CommandList
      @call.call('CommandList', _pack())
    end

    # ExtHelp1() -> null
    #
    # Sakura's F_EXTHELP1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def ExtHelp1
      @call.call('ExtHelp1', _pack())
    end

    # ExtHtmlHelp(arg1: string?, arg2: string?) -> null
    #
    # Sakura's F_EXTHTMLHELP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ExtHtmlHelp(arg1 = UNSET, arg2 = UNSET)
      @call.call('ExtHtmlHelp', _pack(arg1, arg2))
    end

    # About() -> null
    #
    # Sakura's F_ABOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
    # and a key macro run. The last arguments may be left out. A command that this editor does
    # not have yet answers the error -32001 (not implemented).
    def About
      @call.call('About', _pack())
    end

    # StatusMsg(arg1: string?, arg2: int?) -> null
    #
    # Sakura's F_STATUSMSG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def StatusMsg(arg1 = UNSET, arg2 = UNSET)
      @call.call('StatusMsg', _pack(arg1, arg2))
    end

    # MsgBeep(arg1: int?) -> null
    #
    # Sakura's F_MSGBEEP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def MsgBeep(arg1 = UNSET)
      @call.call('MsgBeep', _pack(arg1))
    end

    # CommitUndoBuffer() -> null
    #
    # Sakura's F_COMMITUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def CommitUndoBuffer
      @call.call('CommitUndoBuffer', _pack())
    end

    # AddRefUndoBuffer() -> null
    #
    # Sakura's F_ADDREFUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
    # key, the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def AddRefUndoBuffer
      @call.call('AddRefUndoBuffer', _pack())
    end

    # SetUndoBuffer() -> null
    #
    # Sakura's F_SETUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SetUndoBuffer
      @call.call('SetUndoBuffer', _pack())
    end

    # AppendUndoBufferCursor() -> null
    #
    # Sakura's F_APPENDUNDOBUFFERCURSOR (sakura_core/macro/CSMacroMgr.cpp). Runs the command
    # that a key, the menu and a key macro run. The last arguments may be left out. A command
    # that this editor does not have yet answers the error -32001 (not implemented).
    def AppendUndoBufferCursor
      @call.call('AppendUndoBufferCursor', _pack())
    end

    # ClipboardEmpty() -> null
    #
    # Sakura's F_CLIPBOARDEMPTY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def ClipboardEmpty
      @call.call('ClipboardEmpty', _pack())
    end

    # SetViewTop(arg1: int?) -> null
    #
    # Sakura's F_SETVIEWTOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
    # menu and a key macro run. The last arguments may be left out. A command that this editor
    # does not have yet answers the error -32001 (not implemented).
    def SetViewTop(arg1 = UNSET)
      @call.call('SetViewTop', _pack(arg1))
    end

    # SetViewLeft(arg1: int?) -> null
    #
    # Sakura's F_SETVIEWLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
    # the menu and a key macro run. The last arguments may be left out. A command that this
    # editor does not have yet answers the error -32001 (not implemented).
    def SetViewLeft(arg1 = UNSET)
      @call.call('SetViewLeft', _pack(arg1))
    end

    # InputBox(arg1: string?, arg2: string?, arg3: int?) -> string
    #
    # Sakura's F_INPUTBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
    # editor answers the error -32001 (not implemented) while it cannot answer it.
    def InputBox(arg1 = UNSET, arg2 = UNSET, arg3 = UNSET)
      @call.call('InputBox', _pack(arg1, arg2, arg3))
    end

    # CreateMenu(arg1: int?, arg2: string?) -> int
    #
    # Sakura's F_CREATEMENU (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
    # editor answers the error -32001 (not implemented) while it cannot answer it.
    def CreateMenu(arg1 = UNSET, arg2 = UNSET)
      @call.call('CreateMenu', _pack(arg1, arg2))
    end

    # ChangeInsertSpace(arg1: int?) -> int
    #
    # Sakura's F_CHGINSSPACE (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
    # editor answers the error -32001 (not implemented) while it cannot answer it.
    def ChangeInsertSpace(arg1 = UNSET)
      @call.call('ChangeInsertSpace', _pack(arg1))
    end

    # Any other name that starts like a Sakura macro name (an upper case letter or a digit) is
    # sent to the editor as it is. The editor answers a name it does not know with an error
    # (EditorError).
    def method_missing(name, *args)
      return super unless name.to_s.match?(/\A[0-9A-Z]/)

      @call.call(name.to_s, args)
    end

    def respond_to_missing?(name, include_private = false)
      name.to_s.match?(/\A[0-9A-Z]/) || super
    end

    private

    # A trailing UNSET is a missing argument, not a nil.
    def _pack(*args)
      args.pop while !args.empty? && args.last.equal?(UNSET)
      args
    end
  end

  def self.install(call)
    EditorObject.new(call)
  end
end
