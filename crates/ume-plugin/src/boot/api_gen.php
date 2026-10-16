<?php
// GENERATED from crates/ume-plugin/src/api.rs (FUNCS). Do not edit. Regenerate with: UME_UPDATE_GENERATED=1 cargo test -p ume-plugin generated_files

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
     * 0 when nothing is selected, 1 when text is selected. (Sakura returns 2 for a box
     * selection, which is not supported yet.)
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
