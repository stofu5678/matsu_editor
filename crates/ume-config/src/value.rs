// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CDataProfile.h, sakura_core/env/CDataProfile.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 2003-2006, D.S.Koba
//   Copyright (C) 2004, MIK, genta
//   Copyright (C) 2006, ryoji
//   Copyright (C) 2008, kobake
//   Copyright (C) 2009, ryoji
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// zlib License
//
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
//
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
//
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.

//! Typed values in a profile, read and written the way Sakura's `CDataProfile` does
//! (`sakura_core/env/CDataProfile.h`, namespace `profile_data` and `CDataProfile::IOProfileData`).
//!
//! In Sakura a missing key, or a value that does not parse, leaves the setting at its default. Here the
//! `get_*` functions return `None` in those cases, and callers keep their default.

use crate::profile::{Profile, ProfileError};

fn is_space(c: char) -> bool {
    c.is_whitespace()
}

/// Reads an optional sign and the decimal digits after leading white space, as `wcstol`/`wcstoul` do.
/// Returns the sign, the magnitude (saturated to `u64::MAX`) and whether any digit was read.
fn strto(s: &str) -> (bool, u64, bool) {
    let mut chars = s.trim_start_matches(is_space).chars().peekable();
    let negative = match chars.peek() {
        Some('-') => {
            chars.next();
            true
        }
        Some('+') => {
            chars.next();
            false
        }
        _ => false,
    };
    let mut magnitude: u64 = 0;
    let mut any = false;
    while let Some(d) = chars.peek().and_then(|c| c.to_digit(10)) {
        magnitude = magnitude.saturating_mul(10).saturating_add(u64::from(d));
        any = true;
        chars.next();
    }
    (negative, magnitude, any)
}

/// `::wcstol(s, &end, 10)` on Windows, where `long` has 32 bits: white space, a sign and decimal digits; text
/// after the digits is ignored; a value out of range becomes the nearest limit. `None` if there are no digits
/// (`TryParse<NumType>` in `CDataProfile.h`).
pub fn parse_long(s: &str) -> Option<i32> {
    let (negative, magnitude, any) = strto(s);
    if !any {
        return None;
    }
    Some(if negative {
        i32::try_from(-i128::from(magnitude)).unwrap_or(i32::MIN)
    } else {
        i32::try_from(magnitude).unwrap_or(i32::MAX)
    })
}

/// `::wcstoul(s, &end, 10)` on Windows: like [`parse_long`], but a minus sign negates the value as an unsigned
/// number (so `-1` is `4294967295`) and an overflow becomes `u32::MAX`.
pub fn parse_ulong(s: &str) -> Option<u32> {
    let (negative, magnitude, any) = strto(s);
    if !any {
        return None;
    }
    let value = u32::try_from(magnitude).unwrap_or(u32::MAX);
    Some(if negative {
        value.wrapping_neg()
    } else {
        value
    })
}

/// A bool is an `int` that is not zero (`TryParse<bool>`).
pub fn parse_bool(s: &str) -> Option<bool> {
    parse_long(s).map(|v| v != 0)
}

/// `TryParse<WCHAR>`: the setting for a single character, such as the line number separator. An empty value is
/// `'\0'` ("none"). Sakura refuses a first UTF-16 unit `u` when `(u & 0xd800) == 0xd800`; that is meant to
/// refuse surrogates, but it also refuses U+F800 to U+FFFF (for example the full-width colon U+FF1A). The test
/// is ported as it is, because the same file must read the same way.
pub fn parse_wchar(s: &str) -> Option<char> {
    let Some(unit) = s.encode_utf16().next() else {
        return Some('\0');
    };
    if unit & 0xd800 == 0xd800 {
        return None;
    }
    char::from_u32(u32::from(unit))
}

/// `ToString<WCHAR>`: `'\0'` is written as an empty value.
pub fn format_wchar(c: char) -> String {
    let mut buf = [0u16; 2];
    let unit = c.encode_utf16(&mut buf)[0];
    if c == '\0' || unit & 0xd800 == 0xd800 || c.len_utf16() != 1 {
        String::new()
    } else {
        c.to_string()
    }
}

/// A tiny reader for the `scanf` formats Sakura uses.
struct Scanner<'a> {
    rest: &'a str,
}

impl<'a> Scanner<'a> {
    fn new(s: &'a str) -> Self {
        Scanner { rest: s }
    }

    /// `%d`: skips white space, then reads a signed decimal number (out of range values are clamped).
    fn int(&mut self) -> Option<i32> {
        let value = parse_long(self.rest)?;
        self.rest = skip_number(self.rest);
        Some(value)
    }

    /// `%u`: like `%d`, but the result is unsigned.
    fn uint(&mut self) -> Option<u32> {
        let value = parse_ulong(self.rest)?;
        self.rest = skip_number(self.rest);
        Some(value)
    }

    /// `%06x`: skips white space, then reads at most 6 characters, which may start with `0x`.
    fn hex6(&mut self) -> Option<u32> {
        let s = self.rest.trim_start_matches(is_space);
        let window: String = s.chars().take(6).collect();
        let (digits, prefix) = match window.get(..2) {
            Some("0x" | "0X") => (&window[2..], 2),
            _ => (window.as_str(), 0),
        };
        let len = digits.chars().take_while(char::is_ascii_hexdigit).count();
        if len == 0 {
            return None;
        }
        let value = u32::from_str_radix(&digits[..len], 16).ok()?;
        self.rest = &s[prefix + len..];
        Some(value)
    }

    /// A literal character, which must follow directly (`scanf` does not skip white space before it).
    fn literal(&mut self, c: char) -> Option<()> {
        self.rest = self.rest.strip_prefix(c)?;
        Some(())
    }
}

/// The part of `s` after the sign and digits that `strto` read.
fn skip_number(s: &str) -> &str {
    let s = s.trim_start_matches(is_space);
    let s = s.strip_prefix(['+', '-']).unwrap_or(s);
    s.trim_start_matches(|c: char| c.is_ascii_digit())
}

/// `swscanf(s, L"%d,%d,...,%d", ...)` for `N` numbers. All `N` must be read, or the result is `None`
/// (Sakura treats a short count as a failure). Text after the last number is ignored.
pub fn scan_ints<const N: usize>(s: &str) -> Option<[i32; N]> {
    let mut sc = Scanner::new(s);
    let mut out = [0; N];
    for (i, slot) in out.iter_mut().enumerate() {
        if i > 0 {
            sc.literal(',')?;
        }
        *slot = sc.int()?;
    }
    Some(out)
}

/// `swscanf(s, L"%u,%u,%06x,%06x,%u", ...)`, the format of a colour entry (`CDataProfile.cpp`,
/// `IOProfileData<ColorInfo>`).
pub fn scan_color(s: &str) -> Option<[u32; 5]> {
    let mut sc = Scanner::new(s);
    let a = sc.uint()?;
    sc.literal(',')?;
    let b = sc.uint()?;
    sc.literal(',')?;
    let c = sc.hex6()?;
    sc.literal(',')?;
    let d = sc.hex6()?;
    sc.literal(',')?;
    let e = sc.uint()?;
    Some([a, b, c, d, e])
}

/// Typed reading and writing (`CDataProfile::IOProfileData`).
impl Profile {
    /// An integer value. See [`parse_long`].
    pub fn get_i32(&self, section: &str, key: &str) -> Option<i32> {
        self.get(section, key).and_then(parse_long)
    }

    /// An unsigned integer value. See [`parse_ulong`].
    pub fn get_u32(&self, section: &str, key: &str) -> Option<u32> {
        self.get(section, key).and_then(parse_ulong)
    }

    /// A bool value: any integer that is not zero.
    pub fn get_bool(&self, section: &str, key: &str) -> Option<bool> {
        self.get(section, key).and_then(parse_bool)
    }

    /// A one-character value. See [`parse_wchar`].
    pub fn get_wchar(&self, section: &str, key: &str) -> Option<char> {
        self.get(section, key).and_then(parse_wchar)
    }

    /// A string value, as it is in the file.
    pub fn get_string(&self, section: &str, key: &str) -> Option<String> {
        self.get(section, key).map(str::to_owned)
    }

    /// Writes an integer. If the entry already holds the same number, it is left as written (for example
    /// `+4` or ` 4`), so that saving unchanged settings does not touch the user's file.
    pub fn set_i32(&mut self, section: &str, key: &str, value: i32) -> Result<(), ProfileError> {
        if self.get_i32(section, key) == Some(value) {
            return Ok(());
        }
        self.set(section, key, &value.to_string())
    }

    /// Writes an unsigned integer. Same as [`Profile::set_i32`] for an unchanged value.
    pub fn set_u32(&mut self, section: &str, key: &str, value: u32) -> Result<(), ProfileError> {
        if self.get_u32(section, key) == Some(value) {
            return Ok(());
        }
        self.set(section, key, &value.to_string())
    }

    /// Writes a bool as `1` or `0` (`ToString<bool>`). An entry that already means the same is left as written.
    pub fn set_bool(&mut self, section: &str, key: &str, value: bool) -> Result<(), ProfileError> {
        if self.get_bool(section, key) == Some(value) {
            return Ok(());
        }
        self.set(section, key, if value { "1" } else { "0" })
    }

    /// Writes a one-character value (`ToString<WCHAR>`).
    pub fn set_wchar(&mut self, section: &str, key: &str, value: char) -> Result<(), ProfileError> {
        if self.get_wchar(section, key) == Some(value) {
            return Ok(());
        }
        self.set(section, key, &format_wchar(value))
    }

    /// Writes a string value.
    pub fn set_string(
        &mut self,
        section: &str,
        key: &str,
        value: &str,
    ) -> Result<(), ProfileError> {
        self.set(section, key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_follows_wcstol() {
        assert_eq!(parse_long("12"), Some(12));
        assert_eq!(parse_long("  -7abc"), Some(-7));
        assert_eq!(parse_long("+5"), Some(5));
        assert_eq!(parse_long("0x10"), Some(0));
        assert_eq!(parse_long("99999999999"), Some(i32::MAX));
        assert_eq!(parse_long("-99999999999"), Some(i32::MIN));
        assert_eq!(parse_long(""), None);
        assert_eq!(parse_long("abc"), None);
        assert_eq!(parse_long("-"), None);
        assert_eq!(parse_long("1.5"), Some(1));
    }

    #[test]
    fn ulong_follows_wcstoul() {
        assert_eq!(parse_ulong("42"), Some(42));
        assert_eq!(parse_ulong("-1"), Some(u32::MAX));
        assert_eq!(parse_ulong("99999999999"), Some(u32::MAX));
        assert_eq!(parse_ulong("x"), None);
    }

    #[test]
    fn bool_is_nonzero() {
        assert_eq!(parse_bool("1"), Some(true));
        assert_eq!(parse_bool("0"), Some(false));
        assert_eq!(parse_bool("2"), Some(true));
        assert_eq!(parse_bool("-1"), Some(true));
        assert_eq!(parse_bool("true"), None);
        assert_eq!(parse_bool(""), None);
    }

    #[test]
    fn wchar_follows_profile_data() {
        assert_eq!(parse_wchar(""), Some('\0'));
        assert_eq!(parse_wchar(":"), Some(':'));
        assert_eq!(parse_wchar("ab"), Some('a'));
        assert_eq!(parse_wchar("あ"), Some('あ'));
        // 0xFF1A & 0xd800 == 0xd800: refused by Sakura as well.
        assert_eq!(parse_wchar("："), None);
        assert_eq!(parse_wchar("😀"), None);
        assert_eq!(format_wchar('\0'), "");
        assert_eq!(format_wchar(':'), ":");
        assert_eq!(format_wchar('😀'), "");
    }

    #[test]
    fn ints_need_all_numbers() {
        // ShareData_IO_TypeInts: `nInts` has 12 numbers.
        assert_eq!(scan_ints::<3>("1,2,3"), Some([1, 2, 3]));
        assert_eq!(scan_ints::<3>("1, 2, -3 tail"), Some([1, 2, -3]));
        assert_eq!(scan_ints::<3>("1,2"), None);
        assert_eq!(scan_ints::<3>("1 ,2,3"), None);
        assert_eq!(scan_ints::<2>("a,1"), None);
        assert_eq!(scan_ints::<1>(" 5"), Some([5]));
    }

    #[test]
    fn color_format() {
        assert_eq!(
            scan_color("1,0,ff8000,fffbf0,1"),
            Some([1, 0, 0xff8000, 0xfffbf0, 1])
        );
        assert_eq!(scan_color("1,0,0,0,0"), Some([1, 0, 0, 0, 0]));
        // %06x stops after 6 characters, so a longer number does not match the following comma.
        assert_eq!(scan_color("1,0,ff80000,0,0"), None);
        assert_eq!(scan_color("1,0,ff8000,fffbf0"), None);
    }

    #[test]
    fn typed_get_and_set() {
        let mut p = Profile::from_bytes(b"[S]\r\nn= 4\r\nb=1\r\nc=:\r\ns=abc\r\n");
        assert_eq!(p.get_i32("S", "n"), Some(4));
        assert_eq!(p.get_bool("S", "b"), Some(true));
        assert_eq!(p.get_wchar("S", "c"), Some(':'));
        assert_eq!(p.get_string("S", "s").as_deref(), Some("abc"));
        assert_eq!(p.get_i32("S", "missing"), None);
        assert_eq!(p.get_i32("S", "s"), None);
        // Same number: untouched.
        p.set_i32("S", "n", 4).unwrap();
        p.set_bool("S", "b", true).unwrap();
        assert_eq!(p.to_bytes(), b"[S]\r\nn= 4\r\nb=1\r\nc=:\r\ns=abc\r\n");
        p.set_i32("S", "n", 8).unwrap();
        p.set_bool("S", "b", false).unwrap();
        p.set_wchar("S", "c", '\0').unwrap();
        assert_eq!(p.to_bytes(), b"[S]\r\nn=8\r\nb=0\r\nc=\r\ns=abc\r\n");
    }
}
