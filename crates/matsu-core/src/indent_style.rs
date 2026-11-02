// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/parse/DetectIndentationStyle.cpp (DetectIndentationStyle, ReadEditorConfig,
//     EditorConfigParser, glob_matches_extension)
//   sakura_core/parse/DetectIndentationStyle.h (IndentationStyle)
//   sakura_core/doc/CDocEditor.cpp (CDocEditor::OnAfterLoad, the part that applies the style)
//
// Original copyright notices of those files:
//   Copyright (C) 2018-2022, Sakura Editor Organization
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

//! Detects the indentation style of a file that was just loaded (`m_bDetectIndentationStyleOnFileLoad`).
//!
//! Sakura: `DetectIndentationStyle` (`sakura_core/parse/DetectIndentationStyle.cpp`), called by
//! `CDocEditor::OnAfterLoad` (`sakura_core/doc/CDocEditor.cpp`) with the first 256 lines. A
//! `.editorconfig` that names the style (`indent_style`) wins over the text; otherwise the first
//! chars of the lines say it: a style wins if its lines are more than four times as many as the
//! other's. For spaces, the most frequent difference of the indentation of a line to the line
//! before is the width.
//!
//! This module only decides. [`IndentStyle::changes`] says what that means for the document
//! (`OnAfterLoad`): the caller sets the width of the tab and the use of spaces for that
//! document only; the settings of the type are never written.
//!
//! Differences from Sakura:
//!
//! * The text is read by ranges: at most [`LEAD_BYTES`] bytes of each of the first
//!   [`MAX_LINES_TO_CHECK`] lines, so a huge line or file costs nothing.
//! * A `.editorconfig` glob `*.ext` matches the extension exactly. Sakura compares with
//!   `starts_with`, so `[*.cpp]` also matched a `.c` file (type A of ADR-0018).
//! * A tab width outside `1..=`[`TAB_WIDTH_MAX`] (`TABSPACE_MAX`, the limit of the dialog) is not
//!   taken; Sakura passes any number from `.editorconfig` to the layout.

use std::path::Path;

use crate::buffer::Document;

/// The number of lines that are looked at (`CDocEditor::OnAfterLoad` passes 256).
pub const MAX_LINES_TO_CHECK: usize = 256;

/// `TABSPACE_MAX` (`config/app_constants.h`): the largest tab width.
pub const TAB_WIDTH_MAX: u32 = 64;

/// How many bytes of the start of a line are read. The leading blanks of a line are counted up to
/// here.
pub const LEAD_BYTES: usize = 1024;

/// `IndentationStyle::Character`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentChar {
    Spaces,
    Tabs,
    /// Could not be decided.
    Unknown,
}

/// `IndentationStyle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndentStyle {
    pub character: IndentChar,
    /// `tabSpace`: the width found, `-1` for none (the width in use stays). It is not checked
    /// here; [`IndentStyle::changes`] checks it.
    pub tab_space: i32,
}

/// What a loaded document takes from a style (`OnAfterLoad`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct IndentChanges {
    /// `m_bInsSpace`: the Tab key inserts spaces (`Some(true)`) or a tab (`Some(false)`).
    pub ins_space: Option<bool>,
    /// The width of a tab, temporarily (`m_bTabSpaceCurTemp`).
    pub tab_width: Option<u32>,
}

impl IndentStyle {
    fn unknown() -> Self {
        Self {
            character: IndentChar::Unknown,
            tab_space: -1,
        }
    }

    /// What the document takes from this style (the end of `CDocEditor::OnAfterLoad`'s detection).
    pub fn changes(&self) -> IndentChanges {
        let width = u32::try_from(self.tab_space)
            .ok()
            .filter(|w| (1..=TAB_WIDTH_MAX).contains(w));
        match self.character {
            // Only with a width: the spaces cannot be turned on without one.
            IndentChar::Spaces => match width {
                Some(w) => IndentChanges {
                    ins_space: Some(true),
                    tab_width: Some(w),
                },
                None => IndentChanges::default(),
            },
            // The width stays if there is none.
            IndentChar::Tabs => IndentChanges {
                ins_space: Some(false),
                tab_width: width,
            },
            IndentChar::Unknown => IndentChanges::default(),
        }
    }
}

/// The style of the first lines of `doc`, a file at `path` (`DetectIndentationStyle`).
pub fn detect(path: Option<&Path>, doc: &Document) -> IndentStyle {
    if let Some(style) = path.and_then(read_editorconfig) {
        return style;
    }
    let mut leads = Vec::new();
    for i in 0..MAX_LINES_TO_CHECK {
        // `GetLengthWithoutEOL() == 0` is an empty lead; the end of the document ends the loop.
        match doc.line_raw_len(i) {
            Ok(Some(0)) => leads.push(String::new()),
            Ok(Some(_)) => match doc.line_slice(i, 0, LEAD_BYTES) {
                Ok(Some(slice)) => leads.push(slice.text),
                _ => break,
            },
            _ => break,
        }
    }
    detect_from_leads(&leads)
}

/// The style from the starts of the lines (`leads`: an empty one for an empty line). Only the
/// first [`MAX_LINES_TO_CHECK`] are looked at.
pub fn detect_from_leads<S: AsRef<str>>(leads: &[S]) -> IndentStyle {
    let leads = &leads[..leads.len().min(MAX_LINES_TO_CHECK)];
    let mut spaces = 0usize;
    let mut tabs = 0usize;
    for lead in leads {
        match lead.as_ref().chars().next() {
            Some('\t') => tabs += 1,
            Some(' ') => spaces += 1,
            _ => {}
        }
    }
    let mut style = IndentStyle::unknown();
    // A clear difference: four times as many lines.
    if spaces > tabs * 4 {
        style.character = IndentChar::Spaces;
    } else if tabs > spaces * 4 {
        style.character = IndentChar::Tabs;
    }
    if style.character == IndentChar::Spaces {
        // The most frequent width of the difference to the line before
        // (https://heathermoor.medium.com/detecting-code-indentation-eff3ed0fb56b).
        let mut counts = [0usize; TAB_WIDTH_MAX as usize + 1];
        let mut last = 0usize;
        for lead in leads {
            let lead = lead.as_ref();
            if lead.is_empty() {
                continue;
            }
            let width = lead.bytes().take_while(|&b| b == b' ').count();
            let diff = width.abs_diff(last);
            if diff > 1 && diff < counts.len() {
                counts[diff] += 1;
            }
            last = width;
        }
        // The first of the most frequent, from the smallest width.
        let mut best = 0;
        for (width, &n) in counts.iter().enumerate().skip(1) {
            if n > best {
                best = n;
                style.tab_space = width as i32;
            }
        }
    }
    style
}

// ---- .editorconfig ----

#[derive(Default)]
struct Section {
    name: String,
    /// `Some(true)`: tab, `Some(false)`: space.
    indent_tab: Option<bool>,
    /// `-1`: `tab`.
    indent_size: Option<i32>,
    tab_width: Option<i32>,
}

#[derive(Default)]
struct EditorConfig {
    root: bool,
    sections: Vec<Section>,
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The regex `^\s*(\w+)\s*=\s*(\w+)\s*$` of `rePropLine`: the key and the value of a line.
fn property(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_matches(|c: char| c.is_ascii_whitespace());
    let key_end = line.find(|c| !is_word(c))?;
    let (key, rest) = line.split_at(key_end);
    if key.is_empty() {
        return None;
    }
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let rest = rest.strip_prefix('=')?;
    let value = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
    (!value.is_empty() && value.chars().all(is_word)).then_some((key, value))
}

fn parse_int(value: &str) -> Option<i32> {
    // `from_chars`: a whole decimal number, with a sign for a minus only.
    if value.starts_with('+') {
        return None;
    }
    value.parse().ok()
}

fn parse_editorconfig(bytes: &[u8]) -> EditorConfig {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let text = String::from_utf8_lossy(bytes);
    let mut config = EditorConfig::default();
    for raw in text.lines() {
        let line = raw.trim_matches([' ', '\t']);
        if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
            config.sections.push(Section {
                name: line[1..line.len() - 1].to_owned(),
                ..Section::default()
            });
        } else if line.starts_with('#') || line.starts_with(';') {
            // a comment
        } else if let Some((key, value)) = property(line) {
            let key = key.to_ascii_lowercase();
            if let Some(section) = config.sections.last_mut() {
                match key.as_str() {
                    "indent_style" => {
                        if value.eq_ignore_ascii_case("tab") {
                            section.indent_tab = Some(true);
                        } else if value.eq_ignore_ascii_case("space") {
                            section.indent_tab = Some(false);
                        }
                    }
                    "indent_size" => {
                        if value.eq_ignore_ascii_case("tab") {
                            section.indent_size = Some(-1);
                        } else if let Some(n) = parse_int(value) {
                            section.indent_size = Some(n);
                        }
                    }
                    "tab_width" => {
                        if let Some(n) = parse_int(value) {
                            section.tab_width = Some(n);
                        }
                    }
                    _ => {}
                }
            } else if key == "root" {
                config.root = value.eq_ignore_ascii_case("true");
            }
        }
    }
    config
}

/// `glob_matches_extension`: `*.ext` and `*.{ext1,ext2}`; `extension` has its dot.
fn glob_matches_extension(glob: &str, extension: &str) -> bool {
    let glob = glob.to_lowercase();
    let ext = extension.to_lowercase();
    if glob == format!("*{ext}") {
        return true;
    }
    if let Some(list) = glob.strip_prefix("*.{").and_then(|g| g.strip_suffix('}')) {
        let wanted = ext.strip_prefix('.').unwrap_or(&ext);
        return list
            .split(',')
            .any(|candidate| !candidate.is_empty() && candidate == wanted);
    }
    false
}

/// `ReadEditorConfig`: the style the `.editorconfig` files above `path` give, if any. The
/// directories are searched from the file's up to the one with `root = true`.
fn read_editorconfig(path: &Path) -> Option<IndentStyle> {
    let extension = format!(".{}", path.extension()?.to_str()?);
    for dir in path
        .ancestors()
        .skip(1)
        .filter(|d| !d.as_os_str().is_empty())
    {
        let Ok(bytes) = std::fs::read(dir.join(".editorconfig")) else {
            continue;
        };
        let config = parse_editorconfig(&bytes);
        for section in &config.sections {
            if !glob_matches_extension(&section.name, &extension) {
                continue;
            }
            // The first section for the extension decides, also when it gives nothing.
            return match section.indent_tab {
                Some(true) => Some(IndentStyle {
                    character: IndentChar::Tabs,
                    // No width: the width in use stays.
                    tab_space: section.tab_width.or(section.indent_size).unwrap_or(-1),
                }),
                Some(false) => {
                    // Without a width the spaces cannot be turned on: the text decides.
                    let size = section.indent_size?;
                    let width = if size == -1 {
                        section.tab_width.unwrap_or(size)
                    } else {
                        size
                    };
                    Some(IndentStyle {
                        character: IndentChar::Spaces,
                        tab_space: width,
                    })
                }
                None => None,
            };
        }
        if config.root {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests;
