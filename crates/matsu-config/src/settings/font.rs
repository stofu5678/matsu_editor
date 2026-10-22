// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_LogFont, ShareData_IO_Font)
//   sakura_core/env/CShareData.cpp (the default font)
//   sakura_core/util/window.h (DPI conversion)
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

//! Font settings stored as a Windows `LOGFONT`.
//!
//! Source: `sakura_core/env/CShareData_IO.cpp` (`ShareData_IO_LogFont`, `ShareData_IO_Font`) and
//! `sakura_core/env/CShareData.cpp` (the default font).
//!
//! The values are read and kept as they are in the file; this module does not choose a font on other
//! systems. How a Windows face name such as "ＭＳ ゴシック" is shown on macOS and Linux is a decision for
//! the UI (see the PR description).

use crate::profile::{Profile, ProfileError};
use crate::value::{parse_long, scan_ints};

/// `LF_FACESIZE`: a face name has at most 31 UTF-16 units (`StaticString<LF_FACESIZE>`).
const FACE_NAME_MAX_UNITS: usize = 31;

/// The fields of a Windows `LOGFONT`, in the order Sakura writes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFont {
    pub height: i32,
    pub width: i32,
    pub escapement: i32,
    pub orientation: i32,
    pub weight: i32,
    pub italic: u8,
    pub underline: u8,
    pub strike_out: u8,
    pub char_set: u8,
    pub out_precision: u8,
    pub clip_precision: u8,
    pub quality: u8,
    pub pitch_and_family: u8,
    pub face_name: String,
}

impl LogFont {
    /// `SHIFTJIS_CHARSET`.
    pub const SHIFTJIS_CHARSET: u8 = 0x80;

    fn to_ints(&self) -> [i32; 13] {
        [
            self.height,
            self.width,
            self.escapement,
            self.orientation,
            self.weight,
            i32::from(self.italic),
            i32::from(self.underline),
            i32::from(self.strike_out),
            i32::from(self.char_set),
            i32::from(self.out_precision),
            i32::from(self.clip_precision),
            i32::from(self.quality),
            i32::from(self.pitch_and_family),
        ]
    }

    fn to_value(&self) -> String {
        let ints = self.to_ints().map(|n| n.to_string());
        ints.join(",")
    }
}

/// A font setting: a `LOGFONT` and the point size next to it (`nPointSize`, in 1/10 point).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontSetting {
    pub log_font: LogFont,
    /// 1/10 point. `0` means "not set yet" (an old file); the height of `log_font` is used then.
    pub point_size: i32,
}

impl Default for FontSetting {
    /// Sakura's default: MS Gothic 10 pt (`CShareData::InitShareData`, "MS ゴシック標準スタイル10ptに設定").
    /// The height is -13 pixels, which is 10 pt at 96 dpi.
    fn default() -> Self {
        FontSetting {
            log_font: LogFont {
                height: -13,
                width: 0,
                escapement: 0,
                orientation: 0,
                weight: 400,
                italic: 0,
                underline: 0,
                strike_out: 0,
                char_set: LogFont::SHIFTJIS_CHARSET,
                out_precision: 0x3,
                clip_precision: 0x2,
                quality: 0x1,
                pitch_and_family: 0x31,
                face_name: "ＭＳ ゴシック".to_owned(),
            },
            point_size: 0,
        }
    }
}

/// Windows `MulDiv`: `a * b / c`, rounded to the nearest, halves away from zero.
fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    let n = a * b;
    let negative = (n < 0) != (c < 0);
    let (n, c) = (n.abs(), c.abs());
    let q = (n + c / 2) / c;
    if negative { -q } else { q }
}

impl FontSetting {
    /// `ShareData_IO_LogFont`, read side. The keys are `key_lf`, `nPointSize` and `{key_lf}FaceName` unless
    /// the optional keys are given.
    ///
    /// Like Sakura, nothing is changed when the `LOGFONT` or the face name is missing or broken, except the
    /// point size, which is read first and on its own. Returns whether the font was read.
    ///
    /// Unlike Sakura, the height is not recomputed from the point size here, because that depends on the
    /// DPI of the screen. Use [`FontSetting::height_for_dpi`].
    pub fn read(
        &mut self,
        profile: &Profile,
        section: &str,
        key_lf: &str,
        key_point_size: Option<&str>,
        key_face_name: Option<&str>,
    ) -> bool {
        let key_point_size = key_point_size.unwrap_or("nPointSize");
        if let Some(n) = profile.get(section, key_point_size).and_then(parse_long) {
            self.point_size = n;
        }
        let Some(ints) = profile.get(section, key_lf).and_then(scan_ints::<13>) else {
            return false;
        };
        let face_key = key_face_name.map_or_else(|| format!("{key_lf}FaceName"), str::to_owned);
        let Some(face_name) = profile.get(section, &face_key) else {
            return false;
        };
        if face_name.encode_utf16().count() > FACE_NAME_MAX_UNITS {
            return false;
        }
        // `static_cast<BYTE>(int)` keeps the low 8 bits.
        self.log_font = LogFont {
            height: ints[0],
            width: ints[1],
            escapement: ints[2],
            orientation: ints[3],
            weight: ints[4],
            italic: ints[5] as u8,
            underline: ints[6] as u8,
            strike_out: ints[7] as u8,
            char_set: ints[8] as u8,
            out_precision: ints[9] as u8,
            clip_precision: ints[10] as u8,
            quality: ints[11] as u8,
            pitch_and_family: ints[12] as u8,
            face_name: face_name.to_owned(),
        };
        true
    }

    /// `ShareData_IO_LogFont`, write side. Entries that already hold the same value are left as they are.
    pub fn write(
        &self,
        profile: &mut Profile,
        section: &str,
        key_lf: &str,
        key_point_size: Option<&str>,
        key_face_name: Option<&str>,
    ) -> Result<(), ProfileError> {
        profile.set_i32(
            section,
            key_point_size.unwrap_or("nPointSize"),
            self.point_size,
        )?;
        // Compare as numbers, so that spaces in the user's file do not matter.
        let ints = self.log_font.to_ints();
        if profile.get(section, key_lf).and_then(scan_ints::<13>) != Some(ints) {
            profile.set(section, key_lf, &self.log_font.to_value())?;
        }
        let face_key = key_face_name.map_or_else(|| format!("{key_lf}FaceName"), str::to_owned);
        profile.set(section, &face_key, &self.log_font.face_name)
    }

    /// The height in pixels at a screen resolution: when `point_size` is set, it is turned into pixels
    /// (`-DpiPointsToPixels(abs(nPointSize), 10)`), otherwise the height in the file is used.
    pub fn height_for_dpi(&self, dpi: i32) -> i32 {
        if self.point_size != 0 {
            let pixels = mul_div(i64::from(self.point_size).abs(), i64::from(dpi), 72 * 10);
            i32::try_from(-pixels).unwrap_or(i32::MIN)
        } else {
            self.log_font.height
        }
    }

    /// The size in 1/10 point at a screen resolution: `point_size` if set, otherwise worked out from the
    /// height (`DpiPixelsToPoints(abs(lfHeight), 10)`).
    pub fn point_size_for_dpi(&self, dpi: i32) -> i32 {
        if self.point_size != 0 {
            self.point_size
        } else {
            let tenths = mul_div(
                i64::from(self.log_font.height).abs() * 10,
                72,
                i64::from(dpi),
            );
            i32::try_from(tenths).unwrap_or(i32::MAX)
        }
    }
}

/// The `[Font]` section: the font of the edit area, unless a document type has its own (`ShareData_IO_Font`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewFont {
    pub font: FontSetting,
    /// `bFontIs_FIXED_PITCH`: the font is a fixed pitch font.
    pub fixed_pitch: bool,
}

impl Default for ViewFont {
    fn default() -> Self {
        ViewFont {
            font: FontSetting::default(),
            fixed_pitch: true,
        }
    }
}

impl ViewFont {
    /// Section name in `sakura.ini`.
    pub const SECTION: &'static str = "Font";

    /// Reads the section over the defaults.
    pub fn read(profile: &Profile) -> Self {
        let mut v = ViewFont::default();
        v.font.read(profile, Self::SECTION, "lf", None, None);
        if let Some(b) = profile.get_bool(Self::SECTION, "bFontIs_FIXED_PITCH") {
            v.fixed_pitch = b;
        }
        v
    }

    /// Writes the section.
    pub fn write(&self, profile: &mut Profile) -> Result<(), ProfileError> {
        self.font.write(profile, Self::SECTION, "lf", None, None)?;
        profile.set_bool(Self::SECTION, "bFontIs_FIXED_PITCH", self.fixed_pitch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf8(s: &str) -> Vec<u8> {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(s.as_bytes());
        bytes
    }

    const FONT: &str = "[Font]\r\nnPointSize=100\r\nlf=-13,0,0,0,400,0,0,0,128,3,2,1,49\r\nlfFaceName=ＭＳ ゴシック\r\nbFontIs_FIXED_PITCH=1\r\n";

    #[test]
    fn reads_the_font_section() {
        let p = Profile::from_bytes(&utf8(FONT));
        let v = ViewFont::read(&p);
        assert_eq!(
            v.font,
            FontSetting {
                point_size: 100,
                ..FontSetting::default()
            }
        );
        assert!(v.fixed_pitch);
    }

    #[test]
    fn missing_or_broken_font_keeps_the_default() {
        // Only 12 numbers: the whole LOGFONT is refused (`13 != swscanf_s(...)`), the point size is still read.
        let p = Profile::from_bytes(
            b"[Font]\r\nnPointSize=120\r\nlf=-13,0,0,0,400,0,0,0,128,3,2,1\r\nlfFaceName=Foo\r\n",
        );
        let v = ViewFont::read(&p);
        assert_eq!(v.font.log_font, FontSetting::default().log_font);
        assert_eq!(v.font.point_size, 120);
        // No face name: refused.
        let p = Profile::from_bytes(b"[Font]\r\nlf=-13,0,0,0,400,0,0,0,128,3,2,1,49\r\n");
        assert_eq!(ViewFont::read(&p), ViewFont::default());
        // A face name longer than 31 units does not fit LF_FACESIZE.
        let long = "a".repeat(32);
        let p = Profile::from_bytes(
            format!("[Font]\r\nlf=-13,0,0,0,400,0,0,0,128,3,2,1,49\r\nlfFaceName={long}\r\n")
                .as_bytes(),
        );
        assert_eq!(ViewFont::read(&p), ViewFont::default());
    }

    #[test]
    fn writing_unchanged_font_keeps_the_file() {
        let src = FONT.replace("lf=-13,0", "lf=-13, 0");
        let mut p = Profile::from_bytes(&utf8(&src));
        ViewFont::read(&p).write(&mut p).unwrap();
        assert_eq!(p.to_bytes(), utf8(&src));
    }

    #[test]
    fn writes_a_new_font() {
        let mut p = Profile::new();
        let mut v = ViewFont::default();
        v.font.point_size = 110;
        v.font.log_font.face_name = "Consolas".to_owned();
        v.write(&mut p).unwrap();
        assert_eq!(p.get("Font", "nPointSize"), Some("110"));
        assert_eq!(
            p.get("Font", "lf"),
            Some("-13,0,0,0,400,0,0,0,128,3,2,1,49")
        );
        assert_eq!(p.get("Font", "lfFaceName"), Some("Consolas"));
        assert_eq!(ViewFont::read(&p), v);
    }

    #[test]
    fn dpi_conversion_follows_muldiv() {
        let mut f = FontSetting {
            point_size: 100,
            ..FontSetting::default()
        };
        // 10 pt at 96 dpi is 13 pixels; at 144 dpi it is 20.
        assert_eq!(f.height_for_dpi(96), -13);
        assert_eq!(f.height_for_dpi(144), -20);
        // Not set: the height of the file, and the size worked out from it.
        f.point_size = 0;
        assert_eq!(f.height_for_dpi(144), -13);
        assert_eq!(f.point_size_for_dpi(96), 98);
        // Values at the limits of the integer type do not overflow.
        f.point_size = i32::MIN;
        assert_eq!(f.height_for_dpi(96), -286_331_153);
        f.point_size = 0;
        f.log_font.height = i32::MIN;
        assert_eq!(f.point_size_for_dpi(96), i32::MAX);
    }
}
