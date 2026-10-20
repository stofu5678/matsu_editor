// Ported from Sakura Editor and modified for UME Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/env/CShareData_IO.cpp (ShareData_IO_Print, ShareData_IO_PrintInts)
//   sakura_core/env/CDataProfile.cpp (IOProfileData<PRINTSETTING>)
//   sakura_core/env/CShareData.cpp (the names of the print settings), sakura_core/config/maxdata.h
//
// Original copyright notices of those files:
//   Copyright (C) 2005, D.S.Koba
//   Copyright (C) 2008, kobake
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

//! The print settings (印刷ページ設定) in the settings file: the eight named settings of `[Print]`.
//!
//! Sakura keeps `MAX_PRINTSETTINGARR` (8) of them in the shared data (`m_PrintSettingArr`) and
//! writes each as `PS[NN]`:
//!
//! ```text
//! [Print]
//! PS[00].nInts=12,24,1,70,30,100,200,200,100,1,9,1,0,1,0,0,1,0,0
//! PS[00].szSName=印刷設定 1
//! PS[00].szFF=ＭＳ 明朝
//! PS[00].szFFZ=ＭＳ 明朝
//! PS[00].szHF[0]=$f
//! PS[00].szFTF[1]=- $p -
//! PS[00].lfHeader=0,0,0,0,0,0,0,0,0,0,0,0,0
//! ...
//! ```
//!
//! The 19 numbers of `nInts` are read all together or not at all (`ShareData_IO_PrintInts`).
//! Everything else in the file stays as it is. The printer of a setting (`szDriver`, `szDevice`,
//! `szOutput`) is not used by UME Editor and is left alone; so are the flags `m_bHeaderUse` and
//! `m_bFooterUse` (the last six numbers of `nInts`), which Sakura's layout never reads.
//!
//! Only the face and the size of the header and the footer fonts are used. When the face of a font
//! is unchanged, the rest of its `LOGFONT` in the file is kept.
//!
//! Which of the eight a document type prints with is `nCurrentPrintSetting` of `Types(N)`
//! ([`store_current`]).

use ume_print::{HeaderFooterFont, Orientation, PrintSetting};

use super::font::{FontSetting, LogFont};
use super::{DocumentTypes, TypeSettings};
use crate::profile::{Profile, ProfileError};
use crate::value::scan_ints;

/// The section of the settings.
pub const SECTION: &str = "Print";
/// `MAX_PRINTSETTINGARR`.
pub const MAX_PRINT_SETTINGS: usize = 8;
/// The most characters of the name of a setting (`m_szPrintSettingName[32 + 1]`).
pub const NAME_MAX: usize = 32;
/// The most characters of the name of a font (`LF_FACESIZE` without the end mark).
pub const FACE_MAX: usize = 31;

/// `DMORIENT_LANDSCAPE`.
const DMORIENT_LANDSCAPE: i32 = 2;
/// The flags of `nInts` that Sakura initializes (`CPrint::SettingInitialize`): the left part of the
/// header and of the footer are in use.
const USE_FLAGS_DEFAULT: [i32; 6] = [1, 0, 0, 1, 0, 0];
/// The resolution the fonts of the file are worked out at.
const DPI: i32 = 96;

/// The eight print settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintSettings {
    slots: Vec<PrintSetting>,
}

/// The name Sakura gives the setting `index` (from 0): 「印刷設定 1」 and so on.
pub fn default_name(index: usize) -> String {
    format!("印刷設定 {}", index + 1)
}

impl PrintSettings {
    /// Eight copies of `base`, named 「印刷設定 N」 (`CShareData::InitShareData`).
    pub fn new(base: &PrintSetting) -> Self {
        let slots = (0..MAX_PRINT_SETTINGS)
            .map(|i| PrintSetting {
                name: default_name(i),
                ..base.clone()
            })
            .collect();
        Self { slots }
    }

    pub fn slots(&self) -> &[PrintSetting] {
        &self.slots
    }

    /// The setting `index`, or the first one for a number that is out of range.
    pub fn get(&self, index: usize) -> &PrintSetting {
        self.slots.get(index).unwrap_or(&self.slots[0])
    }

    /// Replaces the setting `index`. A number that is out of range does nothing.
    pub fn set(&mut self, index: usize, setting: PrintSetting) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = setting;
        }
    }

    /// Eight settings read from `profile` over `base`: what the file does not have is `base`'s.
    pub fn from_profile(base: &PrintSetting, profile: &Profile) -> Self {
        let mut settings = Self::new(base);
        settings.read(profile);
        settings
    }

    /// `ShareData_IO_Print`, reading: what the file has replaces what we have.
    pub fn read(&mut self, profile: &Profile) {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            read_slot(slot, profile, i);
        }
    }

    /// `ShareData_IO_Print`, writing: the settings that are not what `original` has. Writing
    /// only those keeps the others as the file has them (another window may have changed them
    /// since).
    pub fn write_changed(
        &self,
        original: &PrintSettings,
        profile: &mut Profile,
    ) -> Result<(), ProfileError> {
        for (i, slot) in self.slots.iter().enumerate() {
            if original.slots.get(i) != Some(slot) {
                write_slot(slot, profile, i)?;
            }
        }
        Ok(())
    }
}

/// The key of an entry of the setting `index` (`PS[%02d]` and the entry's own name).
fn key(index: usize, name: &str) -> String {
    format!("PS[{index:02}].{name}")
}

fn clip(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// `IOProfileData<PRINTSETTING>`, reading. An entry the file does not have keeps the value.
fn read_slot(s: &mut PrintSetting, profile: &Profile, index: usize) {
    let get = |name: &str| profile.get(SECTION, &key(index, name));
    // `ShareData_IO_PrintInts`: all 19 numbers or none.
    if let Some(n) = get("nInts").and_then(scan_ints::<19>) {
        s.print_font_width = n[0];
        s.print_font_height = n[1];
        s.print_dansuu = n[2];
        s.print_dan_space = n[3];
        s.print_line_spacing = n[4];
        s.print_margin_ty = n[5];
        s.print_margin_by = n[6];
        s.print_margin_lx = n[7];
        s.print_margin_rx = n[8];
        s.print_paper_orientation = if n[9] == DMORIENT_LANDSCAPE {
            Orientation::Landscape
        } else {
            Orientation::Portrait
        };
        // `(short)ints[10]`
        s.print_paper_size = n[10] as i16;
        s.print_word_wrap = n[11] != 0;
        s.print_line_number = n[12] != 0;
    }
    if let Some(v) = get("szSName") {
        s.name = clip(v, NAME_MAX);
    }
    if let Some(v) = get("szFF") {
        s.print_font_face_han = clip(v, FACE_MAX);
    }
    if let Some(v) = get("szFFZ") {
        s.print_font_face_zen = clip(v, FACE_MAX);
    }
    for j in 0..3 {
        if let Some(v) = get(&format!("szHF[{j}]")) {
            s.header_form[j] = clip(v, ume_print::HEADER_MAX - 1);
        }
        if let Some(v) = get(&format!("szFTF[{j}]")) {
            s.footer_form[j] = clip(v, ume_print::HEADER_MAX - 1);
        }
    }
    // 2002.02.16 hor: the old default form is converted.
    if s.header_form[0] == "&f" && s.footer_form[0] == "&C- &P -" {
        s.header_form[0] = "$f".into();
        s.footer_form[0] = String::new();
        s.footer_form[1] = "- $p -".into();
    }
    read_font(&mut s.header_font, profile, index, "Header");
    read_font(&mut s.footer_font, profile, index, "Footer");
    for (name, flag) in [
        ("bKinsokuHead", &mut s.print_kinsoku_head),
        ("bKinsokuTail", &mut s.print_kinsoku_tail),
        ("bKinsokuRet", &mut s.print_kinsoku_ret),
        ("bKinsokuKuto", &mut s.print_kinsoku_kuto),
        ("bColorPrint", &mut s.color_print),
    ] {
        if let Some(b) = profile.get_bool(SECTION, &key(index, name)) {
            *flag = b;
        }
    }
}

/// The three keys of a header or footer font: `lfHeader`, `nHeaderPointSize`, `lfHeaderFaceName`.
fn font_keys(index: usize, which: &str) -> (String, String, String) {
    (
        key(index, &format!("lf{which}")),
        key(index, &format!("n{which}PointSize")),
        key(index, &format!("lf{which}FaceName")),
    )
}

fn read_font(font: &mut Option<HeaderFooterFont>, profile: &Profile, index: usize, which: &str) {
    let (lf, size, face) = font_keys(index, which);
    let mut setting = FontSetting::default();
    if !setting.read(profile, SECTION, &lf, Some(&size), Some(&face)) {
        return;
    }
    let face = setting.log_font.face_name.clone();
    // Sakura takes an empty face name as "no font set".
    *font = (!face.is_empty()).then(|| HeaderFooterFont {
        face,
        point_size: setting.point_size_for_dpi(DPI),
    });
}

/// `IOProfileData<PRINTSETTING>`, writing.
fn write_slot(s: &PrintSetting, profile: &mut Profile, index: usize) -> Result<(), ProfileError> {
    // The flags of `nInts` that nothing here changes stay as the file has them.
    let old = profile
        .get(SECTION, &key(index, "nInts"))
        .and_then(scan_ints::<19>);
    let use_flags: [i32; 6] = old.map_or(USE_FLAGS_DEFAULT, |n| {
        [n[13], n[14], n[15], n[16], n[17], n[18]]
    });
    let orientation = match s.print_paper_orientation {
        Orientation::Portrait => 1,
        Orientation::Landscape => DMORIENT_LANDSCAPE,
    };
    let mut ints = [0; 19];
    ints[..13].copy_from_slice(&[
        s.print_font_width,
        s.print_font_height,
        s.print_dansuu,
        s.print_dan_space,
        s.print_line_spacing,
        s.print_margin_ty,
        s.print_margin_by,
        s.print_margin_lx,
        s.print_margin_rx,
        orientation,
        i32::from(s.print_paper_size),
        i32::from(s.print_word_wrap),
        i32::from(s.print_line_number),
    ]);
    ints[13..].copy_from_slice(&use_flags);
    // Compared as numbers, so that the spaces of the user's file do not matter.
    if old != Some(ints) {
        let value = ints.map(|n| n.to_string()).join(",");
        profile.set(SECTION, &key(index, "nInts"), &value)?;
    }
    profile.set_string(SECTION, &key(index, "szSName"), &s.name)?;
    profile.set_string(SECTION, &key(index, "szFF"), &s.print_font_face_han)?;
    profile.set_string(SECTION, &key(index, "szFFZ"), &s.print_font_face_zen)?;
    for j in 0..3 {
        profile.set_string(
            SECTION,
            &key(index, &format!("szHF[{j}]")),
            &s.header_form[j],
        )?;
        profile.set_string(
            SECTION,
            &key(index, &format!("szFTF[{j}]")),
            &s.footer_form[j],
        )?;
    }
    write_font(s.header_font.as_ref(), profile, index, "Header")?;
    write_font(s.footer_font.as_ref(), profile, index, "Footer")?;
    for (name, flag) in [
        ("bKinsokuHead", s.print_kinsoku_head),
        ("bKinsokuTail", s.print_kinsoku_tail),
        ("bKinsokuRet", s.print_kinsoku_ret),
        ("bKinsokuKuto", s.print_kinsoku_kuto),
        ("bColorPrint", s.color_print),
    ] {
        profile.set_bool(SECTION, &key(index, name), flag)?;
    }
    Ok(())
}

fn write_font(
    font: Option<&HeaderFooterFont>,
    profile: &mut Profile,
    index: usize,
    which: &str,
) -> Result<(), ProfileError> {
    let (lf, size, face) = font_keys(index, which);
    let setting = match font {
        Some(f) => {
            let mut current = FontSetting::default();
            let same_face = current.read(profile, SECTION, &lf, Some(&size), Some(&face))
                && current.log_font.face_name == f.face;
            let log_font = if same_face {
                current.log_font.clone()
            } else {
                LogFont {
                    face_name: f.face.clone(),
                    ..FontSetting::default().log_font
                }
            };
            let mut setting = FontSetting {
                log_font,
                point_size: f.point_size,
            };
            // The height follows the size, unless the file has the height of this size already.
            if !same_face || current.point_size_for_dpi(DPI) != f.point_size {
                setting.log_font.height = setting.height_for_dpi(DPI);
            }
            setting
        }
        // `memset( &m_lfHeader, 0, sizeof(LOGFONT) )`
        None => FontSetting {
            log_font: LogFont {
                height: 0,
                width: 0,
                escapement: 0,
                orientation: 0,
                weight: 0,
                italic: 0,
                underline: 0,
                strike_out: 0,
                char_set: 0,
                out_precision: 0,
                clip_precision: 0,
                quality: 0,
                pitch_and_family: 0,
                face_name: String::new(),
            },
            point_size: 0,
        },
    };
    setting.write(profile, SECTION, &lf, Some(&size), Some(&face))
}

/// Which of the eight a document type prints with (`m_nCurrentPrintSetting`), from `Types(N)`.
/// A number outside the eight is 0, as the dialog's list has no such row.
pub fn current_of(type_settings: &TypeSettings) -> usize {
    usize::try_from(type_settings.current_print_setting)
        .ok()
        .filter(|&i| i < MAX_PRINT_SETTINGS)
        .unwrap_or(0)
}

/// Writes the choice of the print setting to `Types(index)` (`CEditWnd::OnPrintPageSetting` stores
/// the type with the new number). Only that number of `nInts` changes; a type the file has no
/// section for is written with all its keys. Returns whether the file changed.
pub fn store_current(
    profile: &mut Profile,
    index: usize,
    current: usize,
) -> Result<bool, ProfileError> {
    let Ok(number) = i32::try_from(index) else {
        return Ok(false);
    };
    let wanted = i32::try_from(current).unwrap_or(0);
    let section = TypeSettings::section_name(number);
    if let Some(mut ints) = profile.get(&section, "nInts").and_then(scan_ints::<12>) {
        if ints[10] == wanted {
            return Ok(false);
        }
        ints[10] = wanted;
        profile.set(&section, "nInts", &ints.map(|n| n.to_string()).join(","))?;
        return Ok(true);
    }
    let types = DocumentTypes::from_profile(profile);
    let Some(mut type_settings) = types.get(index).cloned() else {
        return Ok(false);
    };
    if type_settings.current_print_setting == wanted {
        return Ok(false);
    }
    type_settings.current_print_setting = wanted;
    type_settings.write(profile, number)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
