// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original file:
//   sakura_core/types/CType_Vb.cpp (the array g_ppszKeywordsVB)
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

//! The built-in keyword set from `g_ppszKeywordsVB` (`sakura_core/types/CType_Vb.cpp`).

/// The words, in the order of the original.
pub const WORDS: &[&str] = &[
    "And",
    "As",
    "Attribute",
    "Begin",
    "BeginProperty",
    "Boolean",
    "ByVal",
    "Byte",
    "Call",
    "Case",
    "Const",
    "Currency",
    "Date",
    "Declare",
    "Dim",
    "Do",
    "Double",
    "Each",
    "Else",
    "ElseIf",
    "Empty",
    "End",
    "EndProperty",
    "Error",
    "Eqv",
    "Exit",
    "False",
    "For",
    "Friend",
    "Function",
    "Get",
    "GoTo",
    "If",
    "Imp",
    "Integer",
    "Is",
    "Let",
    "Like",
    "Long",
    "Loop",
    "Me",
    "Mod",
    "New",
    "Next",
    "Not",
    "Null",
    "Object",
    "On",
    "Option",
    "Or",
    "Private",
    "Property",
    "Public",
    "RSet",
    "ReDim",
    "Rem",
    "Resume",
    "Select",
    "Set",
    "Single",
    "Static",
    "Step",
    "Stop",
    "String",
    "Sub",
    "Then",
    "To",
    "True",
    "Type",
    "Wend",
    "While",
    "With",
    "Xor",
    "#If",
    "#Else",
    "#End",
    "#Const",
    "AddressOf",
    "Alias",
    "Append",
    "Array",
    "ByRef",
    "Explicit",
    "Global",
    "In",
    "Lib",
    "Nothing",
    "Optional",
    "Output",
    "Terminate",
    "Until",
    "DefBool",
    "DefByte",
    "DefCur",
    "DefDate",
    "DefDbl",
    "DefInt",
    "DefLng",
    "DefObj",
    "DefSng",
    "DefStr",
    "DefVar",
    "LSet",
    "GoSub",
    "Return",
    "Variant",
];
