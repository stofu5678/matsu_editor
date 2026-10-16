// GENERATED from crates/ume-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files
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
     * 0 when nothing is selected, 1 when text is selected. (Sakura returns 2 for a box
     * selection, which is not supported yet.)
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
     * Sakura's F_FILEOPEN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_PUTFILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_INSFILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_MOVECURSORLAYOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_INSBOXTEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_CHGMOD_EOL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_CHANGETYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_SET_QUOTESTRING (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_TEXTWRAPMETHOD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_WINMAXIMIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_WINMINIMIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_WINRESTORE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_TRACEOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_STATUSMSG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_MSGBEEP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_COMMITUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_ADDREFUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_SETUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_APPENDUNDOBUFFERCURSOR (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_CLIPBOARDEMPTY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_SETVIEWTOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
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
     * Sakura's F_SETVIEWLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    SetViewLeft(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetViewLeft', args);
    },
    /**
     * GetSaveFilename() -> string
     *
     * Sakura's F_GETSAVEFILENAME (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetSaveFilename() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetSaveFilename', args);
    },
    /**
     * ExpandParameter(arg1: string?) -> string
     *
     * Sakura's F_EXPANDPARAMETER (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    ExpandParameter(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ExpandParameter', args);
    },
    /**
     * ChangeTabWidth(arg1: int?) -> int
     *
     * Sakura's F_CHGTABWIDTH (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    ChangeTabWidth(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeTabWidth', args);
    },
    /**
     * GetSelectLineFrom() -> int
     *
     * Sakura's F_GETSELLINEFROM (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_GETSELCOLUMNFROM (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_GETSELCOLUMNFROM (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_GETSELLINETO (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_GETSELCOLUMNTO (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_GETSELCOLUMNTO (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_ISINSMODE (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_GETCHARCODE (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_GETLINECODE (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_ISPOSSIBLEUNDO (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_ISPOSSIBLEREDO (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    IsPossibleRedo() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsPossibleRedo', args);
    },
    /**
     * ChangeWrapColm(arg1: int?) -> int
     *
     * Sakura's F_CHGWRAPCOLUMN (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    ChangeWrapColm(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeWrapColm', args);
    },
    /**
     * ChangeWrapColumn(arg1: int?) -> int
     *
     * Sakura's F_CHGWRAPCOLUMN (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    ChangeWrapColumn(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeWrapColumn', args);
    },
    /**
     * IsCurTypeExt(arg1: string?) -> int
     *
     * Sakura's F_ISCURTYPEEXT (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    IsCurTypeExt(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsCurTypeExt', args);
    },
    /**
     * IsSameTypeExt(arg1: string?, arg2: string?) -> int
     *
     * Sakura's F_ISSAMETYPEEXT (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    IsSameTypeExt(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsSameTypeExt', args);
    },
    /**
     * InputBox(arg1: string?, arg2: string?, arg3: int?) -> string
     *
     * Sakura's F_INPUTBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    InputBox(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InputBox', args);
    },
    /**
     * MessageBox(arg1: string?, arg2: int?) -> int
     *
     * Sakura's F_MESSAGEBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    MessageBox(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('MessageBox', args);
    },
    /**
     * ErrorMsg(arg1: string?) -> int
     *
     * Sakura's F_ERRORMSG (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    ErrorMsg(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ErrorMsg', args);
    },
    /**
     * WarnMsg(arg1: string?) -> int
     *
     * Sakura's F_WARNMSG (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    WarnMsg(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('WarnMsg', args);
    },
    /**
     * InfoMsg(arg1: string?) -> int
     *
     * Sakura's F_INFOMSG (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    InfoMsg(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('InfoMsg', args);
    },
    /**
     * OkCancelBox(arg1: string?) -> int
     *
     * Sakura's F_OKCANCELBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    OkCancelBox(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('OkCancelBox', args);
    },
    /**
     * YesNoBox(arg1: string?) -> int
     *
     * Sakura's F_YESNOBOX (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    YesNoBox(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('YesNoBox', args);
    },
    /**
     * CompareVersion(arg1: string?, arg2: string?) -> int
     *
     * Sakura's F_COMPAREVERSION (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    CompareVersion(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CompareVersion', args);
    },
    /**
     * Sleep(arg1: int?) -> int
     *
     * Sakura's F_MACROSLEEP (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    Sleep(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('Sleep', args);
    },
    /**
     * FileOpenDialog(arg1: string?, arg2: string?) -> string
     *
     * Sakura's F_FILEOPENDIALOG (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    FileOpenDialog(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileOpenDialog', args);
    },
    /**
     * FileSaveDialog(arg1: string?, arg2: string?) -> string
     *
     * Sakura's F_FILESAVEDIALOG (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    FileSaveDialog(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FileSaveDialog', args);
    },
    /**
     * FolderDialog(arg1: string?, arg2: string?) -> string
     *
     * Sakura's F_FOLDERDIALOG (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    FolderDialog(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('FolderDialog', args);
    },
    /**
     * GetClipboard(arg1: int?) -> string
     *
     * Sakura's F_GETCLIPBOARD (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetClipboard(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetClipboard', args);
    },
    /**
     * SetClipboard(arg1: int?, arg2: string?) -> int
     *
     * Sakura's F_SETCLIPBOARD (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    SetClipboard(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetClipboard', args);
    },
    /**
     * LayoutToLogicLineNum(arg1: int?) -> int
     *
     * Sakura's F_LAYOUTTOLOGICLINENUM (sakura_core/macro/CSMacroMgr.cpp). A function with a
     * result. This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    LayoutToLogicLineNum(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LayoutToLogicLineNum', args);
    },
    /**
     * LogicToLayoutLineNum(arg1: int?, arg2: int?) -> int
     *
     * Sakura's F_LOGICTOLAYOUTLINENUM (sakura_core/macro/CSMacroMgr.cpp). A function with a
     * result. This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    LogicToLayoutLineNum(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LogicToLayoutLineNum', args);
    },
    /**
     * LineColumnToIndex(arg1: int?, arg2: int?) -> int
     *
     * Sakura's F_LINECOLUMNTOINDEX (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    LineColumnToIndex(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LineColumnToIndex', args);
    },
    /**
     * LineIndexToColumn(arg1: int?, arg2: int?) -> int
     *
     * Sakura's F_LINEINDEXTOCOLUMN (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    LineIndexToColumn(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('LineIndexToColumn', args);
    },
    /**
     * GetCookie(arg1: string?, arg2: string?) -> string
     *
     * Sakura's F_GETCOOKIE (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetCookie(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetCookie', args);
    },
    /**
     * GetCookieDefault(arg1: string?, arg2: string?, arg3: string?) -> string
     *
     * Sakura's F_GETCOOKIEDEFAULT (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetCookieDefault(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetCookieDefault', args);
    },
    /**
     * SetCookie(arg1: string?, arg2: string?, arg3: string?) -> int
     *
     * Sakura's F_SETCOOKIE (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    SetCookie(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetCookie', args);
    },
    /**
     * DeleteCookie(arg1: string?, arg2: string?) -> int
     *
     * Sakura's F_DELETECOOKIE (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    DeleteCookie(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('DeleteCookie', args);
    },
    /**
     * GetCookieNames(arg1: string?) -> string
     *
     * Sakura's F_GETCOOKIENAMES (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetCookieNames(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetCookieNames', args);
    },
    /**
     * SetDrawSwitch(arg1: int?) -> int
     *
     * Sakura's F_SETDRAWSWITCH (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    SetDrawSwitch(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetDrawSwitch', args);
    },
    /**
     * GetDrawSwitch() -> int
     *
     * Sakura's F_GETDRAWSWITCH (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_ISSHOWNSTATUS (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    IsShownStatus() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsShownStatus', args);
    },
    /**
     * GetStrWidth(arg1: string?, arg2: int?) -> int
     *
     * Sakura's F_GETSTRWIDTH (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetStrWidth(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetStrWidth', args);
    },
    /**
     * GetStrLayoutLength(arg1: string?, arg2: int?) -> int
     *
     * Sakura's F_GETSTRLAYOUTLENGTH (sakura_core/macro/CSMacroMgr.cpp). A function with a
     * result. This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetStrLayoutLength(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetStrLayoutLength', args);
    },
    /**
     * GetDefaultCharLength() -> int
     *
     * Sakura's F_GETDEFAULTCHARLENGTH (sakura_core/macro/CSMacroMgr.cpp). A function with a
     * result. This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetDefaultCharLength() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetDefaultCharLength', args);
    },
    /**
     * IsIncludeClipboardFormat(arg1: string?) -> int
     *
     * Sakura's F_ISINCLUDECLIPBOARDFORMAT (sakura_core/macro/CSMacroMgr.cpp). A function with a
     * result. This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    IsIncludeClipboardFormat(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsIncludeClipboardFormat', args);
    },
    /**
     * GetClipboardByFormat(arg1: string?, arg2: int?, arg3: int?) -> string
     *
     * Sakura's F_GETCLIPBOARDBYFORMAT (sakura_core/macro/CSMacroMgr.cpp). A function with a
     * result. This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetClipboardByFormat(arg1, arg2, arg3) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetClipboardByFormat', args);
    },
    /**
     * SetClipboardByFormat(arg1: string?, arg2: string?, arg3: int?, arg4: int?) -> int
     *
     * Sakura's F_SETCLIPBOARDBYFORMAT (sakura_core/macro/CSMacroMgr.cpp). A function with a
     * result. This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    SetClipboardByFormat(arg1, arg2, arg3, arg4) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('SetClipboardByFormat', args);
    },
    /**
     * GetLineAttribute(arg1: int?, arg2: int?) -> int
     *
     * Sakura's F_GETLINEATTRIBUTE (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetLineAttribute(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetLineAttribute', args);
    },
    /**
     * IsTextSelectingLock() -> int
     *
     * Sakura's F_ISTEXTSELECTINGLOCK (sakura_core/macro/CSMacroMgr.cpp). A function with a
     * result. This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    IsTextSelectingLock() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('IsTextSelectingLock', args);
    },
    /**
     * GetViewLines() -> int
     *
     * Sakura's F_GETVIEWLINES (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
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
     * Sakura's F_GETVIEWCOLUMNS (sakura_core/macro/CSMacroMgr.cpp). A function with a result.
     * This editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetViewColumns() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetViewColumns', args);
    },
    /**
     * CreateMenu(arg1: int?, arg2: string?) -> int
     *
     * Sakura's F_CREATEMENU (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    CreateMenu(arg1, arg2) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('CreateMenu', args);
    },
    /**
     * GetViewTop() -> int
     *
     * Sakura's F_GETVIEWTOP (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    GetViewTop() {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('GetViewTop', args);
    },
    /**
     * ChangeInsertSpace(arg1: int?) -> int
     *
     * Sakura's F_CHGINSSPACE (sakura_core/macro/CSMacroMgr.cpp). A function with a result. This
     * editor answers the error -32001 (not implemented) while it cannot answer it.
     */
    ChangeInsertSpace(arg1) {
      // A trailing undefined is a missing argument, not a null.
      const args = Array.prototype.slice.call(arguments);
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      return call('ChangeInsertSpace', args);
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
