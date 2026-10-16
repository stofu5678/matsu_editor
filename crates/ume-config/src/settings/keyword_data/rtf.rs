// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/types/CType_Rich.cpp (the array g_ppszKeywordsRTF)
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

//! The built-in keyword set from `g_ppszKeywordsRTF` (`sakura_core/types/CType_Rich.cpp`).

/// The words, in the order of the original.
pub const WORDS: &[&str] = &[
    "\\ansi",
    "\\b",
    "\\bin",
    "\\box",
    "\\brdrb",
    "\\brdrbar",
    "\\brdrdb",
    "\\brdrdot",
    "\\brdrl",
    "\\brdrr",
    "\\brdrs",
    "\\brdrsh",
    "\\brdrt",
    "\\brdrth",
    "\\cell",
    "\\cellx",
    "\\cf",
    "\\chftn",
    "\\clmgf",
    "\\clmrg",
    "\\colortbl",
    "\\deff",
    "\\f",
    "\\fi",
    "\\field",
    "\\fldrslt",
    "\\fonttbl",
    "\\footnote",
    "\\fs",
    "\\i",
    "\\intbl",
    "\\keep",
    "\\keepn",
    "\\li",
    "\\line",
    "\\mac",
    "\\page",
    "\\par",
    "\\pard",
    "\\pc",
    "\\pich",
    "\\pichgoal",
    "\\picscalex",
    "\\picscaley",
    "\\pict",
    "\\picw",
    "\\picwgoal",
    "\\plain",
    "\\qc",
    "\\ql",
    "\\qr",
    "\\ri",
    "\\row",
    "\\rtf",
    "\\sa",
    "\\sb",
    "\\scaps",
    "\\sect",
    "\\sl",
    "\\strike",
    "\\tab",
    "\\tqc",
    "\\tqr",
    "\\trgaph",
    "\\trleft",
    "\\trowd",
    "\\trqc",
    "\\trql",
    "\\tx",
    "\\ul",
    "\\uldb",
    "\\v",
    "\\wbitmap",
    "\\wbmbitspixel",
    "\\wbmplanes",
    "\\wbmwidthbytes",
    "\\wmetafile",
    "bmc",
    "bml",
    "bmr",
    "emc",
    "eml",
    "emr",
];
