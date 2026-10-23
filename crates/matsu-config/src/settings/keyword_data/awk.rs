// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/types/CType_Awk.cpp (the array g_ppszKeywordsAWK)
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

//! The built-in keyword set from `g_ppszKeywordsAWK` (`sakura_core/types/CType_Awk.cpp`).

/// The words, in the order of the original.
pub const WORDS: &[&str] = &[
    "BEGIN",
    "END",
    "next",
    "exit",
    "func",
    "function",
    "return",
    "if",
    "else",
    "for",
    "in",
    "do",
    "while",
    "break",
    "continue",
    "$0",
    "$1",
    "$2",
    "$3",
    "$4",
    "$5",
    "$6",
    "$7",
    "$8",
    "$9",
    "$10",
    "$11",
    "$12",
    "$13",
    "$14",
    "$15",
    "$16",
    "$17",
    "$18",
    "$19",
    "$20",
    "FS",
    "OFS",
    "NF",
    "RS",
    "ORS",
    "NR",
    "FNR",
    "ARGV",
    "ARGC",
    "ARGIND",
    "FILENAME",
    "ENVIRON",
    "ERRNO",
    "OFMT",
    "CONVFMT",
    "FIELDWIDTHS",
    "IGNORECASE",
    "RLENGTH",
    "RSTART",
    "SUBSEP",
    "delete",
    "index",
    "jindex",
    "length",
    "jlength",
    "substr",
    "jsubstr",
    "match",
    "split",
    "sub",
    "gsub",
    "sprintf",
    "tolower",
    "toupper",
    "print",
    "printf",
    "getline",
    "system",
    "close",
    "sin",
    "cos",
    "atan2",
    "exp",
    "log",
    "int",
    "sqrt",
    "srand",
    "rand",
    "strftime",
    "systime",
];
