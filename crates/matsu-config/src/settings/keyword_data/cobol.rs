// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/types/CType_Cobol.cpp (the array g_ppszKeywordsCOBOL)
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

//! The built-in keyword set from `g_ppszKeywordsCOBOL` (`sakura_core/types/CType_Cobol.cpp`).

/// The words, in the order of the original.
pub const WORDS: &[&str] = &[
    "ACCEPT",
    "ADD",
    "ADVANCING",
    "AFTER",
    "ALL",
    "AND",
    "ARGUMENT",
    "ASSIGN",
    "AUTHOR",
    "BEFORE",
    "BLOCK",
    "BY",
    "CALL",
    "CHARACTERS",
    "CLOSE",
    "COMP",
    "COMPILED",
    "COMPUTE",
    "COMPUTER",
    "CONFIGURATION",
    "CONSOLE",
    "CONTAINS",
    "CONTINUE",
    "CONTROL",
    "COPY",
    "DATA",
    "DELETE",
    "DISPLAY",
    "DIVIDE",
    "DIVISION",
    "ELSE",
    "END",
    "ENVIRONMENT",
    "EVALUATE",
    "EXAMINE",
    "EXIT",
    "EXTERNAL",
    "FD",
    "FILE",
    "FILLER",
    "FROM",
    "GIVING",
    "GO",
    "GOBACK",
    "HIGH-VALUE",
    "IDENTIFICATION",
    "IF",
    "INITIALIZE",
    "INPUT",
    "INTO",
    "IS",
    "LABEL",
    "LINKAGE",
    "LOW-VALUE",
    "MODE",
    "MOVE",
    "NOT",
    "OBJECT",
    "OCCURS",
    "OF",
    "ON",
    "OPEN",
    "OR",
    "OTHER",
    "OUTPUT",
    "PERFORM",
    "PIC",
    "PROCEDURE",
    "PROGRAM",
    "READ",
    "RECORD",
    "RECORDING",
    "REDEFINES",
    "REMAINDER",
    "REMARKS",
    "REPLACING",
    "REWRITE",
    "ROLLBACK",
    "SECTION",
    "SELECT",
    "SOURCE",
    "SPACE",
    "STANDARD",
    "STOP",
    "STORAGE",
    "SYSOUT",
    "TEST",
    "THEN",
    "TO",
    "TODAY",
    "TRANSFORM",
    "UNTIL",
    "UPON",
    "USING",
    "VALUE",
    "VARYING",
    "WHEN",
    "WITH",
    "WORKING",
    "WRITE",
    "WRITTEN",
    "ZERO",
];
