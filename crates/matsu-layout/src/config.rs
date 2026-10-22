// Derived from Sakura Editor (zlib License):
//   sakura_core/doc/layout/CLayoutMgr.h (GetActualTsvSpace, GetLayoutXOfChar),
//   sakura_core/doc/CEditDoc.cpp (OnChangeSetting: the width when lines are not wrapped),
//   sakura_core/config/app_constants.h (MAXLINEKETAS, MINLINEKETAS)
// Modified: rewritten in Rust, measured in columns instead of pixels, without TSV/CSV mode.
//
// Copyright (C) 2018-2022, Sakura Editor Organization
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

use crate::kinsoku::{CharSet, Kinsoku};
use crate::width::WidthRule;

/// The most columns Sakura lets a line have (`MAXLINEKETAS`, `config/app_constants.h`). It is
/// also the width when lines are "not wrapped" (`CEditDoc::OnChangeSetting`: "otherwise
/// MAXLINEKETAS").
pub const MAX_LINE_COLUMNS: u32 = 10240;

/// The fewest columns Sakura accepts as the wrap column (`MINLINEKETAS`). [`LayoutConfig`] does
/// not enforce it, because the printing layout may use fewer; the settings dialog does.
pub const MIN_LINE_COLUMNS: u32 = 10;

/// Where wrapped rows start (折り返し行のインデント, `STypeConfig::m_nIndentLayout`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum WrapIndent {
    /// Wrapped rows start at the left edge (`getIndentOffset_Normal`). Sakura's default.
    #[default]
    None,
    /// Wrapped rows start after the last tab of the first row (`getIndentOffset_Tx2x`).
    AfterLastTab,
    /// Wrapped rows start under the first character after the leading white space of the line
    /// (`getIndentOffset_LeftSpace`).
    LeadingWhiteSpace {
        /// A full-width space counts as white space (`m_bAutoIndent_ZENSPACE`, on by default).
        full_width_space: bool,
        /// Other characters that count as white space (`m_szIndentChars`, empty by default).
        extra: CharSet,
    },
}

impl WrapIndent {
    /// `LeadingWhiteSpace` with Sakura's defaults.
    pub fn leading_white_space() -> Self {
        Self::LeadingWhiteSpace {
            full_width_space: true,
            extra: CharSet::default(),
        }
    }
}

/// What the wrap column is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    /// 折り返さない. Sakura still breaks at [`MAX_LINE_COLUMNS`] columns.
    None,
    /// 指定桁で折り返す, with the number of columns (折り返し桁数).
    Columns(u32),
}

/// The settings of the layout, which Sakura keeps in a document type (`STypeConfig`) and in
/// `CLayoutMgr`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutConfig {
    pub wrap: Wrap,
    /// Tab stops every this many columns (`m_nTabSpace`, 4 by default). It must be less than the
    /// wrap column; otherwise 4 is used (`_DoLayoutSub`: "if TAB >= wrap width, TAB = 4").
    pub tab_width: u32,
    /// 英文ワードラップ (`m_bWordWrap`): a line breaks before an English word rather than in it.
    pub word_wrap: bool,
    pub kinsoku: Kinsoku,
    pub width: WidthRule,
    pub indent: WrapIndent,
}

impl Default for LayoutConfig {
    /// Sakura's defaults of a document type (`CType.cpp`): tab width 4, no wrap, no word wrap,
    /// no kinsoku, no indent. For a wrapped screen, set [`LayoutConfig::wrap`].
    fn default() -> Self {
        Self {
            wrap: Wrap::None,
            tab_width: 4,
            word_wrap: false,
            kinsoku: Kinsoku::default(),
            width: WidthRule::default(),
            indent: WrapIndent::None,
        }
    }
}

impl LayoutConfig {
    /// The columns in a row (`GetMaxLineKetas`).
    pub fn max_columns(&self) -> u32 {
        match self.wrap {
            Wrap::None => MAX_LINE_COLUMNS,
            Wrap::Columns(n) => n.max(1),
        }
    }

    /// The tab width in use: the setting, or 4 when it is not less than the wrap column
    /// (`CLayoutMgr::_DoLayoutSub`, "Nov. 16, 2002 genta"). 0 counts as 1.
    pub fn effective_tab_width(&self) -> u32 {
        let tab = self.tab_width.max(1);
        if tab >= self.max_columns() { 4 } else { tab }
    }

    /// The columns `c` takes when it starts at column `x`: a tab goes to the next tab stop
    /// (`GetActualTsvSpace`, `TSV_MODE_NONE`: `tab - x % tab`, 1 to the tab width), and the other
    /// characters have their own width ([`WidthRule::columns`]). Control characters take 1, like
    /// the proxy character Sakura measures for them (`CCharWidthCache::QueryPixelWidth`).
    pub fn columns_of(&self, c: char, x: u32) -> u32 {
        if c == '\t' {
            let tab = self.effective_tab_width();
            tab - x % tab
        } else {
            self.width.columns(c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_goes_to_the_next_stop() {
        let cfg = LayoutConfig::default();
        // GetActualTsvSpace: tab - pos % tab, so a tab on a stop takes the whole width.
        assert_eq!(cfg.columns_of('\t', 0), 4);
        assert_eq!(cfg.columns_of('\t', 1), 3);
        assert_eq!(cfg.columns_of('\t', 3), 1);
        assert_eq!(cfg.columns_of('\t', 4), 4);
        assert_eq!(cfg.columns_of('\t', 6), 2);
    }

    #[test]
    fn tab_wider_than_the_wrap_column_becomes_4() {
        let cfg = LayoutConfig {
            wrap: Wrap::Columns(8),
            tab_width: 8,
            ..LayoutConfig::default()
        };
        assert_eq!(cfg.effective_tab_width(), 4);
        let ok = LayoutConfig {
            tab_width: 7,
            ..cfg
        };
        assert_eq!(ok.effective_tab_width(), 7);
    }

    #[test]
    fn no_wrap_is_the_sakura_maximum() {
        assert_eq!(LayoutConfig::default().max_columns(), 10240);
    }
}
