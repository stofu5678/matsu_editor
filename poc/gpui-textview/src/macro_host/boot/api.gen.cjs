// GENERATED from src/macro_host/api.rs (FUNCS). Do not edit. Regenerate with: UME_UPDATE_GENERATED=1 cargo test generated_files
'use strict';

module.exports = function install(call) {
  return {
    /**
     * GetLineCount(reserved: int) -> int
     *
     * The number of logical lines (lines separated by line breaks). The text after the last line
     * break is not a line unless it is not empty. The argument is reserved and must be 0.
     */
    GetLineCount(reserved) {
      return call('GetLineCount', Array.prototype.slice.call(arguments));
    },
    /**
     * GetLineStr(line: int) -> string
     *
     * A logical line with its line break ("\n"; the last line has none unless the text ends with
     * one). 0 is the line of the cursor, 1 is the first line. Past the last line it is an empty
     * string.
     */
    GetLineStr(line) {
      return call('GetLineStr', Array.prototype.slice.call(arguments));
    },
    /**
     * InsText(text: string) -> null
     *
     * Inserts the text at the cursor, replacing the selection. The cursor ends after the text.
     */
    InsText(text) {
      return call('InsText', Array.prototype.slice.call(arguments));
    },
    /**
     * AddTail(text: string) -> null
     *
     * Moves to the end of the file, drops the selection, and inserts the text there.
     */
    AddTail(text) {
      return call('AddTail', Array.prototype.slice.call(arguments));
    },
    /**
     * GoFileTop() -> null
     *
     * Moves the cursor to the start of the file and drops the selection.
     */
    GoFileTop() {
      return call('GoFileTop', Array.prototype.slice.call(arguments));
    },
    /**
     * GoFileEnd() -> null
     *
     * Moves the cursor to the end of the file and drops the selection.
     */
    GoFileEnd() {
      return call('GoFileEnd', Array.prototype.slice.call(arguments));
    },
    /**
     * Up() -> null
     *
     * Moves the cursor up one line and drops the selection.
     */
    Up() {
      return call('Up', Array.prototype.slice.call(arguments));
    },
    /**
     * Down() -> null
     *
     * Moves the cursor down one line and drops the selection.
     */
    Down() {
      return call('Down', Array.prototype.slice.call(arguments));
    },
    /**
     * SelectAll() -> null
     *
     * Selects the whole text. The cursor goes to the end.
     */
    SelectAll() {
      return call('SelectAll', Array.prototype.slice.call(arguments));
    },
    /**
     * GetSelectedString(reserved: int) -> string
     *
     * The selected text with its line breaks ("\n"), or an empty string when nothing is
     * selected. The argument is reserved and ignored.
     */
    GetSelectedString(reserved) {
      return call('GetSelectedString', Array.prototype.slice.call(arguments));
    },
    /**
     * IsTextSelected() -> int
     *
     * 0 when nothing is selected, 1 when text is selected. (Sakura returns 2 for a box
     * selection, which the PoC does not have.)
     */
    IsTextSelected() {
      return call('IsTextSelected', Array.prototype.slice.call(arguments));
    },
    /**
     * MoveCursor(line: int, column: int, option: int) -> null
     *
     * Moves the cursor to a line and a column, both counted from 1. The column counts UTF-16
     * code units. Option: 0 does not select, 1 selects from the old position. (2, the box
     * selection, is not supported.) Past the end of the text it goes to the end of the text.
     */
    MoveCursor(line, column, option) {
      return call('MoveCursor', Array.prototype.slice.call(arguments));
    },
    /**
     * Jump(line: int, flag: int?) -> null
     *
     * Moves to the start of a line, counted from 1 in logical lines. Lines before 1 and after
     * the last one are limited to the first and the last line. Flag bit 0 (1, the default) is
     * the line break unit, which is the only unit that the PoC has; bit 1 (PL/SQL) is not
     * supported.
     */
    Jump(line, flag) {
      return call('Jump', Array.prototype.slice.call(arguments));
    },
    /**
     * GetFilename() -> string
     *
     * The full path of the file, or an empty string when the text has no file.
     */
    GetFilename() {
      return call('GetFilename', Array.prototype.slice.call(arguments));
    },
  };
};
