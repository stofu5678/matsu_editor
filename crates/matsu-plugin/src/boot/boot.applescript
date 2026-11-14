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

-- ---- Plugin host (ADR-0026) -------------------------------------------------------------------
--
-- A plugin host: `initialize` says kind "plugin" and brings no script. The editor sends
-- `plugin/invoke` (the jack, the name and the text of the handler, the arguments) and waits for
-- the answer. A handler cannot be a part of this program (its text comes with each call), so the
-- host builds a program for it: this boot text, the handler inside `script UserMacro ... end
-- script`, and an `on run`; compiles it with NSAppleScript (which says where an error is) and
-- runs it. That program is a script of its own: it sees none of the variables of the host's
-- loop, and what the handler sets (properties, handlers) is gone when the call ends. Its own
-- `return` ends only the handler, and so does an `error` with the number -128 (the way a script
-- says "stop quietly"); any other `error` is a failure, with its kind, its message and its place
-- in the handler. Output that the handler wrote before a failure is not lost: `editorPrint`
-- writes at once.
--
-- The handler sees `pluginId()`, `pluginDir()`, `pluginJack()`, `pluginArg("key")`,
-- `complementGetCurrentWord()`, `complementGetOption()`, `indentGetChar()`, and calls the
-- objects of the plugin through `editorCall("Plugin.GetOption", {"Section", "Key"})`,
-- `editorCall("Outline.SetTitle", {"Title"})` and `editorCall("Complement.AddList", {"word"})`.
property pluginIdentity : missing value
property pluginContextText : missing value
property pluginContext : missing value

-- An NSString as text; any other object (or `missing value`) as "".
on editorString(someObject)
	if someObject is missing value then return ""
	if (someObject's isKindOfClass:(current application's NSString)) as boolean then return someObject as text
	return ""
end editorString

-- A field of the call that is a string: "id", "dir" or "jack". "" outside a plugin.
on pluginString(keyName)
	if pluginContext is missing value then return ""
	return my editorString(pluginContext's objectForKey:keyName)
end pluginString

on pluginId()
	return my pluginString("id")
end pluginId

on pluginDir()
	return my pluginString("dir")
end pluginDir

on pluginJack()
	return my pluginString("jack")
end pluginJack

-- A member of the arguments of the call: text or a number, or `missing value` if there is none.
on pluginArg(keyName)
	if pluginContext is missing value then return missing value
	set argsObject to pluginContext's objectForKey:"args"
	if argsObject is missing value then return missing value
	if not ((argsObject's isKindOfClass:(current application's NSDictionary)) as boolean) then return missing value
	set theValue to argsObject's objectForKey:keyName
	if theValue is missing value then return missing value
	if (theValue's isKindOfClass:(current application's NSString)) as boolean then return theValue as text
	if (theValue's isKindOfClass:(current application's NSNumber)) as boolean then return my editorNumber(theValue)
	return missing value
end pluginArg

-- The jack Complement: the word to complete.
on complementGetCurrentWord()
	set theWord to my pluginArg("word")
	if theWord is missing value then return ""
	return theWord as text
end complementGetCurrentWord

-- The jack Complement: the option of the call.
on complementGetOption()
	set theOption to my pluginArg("option")
	if theOption is missing value then return 0
	try
		return theOption as integer
	end try
	return 0
end complementGetOption

-- The jack SmartIndent: the char that was typed. "" in the other jacks.
on indentGetChar()
	if my pluginJack() is not "SmartIndent" then return ""
	set theChar to my pluginArg("char")
	if theChar is missing value then return ""
	return theChar as text
end indentGetChar

-- The first thing a handler's program does: the buffer for the lines, and what the host says
-- about this call (`pluginContextText`, set by the program's `on run`).
on pluginPrepare()
	set my editorBuffer to current application's NSMutableData's |data|()
	set my pluginContext to my editorParse(pluginContextText)
	if pluginContext is not missing value then set my editorInfoObject to pluginContext's objectForKey:"info"
end pluginPrepare

-- The number of lines of the text (a text that ends with a line break has no line after it).
on pluginLineBreaks(someString)
	return ((someString's componentsSeparatedByString:linefeed)'s |count|()) - 1
end pluginLineBreaks

-- The boot text (this text), as the editor wrote it to the private folder of the run.
on pluginBootText()
	set bootFolder to system attribute "MATSU_BOOT_DIR"
	set bootString to current application's NSString's stringWithContentsOfFile:(bootFolder & "/boot.applescript") encoding:(editorUTF8) |error|:(missing value)
	if bootString is missing value then error "EditorError -32000: the boot text of the plugin host cannot be read" number -32000
	set bootString to bootString's stringByReplacingOccurrencesOfString:((character id 13) & linefeed) withString:linefeed
	return bootString's stringByReplacingOccurrencesOfString:(character id 13) withString:linefeed
end pluginBootText

-- A text as the inside of an AppleScript string literal.
on pluginQuote(someString)
	set quoted to someString's stringByReplacingOccurrencesOfString:"\\" withString:"\\\\"
	return (quoted's stringByReplacingOccurrencesOfString:"\"" withString:"\\\"") as text
end pluginQuote

-- The line of the handler (from 1) where an error is, or 0 if it is not in the handler (or is not
-- known). `errorInfo` is the dictionary of NSAppleScript; its range counts UTF-16 units of the program.
on pluginErrorLine(errorInfo, programString, linesBefore, userLines)
	try
		set errorRange to (errorInfo's objectForKey:"NSAppleScriptErrorRange")'s rangeValue()
		set errorStart to location of errorRange
		set programLine to (my pluginLineBreaks(programString's substringToIndex:errorStart)) + 1
		set userLine to programLine - linesBefore
		if userLine >= 1 and userLine <= userLines then return userLine
	end try
	return 0
end pluginErrorLine

-- The text of a failure for the editor: the kind of error, the message and number that
-- AppleScript gave, and the place in the handler.
on pluginFailure(kindName, errorInfo, handlerName, programString, linesBefore, userLines)
	set errorMessage to my editorString(errorInfo's objectForKey:"NSAppleScriptErrorMessage")
	set errorNumber to 0
	try
		set errorNumber to (errorInfo's objectForKey:"NSAppleScriptErrorNumber") as integer
	end try
	set lineNumber to my pluginErrorLine(errorInfo, programString, linesBefore, userLines)
	set failureText to kindName & ": " & errorMessage & " (" & errorNumber & ")" & linefeed & "  at " & handlerName
	if lineNumber > 0 then set failureText to failureText & ":" & lineNumber
	return failureText
end pluginFailure

-- Runs one handler. `missing value` if it went well, else the text of its failure.
on pluginRun(handlerName, handlerSource, paramsObject, bootText)
	set contextDictionary to current application's NSMutableDictionary's |dictionary|()
	set identityId to ""
	set identityDir to ""
	if pluginIdentity is not missing value then
		set identityId to my editorString(pluginIdentity's objectForKey:"id")
		set identityDir to my editorString(pluginIdentity's objectForKey:"dir")
	end if
	contextDictionary's setObject:identityId forKey:"id"
	contextDictionary's setObject:identityDir forKey:"dir"
	contextDictionary's setObject:(my editorString(paramsObject's objectForKey:"jack")) forKey:"jack"
	set argsObject to paramsObject's objectForKey:"args"
	if argsObject is not missing value then contextDictionary's setObject:argsObject forKey:"args"
	if editorInfoObject is not missing value then contextDictionary's setObject:editorInfoObject forKey:"info"
	set {contextData, contextError} to current application's NSJSONSerialization's dataWithJSONObject:contextDictionary options:0 |error|:(reference)
	if contextData is missing value then return "EditorError -32602: the arguments of the call cannot be sent to the handler" & linefeed & "  at " & handlerName
	set contextText to (current application's NSString's alloc()'s initWithData:contextData encoding:(editorUTF8))
	-- The handler's text, with the line breaks of the program (LF), ending with one.
	set userString to current application's NSString's stringWithString:handlerSource
	set userString to userString's stringByReplacingOccurrencesOfString:((character id 13) & linefeed) withString:linefeed
	set userString to userString's stringByReplacingOccurrencesOfString:(character id 13) withString:linefeed
	set userText to userString as text
	if userText is not "" and userText does not end with linefeed then set userText to userText & linefeed
	if userText is "" then set userText to linefeed
	set linesBefore to my pluginLineBreaks(current application's NSString's stringWithString:bootText)
	set userLines to my pluginLineBreaks(current application's NSString's stringWithString:userText)
	set programText to bootText & userText & "end script" & linefeed & linefeed & "on run" & linefeed & tab & "if editorStarted then return" & linefeed & tab & "set my editorStarted to true" & linefeed & tab & "set my pluginContextText to \"" & my pluginQuote(contextText) & "\"" & linefeed & tab & "my pluginPrepare()" & linefeed & tab & "set ignoredResult to run UserMacro" & linefeed & "end run" & linefeed
	set programString to current application's NSString's stringWithString:programText
	set theScript to current application's NSAppleScript's alloc()'s initWithSource:programString
	if theScript is missing value then return "script error: the handler cannot be compiled" & linefeed & "  at " & handlerName
	set {didCompile, compileInfo} to theScript's compileAndReturnError:(reference)
	if not (didCompile as boolean) then return my pluginFailure("script error", compileInfo, handlerName, programString, linesBefore, userLines)
	set {runResult, runInfo} to theScript's executeAndReturnError:(reference)
	if runResult is missing value then
		set errorNumber to 0
		try
			set errorNumber to (runInfo's objectForKey:"NSAppleScriptErrorNumber") as integer
		end try
		-- `error number -128`: the script says "stop quietly".
		if errorNumber is -128 then return missing value
		return my pluginFailure("execution error", runInfo, handlerName, programString, linesBefore, userLines)
	end if
	return missing value
end pluginRun

-- Answers the invoke: ok, or the text `failure`.
on pluginRespond(invokeId, failure)
	set idNumber to 0
	try
		set idNumber to invokeId as integer
	end try
	if failure is missing value then
		my editorWrite((character id 30) & "{\"jsonrpc\":\"2.0\",\"id\":" & idNumber & ",\"result\":{\"ok\":true}}" & linefeed)
		return
	end if
	set theAnswer to current application's NSDictionary's dictionaryWithDictionary:{jsonrpc:"2.0", |id|:idNumber, |error|:{|code|:-32000, |message|:failure}}
	set {theData, theError} to current application's NSJSONSerialization's dataWithJSONObject:theAnswer options:0 |error|:(reference)
	if theData is missing value then
		my editorWrite((character id 30) & "{\"jsonrpc\":\"2.0\",\"id\":" & idNumber & ",\"error\":{\"code\":-32000,\"message\":\"the handler failed (its text cannot be shown)\"}}" & linefeed)
		return
	end if
	my editorWrite((character id 30) & ((current application's NSString's alloc()'s initWithData:theData encoding:(editorUTF8)) as text) & linefeed)
end pluginRespond

-- Serves the calls of the editor until it says shutdown or closes the input.
on pluginServe()
	set my pluginIdentity to editorInfoObject's objectForKey:"plugin"
	set bootText to my pluginBootText()
	repeat
		set theLine to my editorReadLine()
		if theLine is missing value then exit repeat
		if theLine is not "" then
			set theMessage to my editorParse(theLine)
			if theMessage is not missing value then
				set methodName to my editorString(theMessage's objectForKey:"method")
				if methodName is "plugin/shutdown" then exit repeat
				if methodName is "plugin/invoke" then
					set paramsObject to theMessage's objectForKey:"params"
					if paramsObject is missing value then
						set paramsObject to current application's NSDictionary's |dictionary|()
					else if not ((paramsObject's isKindOfClass:(current application's NSDictionary)) as boolean) then
						set paramsObject to current application's NSDictionary's |dictionary|()
					end if
					set handlerName to my editorString(paramsObject's objectForKey:"name")
					if handlerName is "" then set handlerName to "<plugin>"
					set handlerSource to my editorString(paramsObject's objectForKey:"source")
					set failure to my pluginRun(handlerName, handlerSource, paramsObject, bootText)
					my pluginRespond(theMessage's objectForKey:"id", failure)
				end if
			end if
		end if
	end repeat
end pluginServe

-- The user's script follows. Its statements are the statements of the script object; a handler
-- `on run` of its own is run as well.
script UserMacro
