// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/parse/CWordParse.cpp (url_table, url_char, IsURL, IsMailAddress)
//   sakura_core/view/colors/CColor_Url.cpp
//
// Original copyright notices of those files:
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

//! URLs and mail addresses (`URL`): how long the one is that starts at a position.
//!
//! The schemes are those of `url_table` in `CWordParse.cpp`, in lower case and followed by `://`
//! (`mailto:` and `news:` by a colon only). The characters of the rest are the ASCII ones of
//! `url_char`. A string that is not a URL may be a mail address.

/// One entry of `url_table`.
struct Scheme {
    name: &'static str,
    with_double_slash: bool,
}

const fn scheme(name: &'static str) -> Scheme {
    Scheme {
        name,
        with_double_slash: true,
    }
}

/// `url_table`, in alphabetical order (the search relies on equal first letters being next to
/// each other).
const SCHEMES: [Scheme; 13] = [
    scheme("file"),
    scheme("ftp"),
    scheme("gopher"),
    scheme("http"),
    scheme("https"),
    Scheme {
        name: "mailto",
        with_double_slash: false,
    },
    Scheme {
        name: "news",
        with_double_slash: false,
    },
    scheme("nntp"),
    scheme("prospero"),
    scheme("telnet"),
    scheme("tp"),
    scheme("ttp"),
    scheme("wais"),
];

impl Scheme {
    /// `length()`: the scheme and `://` (or `:`).
    fn len(&self) -> usize {
        self.name.len() + if self.with_double_slash { 3 } else { 1 }
    }

    /// `match()`: does `line` begin with this scheme and its separator?
    fn matches(&self, line: &[u8]) -> bool {
        if line.len() < self.len() || !line.starts_with(self.name.as_bytes()) {
            return false;
        }
        let rest = &line[self.name.len()..];
        if self.with_double_slash {
            rest.starts_with(b"://")
        } else {
            rest.starts_with(b":")
        }
    }
}

/// `url_char`: the ASCII characters that a URL can contain.
fn is_url_char(b: u8) -> bool {
    matches!(b,
        b'!' | b'#' | b'$' | b'%' | b'&' | b'+' | b',' | b'-' | b'.' | b'/'
        | b'0'..=b'9' | b':' | b';' | b'=' | b'?' | b'@'
        | b'A'..=b'Z' | b'\\' | b'_' | b'a'..=b'z' | b'~')
}

/// `IsURL`: the length of the URL (or mail address) at the start of `b`, which is the rest of
/// the line from the position, 0 if there is none. `before` is the byte before the position, for
/// the boundary of a mail address.
pub fn url_len(b: &[u8], before: Option<u8>) -> usize {
    let Some(&head) = b.first() else {
        return 0;
    };
    if !head.is_ascii() {
        return 0;
    }
    // Only the first letters of the schemes start a URL; the others may start a mail address.
    if matches!(head, b'f' | b'g' | b'h' | b'm' | b'n' | b'p' | b't' | b'w') {
        for s in SCHEMES.iter().filter(|s| s.name.as_bytes()[0] == head) {
            if !s.matches(b) {
                continue;
            }
            let schema_len = s.len();
            if s.name == "mailto" {
                // `IsMailAddress` on the rest, after the colon: the colon is not a mail character,
                // so the boundary check passes.
                return match mail_len(&b[schema_len..], Some(b':')) {
                    0 => 0,
                    n => n + schema_len,
                };
            }
            let mut i = schema_len;
            while i < b.len() && is_url_char(b[i]) {
                i += 1;
            }
            // only the header
            return if i == schema_len { 0 } else { i };
        }
    }
    mail_len(b, before)
}

/// The characters of the local part of a mail address: `0x21` to `0x7E` without `"(),:;<>@[\]`
/// (`IsValidChar` in `IsMailAddress`).
fn is_mail_char(b: u8) -> bool {
    (0x21..=0x7E).contains(&b) && !b"\"(),:;<>@[\\]".contains(&b)
}

/// `IsMailAddress`: the length of the mail address at the start of `b`, 0 if there is none.
pub fn mail_len(b: &[u8], before: Option<u8>) -> usize {
    // The boundary: a mail character right before is part of something else.
    if before.is_some_and(is_mail_char) {
        return 0;
    }
    let len = b.len();
    let at = |i: usize| b.get(i).copied().unwrap_or(0);
    let mut j = 0;
    if at(0) != b'.' && is_mail_char(at(0)) {
        j += 1;
    } else {
        return 0;
    }
    while j + 2 < len && is_mail_char(at(j)) {
        j += 1;
    }
    if j == 0 || j + 2 >= len {
        return 0;
    }
    if at(j) != b'@' {
        return 0;
    }
    j += 1;
    let mut dots = 0;
    loop {
        let begin = j;
        while j < len && (at(j).is_ascii_alphanumeric() || at(j) == b'-' || at(j) == b'_') {
            j += 1;
        }
        if j == begin {
            return 0;
        }
        if at(j) != b'.' {
            if dots == 0 {
                return 0;
            }
            break;
        }
        dots += 1;
        j += 1;
    }
    j
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> usize {
        url_len(s.as_bytes(), None)
    }

    #[test]
    fn urls_with_a_scheme() {
        assert_eq!(url("http://example.com/a?b=c#d rest"), 26);
        assert_eq!(url("https://example.com"), 19);
        assert_eq!(url("ftp://host/path)"), 15);
        assert_eq!(url("file:///C:/a.txt"), 16);
        assert_eq!(url("telnet://h:23/"), 14);
        assert_eq!(url("ttp://example.com"), 17);
    }

    #[test]
    fn a_scheme_alone_is_not_a_url() {
        assert_eq!(url("http://"), 0);
        assert_eq!(url("http:// x"), 0);
        assert_eq!(url("http:/x"), 0);
        // The schemes are lower case.
        assert_eq!(url("HTTP://example.com"), 0);
    }

    #[test]
    fn a_url_ends_at_a_character_it_cannot_contain() {
        assert_eq!(url("http://a.b/c\"d"), 12);
        assert_eq!(url("http://a.b/c(d)"), 12);
        assert_eq!(url("http://a.b/日本"), 11);
        assert_eq!(url("http://a.b/c<d>"), 12);
        assert_eq!(url("http://a.b/c\n"), 12);
    }

    #[test]
    fn the_url_at_the_end_of_a_line() {
        assert_eq!(url("http://a.b"), 10);
        assert_eq!(url("see http://a.b/c\r\n"), 0);
        assert_eq!(url("http://a.b/c\r\n"), 12);
    }

    #[test]
    fn mailto_takes_a_mail_address() {
        assert_eq!(url("mailto:a@b.cd rest"), 13);
        assert_eq!(url("mailto:nobody"), 0);
        assert_eq!(url("news:comp.lang.c x"), 16);
    }

    #[test]
    fn mail_addresses() {
        assert_eq!(url("a@b.cd rest"), 6);
        assert_eq!(url("first.last+tag@sub.example.co.jp>"), 32);
        assert_eq!(url("a@b"), 0); // needs a dot in the domain
        assert_eq!(url(".a@b.cd"), 0);
        assert_eq!(url("@b.cd x"), 0);
        assert_eq!(url("a@b.cd\n"), 6);
    }

    #[test]
    fn a_mail_address_needs_a_boundary_before() {
        assert_eq!(url_len(b"a@b.cd rest", Some(b' ')), 6);
        assert_eq!(url_len(b"a@b.cd rest", Some(b'x')), 0);
        assert_eq!(url_len(b"a@b.cd rest", Some(b'(')), 6);
    }
}
