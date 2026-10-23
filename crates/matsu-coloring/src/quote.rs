// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/view/colors/CColor_Quote.cpp, sakura_core/view/colors/CColor_Quote.h
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2022, Sakura Editor Organization
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

//! Where a string in quotes ends (`CColor_Quote::Match_Quote` and `Match_QuoteStr`).
//!
//! `line` is the line with its line ending; `text_len` is where the line ending starts. A match
//! gives the position just after the closing quote. When there is no closing quote on the line
//! `Match_Quote` gives the length of the line plus one and `Match_QuoteStr` the length itself, and
//! both can never be reached by the scan, which stops before the end of the line.

use matsu_config::settings::type_rules::string_type;

/// How a quote is escaped inside a string (`m_nEscapeType`, one of `string_type`'s values that
/// the coloring distinguishes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escape {
    /// `\"`: a backslash escapes the next character, and one at the end of the line goes on to the
    /// next line.
    Backslash,
    /// `""`: a doubled quote is a quote.
    Doubled,
    /// No escape.
    None,
}

/// `Update` of `CColor_Quote`: the escape rule for a string type. C# and Python escape with a
/// backslash like C; only C, PL/SQL and HTML are plain `Match_Quote` rules.
pub fn escape_of(string_type_value: i32) -> Escape {
    match string_type_value {
        string_type::PLSQL => Escape::Doubled,
        string_type::HTML => Escape::None,
        _ => Escape::Backslash,
    }
}

/// `pbEscapeEnd` is set for the types that can go on to the next line after a backslash
/// (`pbEscapeEndList`): C and Python.
pub fn tracks_escaped_line_end(string_type_value: i32) -> bool {
    matches!(string_type_value, string_type::CPP | string_type::PYTHON)
}

/// `Match_Quote`: the end of the string in `quote` whose contents start at `from`. `escaped_end`
/// is set when a backslash is the last character before the line ending.
pub fn match_quote(
    line: &str,
    text_len: usize,
    quote: char,
    from: usize,
    escape: Escape,
    escaped_end: Option<&mut bool>,
) -> usize {
    let mut escaped_end = escaped_end;
    let mut chars = line.get(from..).unwrap_or("").char_indices();
    while let Some((offset, c)) = chars.next() {
        let i = from + offset;
        match escape {
            Escape::Backslash => {
                if c == '\\' {
                    // The next character is escaped. A line ending there goes on to the next line.
                    if let Some((next, _)) = chars.next()
                        && from + next >= text_len
                        && let Some(flag) = escaped_end.as_deref_mut()
                    {
                        *flag = true;
                    }
                } else if c == quote {
                    return i + c.len_utf8();
                }
            }
            Escape::Doubled => {
                if c == quote {
                    if line[i + c.len_utf8()..].starts_with(quote) {
                        chars.next();
                    } else {
                        return i + c.len_utf8();
                    }
                }
            }
            Escape::None => {
                if c == quote {
                    return i + c.len_utf8();
                }
            }
        }
    }
    line.len() + 1
}

/// `Match_QuoteStr`: the end of a string that is closed by `quote` (three quotes, or the tag of a
/// raw string), searched from `from`. With `escape`, a backslash skips the next character.
pub fn match_quote_str(line: &str, quote: &str, from: usize, escape: bool) -> usize {
    let bytes = line.as_bytes();
    let Some(last_start) = (bytes.len() + 1).checked_sub(quote.len()) else {
        return line.len();
    };
    let mut i = from;
    while i < last_start {
        if bytes[i..].starts_with(quote.as_bytes()) {
            return i + quote.len();
        }
        let step = char_step(line, i);
        if escape && bytes[i] == b'\\' {
            i += step.max(1);
            i += char_step(line, i).max(1);
        } else {
            i += step.max(1);
        }
    }
    line.len()
}

fn char_step(line: &str, i: usize) -> usize {
    crate::charkind::char_len_at(line, i)
}

/// `CColor_Quote::IsCppRawString`: the quote at `pos` is the quote of a raw string `R"(`,
/// `u8R"(`, `uR"(`, `UR"(` or `LR"(`, that starts at a word boundary.
pub fn is_cpp_raw_string(line: &str, pos: usize) -> bool {
    let prev: Vec<char> = line[..pos].chars().rev().take(4).collect();
    if pos == 0
        || prev.first() != Some(&'R')
        || !line[pos..].starts_with('"')
        || pos + 1 >= line.len()
    {
        return false;
    }
    const SEPARATORS: &str = " \t!\"#$%&'()=@{};:<>?,.*/-+[]";
    // A missing character counts as a space; `wcschr` finds the terminating NUL of the list too,
    // so a NUL (a missing fourth character) counts as a separator.
    let at = |back: usize, missing: char| prev.get(back).copied().unwrap_or(missing);
    let is_sep = |c: char| c == '\0' || SEPARATORS.contains(c);
    let c1 = at(1, ' ');
    let c2 = at(2, ' ');
    if matches!(c1, 'u' | 'U' | 'L') {
        is_sep(c2)
    } else if c1 == '8' && c2 == 'u' {
        is_sep(at(3, '\0'))
    } else {
        is_sep(c1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mq(line: &str, from: usize, esc: Escape) -> usize {
        let text_len = line.trim_end_matches(['\r', '\n']).len();
        match_quote(line, text_len, '"', from, esc, None)
    }

    #[test]
    fn a_plain_string_ends_at_the_next_quote() {
        assert_eq!(mq("\"abc\" x\n", 1, Escape::Backslash), 5);
        assert_eq!(mq("\"\"\n", 1, Escape::Backslash), 2);
    }

    #[test]
    fn a_backslash_escapes_the_quote() {
        assert_eq!(mq("\"a\\\"b\" x\n", 1, Escape::Backslash), 6);
        assert_eq!(mq("\"a\\\\\" x\n", 1, Escape::Backslash), 5);
        // PL/SQL and HTML do not escape with a backslash.
        assert_eq!(mq("\"a\\\" x\n", 1, Escape::Doubled), 4);
        assert_eq!(mq("\"a\\\" x\n", 1, Escape::None), 4);
    }

    #[test]
    fn a_doubled_quote_is_a_quote_in_plsql() {
        assert_eq!(mq("\"a\"\"b\" x\n", 1, Escape::Doubled), 6);
        assert_eq!(mq("\"a\"\"b\" x\n", 1, Escape::None), 3);
    }

    #[test]
    fn no_closing_quote_gives_the_length_plus_one() {
        assert_eq!(mq("\"abc\n", 1, Escape::Backslash), 6);
        assert_eq!(mq("\"abc", 1, Escape::Backslash), 5);
    }

    #[test]
    fn a_backslash_before_the_line_ending_is_noted() {
        let line = "\"abc\\\r\n";
        let mut escaped = false;
        let text_len = 5;
        assert_eq!(
            match_quote(
                line,
                text_len,
                '"',
                1,
                Escape::Backslash,
                Some(&mut escaped)
            ),
            line.len() + 1
        );
        assert!(escaped);
        let mut escaped = false;
        match_quote(
            "\"abc\\\\\n",
            6,
            '"',
            1,
            Escape::Backslash,
            Some(&mut escaped),
        );
        assert!(!escaped);
    }

    #[test]
    fn japanese_text_in_a_string() {
        assert_eq!(mq("\"日本語\" x\n", 1, Escape::Backslash), 11);
        assert_eq!(mq("\"日\\本\" x\n", 1, Escape::Backslash), 9);
    }

    #[test]
    fn quote_strings_for_python_and_raw_strings() {
        assert_eq!(match_quote_str("'''abc''' x\n", "'''", 3, true), 9);
        assert_eq!(match_quote_str("'''a\\'''b''' x\n", "'''", 3, true), 12);
        assert_eq!(match_quote_str("'''abc\n", "'''", 3, true), 7);
        assert_eq!(match_quote_str("R\"x(a)\" b)x\" c\n", ")x\"", 4, false), 12);
        assert_eq!(match_quote_str("ab\n", "'''", 0, true), 3);
    }

    #[test]
    fn raw_string_prefixes() {
        let raw = |line: &str| {
            let pos = line.find('"').unwrap();
            is_cpp_raw_string(line, pos)
        };
        assert!(raw("R\"(x)\""));
        assert!(raw("x = R\"(x)\""));
        assert!(raw("x = u8R\"(x)\""));
        assert!(raw("u8R\"(x)\""));
        assert!(raw("x = LR\"(x)\""));
        assert!(raw("f(UR\"(x)\")"));
        assert!(!raw("xR\"(x)\""));
        assert!(!raw("\"(x)\""));
        assert!(!raw("abc u9R\"(x)\""));
        assert!(!raw("R\""));
    }
}
