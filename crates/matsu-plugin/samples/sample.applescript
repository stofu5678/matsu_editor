-- Sample macro (AppleScript: .applescript, macOS only).
-- The editor's functions are called with editorCall("Name", {arguments}): the names and the
-- arguments are Sakura's macro API (a handler per name is not possible: copy, delete, print and
-- others are words of AppleScript). editorPrint(text) writes a line to the output panel;
-- editorInfo("editor.name") reads what the editor told the macro in `initialize`.
-- The script is the body of a script object, so it cannot have `use` lines. Inside a `tell` block
-- call the handlers as `my editorCall(...)`.

editorCall("GoFileTop", {})
editorCall("InsText", {"# サンプルマクロ 😀" & linefeed})

set lineCount to editorCall("GetLineCount", {0})
set fileName to editorCall("GetFilename", {})
if fileName is "" then set fileName to "(無題)"
editorPrint("行数: " & lineCount & " ファイル: " & fileName)
repeat with i from 1 to lineCount
	set lineText to editorCall("GetLineStr", {i})
	if lineText is linefeed then
		set lineText to ""
	else if lineText ends with linefeed then
		set lineText to text 1 thru -2 of lineText
	end if
	editorPrint((i as text) & ": " & lineText)
end repeat

-- Select line 3 (a line number and a column both start at 1) and replace it.
editorCall("MoveCursor", {3, 1, 0})
editorCall("MoveCursor", {3, 1000, 1}) -- past the end of the line: the end of the line
editorPrint("選択: " & editorCall("GetSelectedString", {0}) & " / IsTextSelected = " & editorCall("IsTextSelected", {}))
editorCall("InsText", {"みかん😀 (置き換えました)"})

-- An error response is an AppleScript error: its number is the editor's code.
try
	editorCall("GetLineStr", {"not a number"})
on error errorText number errorNumber
	editorPrint("エラー: " & errorNumber)
end try

editorCall("AddTail", {"-- マクロの終わり --" & linefeed})
editorCall("Jump", {1})
