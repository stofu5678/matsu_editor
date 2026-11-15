//! The script that runs a one-liner in PowerShell (ADR-0029, decision 3).
//!
//! The arguments are `-NoProfile -NonInteractive -EncodedCommand <Base64 of UTF-16LE>`.
//! `-EncodedCommand` needs no quoting of the line, and unlike `-Command -` it leaves the standard
//! input to the text (`-Command -` reads the script from it). The script is
//! [`PRELUDE`], the line of the user, [`EPILOGUE`].
//!
//! What it does, as measured on Windows PowerShell 5.1 (docs/poc/oneliner.md):
//!
//! * The encodings of the console and `$OutputEncoding` become UTF-8 without a BOM. The text is
//!   read with `[Console]::In.ReadToEnd()`: `$input` reads in the code page the process started
//!   with (CP932 on Japanese Windows) and a later change does not reach it.
//! * The line runs as a script block with the lines of the text as its pipeline input, so that
//!   `$input` in it is the lines. A text that ends with a line break has no empty last line;
//!   empty lines inside stay; no text is no line.
//! * `-EncodedCommand` makes PowerShell write its standard error as CLIXML (the `#< CLIXML`
//!   header and XML around every message), in the code page of the console, whatever
//!   `-OutputFormat` says. The streams of the line other than the output (error, warning,
//!   verbose, debug and information, which is what `Write-Host` writes) are therefore merged and
//!   written by the script itself to the standard error as plain text in UTF-8, and
//!   `$ProgressPreference` is silent (a progress record is CLIXML too). Anything of the host's own
//!   that is still written as CLIXML (a script that does not parse) is shown as it is.
//! * PowerShell 7 puts ANSI colors into tables even when its output goes to a pipe;
//!   `$PSStyle.OutputRendering` is set to `PlainText` (Windows PowerShell has no `$PSStyle`).
//! * The output objects are handed to `Out-Default` before the script ends, so that a table or a
//!   list is written whole (an `exit` right after the pipeline would cut it off).
//! * The exit code: `-Command` rounds the code of a native program to 0 or 1, so the script
//!   keeps the code of the last native program (`cmd /c exit 5` gives 5); else 1 when a command
//!   wrote an error that is not the standard error of a native program (a failing cmdlet, a
//!   `Write-Error`, a `throw`); else 0. `$?` is not used: it is false in Windows PowerShell 5.1 for
//!   a native program that merely writes to its standard error when the error stream is merged
//!   (as the script does), and a `return` in the line skips whatever the script puts after the
//!   line. A mid-line failure is therefore a failure even when a later command succeeds: the
//!   caller changes nothing and shows the error, which is the safe side for a filter.
//! * A line that starts with a command that only filters or reshapes its input (`ForEach-Object`,
//!   `Where-Object`, `Sort-Object`, ...: [`FILTERS`]) is run as if it began with `$input | `
//!   (decision of 2026-11-16: `ForEach-Object { $_.ToUpper() }` got no input and `$_` was null).
//!   The line of the user is not changed in what is shown; only the script has the prefix.
//!   Every other line gets no pipeline input, so that `Remove-Item` or `Get-ChildItem` never
//!   take the text as paths.
//! * A line that does not mention `$input`, was not piped like that and fails gets one more line
//!   on the standard error ([`HINT`]) that says the selection is read with `$input`. The standard
//!   error is shown in the output window and never goes into the document.

/// What comes before the line of the user.
pub const PRELUDE: &str = r#"$ProgressPreference = 'SilentlyContinue'
try { if ($PSStyle) { $PSStyle.OutputRendering = 'PlainText' } } catch {}
try { [Console]::InputEncoding = New-Object System.Text.UTF8Encoding $false } catch {}
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false } catch {}
$OutputEncoding = New-Object System.Text.UTF8Encoding $false
$global:LASTEXITCODE = $null
$global:__matsu_other = $false
$global:__matsu_hint = $false
$__matsu_raw = [Console]::In.ReadToEnd().Replace("`r`n", "`n").Replace("`r", "`n")
if ($__matsu_raw.Length -eq 0) {
$__matsu_lines = [string[]]@()
} else {
if ($__matsu_raw.EndsWith("`n")) { $__matsu_raw = $__matsu_raw.Substring(0, $__matsu_raw.Length - 1) }
$__matsu_lines = [string[]]$__matsu_raw.Split([char]10)
}
try {
$__matsu_lines | & {
"#;

/// What comes after the line of the user. `@@HINT@@` is replaced by [`HINT`].
pub const EPILOGUE: &str = r#"
} *>&1 | ForEach-Object { if ($_ -is [System.Management.Automation.ErrorRecord]) { if ($_.FullyQualifiedErrorId -notlike 'NativeCommandError*') { $global:__matsu_other = $true }; [Console]::Error.WriteLine($_.ToString()) } elseif ($_ -is [System.Management.Automation.WarningRecord]) { [Console]::Error.WriteLine('WARNING: ' + $_.Message) } elseif ($_ -is [System.Management.Automation.VerboseRecord]) { [Console]::Error.WriteLine('VERBOSE: ' + $_.Message) } elseif ($_ -is [System.Management.Automation.DebugRecord]) { [Console]::Error.WriteLine('DEBUG: ' + $_.Message) } elseif ($_ -is [System.Management.Automation.InformationRecord]) { [Console]::Error.WriteLine([string]$_.MessageData) } else { $_ } } | Out-Default
} catch { [Console]::Error.WriteLine($_.ToString()); if ($global:__matsu_hint) { [Console]::Error.WriteLine('@@HINT@@') }; exit 1 }
$__matsu_code = $global:LASTEXITCODE
if (($__matsu_code -is [int]) -and ($__matsu_code -ne 0)) { if ($global:__matsu_hint) { [Console]::Error.WriteLine('@@HINT@@') }; exit $__matsu_code }
if ($global:__matsu_other) { if ($global:__matsu_hint) { [Console]::Error.WriteLine('@@HINT@@') }; exit 1 }
exit 0
"#;

/// The hint written to the standard error when a line that does not use `$input` fails. The
/// epilogue has it where `@@HINT@@` is. It has no single quote (the script writes it in one).
pub const HINT: &str =
    "選択範囲は $input で読めます（例: $input | ForEach-Object { $_.ToUpper() }）";

/// The commands, and their aliases in Windows PowerShell 5.1 and PowerShell 7, that only filter or
/// reshape what they are piped, in lower case. A line that starts with one is given the text as
/// its pipeline input.
///
/// Left out on purpose:
///
/// * `Tee-Object` (`tee`), `Out-File`, `Export-*`, `Set-Content`, `Add-Content`: they write files.
/// * `Get-Content`, `Get-ChildItem`, `Remove-Item`, `Invoke-*`, and the like: they would take the
///   text as paths, URLs or commands.
/// * `Write-Output` (`echo`, `write`) and `Write-Host`: a line that starts with them has its own
///   argument, and the piped text would be a second input.
/// * `Compare-Object` (`compare`, `diff`): it needs `-ReferenceObject`, which is not the text.
/// * `Out-Null`, `Out-Host`, `Out-Default`: they drop or only show the input.
///
/// `sort` is an alias of `Sort-Object` in Windows PowerShell 5.1 and in PowerShell 7 on Windows.
/// In PowerShell 7 on Linux and macOS it is the program `sort`, which reads its standard input; it
/// is a filter either way, so the text is piped to it there too.
pub const FILTERS: &[&str] = &[
    "foreach-object",
    "%",
    "foreach",
    "where-object",
    "?",
    "where",
    "sort-object",
    "sort",
    "select-object",
    "select",
    "select-string",
    "sls",
    "group-object",
    "group",
    "measure-object",
    "measure",
    "get-unique",
    "gu",
    "format-table",
    "ft",
    "format-list",
    "fl",
    "format-wide",
    "fw",
    "format-custom",
    "fc",
    "out-string",
    "convertto-json",
    "convertto-csv",
    "convertfrom-json",
    "convertfrom-csv",
];

/// Whether `line` mentions `$input` (any case) anywhere.
pub fn mentions_input(line: &str) -> bool {
    line.to_ascii_lowercase().contains("$input")
}

/// Whether the text is piped into `line` as if it began with `$input | `: it does not mention
/// `$input` and its first command is one of [`FILTERS`].
pub fn pipes_input(line: &str) -> bool {
    if mentions_input(line) {
        return false;
    }
    let line = line.trim_start();
    let end = line
        .find(|c: char| c.is_whitespace() || matches!(c, '{' | '(' | '|' | ';' | '&'))
        .unwrap_or(line.len());
    let (word, rest) = line.split_at(end);
    let word = word.to_ascii_lowercase();
    if !FILTERS.contains(&word.as_str()) {
        return false;
    }
    // `foreach ($x in $list) { }` is the statement, not the alias of ForEach-Object
    if word == "foreach" && rest.trim_start().starts_with('(') {
        return false;
    }
    // The line gives its input itself (`Sort-Object -InputObject 3,2,1`; `Select-String a -Path
    // file` reads the file): the text on top of it would be a parameter-binding error. What is
    // checked is the parameters that supply the input of the commands of [`FILTERS`]:
    // `-InputObject` (every command of the list has it; PowerShell binds any unambiguous
    // prefix, so `-InputO`, `-inp`, ... count) and, for `Select-String`, `-Path` (`-Pat` is
    // ambiguous with `-Pattern`, so only the full name) and `-LiteralPath` (the ways it is given
    // files; `-Include`/`-Exclude` only narrow them). `-i` and `-in` bind `-InputObject` in some
    // commands, are ambiguous in others and are the operator `-in` of `Where-Object`: they are
    // not counted, so such a line is piped the text and fails; write `-InputObject` in full.
    // Only the words outside `{ }`, `( )` and `[ ]` and quotes are looked at, so that a
    // script block does not count.
    for name in top_level_parameters(rest) {
        if name.len() >= 3 && "inputobject".starts_with(name.as_str()) {
            return false;
        }
        if matches!(word.as_str(), "select-string" | "sls")
            && (name == "path" || (name.len() >= 2 && "literalpath".starts_with(name.as_str())))
        {
            return false;
        }
    }
    true
}

/// The names (lower case, without the dash, up to a `:`) of the parameters written as words of
/// `text` outside any `{ }`, `( )` and `[ ]` and outside quotes.
fn top_level_parameters(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut word = String::new();
    let flush = |word: &mut String, names: &mut Vec<String>| {
        if let Some(name) = word.strip_prefix('-') {
            let name = name.split(':').next().unwrap_or("");
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic()) {
                names.push(name.to_ascii_lowercase());
            }
        }
        word.clear();
    };
    for c in text.chars() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                quote = Some(c);
                word.clear();
            }
            '{' | '(' | '[' => {
                if depth == 0 {
                    flush(&mut word, &mut names);
                }
                depth += 1;
            }
            '}' | ')' | ']' => depth = depth.saturating_sub(1),
            c if c.is_whitespace() || c == '|' || c == ';' => {
                if depth == 0 {
                    flush(&mut word, &mut names);
                }
            }
            c if depth == 0 => word.push(c),
            _ => {}
        }
    }
    flush(&mut word, &mut names);
    names
}

/// Whether the hint is armed for `line`: it does not mention `$input` and was not piped.
fn hint_applies(line: &str) -> bool {
    !mentions_input(line) && !pipes_input(line)
}

/// The whole script for `line`. A line break before the epilogue keeps a `#` comment at the end of
/// the line from taking the closing brace with it.
pub fn script(line: &str) -> String {
    let mut text = String::with_capacity(PRELUDE.len() + line.len() + EPILOGUE.len());
    // The flag is set in the prelude, outside the script block of the line, so that the block
    // starts with the text of the user (`param()`, `process { }` must be the first thing in it).
    text.push_str(&PRELUDE.replace(
        "$global:__matsu_hint = $false",
        if hint_applies(line) {
            "$global:__matsu_hint = $true"
        } else {
            "$global:__matsu_hint = $false"
        },
    ));
    if pipes_input(line) {
        text.push_str("$input | ");
    }
    text.push_str(line);
    text.push_str(&EPILOGUE.replace("@@HINT@@", HINT));
    // PowerShell takes either; the line breaks of Windows are what its own files have
    text.replace("\r\n", "\n").replace('\n', "\r\n")
}

/// The value of `-EncodedCommand` for `line`: the script as UTF-16LE in Base64.
pub fn encoded_command(line: &str) -> String {
    let bytes: Vec<u8> = script(line)
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    base64(&bytes)
}

/// Base64 with `=` padding (RFC 4648), the form `-EncodedCommand` reads.
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let n = (u32::from(group[0]) << 16)
            | (u32::from(*group.get(1).unwrap_or(&0)) << 8)
            | u32::from(*group.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= group.len() {
                out.push(char::from(ALPHABET[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
