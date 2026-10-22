// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/outline/CFuncInfo.cpp, sakura_core/outline/CFuncInfoArr.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2001, Norio Nakatani
//   Copyright (C) 2000-2014, the authors named in the original files (genta, zenryaku, Moca,
//     aroka, hor, YAZAKI, little YOSHI, ryoji, syat and others)
//   Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a UTF-16 buffer
// walked with `CDocLineMgr`, the results put in `CFuncInfoArr`, a Windows dialog) was rewritten in
// Rust on the lines of `LineSource`, the positions in the layout are left to the window, and the
// tests are new.
//
// zlib License
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

//! The results of an analysis: Sakura's `CFuncInfo` and `CFuncInfoArr`.
//!
//! Sakura: `sakura_core/outline/CFuncInfo.cpp`, `sakura_core/outline/CFuncInfoArr.cpp`.

use std::collections::BTreeMap;

/// `FUNCINFO_NOCLIPTEXT`: a dummy item that is not put in the text that the copy button makes.
pub const NO_CLIP_TEXT: u32 = 0x10000;

/// `FL_OBJ_*` (`sakura_core/outline/CFuncInfoArr.h`): what `FuncInfo::info` says of an item in the
/// trees of C/C++ and Java.
pub mod func_obj {
    /// The place where a parent class is defined.
    pub const DEFINITION: u32 = 0;
    /// A declaration (a function prototype).
    pub const DECLARE: u32 = 1;
    pub const FUNCTION: u32 = 2;
    pub const CLASS: u32 = 3;
    pub const STRUCT: u32 = 4;
    pub const ENUM: u32 = 5;
    pub const UNION: u32 = 6;
    pub const NAMESPACE: u32 = 7;
    pub const INTERFACE: u32 = 8;
    /// "Global": used by the tree only.
    pub const GLOBAL: u32 = 9;
    /// The largest value that a plug-in may add.
    pub const ELEMENT_MAX: u32 = 30;
}

/// One item of an outline (`CFuncInfo`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FuncInfo {
    /// The line in the document, counted from 1 (`m_nFuncLineCRLF`).
    pub line: usize,
    /// The column, counted from 1 (`m_nFuncColCRLF`). Sakura's analyzers all give 1.
    pub col: usize,
    /// The name that the list shows (`m_cmemFuncName`).
    pub name: String,
    /// The depth in a tree (`m_nDepth`).
    pub depth: usize,
    /// Extra information (`m_nInfo`): [`func_obj`] for C/C++ and Java, the flags of a procedure
    /// for Visual Basic, the kind of the heading elsewhere. [`NO_CLIP_TEXT`] may be set.
    pub info: u32,
}

impl FuncInfo {
    /// `CFuncInfo::IsAddClipText`.
    pub fn is_added_to_clip_text(&self) -> bool {
        self.info & NO_CLIP_TEXT == 0
    }

    /// `m_nInfo & FUNCINFO_INFOMASK`.
    pub fn info_value(&self) -> u32 {
        self.info & 0xFFFF
    }
}

/// The items of an analysis in the order they were found (`CFuncInfoArr`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FuncInfoArr {
    items: Vec<FuncInfo>,
    /// `m_AppendTextArr`: what the tree adds after the name of an item of a given `info`.
    append: BTreeMap<u32, String>,
}

impl FuncInfoArr {
    pub fn new() -> Self {
        Self::default()
    }

    /// `AppendData` of the short form: the column is 1.
    pub fn push(&mut self, line: usize, name: impl Into<String>, info: u32, depth: usize) {
        self.items.push(FuncInfo {
            line,
            col: 1,
            name: name.into(),
            depth,
            info,
        });
    }

    pub fn items(&self) -> &[FuncInfo] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `CFuncInfoArr::GetAt`.
    pub fn get(&self, index: usize) -> Option<&FuncInfo> {
        self.items.get(index)
    }

    /// `SetAppendText`: sets the text added after the items of `info`; with `overwrite` false an
    /// existing text stays.
    pub fn set_append_text(&mut self, info: u32, text: impl Into<String>, overwrite: bool) {
        if overwrite || !self.append.contains_key(&info) {
            self.append.insert(info, text.into());
        }
    }

    /// `GetAppendText`: empty when there is none.
    pub fn append_text(&self, info: u32) -> &str {
        self.append.get(&info).map_or("", String::as_str)
    }
}
