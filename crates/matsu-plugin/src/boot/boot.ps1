# The boot library of the PowerShell macros (PowerShell 7 and Windows PowerShell 5.1).
#
# Cmdlets are avoided on the way in (New-Object, Join-Path, Add-Member): the first use of the
# modules that hold them costs 100 to 300 ms, and the start-up is slow enough.
#
# The editor starts PowerShell with -Command, which reads this file as text and dot-sources it as
# a script block (an execution policy governs script files, not text). The first line of the
# standard input is the `initialize` notification; its params.script holds the name and the text
# of the user's script. The text is compiled with [scriptblock]::Create and run in the global
# scope with $Editor (api_gen.ps1) ready, so the script needs no import.
#
# The rest follows docs/poc/json-rpc.md (sections 2 and 3): the lines are read and written
# synchronously, a line to the editor starts with RS, and every line is ASCII.
#
# This file is ASCII too: Windows PowerShell 5.1 reads a file without a BOM in the ANSI code
# page.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$global:MatsuRS = [string][char]0x1e
$global:MatsuNextId = 1
$utf8 = [System.Text.UTF8Encoding]::new($false)
# The user's output (Write-Host, Write-Output) is UTF-8, whatever the code page of the console.
try { [Console]::OutputEncoding = $utf8 } catch { }
try { $OutputEncoding = $utf8 } catch { }
# No colour codes in the text that goes to the output panel (PowerShell 7.2 and later; 5.1 has no
# $PSStyle and the assignment fails, which is fine).
try { $PSStyle.OutputRendering = 'PlainText' } catch { }
# The input is read as bytes, so no code page gets in the way (the lines are ASCII anyway).
$global:MatsuIn = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), $utf8, $false)
$global:MatsuOut = [Console]::Out

# ---- JSON ----------------------------------------------------------------------------------
# ConvertFrom-Json is not used: it turns a string that looks like a date into a DateTime in some
# versions, and a macro would get another text than the editor sent. ConvertTo-Json differs
# between 5.1 and 7 (-Depth, escaping). The editor's messages are small objects, so a parser of
# its own is short.

$global:MatsuTokens = [regex]'\G\s*(?:(?<s>"[^"\\]*(?:\\.[^"\\]*)*")|(?<n>-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)|(?<l>true|false|null)|(?<p>[{}\[\],:]))'
$global:MatsuEscape = [regex]'\\(?:u([0-9a-fA-F]{4})|(.))'
$global:MatsuUnescape = [System.Text.RegularExpressions.MatchEvaluator] {
    param($m)
    if ($m.Groups[1].Success) { return [string][char][Convert]::ToInt32($m.Groups[1].Value, 16) }
    switch -CaseSensitive ($m.Groups[2].Value) {
        'n' { "`n" }
        'r' { "`r" }
        't' { "`t" }
        'b' { "`b" }
        'f' { "`f" }
        default { $m.Groups[2].Value }
    }
}

function global:Read-MatsuToken {
    $m = $global:MatsuTokens.Match($global:MatsuText, $global:MatsuPos)
    if (-not $m.Success) { throw "malformed JSON from the editor at $($global:MatsuPos)" }
    $global:MatsuPos = $m.Index + $m.Length
    $m
}

function global:Read-MatsuValue($tok) {
    if ($tok.Groups['s'].Success) {
        $raw = $tok.Groups['s'].Value
        $raw = $raw.Substring(1, $raw.Length - 2)
        if ($raw.IndexOf('\') -ge 0) { $raw = $global:MatsuEscape.Replace($raw, $global:MatsuUnescape) }
        return $raw
    }
    if ($tok.Groups['n'].Success) {
        $text = $tok.Groups['n'].Value
        if ($text -match '^-?\d+$') { return [long]$text }
        return [double]::Parse($text, [Globalization.CultureInfo]::InvariantCulture)
    }
    if ($tok.Groups['l'].Success) {
        switch -CaseSensitive ($tok.Groups['l'].Value) {
            'true' { return $true }
            'false' { return $false }
            default { return $null }
        }
    }
    $p = $tok.Groups['p'].Value
    if ($p -eq '{') {
        $object = [ordered]@{}
        $tok = Read-MatsuToken
        if ($tok.Groups['p'].Value -eq '}') { return $object }
        while ($true) {
            $key = Read-MatsuValue $tok
            $tok = Read-MatsuToken
            if ($tok.Groups['p'].Value -ne ':') { throw 'malformed JSON from the editor: ":" expected' }
            $object[$key] = Read-MatsuValue (Read-MatsuToken)
            $tok = Read-MatsuToken
            if ($tok.Groups['p'].Value -eq '}') { return $object }
            if ($tok.Groups['p'].Value -ne ',') { throw 'malformed JSON from the editor: "," expected' }
            $tok = Read-MatsuToken
        }
    }
    if ($p -eq '[') {
        $items = [System.Collections.ArrayList]::new()
        $tok = Read-MatsuToken
        if ($tok.Groups['p'].Value -eq ']') { return , $items.ToArray() }
        while ($true) {
            [void]$items.Add((Read-MatsuValue $tok))
            $tok = Read-MatsuToken
            if ($tok.Groups['p'].Value -eq ']') { return , $items.ToArray() }
            if ($tok.Groups['p'].Value -ne ',') { throw 'malformed JSON from the editor: "," expected' }
            $tok = Read-MatsuToken
        }
    }
    throw "malformed JSON from the editor: unexpected '$p'"
}

function global:ConvertFrom-MatsuJson([string]$Text) {
    $global:MatsuText = $Text
    $global:MatsuPos = 0
    Read-MatsuValue (Read-MatsuToken)
}

$global:MatsuNonAscii = [regex]'[^\x20-\x7e]'
$global:MatsuAsciiEscape = [System.Text.RegularExpressions.MatchEvaluator] {
    param($m)
    # By number: a switch on the strings would take some ignorable characters (U+FEFF, U+00AD,
    # NUL) for a backspace.
    $code = [int]$m.Value[0]
    switch ($code) {
        10 { '\n' }
        13 { '\r' }
        9 { '\t' }
        8 { '\b' }
        12 { '\f' }
        default { '\u' + $code.ToString('x4') }
    }
}

# A string as JSON, ASCII only (a character above U+FFFF is a surrogate pair already).
function global:ConvertTo-MatsuJsonString([string]$Text) {
    $Text = $Text.Replace('\', '\\').Replace('"', '\"')
    '"' + $global:MatsuNonAscii.Replace($Text, $global:MatsuAsciiEscape) + '"'
}

# ---- Errors ----------------------------------------------------------------------------------

# The line of the macro's own code that is calling the editor: the frame two above
# Invoke-MatsuCall (the one in between is the script method of $Editor). 0 when it is not
# found. The line counts from the first line of the macro, since the text is compiled as it is.
function global:Get-MatsuCallerLine {
    $stack = @(Get-PSCallStack)
    for ($i = 0; $i -lt $stack.Count; $i++) {
        if ($stack[$i].FunctionName -eq 'Invoke-MatsuCall') {
            if ($i + 2 -lt $stack.Count) { return [int]$stack[$i + 2].ScriptLineNumber }
            return 0
        }
    }
    0
}

# An error of the editor (a JSON-RPC error object) or of the macro's own call. A method of
# $Editor is a script method, and PowerShell wraps what it throws in a
# ScriptMethodRuntimeException, so a macro that catches it asks Get-EditorErrorCode, below. The
# exception made here has the code in Data['Code'] and the line of the macro that made the call
# in Data['Line'].
# The exception of an editor error in an error record (the one that PowerShell wrapped, or the
# record's own), or $null if the record is some other error.
function global:Get-MatsuEditorException($Record) {
    $e = $Record.Exception
    while ($e) {
        if ($e.Data -and $e.Data.Contains('Code') -and $e.Data.Contains('Line')) { return $e }
        $e = $e.InnerException
    }
    $null
}

# For a macro: the code of the editor's error in $_ (for example -32602), or $null if $_ is not
# an error of the editor.
function global:Get-EditorErrorCode($Record) {
    $e = Get-MatsuEditorException $Record
    if ($e) { return $e.Data['Code'] }
    $null
}

function global:New-MatsuError([long]$Code, [string]$Message) {
    $ex = [System.InvalidOperationException]::new("EditorError ${Code}: $Message")
    $Line = Get-MatsuCallerLine
    $ex.Data['Code'] = $Code
    $ex.Data['Line'] = $Line
    [System.Management.Automation.ErrorRecord]::new($ex, 'EditorError', [System.Management.Automation.ErrorCategory]::InvalidOperation, $null)
}

# ---- Calls -----------------------------------------------------------------------------------

function global:ConvertTo-MatsuJsonValue($Value) {
    if ($null -eq $Value) { return 'null' }
    if ($Value -is [string]) { return ConvertTo-MatsuJsonString $Value }
    if ($Value -is [bool]) { if ($Value) { return 'true' } else { return 'false' } }
    if ($Value -is [int] -or $Value -is [long] -or $Value -is [int16] -or $Value -is [byte] -or $Value -is [uint16] -or $Value -is [uint32]) {
        return ([long]$Value).ToString([Globalization.CultureInfo]::InvariantCulture)
    }
    if ($Value -is [double] -or $Value -is [single] -or $Value -is [decimal]) {
        $d = [double]$Value
        # NaN and infinity are not JSON; the editor would answer a parse error that nobody waits for.
        if ([double]::IsNaN($d) -or [double]::IsInfinity($d)) {
            throw (New-MatsuError -32602 'cannot send NaN or infinity to the editor')
        }
        # A whole number goes as an integer ('4 / 2' is 2, not 2.0), as in the other languages.
        if ($d -eq [math]::Floor($d) -and [math]::Abs($d) -lt 9007199254740992) { return ([long]$d).ToString([Globalization.CultureInfo]::InvariantCulture) }
        return $d.ToString('R', [Globalization.CultureInfo]::InvariantCulture)
    }
    throw (New-MatsuError -32602 "cannot send a value of type $($Value.GetType().Name) to the editor")
}

function global:Read-MatsuLine {
    $line = $global:MatsuIn.ReadLine()
    $line
}

# Sends a request and waits for the answer with its id.
function global:Invoke-MatsuCall([string]$Method, $Arguments) {
    $id = $global:MatsuNextId
    $global:MatsuNextId = $id + 1
    $parts = [System.Collections.Generic.List[string]]::new()
    foreach ($a in @($Arguments)) { $parts.Add((ConvertTo-MatsuJsonValue $a)) }
    $request = '{"jsonrpc":"2.0","id":' + $id + ',"method":' + (ConvertTo-MatsuJsonString $Method) + ',"params":[' + ($parts -join ',') + ']}'
    $global:MatsuOut.Write($global:MatsuRS + $request + "`n")
    $global:MatsuOut.Flush()
    while ($true) {
        $text = Read-MatsuLine
        if ($null -eq $text) { throw (New-MatsuError -32000 "The editor closed the connection before it answered $Method") }
        $text = $text.Trim()
        if ($text -eq '') { continue }
        $message = ConvertFrom-MatsuJson $text
        # A notification, or an answer to something that is no longer waited for.
        if (-not ($message -is [System.Collections.IDictionary]) -or $message['id'] -ne $id) { continue }
        if ($message.Contains('error') -and $null -ne $message['error']) {
            $e = $message['error']
            throw (New-MatsuError ([long]$e['code']) ([string]$e['message']))
        }
        return $message['result']
    }
}

# ---- Start -----------------------------------------------------------------------------------

# The line of the macro's own code from an error, for "name:LINE: message".
function global:Get-MatsuErrorLine($Record) {
    $e = Get-MatsuEditorException $Record
    if ($e -and $e.Data['Line'] -gt 0) { return [int]$e.Data['Line'] }
    if ($Record.InvocationInfo) { return [int]$Record.InvocationInfo.ScriptLineNumber }
    0
}

function global:Write-MatsuFailure([string]$Name, [int]$Line, [string]$Message) {
    [Console]::Error.WriteLine("${Name}:${Line}: $Message")
}

$global:MatsuInfo = $null
$first = Read-MatsuLine
if ($null -ne $first -and $first.Trim() -ne '') {
    $message = ConvertFrom-MatsuJson $first
    if ($message -is [System.Collections.IDictionary] -and $message['method'] -eq 'initialize') { $global:MatsuInfo = $message['params'] }
}
if ($null -eq $global:MatsuInfo -or -not $global:MatsuInfo.Contains('script')) {
    [Console]::Error.WriteLine('the editor did not send the script in initialize')
    exit 1
}
$global:MatsuName = [string]$global:MatsuInfo['script']['name']
$global:MatsuSource = [string]$global:MatsuInfo['script']['source']

. ([scriptblock]::Create([IO.File]::ReadAllText(([IO.Path]::Combine($env:MATSU_BOOT_DIR, 'api_gen.ps1')), $utf8)))
$global:Editor = New-MatsuEditor
$global:Editor.Info = $global:MatsuInfo

$userCode = $null
try {
    $userCode = [scriptblock]::Create($global:MatsuSource)
} catch {
    # A syntax error: the parser knows the line.
    $line = 0
    $inner = $_.Exception
    while ($inner.InnerException) { $inner = $inner.InnerException }
    if ($inner.PSObject.Properties['Errors'] -and $inner.Errors.Count -gt 0) {
        $line = [int]$inner.Errors[0].Extent.StartLineNumber
        Write-MatsuFailure $global:MatsuName $line $inner.Errors[0].Message
    } else {
        Write-MatsuFailure $global:MatsuName 0 $inner.Message
    }
    exit 1
}

try {
    # Dot-sourced: the macro's variables and functions are in the global scope, as in a console.
    # Out-Default ends its pipeline here, so that output which PowerShell holds back to format
    # (a table) is written before the process exits.
    . $userCode | Out-Default
} catch {
    $record = $_
    $editorError = Get-MatsuEditorException $record
    if ($editorError) { $text = $editorError.Message } else { $text = $record.Exception.Message }
    Write-MatsuFailure $global:MatsuName (Get-MatsuErrorLine $record) $text
    exit 1
}
exit 0
