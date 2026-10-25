-- The boot library of the AppleScript macros (macOS only): osascript macro.applescript
--
-- The editor builds one script from three parts: this prefix, the user's script inside
-- `script UserMacro ... end script`, and the suffix (the `on run` that follows this text in
-- applescript.rs). The user's script sees the handlers below because a script object sees the
-- handlers of the script around it.
--
-- AppleScript cannot read the standard input or write the standard output by itself, so this
-- uses AppleScriptObjC (Foundation) for both: NSFileHandle for the lines and
-- NSJSONSerialization for the JSON. The lines follow docs/poc/json-rpc.md (sections 2 and 3):
-- one request per line, starting with RS, and the answer is the line with the same id. The
-- JSON that is written is UTF-8, not ASCII-escaped (the standard output of macOS is UTF-8).
use AppleScript version "2.4"
use framework "Foundation"
use scripting additions

property editorBuffer : missing value
property editorNextId : 1
property editorInfoObject : missing value
property editorUTF8 : 4
property editorStarted : false

-- Writes the text to the standard output at once (no buffer to flush).
on editorWrite(someText)
	set theData to (current application's NSString's stringWithString:someText)'s dataUsingEncoding:(editorUTF8)
	(current application's NSFileHandle's fileHandleWithStandardOutput())'s writeData:theData
end editorWrite

-- For a macro: prints the text and a line break to the output panel. (`log` goes to the error
-- output; the value of the last expression of the macro is not printed.)
on editorPrint(someText)
	my editorWrite(someText & linefeed)
end editorPrint

-- One line from the standard input without its line break, or `missing value` at the end.
-- The bytes are kept as bytes until a whole line is there, and only then decoded: a read of the
-- pipe can end in the middle of a character of more than one byte.
on editorReadLine()
	set newlineData to (current application's NSString's stringWithString:linefeed)'s dataUsingEncoding:(editorUTF8)
	repeat
		set bufferLength to editorBuffer's |length|()
		set theRange to editorBuffer's rangeOfData:newlineData options:0 range:{location:0, |length|:bufferLength}
		-- NSNotFound is 2^63 - 1, which AppleScript holds as a real that does not equal the
		-- constant: any position that is not huge is a position.
		if (location of theRange) < 1.0E+15 then
			set lineEnd to location of theRange
			set lineData to editorBuffer's subdataWithRange:{location:0, |length|:lineEnd}
			set my editorBuffer to (editorBuffer's subdataWithRange:{location:lineEnd + 1, |length|:(bufferLength - lineEnd - 1)})'s mutableCopy()
			return my editorDecode(lineData)
		end if
		set chunk to (current application's NSFileHandle's fileHandleWithStandardInput())'s availableData()
		if (chunk's |length|()) is 0 then
			if bufferLength is 0 then return missing value
			set lineData to editorBuffer
			set my editorBuffer to current application's NSMutableData's |data|()
			return my editorDecode(lineData)
		end if
		editorBuffer's appendData:chunk
	end repeat
end editorReadLine

-- The bytes of a line as text.
on editorDecode(lineData)
	set lineString to current application's NSString's alloc()'s initWithData:lineData encoding:(editorUTF8)
	if lineString is missing value then error "the editor sent text that is not UTF-8" number -32700
	return lineString as text
end editorDecode

-- A JSON text as an NSDictionary, or `missing value` if it is not one.
on editorParse(jsonText)
	set theData to (current application's NSString's stringWithString:jsonText)'s dataUsingEncoding:(editorUTF8)
	set {theObject, theError} to current application's NSJSONSerialization's JSONObjectWithData:theData options:0 |error|:(reference)
	if theObject is missing value then return missing value
	if not ((theObject's isKindOfClass:(current application's NSDictionary)) as boolean) then return missing value
	return theObject
end editorParse

-- Calls the macro function `methodName` of the editor (the Sakura name, such as "InsText") with
-- the list `argList`. Returns the result. An error of the editor is an AppleScript error whose
-- number is the editor's code and whose text is "EditorError CODE: MESSAGE".
on editorCall(methodName, argList)
	set requestId to editorNextId
	set my editorNextId to editorNextId + 1
	set theRequest to current application's NSDictionary's dictionaryWithDictionary:{jsonrpc:"2.0", |id|:requestId, |method|:methodName, params:argList}
	set {theData, theError} to current application's NSJSONSerialization's dataWithJSONObject:theRequest options:0 |error|:(reference)
	if theData is missing value then error "EditorError -32602: cannot send these arguments to the editor" number -32602
	set requestText to (current application's NSString's alloc()'s initWithData:theData encoding:(editorUTF8)) as text
	my editorWrite((character id 30) & requestText & linefeed)
	repeat
		set replyText to my editorReadLine()
		if replyText is missing value then error "EditorError -32000: the editor closed the connection before it answered " & methodName number -32000
		if replyText is not "" then
			set replyObject to my editorParse(replyText)
			if replyObject is not missing value then
				set replyId to replyObject's objectForKey:"id"
				set isOurs to false
				if replyId is not missing value then
					try
						set isOurs to ((replyId as integer) = requestId)
					end try
				end if
				if isOurs then
					set errorObject to replyObject's objectForKey:"error"
					if errorObject is not missing value then
						if (errorObject's isKindOfClass:(current application's NSDictionary)) as boolean then
							set errorCode to (errorObject's objectForKey:"code") as integer
							set errorText to (errorObject's objectForKey:"message") as text
							error "EditorError " & errorCode & ": " & errorText number errorCode
						end if
					end if
					set resultObject to replyObject's objectForKey:"result"
					if resultObject is missing value then return missing value
					if (resultObject's isKindOfClass:(current application's NSString)) as boolean then return resultObject as text
					if (resultObject's isKindOfClass:(current application's NSNumber)) as boolean then return my editorNumber(resultObject)
					return missing value
				end if
			end if
		end if
	end repeat
end editorCall

-- A number of the editor: an integer if it is whole, a real if not.
on editorNumber(someNumber)
	set realValue to someNumber's doubleValue() as real
	set wholeValue to round realValue rounding toward zero
	if realValue is wholeValue then return wholeValue
	return realValue
end editorNumber

-- What the editor told the macro in `initialize`, by key path: editorInfo("editor.name"),
-- editorInfo("document.path"), editorInfo("processId"). `missing value` if there is none.
on editorInfo(keyPath)
	set theValue to editorInfoObject's valueForKeyPath:keyPath
	if theValue is missing value then return missing value
	if (theValue's isKindOfClass:(current application's NSString)) as boolean then return theValue as text
	if (theValue's isKindOfClass:(current application's NSNumber)) as boolean then return my editorNumber(theValue)
	return missing value
end editorInfo

-- Reads the `initialize` line.
on editorStart()
	set my editorBuffer to current application's NSMutableData's |data|()
	set firstLine to my editorReadLine()
	if firstLine is missing value then error "the editor did not send initialize" number -32700
	set firstObject to my editorParse(firstLine)
	if firstObject is missing value then error "the editor did not send initialize" number -32700
	set my editorInfoObject to firstObject's objectForKey:"params"
end editorStart

-- The user's script follows. Its statements are the statements of the script object; a handler
-- `on run` of its own is run as well.
script UserMacro
