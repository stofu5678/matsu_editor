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

/// What comes before the line of the user.
pub const PRELUDE: &str = r#"$ProgressPreference = 'SilentlyContinue'
try { if ($PSStyle) { $PSStyle.OutputRendering = 'PlainText' } } catch {}
try { [Console]::InputEncoding = New-Object System.Text.UTF8Encoding $false } catch {}
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false } catch {}
$OutputEncoding = New-Object System.Text.UTF8Encoding $false
$global:LASTEXITCODE = $null
$global:__matsu_other = $false
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

/// What comes after the line of the user.
pub const EPILOGUE: &str = r#"
} *>&1 | ForEach-Object { if ($_ -is [System.Management.Automation.ErrorRecord]) { if ($_.FullyQualifiedErrorId -notlike 'NativeCommandError*') { $global:__matsu_other = $true }; [Console]::Error.WriteLine($_.ToString()) } elseif ($_ -is [System.Management.Automation.WarningRecord]) { [Console]::Error.WriteLine('WARNING: ' + $_.Message) } elseif ($_ -is [System.Management.Automation.VerboseRecord]) { [Console]::Error.WriteLine('VERBOSE: ' + $_.Message) } elseif ($_ -is [System.Management.Automation.DebugRecord]) { [Console]::Error.WriteLine('DEBUG: ' + $_.Message) } elseif ($_ -is [System.Management.Automation.InformationRecord]) { [Console]::Error.WriteLine([string]$_.MessageData) } else { $_ } } | Out-Default
} catch { [Console]::Error.WriteLine($_.ToString()); exit 1 }
$__matsu_code = $global:LASTEXITCODE
if (($__matsu_code -is [int]) -and ($__matsu_code -ne 0)) { exit $__matsu_code }
if ($global:__matsu_other) { exit 1 }
exit 0
"#;

/// The whole script for `line`. A line break before the epilogue keeps a `#` comment at the end of
/// the line from taking the closing brace with it.
pub fn script(line: &str) -> String {
    let mut text = String::with_capacity(PRELUDE.len() + line.len() + EPILOGUE.len());
    text.push_str(PRELUDE);
    text.push_str(line);
    text.push_str(EPILOGUE);
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
