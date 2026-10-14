# GENERATED from src/macro_host/api.rs (FUNCS). Do not edit. Regenerate with: UME_UPDATE_GENERATED=1 cargo test generated_files

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
            selection, which the PoC does not have.)"""
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
            (1, the default) is the line break unit, which is the only unit that the PoC
            has; bit 1 (PL/SQL) is not supported."""
            return call("Jump", _pack(line, flag))

        def GetFilename(self) -> str:
            """The full path of the file, or an empty string when the text has no file."""
            return call("GetFilename", _pack())

    return _Editor()
