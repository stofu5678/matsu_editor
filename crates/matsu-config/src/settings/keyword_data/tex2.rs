// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/types/CType_Tex.cpp (the array g_ppszKeywordsTEX2)
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

//! The built-in keyword set from `g_ppszKeywordsTEX2` (`sakura_core/types/CType_Tex.cpp`).

/// The words, in the order of the original.
pub const WORDS: &[&str] = &[
    "abstract",
    "array",
    "center",
    "description",
    "document",
    "displaymath",
    "em",
    "enumerate",
    "eqnarray",
    "eqnarray*",
    "equation",
    "figure",
    "figure*",
    "floatingfigure",
    "flushleft",
    "flushright",
    "itemize",
    "letter",
    "list",
    "math",
    "minipage",
    "multicols",
    "namelist",
    "picture",
    "quotation",
    "quote",
    "sloppypar",
    "subeqnarray",
    "subeqnarray*",
    "subequations",
    "subfigure",
    "tabbing",
    "table",
    "table*",
    "tabular",
    "tabular*",
    "tatepage",
    "thebibliography",
    "theindex",
    "titlepage",
    "trivlist",
    "verbatim",
    "verbatim*",
    "verse",
    "wrapfigure",
    "a4",
    "a4j",
    "a5",
    "a5j",
    "Alph",
    "alph",
    "annote",
    "arabic",
    "b4",
    "b4j",
    "b5",
    "b5j",
    "bezier",
    "booktitle",
    "boxedminipage",
    "boxit",
    "dbltopnumber",
    "eclepsf",
    "eepic",
    "enumi",
    "enumii",
    "enumiii",
    "enumiv",
    "epic",
    "epsbox",
    "epsf",
    "fancybox",
    "fancyheadings",
    "fleqn",
    "footnote",
    "howpublished",
    "jabbrv",
    "jalpha",
    "jarticle",
    "jsarticle",
    "jbook",
    "jsbook",
    "jletter",
    "jplain",
    "jreport",
    "jtwocolumn",
    "junsrt",
    "leqno",
    "makeidx",
    "markboth",
    "markright",
    "multicol",
    "myheadings",
    "openbib",
    "secnumdepth",
    "titlepage",
    "tjarticle",
    "topnumber",
    "totalnumber",
    "twocolumn",
    "twoside",
    "yomi",
];
