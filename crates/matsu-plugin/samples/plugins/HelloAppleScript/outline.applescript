-- The jack Outline: a tree of the lines that start with "#".
editorCall("Outline.SetListType", {100})
editorCall("Outline.SetTitle", {"Headings (AppleScript sample)"})
set totalLines to editorCall("GetLineCount", {0})
repeat with n from 1 to totalLines
	set lineText to my trimLineEnd(editorCall("GetLineStr", {n}))
	if lineText starts with "#" then
		set headingLevel to 0
		repeat while lineText starts with "#"
			set headingLevel to headingLevel + 1
			set lineText to my dropFirst(lineText)
		end repeat
		repeat while lineText starts with " " or lineText starts with tab
			set lineText to my dropFirst(lineText)
		end repeat
		editorCall("Outline.AddFuncInfo2", {n, 1, lineText, headingLevel - 1})
	end if
end repeat

-- The text without the line breaks at its end.
on trimLineEnd(someText)
	repeat while someText ends with linefeed or someText ends with (character id 13)
		if (length of someText) is 1 then return ""
		set someText to text 1 thru -2 of someText
	end repeat
	return someText
end trimLineEnd

-- The text without its first character.
on dropFirst(someText)
	if (length of someText) <= 1 then return ""
	return text 2 thru -1 of someText
end dropFirst
