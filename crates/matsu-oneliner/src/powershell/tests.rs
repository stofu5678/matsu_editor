use super::*;

#[test]
fn base64_matches_rfc_4648() {
    for (input, expected) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
        ("fooba", "Zm9vYmE="),
        ("foobar", "Zm9vYmFy"),
    ] {
        assert_eq!(base64(input.as_bytes()), expected, "{input:?}");
    }
    assert_eq!(base64(&[0xFB, 0xFF, 0xFE]), "+//+");
}

/// Reads an encoded command back: Base64, then UTF-16LE.
fn decode(encoded: &str) -> String {
    let value = |c: u8| match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => unreachable!("not Base64: {}", c as char),
    };
    let mut bytes = Vec::new();
    let data = encoded.trim_end_matches('=').as_bytes();
    for group in data.chunks(4) {
        let mut n = 0u32;
        for (i, &c) in group.iter().enumerate() {
            n |= u32::from(value(c)) << (18 - 6 * i);
        }
        let count = group.len() - 1;
        for i in 0..count {
            bytes.push((n >> (16 - 8 * i)) as u8);
        }
    }
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    String::from_utf16(&units).unwrap()
}

#[test]
fn the_encoded_command_is_the_script_in_utf16le() {
    for line in ["$input | Sort-Object", "日本語 😀 'quoted' \"double\"", ""] {
        assert_eq!(decode(&encoded_command(line)), script(line), "{line:?}");
    }
}

#[test]
fn the_script_is_the_prelude_the_line_and_the_epilogue() {
    let line = "$input | % { $_.ToUpper() }";
    let text = script(line).replace("\r\n", "\n");
    let want = format!("{PRELUDE}{line}{EPILOGUE}");
    assert_eq!(text, want.replace("\r\n", "\n"));
    // the line is on a line of its own, so that a comment at its end cannot take the brace
    assert!(text.contains(&format!("\n{line}\n")));
}

#[test]
fn the_script_has_windows_line_breaks_only() {
    let text = script("a\nb\r\nc");
    assert!(!text.replace("\r\n", "").contains('\n'));
    assert!(!text.replace("\r\n", "").contains('\r'));
}

#[test]
fn the_prelude_sets_utf8_without_a_bom_and_reads_with_the_console() {
    for needed in [
        "[Console]::InputEncoding = New-Object System.Text.UTF8Encoding $false",
        "[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding $false",
        "$OutputEncoding = New-Object System.Text.UTF8Encoding $false",
        "[Console]::In.ReadToEnd()",
        "$ProgressPreference = 'SilentlyContinue'",
        "$PSStyle.OutputRendering = 'PlainText'",
    ] {
        assert!(PRELUDE.contains(needed), "{needed}");
    }
    // `$input` reads in the code page of the start: the script must not read it itself
    assert!(!PRELUDE.contains("$input"));
}

#[test]
fn the_epilogue_keeps_the_code_of_a_native_program() {
    assert!(EPILOGUE.contains("exit $__matsu_code"));
    assert!(EPILOGUE.contains("$global:LASTEXITCODE"));
    assert!(EPILOGUE.contains("exit 1"));
    // `$?` is lost by a `return` in the line and wrong after a native program's standard error
    assert!(!EPILOGUE.contains("$?"));
    assert!(EPILOGUE.contains("Out-Default"));
}
