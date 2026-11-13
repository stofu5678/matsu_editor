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
//!   `-OutputFormat` says. The error stream of the line is therefore written by the script itself,
//!   as plain text in UTF-8, and `$ProgressPreference` is silent (a progress record is CLIXML
//!   too).
//! * The exit code: `-Command` rounds the code of a native program to 0 or 1, so the script
//!   keeps the code of the last native program (`cmd /c exit 5` gives 5); else 1 when the last
//!   command failed (`$?`); else 0. A native program that writes to its standard error leaves
//!   `$?` false in Windows PowerShell 5.1 when the error stream is merged (as the script does)
//!   even though it exited with 0; that alone is not a failure.

/// What comes before the line of the user.
pub const PRELUDE: &str = r#"$ProgressPreference = 'SilentlyContinue'
try { [Console]::InputEncoding = New-Object System.Text.UTF8Encoding $false } catch {}
try { [Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false } catch {}
$OutputEncoding = New-Object System.Text.UTF8Encoding $false
$global:LASTEXITCODE = $null
$global:__matsu_native = $false
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
$global:__matsu_ok = $?
$global:__matsu_code = $LASTEXITCODE
} 2>&1 | ForEach-Object { if ($_ -is [System.Management.Automation.ErrorRecord]) { if ($_.FullyQualifiedErrorId -like 'NativeCommandError*') { $global:__matsu_native = $true } else { $global:__matsu_other = $true }; [Console]::Error.WriteLine($_.ToString()) } else { $_ } }
} catch { [Console]::Error.WriteLine($_.ToString()); exit 1 }
$__matsu_code = $global:__matsu_code
if (($__matsu_code -is [int]) -and ($__matsu_code -ne 0)) { exit $__matsu_code }
if (-not $global:__matsu_ok) {
if ($global:__matsu_native -and -not $global:__matsu_other) { exit 0 }
exit 1
}
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
