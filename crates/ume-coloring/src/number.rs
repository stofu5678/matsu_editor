// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/view/colors/CColor_Numeric.cpp (IsNumber)
//
// Original copyright notices of those files:
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

//! Half-width numbers (`NUM`): how long the number is that starts at a position.
//!
//! ```text
//! 123      123            0x567.8  0x567 , then 8
//! 0123     0123           0.56.1   0.56.1
//! 0xfedc   0xfedc         .5       .5
//! -123     -123           123.     123
//! -.5      -.5
//! 1e2  1.2e+3  1.2e-3  1L  1F  1.2f  0x1L  0x2F
//! ```
//! The table is what the code of `IsNumber` in `CColor_Numeric.cpp` does. Its comment lists
//! `0.56.1` as two numbers and `.5` as `5`, which the compiled code does not do.

/// `IsNumber`: the length in bytes of the number at the start of `b`, 0 if there is none. `b` is
/// the rest of the line from the position, with the line ending.
pub fn number_len(b: &[u8]) -> usize {
    let q = b.len();
    let Some(&first) = b.first() else {
        return 0;
    };
    let mut p = 0;
    let mut i = 0;
    let mut d = 0;
    let mut f = false;

    if first == b'0' {
        // decimal and C hexadecimal
        p += 1;
        i += 1;
        if p < q && b[p] == b'x' {
            p += 1;
            i += 1;
            while p < q && b[p].is_ascii_hexdigit() {
                p += 1;
                i += 1;
            }
            // "0x" alone: "0" is the number
            if i == 2 {
                return 1;
            }
            if p < q && matches!(b[p], b'L' | b'l' | b'F' | b'f') {
                i += 1;
            }
            return i;
        } else if p < q && b[p].is_ascii_digit() {
            p += 1;
            i += 1;
            decimal_loop(b, &mut p, &mut i, &mut d, &mut f);
            if b[p - 1] == b'.' {
                return i - 1;
            }
            return i + suffix(b, p, d, true);
        } else if p < q && b[p] == b'.' {
            decimal_loop(b, &mut p, &mut i, &mut d, &mut f);
            if b[p - 1] == b'.' {
                return i - 1;
            }
            // after "0." only F and f
            return i + suffix(b, p, d, false);
        } else if p < q && (b[p] == b'E' || b[p] == b'e') {
            p += 1;
            i += 1;
            while p < q {
                if !b[p].is_ascii_digit() {
                    if (b[p] == b'+' || b[p] == b'-') && (b[p - 1] == b'E' || b[p - 1] == b'e') {
                        // "0E+" and "0E-" without a digit are not an exponent
                        if p + 1 >= q || !b[p + 1].is_ascii_digit() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                p += 1;
                i += 1;
            }
            // "0E" and "0e": "0" is the number
            if i == 2 {
                return 1;
            }
            return i + suffix(b, p, d, true);
        }
        // "0" alone
        return i + suffix(b, p, d, true);
    }

    if (b'1'..=b'9').contains(&first) {
        p += 1;
        i += 1;
        decimal_loop(b, &mut p, &mut i, &mut d, &mut f);
        if b[p - 1] == b'.' {
            return i - 1;
        }
        return i + suffix(b, p, d, true);
    }

    if first == b'-' {
        p += 1;
        i += 1;
        decimal_loop(b, &mut p, &mut i, &mut d, &mut f);
        // "-" alone is not a number
        if i == 1 {
            return 0;
        }
        if b[p - 1] == b'.' {
            i -= 1;
            // "-." is not a number
            if i == 1 {
                return 0;
            }
            return i;
        }
        return i + suffix(b, p, d, true);
    }

    if first == b'.' {
        d += 1;
        p += 1;
        i += 1;
        decimal_loop(b, &mut p, &mut i, &mut d, &mut f);
        // "." alone is not a number
        if i == 1 {
            return 0;
        }
        if b[p - 1] == b'.' {
            return i - 1;
        }
        return i + suffix(b, p, d, false);
    }

    0
}

/// The loop that every decimal form shares: digits, dots (not two in a row, not after the
/// exponent) and an exponent `e`, `e+`, `e-` with a digit after it.
fn decimal_loop(b: &[u8], p: &mut usize, i: &mut usize, d: &mut u32, f: &mut bool) {
    let q = b.len();
    let digit_at = |k: usize| b.get(k).is_some_and(u8::is_ascii_digit);
    while *p < q {
        if !b[*p].is_ascii_digit() {
            if b[*p] == b'.' {
                if *f {
                    break; // inside the exponent
                }
                *d += 1;
                if *d > 1 && b[*p - 1] == b'.' {
                    break; // two dots in a row
                }
            } else if b[*p] == b'E' || b[*p] == b'e' {
                if *f {
                    break;
                }
                if *p + 2 < q {
                    if (b[*p + 1] == b'+' || b[*p + 1] == b'-') && digit_at(*p + 2) {
                        *p += 2;
                        *i += 2;
                        *f = true;
                    } else if digit_at(*p + 1) {
                        *p += 1;
                        *i += 1;
                        *f = true;
                    } else {
                        break;
                    }
                } else if *p + 1 < q {
                    if digit_at(*p + 1) {
                        *p += 1;
                        *i += 1;
                        *f = true;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        *p += 1;
        *i += 1;
    }
}

/// The suffix after a number: `L` and `l` when there was no dot (`allow_long`), and `F` and `f`.
/// Returns how many bytes it takes (0 or 1).
fn suffix(b: &[u8], p: usize, dots: u32, allow_long: bool) -> usize {
    match b.get(p) {
        Some(b'L' | b'l') if allow_long && dots == 0 => 1,
        Some(b'F' | b'f') => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::number_len;

    fn len(s: &str) -> usize {
        number_len(s.as_bytes())
    }

    #[test]
    fn integers() {
        assert_eq!(len("123"), 3);
        assert_eq!(len("0123"), 4);
        assert_eq!(len("0"), 1);
        assert_eq!(len("42;"), 2);
        assert_eq!(len("1_000"), 1); // the underscore ends it, as in Sakura
        assert_eq!(len("abc"), 0);
        assert_eq!(len(""), 0);
    }

    #[test]
    fn hexadecimal() {
        assert_eq!(len("0x1F"), 4);
        assert_eq!(len("0xfedc"), 6);
        assert_eq!(len("0x"), 1);
        assert_eq!(len("0xg"), 1);
        assert_eq!(len("0x1FL"), 5);
        assert_eq!(len("0x2F;"), 4);
        assert_eq!(len("0x567.8"), 5);
    }

    #[test]
    fn fractions_and_exponents() {
        assert_eq!(len("0.5"), 3);
        assert_eq!(len("1.5e3"), 5);
        assert_eq!(len("1.2e+3"), 6);
        assert_eq!(len("1.2e-3"), 6);
        assert_eq!(len("1e2"), 3);
        assert_eq!(len("123."), 3);
        assert_eq!(len("1.2.3"), 5);
        assert_eq!(len("1..2"), 1);
        assert_eq!(len("0.56.1"), 6);
        assert_eq!(len("1e"), 1);
        assert_eq!(len("1e+"), 1);
        assert_eq!(len("1ex"), 1);
        assert_eq!(len("0e5"), 3);
        assert_eq!(len("0E"), 1);
        assert_eq!(len("0E+"), 1);
        assert_eq!(len("0E+5"), 4);
    }

    #[test]
    fn signs_and_leading_dots() {
        assert_eq!(len("-123"), 4);
        assert_eq!(len("-.5"), 3);
        assert_eq!(len("-"), 0);
        assert_eq!(len("-."), 0);
        assert_eq!(len("-x"), 0);
        assert_eq!(len(".5"), 2);
        assert_eq!(len("."), 0);
        assert_eq!(len(".x"), 0);
        assert_eq!(len("-1e0"), 4);
    }

    #[test]
    fn suffixes() {
        assert_eq!(len("1L"), 2);
        assert_eq!(len("1l"), 2);
        assert_eq!(len("1F"), 2);
        assert_eq!(len("1.2f"), 4);
        // L only counts when there was no dot
        assert_eq!(len("1.2L"), 3);
        assert_eq!(len("0L"), 2);
        assert_eq!(len("0.5f"), 4);
        assert_eq!(len("0.5L"), 3);
        assert_eq!(len(".5f"), 3);
        assert_eq!(len("-1L"), 3);
    }
}
