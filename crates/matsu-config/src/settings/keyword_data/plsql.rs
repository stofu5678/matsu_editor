// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/types/CType_Sql.cpp (the array g_ppszKeywordsPLSQL)
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

//! The built-in keyword set from `g_ppszKeywordsPLSQL` (`sakura_core/types/CType_Sql.cpp`).

/// The words, in the order of the original.
pub const WORDS: &[&str] = &[
    "AND",
    "AS",
    "BEGIN",
    "BINARY_INTEGER",
    "BODY",
    "BOOLEAN",
    "BY",
    "CHAR",
    "CHR",
    "COMMIT",
    "COUNT",
    "CREATE",
    "CURSOR",
    "DATE",
    "DECLARE",
    "DEFAULT",
    "DELETE",
    "ELSE",
    "ELSIF",
    "END",
    "ERRORS",
    "EXCEPTION",
    "FALSE",
    "FOR",
    "FROM",
    "FUNCTION",
    "GOTO",
    "HTP",
    "IDENT_ARR",
    "IF",
    "IN",
    "INDEX",
    "INTEGER",
    "IS",
    "LOOP",
    "NOT",
    "NO_DATA_FOUND",
    "NULL",
    "NUMBER",
    "OF",
    "OR",
    "ORDER",
    "OUT",
    "OWA_UTIL",
    "PACKAGE",
    "PRAGMA",
    "PRN",
    "PROCEDURE",
    "REPLACE",
    "RESTRICT_REFERENCES",
    "RETURN",
    "ROWTYPE",
    "SELECT",
    "SHOW",
    "SUBSTR",
    "TABLE",
    "THEN",
    "TRUE",
    "TYPE",
    "UPDATE",
    "VARCHAR",
    "VARCHAR2",
    "WHEN",
    "WHERE",
    "WHILE",
    "WNDS",
    "WNPS",
    "RAISE",
    "INSERT",
    "INTO",
    "VALUES",
    "SET",
    "SYSDATE",
    "RTRIM",
    "LTRIM",
    "TO_CHAR",
    "DUP_VAL_ON_INDEX",
    "ROLLBACK",
    "OTHERS",
    "SQLCODE",
];
