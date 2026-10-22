// Parts of this file are ported from Sakura Editor (zlib License) and were modified.
//
// Original files:
//   sakura_core/extmodule/CBregexp.cpp (CBregexp::MakePatternAlternate)
//
// Copyright (C) 2001-2002, genta
// Copyright (C) 2002, novice, hor, Azumaiya
// Copyright (C) 2003, かろと
// Copyright (C) 2005, かろと
// Copyright (C) 2006, かろと
// Copyright (C) 2007, ryoji
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
//
// Modifications: rewritten in Rust for MATSU Editor. The pattern is a UTF-8 `&str` instead of a
// NUL-terminated UTF-16 string, and the state table is a `match`.

//! Sakura's pre-processing of a search pattern.
//!
//! What Sakura does to a pattern before it reaches bregonig.dll (`CBregexp::Compile`):
//!
//! 1. `MakePatternAlternate` replaces every unescaped `.` outside of a character class and of
//!    `\Q...\E` with `[^\r\n]`, and every such `$` with `(?<![\r\n])(?=\r|$)`, because the
//!    library only knows LF as a line ending. [`make_pattern_alternate`] does the same, with the
//!    same state machine, so a pattern means exactly what it means in Sakura (including its
//!    quirks, for example that `(?#...)` comments and `x`-mode `#` comments are not skipped).
//! 2. `MakePatternSub` wraps the result as `m<D>pattern<D>km` plus `i` when case is ignored, with
//!    `D` = U+FFFF. [`Options`](crate::Options) already carries `m`, `i`, `x` and the others, so
//!    nothing is built here. But a pattern (or replacement) that ends with an odd number of
//!    backslashes escapes the delimiter `D` after it, so the pattern would have no end for
//!    bregonig.dll, which refuses it with "unmatch separater" (measured on bregonig.dll 4.20, see
//!    `tools/bregonig-compat`). [`ends_with_escape`] tells that. A `D` (U+FFFF) typed in a
//!    pattern ends it for the DLL; that is not reproduced.
//! 3. `CheckPattern` and the `MakePattern` family (`^`/`$` special cases) are only used by
//!    `MakePattern`, which `Compile` does not call any more, and by the fallback for a
//!    bregonig.dll without `BMatchEx`. They are not needed.
//!
//! The lines of a document are matched with their own line endings (ADR-0006), so these two
//! replacements matter: `.` must not take the CR or LF of the line ending, and `$` is before a
//! CR LF as well as before an LF. A pattern such as `(?<=$)` fails to compile in the same way as
//! in Sakura.

const DOT_ALTERNATIVE: &str = r"[^\r\n]";
const DOLLAR_ALTERNATIVE: &str = r"(?<![\r\n])(?=\r|$)";

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Outside of everything.
    Def,
    /// After a backslash.
    DefEscaped,
    /// After `\c`.
    DefControl,
    /// In a character class.
    Class,
    /// In a character class, after a backslash.
    ClassEscaped,
    /// In a character class, after `\c`.
    ClassControl,
    /// In `\Q...\E`.
    Quote,
    /// In `\Q...\E`, after a backslash.
    QuoteEscaped,
}

/// Replaces `.` and `$` as `CBregexp::MakePatternAlternate` does
/// (`sakura_core/extmodule/CBregexp.cpp`).
pub fn make_pattern_alternate(pattern: &str) -> String {
    use State::*;
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut state = Def;
    // Nesting of `[` inside a class (POSIX brackets such as `[[:alpha:]]` nest).
    let mut class_level = 0usize;
    for ch in pattern.chars() {
        // `None` copies the char.
        let mut replacement = None;
        state = match (state, ch) {
            (Def, '.') => {
                replacement = Some(DOT_ALTERNATIVE);
                Def
            }
            (Def, '$') => {
                replacement = Some(DOLLAR_ALTERNATIVE);
                Def
            }
            (Def, '[') => {
                class_level += 1;
                Class
            }
            (Def, '\\') => DefEscaped,
            (Def, _) => Def,

            (DefEscaped, 'c') => DefControl,
            (DefEscaped, 'Q') => Quote,
            (DefEscaped, _) => Def,

            (DefControl, '\\') => DefEscaped,
            (DefControl, _) => Def,

            (Class, '[') => {
                class_level += 1;
                Class
            }
            (Class, ']') => {
                class_level -= 1;
                if class_level > 0 { Class } else { Def }
            }
            (Class, '\\') => ClassEscaped,
            (Class, _) => Class,

            (ClassEscaped, 'c') => ClassControl,
            (ClassEscaped, _) => Class,

            (ClassControl, '\\') => ClassEscaped,
            (ClassControl, _) => Class,

            (Quote, '\\') => QuoteEscaped,
            (Quote, _) => Quote,

            (QuoteEscaped, 'E') => Def,
            (QuoteEscaped, '\\') => QuoteEscaped,
            (QuoteEscaped, _) => Quote,
        };
        match replacement {
            Some(r) => out.push_str(r),
            None => out.push(ch),
        }
    }
    out
}

/// Whether `text` ends with a backslash that is not itself escaped, that is, with an odd number of
/// backslashes. Sakura puts a delimiter (U+FFFF) right after the pattern and after the
/// replacement (`CBregexp::MakePatternSub`), and bregonig.dll 4.20 reads a backslash before the
/// delimiter as escaping it, so the text has no end: it reports "unmatch separater". Measured
/// with the DLL: `a\` is refused, `a\\` is fine, `a\\\` is refused.
pub fn ends_with_escape(text: &str) -> bool {
    let backslashes = text.chars().rev().take_while(|&c| c == '\\').count();
    backslashes % 2 == 1
}
