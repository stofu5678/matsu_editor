// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/types/CType_Perl.cpp (the array g_ppszKeywordsPERL2)
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

//! The built-in keyword set from `g_ppszKeywordsPERL2` (`sakura_core/types/CType_Perl.cpp`).

/// The words, in the order of the original.
pub const WORDS: &[&str] = &[
    "$ARGV",
    "$_",
    "$1",
    "$2",
    "$3",
    "$4",
    "$5",
    "$6",
    "$7",
    "$8",
    "$9",
    "$0",
    "$MATCH",
    "$&",
    "$PREMATCH",
    "$`",
    "$POSTMATCH",
    "$'",
    "$LAST_PAREN_MATCH",
    "$+",
    "$MULTILINE_MATCHING",
    "$*",
    "$INPUT_LINE_NUMBER",
    "$NR",
    "$.",
    "$INPUT_RECORD_SEPARATOR",
    "$RS",
    "$/",
    "$OUTPUT_AUTOFLUSH",
    "$|",
    "$OUTPUT_FIELD_SEPARATOR",
    "$OFS",
    "$,",
    "$OUTPUT_RECORD_SEPARATOR",
    "$ORS",
    "$\\",
    "$LIST_SEPARATOR",
    "$\"",
    "$SUBSCRIPT_SEPARATOR",
    "$SUBSEP",
    "$;",
    "$OFMT",
    "$#",
    "$FORMAT_PAGE_NUMBER",
    "$%",
    "$FORMAT_LINES_PER_PAGE",
    "$=",
    "$FORMAT_LINES_LEFT",
    "$-",
    "$FORMAT_NAME",
    "$~",
    "$FORMAT_TOP_NAME",
    "$^",
    "$FORMAT_LINE_BREAK_CHARACTERS",
    "$:",
    "$FORMAT_FORMFEED",
    "$^L",
    "$ACCUMULATOR",
    "$^A",
    "$CHILD_ERROR",
    "$?",
    "$OS_ERROR",
    "$ERRNO",
    "$!",
    "$EVAL_ERROR",
    "$@",
    "$PROCESS_ID",
    "$PID",
    "$$",
    "$REAL_USER_ID",
    "$UID",
    "$<",
    "$EFFECTIVE_USER_ID",
    "$EUID",
    "$>",
    "$REAL_GROUP_ID",
    "$GID",
    "$(",
    "$EFFECTIVE_GROUP_ID",
    "$EGID",
    "$)",
    "$PROGRAM_NAME",
    "$[",
    "$PERL_VERSION",
    "$]",
    "$DEBUGGING",
    "$^D",
    "$SYSTEM_FD_MAX",
    "$^F",
    "$INPLACE_EDIT",
    "$^I",
    "$PERLDB",
    "$^P",
    "$BASETIME",
    "$^T",
    "$WARNING",
    "$^W",
    "$EXECUTABLE_NAME",
    "$^X",
    "$ENV",
    "$SIG",
];
