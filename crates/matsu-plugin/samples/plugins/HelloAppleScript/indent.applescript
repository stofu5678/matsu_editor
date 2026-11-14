-- The jack SmartIndent: after Enter, four more spaces when the line above ends with a colon.
set typedChar to indentGetChar()
if typedChar is (character id 13) or typedChar is linefeed then
	set lineNumber to (editorCall("ExpandParameter", {"$y"})) as integer
	if lineNumber >= 2 then
		set aboveText to my trimLineEnd(editorCall("GetLineStr", {lineNumber - 1}))
		if aboveText ends with ":" then editorCall("InsText", {"    "})
	end if
end if

-- The text without the line breaks at its end.
on trimLineEnd(someText)
	repeat while someText ends with linefeed or someText ends with (character id 13)
		if (length of someText) is 1 then return ""
		set someText to text 1 thru -2 of someText
	end repeat
	return someText
end trimLineEnd
