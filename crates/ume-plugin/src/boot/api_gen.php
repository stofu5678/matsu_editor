<?php
// GENERATED from crates/ume-plugin/src/catalog.rs (api.rs FUNCS and Sakura's macro tables). Do not edit. Regenerate with: UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files

/**
 * The editor's macro API (the names and the arguments of Sakura's macros).
 */
final class Editor
{
    /** What the editor told the macro in `initialize`, or null. */
    public static $info = null;

    /** @var callable */
    private static $call;

    /** Used by the boot library; a macro does not call this. */
    public static function install(callable $call): void
    {
        self::$call = $call;
    }

    /**
     * GetLineCount(reserved: int) -> int
     *
     * The number of logical lines (lines separated by line breaks). The text after the last line
     * break is not a line unless it is not empty. The argument is reserved and must be 0.
     */
    public static function GetLineCount($reserved)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetLineCount', func_get_args());
    }

    /**
     * GetLineStr(line: int) -> string
     *
     * A logical line with its line break ("\n"; the last line has none unless the text ends with
     * one). 0 is the line of the cursor, 1 is the first line. Past the last line it is an empty
     * string.
     */
    public static function GetLineStr($line)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetLineStr', func_get_args());
    }

    /**
     * InsText(text: string) -> null
     *
     * Inserts the text at the cursor, replacing the selection. The cursor ends after the text.
     */
    public static function InsText($text)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InsText', func_get_args());
    }

    /**
     * AddTail(text: string) -> null
     *
     * Moves to the end of the file, drops the selection, and inserts the text there.
     */
    public static function AddTail($text)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('AddTail', func_get_args());
    }

    /**
     * GoFileTop() -> null
     *
     * Moves the cursor to the start of the file and drops the selection.
     */
    public static function GoFileTop()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoFileTop', func_get_args());
    }

    /**
     * GoFileEnd() -> null
     *
     * Moves the cursor to the end of the file and drops the selection.
     */
    public static function GoFileEnd()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoFileEnd', func_get_args());
    }

    /**
     * Up() -> null
     *
     * Moves the cursor up one line and drops the selection.
     */
    public static function Up()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Up', func_get_args());
    }

    /**
     * Down() -> null
     *
     * Moves the cursor down one line and drops the selection.
     */
    public static function Down()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Down', func_get_args());
    }

    /**
     * SelectAll() -> null
     *
     * Selects the whole text. The cursor goes to the end.
     */
    public static function SelectAll()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SelectAll', func_get_args());
    }

    /**
     * GetSelectedString(reserved: int) -> string
     *
     * The selected text with its line breaks ("\n"), or an empty string when nothing is
     * selected. The argument is reserved and ignored.
     */
    public static function GetSelectedString($reserved)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetSelectedString', func_get_args());
    }

    /**
     * IsTextSelected() -> int
     *
     * 0 when nothing is selected, 1 when text is selected, 2 when a box selection is selected.
     */
    public static function IsTextSelected()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsTextSelected', func_get_args());
    }

    /**
     * MoveCursor(line: int, column: int, option: int) -> null
     *
     * Moves the cursor to a line and a column, both counted from 1. The column counts UTF-16
     * code units. Option: 0 does not select, 1 selects from the old position. (2, the box
     * selection, is not supported.) Past the end of the text it goes to the end of the text.
     */
    public static function MoveCursor($line, $column, $option)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MoveCursor', func_get_args());
    }

    /**
     * Jump(line: int, flag: int?) -> null
     *
     * Moves to the start of a line, counted from 1 in logical lines. Lines before 1 and after
     * the last one are limited to the first and the last line. Flag bit 0 (1, the default) is
     * the line break unit, which is the only unit supported yet; bit 1 (PL/SQL) is not
     * supported.
     */
    public static function Jump($line, $flag = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Jump', func_get_args());
    }

    /**
     * GetFilename() -> string
     *
     * The full path of the file, or an empty string when the text has no file.
     */
    public static function GetFilename()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetFilename', func_get_args());
    }

    /**
     * GetSelectLineFrom() -> int
     *
     * The line where the selection starts, counted from 1 in layout lines (wrapped rows count as
     * lines). 0 when nothing is selected.
     */
    public static function GetSelectLineFrom()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetSelectLineFrom', func_get_args());
    }

    /**
     * GetSelectColmFrom() -> int
     *
     * The column where the selection starts, counted from 1 in layout columns (cells: a
     * full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected. Same
     * as GetSelectColumnFrom.
     */
    public static function GetSelectColmFrom()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetSelectColmFrom', func_get_args());
    }

    /**
     * GetSelectColumnFrom() -> int
     *
     * The column where the selection starts, counted from 1 in layout columns (cells: a
     * full-width character is 2, a tab goes to the tab stop). 0 when nothing is selected.
     */
    public static function GetSelectColumnFrom()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetSelectColumnFrom', func_get_args());
    }

    /**
     * GetSelectLineTo() -> int
     *
     * The line where the selection ends, counted from 1 in layout lines. 0 when nothing is
     * selected.
     */
    public static function GetSelectLineTo()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetSelectLineTo', func_get_args());
    }

    /**
     * GetSelectColmTo() -> int
     *
     * The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
     * selected. Same as GetSelectColumnTo.
     */
    public static function GetSelectColmTo()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetSelectColmTo', func_get_args());
    }

    /**
     * GetSelectColumnTo() -> int
     *
     * The column where the selection ends, counted from 1 in layout columns. 0 when nothing is
     * selected.
     */
    public static function GetSelectColumnTo()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetSelectColumnTo', func_get_args());
    }

    /**
     * IsInsMode() -> int
     *
     * 1 in insert mode, 0 in overwrite mode.
     */
    public static function IsInsMode()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsInsMode', func_get_args());
    }

    /**
     * GetCharCode() -> int
     *
     * The encoding of the file: 0 Shift_JIS, 1 JIS, 2 EUC-JP, 3 UTF-16, 4 UTF-8, 5 UTF-7, 6
     * UTF-16 BE, 7 CESU-8, 8 Latin1.
     */
    public static function GetCharCode()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetCharCode', func_get_args());
    }

    /**
     * GetLineCode() -> int
     *
     * The line ending that new line breaks get: 0 CR LF, 1 CR, 2 LF (3 NEL, 4 LS, 5 PS).
     */
    public static function GetLineCode()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetLineCode', func_get_args());
    }

    /**
     * IsPossibleUndo() -> int
     *
     * 1 when there is something to undo, else 0.
     */
    public static function IsPossibleUndo()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsPossibleUndo', func_get_args());
    }

    /**
     * IsPossibleRedo() -> int
     *
     * 1 when there is something to redo, else 0.
     */
    public static function IsPossibleRedo()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsPossibleRedo', func_get_args());
    }

    /**
     * GetLineAttribute(line: int, type: int) -> int
     *
     * An attribute of a logical line (0 is the line of the cursor, 1 the first line). Type 0 is
     * 1 when the line changed since the last save, type 1 the number of the undo step that last
     * touched it, type 2 the bookmark (1 or 0). Types 3 (diff mark) and 4 (function list mark)
     * answer the error -32001 until the editor has them. A negative line, a line past the end
     * and any other type are errors.
     */
    public static function GetLineAttribute($line, $type)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetLineAttribute', func_get_args());
    }

    /**
     * IsTextSelectingLock() -> int
     *
     * 0 when the selection is not locked, 1 when it is locked (plain moves extend the
     * selection), 2 when it is locked and a box selection.
     */
    public static function IsTextSelectingLock()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsTextSelectingLock', func_get_args());
    }

    /**
     * GetSaveFilename() -> string
     *
     * The full path that a save writes to, or an empty string when the text has no file. The
     * same as GetFilename, except in the macro that runs before a save (the registered macro of
     * 保存前), where it is the file being saved.
     */
    public static function GetSaveFilename()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetSaveFilename', func_get_args());
    }

    /**
     * ChangeInsertSpace(value: int) -> int
     *
     * Whether the Tab key inserts spaces instead of a tab character, before the call (1 or 0). 0
     * turns it off, 1 turns it on for this window (not saved); any other value only reads. Only
     * the low 16 bits of the value count, as in Sakura.
     */
    public static function ChangeInsertSpace($value)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ChangeInsertSpace', func_get_args());
    }

    /**
     * LayoutToLogicLineNum(layoutLine: int) -> int
     *
     * The logical line (up to a line break) that a layout line (a row on the screen) belongs to,
     * both counted from 1. The line after the last layout line gives the line after the last
     * logical line. Any other line outside the text is an error.
     */
    public static function LayoutToLogicLineNum($layoutLine)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LayoutToLogicLineNum', func_get_args());
    }

    /**
     * LogicToLayoutLineNum(logicLine: int, logicIndex: int) -> int
     *
     * The layout line (a row on the screen) of a position in a logical line: the line and the
     * index in it (UTF-16 code units), both counted from 1. A line past the end gives the line
     * after the last layout line.
     */
    public static function LogicToLayoutLineNum($logicLine, $logicIndex)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LogicToLayoutLineNum', func_get_args());
    }

    /**
     * LineColumnToIndex(layoutLine: int, layoutColumn: int) -> int
     *
     * The index (UTF-16 code units, from 1) in the logical line of a layout line and a layout
     * column (cells), both from 1. A column past the text gives the end of the text of the row.
     */
    public static function LineColumnToIndex($layoutLine, $layoutColumn)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LineColumnToIndex', func_get_args());
    }

    /**
     * LineIndexToColumn(logicLine: int, logicIndex: int) -> int
     *
     * The layout column (cells, from 1) of a position in a logical line: the line and the index
     * in it (UTF-16 code units), both from 1. Tabs and full-width characters take their width.
     */
    public static function LineIndexToColumn($logicLine, $logicIndex)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LineIndexToColumn', func_get_args());
    }

    /**
     * GetStrWidth(text: string, column: int?) -> int
     *
     * The width of a string in cells, starting at layout column `column` (from 1, default 1; it
     * matters for tabs). A CR LF at the end counts as the CR alone.
     */
    public static function GetStrWidth($text, $column = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetStrWidth', func_get_args());
    }

    /**
     * GetStrLayoutLength(text: string, column: int?) -> int
     *
     * The width of a string as a line of the document in cells, starting at layout column
     * `column` (from 1, default 1). A line ending at the end takes one cell.
     */
    public static function GetStrLayoutLength($text, $column = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetStrLayoutLength', func_get_args());
    }

    /**
     * GetDefaultCharLength() -> int
     *
     * The layout width of a half-width character. This editor lays out in cells, so it is 1.
     */
    public static function GetDefaultCharLength()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetDefaultCharLength', func_get_args());
    }

    /**
     * GetViewLines() -> int
     *
     * How many rows the text area shows (at least 1).
     */
    public static function GetViewLines()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetViewLines', func_get_args());
    }

    /**
     * GetViewColumns() -> int
     *
     * How many columns the text area shows (at least 1).
     */
    public static function GetViewColumns()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetViewColumns', func_get_args());
    }

    /**
     * GetViewTop() -> int
     *
     * The layout line at the top of the text area, counted from 1.
     */
    public static function GetViewTop()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetViewTop', func_get_args());
    }

    /**
     * ChangeTabWidth(width: int) -> int
     *
     * The tab width before the call. A width from 1 to 64 that differs from it becomes the tab
     * width of this window (not saved). Any other value, such as 0, only reads.
     */
    public static function ChangeTabWidth($width)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ChangeTabWidth', func_get_args());
    }

    /**
     * ChangeWrapColm(column: int) -> int
     *
     * The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
     * column (not saved). Any other value only reads. Same as ChangeWrapColumn.
     */
    public static function ChangeWrapColm($column)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ChangeWrapColm', func_get_args());
    }

    /**
     * ChangeWrapColumn(column: int) -> int
     *
     * The wrap column before the call. A column from 10 to 10240 makes this window wrap at that
     * column (not saved). Any other value only reads.
     */
    public static function ChangeWrapColumn($column)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ChangeWrapColumn', func_get_args());
    }

    /**
     * GetClipboard(option: int?) -> string
     *
     * The text on the clipboard, or an empty string when there is none. The line endings are
     * those of the source. The option is reserved.
     */
    public static function GetClipboard($option = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetClipboard', func_get_args());
    }

    /**
     * SetClipboard(option: int?, text: string?) -> int
     *
     * Puts the text on the clipboard. Option bit 0 marks a box copy (a paste puts it in as a
     * box), bit 1 a row copy (a paste puts it in front of the row); with both, the box wins.
     * Returns 1 when it worked, 0 when not.
     */
    public static function SetClipboard($option = null, $text = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetClipboard', func_get_args());
    }

    /**
     * IsIncludeClipboardFormat(format: string) -> int
     *
     * 1 when the clipboard has the format. Only text is known: CF_TEXT, CF_OEMTEXT and
     * CF_UNICODETEXT (or 1, 7, 13) are 1 when the clipboard has text; every other format is 0.
     */
    public static function IsIncludeClipboardFormat($format)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsIncludeClipboardFormat', func_get_args());
    }

    /**
     * GetClipboardByFormat(format: string, mode: int, endMode: int?) -> string
     *
     * The text of the clipboard in a text format (CF_TEXT, CF_OEMTEXT, CF_UNICODETEXT), with the
     * line endings of the document. Mode -2 is the usual way and 3 is UTF-16 (the text itself);
     * the other modes and formats are errors (-32001). The end mode is not used.
     */
    public static function GetClipboardByFormat($format, $mode, $endMode = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetClipboardByFormat', func_get_args());
    }

    /**
     * SetClipboardByFormat(text: string, format: string, mode: int, endMode: int?) -> int
     *
     * Puts the text on the clipboard as text. The formats and modes are those of
     * GetClipboardByFormat. Returns 1 when it worked.
     */
    public static function SetClipboardByFormat($text, $format, $mode, $endMode = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetClipboardByFormat', func_get_args());
    }

    /**
     * GetCookie(scope: string, name: string) -> string
     *
     * The value of a cookie, or an empty string when there is none. The scope is "window" or
     * "document".
     */
    public static function GetCookie($scope, $name)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetCookie', func_get_args());
    }

    /**
     * GetCookieDefault(scope: string, name: string, defaultValue: string) -> string
     *
     * The value of a cookie, or the default when there is none (an unknown scope still gives an
     * empty string). The scope is "window" or "document".
     */
    public static function GetCookieDefault($scope, $name, $defaultValue)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetCookieDefault', func_get_args());
    }

    /**
     * SetCookie(scope: string, name: string, value: string) -> int
     *
     * Sets a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a name with a
     * character that a name cannot have (Sakura lets in digits, letters and every character from
     * "_" up in the character table). The scope is "window" or "document".
     */
    public static function SetCookie($scope, $name, $value)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetCookie', func_get_args());
    }

    /**
     * DeleteCookie(scope: string, name: string) -> int
     *
     * Deletes a cookie. Returns 0 when it worked, 1 for an unknown scope, 2 for a bad name, 5
     * when there is no such cookie. The scope is "window" or "document".
     */
    public static function DeleteCookie($scope, $name)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DeleteCookie', func_get_args());
    }

    /**
     * GetCookieNames(scope: string) -> string
     *
     * The names of the cookies of a scope, joined with commas, in the order of the names (UTF-16
     * code units). An unknown scope gives an empty string. The scope is "window" or "document".
     */
    public static function GetCookieNames($scope)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetCookieNames', func_get_args());
    }

    /**
     * CompareVersion(versionA: string, versionB: string) -> int
     *
     * Compares two version strings: 0 when equal, positive when A is newer, negative when B is.
     * Four parts at most; a part has two digits at most; "alpha", "beta", "rc" and "pl" put a
     * part before or after the release (like PHP's version_compare).
     */
    public static function CompareVersion($versionA, $versionB)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CompareVersion', func_get_args());
    }

    /**
     * SetDrawSwitch(on: int) -> int
     *
     * Turns the draw switch on (non-zero) or off (0) and returns the state before (1 or 0). The
     * editor keeps the flag for GetDrawSwitch but still paints the text when it likes.
     */
    public static function SetDrawSwitch($on)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetDrawSwitch', func_get_args());
    }

    /**
     * GetDrawSwitch() -> int
     *
     * 1 when the draw switch is on (the start state), 0 when a macro turned it off.
     */
    public static function GetDrawSwitch()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GetDrawSwitch', func_get_args());
    }

    /**
     * IsShownStatus() -> int
     *
     * 1 when the status bar is shown, else 0.
     */
    public static function IsShownStatus()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsShownStatus', func_get_args());
    }

    /**
     * IsCurTypeExt(extension: string) -> int
     *
     * 1 when the document type of this file is the type of the extension (looked up in the
     * extension lists of the types in the settings file, as a file name). Without type settings
     * every extension is the basic type.
     */
    public static function IsCurTypeExt($extension)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsCurTypeExt', func_get_args());
    }

    /**
     * IsSameTypeExt(extension1: string, extension2: string) -> int
     *
     * 1 when the two extensions belong to the same document type (see IsCurTypeExt).
     */
    public static function IsSameTypeExt($extension1, $extension2)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IsSameTypeExt', func_get_args());
    }

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
    public static function ExpandParameter($format)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExpandParameter', func_get_args());
    }

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
    public static function MessageBox($message, $type = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MessageBox', func_get_args());
    }

    /**
     * ErrorMsg(message: string) -> int
     *
     * Shows an error message with an OK button and waits. Returns 1.
     */
    public static function ErrorMsg($message)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ErrorMsg', func_get_args());
    }

    /**
     * WarnMsg(message: string) -> int
     *
     * Shows a warning with an OK button and waits. Returns 1.
     */
    public static function WarnMsg($message)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WarnMsg', func_get_args());
    }

    /**
     * InfoMsg(message: string) -> int
     *
     * Shows an information message with an OK button and waits. Returns 1.
     */
    public static function InfoMsg($message)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InfoMsg', func_get_args());
    }

    /**
     * OkCancelBox(message: string) -> int
     *
     * Asks a question with OK and Cancel and waits. Returns 1 (OK) or 2 (Cancel).
     */
    public static function OkCancelBox($message)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('OkCancelBox', func_get_args());
    }

    /**
     * YesNoBox(message: string) -> int
     *
     * Asks a question with Yes and No and waits. Returns 6 (Yes) or 7 (No).
     */
    public static function YesNoBox($message)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('YesNoBox', func_get_args());
    }

    /**
     * FileOpenDialog(defaultName: string?, filter: string?) -> string
     *
     * Lets the user choose a file to open and returns its full path, or an empty string when the
     * dialog is cancelled. The default name and the filter are not used: the dialog of the
     * platform takes neither.
     */
    public static function FileOpenDialog($defaultName = null, $filter = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileOpenDialog', func_get_args());
    }

    /**
     * FileSaveDialog(defaultName: string?, filter: string?) -> string
     *
     * Lets the user choose a file name to save under and returns the full path, or an empty
     * string when the dialog is cancelled. The dialog starts with the default name. The filter
     * is not used.
     */
    public static function FileSaveDialog($defaultName = null, $filter = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileSaveDialog', func_get_args());
    }

    /**
     * FolderDialog(message: string?, defaultFolder: string?) -> string
     *
     * Lets the user choose a folder and returns its full path, or an empty string when the
     * dialog is cancelled. The message becomes the prompt text of the platform dialog where it
     * has one; the default folder is not used.
     */
    public static function FolderDialog($message = null, $defaultFolder = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FolderDialog', func_get_args());
    }

    /**
     * InputBox(message: string, defaultValue: string?, maxLength: int?) -> string
     *
     * Shows a dialog with a text field and waits. The message is shown above the field, the
     * default value is in the field, and the text cannot be longer than the maximum length (in
     * UTF-16 code units; 0 or less, or no value, is 260). Returns the text of the field when OK
     * is pressed, or an empty string when the dialog is cancelled.
     */
    public static function InputBox($message, $defaultValue = null, $maxLength = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InputBox', func_get_args());
    }

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
    public static function CreateMenu($position, $menu)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CreateMenu', func_get_args());
    }

    /**
     * Sleep(milliseconds: int) -> int
     *
     * Waits for the given number of milliseconds (the window stays alive, and the macro can be
     * cancelled). Returns 0.
     */
    public static function Sleep($milliseconds)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Sleep', func_get_args());
    }

    /**
     * FileNew() -> null
     *
     * Sakura's F_FILENEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function FileNew()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileNew', func_get_args());
    }

    /**
     * FileOpen(arg1: string?, arg2: int?, arg3: int?, arg4: string?) -> null
     *
     * Sakura's F_FILEOPEN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function FileOpen($arg1 = null, $arg2 = null, $arg3 = null, $arg4 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileOpen', func_get_args());
    }

    /**
     * FileSave() -> null
     *
     * Sakura's F_FILESAVE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function FileSave()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileSave', func_get_args());
    }

    /**
     * FileSaveAll() -> null
     *
     * Sakura's F_FILESAVEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileSaveAll()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileSaveAll', func_get_args());
    }

    /**
     * FileSaveAsDialog(arg1: string?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_FILESAVEAS_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileSaveAsDialog($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileSaveAsDialog', func_get_args());
    }

    /**
     * FileSaveAs(arg1: string?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_FILESAVEAS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function FileSaveAs($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileSaveAs', func_get_args());
    }

    /**
     * FileClose() -> null
     *
     * Sakura's F_FILECLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function FileClose()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileClose', func_get_args());
    }

    /**
     * FileCloseOpen() -> null
     *
     * Sakura's F_FILECLOSE_OPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileCloseOpen()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileCloseOpen', func_get_args());
    }

    /**
     * FileReopen(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopen($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopen', func_get_args());
    }

    /**
     * FileReopenSJIS(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenSJIS($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenSJIS', func_get_args());
    }

    /**
     * FileReopenJIS(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenJIS($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenJIS', func_get_args());
    }

    /**
     * FileReopenEUC(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenEUC($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenEUC', func_get_args());
    }

    /**
     * FileReopenLatin1(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_LATIN1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenLatin1($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenLatin1', func_get_args());
    }

    /**
     * FileReopenUNICODE(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_UNICODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenUNICODE($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenUNICODE', func_get_args());
    }

    /**
     * FileReopenUNICODEBE(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_UNICODEBE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
     * a key, the menu and a key macro run. The last arguments may be left out. A command that
     * this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenUNICODEBE($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenUNICODEBE', func_get_args());
    }

    /**
     * FileReopenUTF8(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenUTF8($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenUTF8', func_get_args());
    }

    /**
     * FileReopenCESU8(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_CESU8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenCESU8($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenCESU8', func_get_args());
    }

    /**
     * FileReopenUTF7(arg1: int?) -> null
     *
     * Sakura's F_FILE_REOPEN_UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FileReopenUTF7($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FileReopenUTF7', func_get_args());
    }

    /**
     * Print() -> null
     *
     * Sakura's F_PRINT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Print()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Print', func_get_args());
    }

    /**
     * PrintPreview() -> null
     *
     * Sakura's F_PRINT_PREVIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function PrintPreview()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PrintPreview', func_get_args());
    }

    /**
     * PrintPageSetup() -> null
     *
     * Sakura's F_PRINT_PAGESETUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function PrintPageSetup()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PrintPageSetup', func_get_args());
    }

    /**
     * OpenHfromtoC() -> null
     *
     * Sakura's F_OPEN_HfromtoC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function OpenHfromtoC()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('OpenHfromtoC', func_get_args());
    }

    /**
     * ActivateSQLPLUS() -> null
     *
     * Sakura's F_ACTIVATE_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ActivateSQLPLUS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ActivateSQLPLUS', func_get_args());
    }

    /**
     * ExecSQLPLUS() -> null
     *
     * Sakura's F_PLSQL_COMPILE_ON_SQLPLUS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ExecSQLPLUS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExecSQLPLUS', func_get_args());
    }

    /**
     * Browse() -> null
     *
     * Sakura's F_BROWSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Browse()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Browse', func_get_args());
    }

    /**
     * ViewMode() -> null
     *
     * Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ViewMode()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ViewMode', func_get_args());
    }

    /**
     * ReadOnly() -> null
     *
     * Sakura's F_VIEWMODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ReadOnly()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ReadOnly', func_get_args());
    }

    /**
     * PropertyFile() -> null
     *
     * Sakura's F_PROPERTY_FILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function PropertyFile()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PropertyFile', func_get_args());
    }

    /**
     * ExitAllEditors() -> null
     *
     * Sakura's F_EXITALLEDITORS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ExitAllEditors()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExitAllEditors', func_get_args());
    }

    /**
     * ExitAll() -> null
     *
     * Sakura's F_EXITALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ExitAll()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExitAll', func_get_args());
    }

    /**
     * PutFile(arg1: string?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_PUTFILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function PutFile($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PutFile', func_get_args());
    }

    /**
     * InsFile(arg1: string?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_INSFILE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function InsFile($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InsFile', func_get_args());
    }

    /**
     * Char(arg1: int?) -> null
     *
     * Sakura's F_WCHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Char($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Char', func_get_args());
    }

    /**
     * CharIme(arg1: int?) -> null
     *
     * Sakura's F_IME_CHAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CharIme($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CharIme', func_get_args());
    }

    /**
     * Undo() -> null
     *
     * Sakura's F_UNDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Undo()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Undo', func_get_args());
    }

    /**
     * Redo() -> null
     *
     * Sakura's F_REDO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Redo()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Redo', func_get_args());
    }

    /**
     * Delete() -> null
     *
     * Sakura's F_DELETE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Delete()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Delete', func_get_args());
    }

    /**
     * DeleteBack() -> null
     *
     * Sakura's F_DELETE_BACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function DeleteBack()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DeleteBack', func_get_args());
    }

    /**
     * WordDeleteToStart() -> null
     *
     * Sakura's F_WordDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WordDeleteToStart()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordDeleteToStart', func_get_args());
    }

    /**
     * WordDeleteToEnd() -> null
     *
     * Sakura's F_WordDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WordDeleteToEnd()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordDeleteToEnd', func_get_args());
    }

    /**
     * WordCut() -> null
     *
     * Sakura's F_WordCut (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WordCut()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordCut', func_get_args());
    }

    /**
     * WordDelete() -> null
     *
     * Sakura's F_WordDelete (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WordDelete()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordDelete', func_get_args());
    }

    /**
     * LineCutToStart() -> null
     *
     * Sakura's F_LineCutToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function LineCutToStart()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LineCutToStart', func_get_args());
    }

    /**
     * LineCutToEnd() -> null
     *
     * Sakura's F_LineCutToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function LineCutToEnd()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LineCutToEnd', func_get_args());
    }

    /**
     * LineDeleteToStart() -> null
     *
     * Sakura's F_LineDeleteToStart (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function LineDeleteToStart()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LineDeleteToStart', func_get_args());
    }

    /**
     * LineDeleteToEnd() -> null
     *
     * Sakura's F_LineDeleteToEnd (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function LineDeleteToEnd()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LineDeleteToEnd', func_get_args());
    }

    /**
     * CutLine() -> null
     *
     * Sakura's F_CUT_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CutLine()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CutLine', func_get_args());
    }

    /**
     * DeleteLine() -> null
     *
     * Sakura's F_DELETE_LINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function DeleteLine()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DeleteLine', func_get_args());
    }

    /**
     * DuplicateLine() -> null
     *
     * Sakura's F_DUPLICATELINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function DuplicateLine()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DuplicateLine', func_get_args());
    }

    /**
     * IndentTab() -> null
     *
     * Sakura's F_INDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function IndentTab()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IndentTab', func_get_args());
    }

    /**
     * UnindentTab() -> null
     *
     * Sakura's F_UNINDENT_TAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function UnindentTab()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('UnindentTab', func_get_args());
    }

    /**
     * IndentSpace() -> null
     *
     * Sakura's F_INDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function IndentSpace()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('IndentSpace', func_get_args());
    }

    /**
     * UnindentSpace() -> null
     *
     * Sakura's F_UNINDENT_SPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function UnindentSpace()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('UnindentSpace', func_get_args());
    }

    /**
     * LTrim() -> null
     *
     * Sakura's F_LTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function LTrim()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('LTrim', func_get_args());
    }

    /**
     * RTrim() -> null
     *
     * Sakura's F_RTRIM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function RTrim()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('RTrim', func_get_args());
    }

    /**
     * SortAsc() -> null
     *
     * Sakura's F_SORT_ASC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SortAsc()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SortAsc', func_get_args());
    }

    /**
     * SortDesc() -> null
     *
     * Sakura's F_SORT_DESC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SortDesc()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SortDesc', func_get_args());
    }

    /**
     * Merge() -> null
     *
     * Sakura's F_MERGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Merge()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Merge', func_get_args());
    }

    /**
     * Left() -> null
     *
     * Sakura's F_LEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Left()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Left', func_get_args());
    }

    /**
     * Right() -> null
     *
     * Sakura's F_RIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Right()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Right', func_get_args());
    }

    /**
     * Up2() -> null
     *
     * Sakura's F_UP2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Up2()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Up2', func_get_args());
    }

    /**
     * Down2() -> null
     *
     * Sakura's F_DOWN2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Down2()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Down2', func_get_args());
    }

    /**
     * WordLeft() -> null
     *
     * Sakura's F_WORDLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WordLeft()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordLeft', func_get_args());
    }

    /**
     * WordRight() -> null
     *
     * Sakura's F_WORDRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WordRight()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordRight', func_get_args());
    }

    /**
     * GoLineTop(arg1: int?) -> null
     *
     * Sakura's F_GOLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function GoLineTop($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoLineTop', func_get_args());
    }

    /**
     * GoLineEnd(arg1: int?) -> null
     *
     * Sakura's F_GOLINEEND (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function GoLineEnd($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoLineEnd', func_get_args());
    }

    /**
     * HalfPageUp(arg1: int?) -> null
     *
     * Sakura's F_HalfPageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function HalfPageUp($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HalfPageUp', func_get_args());
    }

    /**
     * HalfPageDown(arg1: int?) -> null
     *
     * Sakura's F_HalfPageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HalfPageDown($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HalfPageDown', func_get_args());
    }

    /**
     * PageUp(arg1: int?) -> null
     *
     * Sakura's F_1PageUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function PageUp($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PageUp', func_get_args());
    }

    /**
     * PageDown(arg1: int?) -> null
     *
     * Sakura's F_1PageDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function PageDown($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PageDown', func_get_args());
    }

    /**
     * CurLineCenter() -> null
     *
     * Sakura's F_CURLINECENTER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CurLineCenter()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CurLineCenter', func_get_args());
    }

    /**
     * CurLineTop() -> null
     *
     * Sakura's F_CURLINETOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CurLineTop()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CurLineTop', func_get_args());
    }

    /**
     * CurLineBottom() -> null
     *
     * Sakura's F_CURLINEBOTTOM (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CurLineBottom()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CurLineBottom', func_get_args());
    }

    /**
     * MoveHistPrev() -> null
     *
     * Sakura's F_JUMPHIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function MoveHistPrev()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MoveHistPrev', func_get_args());
    }

    /**
     * MoveHistNext() -> null
     *
     * Sakura's F_JUMPHIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function MoveHistNext()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MoveHistNext', func_get_args());
    }

    /**
     * MoveHistSet() -> null
     *
     * Sakura's F_JUMPHIST_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function MoveHistSet()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MoveHistSet', func_get_args());
    }

    /**
     * F_WndScrollDown() -> null
     *
     * Sakura's F_WndScrollDown (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function F_WndScrollDown()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('F_WndScrollDown', func_get_args());
    }

    /**
     * F_WndScrollUp() -> null
     *
     * Sakura's F_WndScrollUp (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function F_WndScrollUp()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('F_WndScrollUp', func_get_args());
    }

    /**
     * GoNextParagraph() -> null
     *
     * Sakura's F_GONEXTPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoNextParagraph()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoNextParagraph', func_get_args());
    }

    /**
     * GoPrevParagraph() -> null
     *
     * Sakura's F_GOPREVPARAGRAPH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoPrevParagraph()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoPrevParagraph', func_get_args());
    }

    /**
     * GoModifyLineNext() -> null
     *
     * Sakura's F_MODIFYLINE_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoModifyLineNext()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoModifyLineNext', func_get_args());
    }

    /**
     * GoModifyLinePrev() -> null
     *
     * Sakura's F_MODIFYLINE_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoModifyLinePrev()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoModifyLinePrev', func_get_args());
    }

    /**
     * MoveCursorLayout(arg1: int?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_MOVECURSORLAYOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function MoveCursorLayout($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MoveCursorLayout', func_get_args());
    }

    /**
     * WheelUp(arg1: int?) -> null
     *
     * Sakura's F_WHEELUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WheelUp($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WheelUp', func_get_args());
    }

    /**
     * WheelDown(arg1: int?) -> null
     *
     * Sakura's F_WHEELDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WheelDown($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WheelDown', func_get_args());
    }

    /**
     * WheelLeft(arg1: int?) -> null
     *
     * Sakura's F_WHEELLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WheelLeft($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WheelLeft', func_get_args());
    }

    /**
     * WheelRight(arg1: int?) -> null
     *
     * Sakura's F_WHEELRIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WheelRight($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WheelRight', func_get_args());
    }

    /**
     * WheelPageUp(arg1: int?) -> null
     *
     * Sakura's F_WHEELPAGEUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WheelPageUp($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WheelPageUp', func_get_args());
    }

    /**
     * WheelPageDown(arg1: int?) -> null
     *
     * Sakura's F_WHEELPAGEDOWN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WheelPageDown($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WheelPageDown', func_get_args());
    }

    /**
     * WheelPageLeft(arg1: int?) -> null
     *
     * Sakura's F_WHEELPAGELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WheelPageLeft($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WheelPageLeft', func_get_args());
    }

    /**
     * WheelPageRight(arg1: int?) -> null
     *
     * Sakura's F_WHEELPAGERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WheelPageRight($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WheelPageRight', func_get_args());
    }

    /**
     * SelectWord() -> null
     *
     * Sakura's F_SELECTWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SelectWord()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SelectWord', func_get_args());
    }

    /**
     * SelectLine(arg1: int?) -> null
     *
     * Sakura's F_SELECTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SelectLine($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SelectLine', func_get_args());
    }

    /**
     * BeginSelect() -> null
     *
     * Sakura's F_BEGIN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function BeginSelect()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BeginSelect', func_get_args());
    }

    /**
     * Up_Sel() -> null
     *
     * Sakura's F_UP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Up_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Up_Sel', func_get_args());
    }

    /**
     * Down_Sel() -> null
     *
     * Sakura's F_DOWN_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Down_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Down_Sel', func_get_args());
    }

    /**
     * Left_Sel() -> null
     *
     * Sakura's F_LEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Left_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Left_Sel', func_get_args());
    }

    /**
     * Right_Sel() -> null
     *
     * Sakura's F_RIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Right_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Right_Sel', func_get_args());
    }

    /**
     * Up2_Sel() -> null
     *
     * Sakura's F_UP2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Up2_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Up2_Sel', func_get_args());
    }

    /**
     * Down2_Sel() -> null
     *
     * Sakura's F_DOWN2_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Down2_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Down2_Sel', func_get_args());
    }

    /**
     * WordLeft_Sel() -> null
     *
     * Sakura's F_WORDLEFT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WordLeft_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordLeft_Sel', func_get_args());
    }

    /**
     * WordRight_Sel() -> null
     *
     * Sakura's F_WORDRIGHT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WordRight_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordRight_Sel', func_get_args());
    }

    /**
     * GoLineTop_Sel(arg1: int?) -> null
     *
     * Sakura's F_GOLINETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoLineTop_Sel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoLineTop_Sel', func_get_args());
    }

    /**
     * GoLineEnd_Sel(arg1: int?) -> null
     *
     * Sakura's F_GOLINEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoLineEnd_Sel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoLineEnd_Sel', func_get_args());
    }

    /**
     * HalfPageUp_Sel(arg1: int?) -> null
     *
     * Sakura's F_HalfPageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HalfPageUp_Sel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HalfPageUp_Sel', func_get_args());
    }

    /**
     * HalfPageDown_Sel(arg1: int?) -> null
     *
     * Sakura's F_HalfPageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HalfPageDown_Sel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HalfPageDown_Sel', func_get_args());
    }

    /**
     * PageUp_Sel(arg1: int?) -> null
     *
     * Sakura's F_1PageUp_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function PageUp_Sel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PageUp_Sel', func_get_args());
    }

    /**
     * PageDown_Sel(arg1: int?) -> null
     *
     * Sakura's F_1PageDown_Sel (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function PageDown_Sel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PageDown_Sel', func_get_args());
    }

    /**
     * GoFileTop_Sel() -> null
     *
     * Sakura's F_GOFILETOP_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoFileTop_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoFileTop_Sel', func_get_args());
    }

    /**
     * GoFileEnd_Sel() -> null
     *
     * Sakura's F_GOFILEEND_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoFileEnd_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoFileEnd_Sel', func_get_args());
    }

    /**
     * GoNextParagraph_Sel() -> null
     *
     * Sakura's F_GONEXTPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoNextParagraph_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoNextParagraph_Sel', func_get_args());
    }

    /**
     * GoPrevParagraph_Sel() -> null
     *
     * Sakura's F_GOPREVPARAGRAPH_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoPrevParagraph_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoPrevParagraph_Sel', func_get_args());
    }

    /**
     * GoModifyLineNext_Sel() -> null
     *
     * Sakura's F_MODIFYLINE_NEXT_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoModifyLineNext_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoModifyLineNext_Sel', func_get_args());
    }

    /**
     * GoModifyLinePrev_Sel() -> null
     *
     * Sakura's F_MODIFYLINE_PREV_SEL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoModifyLinePrev_Sel()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoModifyLinePrev_Sel', func_get_args());
    }

    /**
     * BeginBoxSelect() -> null
     *
     * Sakura's F_BEGIN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function BeginBoxSelect()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BeginBoxSelect', func_get_args());
    }

    /**
     * Up_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_UP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Up_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Up_BoxSel', func_get_args());
    }

    /**
     * Down_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_DOWN_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Down_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Down_BoxSel', func_get_args());
    }

    /**
     * Left_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_LEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Left_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Left_BoxSel', func_get_args());
    }

    /**
     * Right_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_RIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Right_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Right_BoxSel', func_get_args());
    }

    /**
     * Up2_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_UP2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Up2_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Up2_BoxSel', func_get_args());
    }

    /**
     * Down2_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_DOWN2_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Down2_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Down2_BoxSel', func_get_args());
    }

    /**
     * WordLeft_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_WORDLEFT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WordLeft_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordLeft_BoxSel', func_get_args());
    }

    /**
     * WordRight_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_WORDRIGHT_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WordRight_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WordRight_BoxSel', func_get_args());
    }

    /**
     * GoLogicalLineTop_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_GOLOGICALLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
     * a key, the menu and a key macro run. The last arguments may be left out. A command that
     * this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoLogicalLineTop_BoxSel($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoLogicalLineTop_BoxSel', func_get_args());
    }

    /**
     * GoLineTop_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_GOLINETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoLineTop_BoxSel($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoLineTop_BoxSel', func_get_args());
    }

    /**
     * GoLineEnd_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_GOLINEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoLineEnd_BoxSel($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoLineEnd_BoxSel', func_get_args());
    }

    /**
     * HalfPageUp_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_HalfPageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HalfPageUp_BoxSel($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HalfPageUp_BoxSel', func_get_args());
    }

    /**
     * HalfPageDown_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_HalfPageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HalfPageDown_BoxSel($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HalfPageDown_BoxSel', func_get_args());
    }

    /**
     * PageUp_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_1PageUp_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function PageUp_BoxSel($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PageUp_BoxSel', func_get_args());
    }

    /**
     * PageDown_BoxSel(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_1PageDown_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function PageDown_BoxSel($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PageDown_BoxSel', func_get_args());
    }

    /**
     * GoFileTop_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_GOFILETOP_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoFileTop_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoFileTop_BoxSel', func_get_args());
    }

    /**
     * GoFileEnd_BoxSel(arg1: int?) -> null
     *
     * Sakura's F_GOFILEEND_BOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GoFileEnd_BoxSel($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GoFileEnd_BoxSel', func_get_args());
    }

    /**
     * Cut() -> null
     *
     * Sakura's F_CUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Cut()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Cut', func_get_args());
    }

    /**
     * Copy() -> null
     *
     * Sakura's F_COPY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Copy()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Copy', func_get_args());
    }

    /**
     * Paste(arg1: int?) -> null
     *
     * Sakura's F_PASTE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Paste($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Paste', func_get_args());
    }

    /**
     * CopyAddCRLF() -> null
     *
     * Sakura's F_COPY_ADDCRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyAddCRLF()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyAddCRLF', func_get_args());
    }

    /**
     * CopyCRLF() -> null
     *
     * Sakura's F_COPY_CRLF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyCRLF()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyCRLF', func_get_args());
    }

    /**
     * PasteBox(arg1: int?) -> null
     *
     * Sakura's F_PASTEBOX (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function PasteBox($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PasteBox', func_get_args());
    }

    /**
     * InsBoxText(arg1: string?) -> null
     *
     * Sakura's F_INSBOXTEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function InsBoxText($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InsBoxText', func_get_args());
    }

    /**
     * CopyLines() -> null
     *
     * Sakura's F_COPYLINES (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyLines()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyLines', func_get_args());
    }

    /**
     * CopyLinesAsPassage() -> null
     *
     * Sakura's F_COPYLINESASPASSAGE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyLinesAsPassage()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyLinesAsPassage', func_get_args());
    }

    /**
     * CopyLinesWithLineNumber() -> null
     *
     * Sakura's F_COPYLINESWITHLINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyLinesWithLineNumber()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyLinesWithLineNumber', func_get_args());
    }

    /**
     * CopyColorHtml() -> null
     *
     * Sakura's F_COPY_COLOR_HTML (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyColorHtml()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyColorHtml', func_get_args());
    }

    /**
     * CopyColorHtmlWithLineNumber() -> null
     *
     * Sakura's F_COPY_COLOR_HTML_LINENUMBER (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyColorHtmlWithLineNumber()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyColorHtmlWithLineNumber', func_get_args());
    }

    /**
     * CopyPath() -> null
     *
     * Sakura's F_COPYPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyPath()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyPath', func_get_args());
    }

    /**
     * CopyDirPath() -> null
     *
     * Sakura's F_COPYDIRPATH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyDirPath()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyDirPath', func_get_args());
    }

    /**
     * CopyFilename() -> null
     *
     * Sakura's F_COPYFNAME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyFilename()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyFilename', func_get_args());
    }

    /**
     * CopyTag() -> null
     *
     * Sakura's F_COPYTAG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyTag()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyTag', func_get_args());
    }

    /**
     * CopyKeyBindList() -> null
     *
     * Sakura's F_CREATEKEYBINDLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CopyKeyBindList()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CopyKeyBindList', func_get_args());
    }

    /**
     * InsertDate() -> null
     *
     * Sakura's F_INS_DATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function InsertDate()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InsertDate', func_get_args());
    }

    /**
     * InsertTime() -> null
     *
     * Sakura's F_INS_TIME (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function InsertTime()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InsertTime', func_get_args());
    }

    /**
     * CtrlCodeDialog() -> null
     *
     * Sakura's F_CTRL_CODE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CtrlCodeDialog()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CtrlCodeDialog', func_get_args());
    }

    /**
     * CtrlCode(arg1: int?) -> null
     *
     * Sakura's F_CTRL_CODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CtrlCode($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CtrlCode', func_get_args());
    }

    /**
     * InsertFileUsedRecently() -> null
     *
     * Sakura's F_INS_FILE_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function InsertFileUsedRecently()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InsertFileUsedRecently', func_get_args());
    }

    /**
     * InsertFolderUsedRecently() -> null
     *
     * Sakura's F_INS_FOLDER_USED_RECENTLY (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function InsertFolderUsedRecently()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('InsertFolderUsedRecently', func_get_args());
    }

    /**
     * ToLower() -> null
     *
     * Sakura's F_TOLOWER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ToLower()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToLower', func_get_args());
    }

    /**
     * ToUpper() -> null
     *
     * Sakura's F_TOUPPER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ToUpper()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToUpper', func_get_args());
    }

    /**
     * ToHankaku() -> null
     *
     * Sakura's F_TOHANKAKU (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ToHankaku()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToHankaku', func_get_args());
    }

    /**
     * ToHankata() -> null
     *
     * Sakura's F_TOHANKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ToHankata()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToHankata', func_get_args());
    }

    /**
     * ToZenEi() -> null
     *
     * Sakura's F_TOZENEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ToZenEi()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToZenEi', func_get_args());
    }

    /**
     * ToHanEi() -> null
     *
     * Sakura's F_TOHANEI (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ToHanEi()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToHanEi', func_get_args());
    }

    /**
     * ToZenKata() -> null
     *
     * Sakura's F_TOZENKAKUKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ToZenKata()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToZenKata', func_get_args());
    }

    /**
     * ToZenHira() -> null
     *
     * Sakura's F_TOZENKAKUHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ToZenHira()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToZenHira', func_get_args());
    }

    /**
     * HanKataToZenKata() -> null
     *
     * Sakura's F_HANKATATOZENKATA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HanKataToZenKata()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HanKataToZenKata', func_get_args());
    }

    /**
     * HanKataToZenHira() -> null
     *
     * Sakura's F_HANKATATOZENHIRA (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HanKataToZenHira()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HanKataToZenHira', func_get_args());
    }

    /**
     * TABToSPACE() -> null
     *
     * Sakura's F_TABTOSPACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function TABToSPACE()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TABToSPACE', func_get_args());
    }

    /**
     * SPACEToTAB() -> null
     *
     * Sakura's F_SPACETOTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SPACEToTAB()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SPACEToTAB', func_get_args());
    }

    /**
     * AutoToSJIS() -> null
     *
     * Sakura's F_CODECNV_AUTO2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function AutoToSJIS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('AutoToSJIS', func_get_args());
    }

    /**
     * JIStoSJIS() -> null
     *
     * Sakura's F_CODECNV_EMAIL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function JIStoSJIS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('JIStoSJIS', func_get_args());
    }

    /**
     * EUCtoSJIS() -> null
     *
     * Sakura's F_CODECNV_EUC2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function EUCtoSJIS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('EUCtoSJIS', func_get_args());
    }

    /**
     * CodeCnvUNICODEtoSJIS() -> null
     *
     * Sakura's F_CODECNV_UNICODE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that
     * a key, the menu and a key macro run. The last arguments may be left out. A command that
     * this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CodeCnvUNICODEtoSJIS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CodeCnvUNICODEtoSJIS', func_get_args());
    }

    /**
     * CodeCnvUNICODEBEtoSJIS() -> null
     *
     * Sakura's F_CODECNV_UNICODEBE2SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CodeCnvUNICODEBEtoSJIS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CodeCnvUNICODEBEtoSJIS', func_get_args());
    }

    /**
     * UTF8toSJIS() -> null
     *
     * Sakura's F_CODECNV_UTF82SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function UTF8toSJIS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('UTF8toSJIS', func_get_args());
    }

    /**
     * UTF7toSJIS() -> null
     *
     * Sakura's F_CODECNV_UTF72SJIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function UTF7toSJIS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('UTF7toSJIS', func_get_args());
    }

    /**
     * SJIStoJIS() -> null
     *
     * Sakura's F_CODECNV_SJIS2JIS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SJIStoJIS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SJIStoJIS', func_get_args());
    }

    /**
     * SJIStoEUC() -> null
     *
     * Sakura's F_CODECNV_SJIS2EUC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SJIStoEUC()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SJIStoEUC', func_get_args());
    }

    /**
     * SJIStoUTF8() -> null
     *
     * Sakura's F_CODECNV_SJIS2UTF8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SJIStoUTF8()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SJIStoUTF8', func_get_args());
    }

    /**
     * SJIStoUTF7() -> null
     *
     * Sakura's F_CODECNV_SJIS2UTF7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SJIStoUTF7()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SJIStoUTF7', func_get_args());
    }

    /**
     * Base64Decode() -> null
     *
     * Sakura's F_BASE64DECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function Base64Decode()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Base64Decode', func_get_args());
    }

    /**
     * Uudecode() -> null
     *
     * Sakura's F_UUDECODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Uudecode()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Uudecode', func_get_args());
    }

    /**
     * SearchDialog() -> null
     *
     * Sakura's F_SEARCH_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SearchDialog()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SearchDialog', func_get_args());
    }

    /**
     * SearchNext(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_SEARCH_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SearchNext($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SearchNext', func_get_args());
    }

    /**
     * SearchPrev(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_SEARCH_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SearchPrev($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SearchPrev', func_get_args());
    }

    /**
     * ReplaceDialog() -> null
     *
     * Sakura's F_REPLACE_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ReplaceDialog()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ReplaceDialog', func_get_args());
    }

    /**
     * Replace(arg1: string?, arg2: string?, arg3: int?) -> null
     *
     * Sakura's F_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Replace($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Replace', func_get_args());
    }

    /**
     * ReplaceAll(arg1: string?, arg2: string?, arg3: int?) -> null
     *
     * Sakura's F_REPLACE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ReplaceAll($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ReplaceAll', func_get_args());
    }

    /**
     * SearchClearMark() -> null
     *
     * Sakura's F_SEARCH_CLEARMARK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SearchClearMark()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SearchClearMark', func_get_args());
    }

    /**
     * SearchStartPos() -> null
     *
     * Sakura's F_JUMP_SRCHSTARTPOS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SearchStartPos()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SearchStartPos', func_get_args());
    }

    /**
     * Grep(arg1: string?, arg2: string?, arg3: string?, arg4: int?, arg5: int?) -> null
     *
     * Sakura's F_GREP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Grep($arg1 = null, $arg2 = null, $arg3 = null, $arg4 = null, $arg5 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Grep', func_get_args());
    }

    /**
     * GrepReplace(arg1: string?, arg2: string?, arg3: string?, arg4: string?, arg5: int?, arg6: int?) -> null
     *
     * Sakura's F_GREP_REPLACE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function GrepReplace($arg1 = null, $arg2 = null, $arg3 = null, $arg4 = null, $arg5 = null, $arg6 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GrepReplace', func_get_args());
    }

    /**
     * Outline(arg1: int?) -> null
     *
     * Sakura's F_OUTLINE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Outline($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Outline', func_get_args());
    }

    /**
     * TagJump() -> null
     *
     * Sakura's F_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function TagJump()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TagJump', func_get_args());
    }

    /**
     * TagJumpBack() -> null
     *
     * Sakura's F_TAGJUMPBACK (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TagJumpBack()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TagJumpBack', func_get_args());
    }

    /**
     * TagMake() -> null
     *
     * Sakura's F_TAGS_MAKE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function TagMake()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TagMake', func_get_args());
    }

    /**
     * DirectTagJump() -> null
     *
     * Sakura's F_DIRECT_TAGJUMP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function DirectTagJump()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DirectTagJump', func_get_args());
    }

    /**
     * KeywordTagJump(arg1: string?) -> null
     *
     * Sakura's F_TAGJUMP_KEYWORD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function KeywordTagJump($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('KeywordTagJump', func_get_args());
    }

    /**
     * Compare() -> null
     *
     * Sakura's F_COMPARE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function Compare()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Compare', func_get_args());
    }

    /**
     * DiffDialog() -> null
     *
     * Sakura's F_DIFF_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function DiffDialog()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DiffDialog', func_get_args());
    }

    /**
     * Diff(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_DIFF (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Diff($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Diff', func_get_args());
    }

    /**
     * DiffNext() -> null
     *
     * Sakura's F_DIFF_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function DiffNext()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DiffNext', func_get_args());
    }

    /**
     * DiffPrev() -> null
     *
     * Sakura's F_DIFF_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function DiffPrev()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DiffPrev', func_get_args());
    }

    /**
     * DiffReset() -> null
     *
     * Sakura's F_DIFF_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function DiffReset()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('DiffReset', func_get_args());
    }

    /**
     * BracketPair() -> null
     *
     * Sakura's F_BRACKETPAIR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function BracketPair()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BracketPair', func_get_args());
    }

    /**
     * BookmarkSet() -> null
     *
     * Sakura's F_BOOKMARK_SET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function BookmarkSet()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BookmarkSet', func_get_args());
    }

    /**
     * BookmarkNext() -> null
     *
     * Sakura's F_BOOKMARK_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function BookmarkNext()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BookmarkNext', func_get_args());
    }

    /**
     * BookmarkPrev() -> null
     *
     * Sakura's F_BOOKMARK_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function BookmarkPrev()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BookmarkPrev', func_get_args());
    }

    /**
     * BookmarkReset() -> null
     *
     * Sakura's F_BOOKMARK_RESET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function BookmarkReset()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BookmarkReset', func_get_args());
    }

    /**
     * BookmarkView() -> null
     *
     * Sakura's F_BOOKMARK_VIEW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function BookmarkView()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BookmarkView', func_get_args());
    }

    /**
     * BookmarkPattern(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_BOOKMARK_PATTERN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function BookmarkPattern($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('BookmarkPattern', func_get_args());
    }

    /**
     * FuncListNext() -> null
     *
     * Sakura's F_FUNCLIST_NEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FuncListNext()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FuncListNext', func_get_args());
    }

    /**
     * FuncListPrev() -> null
     *
     * Sakura's F_FUNCLIST_PREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function FuncListPrev()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('FuncListPrev', func_get_args());
    }

    /**
     * ChgmodINS() -> null
     *
     * Sakura's F_CHGMOD_INS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ChgmodINS()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ChgmodINS', func_get_args());
    }

    /**
     * ChgCharSet(arg1: int?, arg2: int?) -> null
     *
     * Sakura's F_CHG_CHARSET (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ChgCharSet($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ChgCharSet', func_get_args());
    }

    /**
     * ChgmodEOL(arg1: int?) -> null
     *
     * Sakura's F_CHGMOD_EOL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ChgmodEOL($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ChgmodEOL', func_get_args());
    }

    /**
     * CancelMode() -> null
     *
     * Sakura's F_CANCEL_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CancelMode()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CancelMode', func_get_args());
    }

    /**
     * ExecExternalMacro(arg1: string?, arg2: string?) -> null
     *
     * Sakura's F_EXECEXTMACRO (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ExecExternalMacro($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExecExternalMacro', func_get_args());
    }

    /**
     * ShowToolbar() -> null
     *
     * Sakura's F_SHOWTOOLBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ShowToolbar()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ShowToolbar', func_get_args());
    }

    /**
     * ShowFunckey() -> null
     *
     * Sakura's F_SHOWFUNCKEY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ShowFunckey()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ShowFunckey', func_get_args());
    }

    /**
     * ShowTab() -> null
     *
     * Sakura's F_SHOWTAB (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ShowTab()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ShowTab', func_get_args());
    }

    /**
     * ShowStatusbar() -> null
     *
     * Sakura's F_SHOWSTATUSBAR (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ShowStatusbar()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ShowStatusbar', func_get_args());
    }

    /**
     * ShowMiniMap() -> null
     *
     * Sakura's F_SHOWMINIMAP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ShowMiniMap()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ShowMiniMap', func_get_args());
    }

    /**
     * TypeList() -> null
     *
     * Sakura's F_TYPE_LIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function TypeList()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TypeList', func_get_args());
    }

    /**
     * ChangeType(arg1: int?) -> null
     *
     * Sakura's F_CHANGETYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ChangeType($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ChangeType', func_get_args());
    }

    /**
     * OptionType() -> null
     *
     * Sakura's F_OPTION_TYPE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function OptionType()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('OptionType', func_get_args());
    }

    /**
     * OptionCommon() -> null
     *
     * Sakura's F_OPTION (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function OptionCommon()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('OptionCommon', func_get_args());
    }

    /**
     * SelectFont() -> null
     *
     * Sakura's F_FONT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function SelectFont()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SelectFont', func_get_args());
    }

    /**
     * SetFontSize(arg1: int?, arg2: int?, arg3: int?) -> null
     *
     * Sakura's F_SETFONTSIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SetFontSize($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetFontSize', func_get_args());
    }

    /**
     * WrapWindowWidth() -> null
     *
     * Sakura's F_WRAPWINDOWWIDTH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WrapWindowWidth()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WrapWindowWidth', func_get_args());
    }

    /**
     * OptionFavorite() -> null
     *
     * Sakura's F_FAVORITE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function OptionFavorite()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('OptionFavorite', func_get_args());
    }

    /**
     * SetMsgQuoteStr(arg1: string?) -> null
     *
     * Sakura's F_SET_QUOTESTRING (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SetMsgQuoteStr($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetMsgQuoteStr', func_get_args());
    }

    /**
     * TextWrapMethod(arg1: int?) -> null
     *
     * Sakura's F_TEXTWRAPMETHOD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TextWrapMethod($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TextWrapMethod', func_get_args());
    }

    /**
     * SelectCountMode(arg1: int?) -> null
     *
     * Sakura's F_SELECT_COUNT_MODE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SelectCountMode($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SelectCountMode', func_get_args());
    }

    /**
     * ExecCommand(arg1: string?, arg2: int?, arg3: string?) -> null
     *
     * Sakura's F_EXECMD (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ExecCommand($arg1 = null, $arg2 = null, $arg3 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExecCommand', func_get_args());
    }

    /**
     * ExecCommandDialog() -> null
     *
     * Sakura's F_EXECMD_DIALOG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ExecCommandDialog()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExecCommandDialog', func_get_args());
    }

    /**
     * RMenu() -> null
     *
     * Sakura's F_MENU_RBUTTON (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function RMenu()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('RMenu', func_get_args());
    }

    /**
     * CustMenu1() -> null
     *
     * Sakura's F_CUSTMENU_1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu1()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu1', func_get_args());
    }

    /**
     * CustMenu2() -> null
     *
     * Sakura's F_CUSTMENU_2 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu2()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu2', func_get_args());
    }

    /**
     * CustMenu3() -> null
     *
     * Sakura's F_CUSTMENU_3 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu3()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu3', func_get_args());
    }

    /**
     * CustMenu4() -> null
     *
     * Sakura's F_CUSTMENU_4 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu4()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu4', func_get_args());
    }

    /**
     * CustMenu5() -> null
     *
     * Sakura's F_CUSTMENU_5 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu5()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu5', func_get_args());
    }

    /**
     * CustMenu6() -> null
     *
     * Sakura's F_CUSTMENU_6 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu6()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu6', func_get_args());
    }

    /**
     * CustMenu7() -> null
     *
     * Sakura's F_CUSTMENU_7 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu7()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu7', func_get_args());
    }

    /**
     * CustMenu8() -> null
     *
     * Sakura's F_CUSTMENU_8 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu8()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu8', func_get_args());
    }

    /**
     * CustMenu9() -> null
     *
     * Sakura's F_CUSTMENU_9 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu9()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu9', func_get_args());
    }

    /**
     * CustMenu10() -> null
     *
     * Sakura's F_CUSTMENU_10 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu10()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu10', func_get_args());
    }

    /**
     * CustMenu11() -> null
     *
     * Sakura's F_CUSTMENU_11 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu11()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu11', func_get_args());
    }

    /**
     * CustMenu12() -> null
     *
     * Sakura's F_CUSTMENU_12 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu12()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu12', func_get_args());
    }

    /**
     * CustMenu13() -> null
     *
     * Sakura's F_CUSTMENU_13 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu13()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu13', func_get_args());
    }

    /**
     * CustMenu14() -> null
     *
     * Sakura's F_CUSTMENU_14 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu14()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu14', func_get_args());
    }

    /**
     * CustMenu15() -> null
     *
     * Sakura's F_CUSTMENU_15 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu15()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu15', func_get_args());
    }

    /**
     * CustMenu16() -> null
     *
     * Sakura's F_CUSTMENU_16 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu16()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu16', func_get_args());
    }

    /**
     * CustMenu17() -> null
     *
     * Sakura's F_CUSTMENU_17 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu17()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu17', func_get_args());
    }

    /**
     * CustMenu18() -> null
     *
     * Sakura's F_CUSTMENU_18 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu18()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu18', func_get_args());
    }

    /**
     * CustMenu19() -> null
     *
     * Sakura's F_CUSTMENU_19 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu19()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu19', func_get_args());
    }

    /**
     * CustMenu20() -> null
     *
     * Sakura's F_CUSTMENU_20 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu20()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu20', func_get_args());
    }

    /**
     * CustMenu21() -> null
     *
     * Sakura's F_CUSTMENU_21 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu21()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu21', func_get_args());
    }

    /**
     * CustMenu22() -> null
     *
     * Sakura's F_CUSTMENU_22 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu22()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu22', func_get_args());
    }

    /**
     * CustMenu23() -> null
     *
     * Sakura's F_CUSTMENU_23 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu23()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu23', func_get_args());
    }

    /**
     * CustMenu24() -> null
     *
     * Sakura's F_CUSTMENU_24 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CustMenu24()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CustMenu24', func_get_args());
    }

    /**
     * SplitWinV() -> null
     *
     * Sakura's F_SPLIT_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SplitWinV()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SplitWinV', func_get_args());
    }

    /**
     * SplitWinH() -> null
     *
     * Sakura's F_SPLIT_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SplitWinH()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SplitWinH', func_get_args());
    }

    /**
     * SplitWinVH() -> null
     *
     * Sakura's F_SPLIT_VH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SplitWinVH()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SplitWinVH', func_get_args());
    }

    /**
     * WinMaximize() -> null
     *
     * Sakura's F_WINMAXIMIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WinMaximize()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WinMaximize', func_get_args());
    }

    /**
     * WinMinimize() -> null
     *
     * Sakura's F_WINMINIMIZE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WinMinimize()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WinMinimize', func_get_args());
    }

    /**
     * WinRestore() -> null
     *
     * Sakura's F_WINRESTORE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WinRestore()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WinRestore', func_get_args());
    }

    /**
     * WinClose() -> null
     *
     * Sakura's F_WINCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WinClose()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WinClose', func_get_args());
    }

    /**
     * WinCloseAll() -> null
     *
     * Sakura's F_WIN_CLOSEALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function WinCloseAll()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WinCloseAll', func_get_args());
    }

    /**
     * CascadeWin() -> null
     *
     * Sakura's F_CASCADE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function CascadeWin()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CascadeWin', func_get_args());
    }

    /**
     * TileWinV() -> null
     *
     * Sakura's F_TILE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function TileWinV()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TileWinV', func_get_args());
    }

    /**
     * TileWinH() -> null
     *
     * Sakura's F_TILE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function TileWinH()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TileWinH', func_get_args());
    }

    /**
     * NextWindow() -> null
     *
     * Sakura's F_NEXTWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function NextWindow()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('NextWindow', func_get_args());
    }

    /**
     * PrevWindow() -> null
     *
     * Sakura's F_PREVWINDOW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function PrevWindow()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PrevWindow', func_get_args());
    }

    /**
     * WindowList() -> null
     *
     * Sakura's F_WINLIST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WindowList()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WindowList', func_get_args());
    }

    /**
     * MaximizeV() -> null
     *
     * Sakura's F_MAXIMIZE_V (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function MaximizeV()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MaximizeV', func_get_args());
    }

    /**
     * MaximizeH() -> null
     *
     * Sakura's F_MAXIMIZE_H (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function MaximizeH()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MaximizeH', func_get_args());
    }

    /**
     * MinimizeAll() -> null
     *
     * Sakura's F_MINIMIZE_ALL (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function MinimizeAll()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MinimizeAll', func_get_args());
    }

    /**
     * ReDraw() -> null
     *
     * Sakura's F_REDRAW (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ReDraw()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ReDraw', func_get_args());
    }

    /**
     * ActivateWinOutput() -> null
     *
     * Sakura's F_WIN_OUTPUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ActivateWinOutput()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ActivateWinOutput', func_get_args());
    }

    /**
     * TraceOut(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_TRACEOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function TraceOut($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TraceOut', func_get_args());
    }

    /**
     * WindowTopMost(arg1: int?) -> null
     *
     * Sakura's F_TOPMOST (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function WindowTopMost($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('WindowTopMost', func_get_args());
    }

    /**
     * GroupClose() -> null
     *
     * Sakura's F_GROUPCLOSE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function GroupClose()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('GroupClose', func_get_args());
    }

    /**
     * NextGroup() -> null
     *
     * Sakura's F_NEXTGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function NextGroup()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('NextGroup', func_get_args());
    }

    /**
     * PrevGroup() -> null
     *
     * Sakura's F_PREVGROUP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function PrevGroup()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('PrevGroup', func_get_args());
    }

    /**
     * TabMoveRight() -> null
     *
     * Sakura's F_TAB_MOVERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TabMoveRight()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TabMoveRight', func_get_args());
    }

    /**
     * TabMoveLeft() -> null
     *
     * Sakura's F_TAB_MOVELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TabMoveLeft()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TabMoveLeft', func_get_args());
    }

    /**
     * TabSeparate() -> null
     *
     * Sakura's F_TAB_SEPARATE (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TabSeparate()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TabSeparate', func_get_args());
    }

    /**
     * TabJointNext() -> null
     *
     * Sakura's F_TAB_JOINTNEXT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TabJointNext()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TabJointNext', func_get_args());
    }

    /**
     * TabJointPrev() -> null
     *
     * Sakura's F_TAB_JOINTPREV (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TabJointPrev()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TabJointPrev', func_get_args());
    }

    /**
     * TabCloseOther() -> null
     *
     * Sakura's F_TAB_CLOSEOTHER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TabCloseOther()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TabCloseOther', func_get_args());
    }

    /**
     * TabCloseLeft() -> null
     *
     * Sakura's F_TAB_CLOSELEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TabCloseLeft()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TabCloseLeft', func_get_args());
    }

    /**
     * TabCloseRight() -> null
     *
     * Sakura's F_TAB_CLOSERIGHT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function TabCloseRight()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('TabCloseRight', func_get_args());
    }

    /**
     * Complete() -> null
     *
     * Sakura's F_HOKAN (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function Complete()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('Complete', func_get_args());
    }

    /**
     * ToggleKeyHelpSearch(arg1: int?) -> null
     *
     * Sakura's F_TOGGLE_KEY_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ToggleKeyHelpSearch($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ToggleKeyHelpSearch', func_get_args());
    }

    /**
     * HelpContents() -> null
     *
     * Sakura's F_HELP_CONTENTS (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HelpContents()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HelpContents', func_get_args());
    }

    /**
     * HelpSearch() -> null
     *
     * Sakura's F_HELP_SEARCH (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function HelpSearch()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('HelpSearch', func_get_args());
    }

    /**
     * CommandList() -> null
     *
     * Sakura's F_MENU_ALLFUNC (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CommandList()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CommandList', func_get_args());
    }

    /**
     * ExtHelp1() -> null
     *
     * Sakura's F_EXTHELP1 (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function ExtHelp1()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExtHelp1', func_get_args());
    }

    /**
     * ExtHtmlHelp(arg1: string?, arg2: string?) -> null
     *
     * Sakura's F_EXTHTMLHELP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ExtHtmlHelp($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ExtHtmlHelp', func_get_args());
    }

    /**
     * About() -> null
     *
     * Sakura's F_ABOUT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the menu
     * and a key macro run. The last arguments may be left out. A command that this editor does
     * not have yet answers the error -32001 (not implemented).
     */
    public static function About()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('About', func_get_args());
    }

    /**
     * StatusMsg(arg1: string?, arg2: int?) -> null
     *
     * Sakura's F_STATUSMSG (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function StatusMsg($arg1 = null, $arg2 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('StatusMsg', func_get_args());
    }

    /**
     * MsgBeep(arg1: int?) -> null
     *
     * Sakura's F_MSGBEEP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function MsgBeep($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('MsgBeep', func_get_args());
    }

    /**
     * CommitUndoBuffer() -> null
     *
     * Sakura's F_COMMITUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function CommitUndoBuffer()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('CommitUndoBuffer', func_get_args());
    }

    /**
     * AddRefUndoBuffer() -> null
     *
     * Sakura's F_ADDREFUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a
     * key, the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function AddRefUndoBuffer()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('AddRefUndoBuffer', func_get_args());
    }

    /**
     * SetUndoBuffer() -> null
     *
     * Sakura's F_SETUNDOBUFFER (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SetUndoBuffer()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetUndoBuffer', func_get_args());
    }

    /**
     * AppendUndoBufferCursor() -> null
     *
     * Sakura's F_APPENDUNDOBUFFERCURSOR (sakura_core/macro/CSMacroMgr.cpp). Runs the command
     * that a key, the menu and a key macro run. The last arguments may be left out. A command
     * that this editor does not have yet answers the error -32001 (not implemented).
     */
    public static function AppendUndoBufferCursor()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('AppendUndoBufferCursor', func_get_args());
    }

    /**
     * ClipboardEmpty() -> null
     *
     * Sakura's F_CLIPBOARDEMPTY (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function ClipboardEmpty()
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('ClipboardEmpty', func_get_args());
    }

    /**
     * SetViewTop(arg1: int?) -> null
     *
     * Sakura's F_SETVIEWTOP (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key, the
     * menu and a key macro run. The last arguments may be left out. A command that this editor
     * does not have yet answers the error -32001 (not implemented).
     */
    public static function SetViewTop($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetViewTop', func_get_args());
    }

    /**
     * SetViewLeft(arg1: int?) -> null
     *
     * Sakura's F_SETVIEWLEFT (sakura_core/macro/CSMacroMgr.cpp). Runs the command that a key,
     * the menu and a key macro run. The last arguments may be left out. A command that this
     * editor does not have yet answers the error -32001 (not implemented).
     */
    public static function SetViewLeft($arg1 = null)
    {
        // func_get_args() holds only the arguments that were given.
        return (self::$call)('SetViewLeft', func_get_args());
    }

    /**
     * Any other name that starts like a Sakura macro name (an upper case letter or a digit) is
     * sent to the editor as it is. The editor answers a name it does not know with an error
     * (EditorError).
     */
    public static function __callStatic($name, $arguments)
    {
        if (!preg_match('/^[0-9A-Z]/', $name)) {
            throw new BadMethodCallException("Call to undefined method Editor::$name()");
        }
        return (self::$call)($name, $arguments);
    }
}
