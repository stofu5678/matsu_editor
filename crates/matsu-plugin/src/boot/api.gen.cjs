// GENERATED from crates/matsu-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: MATSU_UPDATE_GENERATED=1 cargo test -p matsu-plugin generated_files
'use strict';

module.exports = function install(call) {
  const editor = {
    /**
     * GetLineCount(reserved: int) -> int
     *
     * The number of logical lines (lines separated by line breaks). The text after the last line
     * break is not a line unless it is not empty. The argument is reserved and must be 0.
     */
    GetLineCount(reserved) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetLineCount', args);
    },
    /**
     * GetLineStr(line: int) -> string
     *
     * A logical line with its line break ("\n"; the last line has none unless the text ends with
     * one). 0 is the line of the cursor, 1 is the first line. Past the last line it is an empty
     * string.
     */
    GetLineStr(line) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetLineStr', args);
    },
    /**
     * InsText(text: string) -> null
     *
     * Inserts the text at the cursor, replacing the selection. The cursor ends after the text.
     */
    InsText(text) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InsText', args);
    },
    /**
     * AddTail(text: string) -> null
     *
     * Moves to the end of the file, drops the selection, and inserts the text there.
     */
    AddTail(text) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('AddTail', args);
    },
    /**
     * GoFileTop() -> null
     *
     * Moves the cursor to the start of the file and drops the selection.
     */
    GoFileTop() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoFileTop', args);
    },
    /**
     * GoFileEnd() -> null
     *
     * Moves the cursor to the end of the file and drops the selection.
     */
    GoFileEnd() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoFileEnd', args);
    },
    /**
     * Up() -> null
     *
     * Moves the cursor up one line and drops the selection.
     */
    Up() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Up', args);
    },
    /**
     * Down() -> null
     *
     * Moves the cursor down one line and drops the selection.
     */
    Down() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Down', args);
    },
    /**
     * SelectAll() -> null
     *
     * Selects the whole text. The cursor goes to the end.
     */
    SelectAll() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SelectAll', args);
    },
    /**
     * GetSelectedString(reserved: int) -> string
     *
     * The selected text with its line breaks ("\n"), or an empty string when nothing is
     * selected. The argument is reserved and ignored.
     */
    GetSelectedString(reserved) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSelectedString', args);
    },
    /**
     * IsTextSelected() -> int
     *
     * 0 when nothing is selected, 1 when text is selected, 2 when a box selection is selected.
     */
    IsTextSelected() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsTextSelected', args);
    },
    /**
     * MoveCursor(line: int, column: int, option: int) -> null
     *
     * Moves the cursor to a line and a column, both counted from 1. The column counts UTF-16
     * code units. Option: 0 does not select, 1 selects from the old position. (2, the box
     * selection, is not supported.) Past the end of the text it goes to the end of the text.
     */
    MoveCursor(line, column, option) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MoveCursor', args);
    },
    /**
     * Jump(line: int, flag: int?) -> null
     *
     * Moves to the start of a line, counted from 1 in logical lines. Lines before 1 and after
     * the last one are limited to the first and the last line. Flag bit 0 (1, the default) is
     * the line break unit, which is the only unit supported yet; bit 1 (PL/SQL) is not
     * supported.
     */
    Jump(line, flag) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Jump', args);
    },
    /**
     * GetFilename() -> string
     *
     * The full path of the file, or an empty string when the text has no file.
     */
    GetFilename() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetFilename', args);
    },
    /**
     * GetSelectLineFrom() -> int
     *
     * The line where the selection starts, counted from 1 in layout lines (wrapped rows count as
     * lines). 0 when nothing is selected.
     */
    GetSelectLineFrom() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSelectLineFrom', args);
    },
    /**
     * GetSelectColmFrom() -> int
     *
     * The column where the selection starts, counted from 1 in layout columns (cells: a
     * full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected. Same
     * as GetSelectColumnFrom.
     */
    GetSelectColmFrom() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSelectColmFrom', args);
    },
    /**
     * GetSelectColumnFrom() -> int
     *
     * The column where the selection starts, counted from 1 in layout columns (cells: a
     * full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected.
     */
    GetSelectColumnFrom() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSelectColumnFrom', args);
    },
    /**
     * GetSelectLineTo() -> int
     *
     * The line where the selection ends, counted from 1 in layout lines. 0 when nothing is
     * selected.
     */
    GetSelectLineTo() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSelectLineTo', args);
    },
    /**
     * GetSelectColmTo() -> int
     *
     * The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
     * selected. Same as GetSelectColumnTo.
     */
    GetSelectColmTo() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSelectColmTo', args);
    },
    /**
     * GetSelectColumnTo() -> int
     *
     * The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
     * selected.
     */
    GetSelectColumnTo() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSelectColumnTo', args);
    },
    /**
     * IsInsMode() -> int
     *
     * 1 in insert mode, 0 in overwrite mode.
     */
    IsInsMode() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsInsMode', args);
    },
    /**
     * GetCharCode() -> int
     *
     * The encoding of the file: 0 Shift_JIS, 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, 5 UTF-7, 6
     * UTF-16 BE, 7 CESU-8, 8 Latin1.
     */
    GetCharCode() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetCharCode', args);
    },
    /**
     * GetLineCode() -> int
     *
     * The line ending that new line breaks get: 0 CR LF, 1 CR, 2 LF (3 NEL, 4 LS, 5 PS).
     */
    GetLineCode() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetLineCode', args);
    },
    /**
     * IsPossibleUndo() -> int
     *
     * 1 when there is something to undo, else 0.
     */
    IsPossibleUndo() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsPossibleUndo', args);
    },
    /**
     * IsPossibleRedo() -> int
     *
     * 1 when there is something to redo, else 0.
     */
    IsPossibleRedo() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsPossibleRedo', args);
    },
    /**
     * GetLineAttribute(line: int, type: int) -> int
     *
     * An attribute of a logical line (0 is the line of the cursor, 1 the first line). Type 0 is
     * 1 when the line changed since the last save, type 1 the number of the undo step that last
     * touched it, type 2 the bookmark (1 or 0). Types 3 (diff mark) and 4 (function list mark)
     * answer the error -32001 until the editor has them. A negative line, a line past the end
     * and any other type are errors.
     */
    GetLineAttribute(line, type) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetLineAttribute', args);
    },
    /**
     * IsTextSelectingLock() -> int
     *
     * 0 when the selection is not locked, 1 when it is locked (plain moves extend the
     * selection), 2 when it is locked and a box selection.
     */
    IsTextSelectingLock() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsTextSelectingLock', args);
    },
    /**
     * GetSaveFilename() -> string
     *
     * The full path that a save writes to, or an empty string when the text has no file. The
     * same as GetFilename, except in the macro that runs before a save (the registered macro of
     * 保存前), where it is the file being saved.
     */
    GetSaveFilename() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSaveFilename', args);
    },
    /**
     * ChangeInsertSpace(value: int) -> int
     *
     * Whether the Tab key inserts spaces instead of a tab character, before the call (1 or 0). 0
     * turns it off, 1 turns it on for this window (not saved); any other value only reads. Only
     * the low 16 bits of the value count, as in Sakura.
     */
    ChangeInsertSpace(value) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeInsertSpace', args);
    },
    /**
     * LayoutToLogicLineNum(layoutLine: int) -> int
     *
     * The logical line (up to a line break) that a layout line (a row on the screen) belongs to,
     * both counted from 1. The line after the last layout line gives the line after the last
     * logical line. Any other line outside the text is an error.
     */
    LayoutToLogicLineNum(layoutLine) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LayoutToLogicLineNum', args);
    },
    /**
     * LogicToLayoutLineNum(logicLine: int, logicIndex: int) -> int
     *
     * The layout line (a row on the screen) of a position in a logical line: the line and the
     * index in it (UTF-16 code units), both counted from 1. A line past the end gives the line
     * after the last layout line.
     */
    LogicToLayoutLineNum(logicLine, logicIndex) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LogicToLayoutLineNum', args);
    },
    /**
     * LineColumnToIndex(layoutLine: int, layoutColumn: int) -> int
     *
     * The index (UTF-16 code units, from 1) in the logical line of a layout line and a layout
     * column (cells), both from 1. A column past the text gives the end of the text of the row.
     */
    LineColumnToIndex(layoutLine, layoutColumn) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LineColumnToIndex', args);
    },
    /**
     * LineIndexToColumn(logicLine: int, logicIndex: int) -> int
     *
     * The layout column (cells, from 1) of a position in a logical line: the line and the index
     * in it (UTF-16 code units), both from 1. Tabs and full-width characters take their width.
     */
    LineIndexToColumn(logicLine, logicIndex) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LineIndexToColumn', args);
    },
    /**
     * GetStrWidth(text: string, column: int?) -> int
     *
     * The width of a string in cells, starting at layout column `column` (from 1, default 1; it
     * matters for tabs). A CR LF at the end counts as the CR alone.
     */
    GetStrWidth(text, column) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetStrWidth', args);
    },
    /**
     * GetStrLayoutLength(text: string, column: int?) -> int
     *
     * The width of a string as a line of the document in cells, starting at layout column
     * `column` (from 1, default 1). A line ending at the end takes one cell.
     */
    GetStrLayoutLength(text, column) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetStrLayoutLength', args);
    },
    /**
     * GetDefaultCharLength() -> int
     *
     * The layout width of a half-width character. This editor lays out in cells, so it is 1.
     */
    GetDefaultCharLength() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetDefaultCharLength', args);
    },
    /**
     * GetViewLines() -> int
     *
     * How many rows the text area shows (at least 1).
     */
    GetViewLines() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetViewLines', args);
    },
    /**
     * GetViewColumns() -> int
     *
     * How many columns the text area shows (at least 1).
     */
    GetViewColumns() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetViewColumns', args);
    },
    /**
     * GetViewTop() -> int
     *
     * The layout line at the top of the text area, counted from 1.
     */
    GetViewTop() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetViewTop', args);
    },
    /**
     * ChangeTabWidth(width: int) -> int
     *
     * The tab width before the call. A width from 1 to 64 that differs from it becomes the tab
     * width of this window (not saved). Any other value, such as 0, only reads.
     */
    ChangeTabWidth(width) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeTabWidth', args);
    },
    /**
     * ChangeWrapColm(column: int) -> int
     *
     * The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
     * column (not saved). Any other value only reads. Same as ChangeWrapColumn.
     */
    ChangeWrapColm(column) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeWrapColm', args);
    },
    /**
     * ChangeWrapColumn(column: int) -> int
     *
     * The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
     * column (not saved). Any other value only reads.
     */
    ChangeWrapColumn(column) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeWrapColumn', args);
    },
    /**
     * GetClipboard(option: int?) -> string
     *
     * The text on the clipboard, or an empty string when there is none. The line endings are
     * those of the source. The option is reserved.
     */
    GetClipboard(option) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetClipboard', args);
    },
    /**
     * SetClipboard(option: int?, text: string?) -> int
     *
     * Puts the text on the clipboard. Option bit 0 marks a box copy (a paste puts it in as a
     * box), bit 1 a row copy (a paste puts it in front of the row); with both, the box wins.
     * Returns 1 when it worked, 0 when not.
     */
    SetClipboard(option, text) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetClipboard', args);
    },
    /**
     * IsIncludeClipboardFormat(format: string) -> int
     *
     * 1 when the clipboard has the format. Only text is known: CF_TEXT, CF_OEMTEXT and
     * CF_UNICODETEXT (or 1, 7, 13) are 1 when the clipboard has text; every other format is 0.
     */
    IsIncludeClipboardFormat(format) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsIncludeClipboardFormat', args);
    },
    /**
     * GetClipboardByFormat(format: string, mode: int, endMode: int?) -> string
     *
     * The text of the clipboard in a text format (CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT), with the
     * line endings of the document. Mode -2 is the usual way and 3 is UTF-16 (the text itself);
     * the other modes and formats are errors (-32001). The end mode is not used.
     */
    GetClipboardByFormat(format, mode, endMode) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetClipboardByFormat', args);
    },
    /**
     * SetClipboardByFormat(text: string, format: string, mode: int, endMode: int?) -> int
     *
     * Puts the text on the clipboard as text. The formats and modes are those of
     * GetClipboardByFormat. Returns 1 when it worked.
     */
    SetClipboardByFormat(text, format, mode, endMode) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetClipboardByFormat', args);
    },
    /**
     * GetCookie(scope: string, name: string) -> string
     *
     * The value of a cookie, or an empty string when there is none. The scope is "window" or
     * "document".
     */
    GetCookie(scope, name) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetCookie', args);
    },
    /**
     * GetCookieDefault(scope: string, name: string, defaultValue: string) -> string
     *
     * The value of a cookie, or the default when there is none (an unknown scope still gives an
     * empty string). The scope is "window" or "document".
     */
    GetCookieDefault(scope, name, defaultValue) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetCookieDefault', args);
    },
    /**
     * SetCookie(scope: string, name: string, value: string) -> int
     *
     * Sets a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a name with a
     * character that a name cannot have (Sakura lets in digits, letters and every character from
     * "_" up in the character table). The scope is "window" or "document".
     */
    SetCookie(scope, name, value) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetCookie', args);
    },
    /**
     * DeleteCookie(scope: string, name: string) -> int
     *
     * Deletes a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a bad name, 5
     * when there is no such cookie. The scope is "window" or "document".
     */
    DeleteCookie(scope, name) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DeleteCookie', args);
    },
    /**
     * GetCookieNames(scope: string) -> string
     *
     * The names of the cookies of a scope, joined with commas, in the order of the names (UTF-16
     * code units). An unknown scope gives an empty string. The scope is "window" or "document".
     */
    GetCookieNames(scope) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetCookieNames', args);
    },
    /**
     * CompareVersion(versionA: string, versionB: string) -> int
     *
     * Compares two version strings: 0 when equal, positive when A is newer, negative when B is.
     * Four parts at most; a part has two digits at most; "alpha", "beta", "rc" and "pl" put a
     * part before or after the release (like PHP's version_compare).
     */
    CompareVersion(versionA, versionB) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CompareVersion', args);
    },
    /**
     * SetDrawSwitch(on: int) -> int
     *
     * Turns the draw switch on (non-zero) or off (0) and returns the state before (1 or 0). The
     * editor keeps the flag for GetDrawSwitch but still paints the text when it likes.
     */
    SetDrawSwitch(on) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetDrawSwitch', args);
    },
    /**
     * GetDrawSwitch() -> int
     *
     * 1 when the draw switch is on (the start state), 0 when a macro turned it off.
     */
    GetDrawSwitch() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetDrawSwitch', args);
    },
    /**
     * IsShownStatus() -> int
     *
     * 1 when the status bar is shown, else 0.
     */
    IsShownStatus() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsShownStatus', args);
    },
    /**
     * IsCurTypeExt(extension: string) -> int
     *
     * 1 when the document type of this file is the type of the extension (looked up in the
     * extension lists of the types in the settings file, as a file name). Without type settings
     * every extension is the basic type.
     */
    IsCurTypeExt(extension) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsCurTypeExt', args);
    },
    /**
     * IsSameTypeExt(extension1: string, extension2: string) -> int
     *
     * 1 when the two extensions belong to the same document type (see IsCurTypeExt).
     */
    IsSameTypeExt(extension1, extension2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsSameTypeExt', args);
    },
    /**
     * ExpandParameter(format: string) -> string
     *
     * Replaces the codes that start with $ in a string: $$ ($), $F (full path), $f (file name),
     * $g (file name without the extension), $/ (full path with /), $N (as $F), $b (extension),
     * $e and $E (folder), $C (the selected text or the word at the caret), $x and $y (caret
     * column in UTF-16 units and line, from 1), $S (the editor), $M (the macro file), $V
     * (version), $A (name), and conditions ${R?a$:b$:c$} (R, w, M, U, N, I). A text without a
     * file is (無題). $d $t $D $T $B $Q $I are not supported and stay as written. At most 2047
     * UTF-16 units.
     */
    ExpandParameter(format) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExpandParameter', args);
    },
    /**
     * MessageBox(message: string, type: int?) -> int
     *
     * Shows a message box and waits. The type is Windows' MB_ value: the low nibble chooses the
     * buttons (0 OK, 1 OK/Cancel, 2 Abort/Retry/Ignore, 3 Yes/No/Cancel, 4 Yes/No, 5
     * Retry/Cancel, 6 Cancel/Try again/Continue), the icon bits (0x10 stop, 0x20 question, 0x30
     * exclamation, 0x40 information) the icon; the default button and the modality are ignored.
     * Returns the button as Windows numbers it: 1 OK, 2 Cancel, 3 Abort, 4 Retry, 5 Ignore, 6
     * Yes, 7 No, 10 Try again, 11 Continue. A box that is closed without an answer gives Cancel
     * (the last button when there is none).
     */
    MessageBox(message, type) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MessageBox', args);
    },
    /**
     * ErrorMsg(message: string) -> int
     *
     * Shows an error message with an OK button and waits. Returns 1.
     */
    ErrorMsg(message) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ErrorMsg', args);
    },
    /**
     * WarnMsg(message: string) -> int
     *
     * Shows a warning with an OK button and waits. Returns 1.
     */
    WarnMsg(message) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WarnMsg', args);
    },
    /**
     * InfoMsg(message: string) -> int
     *
     * Shows an information message with an OK button and waits. Returns 1.
     */
    InfoMsg(message) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InfoMsg', args);
    },
    /**
     * OkCancelBox(message: string) -> int
     *
     * Asks a question with OK and Cancel and waits. Returns 1 (OK) or 2 (Cancel).
     */
    OkCancelBox(message) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('OkCancelBox', args);
    },
    /**
     * YesNoBox(message: string) -> int
     *
     * Asks a question with Yes and No and waits. Returns 6 (Yes) or 7 (No).
     */
    YesNoBox(message) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('YesNoBox', args);
    },
    /**
     * FileOpenDialog(defaultName: string?, filter: string?) -> string
     *
     * Lets the user choose a file to open and returns its full path, or an empty string when the
     * dialog is cancelled. The default name and the filter are not used: the dialog of the
     * platform takes neither.
     */
    FileOpenDialog(defaultName, filter) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileOpenDialog', args);
    },
    /**
     * FileSaveDialog(defaultName: string?, filter: string?) -> string
     *
     * Lets the user choose a file name to save under and returns the full path, or an empty
     * string when the dialog is cancelled. The dialog starts with the default name. The filter
     * is not used.
     */
    FileSaveDialog(defaultName, filter) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileSaveDialog', args);
    },
    /**
     * FolderDialog(message: string?, defaultFolder: string?) -> string
     *
     * Lets the user choose a folder and returns its full path, or an empty string when the
     * dialog is cancelled. The message becomes the prompt text of the platform dialog where it
     * has one; the default folder is not used.
     */
    FolderDialog(message, defaultFolder) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FolderDialog', args);
    },
    /**
     * InputBox(message: string, defaultValue: string?, maxLength: int?) -> string
     *
     * Shows a dialog with a text field and waits. The message is shown above the field, the
     * default value is in the field, and the text cannot be longer than the maximum length (in
     * UTF-16 code units; 0 or less, or no value, is 260). Returns the text of the field when OK
     * is pressed, or an empty string when the dialog is cancelled.
     */
    InputBox(message, defaultValue, maxLength) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InputBox', args);
    },
    /**
     * CreateMenu(position: int, menu: string) -> int
     *
     * Shows a pop-up menu and waits. The position is 1 for the caret (the pointer when the caret
     * is out of the view) and anything else for the pointer. The menu is a list of items
     * separated by commas; an item can start with flags in brackets: C checked, D disabled, R
     * radio, S opens a submenu, E ends a submenu (one for each E), B (a new column) is ignored.
     * A lone - is a separator. Write [][text] for a text that starts with a bracket. Returns the
     * number of the item chosen, counted from 1 over the items only (separators and submenus are
     * not counted), or 0 when the menu is closed without a choice or has no item.
     */
    CreateMenu(position, menu) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CreateMenu', args);
    },
    /**
     * Sleep(milliseconds: int) -> int
     *
     * Waits for the given number of milliseconds (the window stays alive, and the macro can be
     * cancelled). Returns 0.
     */
    Sleep(milliseconds) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Sleep', args);
    },
    /**
     * FileNew() -> null
     *
     * Sakura's F_FILENEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    FileNew() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileNew', args);
    },
    /**
     * FileOpen(arg1: string?, arg2: int?, arg3: int?, arg4: string?) -> null
     *
     * MATSU Editor's own F_FILEOPEN2 (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    FileOpen(arg1, arg2, arg3, arg4) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileOpen', args);
    },
    /**
     * FileSave() -> null
     *
     * Sakura's F_FILESAVE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    FileSave() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileSave', args);
    },
    /**
     * FileSaveAll() -> null
     *
     * Sakura's F_FILESAVEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileSaveAll() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileSaveAll', args);
    },
    /**
     * FileSaveAsDialog(arg1: string?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_FILESAVEAS_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileSaveAsDialog(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileSaveAsDialog', args);
    },
    /**
     * FileSaveAs(arg1: string?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_FILESAVEAS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    FileSaveAs(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileSaveAs', args);
    },
    /**
     * FileClose() -> null
     *
     * Sakura's F_FILECLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    FileClose() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileClose', args);
    },
    /**
     * FileCloseOpen() -> null
     *
     * Sakura's F_FILECLOSE_OPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileCloseOpen() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileCloseOpen', args);
    },
    /**
     * FileReopen(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopen(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopen', args);
    },
    /**
     * FileReopenSJIS(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenSJIS(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenSJIS', args);
    },
    /**
     * FileReopenJIS(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenJIS(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenJIS', args);
    },
    /**
     * FileReopenEUC(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenEUC(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenEUC', args);
    },
    /**
     * FileReopenLatin1(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_LATIN1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenLatin1(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenLatin1', args);
    },
    /**
     * FileReopenUNICODE(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_UNICODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenUNICODE(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenUNICODE', args);
    },
    /**
     * FileReopenUNICODEBE(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_UNICODEBE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
     * a key, the menu and a key macro run. The last arguments may be left out. A command that
     * this editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenUNICODEBE(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenUNICODEBE', args);
    },
    /**
     * FileReopenUTF8(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenUTF8(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenUTF8', args);
    },
    /**
     * FileReopenCESU8(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_CESU8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenCESU8(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenCESU8', args);
    },
    /**
     * FileReopenUTF7(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FileReopenUTF7(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileReopenUTF7', args);
    },
    /**
     * Print() -> null
     *
     * Sakura's F_PRINT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Print() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Print', args);
    },
    /**
     * PrintPreview() -> null
     *
     * Sakura's F_PRINT_PREVIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    PrintPreview() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PrintPreview', args);
    },
    /**
     * PrintPageSetup() -> null
     *
     * Sakura's F_PRINT_PAGESETUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    PrintPageSetup() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PrintPageSetup', args);
    },
    /**
     * OpenHfromtoC() -> null
     *
     * Sakura's F_OPEN_HfromtoC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    OpenHfromtoC() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('OpenHfromtoC', args);
    },
    /**
     * ActivateSQLPLUS() -> null
     *
     * Sakura's F_ACTIVATE_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ActivateSQLPLUS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ActivateSQLPLUS', args);
    },
    /**
     * ExecSQLPLUS() -> null
     *
     * Sakura's F_PLSQL_COMPILE_ON_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    ExecSQLPLUS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExecSQLPLUS', args);
    },
    /**
     * Browse() -> null
     *
     * Sakura's F_BROWSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Browse() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Browse', args);
    },
    /**
     * ViewMode() -> null
     *
     * Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ViewMode() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ViewMode', args);
    },
    /**
     * ReadOnly() -> null
     *
     * Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ReadOnly() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ReadOnly', args);
    },
    /**
     * PropertyFile() -> null
     *
     * Sakura's F_PROPERTY_FILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    PropertyFile() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PropertyFile', args);
    },
    /**
     * ExitAllEditors() -> null
     *
     * Sakura's F_EXITALLEDITORS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ExitAllEditors() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExitAllEditors', args);
    },
    /**
     * ExitAll() -> null
     *
     * Sakura's F_EXITALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ExitAll() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExitAll', args);
    },
    /**
     * PutFile(arg1: string?, arg2: int?, arg3: int?) -> null
     *
     * MATSU Editor's own F_PUTFILE (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    PutFile(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PutFile', args);
    },
    /**
     * InsFile(arg1: string?, arg2: int?, arg3: int?) -> null
     *
     * MATSU Editor's own F_INSFILE (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    InsFile(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InsFile', args);
    },
    /**
     * Char(arg1: int?) -> null
     *
     * Sakura's F_WCHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Char(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Char', args);
    },
    /**
     * CharIme(arg1: int?) -> null
     *
     * Sakura's F_IME_CHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CharIme(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CharIme', args);
    },
    /**
     * Undo() -> null
     *
     * Sakura's F_UNDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Undo() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Undo', args);
    },
    /**
     * Redo() -> null
     *
     * Sakura's F_REDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Redo() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Redo', args);
    },
    /**
     * Delete() -> null
     *
     * Sakura's F_DELETE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Delete() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Delete', args);
    },
    /**
     * DeleteBack() -> null
     *
     * Sakura's F_DELETE_BACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    DeleteBack() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DeleteBack', args);
    },
    /**
     * WordDeleteToStart() -> null
     *
     * Sakura's F_WordDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WordDeleteToStart() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordDeleteToStart', args);
    },
    /**
     * WordDeleteToEnd() -> null
     *
     * Sakura's F_WordDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WordDeleteToEnd() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordDeleteToEnd', args);
    },
    /**
     * WordCut() -> null
     *
     * Sakura's F_WordCut (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WordCut() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordCut', args);
    },
    /**
     * WordDelete() -> null
     *
     * Sakura's F_WordDelete (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WordDelete() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordDelete', args);
    },
    /**
     * LineCutToStart() -> null
     *
     * Sakura's F_LineCutToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    LineCutToStart() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LineCutToStart', args);
    },
    /**
     * LineCutToEnd() -> null
     *
     * Sakura's F_LineCutToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    LineCutToEnd() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LineCutToEnd', args);
    },
    /**
     * LineDeleteToStart() -> null
     *
     * Sakura's F_LineDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    LineDeleteToStart() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LineDeleteToStart', args);
    },
    /**
     * LineDeleteToEnd() -> null
     *
     * Sakura's F_LineDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    LineDeleteToEnd() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LineDeleteToEnd', args);
    },
    /**
     * CutLine() -> null
     *
     * Sakura's F_CUT_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CutLine() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CutLine', args);
    },
    /**
     * DeleteLine() -> null
     *
     * Sakura's F_DELETE_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    DeleteLine() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DeleteLine', args);
    },
    /**
     * DuplicateLine() -> null
     *
     * Sakura's F_DUPLICATELINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    DuplicateLine() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DuplicateLine', args);
    },
    /**
     * IndentTab() -> null
     *
     * Sakura's F_INDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    IndentTab() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IndentTab', args);
    },
    /**
     * UnindentTab() -> null
     *
     * Sakura's F_UNINDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    UnindentTab() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('UnindentTab', args);
    },
    /**
     * IndentSpace() -> null
     *
     * Sakura's F_INDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    IndentSpace() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IndentSpace', args);
    },
    /**
     * UnindentSpace() -> null
     *
     * Sakura's F_UNINDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    UnindentSpace() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('UnindentSpace', args);
    },
    /**
     * LTrim() -> null
     *
     * Sakura's F_LTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    LTrim() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LTrim', args);
    },
    /**
     * RTrim() -> null
     *
     * Sakura's F_RTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    RTrim() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('RTrim', args);
    },
    /**
     * SortAsc() -> null
     *
     * Sakura's F_SORT_ASC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    SortAsc() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SortAsc', args);
    },
    /**
     * SortDesc() -> null
     *
     * Sakura's F_SORT_DESC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    SortDesc() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SortDesc', args);
    },
    /**
     * Merge() -> null
     *
     * Sakura's F_MERGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Merge() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Merge', args);
    },
    /**
     * Left() -> null
     *
     * Sakura's F_LEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Left() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Left', args);
    },
    /**
     * Right() -> null
     *
     * Sakura's F_RIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Right() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Right', args);
    },
    /**
     * Up2() -> null
     *
     * Sakura's F_UP2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Up2() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Up2', args);
    },
    /**
     * Down2() -> null
     *
     * Sakura's F_DOWN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Down2() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Down2', args);
    },
    /**
     * WordLeft() -> null
     *
     * Sakura's F_WORDLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WordLeft() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordLeft', args);
    },
    /**
     * WordRight() -> null
     *
     * Sakura's F_WORDRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WordRight() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordRight', args);
    },
    /**
     * GoLineTop(arg1: int?) -> null
     *
     * Sakura's F_GOLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    GoLineTop(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoLineTop', args);
    },
    /**
     * GoLineEnd(arg1: int?) -> null
     *
     * Sakura's F_GOLINEEND (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    GoLineEnd(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoLineEnd', args);
    },
    /**
     * HalfPageUp(arg1: int?) -> null
     *
     * Sakura's F_HalfPageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    HalfPageUp(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HalfPageUp', args);
    },
    /**
     * HalfPageDown(arg1: int?) -> null
     *
     * Sakura's F_HalfPageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HalfPageDown(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HalfPageDown', args);
    },
    /**
     * PageUp(arg1: int?) -> null
     *
     * Sakura's F_1PageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    PageUp(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PageUp', args);
    },
    /**
     * PageDown(arg1: int?) -> null
     *
     * Sakura's F_1PageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    PageDown(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PageDown', args);
    },
    /**
     * CurLineCenter() -> null
     *
     * Sakura's F_CURLINECENTER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CurLineCenter() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CurLineCenter', args);
    },
    /**
     * CurLineTop() -> null
     *
     * Sakura's F_CURLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CurLineTop() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CurLineTop', args);
    },
    /**
     * CurLineBottom() -> null
     *
     * Sakura's F_CURLINEBOTTOM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CurLineBottom() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CurLineBottom', args);
    },
    /**
     * MoveHistPrev() -> null
     *
     * Sakura's F_JUMPHIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    MoveHistPrev() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MoveHistPrev', args);
    },
    /**
     * MoveHistNext() -> null
     *
     * Sakura's F_JUMPHIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    MoveHistNext() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MoveHistNext', args);
    },
    /**
     * MoveHistSet() -> null
     *
     * Sakura's F_JUMPHIST_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    MoveHistSet() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MoveHistSet', args);
    },
    /**
     * F_WndScrollDown() -> null
     *
     * Sakura's F_WndScrollDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    F_WndScrollDown() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('F_WndScrollDown', args);
    },
    /**
     * F_WndScrollUp() -> null
     *
     * Sakura's F_WndScrollUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    F_WndScrollUp() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('F_WndScrollUp', args);
    },
    /**
     * GoNextParagraph() -> null
     *
     * Sakura's F_GONEXTPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoNextParagraph() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoNextParagraph', args);
    },
    /**
     * GoPrevParagraph() -> null
     *
     * Sakura's F_GOPREVPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoPrevParagraph() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoPrevParagraph', args);
    },
    /**
     * GoModifyLineNext() -> null
     *
     * Sakura's F_MODIFYLINE_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoModifyLineNext() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoModifyLineNext', args);
    },
    /**
     * GoModifyLinePrev() -> null
     *
     * Sakura's F_MODIFYLINE_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoModifyLinePrev() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoModifyLinePrev', args);
    },
    /**
     * MoveCursorLayout(arg1: int?, arg2: int?, arg3: int?) -> null
     *
     * MATSU Editor's own F_MOVECURSORLAYOUT (not in Sakura). Runs the command that a key, the menu
     * and a key macro run.
     */
    MoveCursorLayout(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MoveCursorLayout', args);
    },
    /**
     * WheelUp(arg1: int?) -> null
     *
     * Sakura's F_WHEELUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WheelUp(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WheelUp', args);
    },
    /**
     * WheelDown(arg1: int?) -> null
     *
     * Sakura's F_WHEELDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WheelDown(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WheelDown', args);
    },
    /**
     * WheelLeft(arg1: int?) -> null
     *
     * Sakura's F_WHEELLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WheelLeft(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WheelLeft', args);
    },
    /**
     * WheelRight(arg1: int?) -> null
     *
     * Sakura's F_WHEELRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WheelRight(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WheelRight', args);
    },
    /**
     * WheelPageUp(arg1: int?) -> null
     *
     * Sakura's F_WHEELPAGEUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WheelPageUp(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WheelPageUp', args);
    },
    /**
     * WheelPageDown(arg1: int?) -> null
     *
     * Sakura's F_WHEELPAGEDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WheelPageDown(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WheelPageDown', args);
    },
    /**
     * WheelPageLeft(arg1: int?) -> null
     *
     * Sakura's F_WHEELPAGELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WheelPageLeft(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WheelPageLeft', args);
    },
    /**
     * WheelPageRight(arg1: int?) -> null
     *
     * Sakura's F_WHEELPAGERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WheelPageRight(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WheelPageRight', args);
    },
    /**
     * SelectWord() -> null
     *
     * Sakura's F_SELECTWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    SelectWord() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SelectWord', args);
    },
    /**
     * SelectLine(arg1: int?) -> null
     *
     * Sakura's F_SELECTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    SelectLine(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SelectLine', args);
    },
    /**
     * BeginSelect() -> null
     *
     * Sakura's F_BEGIN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    BeginSelect() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BeginSelect', args);
    },
    /**
     * Up_Sel() -> null
     *
     * Sakura's F_UP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Up_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Up_Sel', args);
    },
    /**
     * Down_Sel() -> null
     *
     * Sakura's F_DOWN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Down_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Down_Sel', args);
    },
    /**
     * Left_Sel() -> null
     *
     * Sakura's F_LEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Left_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Left_Sel', args);
    },
    /**
     * Right_Sel() -> null
     *
     * Sakura's F_RIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Right_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Right_Sel', args);
    },
    /**
     * Up2_Sel() -> null
     *
     * Sakura's F_UP2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Up2_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Up2_Sel', args);
    },
    /**
     * Down2_Sel() -> null
     *
     * Sakura's F_DOWN2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Down2_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Down2_Sel', args);
    },
    /**
     * WordLeft_Sel() -> null
     *
     * Sakura's F_WORDLEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WordLeft_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordLeft_Sel', args);
    },
    /**
     * WordRight_Sel() -> null
     *
     * Sakura's F_WORDRIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WordRight_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordRight_Sel', args);
    },
    /**
     * GoLineTop_Sel(arg1: int?) -> null
     *
     * Sakura's F_GOLINETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoLineTop_Sel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoLineTop_Sel', args);
    },
    /**
     * GoLineEnd_Sel(arg1: int?) -> null
     *
     * Sakura's F_GOLINEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoLineEnd_Sel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoLineEnd_Sel', args);
    },
    /**
     * HalfPageUp_Sel(arg1: int?) -> null
     *
     * Sakura's F_HalfPageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HalfPageUp_Sel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HalfPageUp_Sel', args);
    },
    /**
     * HalfPageDown_Sel(arg1: int?) -> null
     *
     * Sakura's F_HalfPageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HalfPageDown_Sel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HalfPageDown_Sel', args);
    },
    /**
     * PageUp_Sel(arg1: int?) -> null
     *
     * Sakura's F_1PageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    PageUp_Sel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PageUp_Sel', args);
    },
    /**
     * PageDown_Sel(arg1: int?) -> null
     *
     * Sakura's F_1PageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    PageDown_Sel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PageDown_Sel', args);
    },
    /**
     * GoFileTop_Sel() -> null
     *
     * Sakura's F_GOFILETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoFileTop_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoFileTop_Sel', args);
    },
    /**
     * GoFileEnd_Sel() -> null
     *
     * Sakura's F_GOFILEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoFileEnd_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoFileEnd_Sel', args);
    },
    /**
     * GoNextParagraph_Sel() -> null
     *
     * Sakura's F_GONEXTPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoNextParagraph_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoNextParagraph_Sel', args);
    },
    /**
     * GoPrevParagraph_Sel() -> null
     *
     * Sakura's F_GOPREVPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoPrevParagraph_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoPrevParagraph_Sel', args);
    },
    /**
     * GoModifyLineNext_Sel() -> null
     *
     * Sakura's F_MODIFYLINE_NEXT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoModifyLineNext_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoModifyLineNext_Sel', args);
    },
    /**
     * GoModifyLinePrev_Sel() -> null
     *
     * Sakura's F_MODIFYLINE_PREV_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoModifyLinePrev_Sel() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoModifyLinePrev_Sel', args);
    },
    /**
     * BeginBoxSelect() -> null
     *
     * Sakura's F_BEGIN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    BeginBoxSelect() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BeginBoxSelect', args);
    },
    /**
     * Up_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_UP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Up_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Up_BoxSel', args);
    },
    /**
     * Down_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_DOWN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Down_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Down_BoxSel', args);
    },
    /**
     * Left_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_LEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Left_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Left_BoxSel', args);
    },
    /**
     * Right_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_RIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Right_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Right_BoxSel', args);
    },
    /**
     * Up2_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_UP2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Up2_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Up2_BoxSel', args);
    },
    /**
     * Down2_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_DOWN2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Down2_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Down2_BoxSel', args);
    },
    /**
     * WordLeft_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_WORDLEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WordLeft_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordLeft_BoxSel', args);
    },
    /**
     * WordRight_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_WORDRIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WordRight_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WordRight_BoxSel', args);
    },
    /**
     * GoLogicalLineTop_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_GOLOGICALLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
     * a key, the menu and a key macro run. The last arguments may be left out. A command that
     * this editor does not have yet answers the error -32001 (not implemented).
     */
    GoLogicalLineTop_BoxSel(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoLogicalLineTop_BoxSel', args);
    },
    /**
     * GoLineTop_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_GOLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoLineTop_BoxSel(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoLineTop_BoxSel', args);
    },
    /**
     * GoLineEnd_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_GOLINEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoLineEnd_BoxSel(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoLineEnd_BoxSel', args);
    },
    /**
     * HalfPageUp_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_HalfPageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HalfPageUp_BoxSel(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HalfPageUp_BoxSel', args);
    },
    /**
     * HalfPageDown_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_HalfPageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HalfPageDown_BoxSel(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HalfPageDown_BoxSel', args);
    },
    /**
     * PageUp_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_1PageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    PageUp_BoxSel(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PageUp_BoxSel', args);
    },
    /**
     * PageDown_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_1PageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    PageDown_BoxSel(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PageDown_BoxSel', args);
    },
    /**
     * GoFileTop_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_GOFILETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoFileTop_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoFileTop_BoxSel', args);
    },
    /**
     * GoFileEnd_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_GOFILEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GoFileEnd_BoxSel(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GoFileEnd_BoxSel', args);
    },
    /**
     * Cut() -> null
     *
     * Sakura's F_CUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Cut() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Cut', args);
    },
    /**
     * Copy() -> null
     *
     * Sakura's F_COPY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Copy() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Copy', args);
    },
    /**
     * Paste(arg1: int?) -> null
     *
     * Sakura's F_PASTE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Paste(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Paste', args);
    },
    /**
     * CopyAddCRLF() -> null
     *
     * Sakura's F_COPY_ADDCRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CopyAddCRLF() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyAddCRLF', args);
    },
    /**
     * CopyCRLF() -> null
     *
     * Sakura's F_COPY_CRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CopyCRLF() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyCRLF', args);
    },
    /**
     * CopyLF() -> null
     *
     * MATSU Editor's own F_COPY_LF (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    CopyLF() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyLF', args);
    },
    /**
     * CopyCR() -> null
     *
     * MATSU Editor's own F_COPY_CR (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    CopyCR() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyCR', args);
    },
    /**
     * PasteBox(arg1: int?) -> null
     *
     * Sakura's F_PASTEBOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    PasteBox(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PasteBox', args);
    },
    /**
     * InsBoxText(arg1: string?) -> null
     *
     * MATSU Editor's own F_INSBOXTEXT (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    InsBoxText(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InsBoxText', args);
    },
    /**
     * CopyLines() -> null
     *
     * Sakura's F_COPYLINES (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CopyLines() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyLines', args);
    },
    /**
     * CopyLinesAsPassage() -> null
     *
     * Sakura's F_COPYLINESASPASSAGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CopyLinesAsPassage() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyLinesAsPassage', args);
    },
    /**
     * CopyLinesWithLineNumber() -> null
     *
     * Sakura's F_COPYLINESWITHLINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    CopyLinesWithLineNumber() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyLinesWithLineNumber', args);
    },
    /**
     * CopyColorHtml() -> null
     *
     * Sakura's F_COPY_COLOR_HTML (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CopyColorHtml() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyColorHtml', args);
    },
    /**
     * CopyColorHtmlWithLineNumber() -> null
     *
     * Sakura's F_COPY_COLOR_HTML_LINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    CopyColorHtmlWithLineNumber() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyColorHtmlWithLineNumber', args);
    },
    /**
     * CopyPath() -> null
     *
     * Sakura's F_COPYPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CopyPath() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyPath', args);
    },
    /**
     * CopyDirPath() -> null
     *
     * Sakura's F_COPYDIRPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CopyDirPath() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyDirPath', args);
    },
    /**
     * CopyFilename() -> null
     *
     * Sakura's F_COPYFNAME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CopyFilename() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyFilename', args);
    },
    /**
     * CopyTag() -> null
     *
     * Sakura's F_COPYTAG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CopyTag() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyTag', args);
    },
    /**
     * CopyKeyBindList() -> null
     *
     * Sakura's F_CREATEKEYBINDLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CopyKeyBindList() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CopyKeyBindList', args);
    },
    /**
     * InsertDate() -> null
     *
     * Sakura's F_INS_DATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    InsertDate() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InsertDate', args);
    },
    /**
     * InsertTime() -> null
     *
     * Sakura's F_INS_TIME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    InsertTime() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InsertTime', args);
    },
    /**
     * CtrlCodeDialog() -> null
     *
     * Sakura's F_CTRL_CODE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CtrlCodeDialog() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CtrlCodeDialog', args);
    },
    /**
     * CtrlCode(arg1: int?) -> null
     *
     * Sakura's F_CTRL_CODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CtrlCode(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CtrlCode', args);
    },
    /**
     * InsertFileUsedRecently() -> null
     *
     * Sakura's F_INS_FILE_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    InsertFileUsedRecently() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InsertFileUsedRecently', args);
    },
    /**
     * InsertFolderUsedRecently() -> null
     *
     * Sakura's F_INS_FOLDER_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    InsertFolderUsedRecently() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InsertFolderUsedRecently', args);
    },
    /**
     * ToLower() -> null
     *
     * Sakura's F_TOLOWER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ToLower() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToLower', args);
    },
    /**
     * ToUpper() -> null
     *
     * Sakura's F_TOUPPER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ToUpper() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToUpper', args);
    },
    /**
     * ToHankaku() -> null
     *
     * Sakura's F_TOHANKAKU (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ToHankaku() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToHankaku', args);
    },
    /**
     * ToHankata() -> null
     *
     * Sakura's F_TOHANKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ToHankata() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToHankata', args);
    },
    /**
     * ToZenEi() -> null
     *
     * Sakura's F_TOZENEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ToZenEi() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToZenEi', args);
    },
    /**
     * ToHanEi() -> null
     *
     * Sakura's F_TOHANEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ToHanEi() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToHanEi', args);
    },
    /**
     * ToZenKata() -> null
     *
     * Sakura's F_TOZENKAKUKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ToZenKata() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToZenKata', args);
    },
    /**
     * ToZenHira() -> null
     *
     * Sakura's F_TOZENKAKUHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ToZenHira() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToZenHira', args);
    },
    /**
     * HanKataToZenKata() -> null
     *
     * Sakura's F_HANKATATOZENKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HanKataToZenKata() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HanKataToZenKata', args);
    },
    /**
     * HanKataToZenHira() -> null
     *
     * Sakura's F_HANKATATOZENHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HanKataToZenHira() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HanKataToZenHira', args);
    },
    /**
     * TABToSPACE() -> null
     *
     * Sakura's F_TABTOSPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    TABToSPACE() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TABToSPACE', args);
    },
    /**
     * SPACEToTAB() -> null
     *
     * Sakura's F_SPACETOTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    SPACEToTAB() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SPACEToTAB', args);
    },
    /**
     * AutoToSJIS() -> null
     *
     * Sakura's F_CODECNV_AUTO2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    AutoToSJIS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('AutoToSJIS', args);
    },
    /**
     * JIStoSJIS() -> null
     *
     * Sakura's F_CODECNV_EMAIL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    JIStoSJIS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('JIStoSJIS', args);
    },
    /**
     * EUCtoSJIS() -> null
     *
     * Sakura's F_CODECNV_EUC2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    EUCtoSJIS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('EUCtoSJIS', args);
    },
    /**
     * CodeCnvUNICODEtoSJIS() -> null
     *
     * Sakura's F_CODECNV_UNICODE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
     * a key, the menu and a key macro run. The last arguments may be left out. A command that
     * this editor does not have yet answers the error -32001 (not implemented).
     */
    CodeCnvUNICODEtoSJIS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CodeCnvUNICODEtoSJIS', args);
    },
    /**
     * CodeCnvUNICODEBEtoSJIS() -> null
     *
     * Sakura's F_CODECNV_UNICODEBE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    CodeCnvUNICODEBEtoSJIS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CodeCnvUNICODEBEtoSJIS', args);
    },
    /**
     * UTF8toSJIS() -> null
     *
     * Sakura's F_CODECNV_UTF82SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    UTF8toSJIS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('UTF8toSJIS', args);
    },
    /**
     * UTF7toSJIS() -> null
     *
     * Sakura's F_CODECNV_UTF72SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    UTF7toSJIS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('UTF7toSJIS', args);
    },
    /**
     * SJIStoJIS() -> null
     *
     * Sakura's F_CODECNV_SJIS2JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SJIStoJIS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SJIStoJIS', args);
    },
    /**
     * SJIStoEUC() -> null
     *
     * Sakura's F_CODECNV_SJIS2EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SJIStoEUC() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SJIStoEUC', args);
    },
    /**
     * SJIStoUTF8() -> null
     *
     * Sakura's F_CODECNV_SJIS2UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SJIStoUTF8() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SJIStoUTF8', args);
    },
    /**
     * SJIStoUTF7() -> null
     *
     * Sakura's F_CODECNV_SJIS2UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SJIStoUTF7() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SJIStoUTF7', args);
    },
    /**
     * Base64Decode() -> null
     *
     * Sakura's F_BASE64DECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    Base64Decode() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Base64Decode', args);
    },
    /**
     * Uudecode() -> null
     *
     * Sakura's F_UUDECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Uudecode() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Uudecode', args);
    },
    /**
     * SearchDialog() -> null
     *
     * Sakura's F_SEARCH_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SearchDialog() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SearchDialog', args);
    },
    /**
     * SearchNext(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_SEARCH_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SearchNext(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SearchNext', args);
    },
    /**
     * SearchPrev(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_SEARCH_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SearchPrev(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SearchPrev', args);
    },
    /**
     * ReplaceDialog() -> null
     *
     * Sakura's F_REPLACE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ReplaceDialog() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ReplaceDialog', args);
    },
    /**
     * Replace(arg1: string?, arg2: string?, arg3: int?) -> null
     *
     * Sakura's F_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Replace(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Replace', args);
    },
    /**
     * ReplaceAll(arg1: string?, arg2: string?, arg3: int?) -> null
     *
     * Sakura's F_REPLACE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ReplaceAll(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ReplaceAll', args);
    },
    /**
     * SearchClearMark() -> null
     *
     * Sakura's F_SEARCH_CLEARMARK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SearchClearMark() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SearchClearMark', args);
    },
    /**
     * SearchStartPos() -> null
     *
     * Sakura's F_JUMP_SRCHSTARTPOS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SearchStartPos() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SearchStartPos', args);
    },
    /**
     * Grep(arg1: string?, arg2: string?, arg3: string?, arg4: int?, arg5: int?) -> null
     *
     * Sakura's F_GREP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Grep(arg1, arg2, arg3, arg4, arg5) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Grep', args);
    },
    /**
     * GrepReplace(arg1: string?, arg2: string?, arg3: string?, arg4: string?, arg5: int?, arg6: int?) -> null
     *
     * Sakura's F_GREP_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    GrepReplace(arg1, arg2, arg3, arg4, arg5, arg6) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GrepReplace', args);
    },
    /**
     * Outline(arg1: int?) -> null
     *
     * Sakura's F_OUTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Outline(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Outline', args);
    },
    /**
     * TagJump() -> null
     *
     * Sakura's F_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    TagJump() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TagJump', args);
    },
    /**
     * TagJumpBack() -> null
     *
     * Sakura's F_TAGJUMPBACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TagJumpBack() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TagJumpBack', args);
    },
    /**
     * TagMake() -> null
     *
     * Sakura's F_TAGS_MAKE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    TagMake() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TagMake', args);
    },
    /**
     * DirectTagJump() -> null
     *
     * Sakura's F_DIRECT_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    DirectTagJump() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DirectTagJump', args);
    },
    /**
     * KeywordTagJump(arg1: string?) -> null
     *
     * Sakura's F_TAGJUMP_KEYWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    KeywordTagJump(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('KeywordTagJump', args);
    },
    /**
     * Compare() -> null
     *
     * Sakura's F_COMPARE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    Compare() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Compare', args);
    },
    /**
     * DiffDialog() -> null
     *
     * Sakura's F_DIFF_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    DiffDialog() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DiffDialog', args);
    },
    /**
     * Diff(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_DIFF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Diff(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Diff', args);
    },
    /**
     * DiffNext() -> null
     *
     * Sakura's F_DIFF_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    DiffNext() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DiffNext', args);
    },
    /**
     * DiffPrev() -> null
     *
     * Sakura's F_DIFF_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    DiffPrev() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DiffPrev', args);
    },
    /**
     * DiffReset() -> null
     *
     * Sakura's F_DIFF_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    DiffReset() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DiffReset', args);
    },
    /**
     * BracketPair() -> null
     *
     * Sakura's F_BRACKETPAIR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    BracketPair() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BracketPair', args);
    },
    /**
     * BookmarkSet() -> null
     *
     * Sakura's F_BOOKMARK_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    BookmarkSet() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BookmarkSet', args);
    },
    /**
     * BookmarkNext() -> null
     *
     * Sakura's F_BOOKMARK_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    BookmarkNext() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BookmarkNext', args);
    },
    /**
     * BookmarkPrev() -> null
     *
     * Sakura's F_BOOKMARK_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    BookmarkPrev() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BookmarkPrev', args);
    },
    /**
     * BookmarkReset() -> null
     *
     * Sakura's F_BOOKMARK_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    BookmarkReset() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BookmarkReset', args);
    },
    /**
     * BookmarkView() -> null
     *
     * Sakura's F_BOOKMARK_VIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    BookmarkView() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BookmarkView', args);
    },
    /**
     * BookmarkPattern(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_BOOKMARK_PATTERN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    BookmarkPattern(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('BookmarkPattern', args);
    },
    /**
     * FuncListNext() -> null
     *
     * Sakura's F_FUNCLIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FuncListNext() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FuncListNext', args);
    },
    /**
     * FuncListPrev() -> null
     *
     * Sakura's F_FUNCLIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    FuncListPrev() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FuncListPrev', args);
    },
    /**
     * ChgmodINS() -> null
     *
     * Sakura's F_CHGMOD_INS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ChgmodINS() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChgmodINS', args);
    },
    /**
     * ChgCharSet(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_CHG_CHARSET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ChgCharSet(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChgCharSet', args);
    },
    /**
     * ChgmodEOL(arg1: int?) -> null
     *
     * MATSU Editor's own F_CHGMOD_EOL (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    ChgmodEOL(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChgmodEOL', args);
    },
    /**
     * CancelMode() -> null
     *
     * Sakura's F_CANCEL_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CancelMode() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CancelMode', args);
    },
    /**
     * ExecExternalMacro(arg1: string?, arg2: string?) -> null
     *
     * Sakura's F_EXECEXTMACRO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ExecExternalMacro(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExecExternalMacro', args);
    },
    /**
     * ShowToolbar() -> null
     *
     * Sakura's F_SHOWTOOLBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ShowToolbar() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ShowToolbar', args);
    },
    /**
     * ShowFunckey() -> null
     *
     * Sakura's F_SHOWFUNCKEY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ShowFunckey() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ShowFunckey', args);
    },
    /**
     * ShowTab() -> null
     *
     * Sakura's F_SHOWTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ShowTab() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ShowTab', args);
    },
    /**
     * ShowStatusbar() -> null
     *
     * Sakura's F_SHOWSTATUSBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ShowStatusbar() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ShowStatusbar', args);
    },
    /**
     * ShowMiniMap() -> null
     *
     * Sakura's F_SHOWMINIMAP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ShowMiniMap() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ShowMiniMap', args);
    },
    /**
     * TypeList() -> null
     *
     * Sakura's F_TYPE_LIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    TypeList() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TypeList', args);
    },
    /**
     * ChangeType(arg1: int?) -> null
     *
     * MATSU Editor's own F_CHANGETYPE (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    ChangeType(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeType', args);
    },
    /**
     * OptionType() -> null
     *
     * Sakura's F_OPTION_TYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    OptionType() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('OptionType', args);
    },
    /**
     * OptionCommon() -> null
     *
     * Sakura's F_OPTION (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    OptionCommon() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('OptionCommon', args);
    },
    /**
     * SelectFont() -> null
     *
     * Sakura's F_FONT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    SelectFont() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SelectFont', args);
    },
    /**
     * SetFontSize(arg1: int?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_SETFONTSIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SetFontSize(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetFontSize', args);
    },
    /**
     * WrapWindowWidth() -> null
     *
     * Sakura's F_WRAPWINDOWWIDTH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WrapWindowWidth() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WrapWindowWidth', args);
    },
    /**
     * OptionFavorite() -> null
     *
     * Sakura's F_FAVORITE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    OptionFavorite() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('OptionFavorite', args);
    },
    /**
     * SetMsgQuoteStr(arg1: string?) -> null
     *
     * MATSU Editor's own F_SET_QUOTESTRING (not in Sakura). Runs the command that a key, the menu
     * and a key macro run.
     */
    SetMsgQuoteStr(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetMsgQuoteStr', args);
    },
    /**
     * TextWrapMethod(arg1: int?) -> null
     *
     * MATSU Editor's own F_TEXTWRAPMETHOD (not in Sakura). Runs the command that a key, the menu
     * and a key macro run.
     */
    TextWrapMethod(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TextWrapMethod', args);
    },
    /**
     * SelectCountMode(arg1: int?) -> null
     *
     * Sakura's F_SELECT_COUNT_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SelectCountMode(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SelectCountMode', args);
    },
    /**
     * ExecCommand(arg1: string?, arg2: int?, arg3: string?) -> null
     *
     * Sakura's F_EXECMD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ExecCommand(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExecCommand', args);
    },
    /**
     * ExecCommandDialog() -> null
     *
     * Sakura's F_EXECMD_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ExecCommandDialog() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExecCommandDialog', args);
    },
    /**
     * RMenu() -> null
     *
     * Sakura's F_MENU_RBUTTON (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    RMenu() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('RMenu', args);
    },
    /**
     * CustMenu1() -> null
     *
     * Sakura's F_CUSTMENU_1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu1() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu1', args);
    },
    /**
     * CustMenu2() -> null
     *
     * Sakura's F_CUSTMENU_2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu2() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu2', args);
    },
    /**
     * CustMenu3() -> null
     *
     * Sakura's F_CUSTMENU_3 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu3() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu3', args);
    },
    /**
     * CustMenu4() -> null
     *
     * Sakura's F_CUSTMENU_4 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu4() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu4', args);
    },
    /**
     * CustMenu5() -> null
     *
     * Sakura's F_CUSTMENU_5 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu5() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu5', args);
    },
    /**
     * CustMenu6() -> null
     *
     * Sakura's F_CUSTMENU_6 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu6() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu6', args);
    },
    /**
     * CustMenu7() -> null
     *
     * Sakura's F_CUSTMENU_7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu7() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu7', args);
    },
    /**
     * CustMenu8() -> null
     *
     * Sakura's F_CUSTMENU_8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu8() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu8', args);
    },
    /**
     * CustMenu9() -> null
     *
     * Sakura's F_CUSTMENU_9 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CustMenu9() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu9', args);
    },
    /**
     * CustMenu10() -> null
     *
     * Sakura's F_CUSTMENU_10 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu10() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu10', args);
    },
    /**
     * CustMenu11() -> null
     *
     * Sakura's F_CUSTMENU_11 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu11() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu11', args);
    },
    /**
     * CustMenu12() -> null
     *
     * Sakura's F_CUSTMENU_12 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu12() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu12', args);
    },
    /**
     * CustMenu13() -> null
     *
     * Sakura's F_CUSTMENU_13 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu13() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu13', args);
    },
    /**
     * CustMenu14() -> null
     *
     * Sakura's F_CUSTMENU_14 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu14() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu14', args);
    },
    /**
     * CustMenu15() -> null
     *
     * Sakura's F_CUSTMENU_15 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu15() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu15', args);
    },
    /**
     * CustMenu16() -> null
     *
     * Sakura's F_CUSTMENU_16 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu16() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu16', args);
    },
    /**
     * CustMenu17() -> null
     *
     * Sakura's F_CUSTMENU_17 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu17() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu17', args);
    },
    /**
     * CustMenu18() -> null
     *
     * Sakura's F_CUSTMENU_18 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu18() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu18', args);
    },
    /**
     * CustMenu19() -> null
     *
     * Sakura's F_CUSTMENU_19 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu19() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu19', args);
    },
    /**
     * CustMenu20() -> null
     *
     * Sakura's F_CUSTMENU_20 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu20() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu20', args);
    },
    /**
     * CustMenu21() -> null
     *
     * Sakura's F_CUSTMENU_21 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu21() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu21', args);
    },
    /**
     * CustMenu22() -> null
     *
     * Sakura's F_CUSTMENU_22 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu22() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu22', args);
    },
    /**
     * CustMenu23() -> null
     *
     * Sakura's F_CUSTMENU_23 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu23() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu23', args);
    },
    /**
     * CustMenu24() -> null
     *
     * Sakura's F_CUSTMENU_24 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CustMenu24() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CustMenu24', args);
    },
    /**
     * SplitWinV() -> null
     *
     * Sakura's F_SPLIT_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    SplitWinV() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SplitWinV', args);
    },
    /**
     * SplitWinH() -> null
     *
     * Sakura's F_SPLIT_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    SplitWinH() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SplitWinH', args);
    },
    /**
     * SplitWinVH() -> null
     *
     * Sakura's F_SPLIT_VH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    SplitWinVH() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SplitWinVH', args);
    },
    /**
     * WinMaximize() -> null
     *
     * MATSU Editor's own F_WINMAXIMIZE (not in Sakura). Runs the command that a key, the menu and
     * a key macro run.
     */
    WinMaximize() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WinMaximize', args);
    },
    /**
     * WinMinimize() -> null
     *
     * MATSU Editor's own F_WINMINIMIZE (not in Sakura). Runs the command that a key, the menu and
     * a key macro run.
     */
    WinMinimize() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WinMinimize', args);
    },
    /**
     * WinRestore() -> null
     *
     * MATSU Editor's own F_WINRESTORE (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    WinRestore() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WinRestore', args);
    },
    /**
     * WinClose() -> null
     *
     * Sakura's F_WINCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WinClose() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WinClose', args);
    },
    /**
     * WinCloseAll() -> null
     *
     * Sakura's F_WIN_CLOSEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    WinCloseAll() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WinCloseAll', args);
    },
    /**
     * CascadeWin() -> null
     *
     * Sakura's F_CASCADE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    CascadeWin() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CascadeWin', args);
    },
    /**
     * TileWinV() -> null
     *
     * Sakura's F_TILE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    TileWinV() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TileWinV', args);
    },
    /**
     * TileWinH() -> null
     *
     * Sakura's F_TILE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    TileWinH() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TileWinH', args);
    },
    /**
     * NextWindow() -> null
     *
     * Sakura's F_NEXTWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    NextWindow() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('NextWindow', args);
    },
    /**
     * PrevWindow() -> null
     *
     * Sakura's F_PREVWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    PrevWindow() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PrevWindow', args);
    },
    /**
     * WindowList() -> null
     *
     * Sakura's F_WINLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WindowList() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WindowList', args);
    },
    /**
     * MaximizeV() -> null
     *
     * Sakura's F_MAXIMIZE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    MaximizeV() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MaximizeV', args);
    },
    /**
     * MaximizeH() -> null
     *
     * Sakura's F_MAXIMIZE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    MaximizeH() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MaximizeH', args);
    },
    /**
     * MinimizeAll() -> null
     *
     * Sakura's F_MINIMIZE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    MinimizeAll() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MinimizeAll', args);
    },
    /**
     * ReDraw() -> null
     *
     * Sakura's F_REDRAW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ReDraw() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ReDraw', args);
    },
    /**
     * ActivateWinOutput() -> null
     *
     * Sakura's F_WIN_OUTPUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ActivateWinOutput() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ActivateWinOutput', args);
    },
    /**
     * TraceOut(arg1: string?, arg2: int?) -> null
     *
     * MATSU Editor's own F_TRACEOUT (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    TraceOut(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TraceOut', args);
    },
    /**
     * WindowTopMost(arg1: int?) -> null
     *
     * Sakura's F_TOPMOST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    WindowTopMost(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WindowTopMost', args);
    },
    /**
     * GroupClose() -> null
     *
     * Sakura's F_GROUPCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    GroupClose() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GroupClose', args);
    },
    /**
     * NextGroup() -> null
     *
     * Sakura's F_NEXTGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    NextGroup() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('NextGroup', args);
    },
    /**
     * PrevGroup() -> null
     *
     * Sakura's F_PREVGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    PrevGroup() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('PrevGroup', args);
    },
    /**
     * TabMoveRight() -> null
     *
     * Sakura's F_TAB_MOVERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TabMoveRight() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TabMoveRight', args);
    },
    /**
     * TabMoveLeft() -> null
     *
     * Sakura's F_TAB_MOVELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TabMoveLeft() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TabMoveLeft', args);
    },
    /**
     * TabSeparate() -> null
     *
     * Sakura's F_TAB_SEPARATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TabSeparate() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TabSeparate', args);
    },
    /**
     * TabJointNext() -> null
     *
     * Sakura's F_TAB_JOINTNEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TabJointNext() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TabJointNext', args);
    },
    /**
     * TabJointPrev() -> null
     *
     * Sakura's F_TAB_JOINTPREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TabJointPrev() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TabJointPrev', args);
    },
    /**
     * TabCloseOther() -> null
     *
     * Sakura's F_TAB_CLOSEOTHER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TabCloseOther() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TabCloseOther', args);
    },
    /**
     * TabCloseLeft() -> null
     *
     * Sakura's F_TAB_CLOSELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TabCloseLeft() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TabCloseLeft', args);
    },
    /**
     * TabCloseRight() -> null
     *
     * Sakura's F_TAB_CLOSERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    TabCloseRight() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('TabCloseRight', args);
    },
    /**
     * Complete() -> null
     *
     * Sakura's F_HOKAN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    Complete() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Complete', args);
    },
    /**
     * ToggleKeyHelpSearch(arg1: int?) -> null
     *
     * Sakura's F_TOGGLE_KEY_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ToggleKeyHelpSearch(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ToggleKeyHelpSearch', args);
    },
    /**
     * HelpContents() -> null
     *
     * Sakura's F_HELP_CONTENTS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HelpContents() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HelpContents', args);
    },
    /**
     * HelpSearch() -> null
     *
     * Sakura's F_HELP_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    HelpSearch() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('HelpSearch', args);
    },
    /**
     * CommandList() -> null
     *
     * Sakura's F_MENU_ALLFUNC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    CommandList() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CommandList', args);
    },
    /**
     * ExtHelp1() -> null
     *
     * Sakura's F_EXTHELP1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    ExtHelp1() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExtHelp1', args);
    },
    /**
     * ExtHtmlHelp(arg1: string?, arg2: string?) -> null
     *
     * Sakura's F_EXTHTMLHELP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    ExtHtmlHelp(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExtHtmlHelp', args);
    },
    /**
     * About() -> null
     *
     * Sakura's F_ABOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    About() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('About', args);
    },
    /**
     * StatusMsg(arg1: string?, arg2: int?) -> null
     *
     * MATSU Editor's own F_STATUSMSG (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    StatusMsg(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('StatusMsg', args);
    },
    /**
     * MsgBeep(arg1: int?) -> null
     *
     * MATSU Editor's own F_MSGBEEP (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    MsgBeep(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MsgBeep', args);
    },
    /**
     * CommitUndoBuffer() -> null
     *
     * MATSU Editor's own F_COMMITUNDOBUFFER (not in Sakura). Runs the command that a key, the menu
     * and a key macro run.
     */
    CommitUndoBuffer() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CommitUndoBuffer', args);
    },
    /**
     * AddRefUndoBuffer() -> null
     *
     * MATSU Editor's own F_ADDREFUNDOBUFFER (not in Sakura). Runs the command that a key, the menu
     * and a key macro run.
     */
    AddRefUndoBuffer() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('AddRefUndoBuffer', args);
    },
    /**
     * SetUndoBuffer() -> null
     *
     * MATSU Editor's own F_SETUNDOBUFFER (not in Sakura). Runs the command that a key, the menu
     * and a key macro run.
     */
    SetUndoBuffer() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetUndoBuffer', args);
    },
    /**
     * AppendUndoBufferCursor() -> null
     *
     * MATSU Editor's own F_APPENDUNDOBUFFERCURSOR (not in Sakura). Runs the command that a key,
     * the menu and a key macro run.
     */
    AppendUndoBufferCursor() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('AppendUndoBufferCursor', args);
    },
    /**
     * ClipboardEmpty() -> null
     *
     * MATSU Editor's own F_CLIPBOARDEMPTY (not in Sakura). Runs the command that a key, the menu
     * and a key macro run.
     */
    ClipboardEmpty() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ClipboardEmpty', args);
    },
    /**
     * SetViewTop(arg1: int?) -> null
     *
     * MATSU Editor's own F_SETVIEWTOP (not in Sakura). Runs the command that a key, the menu and a
     * key macro run.
     */
    SetViewTop(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetViewTop', args);
    },
    /**
     * SetViewLeft(arg1: int?) -> null
     *
     * MATSU Editor's own F_SETVIEWLEFT (not in Sakura). Runs the command that a key, the menu and
     * a key macro run.
     */
    SetViewLeft(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetViewLeft', args);
    },
  };
  // Any other name that starts like a Sakura macro name (an upper case letter or a digit) is
  // sent to the editor as it is. The editor answers a name it does not know with an error.
  return new Proxy(editor, {
    get(target, name, receiver) {
      if (typeof name !== 'string' || name in target || !/^[0-9A-Z]/.test(name)) {
        return Reflect.get(target, name, receiver);
      }
      return function () {
        const args = Array.prototype.slice.call(arguments);
        while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
        return call(name, args);
      };
    },
  });
};
