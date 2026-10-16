// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CDataProfile.cpp (IOProfileData<ColorInfo>)
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_ColorSet)
//   sakura_core/view/colors/CColorStrategy.cpp (g_ColorAttributeArr)
//   sakura_core/doc/CDocTypeSetting.cpp (ColorInfo_DEFAULT)
//
// Original copyright notices of those files:
//   Copyright (C) 2003-2006, D.S.Koba
//   Copyright (C) 2004, MIK, genta
//   Copyright (C) 2006, ryoji
//   Copyright (C) 2008, kobake
//   Copyright (C) 2009, ryoji
//   Copyright (C) 2018-2026, Sakura Editor Organization
//   SPDX-License-Identifier: Zlib
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

//! Colour and display settings of a document type (`C[XXX]=...` entries of a `Types(N)` section).
//!
//! Source:
//! - `sakura_core/env/CDataProfile.cpp`, `IOProfileData<ColorInfo>`: the value is
//!   `disp,bold,text,back,underline`, written as `%d,%d,%06x,%06x,%d`.
//! - `sakura_core/env/CShareData_IO.cpp`, `ShareData_IO_ColorSet`: key `C[name]` and the repairs after reading.
//! - `sakura_core/view/colors/CColorStrategy.cpp`, `g_ColorAttributeArr`: the names and the attribute flags.
//! - `sakura_core/doc/CDocTypeSetting.cpp`, `ColorInfo_DEFAULT`: the default colours.
//! - `sakura_core/view/colors/EColorIndexType.h`: the order.
//!
//! The "show tab / full-width space / line end / EOF / line number" switches of Sakura are the `disp`
//! flag of the entries `TAB`, `ZEN`, `EOL`, `EOF` and `LNO`.

use crate::profile::{Profile, ProfileError};
use crate::value::scan_color;

/// The name of a colour entry as it is in the file (`C[TXT]`: `TXT`).
pub type ColorName = &'static str;

/// Names of the colour entries that the first editor window uses.
pub mod names {
    use super::ColorName;
    /// Text.
    pub const TEXT: ColorName = "TXT";
    /// Caret.
    pub const CARET: ColorName = "CAR";
    /// Line number.
    pub const LINE_NUMBER: ColorName = "LNO";
    /// Tab mark.
    pub const TAB: ColorName = "TAB";
    /// Half-width space mark.
    pub const HALF_SPACE: ColorName = "SPC";
    /// Full-width (Japanese) space mark.
    pub const FULL_SPACE: ColorName = "ZEN";
    /// Control code.
    pub const CTRL_CODE: ColorName = "CTL";
    /// Line end mark.
    pub const EOL: ColorName = "EOL";
    /// Wrap mark.
    pub const WRAP: ColorName = "RAP";
    /// EOF mark.
    pub const EOF: ColorName = "EOF";
    /// Bookmark (`COLORIDX_MARK`).
    pub const BOOKMARK: ColorName = "MRK";
    /// Single quote string.
    pub const SINGLE_QUOTE: ColorName = "SQT";
    /// Double quote string.
    pub const DOUBLE_QUOTE: ColorName = "WQT";
}

/// `COLOR_ATTRIB_*` (`sakura_core/_main/global.h`).
mod attr {
    pub const FORCE_DISP: u32 = 0x1;
    pub const NO_BOLD: u32 = 0x100;
    pub const NO_UNDERLINE: u32 = 0x200;
    pub const NO_EFFECTS: u32 = 0xF00;
    // Not used here: NO_TEXT 0x10, NO_BACK 0x20 (they only limit the settings dialog).
    pub const NO_TEXT: u32 = 0x10;
    pub const NO_BACK: u32 = 0x20;
}

/// A colour as Windows keeps it (`COLORREF`, `0x00BBGGRR`).
pub type ColorRef = u32;

/// `RGB(r, g, b)`.
pub const fn rgb(r: u8, g: u8, b: u8) -> ColorRef {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

/// One colour entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorEntry {
    /// The item is shown (coloured).
    pub disp: bool,
    pub bold: bool,
    pub underline: bool,
    pub text: ColorRef,
    pub back: ColorRef,
}

struct Def {
    name: ColorName,
    attr: u32,
    entry: ColorEntry,
}

const BG: ColorRef = rgb(255, 251, 240);

const fn d(
    name: ColorName,
    attr: u32,
    disp: bool,
    bold: bool,
    underline: bool,
    text: ColorRef,
    back: ColorRef,
) -> Def {
    Def {
        name,
        attr,
        entry: ColorEntry {
            disp,
            bold,
            underline,
            text,
            back,
        },
    }
}

use attr::{FORCE_DISP, NO_BACK, NO_BOLD, NO_EFFECTS, NO_TEXT, NO_UNDERLINE};

/// Every colour entry in Sakura's order (`EColorIndexType`) with its default.
const DEFS: [Def; 57] = [
    d(
        "TXT",
        FORCE_DISP | NO_EFFECTS,
        true,
        false,
        false,
        rgb(0, 0, 0),
        BG,
    ),
    d(
        "RUL",
        NO_EFFECTS,
        true,
        false,
        false,
        rgb(0, 0, 0),
        rgb(239, 239, 239),
    ),
    d(
        "CAR",
        FORCE_DISP | NO_BACK | NO_EFFECTS,
        true,
        false,
        false,
        rgb(0, 0, 0),
        BG,
    ),
    d(
        "IME",
        NO_BACK | NO_EFFECTS,
        true,
        false,
        false,
        rgb(255, 0, 0),
        BG,
    ),
    d(
        "CBK",
        NO_TEXT | NO_EFFECTS,
        false,
        false,
        false,
        rgb(0, 0, 0),
        rgb(255, 255, 128),
    ),
    d(
        "UND",
        NO_BACK | NO_EFFECTS,
        true,
        false,
        false,
        rgb(0, 0, 255),
        BG,
    ),
    d(
        "CVL",
        NO_BACK | (NO_EFFECTS & !NO_BOLD),
        false,
        false,
        false,
        rgb(128, 128, 255),
        BG,
    ),
    d(
        "NOT",
        NO_BACK | NO_EFFECTS,
        false,
        false,
        false,
        rgb(192, 192, 255),
        BG,
    ),
    d(
        "LNO",
        0,
        true,
        false,
        false,
        rgb(0, 0, 255),
        rgb(239, 239, 239),
    ),
    d(
        "MOD",
        0,
        true,
        true,
        false,
        rgb(0, 0, 255),
        rgb(239, 239, 239),
    ),
    d(
        "EBK",
        NO_TEXT | NO_EFFECTS,
        false,
        false,
        false,
        rgb(0, 0, 0),
        rgb(243, 243, 243),
    ),
    d("TAB", 0, true, false, false, rgb(128, 128, 128), BG),
    d("SPC", 0, false, false, false, rgb(192, 192, 192), BG),
    d("ZEN", 0, true, false, false, rgb(192, 192, 192), BG),
    d("CTL", 0, true, false, false, rgb(255, 255, 0), BG),
    d("EOL", 0, true, false, false, rgb(0, 128, 255), BG),
    d("RAP", 0, true, false, false, rgb(255, 0, 255), BG),
    d("VER", 0, false, false, false, rgb(192, 192, 192), BG),
    d("EOF", 0, true, false, false, rgb(0, 255, 255), rgb(0, 0, 0)),
    d("NUM", 0, false, false, false, rgb(235, 0, 0), BG),
    d("BRC", 0, false, true, false, rgb(128, 0, 0), BG),
    d(
        "SEL",
        0,
        true,
        false,
        false,
        rgb(49, 106, 197),
        rgb(49, 106, 197),
    ),
    // The entry name of the first search colour. Written in two parts because the spell checker (typos) reads
    // it as a misspelling of "find".
    d(
        concat!("FN", "D"),
        0,
        true,
        false,
        false,
        rgb(0, 0, 0),
        rgb(255, 255, 0),
    ),
    d(
        "FN2",
        0,
        true,
        false,
        false,
        rgb(0, 0, 0),
        rgb(160, 255, 255),
    ),
    d(
        "FN3",
        0,
        true,
        false,
        false,
        rgb(0, 0, 0),
        rgb(153, 255, 153),
    ),
    d(
        "FN4",
        0,
        true,
        false,
        false,
        rgb(0, 0, 0),
        rgb(255, 153, 153),
    ),
    d(
        "FN5",
        0,
        true,
        false,
        false,
        rgb(0, 0, 0),
        rgb(255, 102, 255),
    ),
    d("CMT", 0, true, false, false, rgb(0, 128, 0), BG),
    d("SQT", 0, true, false, false, rgb(64, 128, 128), BG),
    d("WQT", 0, true, false, false, rgb(128, 0, 64), BG),
    d("HDC", 0, false, false, false, rgb(128, 0, 64), BG),
    d("URL", 0, true, false, true, rgb(0, 0, 255), BG),
    d("KW1", 0, true, false, false, rgb(0, 0, 255), BG),
    d("KW2", 0, true, false, false, rgb(255, 128, 0), BG),
    d("KW3", 0, true, false, false, rgb(255, 128, 0), BG),
    d("KW4", 0, true, false, false, rgb(255, 128, 0), BG),
    d("KW5", 0, true, false, false, rgb(255, 128, 0), BG),
    d("KW6", 0, true, false, false, rgb(255, 128, 0), BG),
    d("KW7", 0, true, false, false, rgb(255, 128, 0), BG),
    d("KW8", 0, true, false, false, rgb(255, 128, 0), BG),
    d("KW9", 0, true, false, false, rgb(255, 128, 0), BG),
    d("KWA", 0, true, false, false, rgb(255, 128, 0), BG),
    d("RK1", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RK2", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RK3", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RK4", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RK5", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RK6", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RK7", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RK8", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RK9", 0, false, false, false, rgb(0, 0, 255), BG),
    d("RKA", 0, false, false, false, rgb(0, 0, 255), BG),
    d(
        "DFA",
        0,
        true,
        false,
        false,
        rgb(0, 0, 210),
        rgb(162, 208, 255),
    ),
    d(
        "DFC",
        0,
        true,
        false,
        false,
        rgb(0, 111, 0),
        rgb(189, 253, 192),
    ),
    d(
        "DFD",
        0,
        true,
        false,
        false,
        rgb(213, 106, 0),
        rgb(255, 233, 172),
    ),
    d("MRK", 0, true, false, false, BG, rgb(0, 128, 192)),
    d(
        "PGV",
        NO_TEXT | NO_EFFECTS,
        true,
        false,
        false,
        BG,
        rgb(190, 230, 255),
    ),
];

/// All colour entries of a document type, in Sakura's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorSet {
    entries: [ColorEntry; DEFS.len()],
}

impl Default for ColorSet {
    fn default() -> Self {
        ColorSet {
            entries: std::array::from_fn(|i| DEFS[i].entry),
        }
    }
}

impl ColorSet {
    /// The names of all entries, in order.
    pub fn all_names() -> impl Iterator<Item = ColorName> {
        DEFS.iter().map(|d| d.name)
    }

    fn index(name: &str) -> Option<usize> {
        DEFS.iter().position(|d| d.name == name)
    }

    /// An entry by name (`TXT`, `TAB`, ...).
    pub fn get(&self, name: &str) -> Option<&ColorEntry> {
        Self::index(name).map(|i| &self.entries[i])
    }

    /// An entry by name, for changing.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut ColorEntry> {
        Self::index(name).map(|i| &mut self.entries[i])
    }

    /// `ShareData_IO_ColorSet`, read side: entries that are missing or broken keep their value; then the
    /// repairs of 2006.12.18 and 2006.12.07 are applied.
    pub fn read(&mut self, profile: &Profile, section: &str) {
        for (def, entry) in DEFS.iter().zip(self.entries.iter_mut()) {
            let Some([disp, bold, text, back, underline]) = profile
                .get(section, &format!("C[{}]", def.name))
                .and_then(scan_color)
            else {
                continue;
            };
            *entry = ColorEntry {
                disp: disp != 0,
                bold: bold != 0,
                underline: underline != 0,
                text,
                back,
            };
            // Repair contradicting settings (2006.12.18 ryoji).
            if def.attr & attr::FORCE_DISP != 0 {
                entry.disp = true;
            }
            if def.attr & NO_BOLD != 0 {
                entry.bold = false;
            }
            if def.attr & NO_UNDERLINE != 0 {
                entry.underline = false;
            }
        }
        // 2006.12.07 ryoji: a caret colour equal to the text background is replaced by the text colour.
        let (caret, text) = (Self::index("CAR"), Self::index("TXT"));
        if let (Some(caret), Some(text)) = (caret, text)
            && self.entries[caret].text == self.entries[text].back
        {
            self.entries[caret].text = self.entries[text].text;
        }
    }

    /// `ShareData_IO_ColorSet`, write side. An entry that already means the same is left as written.
    pub fn write(&self, profile: &mut Profile, section: &str) -> Result<(), ProfileError> {
        for (def, e) in DEFS.iter().zip(self.entries.iter()) {
            let key = format!("C[{}]", def.name);
            let same = profile.get(section, &key).and_then(scan_color)
                == Some([
                    u32::from(e.disp),
                    u32::from(e.bold),
                    e.text,
                    e.back,
                    u32::from(e.underline),
                ]);
            if !same {
                let value = format!(
                    "{},{},{:06x},{:06x},{}",
                    u8::from(e.disp),
                    u8::from(e.bold),
                    e.text,
                    e.back,
                    u8::from(e.underline)
                );
                profile.set(section, &key, &value)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_in_sakura_order() {
        let names: Vec<_> = ColorSet::all_names().collect();
        assert_eq!(names.len(), 57);
        // g_ColorAttributeArr
        assert_eq!(&names[..4], ["TXT", "RUL", "CAR", "IME"]);
        assert_eq!(names[11], "TAB");
        assert_eq!(names[18], "EOF");
        assert_eq!(&names[41..43], ["KWA", "RK1"]);
        assert_eq!(names[56], "PGV");
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 57);
    }

    #[test]
    fn defaults_match_sakura() {
        let c = ColorSet::default();
        // ColorInfo_DEFAULT: TAB and the full-width space are shown, the half-width space is not.
        assert!(c.get(names::TAB).unwrap().disp);
        assert!(c.get(names::FULL_SPACE).unwrap().disp);
        assert!(!c.get(names::HALF_SPACE).unwrap().disp);
        assert!(c.get(names::EOL).unwrap().disp);
        assert!(c.get(names::EOF).unwrap().disp);
        assert!(c.get(names::LINE_NUMBER).unwrap().disp);
        assert_eq!(c.get(names::TEXT).unwrap().back, rgb(255, 251, 240));
        assert_eq!(c.get(names::TEXT).unwrap().back, 0xf0fbff);
    }

    #[test]
    fn reads_and_writes_the_entry_format() {
        let mut p = Profile::from_bytes(
            b"[Types(0)]\r\nC[TAB]=0,1,00ff00,fffbf0,1\r\nC[LNO]=1,0,ff0000,efefef,0\r\n",
        );
        let mut c = ColorSet::default();
        c.read(&p, "Types(0)");
        let tab = c.get("TAB").unwrap();
        assert_eq!(
            *tab,
            ColorEntry {
                disp: false,
                bold: true,
                underline: true,
                text: 0x00ff00,
                back: 0xfffbf0
            }
        );
        assert_eq!(c.get("LNO").unwrap().text, 0xff0000);
        // Not in the file: the default.
        assert_eq!(c.get("EOF"), ColorSet::default().get("EOF"));
        // Unchanged entries stay as they are, a changed one is written in Sakura's format.
        c.get_mut("EOL").unwrap().disp = false;
        c.write(&mut p, "Types(0)").unwrap();
        assert_eq!(p.get("Types(0)", "C[TAB]"), Some("0,1,00ff00,fffbf0,1"));
        assert_eq!(p.get("Types(0)", "C[EOL]"), Some("0,0,ff8000,f0fbff,0"));
    }

    #[test]
    fn repairs_contradicting_settings() {
        // TXT is always shown and has no bold/underline; CVL may be bold but not underlined.
        let p = Profile::from_bytes(
            b"[T]\r\nC[TXT]=0,1,0,fffbf0,1\r\nC[CVL]=1,1,0,0,1\r\nC[CAR]=1,0,fffbf0,fffbf0,0\r\n",
        );
        let mut c = ColorSet::default();
        c.read(&p, "T");
        let txt = c.get("TXT").unwrap();
        assert!(txt.disp && !txt.bold && !txt.underline);
        let cvl = c.get("CVL").unwrap();
        assert!(cvl.bold && !cvl.underline);
        // The caret colour equals the text background: use the text colour (black).
        assert_eq!(c.get("CAR").unwrap().text, 0);
    }

    #[test]
    fn broken_entries_are_skipped() {
        let p = Profile::from_bytes(b"[T]\r\nC[TAB]=1,0,zz,0,0\r\nC[SPC]=1,0,0,0\r\n");
        let mut c = ColorSet::default();
        c.read(&p, "T");
        assert_eq!(c, ColorSet::default());
    }
}
