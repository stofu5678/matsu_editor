// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/types/CType_Pascal.cpp (the array g_ppszKeywordsPASCAL)
//
// Original copyright notices of that file:
//   Copyright (C) 2008, kobake
//   Copyright (C) 2018-2022, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
//
// This file only has the words of the array, in the order of the original. The array became a Rust
// constant; nothing else was changed.
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

//! The built-in keyword set from `g_ppszKeywordsPASCAL` (`sakura_core/types/CType_Pascal.cpp`).

/// The words, in the order of the original.
pub const WORDS: &[&str] = &[
    "and",
    "exports",
    "mod",
    "shr",
    "array",
    "file",
    "nil",
    "string",
    "as",
    "finalization",
    "not",
    "stringresource",
    "asm",
    "finally",
    "object",
    "then",
    "begin",
    "for",
    "of",
    "case",
    "function",
    "or",
    "to",
    "class",
    "goto",
    "out",
    "try",
    "const",
    "if",
    "packed",
    "type",
    "constructor",
    "implementation",
    "procedure",
    "unit",
    "destructor",
    "in",
    "program",
    "until",
    "dispinterface",
    "inherited",
    "property",
    "uses",
    "div",
    "initialization",
    "raise",
    "var",
    "do",
    "inline",
    "record",
    "while",
    "downto",
    "interface",
    "repeat",
    "with",
    "else",
    "is",
    "resourcestring",
    "xor",
    "end",
    "label",
    "set",
    "except",
    "library",
    "shl",
    "private",
    "public",
    "published",
    "protected",
    "override",
];
