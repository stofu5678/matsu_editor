# GENERATED from crates/matsu-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: MATSU_UPDATE_GENERATED=1 cargo test -p matsu-plugin generated_files

package Editor;
use strict;
use warnings;

# The editor's macro API (the names and the arguments of Sakura's macros), as class methods:
# Editor->InsText('x'). The boot sets $CALL and $INFO.
our ($CALL, $INFO, $AUTOLOAD);

# What the editor told the macro in `initialize`, as a hash reference.
sub info { return $INFO }

# GetLineCount(reserved: int) -> int
# The number of logical lines (lines separated by line breaks). The text after the last line
# break is not a line unless it is not empty. The argument is reserved and must be 0.
sub GetLineCount { shift; return $CALL->('GetLineCount', @_) }

# GetLineStr(line: int) -> string
# A logical line with its line break ("\n"; the last line has none unless the text ends with
# one). 0 is the line of the cursor, 1 is the first line. Past the last line it is an empty
# string.
sub GetLineStr { shift; return $CALL->('GetLineStr', @_) }

# InsText(text: string) -> null
# Inserts the text at the cursor, replacing the selection. The cursor ends after the text.
sub InsText { shift; return $CALL->('InsText', @_) }

# AddTail(text: string) -> null
# Moves to the end of the file, drops the selection, and inserts the text there.
sub AddTail { shift; return $CALL->('AddTail', @_) }

# GoFileTop() -> null
# Moves the cursor to the start of the file and drops the selection.
sub GoFileTop { shift; return $CALL->('GoFileTop', @_) }

# GoFileEnd() -> null
# Moves the cursor to the end of the file and drops the selection.
sub GoFileEnd { shift; return $CALL->('GoFileEnd', @_) }

# Up() -> null
# Moves the cursor up one line and drops the selection.
sub Up { shift; return $CALL->('Up', @_) }

# Down() -> null
# Moves the cursor down one line and drops the selection.
sub Down { shift; return $CALL->('Down', @_) }

# SelectAll() -> null
# Selects the whole text. The cursor goes to the end.
sub SelectAll { shift; return $CALL->('SelectAll', @_) }

# GetSelectedString(reserved: int) -> string
# The selected text with its line breaks ("\n"), or an empty string when nothing is
# selected. The argument is reserved and ignored.
sub GetSelectedString { shift; return $CALL->('GetSelectedString', @_) }

# IsTextSelected() -> int
# 0 when nothing is selected, 1 when text is selected, 2 when a box selection is selected.
sub IsTextSelected { shift; return $CALL->('IsTextSelected', @_) }

# MoveCursor(line: int, column: int, option: int) -> null
# Moves the cursor to a line and a column, both counted from 1. The column counts UTF-16
# code units. Option: 0 does not select, 1 selects from the old position, 2 selects a box
# with the old and the new position as corners (2 wins over 1). Past the end of the text it
# goes to the end of the text.
sub MoveCursor { shift; return $CALL->('MoveCursor', @_) }

# Jump(line: int, flag: int?) -> null
# Moves to the start of a line, counted from 1 in logical lines. Lines before 1 and after
# the last one are limited to the first and the last line. Flag bit 0 (1, the default) is
# the line break unit, which is the only unit supported yet; bit 1 (PL/SQL) is not
# supported.
sub Jump { shift; return $CALL->('Jump', @_) }

# GetFilename() -> string
# The full path of the file, or an empty string when the text has no file.
sub GetFilename { shift; return $CALL->('GetFilename', @_) }

# GetSelectLineFrom() -> int
# The line where the selection starts, counted from 1 in layout lines (wrapped rows count as
# lines). 0 when nothing is selected.
sub GetSelectLineFrom { shift; return $CALL->('GetSelectLineFrom', @_) }

# GetSelectColmFrom() -> int
# The column where the selection starts, counted from 1 in layout columns (cells: a
# full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected. Same
# as GetSelectColumnFrom.
sub GetSelectColmFrom { shift; return $CALL->('GetSelectColmFrom', @_) }

# GetSelectColumnFrom() -> int
# The column where the selection starts, counted from 1 in layout columns (cells: a
# full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected.
sub GetSelectColumnFrom { shift; return $CALL->('GetSelectColumnFrom', @_) }

# GetSelectLineTo() -> int
# The line where the selection ends, counted from 1 in layout lines. 0 when nothing is
# selected.
sub GetSelectLineTo { shift; return $CALL->('GetSelectLineTo', @_) }

# GetSelectColmTo() -> int
# The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
# selected. Same as GetSelectColumnTo.
sub GetSelectColmTo { shift; return $CALL->('GetSelectColmTo', @_) }

# GetSelectColumnTo() -> int
# The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
# selected.
sub GetSelectColumnTo { shift; return $CALL->('GetSelectColumnTo', @_) }

# IsInsMode() -> int
# 1 in insert mode, 0 in overwrite mode.
sub IsInsMode { shift; return $CALL->('IsInsMode', @_) }

# GetCharCode() -> int
# The encoding of the file: 0 Shift_JIS, 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, 5 UTF-7, 6
# UTF-16 BE, 7 CESU-8, 8 Latin1.
sub GetCharCode { shift; return $CALL->('GetCharCode', @_) }

# GetLineCode() -> int
# The line ending that new line breaks get: 0 CR LF, 1 CR, 2 LF (3 NEL, 4 LS, 5 PS).
sub GetLineCode { shift; return $CALL->('GetLineCode', @_) }

# IsPossibleUndo() -> int
# 1 when there is something to undo, else 0.
sub IsPossibleUndo { shift; return $CALL->('IsPossibleUndo', @_) }

# IsPossibleRedo() -> int
# 1 when there is something to redo, else 0.
sub IsPossibleRedo { shift; return $CALL->('IsPossibleRedo', @_) }

# GetLineAttribute(line: int, type: int) -> int
# An attribute of a logical line (0 is the line of the cursor, 1 the first line). Type 0 is
# 1 when the line changed since the last save, type 1 the number of the undo step that last
# touched it, type 2 the bookmark (1 or 0). Types 3 (diff mark) and 4 (function list mark)
# answer the error -32001 until the editor has them. A negative line, a line past the end
# and any other type are errors.
sub GetLineAttribute { shift; return $CALL->('GetLineAttribute', @_) }

# IsTextSelectingLock() -> int
# 0 when the selection is not locked, 1 when it is locked (plain moves extend the
# selection), 2 when it is locked and a box selection.
sub IsTextSelectingLock { shift; return $CALL->('IsTextSelectingLock', @_) }

# GetSaveFilename() -> string
# The full path that a save writes to, or an empty string when the text has no file. The
# same as GetFilename, except in the macro that runs before a save (the registered macro of
# \u{4FDD}\u{5B58}\u{524D}), where it is the file being saved.
sub GetSaveFilename { shift; return $CALL->('GetSaveFilename', @_) }

# ChangeInsertSpace(value: int) -> int
# Whether the Tab key inserts spaces instead of a tab character, before the call (1 or 0). 0
# turns it off, 1 turns it on for this window (not saved); any other value only reads. Only
# the low 16 bits of the value count, as in Sakura.
sub ChangeInsertSpace { shift; return $CALL->('ChangeInsertSpace', @_) }

# LayoutToLogicLineNum(layoutLine: int) -> int
# The logical line (up to a line break) that a layout line (a row on the screen) belongs to,
# both counted from 1. The line after the last layout line gives the line after the last
# logical line. Any other line outside the text is an error.
sub LayoutToLogicLineNum { shift; return $CALL->('LayoutToLogicLineNum', @_) }

# LogicToLayoutLineNum(logicLine: int, logicIndex: int) -> int
# The layout line (a row on the screen) of a position in a logical line: the line and the
# index in it (UTF-16 code units), both counted from 1. A line past the end gives the line
# after the last layout line.
sub LogicToLayoutLineNum { shift; return $CALL->('LogicToLayoutLineNum', @_) }

# LineColumnToIndex(layoutLine: int, layoutColumn: int) -> int
# The index (UTF-16 code units, from 1) in the logical line of a layout line and a layout
# column (cells), both from 1. A column past the text gives the end of the text of the row.
sub LineColumnToIndex { shift; return $CALL->('LineColumnToIndex', @_) }

# LineIndexToColumn(logicLine: int, logicIndex: int) -> int
# The layout column (cells, from 1) of a position in a logical line: the line and the index
# in it (UTF-16 code units), both from 1. Tabs and full-width characters take their width.
sub LineIndexToColumn { shift; return $CALL->('LineIndexToColumn', @_) }

# GetStrWidth(text: string, column: int?) -> int
# The width of a string in cells, starting at layout column `column` (from 1, default 1; it
# matters for tabs). A CR LF at the end counts as the CR alone.
sub GetStrWidth { shift; return $CALL->('GetStrWidth', @_) }

# GetStrLayoutLength(text: string, column: int?) -> int
# The width of a string as a line of the document in cells, starting at layout column
# `column` (from 1, default 1). A line ending at the end takes one cell.
sub GetStrLayoutLength { shift; return $CALL->('GetStrLayoutLength', @_) }

# GetDefaultCharLength() -> int
# The layout width of a half-width character. This editor lays out in cells, so it is 1.
sub GetDefaultCharLength { shift; return $CALL->('GetDefaultCharLength', @_) }

# GetViewLines() -> int
# How many rows the text area shows (at least 1).
sub GetViewLines { shift; return $CALL->('GetViewLines', @_) }

# GetViewColumns() -> int
# How many columns the text area shows (at least 1).
sub GetViewColumns { shift; return $CALL->('GetViewColumns', @_) }

# GetViewTop() -> int
# The layout line at the top of the text area, counted from 1.
sub GetViewTop { shift; return $CALL->('GetViewTop', @_) }

# ChangeTabWidth(width: int) -> int
# The tab width before the call. A width from 1 to 64 that differs from it becomes the tab
# width of this window (not saved). Any other value, such as 0, only reads.
sub ChangeTabWidth { shift; return $CALL->('ChangeTabWidth', @_) }

# ChangeWrapColm(column: int) -> int
# The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
# column (not saved). Any other value only reads. Same as ChangeWrapColumn.
sub ChangeWrapColm { shift; return $CALL->('ChangeWrapColm', @_) }

# ChangeWrapColumn(column: int) -> int
# The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
# column (not saved). Any other value only reads.
sub ChangeWrapColumn { shift; return $CALL->('ChangeWrapColumn', @_) }

# GetClipboard(option: int?) -> string
# The text on the clipboard, or an empty string when there is none. The line endings are
# those of the source. The option is reserved.
sub GetClipboard { shift; return $CALL->('GetClipboard', @_) }

# SetClipboard(option: int?, text: string?) -> int
# Puts the text on the clipboard. Option bit 0 marks a box copy (a paste puts it in as a
# box), bit 1 a row copy (a paste puts it in front of the row); with both, the box wins.
# Returns 1 when it worked, 0 when not.
sub SetClipboard { shift; return $CALL->('SetClipboard', @_) }

# IsIncludeClipboardFormat(format: string) -> int
# 1 when the clipboard has the format. Only text is known: CF_TEXT, CF_OEMTEXT and
# CF_UNICODETEXT (or 1, 7, 13) are 1 when the clipboard has text; every other format is 0.
sub IsIncludeClipboardFormat { shift; return $CALL->('IsIncludeClipboardFormat', @_) }

# GetClipboardByFormat(format: string, mode: int, endMode: int?) -> string
# The text of the clipboard in a text format (CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT), with the
# line endings of the document. Mode -2 is the usual way and 3 is UTF-16 (the text itself);
# the other modes and formats are errors (-32001). The end mode is not used.
sub GetClipboardByFormat { shift; return $CALL->('GetClipboardByFormat', @_) }

# SetClipboardByFormat(text: string, format: string, mode: int, endMode: int?) -> int
# Puts the text on the clipboard as text. The formats and modes are those of
# GetClipboardByFormat. Returns 1 when it worked.
sub SetClipboardByFormat { shift; return $CALL->('SetClipboardByFormat', @_) }

# GetCookie(scope: string, name: string) -> string
# The value of a cookie, or an empty string when there is none. The scope is "window" or
# "document".
sub GetCookie { shift; return $CALL->('GetCookie', @_) }

# GetCookieDefault(scope: string, name: string, defaultValue: string) -> string
# The value of a cookie, or the default when there is none (an unknown scope still gives an
# empty string). The scope is "window" or "document".
sub GetCookieDefault { shift; return $CALL->('GetCookieDefault', @_) }

# SetCookie(scope: string, name: string, value: string) -> int
# Sets a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a name with a
# character that a name cannot have (Sakura lets in digits, letters and every character from
# "_" up in the character table). The scope is "window" or "document".
sub SetCookie { shift; return $CALL->('SetCookie', @_) }

# DeleteCookie(scope: string, name: string) -> int
# Deletes a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a bad name, 5
# when there is no such cookie. The scope is "window" or "document".
sub DeleteCookie { shift; return $CALL->('DeleteCookie', @_) }

# GetCookieNames(scope: string) -> string
# The names of the cookies of a scope, joined with commas, in the order of the names (UTF-16
# code units). An unknown scope gives an empty string. The scope is "window" or "document".
sub GetCookieNames { shift; return $CALL->('GetCookieNames', @_) }

# CompareVersion(versionA: string, versionB: string) -> int
# Compares two version strings: 0 when equal, positive when A is newer, negative when B is.
# Four parts at most; a part has two digits at most; "alpha", "beta", "rc" and "pl" put a
# part before or after the release (like PHP's version_compare).
sub CompareVersion { shift; return $CALL->('CompareVersion', @_) }

# SetDrawSwitch(on: int) -> int
# Turns the draw switch on (non-zero) or off (0) and returns the state before (1 or 0). The
# editor keeps the flag for GetDrawSwitch but still paints the text when it likes.
sub SetDrawSwitch { shift; return $CALL->('SetDrawSwitch', @_) }

# GetDrawSwitch() -> int
# 1 when the draw switch is on (the start state), 0 when a macro turned it off.
sub GetDrawSwitch { shift; return $CALL->('GetDrawSwitch', @_) }

# IsShownStatus() -> int
# 1 when the status bar is shown, else 0.
sub IsShownStatus { shift; return $CALL->('IsShownStatus', @_) }

# IsCurTypeExt(extension: string) -> int
# 1 when the document type of this file is the type of the extension (looked up in the
# extension lists of the types in the settings file, as a file name). Without type settings
# every extension is the basic type.
sub IsCurTypeExt { shift; return $CALL->('IsCurTypeExt', @_) }

# IsSameTypeExt(extension1: string, extension2: string) -> int
# 1 when the two extensions belong to the same document type (see IsCurTypeExt).
sub IsSameTypeExt { shift; return $CALL->('IsSameTypeExt', @_) }

# ExpandParameter(format: string) -> string
# Replaces the codes that start with $ in a string: $$ ($), $F (full path), $f (file name),
# $g (file name without the extension), $/ (full path with /), $N (as $F), $b (extension),
# $e and $E (folder), $C (the selected text or the word at the caret), $x and $y (caret
# column in UTF-16 units and line, from 1), $S (the editor), $M (the macro file), $V
# (version), $A (name), and conditions ${R?a$:b$:c$} (R, w, M, U, N, I). A text without a
# file is (\u{7121}\u{984C}). $d $t $D $T $B $Q $I are not supported and stay as written. At most 2047
# UTF-16 units.
sub ExpandParameter { shift; return $CALL->('ExpandParameter', @_) }

# MessageBox(message: string, type: int?) -> int
# Shows a message box and waits. The type is Windows' MB_ value: the low nibble chooses the
# buttons (0 OK, 1 OK/Cancel, 2 Abort/Retry/Ignore, 3 Yes/No/Cancel, 4 Yes/No, 5
# Retry/Cancel, 6 Cancel/Try again/Continue), the icon bits (0x10 stop, 0x20 question, 0x30
# exclamation, 0x40 information) the icon; the default button and the modality are ignored.
# Returns the button as Windows numbers it: 1 OK, 2 Cancel, 3 Abort, 4 Retry, 5 Ignore, 6
# Yes, 7 No, 10 Try again, 11 Continue. A box that is closed without an answer gives Cancel
# (the last button when there is none).
sub MessageBox { shift; return $CALL->('MessageBox', @_) }

# ErrorMsg(message: string) -> int
# Shows an error message with an OK button and waits. Returns 1.
sub ErrorMsg { shift; return $CALL->('ErrorMsg', @_) }

# WarnMsg(message: string) -> int
# Shows a warning with an OK button and waits. Returns 1.
sub WarnMsg { shift; return $CALL->('WarnMsg', @_) }

# InfoMsg(message: string) -> int
# Shows an information message with an OK button and waits. Returns 1.
sub InfoMsg { shift; return $CALL->('InfoMsg', @_) }

# OkCancelBox(message: string) -> int
# Asks a question with OK and Cancel and waits. Returns 1 (OK) or 2 (Cancel).
sub OkCancelBox { shift; return $CALL->('OkCancelBox', @_) }

# YesNoBox(message: string) -> int
# Asks a question with Yes and No and waits. Returns 6 (Yes) or 7 (No).
sub YesNoBox { shift; return $CALL->('YesNoBox', @_) }

# FileOpenDialog(defaultName: string?, filter: string?) -> string
# Lets the user choose a file to open and returns its full path, or an empty string when the
# dialog is cancelled. The default name and the filter are not used: the dialog of the
# platform takes neither.
sub FileOpenDialog { shift; return $CALL->('FileOpenDialog', @_) }

# FileSaveDialog(defaultName: string?, filter: string?) -> string
# Lets the user choose a file name to save under and returns the full path, or an empty
# string when the dialog is cancelled. The dialog starts with the default name. The filter
# is not used.
sub FileSaveDialog { shift; return $CALL->('FileSaveDialog', @_) }

# FolderDialog(message: string?, defaultFolder: string?) -> string
# Lets the user choose a folder and returns its full path, or an empty string when the
# dialog is cancelled. The message becomes the prompt text of the platform dialog where it
# has one; the default folder is not used.
sub FolderDialog { shift; return $CALL->('FolderDialog', @_) }

# InputBox(message: string, defaultValue: string?, maxLength: int?) -> string
# Shows a dialog with a text field and waits. The message is shown above the field, the
# default value is in the field, and the text cannot be longer than the maximum length (in
# UTF-16 code units; 0 or less, or no value, is 260). Returns the text of the field when OK
# is pressed, or an empty string when the dialog is cancelled.
sub InputBox { shift; return $CALL->('InputBox', @_) }

# CreateMenu(position: int, menu: string) -> int
# Shows a pop-up menu and waits. The position is 1 for the caret (the pointer when the caret
# is out of the view) and anything else for the pointer. The menu is a list of items
# separated by commas; an item can start with flags in brackets: C checked, D disabled, R
# radio, S opens a submenu, E ends a submenu (one for each E), B (a new column) is ignored.
# A lone - is a separator. Write [][text] for a text that starts with a bracket. Returns the
# number of the item chosen, counted from 1 over the items only (separators and submenus are
# not counted), or 0 when the menu is closed without a choice or has no item.
sub CreateMenu { shift; return $CALL->('CreateMenu', @_) }

# Sleep(milliseconds: int) -> int
# Waits for the given number of milliseconds (the window stays alive, and the macro can be
# cancelled). Returns 0.
sub Sleep { shift; return $CALL->('Sleep', @_) }

# FileNew() -> null
# Sakura's F_FILENEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub FileNew { shift; return $CALL->('FileNew', @_) }

# FileOpen(arg1: string?, arg2: int?, arg3: int?, arg4: string?) -> null
# MATSU Editor's own F_FILEOPEN2 (not in Sakura). Runs the command that a key, the menu and
# a key macro run.
sub FileOpen { shift; return $CALL->('FileOpen', @_) }

# FileSave() -> null
# Sakura's F_FILESAVE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub FileSave { shift; return $CALL->('FileSave', @_) }

# FileSaveAll() -> null
# Sakura's F_FILESAVEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileSaveAll { shift; return $CALL->('FileSaveAll', @_) }

# FileSaveAsDialog(arg1: string?, arg2: int?, arg3: int?) -> null
# Sakura's F_FILESAVEAS_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileSaveAsDialog { shift; return $CALL->('FileSaveAsDialog', @_) }

# FileSaveAs(arg1: string?, arg2: int?, arg3: int?) -> null
# Sakura's F_FILESAVEAS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub FileSaveAs { shift; return $CALL->('FileSaveAs', @_) }

# FileClose() -> null
# Sakura's F_FILECLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub FileClose { shift; return $CALL->('FileClose', @_) }

# FileCloseOpen() -> null
# Sakura's F_FILECLOSE_OPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileCloseOpen { shift; return $CALL->('FileCloseOpen', @_) }

# FileReopen(arg1: int?) -> null
# Sakura's F_FILE_REOPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopen { shift; return $CALL->('FileReopen', @_) }

# FileReopenSJIS(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopenSJIS { shift; return $CALL->('FileReopenSJIS', @_) }

# FileReopenJIS(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopenJIS { shift; return $CALL->('FileReopenJIS', @_) }

# FileReopenEUC(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopenEUC { shift; return $CALL->('FileReopenEUC', @_) }

# FileReopenLatin1(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_LATIN1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopenLatin1 { shift; return $CALL->('FileReopenLatin1', @_) }

# FileReopenUNICODE(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_UNICODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopenUNICODE { shift; return $CALL->('FileReopenUNICODE', @_) }

# FileReopenUNICODEBE(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_UNICODEBE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
# a key, the menu and a key macro run. The last arguments may be left out. A command that
# this editor does not have yet answers the error -32001 (not implemented).
sub FileReopenUNICODEBE { shift; return $CALL->('FileReopenUNICODEBE', @_) }

# FileReopenUTF8(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopenUTF8 { shift; return $CALL->('FileReopenUTF8', @_) }

# FileReopenCESU8(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_CESU8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopenCESU8 { shift; return $CALL->('FileReopenCESU8', @_) }

# FileReopenUTF7(arg1: int?) -> null
# Sakura's F_FILE_REOPEN_UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FileReopenUTF7 { shift; return $CALL->('FileReopenUTF7', @_) }

# Print() -> null
# Sakura's F_PRINT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Print { shift; return $CALL->('Print', @_) }

# PrintPreview() -> null
# Sakura's F_PRINT_PREVIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub PrintPreview { shift; return $CALL->('PrintPreview', @_) }

# PrintPageSetup() -> null
# Sakura's F_PRINT_PAGESETUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub PrintPageSetup { shift; return $CALL->('PrintPageSetup', @_) }

# OpenHfromtoC() -> null
# Sakura's F_OPEN_HfromtoC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub OpenHfromtoC { shift; return $CALL->('OpenHfromtoC', @_) }

# ActivateSQLPLUS() -> null
# Sakura's F_ACTIVATE_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ActivateSQLPLUS { shift; return $CALL->('ActivateSQLPLUS', @_) }

# ExecSQLPLUS() -> null
# Sakura's F_PLSQL_COMPILE_ON_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
# that a key, the menu and a key macro run. The last arguments may be left out. A command
# that this editor does not have yet answers the error -32001 (not implemented).
sub ExecSQLPLUS { shift; return $CALL->('ExecSQLPLUS', @_) }

# ExecDatabaseChoice() -> null
# MATSU Editor's own F_PLSQL_COMPILE_ON_DATABASE_CHOICE (not in Sakura). Runs the command
# that a key, the menu and a key macro run.
sub ExecDatabaseChoice { shift; return $CALL->('ExecDatabaseChoice', @_) }

# Browse() -> null
# Sakura's F_BROWSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Browse { shift; return $CALL->('Browse', @_) }

# ViewMode() -> null
# Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ViewMode { shift; return $CALL->('ViewMode', @_) }

# ReadOnly() -> null
# Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ReadOnly { shift; return $CALL->('ReadOnly', @_) }

# PropertyFile() -> null
# Sakura's F_PROPERTY_FILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub PropertyFile { shift; return $CALL->('PropertyFile', @_) }

# ExitAllEditors() -> null
# Sakura's F_EXITALLEDITORS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ExitAllEditors { shift; return $CALL->('ExitAllEditors', @_) }

# ExitAll() -> null
# Sakura's F_EXITALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ExitAll { shift; return $CALL->('ExitAll', @_) }

# PutFile(arg1: string?, arg2: int?, arg3: int?) -> null
# MATSU Editor's own F_PUTFILE (not in Sakura). Runs the command that a key, the menu and a
# key macro run.
sub PutFile { shift; return $CALL->('PutFile', @_) }

# InsFile(arg1: string?, arg2: int?, arg3: int?) -> null
# MATSU Editor's own F_INSFILE (not in Sakura). Runs the command that a key, the menu and a
# key macro run.
sub InsFile { shift; return $CALL->('InsFile', @_) }

# Char(arg1: int?) -> null
# Sakura's F_WCHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Char { shift; return $CALL->('Char', @_) }

# CharIme(arg1: int?) -> null
# Sakura's F_IME_CHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CharIme { shift; return $CALL->('CharIme', @_) }

# Undo() -> null
# Sakura's F_UNDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Undo { shift; return $CALL->('Undo', @_) }

# Redo() -> null
# Sakura's F_REDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Redo { shift; return $CALL->('Redo', @_) }

# Delete() -> null
# Sakura's F_DELETE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Delete { shift; return $CALL->('Delete', @_) }

# DeleteBack() -> null
# Sakura's F_DELETE_BACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub DeleteBack { shift; return $CALL->('DeleteBack', @_) }

# WordDeleteToStart() -> null
# Sakura's F_WordDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WordDeleteToStart { shift; return $CALL->('WordDeleteToStart', @_) }

# WordDeleteToEnd() -> null
# Sakura's F_WordDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WordDeleteToEnd { shift; return $CALL->('WordDeleteToEnd', @_) }

# WordCut() -> null
# Sakura's F_WordCut (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WordCut { shift; return $CALL->('WordCut', @_) }

# WordDelete() -> null
# Sakura's F_WordDelete (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WordDelete { shift; return $CALL->('WordDelete', @_) }

# LineCutToStart() -> null
# Sakura's F_LineCutToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub LineCutToStart { shift; return $CALL->('LineCutToStart', @_) }

# LineCutToEnd() -> null
# Sakura's F_LineCutToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub LineCutToEnd { shift; return $CALL->('LineCutToEnd', @_) }

# LineDeleteToStart() -> null
# Sakura's F_LineDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub LineDeleteToStart { shift; return $CALL->('LineDeleteToStart', @_) }

# LineDeleteToEnd() -> null
# Sakura's F_LineDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub LineDeleteToEnd { shift; return $CALL->('LineDeleteToEnd', @_) }

# CutLine() -> null
# Sakura's F_CUT_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CutLine { shift; return $CALL->('CutLine', @_) }

# DeleteLine() -> null
# Sakura's F_DELETE_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub DeleteLine { shift; return $CALL->('DeleteLine', @_) }

# DuplicateLine() -> null
# Sakura's F_DUPLICATELINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub DuplicateLine { shift; return $CALL->('DuplicateLine', @_) }

# IndentTab() -> null
# Sakura's F_INDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub IndentTab { shift; return $CALL->('IndentTab', @_) }

# UnindentTab() -> null
# Sakura's F_UNINDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub UnindentTab { shift; return $CALL->('UnindentTab', @_) }

# IndentSpace() -> null
# Sakura's F_INDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub IndentSpace { shift; return $CALL->('IndentSpace', @_) }

# UnindentSpace() -> null
# Sakura's F_UNINDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub UnindentSpace { shift; return $CALL->('UnindentSpace', @_) }

# LTrim() -> null
# Sakura's F_LTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub LTrim { shift; return $CALL->('LTrim', @_) }

# RTrim() -> null
# Sakura's F_RTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub RTrim { shift; return $CALL->('RTrim', @_) }

# SortAsc() -> null
# Sakura's F_SORT_ASC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub SortAsc { shift; return $CALL->('SortAsc', @_) }

# SortDesc() -> null
# Sakura's F_SORT_DESC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub SortDesc { shift; return $CALL->('SortDesc', @_) }

# Merge() -> null
# Sakura's F_MERGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Merge { shift; return $CALL->('Merge', @_) }

# Left() -> null
# Sakura's F_LEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Left { shift; return $CALL->('Left', @_) }

# Right() -> null
# Sakura's F_RIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Right { shift; return $CALL->('Right', @_) }

# Up2() -> null
# Sakura's F_UP2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Up2 { shift; return $CALL->('Up2', @_) }

# Down2() -> null
# Sakura's F_DOWN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Down2 { shift; return $CALL->('Down2', @_) }

# WordLeft() -> null
# Sakura's F_WORDLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WordLeft { shift; return $CALL->('WordLeft', @_) }

# WordRight() -> null
# Sakura's F_WORDRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WordRight { shift; return $CALL->('WordRight', @_) }

# GoLineTop(arg1: int?) -> null
# Sakura's F_GOLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub GoLineTop { shift; return $CALL->('GoLineTop', @_) }

# GoLineEnd(arg1: int?) -> null
# Sakura's F_GOLINEEND (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub GoLineEnd { shift; return $CALL->('GoLineEnd', @_) }

# HalfPageUp(arg1: int?) -> null
# Sakura's F_HalfPageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub HalfPageUp { shift; return $CALL->('HalfPageUp', @_) }

# HalfPageDown(arg1: int?) -> null
# Sakura's F_HalfPageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HalfPageDown { shift; return $CALL->('HalfPageDown', @_) }

# PageUp(arg1: int?) -> null
# Sakura's F_1PageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub PageUp { shift; return $CALL->('PageUp', @_) }

# PageDown(arg1: int?) -> null
# Sakura's F_1PageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub PageDown { shift; return $CALL->('PageDown', @_) }

# CurLineCenter() -> null
# Sakura's F_CURLINECENTER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CurLineCenter { shift; return $CALL->('CurLineCenter', @_) }

# CurLineTop() -> null
# Sakura's F_CURLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CurLineTop { shift; return $CALL->('CurLineTop', @_) }

# CurLineBottom() -> null
# Sakura's F_CURLINEBOTTOM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CurLineBottom { shift; return $CALL->('CurLineBottom', @_) }

# MoveHistPrev() -> null
# Sakura's F_JUMPHIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub MoveHistPrev { shift; return $CALL->('MoveHistPrev', @_) }

# MoveHistNext() -> null
# Sakura's F_JUMPHIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub MoveHistNext { shift; return $CALL->('MoveHistNext', @_) }

# MoveHistSet() -> null
# Sakura's F_JUMPHIST_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub MoveHistSet { shift; return $CALL->('MoveHistSet', @_) }

# F_WndScrollDown() -> null
# Sakura's F_WndScrollDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub F_WndScrollDown { shift; return $CALL->('F_WndScrollDown', @_) }

# F_WndScrollUp() -> null
# Sakura's F_WndScrollUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub F_WndScrollUp { shift; return $CALL->('F_WndScrollUp', @_) }

# GoNextParagraph() -> null
# Sakura's F_GONEXTPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoNextParagraph { shift; return $CALL->('GoNextParagraph', @_) }

# GoPrevParagraph() -> null
# Sakura's F_GOPREVPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoPrevParagraph { shift; return $CALL->('GoPrevParagraph', @_) }

# GoModifyLineNext() -> null
# Sakura's F_MODIFYLINE_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoModifyLineNext { shift; return $CALL->('GoModifyLineNext', @_) }

# GoModifyLinePrev() -> null
# Sakura's F_MODIFYLINE_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoModifyLinePrev { shift; return $CALL->('GoModifyLinePrev', @_) }

# MoveCursorLayout(arg1: int?, arg2: int?, arg3: int?) -> null
# MATSU Editor's own F_MOVECURSORLAYOUT (not in Sakura). Runs the command that a key, the
# menu and a key macro run.
sub MoveCursorLayout { shift; return $CALL->('MoveCursorLayout', @_) }

# WheelUp(arg1: int?) -> null
# Sakura's F_WHEELUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WheelUp { shift; return $CALL->('WheelUp', @_) }

# WheelDown(arg1: int?) -> null
# Sakura's F_WHEELDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WheelDown { shift; return $CALL->('WheelDown', @_) }

# WheelLeft(arg1: int?) -> null
# Sakura's F_WHEELLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WheelLeft { shift; return $CALL->('WheelLeft', @_) }

# WheelRight(arg1: int?) -> null
# Sakura's F_WHEELRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WheelRight { shift; return $CALL->('WheelRight', @_) }

# WheelPageUp(arg1: int?) -> null
# Sakura's F_WHEELPAGEUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WheelPageUp { shift; return $CALL->('WheelPageUp', @_) }

# WheelPageDown(arg1: int?) -> null
# Sakura's F_WHEELPAGEDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WheelPageDown { shift; return $CALL->('WheelPageDown', @_) }

# WheelPageLeft(arg1: int?) -> null
# Sakura's F_WHEELPAGELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WheelPageLeft { shift; return $CALL->('WheelPageLeft', @_) }

# WheelPageRight(arg1: int?) -> null
# Sakura's F_WHEELPAGERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WheelPageRight { shift; return $CALL->('WheelPageRight', @_) }

# SelectWord() -> null
# Sakura's F_SELECTWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub SelectWord { shift; return $CALL->('SelectWord', @_) }

# SelectLine(arg1: int?) -> null
# Sakura's F_SELECTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub SelectLine { shift; return $CALL->('SelectLine', @_) }

# BeginSelect() -> null
# Sakura's F_BEGIN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub BeginSelect { shift; return $CALL->('BeginSelect', @_) }

# Up_Sel() -> null
# Sakura's F_UP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Up_Sel { shift; return $CALL->('Up_Sel', @_) }

# Down_Sel() -> null
# Sakura's F_DOWN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Down_Sel { shift; return $CALL->('Down_Sel', @_) }

# Left_Sel() -> null
# Sakura's F_LEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Left_Sel { shift; return $CALL->('Left_Sel', @_) }

# Right_Sel() -> null
# Sakura's F_RIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Right_Sel { shift; return $CALL->('Right_Sel', @_) }

# Up2_Sel() -> null
# Sakura's F_UP2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Up2_Sel { shift; return $CALL->('Up2_Sel', @_) }

# Down2_Sel() -> null
# Sakura's F_DOWN2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Down2_Sel { shift; return $CALL->('Down2_Sel', @_) }

# WordLeft_Sel() -> null
# Sakura's F_WORDLEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WordLeft_Sel { shift; return $CALL->('WordLeft_Sel', @_) }

# WordRight_Sel() -> null
# Sakura's F_WORDRIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WordRight_Sel { shift; return $CALL->('WordRight_Sel', @_) }

# GoLineTop_Sel(arg1: int?) -> null
# Sakura's F_GOLINETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoLineTop_Sel { shift; return $CALL->('GoLineTop_Sel', @_) }

# GoLineEnd_Sel(arg1: int?) -> null
# Sakura's F_GOLINEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoLineEnd_Sel { shift; return $CALL->('GoLineEnd_Sel', @_) }

# HalfPageUp_Sel(arg1: int?) -> null
# Sakura's F_HalfPageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HalfPageUp_Sel { shift; return $CALL->('HalfPageUp_Sel', @_) }

# HalfPageDown_Sel(arg1: int?) -> null
# Sakura's F_HalfPageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HalfPageDown_Sel { shift; return $CALL->('HalfPageDown_Sel', @_) }

# PageUp_Sel(arg1: int?) -> null
# Sakura's F_1PageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub PageUp_Sel { shift; return $CALL->('PageUp_Sel', @_) }

# PageDown_Sel(arg1: int?) -> null
# Sakura's F_1PageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub PageDown_Sel { shift; return $CALL->('PageDown_Sel', @_) }

# GoFileTop_Sel() -> null
# Sakura's F_GOFILETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoFileTop_Sel { shift; return $CALL->('GoFileTop_Sel', @_) }

# GoFileEnd_Sel() -> null
# Sakura's F_GOFILEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoFileEnd_Sel { shift; return $CALL->('GoFileEnd_Sel', @_) }

# GoNextParagraph_Sel() -> null
# Sakura's F_GONEXTPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoNextParagraph_Sel { shift; return $CALL->('GoNextParagraph_Sel', @_) }

# GoPrevParagraph_Sel() -> null
# Sakura's F_GOPREVPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoPrevParagraph_Sel { shift; return $CALL->('GoPrevParagraph_Sel', @_) }

# GoModifyLineNext_Sel() -> null
# Sakura's F_MODIFYLINE_NEXT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoModifyLineNext_Sel { shift; return $CALL->('GoModifyLineNext_Sel', @_) }

# GoModifyLinePrev_Sel() -> null
# Sakura's F_MODIFYLINE_PREV_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoModifyLinePrev_Sel { shift; return $CALL->('GoModifyLinePrev_Sel', @_) }

# BeginBoxSelect() -> null
# Sakura's F_BEGIN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub BeginBoxSelect { shift; return $CALL->('BeginBoxSelect', @_) }

# Up_BoxSel(arg1: int?) -> null
# Sakura's F_UP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Up_BoxSel { shift; return $CALL->('Up_BoxSel', @_) }

# Down_BoxSel(arg1: int?) -> null
# Sakura's F_DOWN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Down_BoxSel { shift; return $CALL->('Down_BoxSel', @_) }

# Left_BoxSel(arg1: int?) -> null
# Sakura's F_LEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Left_BoxSel { shift; return $CALL->('Left_BoxSel', @_) }

# Right_BoxSel(arg1: int?) -> null
# Sakura's F_RIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Right_BoxSel { shift; return $CALL->('Right_BoxSel', @_) }

# Up2_BoxSel(arg1: int?) -> null
# Sakura's F_UP2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Up2_BoxSel { shift; return $CALL->('Up2_BoxSel', @_) }

# Down2_BoxSel(arg1: int?) -> null
# Sakura's F_DOWN2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Down2_BoxSel { shift; return $CALL->('Down2_BoxSel', @_) }

# WordLeft_BoxSel(arg1: int?) -> null
# Sakura's F_WORDLEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WordLeft_BoxSel { shift; return $CALL->('WordLeft_BoxSel', @_) }

# WordRight_BoxSel(arg1: int?) -> null
# Sakura's F_WORDRIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WordRight_BoxSel { shift; return $CALL->('WordRight_BoxSel', @_) }

# GoLogicalLineTop_BoxSel(arg1: int?, arg2: int?) -> null
# Sakura's F_GOLOGICALLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
# a key, the menu and a key macro run. The last arguments may be left out. A command that
# this editor does not have yet answers the error -32001 (not implemented).
sub GoLogicalLineTop_BoxSel { shift; return $CALL->('GoLogicalLineTop_BoxSel', @_) }

# GoLineTop_BoxSel(arg1: int?, arg2: int?) -> null
# Sakura's F_GOLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoLineTop_BoxSel { shift; return $CALL->('GoLineTop_BoxSel', @_) }

# GoLineEnd_BoxSel(arg1: int?, arg2: int?) -> null
# Sakura's F_GOLINEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoLineEnd_BoxSel { shift; return $CALL->('GoLineEnd_BoxSel', @_) }

# HalfPageUp_BoxSel(arg1: int?, arg2: int?) -> null
# Sakura's F_HalfPageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HalfPageUp_BoxSel { shift; return $CALL->('HalfPageUp_BoxSel', @_) }

# HalfPageDown_BoxSel(arg1: int?, arg2: int?) -> null
# Sakura's F_HalfPageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HalfPageDown_BoxSel { shift; return $CALL->('HalfPageDown_BoxSel', @_) }

# PageUp_BoxSel(arg1: int?, arg2: int?) -> null
# Sakura's F_1PageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub PageUp_BoxSel { shift; return $CALL->('PageUp_BoxSel', @_) }

# PageDown_BoxSel(arg1: int?, arg2: int?) -> null
# Sakura's F_1PageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub PageDown_BoxSel { shift; return $CALL->('PageDown_BoxSel', @_) }

# GoFileTop_BoxSel(arg1: int?) -> null
# Sakura's F_GOFILETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoFileTop_BoxSel { shift; return $CALL->('GoFileTop_BoxSel', @_) }

# GoFileEnd_BoxSel(arg1: int?) -> null
# Sakura's F_GOFILEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GoFileEnd_BoxSel { shift; return $CALL->('GoFileEnd_BoxSel', @_) }

# Cut() -> null
# Sakura's F_CUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Cut { shift; return $CALL->('Cut', @_) }

# Copy() -> null
# Sakura's F_COPY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Copy { shift; return $CALL->('Copy', @_) }

# Paste(arg1: int?) -> null
# Sakura's F_PASTE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Paste { shift; return $CALL->('Paste', @_) }

# CopyAddCRLF() -> null
# Sakura's F_COPY_ADDCRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CopyAddCRLF { shift; return $CALL->('CopyAddCRLF', @_) }

# CopyCRLF() -> null
# Sakura's F_COPY_CRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CopyCRLF { shift; return $CALL->('CopyCRLF', @_) }

# CopyLF() -> null
# MATSU Editor's own F_COPY_LF (not in Sakura). Runs the command that a key, the menu and a
# key macro run.
sub CopyLF { shift; return $CALL->('CopyLF', @_) }

# CopyCR() -> null
# MATSU Editor's own F_COPY_CR (not in Sakura). Runs the command that a key, the menu and a
# key macro run.
sub CopyCR { shift; return $CALL->('CopyCR', @_) }

# PasteBox(arg1: int?) -> null
# Sakura's F_PASTEBOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub PasteBox { shift; return $CALL->('PasteBox', @_) }

# InsBoxText(arg1: string?) -> null
# MATSU Editor's own F_INSBOXTEXT (not in Sakura). Runs the command that a key, the menu and
# a key macro run.
sub InsBoxText { shift; return $CALL->('InsBoxText', @_) }

# CopyLines() -> null
# Sakura's F_COPYLINES (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CopyLines { shift; return $CALL->('CopyLines', @_) }

# CopyLinesAsPassage() -> null
# Sakura's F_COPYLINESASPASSAGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CopyLinesAsPassage { shift; return $CALL->('CopyLinesAsPassage', @_) }

# CopyLinesWithLineNumber() -> null
# Sakura's F_COPYLINESWITHLINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
# that a key, the menu and a key macro run. The last arguments may be left out. A command
# that this editor does not have yet answers the error -32001 (not implemented).
sub CopyLinesWithLineNumber { shift; return $CALL->('CopyLinesWithLineNumber', @_) }

# CopyColorHtml() -> null
# Sakura's F_COPY_COLOR_HTML (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CopyColorHtml { shift; return $CALL->('CopyColorHtml', @_) }

# CopyColorHtmlWithLineNumber() -> null
# Sakura's F_COPY_COLOR_HTML_LINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
# that a key, the menu and a key macro run. The last arguments may be left out. A command
# that this editor does not have yet answers the error -32001 (not implemented).
sub CopyColorHtmlWithLineNumber { shift; return $CALL->('CopyColorHtmlWithLineNumber', @_) }

# CopyPath() -> null
# Sakura's F_COPYPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CopyPath { shift; return $CALL->('CopyPath', @_) }

# CopyDirPath() -> null
# Sakura's F_COPYDIRPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CopyDirPath { shift; return $CALL->('CopyDirPath', @_) }

# CopyFilename() -> null
# Sakura's F_COPYFNAME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CopyFilename { shift; return $CALL->('CopyFilename', @_) }

# CopyTag() -> null
# Sakura's F_COPYTAG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CopyTag { shift; return $CALL->('CopyTag', @_) }

# CopyKeyBindList() -> null
# Sakura's F_CREATEKEYBINDLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CopyKeyBindList { shift; return $CALL->('CopyKeyBindList', @_) }

# InsertDate() -> null
# Sakura's F_INS_DATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub InsertDate { shift; return $CALL->('InsertDate', @_) }

# InsertTime() -> null
# Sakura's F_INS_TIME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub InsertTime { shift; return $CALL->('InsertTime', @_) }

# CtrlCodeDialog() -> null
# Sakura's F_CTRL_CODE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CtrlCodeDialog { shift; return $CALL->('CtrlCodeDialog', @_) }

# CtrlCode(arg1: int?) -> null
# Sakura's F_CTRL_CODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CtrlCode { shift; return $CALL->('CtrlCode', @_) }

# InsertFileUsedRecently() -> null
# Sakura's F_INS_FILE_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
# that a key, the menu and a key macro run. The last arguments may be left out. A command
# that this editor does not have yet answers the error -32001 (not implemented).
sub InsertFileUsedRecently { shift; return $CALL->('InsertFileUsedRecently', @_) }

# InsertFolderUsedRecently() -> null
# Sakura's F_INS_FOLDER_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
# that a key, the menu and a key macro run. The last arguments may be left out. A command
# that this editor does not have yet answers the error -32001 (not implemented).
sub InsertFolderUsedRecently { shift; return $CALL->('InsertFolderUsedRecently', @_) }

# ToLower() -> null
# Sakura's F_TOLOWER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ToLower { shift; return $CALL->('ToLower', @_) }

# ToUpper() -> null
# Sakura's F_TOUPPER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ToUpper { shift; return $CALL->('ToUpper', @_) }

# ToHankaku() -> null
# Sakura's F_TOHANKAKU (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ToHankaku { shift; return $CALL->('ToHankaku', @_) }

# ToHankata() -> null
# Sakura's F_TOHANKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ToHankata { shift; return $CALL->('ToHankata', @_) }

# ToZenEi() -> null
# Sakura's F_TOZENEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ToZenEi { shift; return $CALL->('ToZenEi', @_) }

# ToHanEi() -> null
# Sakura's F_TOHANEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ToHanEi { shift; return $CALL->('ToHanEi', @_) }

# ToZenKata() -> null
# Sakura's F_TOZENKAKUKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ToZenKata { shift; return $CALL->('ToZenKata', @_) }

# ToZenHira() -> null
# Sakura's F_TOZENKAKUHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ToZenHira { shift; return $CALL->('ToZenHira', @_) }

# HanKataToZenKata() -> null
# Sakura's F_HANKATATOZENKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HanKataToZenKata { shift; return $CALL->('HanKataToZenKata', @_) }

# HanKataToZenHira() -> null
# Sakura's F_HANKATATOZENHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HanKataToZenHira { shift; return $CALL->('HanKataToZenHira', @_) }

# TABToSPACE() -> null
# Sakura's F_TABTOSPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub TABToSPACE { shift; return $CALL->('TABToSPACE', @_) }

# SPACEToTAB() -> null
# Sakura's F_SPACETOTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub SPACEToTAB { shift; return $CALL->('SPACEToTAB', @_) }

# AutoToSJIS() -> null
# Sakura's F_CODECNV_AUTO2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub AutoToSJIS { shift; return $CALL->('AutoToSJIS', @_) }

# JIStoSJIS() -> null
# Sakura's F_CODECNV_EMAIL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub JIStoSJIS { shift; return $CALL->('JIStoSJIS', @_) }

# EUCtoSJIS() -> null
# Sakura's F_CODECNV_EUC2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub EUCtoSJIS { shift; return $CALL->('EUCtoSJIS', @_) }

# CodeCnvUNICODEtoSJIS() -> null
# Sakura's F_CODECNV_UNICODE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
# a key, the menu and a key macro run. The last arguments may be left out. A command that
# this editor does not have yet answers the error -32001 (not implemented).
sub CodeCnvUNICODEtoSJIS { shift; return $CALL->('CodeCnvUNICODEtoSJIS', @_) }

# CodeCnvUNICODEBEtoSJIS() -> null
# Sakura's F_CODECNV_UNICODEBE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
# that a key, the menu and a key macro run. The last arguments may be left out. A command
# that this editor does not have yet answers the error -32001 (not implemented).
sub CodeCnvUNICODEBEtoSJIS { shift; return $CALL->('CodeCnvUNICODEBEtoSJIS', @_) }

# UTF8toSJIS() -> null
# Sakura's F_CODECNV_UTF82SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub UTF8toSJIS { shift; return $CALL->('UTF8toSJIS', @_) }

# UTF7toSJIS() -> null
# Sakura's F_CODECNV_UTF72SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub UTF7toSJIS { shift; return $CALL->('UTF7toSJIS', @_) }

# SJIStoJIS() -> null
# Sakura's F_CODECNV_SJIS2JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SJIStoJIS { shift; return $CALL->('SJIStoJIS', @_) }

# SJIStoEUC() -> null
# Sakura's F_CODECNV_SJIS2EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SJIStoEUC { shift; return $CALL->('SJIStoEUC', @_) }

# SJIStoUTF8() -> null
# Sakura's F_CODECNV_SJIS2UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SJIStoUTF8 { shift; return $CALL->('SJIStoUTF8', @_) }

# SJIStoUTF7() -> null
# Sakura's F_CODECNV_SJIS2UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SJIStoUTF7 { shift; return $CALL->('SJIStoUTF7', @_) }

# Base64Decode() -> null
# Sakura's F_BASE64DECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub Base64Decode { shift; return $CALL->('Base64Decode', @_) }

# Uudecode() -> null
# Sakura's F_UUDECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Uudecode { shift; return $CALL->('Uudecode', @_) }

# SearchDialog() -> null
# Sakura's F_SEARCH_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SearchDialog { shift; return $CALL->('SearchDialog', @_) }

# SearchNext(arg1: string?, arg2: int?) -> null
# Sakura's F_SEARCH_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SearchNext { shift; return $CALL->('SearchNext', @_) }

# SearchPrev(arg1: string?, arg2: int?) -> null
# Sakura's F_SEARCH_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SearchPrev { shift; return $CALL->('SearchPrev', @_) }

# ReplaceDialog() -> null
# Sakura's F_REPLACE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ReplaceDialog { shift; return $CALL->('ReplaceDialog', @_) }

# Replace(arg1: string?, arg2: string?, arg3: int?) -> null
# Sakura's F_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Replace { shift; return $CALL->('Replace', @_) }

# ReplaceAll(arg1: string?, arg2: string?, arg3: int?) -> null
# Sakura's F_REPLACE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ReplaceAll { shift; return $CALL->('ReplaceAll', @_) }

# SearchClearMark() -> null
# Sakura's F_SEARCH_CLEARMARK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SearchClearMark { shift; return $CALL->('SearchClearMark', @_) }

# SearchStartPos() -> null
# Sakura's F_JUMP_SRCHSTARTPOS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SearchStartPos { shift; return $CALL->('SearchStartPos', @_) }

# Grep(arg1: string?, arg2: string?, arg3: string?, arg4: int?, arg5: int?) -> null
# Sakura's F_GREP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Grep { shift; return $CALL->('Grep', @_) }

# GrepReplace(arg1: string?, arg2: string?, arg3: string?, arg4: string?, arg5: int?, arg6: int?) -> null
# Sakura's F_GREP_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub GrepReplace { shift; return $CALL->('GrepReplace', @_) }

# Outline(arg1: int?) -> null
# Sakura's F_OUTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Outline { shift; return $CALL->('Outline', @_) }

# TagJump() -> null
# Sakura's F_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub TagJump { shift; return $CALL->('TagJump', @_) }

# TagJumpBack() -> null
# Sakura's F_TAGJUMPBACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TagJumpBack { shift; return $CALL->('TagJumpBack', @_) }

# TagMake() -> null
# Sakura's F_TAGS_MAKE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub TagMake { shift; return $CALL->('TagMake', @_) }

# DirectTagJump() -> null
# Sakura's F_DIRECT_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub DirectTagJump { shift; return $CALL->('DirectTagJump', @_) }

# KeywordTagJump(arg1: string?) -> null
# Sakura's F_TAGJUMP_KEYWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub KeywordTagJump { shift; return $CALL->('KeywordTagJump', @_) }

# Compare() -> null
# Sakura's F_COMPARE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub Compare { shift; return $CALL->('Compare', @_) }

# DiffDialog() -> null
# Sakura's F_DIFF_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub DiffDialog { shift; return $CALL->('DiffDialog', @_) }

# Diff(arg1: string?, arg2: int?) -> null
# Sakura's F_DIFF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Diff { shift; return $CALL->('Diff', @_) }

# DiffNext() -> null
# Sakura's F_DIFF_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub DiffNext { shift; return $CALL->('DiffNext', @_) }

# DiffPrev() -> null
# Sakura's F_DIFF_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub DiffPrev { shift; return $CALL->('DiffPrev', @_) }

# DiffReset() -> null
# Sakura's F_DIFF_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub DiffReset { shift; return $CALL->('DiffReset', @_) }

# BracketPair() -> null
# Sakura's F_BRACKETPAIR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub BracketPair { shift; return $CALL->('BracketPair', @_) }

# BookmarkSet() -> null
# Sakura's F_BOOKMARK_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub BookmarkSet { shift; return $CALL->('BookmarkSet', @_) }

# BookmarkNext() -> null
# Sakura's F_BOOKMARK_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub BookmarkNext { shift; return $CALL->('BookmarkNext', @_) }

# BookmarkPrev() -> null
# Sakura's F_BOOKMARK_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub BookmarkPrev { shift; return $CALL->('BookmarkPrev', @_) }

# BookmarkReset() -> null
# Sakura's F_BOOKMARK_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub BookmarkReset { shift; return $CALL->('BookmarkReset', @_) }

# BookmarkView() -> null
# Sakura's F_BOOKMARK_VIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub BookmarkView { shift; return $CALL->('BookmarkView', @_) }

# BookmarkPattern(arg1: string?, arg2: int?) -> null
# Sakura's F_BOOKMARK_PATTERN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub BookmarkPattern { shift; return $CALL->('BookmarkPattern', @_) }

# FuncListNext() -> null
# Sakura's F_FUNCLIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FuncListNext { shift; return $CALL->('FuncListNext', @_) }

# FuncListPrev() -> null
# Sakura's F_FUNCLIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub FuncListPrev { shift; return $CALL->('FuncListPrev', @_) }

# ChgmodINS() -> null
# Sakura's F_CHGMOD_INS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ChgmodINS { shift; return $CALL->('ChgmodINS', @_) }

# ChgCharSet(arg1: int?, arg2: int?) -> null
# Sakura's F_CHG_CHARSET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ChgCharSet { shift; return $CALL->('ChgCharSet', @_) }

# ChgmodEOL(arg1: int?) -> null
# MATSU Editor's own F_CHGMOD_EOL (not in Sakura). Runs the command that a key, the menu and
# a key macro run.
sub ChgmodEOL { shift; return $CALL->('ChgmodEOL', @_) }

# CancelMode() -> null
# Sakura's F_CANCEL_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CancelMode { shift; return $CALL->('CancelMode', @_) }

# ExecExternalMacro(arg1: string?, arg2: string?) -> null
# Sakura's F_EXECEXTMACRO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ExecExternalMacro { shift; return $CALL->('ExecExternalMacro', @_) }

# ShowToolbar() -> null
# Sakura's F_SHOWTOOLBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ShowToolbar { shift; return $CALL->('ShowToolbar', @_) }

# ShowFunckey() -> null
# Sakura's F_SHOWFUNCKEY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ShowFunckey { shift; return $CALL->('ShowFunckey', @_) }

# ShowTab() -> null
# Sakura's F_SHOWTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ShowTab { shift; return $CALL->('ShowTab', @_) }

# ShowStatusbar() -> null
# Sakura's F_SHOWSTATUSBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ShowStatusbar { shift; return $CALL->('ShowStatusbar', @_) }

# ShowMiniMap() -> null
# Sakura's F_SHOWMINIMAP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ShowMiniMap { shift; return $CALL->('ShowMiniMap', @_) }

# TypeList() -> null
# Sakura's F_TYPE_LIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub TypeList { shift; return $CALL->('TypeList', @_) }

# ChangeType(arg1: int?) -> null
# MATSU Editor's own F_CHANGETYPE (not in Sakura). Runs the command that a key, the menu and
# a key macro run.
sub ChangeType { shift; return $CALL->('ChangeType', @_) }

# OptionType() -> null
# Sakura's F_OPTION_TYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub OptionType { shift; return $CALL->('OptionType', @_) }

# OptionCommon() -> null
# Sakura's F_OPTION (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub OptionCommon { shift; return $CALL->('OptionCommon', @_) }

# SelectFont() -> null
# Sakura's F_FONT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub SelectFont { shift; return $CALL->('SelectFont', @_) }

# SetFontSize(arg1: int?, arg2: int?, arg3: int?) -> null
# Sakura's F_SETFONTSIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SetFontSize { shift; return $CALL->('SetFontSize', @_) }

# WrapWindowWidth() -> null
# Sakura's F_WRAPWINDOWWIDTH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WrapWindowWidth { shift; return $CALL->('WrapWindowWidth', @_) }

# OptionFavorite() -> null
# Sakura's F_FAVORITE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub OptionFavorite { shift; return $CALL->('OptionFavorite', @_) }

# SetMsgQuoteStr(arg1: string?) -> null
# MATSU Editor's own F_SET_QUOTESTRING (not in Sakura). Runs the command that a key, the
# menu and a key macro run.
sub SetMsgQuoteStr { shift; return $CALL->('SetMsgQuoteStr', @_) }

# TextWrapMethod(arg1: int?) -> null
# MATSU Editor's own F_TEXTWRAPMETHOD (not in Sakura). Runs the command that a key, the menu
# and a key macro run.
sub TextWrapMethod { shift; return $CALL->('TextWrapMethod', @_) }

# SelectCountMode(arg1: int?) -> null
# Sakura's F_SELECT_COUNT_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub SelectCountMode { shift; return $CALL->('SelectCountMode', @_) }

# ExecCommand(arg1: string?, arg2: int?, arg3: string?) -> null
# Sakura's F_EXECMD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ExecCommand { shift; return $CALL->('ExecCommand', @_) }

# ExecCommandDialog() -> null
# Sakura's F_EXECMD_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ExecCommandDialog { shift; return $CALL->('ExecCommandDialog', @_) }

# RMenu() -> null
# Sakura's F_MENU_RBUTTON (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub RMenu { shift; return $CALL->('RMenu', @_) }

# CustMenu1() -> null
# Sakura's F_CUSTMENU_1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu1 { shift; return $CALL->('CustMenu1', @_) }

# CustMenu2() -> null
# Sakura's F_CUSTMENU_2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu2 { shift; return $CALL->('CustMenu2', @_) }

# CustMenu3() -> null
# Sakura's F_CUSTMENU_3 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu3 { shift; return $CALL->('CustMenu3', @_) }

# CustMenu4() -> null
# Sakura's F_CUSTMENU_4 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu4 { shift; return $CALL->('CustMenu4', @_) }

# CustMenu5() -> null
# Sakura's F_CUSTMENU_5 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu5 { shift; return $CALL->('CustMenu5', @_) }

# CustMenu6() -> null
# Sakura's F_CUSTMENU_6 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu6 { shift; return $CALL->('CustMenu6', @_) }

# CustMenu7() -> null
# Sakura's F_CUSTMENU_7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu7 { shift; return $CALL->('CustMenu7', @_) }

# CustMenu8() -> null
# Sakura's F_CUSTMENU_8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu8 { shift; return $CALL->('CustMenu8', @_) }

# CustMenu9() -> null
# Sakura's F_CUSTMENU_9 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CustMenu9 { shift; return $CALL->('CustMenu9', @_) }

# CustMenu10() -> null
# Sakura's F_CUSTMENU_10 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu10 { shift; return $CALL->('CustMenu10', @_) }

# CustMenu11() -> null
# Sakura's F_CUSTMENU_11 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu11 { shift; return $CALL->('CustMenu11', @_) }

# CustMenu12() -> null
# Sakura's F_CUSTMENU_12 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu12 { shift; return $CALL->('CustMenu12', @_) }

# CustMenu13() -> null
# Sakura's F_CUSTMENU_13 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu13 { shift; return $CALL->('CustMenu13', @_) }

# CustMenu14() -> null
# Sakura's F_CUSTMENU_14 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu14 { shift; return $CALL->('CustMenu14', @_) }

# CustMenu15() -> null
# Sakura's F_CUSTMENU_15 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu15 { shift; return $CALL->('CustMenu15', @_) }

# CustMenu16() -> null
# Sakura's F_CUSTMENU_16 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu16 { shift; return $CALL->('CustMenu16', @_) }

# CustMenu17() -> null
# Sakura's F_CUSTMENU_17 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu17 { shift; return $CALL->('CustMenu17', @_) }

# CustMenu18() -> null
# Sakura's F_CUSTMENU_18 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu18 { shift; return $CALL->('CustMenu18', @_) }

# CustMenu19() -> null
# Sakura's F_CUSTMENU_19 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu19 { shift; return $CALL->('CustMenu19', @_) }

# CustMenu20() -> null
# Sakura's F_CUSTMENU_20 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu20 { shift; return $CALL->('CustMenu20', @_) }

# CustMenu21() -> null
# Sakura's F_CUSTMENU_21 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu21 { shift; return $CALL->('CustMenu21', @_) }

# CustMenu22() -> null
# Sakura's F_CUSTMENU_22 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu22 { shift; return $CALL->('CustMenu22', @_) }

# CustMenu23() -> null
# Sakura's F_CUSTMENU_23 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu23 { shift; return $CALL->('CustMenu23', @_) }

# CustMenu24() -> null
# Sakura's F_CUSTMENU_24 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CustMenu24 { shift; return $CALL->('CustMenu24', @_) }

# SplitWinV() -> null
# Sakura's F_SPLIT_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub SplitWinV { shift; return $CALL->('SplitWinV', @_) }

# SplitWinH() -> null
# Sakura's F_SPLIT_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub SplitWinH { shift; return $CALL->('SplitWinH', @_) }

# SplitWinVH() -> null
# Sakura's F_SPLIT_VH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub SplitWinVH { shift; return $CALL->('SplitWinVH', @_) }

# WinMaximize() -> null
# MATSU Editor's own F_WINMAXIMIZE (not in Sakura). Runs the command that a key, the menu
# and a key macro run.
sub WinMaximize { shift; return $CALL->('WinMaximize', @_) }

# WinMinimize() -> null
# MATSU Editor's own F_WINMINIMIZE (not in Sakura). Runs the command that a key, the menu
# and a key macro run.
sub WinMinimize { shift; return $CALL->('WinMinimize', @_) }

# WinRestore() -> null
# MATSU Editor's own F_WINRESTORE (not in Sakura). Runs the command that a key, the menu and
# a key macro run.
sub WinRestore { shift; return $CALL->('WinRestore', @_) }

# WinClose() -> null
# Sakura's F_WINCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WinClose { shift; return $CALL->('WinClose', @_) }

# WinCloseAll() -> null
# Sakura's F_WIN_CLOSEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub WinCloseAll { shift; return $CALL->('WinCloseAll', @_) }

# CascadeWin() -> null
# Sakura's F_CASCADE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub CascadeWin { shift; return $CALL->('CascadeWin', @_) }

# TileWinV() -> null
# Sakura's F_TILE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub TileWinV { shift; return $CALL->('TileWinV', @_) }

# TileWinH() -> null
# Sakura's F_TILE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub TileWinH { shift; return $CALL->('TileWinH', @_) }

# NextWindow() -> null
# Sakura's F_NEXTWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub NextWindow { shift; return $CALL->('NextWindow', @_) }

# PrevWindow() -> null
# Sakura's F_PREVWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub PrevWindow { shift; return $CALL->('PrevWindow', @_) }

# WindowList() -> null
# Sakura's F_WINLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WindowList { shift; return $CALL->('WindowList', @_) }

# MaximizeV() -> null
# Sakura's F_MAXIMIZE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub MaximizeV { shift; return $CALL->('MaximizeV', @_) }

# MaximizeH() -> null
# Sakura's F_MAXIMIZE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub MaximizeH { shift; return $CALL->('MaximizeH', @_) }

# MinimizeAll() -> null
# Sakura's F_MINIMIZE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub MinimizeAll { shift; return $CALL->('MinimizeAll', @_) }

# ReDraw() -> null
# Sakura's F_REDRAW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ReDraw { shift; return $CALL->('ReDraw', @_) }

# ActivateWinOutput() -> null
# Sakura's F_WIN_OUTPUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ActivateWinOutput { shift; return $CALL->('ActivateWinOutput', @_) }

# PreviewTab() -> null
# MATSU Editor's own F_PREVIEW_TAB (not in Sakura). Runs the command that a key, the menu
# and a key macro run.
sub PreviewTab { shift; return $CALL->('PreviewTab', @_) }

# TraceOut(arg1: string?, arg2: int?) -> null
# MATSU Editor's own F_TRACEOUT (not in Sakura). Runs the command that a key, the menu and a
# key macro run.
sub TraceOut { shift; return $CALL->('TraceOut', @_) }

# WindowTopMost(arg1: int?) -> null
# Sakura's F_TOPMOST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub WindowTopMost { shift; return $CALL->('WindowTopMost', @_) }

# GroupClose() -> null
# Sakura's F_GROUPCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub GroupClose { shift; return $CALL->('GroupClose', @_) }

# NextGroup() -> null
# Sakura's F_NEXTGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub NextGroup { shift; return $CALL->('NextGroup', @_) }

# PrevGroup() -> null
# Sakura's F_PREVGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub PrevGroup { shift; return $CALL->('PrevGroup', @_) }

# TabMoveRight() -> null
# Sakura's F_TAB_MOVERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TabMoveRight { shift; return $CALL->('TabMoveRight', @_) }

# TabMoveLeft() -> null
# Sakura's F_TAB_MOVELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TabMoveLeft { shift; return $CALL->('TabMoveLeft', @_) }

# TabSeparate() -> null
# Sakura's F_TAB_SEPARATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TabSeparate { shift; return $CALL->('TabSeparate', @_) }

# TabJointNext() -> null
# Sakura's F_TAB_JOINTNEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TabJointNext { shift; return $CALL->('TabJointNext', @_) }

# TabJointPrev() -> null
# Sakura's F_TAB_JOINTPREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TabJointPrev { shift; return $CALL->('TabJointPrev', @_) }

# TabCloseOther() -> null
# Sakura's F_TAB_CLOSEOTHER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TabCloseOther { shift; return $CALL->('TabCloseOther', @_) }

# TabCloseLeft() -> null
# Sakura's F_TAB_CLOSELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TabCloseLeft { shift; return $CALL->('TabCloseLeft', @_) }

# TabCloseRight() -> null
# Sakura's F_TAB_CLOSERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub TabCloseRight { shift; return $CALL->('TabCloseRight', @_) }

# Complete() -> null
# Sakura's F_HOKAN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub Complete { shift; return $CALL->('Complete', @_) }

# ToggleKeyHelpSearch(arg1: int?) -> null
# Sakura's F_TOGGLE_KEY_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
# key, the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ToggleKeyHelpSearch { shift; return $CALL->('ToggleKeyHelpSearch', @_) }

# HelpContents() -> null
# Sakura's F_HELP_CONTENTS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HelpContents { shift; return $CALL->('HelpContents', @_) }

# HelpSearch() -> null
# Sakura's F_HELP_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub HelpSearch { shift; return $CALL->('HelpSearch', @_) }

# CommandList() -> null
# Sakura's F_MENU_ALLFUNC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub CommandList { shift; return $CALL->('CommandList', @_) }

# ExtHelp1() -> null
# Sakura's F_EXTHELP1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
# menu and a key macro run. The last arguments may be left out. A command that this editor
# does not have yet answers the error -32001 (not implemented).
sub ExtHelp1 { shift; return $CALL->('ExtHelp1', @_) }

# ExtHtmlHelp(arg1: string?, arg2: string?) -> null
# Sakura's F_EXTHTMLHELP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
# the menu and a key macro run. The last arguments may be left out. A command that this
# editor does not have yet answers the error -32001 (not implemented).
sub ExtHtmlHelp { shift; return $CALL->('ExtHtmlHelp', @_) }

# About() -> null
# Sakura's F_ABOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
# and a key macro run. The last arguments may be left out. A command that this editor does
# not have yet answers the error -32001 (not implemented).
sub About { shift; return $CALL->('About', @_) }

# StatusMsg(arg1: string?, arg2: int?) -> null
# MATSU Editor's own F_STATUSMSG (not in Sakura). Runs the command that a key, the menu and
# a key macro run.
sub StatusMsg { shift; return $CALL->('StatusMsg', @_) }

# MsgBeep(arg1: int?) -> null
# MATSU Editor's own F_MSGBEEP (not in Sakura). Runs the command that a key, the menu and a
# key macro run.
sub MsgBeep { shift; return $CALL->('MsgBeep', @_) }

# CommitUndoBuffer() -> null
# MATSU Editor's own F_COMMITUNDOBUFFER (not in Sakura). Runs the command that a key, the
# menu and a key macro run.
sub CommitUndoBuffer { shift; return $CALL->('CommitUndoBuffer', @_) }

# AddRefUndoBuffer() -> null
# MATSU Editor's own F_ADDREFUNDOBUFFER (not in Sakura). Runs the command that a key, the
# menu and a key macro run.
sub AddRefUndoBuffer { shift; return $CALL->('AddRefUndoBuffer', @_) }

# SetUndoBuffer() -> null
# MATSU Editor's own F_SETUNDOBUFFER (not in Sakura). Runs the command that a key, the menu
# and a key macro run.
sub SetUndoBuffer { shift; return $CALL->('SetUndoBuffer', @_) }

# AppendUndoBufferCursor() -> null
# MATSU Editor's own F_APPENDUNDOBUFFERCURSOR (not in Sakura). Runs the command that a key,
# the menu and a key macro run.
sub AppendUndoBufferCursor { shift; return $CALL->('AppendUndoBufferCursor', @_) }

# ClipboardEmpty() -> null
# MATSU Editor's own F_CLIPBOARDEMPTY (not in Sakura). Runs the command that a key, the menu
# and a key macro run.
sub ClipboardEmpty { shift; return $CALL->('ClipboardEmpty', @_) }

# SetViewTop(arg1: int?) -> null
# MATSU Editor's own F_SETVIEWTOP (not in Sakura). Runs the command that a key, the menu and
# a key macro run.
sub SetViewTop { shift; return $CALL->('SetViewTop', @_) }

# SetViewLeft(arg1: int?) -> null
# MATSU Editor's own F_SETVIEWLEFT (not in Sakura). Runs the command that a key, the menu
# and a key macro run.
sub SetViewLeft { shift; return $CALL->('SetViewLeft', @_) }

# Any other name of Sakura's macro functions, including those that start with a digit
# (Editor->Call('1PageUp')). The editor answers a name it does not know with an error.
sub Call { shift; my $name = shift; return $CALL->($name, @_) }

# Any other name that starts like a Sakura macro name (an upper case letter or a digit) is sent
# to the editor as it is.
sub AUTOLOAD {
    my $class = shift;
    (my $name = $AUTOLOAD) =~ s/.*:://;
    return if $name eq 'DESTROY';
    die "Can't locate object method \"$name\" via package \"Editor\"\n" unless $name =~ /^[0-9A-Z]/;
    return $CALL->($name, @_);
}

1;
