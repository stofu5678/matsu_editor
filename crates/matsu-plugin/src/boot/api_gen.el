;;; api_gen.el --- GENERATED from crates/matsu-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: MATSU_UPDATE_GENERATED=1 cargo test -p matsu-plugin generated_files  -*- lexical-binding: t; -*-

(defun editor-get-line-count (&rest args)
  "GetLineCount(reserved: int) -> int

The number of logical lines (lines separated by line breaks). The text after the
last line break is not a line unless it is not empty. The argument is reserved
and must be 0."
  (apply #'editor-call "GetLineCount" args))

(defun editor-get-line-str (&rest args)
  "GetLineStr(line: int) -> string

A logical line with its line break (\"\\n\"; the last line has none unless the text
ends with one). 0 is the line of the cursor, 1 is the first line. Past the last
line it is an empty string."
  (apply #'editor-call "GetLineStr" args))

(defun editor-ins-text (&rest args)
  "InsText(text: string) -> null

Inserts the text at the cursor, replacing the selection. The cursor ends after
the text."
  (apply #'editor-call "InsText" args))

(defun editor-add-tail (&rest args)
  "AddTail(text: string) -> null

Moves to the end of the file, drops the selection, and inserts the text there."
  (apply #'editor-call "AddTail" args))

(defun editor-go-file-top (&rest args)
  "GoFileTop() -> null

Moves the cursor to the start of the file and drops the selection."
  (apply #'editor-call "GoFileTop" args))

(defun editor-go-file-end (&rest args)
  "GoFileEnd() -> null

Moves the cursor to the end of the file and drops the selection."
  (apply #'editor-call "GoFileEnd" args))

(defun editor-up (&rest args)
  "Up() -> null

Moves the cursor up one line and drops the selection."
  (apply #'editor-call "Up" args))

(defun editor-down (&rest args)
  "Down() -> null

Moves the cursor down one line and drops the selection."
  (apply #'editor-call "Down" args))

(defun editor-select-all (&rest args)
  "SelectAll() -> null

Selects the whole text. The cursor goes to the end."
  (apply #'editor-call "SelectAll" args))

(defun editor-get-selected-string (&rest args)
  "GetSelectedString(reserved: int) -> string

The selected text with its line breaks (\"\\n\"), or an empty string when nothing
is selected. The argument is reserved and ignored."
  (apply #'editor-call "GetSelectedString" args))

(defun editor-is-text-selected (&rest args)
  "IsTextSelected() -> int

0 when nothing is selected, 1 when text is selected, 2 when a box selection is
selected."
  (apply #'editor-call "IsTextSelected" args))

(defun editor-move-cursor (&rest args)
  "MoveCursor(line: int, column: int, option: int) -> null

Moves the cursor to a line and a column, both counted from 1. The column counts
UTF-16 code units. Option: 0 does not select, 1 selects from the old position, 2
selects a box with the old and the new position as corners (2 wins over 1). Past
the end of the text it goes to the end of the text."
  (apply #'editor-call "MoveCursor" args))

(defun editor-jump (&rest args)
  "Jump(line: int, flag: int?) -> null

Moves to the start of a line, counted from 1 in logical lines. Lines before 1
and after the last one are limited to the first and the last line. Flag bit 0
(1, the default) is the line break unit, which is the only unit supported yet;
bit 1 (PL/SQL) is not supported."
  (apply #'editor-call "Jump" args))

(defun editor-get-filename (&rest args)
  "GetFilename() -> string

The full path of the file, or an empty string when the text has no file."
  (apply #'editor-call "GetFilename" args))

(defun editor-get-select-line-from (&rest args)
  "GetSelectLineFrom() -> int

The line where the selection starts, counted from 1 in layout lines (wrapped
rows count as lines). 0 when nothing is selected."
  (apply #'editor-call "GetSelectLineFrom" args))

(defun editor-get-select-colm-from (&rest args)
  "GetSelectColmFrom() -> int

The column where the selection starts, counted from 1 in layout columns (cells:
a full-width character is 2, a tab goes to the tab stop). 0 when nothing is
selected. Same as GetSelectColumnFrom."
  (apply #'editor-call "GetSelectColmFrom" args))

(defun editor-get-select-column-from (&rest args)
  "GetSelectColumnFrom() -> int

The column where the selection starts, counted from 1 in layout columns (cells:
a full-width character is 2, a tab goes to the tab stop). 0 when nothing is
selected."
  (apply #'editor-call "GetSelectColumnFrom" args))

(defun editor-get-select-line-to (&rest args)
  "GetSelectLineTo() -> int

The line where the selection ends, counted from 1 in layout lines. 0 when
nothing is selected."
  (apply #'editor-call "GetSelectLineTo" args))

(defun editor-get-select-colm-to (&rest args)
  "GetSelectColmTo() -> int

The column where the selection ends, counted from 1 in layout columns. 0 when
nothing is selected. Same as GetSelectColumnTo."
  (apply #'editor-call "GetSelectColmTo" args))

(defun editor-get-select-column-to (&rest args)
  "GetSelectColumnTo() -> int

The column where the selection ends, counted from 1 in layout columns. 0 when
nothing is selected."
  (apply #'editor-call "GetSelectColumnTo" args))

(defun editor-is-ins-mode (&rest args)
  "IsInsMode() -> int

1 in insert mode, 0 in overwrite mode."
  (apply #'editor-call "IsInsMode" args))

(defun editor-get-char-code (&rest args)
  "GetCharCode() -> int

The encoding of the file: 0 Shift_JIS, 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, 5
UTF-7, 6 UTF-16 BE, 7 CESU-8, 8 Latin1."
  (apply #'editor-call "GetCharCode" args))

(defun editor-get-line-code (&rest args)
  "GetLineCode() -> int

The line ending that new line breaks get: 0 CR LF, 1 CR, 2 LF (3 NEL, 4 LS, 5
PS)."
  (apply #'editor-call "GetLineCode" args))

(defun editor-is-possible-undo (&rest args)
  "IsPossibleUndo() -> int

1 when there is something to undo, else 0."
  (apply #'editor-call "IsPossibleUndo" args))

(defun editor-is-possible-redo (&rest args)
  "IsPossibleRedo() -> int

1 when there is something to redo, else 0."
  (apply #'editor-call "IsPossibleRedo" args))

(defun editor-get-line-attribute (&rest args)
  "GetLineAttribute(line: int, type: int) -> int

An attribute of a logical line (0 is the line of the cursor, 1 the first line).
Type 0 is 1 when the line changed since the last save, type 1 the number of the
undo step that last touched it, type 2 the bookmark (1 or 0). Types 3 (diff
mark) and 4 (function list mark) answer the error -32001 until the editor has
them. A negative line, a line past the end and any other type are errors."
  (apply #'editor-call "GetLineAttribute" args))

(defun editor-is-text-selecting-lock (&rest args)
  "IsTextSelectingLock() -> int

0 when the selection is not locked, 1 when it is locked (plain moves extend the
selection), 2 when it is locked and a box selection."
  (apply #'editor-call "IsTextSelectingLock" args))

(defun editor-get-save-filename (&rest args)
  "GetSaveFilename() -> string

The full path that a save writes to, or an empty string when the text has no
file. The same as GetFilename, except in the macro that runs before a save (the
registered macro of U+4FDDU+5B58U+524D), where it is the file being saved."
  (apply #'editor-call "GetSaveFilename" args))

(defun editor-change-insert-space (&rest args)
  "ChangeInsertSpace(value: int) -> int

Whether the Tab key inserts spaces instead of a tab character, before the call
(1 or 0). 0 turns it off, 1 turns it on for this window (not saved); any other
value only reads. Only the low 16 bits of the value count, as in Sakura."
  (apply #'editor-call "ChangeInsertSpace" args))

(defun editor-layout-to-logic-line-num (&rest args)
  "LayoutToLogicLineNum(layoutLine: int) -> int

The logical line (up to a line break) that a layout line (a row on the screen)
belongs to, both counted from 1. The line after the last layout line gives the
line after the last logical line. Any other line outside the text is an error."
  (apply #'editor-call "LayoutToLogicLineNum" args))

(defun editor-logic-to-layout-line-num (&rest args)
  "LogicToLayoutLineNum(logicLine: int, logicIndex: int) -> int

The layout line (a row on the screen) of a position in a logical line: the line
and the index in it (UTF-16 code units), both counted from 1. A line past the
end gives the line after the last layout line."
  (apply #'editor-call "LogicToLayoutLineNum" args))

(defun editor-line-column-to-index (&rest args)
  "LineColumnToIndex(layoutLine: int, layoutColumn: int) -> int

The index (UTF-16 code units, from 1) in the logical line of a layout line and a
layout column (cells), both from 1. A column past the text gives the end of the
text of the row."
  (apply #'editor-call "LineColumnToIndex" args))

(defun editor-line-index-to-column (&rest args)
  "LineIndexToColumn(logicLine: int, logicIndex: int) -> int

The layout column (cells, from 1) of a position in a logical line: the line and
the index in it (UTF-16 code units), both from 1. Tabs and full-width characters
take their width."
  (apply #'editor-call "LineIndexToColumn" args))

(defun editor-get-str-width (&rest args)
  "GetStrWidth(text: string, column: int?) -> int

The width of a string in cells, starting at layout column `column` (from 1,
default 1; it matters for tabs). A CR LF at the end counts as the CR alone."
  (apply #'editor-call "GetStrWidth" args))

(defun editor-get-str-layout-length (&rest args)
  "GetStrLayoutLength(text: string, column: int?) -> int

The width of a string as a line of the document in cells, starting at layout
column `column` (from 1, default 1). A line ending at the end takes one cell."
  (apply #'editor-call "GetStrLayoutLength" args))

(defun editor-get-default-char-length (&rest args)
  "GetDefaultCharLength() -> int

The layout width of a half-width character. This editor lays out in cells, so it
is 1."
  (apply #'editor-call "GetDefaultCharLength" args))

(defun editor-get-view-lines (&rest args)
  "GetViewLines() -> int

How many rows the text area shows (at least 1)."
  (apply #'editor-call "GetViewLines" args))

(defun editor-get-view-columns (&rest args)
  "GetViewColumns() -> int

How many columns the text area shows (at least 1)."
  (apply #'editor-call "GetViewColumns" args))

(defun editor-get-view-top (&rest args)
  "GetViewTop() -> int

The layout line at the top of the text area, counted from 1."
  (apply #'editor-call "GetViewTop" args))

(defun editor-change-tab-width (&rest args)
  "ChangeTabWidth(width: int) -> int

The tab width before the call. A width from 1 to 64 that differs from it becomes
the tab width of this window (not saved). Any other value, such as 0, only
reads."
  (apply #'editor-call "ChangeTabWidth" args))

(defun editor-change-wrap-colm (&rest args)
  "ChangeWrapColm(column: int) -> int

The wrap column before the call. A column from 10 to 10240 makes this window
wrap at that column (not saved). Any other value only reads. Same as
ChangeWrapColumn."
  (apply #'editor-call "ChangeWrapColm" args))

(defun editor-change-wrap-column (&rest args)
  "ChangeWrapColumn(column: int) -> int

The wrap column before the call. A column from 10 to 10240 makes this window
wrap at that column (not saved). Any other value only reads."
  (apply #'editor-call "ChangeWrapColumn" args))

(defun editor-get-clipboard (&rest args)
  "GetClipboard(option: int?) -> string

The text on the clipboard, or an empty string when there is none. The line
endings are those of the source. The option is reserved."
  (apply #'editor-call "GetClipboard" args))

(defun editor-set-clipboard (&rest args)
  "SetClipboard(option: int?, text: string?) -> int

Puts the text on the clipboard. Option bit 0 marks a box copy (a paste puts it
in as a box), bit 1 a row copy (a paste puts it in front of the row); with both,
the box wins. Returns 1 when it worked, 0 when not."
  (apply #'editor-call "SetClipboard" args))

(defun editor-is-include-clipboard-format (&rest args)
  "IsIncludeClipboardFormat(format: string) -> int

1 when the clipboard has the format. Only text is known: CF_TEXT, CF_OEMTEXT and
CF_UNICODETEXT (or 1, 7, 13) are 1 when the clipboard has text; every other
format is 0."
  (apply #'editor-call "IsIncludeClipboardFormat" args))

(defun editor-get-clipboard-by-format (&rest args)
  "GetClipboardByFormat(format: string, mode: int, endMode: int?) -> string

The text of the clipboard in a text format (CF_TEXT, CF_OEMTEXT,
CF_UNICODETEXT), with the line endings of the document. Mode -2 is the usual way
and 3 is UTF-16 (the text itself); the other modes and formats are errors
(-32001). The end mode is not used."
  (apply #'editor-call "GetClipboardByFormat" args))

(defun editor-set-clipboard-by-format (&rest args)
  "SetClipboardByFormat(text: string, format: string, mode: int, endMode: int?) -> int

Puts the text on the clipboard as text. The formats and modes are those of
GetClipboardByFormat. Returns 1 when it worked."
  (apply #'editor-call "SetClipboardByFormat" args))

(defun editor-get-cookie (&rest args)
  "GetCookie(scope: string, name: string) -> string

The value of a cookie, or an empty string when there is none. The scope is
\"window\" or \"document\"."
  (apply #'editor-call "GetCookie" args))

(defun editor-get-cookie-default (&rest args)
  "GetCookieDefault(scope: string, name: string, defaultValue: string) -> string

The value of a cookie, or the default when there is none (an unknown scope still
gives an empty string). The scope is \"window\" or \"document\"."
  (apply #'editor-call "GetCookieDefault" args))

(defun editor-set-cookie (&rest args)
  "SetCookie(scope: string, name: string, value: string) -> int

Sets a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a name
with a character that a name cannot have (Sakura lets in digits, letters and
every character from \"_\" up in the character table). The scope is \"window\" or
\"document\"."
  (apply #'editor-call "SetCookie" args))

(defun editor-delete-cookie (&rest args)
  "DeleteCookie(scope: string, name: string) -> int

Deletes a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a bad
name, 5 when there is no such cookie. The scope is \"window\" or \"document\"."
  (apply #'editor-call "DeleteCookie" args))

(defun editor-get-cookie-names (&rest args)
  "GetCookieNames(scope: string) -> string

The names of the cookies of a scope, joined with commas, in the order of the
names (UTF-16 code units). An unknown scope gives an empty string. The scope is
\"window\" or \"document\"."
  (apply #'editor-call "GetCookieNames" args))

(defun editor-compare-version (&rest args)
  "CompareVersion(versionA: string, versionB: string) -> int

Compares two version strings: 0 when equal, positive when A is newer, negative
when B is. Four parts at most; a part has two digits at most; \"alpha\", \"beta\",
\"rc\" and \"pl\" put a part before or after the release (like PHP's
version_compare)."
  (apply #'editor-call "CompareVersion" args))

(defun editor-set-draw-switch (&rest args)
  "SetDrawSwitch(on: int) -> int

Turns the draw switch on (non-zero) or off (0) and returns the state before (1
or 0). The editor keeps the flag for GetDrawSwitch but still paints the text
when it likes."
  (apply #'editor-call "SetDrawSwitch" args))

(defun editor-get-draw-switch (&rest args)
  "GetDrawSwitch() -> int

1 when the draw switch is on (the start state), 0 when a macro turned it off."
  (apply #'editor-call "GetDrawSwitch" args))

(defun editor-is-shown-status (&rest args)
  "IsShownStatus() -> int

1 when the status bar is shown, else 0."
  (apply #'editor-call "IsShownStatus" args))

(defun editor-is-cur-type-ext (&rest args)
  "IsCurTypeExt(extension: string) -> int

1 when the document type of this file is the type of the extension (looked up in
the extension lists of the types in the settings file, as a file name). Without
type settings every extension is the basic type."
  (apply #'editor-call "IsCurTypeExt" args))

(defun editor-is-same-type-ext (&rest args)
  "IsSameTypeExt(extension1: string, extension2: string) -> int

1 when the two extensions belong to the same document type (see IsCurTypeExt)."
  (apply #'editor-call "IsSameTypeExt" args))

(defun editor-expand-parameter (&rest args)
  "ExpandParameter(format: string) -> string

Replaces the codes that start with $ in a string: $$ ($), $F (full path), $f
(file name), $g (file name without the extension), $/ (full path with /), $N (as
$F), $b (extension), $e and $E (folder), $C (the selected text or the word at
the caret), $x and $y (caret column in UTF-16 units and line, from 1), $S (the
editor), $M (the macro file), $V (version), $A (name), and conditions
${R?a$:b$:c$} (R, w, M, U, N, I). A text without a file is (U+7121U+984C). $d $t $D $T $B
$Q $I are not supported and stay as written. At most 2047 UTF-16 units."
  (apply #'editor-call "ExpandParameter" args))

(defun editor-message-box (&rest args)
  "MessageBox(message: string, type: int?) -> int

Shows a message box and waits. The type is Windows' MB_ value: the low nibble
chooses the buttons (0 OK, 1 OK/Cancel, 2 Abort/Retry/Ignore, 3 Yes/No/Cancel, 4
Yes/No, 5 Retry/Cancel, 6 Cancel/Try again/Continue), the icon bits (0x10 stop,
0x20 question, 0x30 exclamation, 0x40 information) the icon; the default button
and the modality are ignored. Returns the button as Windows numbers it: 1 OK, 2
Cancel, 3 Abort, 4 Retry, 5 Ignore, 6 Yes, 7 No, 10 Try again, 11 Continue. A
box that is closed without an answer gives Cancel (the last button when there is
none)."
  (apply #'editor-call "MessageBox" args))

(defun editor-error-msg (&rest args)
  "ErrorMsg(message: string) -> int

Shows an error message with an OK button and waits. Returns 1."
  (apply #'editor-call "ErrorMsg" args))

(defun editor-warn-msg (&rest args)
  "WarnMsg(message: string) -> int

Shows a warning with an OK button and waits. Returns 1."
  (apply #'editor-call "WarnMsg" args))

(defun editor-info-msg (&rest args)
  "InfoMsg(message: string) -> int

Shows an information message with an OK button and waits. Returns 1."
  (apply #'editor-call "InfoMsg" args))

(defun editor-ok-cancel-box (&rest args)
  "OkCancelBox(message: string) -> int

Asks a question with OK and Cancel and waits. Returns 1 (OK) or 2 (Cancel)."
  (apply #'editor-call "OkCancelBox" args))

(defun editor-yes-no-box (&rest args)
  "YesNoBox(message: string) -> int

Asks a question with Yes and No and waits. Returns 6 (Yes) or 7 (No)."
  (apply #'editor-call "YesNoBox" args))

(defun editor-file-open-dialog (&rest args)
  "FileOpenDialog(defaultName: string?, filter: string?) -> string

Lets the user choose a file to open and returns its full path, or an empty
string when the dialog is cancelled. The default name and the filter are not
used: the dialog of the platform takes neither."
  (apply #'editor-call "FileOpenDialog" args))

(defun editor-file-save-dialog (&rest args)
  "FileSaveDialog(defaultName: string?, filter: string?) -> string

Lets the user choose a file name to save under and returns the full path, or an
empty string when the dialog is cancelled. The dialog starts with the default
name. The filter is not used."
  (apply #'editor-call "FileSaveDialog" args))

(defun editor-folder-dialog (&rest args)
  "FolderDialog(message: string?, defaultFolder: string?) -> string

Lets the user choose a folder and returns its full path, or an empty string when
the dialog is cancelled. The message becomes the prompt text of the platform
dialog where it has one; the default folder is not used."
  (apply #'editor-call "FolderDialog" args))

(defun editor-input-box (&rest args)
  "InputBox(message: string, defaultValue: string?, maxLength: int?) -> string

Shows a dialog with a text field and waits. The message is shown above the
field, the default value is in the field, and the text cannot be longer than the
maximum length (in UTF-16 code units; 0 or less, or no value, is 260). Returns
the text of the field when OK is pressed, or an empty string when the dialog is
cancelled."
  (apply #'editor-call "InputBox" args))

(defun editor-create-menu (&rest args)
  "CreateMenu(position: int, menu: string) -> int

Shows a pop-up menu and waits. The position is 1 for the caret (the pointer when
the caret is out of the view) and anything else for the pointer. The menu is a
list of items separated by commas; an item can start with flags in brackets: C
checked, D disabled, R radio, S opens a submenu, E ends a submenu (one for each
E), B (a new column) is ignored. A lone - is a separator. Write [][text] for a
text that starts with a bracket. Returns the number of the item chosen, counted
from 1 over the items only (separators and submenus are not counted), or 0 when
the menu is closed without a choice or has no item."
  (apply #'editor-call "CreateMenu" args))

(defun editor-sleep (&rest args)
  "Sleep(milliseconds: int) -> int

Waits for the given number of milliseconds (the window stays alive, and the
macro can be cancelled). Returns 0."
  (apply #'editor-call "Sleep" args))

(defun editor-file-new (&rest args)
  "FileNew() -> null

Sakura's F_FILENEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "FileNew" args))

(defun editor-file-open (&rest args)
  "FileOpen(arg1: string?, arg2: int?, arg3: int?, arg4: string?) -> null

MATSU Editor's own F_FILEOPEN2 (not in Sakura). Runs the command that a key, the
menu and a key macro run."
  (apply #'editor-call "FileOpen" args))

(defun editor-file-save (&rest args)
  "FileSave() -> null

Sakura's F_FILESAVE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "FileSave" args))

(defun editor-file-save-all (&rest args)
  "FileSaveAll() -> null

Sakura's F_FILESAVEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileSaveAll" args))

(defun editor-file-save-as-dialog (&rest args)
  "FileSaveAsDialog(arg1: string?, arg2: int?, arg3: int?) -> null

Sakura's F_FILESAVEAS_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileSaveAsDialog" args))

(defun editor-file-save-as (&rest args)
  "FileSaveAs(arg1: string?, arg2: int?, arg3: int?) -> null

Sakura's F_FILESAVEAS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileSaveAs" args))

(defun editor-file-close (&rest args)
  "FileClose() -> null

Sakura's F_FILECLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "FileClose" args))

(defun editor-file-close-open (&rest args)
  "FileCloseOpen() -> null

Sakura's F_FILECLOSE_OPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileCloseOpen" args))

(defun editor-file-reopen (&rest args)
  "FileReopen(arg1: int?) -> null

Sakura's F_FILE_REOPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopen" args))

(defun editor-file-reopen-sjis (&rest args)
  "FileReopenSJIS(arg1: int?) -> null

Sakura's F_FILE_REOPEN_SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenSJIS" args))

(defun editor-file-reopen-jis (&rest args)
  "FileReopenJIS(arg1: int?) -> null

Sakura's F_FILE_REOPEN_JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenJIS" args))

(defun editor-file-reopen-euc (&rest args)
  "FileReopenEUC(arg1: int?) -> null

Sakura's F_FILE_REOPEN_EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenEUC" args))

(defun editor-file-reopen-latin1 (&rest args)
  "FileReopenLatin1(arg1: int?) -> null

Sakura's F_FILE_REOPEN_LATIN1 (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenLatin1" args))

(defun editor-file-reopen-unicode (&rest args)
  "FileReopenUNICODE(arg1: int?) -> null

Sakura's F_FILE_REOPEN_UNICODE (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenUNICODE" args))

(defun editor-file-reopen-unicodebe (&rest args)
  "FileReopenUNICODEBE(arg1: int?) -> null

Sakura's F_FILE_REOPEN_UNICODEBE (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenUNICODEBE" args))

(defun editor-file-reopen-utf8 (&rest args)
  "FileReopenUTF8(arg1: int?) -> null

Sakura's F_FILE_REOPEN_UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenUTF8" args))

(defun editor-file-reopen-cesu8 (&rest args)
  "FileReopenCESU8(arg1: int?) -> null

Sakura's F_FILE_REOPEN_CESU8 (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenCESU8" args))

(defun editor-file-reopen-utf7 (&rest args)
  "FileReopenUTF7(arg1: int?) -> null

Sakura's F_FILE_REOPEN_UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FileReopenUTF7" args))

(defun editor-print (&rest args)
  "Print() -> null

Sakura's F_PRINT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Print" args))

(defun editor-print-preview (&rest args)
  "PrintPreview() -> null

Sakura's F_PRINT_PREVIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "PrintPreview" args))

(defun editor-print-page-setup (&rest args)
  "PrintPageSetup() -> null

Sakura's F_PRINT_PAGESETUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "PrintPageSetup" args))

(defun editor-open-hfromto-c (&rest args)
  "OpenHfromtoC() -> null

Sakura's F_OPEN_HfromtoC (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "OpenHfromtoC" args))

(defun editor-activate-sqlplus (&rest args)
  "ActivateSQLPLUS() -> null

Sakura's F_ACTIVATE_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ActivateSQLPLUS" args))

(defun editor-exec-sqlplus (&rest args)
  "ExecSQLPLUS() -> null

Sakura's F_PLSQL_COMPILE_ON_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ExecSQLPLUS" args))

(defun editor-exec-database-choice (&rest args)
  "ExecDatabaseChoice() -> null

MATSU Editor's own F_PLSQL_COMPILE_ON_DATABASE_CHOICE (not in Sakura). Runs the
command that a key, the menu and a key macro run."
  (apply #'editor-call "ExecDatabaseChoice" args))

(defun editor-browse (&rest args)
  "Browse() -> null

Sakura's F_BROWSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Browse" args))

(defun editor-view-mode (&rest args)
  "ViewMode() -> null

Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ViewMode" args))

(defun editor-read-only (&rest args)
  "ReadOnly() -> null

Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ReadOnly" args))

(defun editor-property-file (&rest args)
  "PropertyFile() -> null

Sakura's F_PROPERTY_FILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "PropertyFile" args))

(defun editor-exit-all-editors (&rest args)
  "ExitAllEditors() -> null

Sakura's F_EXITALLEDITORS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ExitAllEditors" args))

(defun editor-exit-all (&rest args)
  "ExitAll() -> null

Sakura's F_EXITALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ExitAll" args))

(defun editor-put-file (&rest args)
  "PutFile(arg1: string?, arg2: int?, arg3: int?) -> null

MATSU Editor's own F_PUTFILE (not in Sakura). Runs the command that a key, the
menu and a key macro run."
  (apply #'editor-call "PutFile" args))

(defun editor-ins-file (&rest args)
  "InsFile(arg1: string?, arg2: int?, arg3: int?) -> null

MATSU Editor's own F_INSFILE (not in Sakura). Runs the command that a key, the
menu and a key macro run."
  (apply #'editor-call "InsFile" args))

(defun editor-char (&rest args)
  "Char(arg1: int?) -> null

Sakura's F_WCHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Char" args))

(defun editor-char-ime (&rest args)
  "CharIme(arg1: int?) -> null

Sakura's F_IME_CHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CharIme" args))

(defun editor-undo (&rest args)
  "Undo() -> null

Sakura's F_UNDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Undo" args))

(defun editor-redo (&rest args)
  "Redo() -> null

Sakura's F_REDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Redo" args))

(defun editor-delete (&rest args)
  "Delete() -> null

Sakura's F_DELETE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Delete" args))

(defun editor-delete-back (&rest args)
  "DeleteBack() -> null

Sakura's F_DELETE_BACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "DeleteBack" args))

(defun editor-word-delete-to-start (&rest args)
  "WordDeleteToStart() -> null

Sakura's F_WordDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WordDeleteToStart" args))

(defun editor-word-delete-to-end (&rest args)
  "WordDeleteToEnd() -> null

Sakura's F_WordDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WordDeleteToEnd" args))

(defun editor-word-cut (&rest args)
  "WordCut() -> null

Sakura's F_WordCut (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WordCut" args))

(defun editor-word-delete (&rest args)
  "WordDelete() -> null

Sakura's F_WordDelete (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WordDelete" args))

(defun editor-line-cut-to-start (&rest args)
  "LineCutToStart() -> null

Sakura's F_LineCutToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "LineCutToStart" args))

(defun editor-line-cut-to-end (&rest args)
  "LineCutToEnd() -> null

Sakura's F_LineCutToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "LineCutToEnd" args))

(defun editor-line-delete-to-start (&rest args)
  "LineDeleteToStart() -> null

Sakura's F_LineDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "LineDeleteToStart" args))

(defun editor-line-delete-to-end (&rest args)
  "LineDeleteToEnd() -> null

Sakura's F_LineDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "LineDeleteToEnd" args))

(defun editor-cut-line (&rest args)
  "CutLine() -> null

Sakura's F_CUT_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CutLine" args))

(defun editor-delete-line (&rest args)
  "DeleteLine() -> null

Sakura's F_DELETE_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "DeleteLine" args))

(defun editor-duplicate-line (&rest args)
  "DuplicateLine() -> null

Sakura's F_DUPLICATELINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "DuplicateLine" args))

(defun editor-indent-tab (&rest args)
  "IndentTab() -> null

Sakura's F_INDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "IndentTab" args))

(defun editor-unindent-tab (&rest args)
  "UnindentTab() -> null

Sakura's F_UNINDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "UnindentTab" args))

(defun editor-indent-space (&rest args)
  "IndentSpace() -> null

Sakura's F_INDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "IndentSpace" args))

(defun editor-unindent-space (&rest args)
  "UnindentSpace() -> null

Sakura's F_UNINDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "UnindentSpace" args))

(defun editor-l-trim (&rest args)
  "LTrim() -> null

Sakura's F_LTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "LTrim" args))

(defun editor-r-trim (&rest args)
  "RTrim() -> null

Sakura's F_RTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "RTrim" args))

(defun editor-sort-asc (&rest args)
  "SortAsc() -> null

Sakura's F_SORT_ASC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "SortAsc" args))

(defun editor-sort-desc (&rest args)
  "SortDesc() -> null

Sakura's F_SORT_DESC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "SortDesc" args))

(defun editor-merge (&rest args)
  "Merge() -> null

Sakura's F_MERGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Merge" args))

(defun editor-left (&rest args)
  "Left() -> null

Sakura's F_LEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Left" args))

(defun editor-right (&rest args)
  "Right() -> null

Sakura's F_RIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Right" args))

(defun editor-up2 (&rest args)
  "Up2() -> null

Sakura's F_UP2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Up2" args))

(defun editor-down2 (&rest args)
  "Down2() -> null

Sakura's F_DOWN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Down2" args))

(defun editor-word-left (&rest args)
  "WordLeft() -> null

Sakura's F_WORDLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WordLeft" args))

(defun editor-word-right (&rest args)
  "WordRight() -> null

Sakura's F_WORDRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WordRight" args))

(defun editor-go-line-top (&rest args)
  "GoLineTop(arg1: int?) -> null

Sakura's F_GOLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "GoLineTop" args))

(defun editor-go-line-end (&rest args)
  "GoLineEnd(arg1: int?) -> null

Sakura's F_GOLINEEND (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "GoLineEnd" args))

(defun editor-half-page-up (&rest args)
  "HalfPageUp(arg1: int?) -> null

Sakura's F_HalfPageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HalfPageUp" args))

(defun editor-half-page-down (&rest args)
  "HalfPageDown(arg1: int?) -> null

Sakura's F_HalfPageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HalfPageDown" args))

(defun editor-page-up (&rest args)
  "PageUp(arg1: int?) -> null

Sakura's F_1PageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "PageUp" args))

(defun editor-page-down (&rest args)
  "PageDown(arg1: int?) -> null

Sakura's F_1PageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "PageDown" args))

(defun editor-cur-line-center (&rest args)
  "CurLineCenter() -> null

Sakura's F_CURLINECENTER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CurLineCenter" args))

(defun editor-cur-line-top (&rest args)
  "CurLineTop() -> null

Sakura's F_CURLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CurLineTop" args))

(defun editor-cur-line-bottom (&rest args)
  "CurLineBottom() -> null

Sakura's F_CURLINEBOTTOM (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CurLineBottom" args))

(defun editor-move-hist-prev (&rest args)
  "MoveHistPrev() -> null

Sakura's F_JUMPHIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "MoveHistPrev" args))

(defun editor-move-hist-next (&rest args)
  "MoveHistNext() -> null

Sakura's F_JUMPHIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "MoveHistNext" args))

(defun editor-move-hist-set (&rest args)
  "MoveHistSet() -> null

Sakura's F_JUMPHIST_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "MoveHistSet" args))

(defun editor-f_wnd-scroll-down (&rest args)
  "F_WndScrollDown() -> null

Sakura's F_WndScrollDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "F_WndScrollDown" args))

(defun editor-f_wnd-scroll-up (&rest args)
  "F_WndScrollUp() -> null

Sakura's F_WndScrollUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "F_WndScrollUp" args))

(defun editor-go-next-paragraph (&rest args)
  "GoNextParagraph() -> null

Sakura's F_GONEXTPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoNextParagraph" args))

(defun editor-go-prev-paragraph (&rest args)
  "GoPrevParagraph() -> null

Sakura's F_GOPREVPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoPrevParagraph" args))

(defun editor-go-modify-line-next (&rest args)
  "GoModifyLineNext() -> null

Sakura's F_MODIFYLINE_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoModifyLineNext" args))

(defun editor-go-modify-line-prev (&rest args)
  "GoModifyLinePrev() -> null

Sakura's F_MODIFYLINE_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoModifyLinePrev" args))

(defun editor-move-cursor-layout (&rest args)
  "MoveCursorLayout(arg1: int?, arg2: int?, arg3: int?) -> null

MATSU Editor's own F_MOVECURSORLAYOUT (not in Sakura). Runs the command that a
key, the menu and a key macro run."
  (apply #'editor-call "MoveCursorLayout" args))

(defun editor-wheel-up (&rest args)
  "WheelUp(arg1: int?) -> null

Sakura's F_WHEELUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WheelUp" args))

(defun editor-wheel-down (&rest args)
  "WheelDown(arg1: int?) -> null

Sakura's F_WHEELDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WheelDown" args))

(defun editor-wheel-left (&rest args)
  "WheelLeft(arg1: int?) -> null

Sakura's F_WHEELLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WheelLeft" args))

(defun editor-wheel-right (&rest args)
  "WheelRight(arg1: int?) -> null

Sakura's F_WHEELRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WheelRight" args))

(defun editor-wheel-page-up (&rest args)
  "WheelPageUp(arg1: int?) -> null

Sakura's F_WHEELPAGEUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WheelPageUp" args))

(defun editor-wheel-page-down (&rest args)
  "WheelPageDown(arg1: int?) -> null

Sakura's F_WHEELPAGEDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WheelPageDown" args))

(defun editor-wheel-page-left (&rest args)
  "WheelPageLeft(arg1: int?) -> null

Sakura's F_WHEELPAGELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WheelPageLeft" args))

(defun editor-wheel-page-right (&rest args)
  "WheelPageRight(arg1: int?) -> null

Sakura's F_WHEELPAGERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WheelPageRight" args))

(defun editor-select-word (&rest args)
  "SelectWord() -> null

Sakura's F_SELECTWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SelectWord" args))

(defun editor-select-line (&rest args)
  "SelectLine(arg1: int?) -> null

Sakura's F_SELECTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SelectLine" args))

(defun editor-begin-select (&rest args)
  "BeginSelect() -> null

Sakura's F_BEGIN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "BeginSelect" args))

(defun editor-up_sel (&rest args)
  "Up_Sel() -> null

Sakura's F_UP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Up_Sel" args))

(defun editor-down_sel (&rest args)
  "Down_Sel() -> null

Sakura's F_DOWN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Down_Sel" args))

(defun editor-left_sel (&rest args)
  "Left_Sel() -> null

Sakura's F_LEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Left_Sel" args))

(defun editor-right_sel (&rest args)
  "Right_Sel() -> null

Sakura's F_RIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Right_Sel" args))

(defun editor-up2_sel (&rest args)
  "Up2_Sel() -> null

Sakura's F_UP2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Up2_Sel" args))

(defun editor-down2_sel (&rest args)
  "Down2_Sel() -> null

Sakura's F_DOWN2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Down2_Sel" args))

(defun editor-word-left_sel (&rest args)
  "WordLeft_Sel() -> null

Sakura's F_WORDLEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WordLeft_Sel" args))

(defun editor-word-right_sel (&rest args)
  "WordRight_Sel() -> null

Sakura's F_WORDRIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WordRight_Sel" args))

(defun editor-go-line-top_sel (&rest args)
  "GoLineTop_Sel(arg1: int?) -> null

Sakura's F_GOLINETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoLineTop_Sel" args))

(defun editor-go-line-end_sel (&rest args)
  "GoLineEnd_Sel(arg1: int?) -> null

Sakura's F_GOLINEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoLineEnd_Sel" args))

(defun editor-half-page-up_sel (&rest args)
  "HalfPageUp_Sel(arg1: int?) -> null

Sakura's F_HalfPageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HalfPageUp_Sel" args))

(defun editor-half-page-down_sel (&rest args)
  "HalfPageDown_Sel(arg1: int?) -> null

Sakura's F_HalfPageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HalfPageDown_Sel" args))

(defun editor-page-up_sel (&rest args)
  "PageUp_Sel(arg1: int?) -> null

Sakura's F_1PageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "PageUp_Sel" args))

(defun editor-page-down_sel (&rest args)
  "PageDown_Sel(arg1: int?) -> null

Sakura's F_1PageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "PageDown_Sel" args))

(defun editor-go-file-top_sel (&rest args)
  "GoFileTop_Sel() -> null

Sakura's F_GOFILETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoFileTop_Sel" args))

(defun editor-go-file-end_sel (&rest args)
  "GoFileEnd_Sel() -> null

Sakura's F_GOFILEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoFileEnd_Sel" args))

(defun editor-go-next-paragraph_sel (&rest args)
  "GoNextParagraph_Sel() -> null

Sakura's F_GONEXTPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoNextParagraph_Sel" args))

(defun editor-go-prev-paragraph_sel (&rest args)
  "GoPrevParagraph_Sel() -> null

Sakura's F_GOPREVPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoPrevParagraph_Sel" args))

(defun editor-go-modify-line-next_sel (&rest args)
  "GoModifyLineNext_Sel() -> null

Sakura's F_MODIFYLINE_NEXT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoModifyLineNext_Sel" args))

(defun editor-go-modify-line-prev_sel (&rest args)
  "GoModifyLinePrev_Sel() -> null

Sakura's F_MODIFYLINE_PREV_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoModifyLinePrev_Sel" args))

(defun editor-begin-box-select (&rest args)
  "BeginBoxSelect() -> null

Sakura's F_BEGIN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "BeginBoxSelect" args))

(defun editor-up_box-sel (&rest args)
  "Up_BoxSel(arg1: int?) -> null

Sakura's F_UP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Up_BoxSel" args))

(defun editor-down_box-sel (&rest args)
  "Down_BoxSel(arg1: int?) -> null

Sakura's F_DOWN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Down_BoxSel" args))

(defun editor-left_box-sel (&rest args)
  "Left_BoxSel(arg1: int?) -> null

Sakura's F_LEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Left_BoxSel" args))

(defun editor-right_box-sel (&rest args)
  "Right_BoxSel(arg1: int?) -> null

Sakura's F_RIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Right_BoxSel" args))

(defun editor-up2_box-sel (&rest args)
  "Up2_BoxSel(arg1: int?) -> null

Sakura's F_UP2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Up2_BoxSel" args))

(defun editor-down2_box-sel (&rest args)
  "Down2_BoxSel(arg1: int?) -> null

Sakura's F_DOWN2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Down2_BoxSel" args))

(defun editor-word-left_box-sel (&rest args)
  "WordLeft_BoxSel(arg1: int?) -> null

Sakura's F_WORDLEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WordLeft_BoxSel" args))

(defun editor-word-right_box-sel (&rest args)
  "WordRight_BoxSel(arg1: int?) -> null

Sakura's F_WORDRIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WordRight_BoxSel" args))

(defun editor-go-logical-line-top_box-sel (&rest args)
  "GoLogicalLineTop_BoxSel(arg1: int?, arg2: int?) -> null

Sakura's F_GOLOGICALLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoLogicalLineTop_BoxSel" args))

(defun editor-go-line-top_box-sel (&rest args)
  "GoLineTop_BoxSel(arg1: int?, arg2: int?) -> null

Sakura's F_GOLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoLineTop_BoxSel" args))

(defun editor-go-line-end_box-sel (&rest args)
  "GoLineEnd_BoxSel(arg1: int?, arg2: int?) -> null

Sakura's F_GOLINEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoLineEnd_BoxSel" args))

(defun editor-half-page-up_box-sel (&rest args)
  "HalfPageUp_BoxSel(arg1: int?, arg2: int?) -> null

Sakura's F_HalfPageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HalfPageUp_BoxSel" args))

(defun editor-half-page-down_box-sel (&rest args)
  "HalfPageDown_BoxSel(arg1: int?, arg2: int?) -> null

Sakura's F_HalfPageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HalfPageDown_BoxSel" args))

(defun editor-page-up_box-sel (&rest args)
  "PageUp_BoxSel(arg1: int?, arg2: int?) -> null

Sakura's F_1PageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "PageUp_BoxSel" args))

(defun editor-page-down_box-sel (&rest args)
  "PageDown_BoxSel(arg1: int?, arg2: int?) -> null

Sakura's F_1PageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "PageDown_BoxSel" args))

(defun editor-go-file-top_box-sel (&rest args)
  "GoFileTop_BoxSel(arg1: int?) -> null

Sakura's F_GOFILETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoFileTop_BoxSel" args))

(defun editor-go-file-end_box-sel (&rest args)
  "GoFileEnd_BoxSel(arg1: int?) -> null

Sakura's F_GOFILEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GoFileEnd_BoxSel" args))

(defun editor-cut (&rest args)
  "Cut() -> null

Sakura's F_CUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Cut" args))

(defun editor-copy (&rest args)
  "Copy() -> null

Sakura's F_COPY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Copy" args))

(defun editor-paste (&rest args)
  "Paste(arg1: int?) -> null

Sakura's F_PASTE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Paste" args))

(defun editor-copy-add-crlf (&rest args)
  "CopyAddCRLF() -> null

Sakura's F_COPY_ADDCRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CopyAddCRLF" args))

(defun editor-copy-crlf (&rest args)
  "CopyCRLF() -> null

Sakura's F_COPY_CRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CopyCRLF" args))

(defun editor-copy-lf (&rest args)
  "CopyLF() -> null

MATSU Editor's own F_COPY_LF (not in Sakura). Runs the command that a key, the
menu and a key macro run."
  (apply #'editor-call "CopyLF" args))

(defun editor-copy-cr (&rest args)
  "CopyCR() -> null

MATSU Editor's own F_COPY_CR (not in Sakura). Runs the command that a key, the
menu and a key macro run."
  (apply #'editor-call "CopyCR" args))

(defun editor-paste-box (&rest args)
  "PasteBox(arg1: int?) -> null

Sakura's F_PASTEBOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "PasteBox" args))

(defun editor-ins-box-text (&rest args)
  "InsBoxText(arg1: string?) -> null

MATSU Editor's own F_INSBOXTEXT (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "InsBoxText" args))

(defun editor-copy-lines (&rest args)
  "CopyLines() -> null

Sakura's F_COPYLINES (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CopyLines" args))

(defun editor-copy-lines-as-passage (&rest args)
  "CopyLinesAsPassage() -> null

Sakura's F_COPYLINESASPASSAGE (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CopyLinesAsPassage" args))

(defun editor-copy-lines-with-line-number (&rest args)
  "CopyLinesWithLineNumber() -> null

Sakura's F_COPYLINESWITHLINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CopyLinesWithLineNumber" args))

(defun editor-copy-color-html (&rest args)
  "CopyColorHtml() -> null

Sakura's F_COPY_COLOR_HTML (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CopyColorHtml" args))

(defun editor-copy-color-html-with-line-number (&rest args)
  "CopyColorHtmlWithLineNumber() -> null

Sakura's F_COPY_COLOR_HTML_LINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs
the command that a key, the menu and a key macro run. The last arguments may be
left out. A command that this editor does not have yet answers the error -32001
(not implemented)."
  (apply #'editor-call "CopyColorHtmlWithLineNumber" args))

(defun editor-copy-path (&rest args)
  "CopyPath() -> null

Sakura's F_COPYPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CopyPath" args))

(defun editor-copy-dir-path (&rest args)
  "CopyDirPath() -> null

Sakura's F_COPYDIRPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CopyDirPath" args))

(defun editor-copy-filename (&rest args)
  "CopyFilename() -> null

Sakura's F_COPYFNAME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CopyFilename" args))

(defun editor-copy-tag (&rest args)
  "CopyTag() -> null

Sakura's F_COPYTAG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CopyTag" args))

(defun editor-copy-key-bind-list (&rest args)
  "CopyKeyBindList() -> null

Sakura's F_CREATEKEYBINDLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CopyKeyBindList" args))

(defun editor-insert-date (&rest args)
  "InsertDate() -> null

Sakura's F_INS_DATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "InsertDate" args))

(defun editor-insert-time (&rest args)
  "InsertTime() -> null

Sakura's F_INS_TIME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "InsertTime" args))

(defun editor-ctrl-code-dialog (&rest args)
  "CtrlCodeDialog() -> null

Sakura's F_CTRL_CODE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CtrlCodeDialog" args))

(defun editor-ctrl-code (&rest args)
  "CtrlCode(arg1: int?) -> null

Sakura's F_CTRL_CODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CtrlCode" args))

(defun editor-insert-file-used-recently (&rest args)
  "InsertFileUsedRecently() -> null

Sakura's F_INS_FILE_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "InsertFileUsedRecently" args))

(defun editor-insert-folder-used-recently (&rest args)
  "InsertFolderUsedRecently() -> null

Sakura's F_INS_FOLDER_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "InsertFolderUsedRecently" args))

(defun editor-to-lower (&rest args)
  "ToLower() -> null

Sakura's F_TOLOWER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ToLower" args))

(defun editor-to-upper (&rest args)
  "ToUpper() -> null

Sakura's F_TOUPPER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ToUpper" args))

(defun editor-to-hankaku (&rest args)
  "ToHankaku() -> null

Sakura's F_TOHANKAKU (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ToHankaku" args))

(defun editor-to-hankata (&rest args)
  "ToHankata() -> null

Sakura's F_TOHANKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ToHankata" args))

(defun editor-to-zen-ei (&rest args)
  "ToZenEi() -> null

Sakura's F_TOZENEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ToZenEi" args))

(defun editor-to-han-ei (&rest args)
  "ToHanEi() -> null

Sakura's F_TOHANEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ToHanEi" args))

(defun editor-to-zen-kata (&rest args)
  "ToZenKata() -> null

Sakura's F_TOZENKAKUKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ToZenKata" args))

(defun editor-to-zen-hira (&rest args)
  "ToZenHira() -> null

Sakura's F_TOZENKAKUHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ToZenHira" args))

(defun editor-han-kata-to-zen-kata (&rest args)
  "HanKataToZenKata() -> null

Sakura's F_HANKATATOZENKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HanKataToZenKata" args))

(defun editor-han-kata-to-zen-hira (&rest args)
  "HanKataToZenHira() -> null

Sakura's F_HANKATATOZENHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HanKataToZenHira" args))

(defun editor-tab-to-space (&rest args)
  "TABToSPACE() -> null

Sakura's F_TABTOSPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TABToSPACE" args))

(defun editor-space-to-tab (&rest args)
  "SPACEToTAB() -> null

Sakura's F_SPACETOTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SPACEToTAB" args))

(defun editor-auto-to-sjis (&rest args)
  "AutoToSJIS() -> null

Sakura's F_CODECNV_AUTO2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "AutoToSJIS" args))

(defun editor-ji-sto-sjis (&rest args)
  "JIStoSJIS() -> null

Sakura's F_CODECNV_EMAIL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "JIStoSJIS" args))

(defun editor-eu-cto-sjis (&rest args)
  "EUCtoSJIS() -> null

Sakura's F_CODECNV_EUC2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "EUCtoSJIS" args))

(defun editor-code-cnv-unicod-eto-sjis (&rest args)
  "CodeCnvUNICODEtoSJIS() -> null

Sakura's F_CODECNV_UNICODE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CodeCnvUNICODEtoSJIS" args))

(defun editor-code-cnv-unicodeb-eto-sjis (&rest args)
  "CodeCnvUNICODEBEtoSJIS() -> null

Sakura's F_CODECNV_UNICODEBE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CodeCnvUNICODEBEtoSJIS" args))

(defun editor-utf8to-sjis (&rest args)
  "UTF8toSJIS() -> null

Sakura's F_CODECNV_UTF82SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "UTF8toSJIS" args))

(defun editor-utf7to-sjis (&rest args)
  "UTF7toSJIS() -> null

Sakura's F_CODECNV_UTF72SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "UTF7toSJIS" args))

(defun editor-sji-sto-jis (&rest args)
  "SJIStoJIS() -> null

Sakura's F_CODECNV_SJIS2JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SJIStoJIS" args))

(defun editor-sji-sto-euc (&rest args)
  "SJIStoEUC() -> null

Sakura's F_CODECNV_SJIS2EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SJIStoEUC" args))

(defun editor-sji-sto-utf8 (&rest args)
  "SJIStoUTF8() -> null

Sakura's F_CODECNV_SJIS2UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SJIStoUTF8" args))

(defun editor-sji-sto-utf7 (&rest args)
  "SJIStoUTF7() -> null

Sakura's F_CODECNV_SJIS2UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SJIStoUTF7" args))

(defun editor-base64-decode (&rest args)
  "Base64Decode() -> null

Sakura's F_BASE64DECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "Base64Decode" args))

(defun editor-uudecode (&rest args)
  "Uudecode() -> null

Sakura's F_UUDECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Uudecode" args))

(defun editor-search-dialog (&rest args)
  "SearchDialog() -> null

Sakura's F_SEARCH_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SearchDialog" args))

(defun editor-search-next (&rest args)
  "SearchNext(arg1: string?, arg2: int?) -> null

Sakura's F_SEARCH_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SearchNext" args))

(defun editor-search-prev (&rest args)
  "SearchPrev(arg1: string?, arg2: int?) -> null

Sakura's F_SEARCH_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SearchPrev" args))

(defun editor-replace-dialog (&rest args)
  "ReplaceDialog() -> null

Sakura's F_REPLACE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ReplaceDialog" args))

(defun editor-replace (&rest args)
  "Replace(arg1: string?, arg2: string?, arg3: int?) -> null

Sakura's F_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Replace" args))

(defun editor-replace-all (&rest args)
  "ReplaceAll(arg1: string?, arg2: string?, arg3: int?) -> null

Sakura's F_REPLACE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ReplaceAll" args))

(defun editor-search-clear-mark (&rest args)
  "SearchClearMark() -> null

Sakura's F_SEARCH_CLEARMARK (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SearchClearMark" args))

(defun editor-search-start-pos (&rest args)
  "SearchStartPos() -> null

Sakura's F_JUMP_SRCHSTARTPOS (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SearchStartPos" args))

(defun editor-grep (&rest args)
  "Grep(arg1: string?, arg2: string?, arg3: string?, arg4: int?, arg5: int?) -> null

Sakura's F_GREP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Grep" args))

(defun editor-grep-replace (&rest args)
  "GrepReplace(arg1: string?, arg2: string?, arg3: string?, arg4: string?, arg5: int?, arg6: int?) -> null

Sakura's F_GREP_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GrepReplace" args))

(defun editor-outline (&rest args)
  "Outline(arg1: int?) -> null

Sakura's F_OUTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Outline" args))

(defun editor-tag-jump (&rest args)
  "TagJump() -> null

Sakura's F_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "TagJump" args))

(defun editor-tag-jump-back (&rest args)
  "TagJumpBack() -> null

Sakura's F_TAGJUMPBACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TagJumpBack" args))

(defun editor-tag-make (&rest args)
  "TagMake() -> null

Sakura's F_TAGS_MAKE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "TagMake" args))

(defun editor-direct-tag-jump (&rest args)
  "DirectTagJump() -> null

Sakura's F_DIRECT_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "DirectTagJump" args))

(defun editor-keyword-tag-jump (&rest args)
  "KeywordTagJump(arg1: string?) -> null

Sakura's F_TAGJUMP_KEYWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "KeywordTagJump" args))

(defun editor-compare (&rest args)
  "Compare() -> null

Sakura's F_COMPARE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Compare" args))

(defun editor-diff-dialog (&rest args)
  "DiffDialog() -> null

Sakura's F_DIFF_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "DiffDialog" args))

(defun editor-diff (&rest args)
  "Diff(arg1: string?, arg2: int?) -> null

Sakura's F_DIFF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Diff" args))

(defun editor-diff-next (&rest args)
  "DiffNext() -> null

Sakura's F_DIFF_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "DiffNext" args))

(defun editor-diff-prev (&rest args)
  "DiffPrev() -> null

Sakura's F_DIFF_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "DiffPrev" args))

(defun editor-diff-reset (&rest args)
  "DiffReset() -> null

Sakura's F_DIFF_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "DiffReset" args))

(defun editor-bracket-pair (&rest args)
  "BracketPair() -> null

Sakura's F_BRACKETPAIR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "BracketPair" args))

(defun editor-bookmark-set (&rest args)
  "BookmarkSet() -> null

Sakura's F_BOOKMARK_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "BookmarkSet" args))

(defun editor-bookmark-next (&rest args)
  "BookmarkNext() -> null

Sakura's F_BOOKMARK_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "BookmarkNext" args))

(defun editor-bookmark-prev (&rest args)
  "BookmarkPrev() -> null

Sakura's F_BOOKMARK_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "BookmarkPrev" args))

(defun editor-bookmark-reset (&rest args)
  "BookmarkReset() -> null

Sakura's F_BOOKMARK_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "BookmarkReset" args))

(defun editor-bookmark-view (&rest args)
  "BookmarkView() -> null

Sakura's F_BOOKMARK_VIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "BookmarkView" args))

(defun editor-bookmark-pattern (&rest args)
  "BookmarkPattern(arg1: string?, arg2: int?) -> null

Sakura's F_BOOKMARK_PATTERN (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "BookmarkPattern" args))

(defun editor-func-list-next (&rest args)
  "FuncListNext() -> null

Sakura's F_FUNCLIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FuncListNext" args))

(defun editor-func-list-prev (&rest args)
  "FuncListPrev() -> null

Sakura's F_FUNCLIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "FuncListPrev" args))

(defun editor-chgmod-ins (&rest args)
  "ChgmodINS() -> null

Sakura's F_CHGMOD_INS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ChgmodINS" args))

(defun editor-chg-char-set (&rest args)
  "ChgCharSet(arg1: int?, arg2: int?) -> null

Sakura's F_CHG_CHARSET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ChgCharSet" args))

(defun editor-chgmod-eol (&rest args)
  "ChgmodEOL(arg1: int?) -> null

MATSU Editor's own F_CHGMOD_EOL (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "ChgmodEOL" args))

(defun editor-cancel-mode (&rest args)
  "CancelMode() -> null

Sakura's F_CANCEL_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CancelMode" args))

(defun editor-exec-external-macro (&rest args)
  "ExecExternalMacro(arg1: string?, arg2: string?) -> null

Sakura's F_EXECEXTMACRO (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ExecExternalMacro" args))

(defun editor-show-toolbar (&rest args)
  "ShowToolbar() -> null

Sakura's F_SHOWTOOLBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ShowToolbar" args))

(defun editor-show-funckey (&rest args)
  "ShowFunckey() -> null

Sakura's F_SHOWFUNCKEY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ShowFunckey" args))

(defun editor-show-tab (&rest args)
  "ShowTab() -> null

Sakura's F_SHOWTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ShowTab" args))

(defun editor-show-statusbar (&rest args)
  "ShowStatusbar() -> null

Sakura's F_SHOWSTATUSBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ShowStatusbar" args))

(defun editor-show-mini-map (&rest args)
  "ShowMiniMap() -> null

Sakura's F_SHOWMINIMAP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ShowMiniMap" args))

(defun editor-type-list (&rest args)
  "TypeList() -> null

Sakura's F_TYPE_LIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "TypeList" args))

(defun editor-change-type (&rest args)
  "ChangeType(arg1: int?) -> null

MATSU Editor's own F_CHANGETYPE (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "ChangeType" args))

(defun editor-option-type (&rest args)
  "OptionType() -> null

Sakura's F_OPTION_TYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "OptionType" args))

(defun editor-option-common (&rest args)
  "OptionCommon() -> null

Sakura's F_OPTION (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "OptionCommon" args))

(defun editor-select-font (&rest args)
  "SelectFont() -> null

Sakura's F_FONT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
the menu and a key macro run. The last arguments may be left out. A command that
this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "SelectFont" args))

(defun editor-set-font-size (&rest args)
  "SetFontSize(arg1: int?, arg2: int?, arg3: int?) -> null

Sakura's F_SETFONTSIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SetFontSize" args))

(defun editor-wrap-window-width (&rest args)
  "WrapWindowWidth() -> null

Sakura's F_WRAPWINDOWWIDTH (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WrapWindowWidth" args))

(defun editor-option-favorite (&rest args)
  "OptionFavorite() -> null

Sakura's F_FAVORITE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "OptionFavorite" args))

(defun editor-set-msg-quote-str (&rest args)
  "SetMsgQuoteStr(arg1: string?) -> null

MATSU Editor's own F_SET_QUOTESTRING (not in Sakura). Runs the command that a
key, the menu and a key macro run."
  (apply #'editor-call "SetMsgQuoteStr" args))

(defun editor-text-wrap-method (&rest args)
  "TextWrapMethod(arg1: int?) -> null

MATSU Editor's own F_TEXTWRAPMETHOD (not in Sakura). Runs the command that a
key, the menu and a key macro run."
  (apply #'editor-call "TextWrapMethod" args))

(defun editor-select-count-mode (&rest args)
  "SelectCountMode(arg1: int?) -> null

Sakura's F_SELECT_COUNT_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "SelectCountMode" args))

(defun editor-exec-command (&rest args)
  "ExecCommand(arg1: string?, arg2: int?, arg3: string?) -> null

Sakura's F_EXECMD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ExecCommand" args))

(defun editor-exec-command-dialog (&rest args)
  "ExecCommandDialog() -> null

Sakura's F_EXECMD_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ExecCommandDialog" args))

(defun editor-r-menu (&rest args)
  "RMenu() -> null

Sakura's F_MENU_RBUTTON (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "RMenu" args))

(defun editor-cust-menu1 (&rest args)
  "CustMenu1() -> null

Sakura's F_CUSTMENU_1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu1" args))

(defun editor-cust-menu2 (&rest args)
  "CustMenu2() -> null

Sakura's F_CUSTMENU_2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu2" args))

(defun editor-cust-menu3 (&rest args)
  "CustMenu3() -> null

Sakura's F_CUSTMENU_3 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu3" args))

(defun editor-cust-menu4 (&rest args)
  "CustMenu4() -> null

Sakura's F_CUSTMENU_4 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu4" args))

(defun editor-cust-menu5 (&rest args)
  "CustMenu5() -> null

Sakura's F_CUSTMENU_5 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu5" args))

(defun editor-cust-menu6 (&rest args)
  "CustMenu6() -> null

Sakura's F_CUSTMENU_6 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu6" args))

(defun editor-cust-menu7 (&rest args)
  "CustMenu7() -> null

Sakura's F_CUSTMENU_7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu7" args))

(defun editor-cust-menu8 (&rest args)
  "CustMenu8() -> null

Sakura's F_CUSTMENU_8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu8" args))

(defun editor-cust-menu9 (&rest args)
  "CustMenu9() -> null

Sakura's F_CUSTMENU_9 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu9" args))

(defun editor-cust-menu10 (&rest args)
  "CustMenu10() -> null

Sakura's F_CUSTMENU_10 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu10" args))

(defun editor-cust-menu11 (&rest args)
  "CustMenu11() -> null

Sakura's F_CUSTMENU_11 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu11" args))

(defun editor-cust-menu12 (&rest args)
  "CustMenu12() -> null

Sakura's F_CUSTMENU_12 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu12" args))

(defun editor-cust-menu13 (&rest args)
  "CustMenu13() -> null

Sakura's F_CUSTMENU_13 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu13" args))

(defun editor-cust-menu14 (&rest args)
  "CustMenu14() -> null

Sakura's F_CUSTMENU_14 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu14" args))

(defun editor-cust-menu15 (&rest args)
  "CustMenu15() -> null

Sakura's F_CUSTMENU_15 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu15" args))

(defun editor-cust-menu16 (&rest args)
  "CustMenu16() -> null

Sakura's F_CUSTMENU_16 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu16" args))

(defun editor-cust-menu17 (&rest args)
  "CustMenu17() -> null

Sakura's F_CUSTMENU_17 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu17" args))

(defun editor-cust-menu18 (&rest args)
  "CustMenu18() -> null

Sakura's F_CUSTMENU_18 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu18" args))

(defun editor-cust-menu19 (&rest args)
  "CustMenu19() -> null

Sakura's F_CUSTMENU_19 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu19" args))

(defun editor-cust-menu20 (&rest args)
  "CustMenu20() -> null

Sakura's F_CUSTMENU_20 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu20" args))

(defun editor-cust-menu21 (&rest args)
  "CustMenu21() -> null

Sakura's F_CUSTMENU_21 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu21" args))

(defun editor-cust-menu22 (&rest args)
  "CustMenu22() -> null

Sakura's F_CUSTMENU_22 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu22" args))

(defun editor-cust-menu23 (&rest args)
  "CustMenu23() -> null

Sakura's F_CUSTMENU_23 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu23" args))

(defun editor-cust-menu24 (&rest args)
  "CustMenu24() -> null

Sakura's F_CUSTMENU_24 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CustMenu24" args))

(defun editor-split-win-v (&rest args)
  "SplitWinV() -> null

Sakura's F_SPLIT_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "SplitWinV" args))

(defun editor-split-win-h (&rest args)
  "SplitWinH() -> null

Sakura's F_SPLIT_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "SplitWinH" args))

(defun editor-split-win-vh (&rest args)
  "SplitWinVH() -> null

Sakura's F_SPLIT_VH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "SplitWinVH" args))

(defun editor-win-maximize (&rest args)
  "WinMaximize() -> null

MATSU Editor's own F_WINMAXIMIZE (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "WinMaximize" args))

(defun editor-win-minimize (&rest args)
  "WinMinimize() -> null

MATSU Editor's own F_WINMINIMIZE (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "WinMinimize" args))

(defun editor-win-restore (&rest args)
  "WinRestore() -> null

MATSU Editor's own F_WINRESTORE (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "WinRestore" args))

(defun editor-win-close (&rest args)
  "WinClose() -> null

Sakura's F_WINCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WinClose" args))

(defun editor-win-close-all (&rest args)
  "WinCloseAll() -> null

Sakura's F_WIN_CLOSEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "WinCloseAll" args))

(defun editor-cascade-win (&rest args)
  "CascadeWin() -> null

Sakura's F_CASCADE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "CascadeWin" args))

(defun editor-tile-win-v (&rest args)
  "TileWinV() -> null

Sakura's F_TILE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "TileWinV" args))

(defun editor-tile-win-h (&rest args)
  "TileWinH() -> null

Sakura's F_TILE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "TileWinH" args))

(defun editor-next-window (&rest args)
  "NextWindow() -> null

Sakura's F_NEXTWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "NextWindow" args))

(defun editor-prev-window (&rest args)
  "PrevWindow() -> null

Sakura's F_PREVWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "PrevWindow" args))

(defun editor-window-list (&rest args)
  "WindowList() -> null

Sakura's F_WINLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WindowList" args))

(defun editor-maximize-v (&rest args)
  "MaximizeV() -> null

Sakura's F_MAXIMIZE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "MaximizeV" args))

(defun editor-maximize-h (&rest args)
  "MaximizeH() -> null

Sakura's F_MAXIMIZE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "MaximizeH" args))

(defun editor-minimize-all (&rest args)
  "MinimizeAll() -> null

Sakura's F_MINIMIZE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "MinimizeAll" args))

(defun editor-re-draw (&rest args)
  "ReDraw() -> null

Sakura's F_REDRAW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ReDraw" args))

(defun editor-activate-win-output (&rest args)
  "ActivateWinOutput() -> null

Sakura's F_WIN_OUTPUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ActivateWinOutput" args))

(defun editor-preview-tab (&rest args)
  "PreviewTab() -> null

MATSU Editor's own F_PREVIEW_TAB (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "PreviewTab" args))

(defun editor-trace-out (&rest args)
  "TraceOut(arg1: string?, arg2: int?) -> null

MATSU Editor's own F_TRACEOUT (not in Sakura). Runs the command that a key, the
menu and a key macro run."
  (apply #'editor-call "TraceOut" args))

(defun editor-window-top-most (&rest args)
  "WindowTopMost(arg1: int?) -> null

Sakura's F_TOPMOST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "WindowTopMost" args))

(defun editor-group-close (&rest args)
  "GroupClose() -> null

Sakura's F_GROUPCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "GroupClose" args))

(defun editor-next-group (&rest args)
  "NextGroup() -> null

Sakura's F_NEXTGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "NextGroup" args))

(defun editor-prev-group (&rest args)
  "PrevGroup() -> null

Sakura's F_PREVGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "PrevGroup" args))

(defun editor-tab-move-right (&rest args)
  "TabMoveRight() -> null

Sakura's F_TAB_MOVERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TabMoveRight" args))

(defun editor-tab-move-left (&rest args)
  "TabMoveLeft() -> null

Sakura's F_TAB_MOVELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TabMoveLeft" args))

(defun editor-tab-separate (&rest args)
  "TabSeparate() -> null

Sakura's F_TAB_SEPARATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TabSeparate" args))

(defun editor-tab-joint-next (&rest args)
  "TabJointNext() -> null

Sakura's F_TAB_JOINTNEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TabJointNext" args))

(defun editor-tab-joint-prev (&rest args)
  "TabJointPrev() -> null

Sakura's F_TAB_JOINTPREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TabJointPrev" args))

(defun editor-tab-close-other (&rest args)
  "TabCloseOther() -> null

Sakura's F_TAB_CLOSEOTHER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TabCloseOther" args))

(defun editor-tab-close-left (&rest args)
  "TabCloseLeft() -> null

Sakura's F_TAB_CLOSELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TabCloseLeft" args))

(defun editor-tab-close-right (&rest args)
  "TabCloseRight() -> null

Sakura's F_TAB_CLOSERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "TabCloseRight" args))

(defun editor-complete (&rest args)
  "Complete() -> null

Sakura's F_HOKAN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "Complete" args))

(defun editor-toggle-key-help-search (&rest args)
  "ToggleKeyHelpSearch(arg1: int?) -> null

Sakura's F_TOGGLE_KEY_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the
command that a key, the menu and a key macro run. The last arguments may be left
out. A command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ToggleKeyHelpSearch" args))

(defun editor-help-contents (&rest args)
  "HelpContents() -> null

Sakura's F_HELP_CONTENTS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HelpContents" args))

(defun editor-help-search (&rest args)
  "HelpSearch() -> null

Sakura's F_HELP_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "HelpSearch" args))

(defun editor-command-list (&rest args)
  "CommandList() -> null

Sakura's F_MENU_ALLFUNC (sakura_core/macro/CSMacroMgr.cpp). Runs the command
that a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "CommandList" args))

(defun editor-ext-help1 (&rest args)
  "ExtHelp1() -> null

Sakura's F_EXTHELP1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "ExtHelp1" args))

(defun editor-ext-html-help (&rest args)
  "ExtHtmlHelp(arg1: string?, arg2: string?) -> null

Sakura's F_EXTHTMLHELP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
a key, the menu and a key macro run. The last arguments may be left out. A
command that this editor does not have yet answers the error -32001 (not
implemented)."
  (apply #'editor-call "ExtHtmlHelp" args))

(defun editor-about (&rest args)
  "About() -> null

Sakura's F_ABOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
key, the menu and a key macro run. The last arguments may be left out. A command
that this editor does not have yet answers the error -32001 (not implemented)."
  (apply #'editor-call "About" args))

(defun editor-status-msg (&rest args)
  "StatusMsg(arg1: string?, arg2: int?) -> null

MATSU Editor's own F_STATUSMSG (not in Sakura). Runs the command that a key, the
menu and a key macro run."
  (apply #'editor-call "StatusMsg" args))

(defun editor-msg-beep (&rest args)
  "MsgBeep(arg1: int?) -> null

MATSU Editor's own F_MSGBEEP (not in Sakura). Runs the command that a key, the
menu and a key macro run."
  (apply #'editor-call "MsgBeep" args))

(defun editor-commit-undo-buffer (&rest args)
  "CommitUndoBuffer() -> null

MATSU Editor's own F_COMMITUNDOBUFFER (not in Sakura). Runs the command that a
key, the menu and a key macro run."
  (apply #'editor-call "CommitUndoBuffer" args))

(defun editor-add-ref-undo-buffer (&rest args)
  "AddRefUndoBuffer() -> null

MATSU Editor's own F_ADDREFUNDOBUFFER (not in Sakura). Runs the command that a
key, the menu and a key macro run."
  (apply #'editor-call "AddRefUndoBuffer" args))

(defun editor-set-undo-buffer (&rest args)
  "SetUndoBuffer() -> null

MATSU Editor's own F_SETUNDOBUFFER (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "SetUndoBuffer" args))

(defun editor-append-undo-buffer-cursor (&rest args)
  "AppendUndoBufferCursor() -> null

MATSU Editor's own F_APPENDUNDOBUFFERCURSOR (not in Sakura). Runs the command
that a key, the menu and a key macro run."
  (apply #'editor-call "AppendUndoBufferCursor" args))

(defun editor-clipboard-empty (&rest args)
  "ClipboardEmpty() -> null

MATSU Editor's own F_CLIPBOARDEMPTY (not in Sakura). Runs the command that a
key, the menu and a key macro run."
  (apply #'editor-call "ClipboardEmpty" args))

(defun editor-set-view-top (&rest args)
  "SetViewTop(arg1: int?) -> null

MATSU Editor's own F_SETVIEWTOP (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "SetViewTop" args))

(defun editor-set-view-left (&rest args)
  "SetViewLeft(arg1: int?) -> null

MATSU Editor's own F_SETVIEWLEFT (not in Sakura). Runs the command that a key,
the menu and a key macro run."
  (apply #'editor-call "SetViewLeft" args))

(provide 'api_gen)
