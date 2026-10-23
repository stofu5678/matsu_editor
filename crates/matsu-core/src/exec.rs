// Ported from Sakura Editor and modified for MATSU Editor (Rust rewrite, changed from the C++ original).
//
// Original files:
//   sakura_core/view/CEditView_ExecCmd.cpp (CEditView::ExecCmd: the option bits, the encodings, the
//   rules that keep a character or a line break from being cut between two reads, the extra
//   information and its words), sakura_core/dlg/CDlgExec.cpp (the tables of the encodings)
//
// Original copyright notices of those files:
//   Copyright (C) 1998-2002, Norio Nakatani
//   Copyright (C) 2000, genta, JEPRO, MIK
//   Copyright (C) 2001, genta, GAE, MIK, hor, asa-o, Stonee, Misaka, novice, YAZAKI
//   Copyright (C) 2002, YAZAKI, hor, aroka, MIK, Moca, minfu, KK, novice, ai, Azumaiya, genta
//   Copyright (C) 2003, MIK, ai, ryoji, Moca, wmlhq, genta
//   Copyright (C) 2004, genta, Moca, novice, naoh, isearch, fotomo
//   Copyright (C) 2005, genta, MIK, novice, aroka, D.S.Koba, かろと, Moca
//   Copyright (C) 2006, Moca, aroka, ryoji, fon, genta
//   Copyright (C) 2007, ryoji, じゅうじ, maru
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

//! The rules of 外部コマンド実行 (`F_EXECMD`, `F_EXECMD_DIALOG`) that need no window and no
//! process: the option bits, the encodings of the output and of the input, the text that is
//! written around the output, and the reading and writing of the text in pieces.
//!
//! * [`ExecFlags`]: the sum of bits that the dialog keeps (`m_nExecFlgOpt`) and the macro passes.
//! * [`OutputDecoder`]: the bytes of the command's output, in pieces as they are read, become
//!   text. A character, or a CR that may be followed by an LF, that is cut at the end of a piece
//!   waits for the next one.
//! * [`feed_text`]: the document, or a part of it, read by lines and encoded in pieces for the
//!   standard input of the command (Sakura writes a temporary file of the whole text; here
//!   nothing is made as one string).
//!
//! The process itself is `matsu_platform::exec`; the window is `matsu_ui`.

use crate::buffer::{BufferError, Document, Pos};
use crate::encoding::Encoding;

/// The sum of bits of the option (`nFlgOpt` of `CEditView::ExecCmd`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExecFlags(pub i32);

impl ExecFlags {
    /// 標準出力を得る.
    pub const GET_STDOUT: i32 = 0x01;
    /// 標準出力のリダイレクト先: the window being edited (not the output window).
    pub const TO_WINDOW: i32 = 0x02;
    /// 編集中ファイルを標準入力へ.
    pub const SEND_STDIN: i32 = 0x04;
    /// 標準出力をUnicodeで行う.
    pub const OUT_UNICODE: i32 = 0x08;
    /// 標準入力をUnicodeで行う.
    pub const IN_UNICODE: i32 = 0x10;
    /// 情報出力する.
    pub const EXT_INFO: i32 = 0x20;
    /// 情報出力しない.
    pub const NO_EXT_INFO: i32 = 0x40;
    /// 標準出力をUTF-8で行う.
    pub const OUT_UTF8: i32 = 0x80;
    /// 標準入力をUTF-8で行う.
    pub const IN_UTF8: i32 = 0x100;
    /// カレントディレクトリを指定.
    pub const CUR_DIR: i32 = 0x200;

    /// The value the shared data starts with: only 標準出力を得る (`m_nExecFlgOpt = 1`,
    /// `CShareData::InitShareData`).
    pub const DEFAULT: ExecFlags = ExecFlags(Self::GET_STDOUT);

    fn has(self, bit: i32) -> bool {
        self.0 & bit != 0
    }

    pub fn get_stdout(self) -> bool {
        self.has(Self::GET_STDOUT)
    }

    /// Whether the output goes into the window: asked for, and the document can be edited (a
    /// document in view mode gets its output in the output window, `bEditable` in `ExecCmd`).
    pub fn to_window(self, editable: bool) -> bool {
        self.has(Self::TO_WINDOW) && editable
    }

    pub fn send_stdin(self) -> bool {
        self.has(Self::SEND_STDIN)
    }

    /// The encoding of the output: 0x08 before 0x80, else Shift_JIS.
    pub fn output_encoding(self) -> Encoding {
        if self.has(Self::OUT_UNICODE) {
            Encoding::Utf16Le
        } else if self.has(Self::OUT_UTF8) {
            Encoding::Utf8
        } else {
            Encoding::ShiftJis
        }
    }

    /// The encoding of the input: 0x10 before 0x100, else Shift_JIS.
    pub fn input_encoding(self) -> Encoding {
        if self.has(Self::IN_UNICODE) {
            Encoding::Utf16Le
        } else if self.has(Self::IN_UTF8) {
            Encoding::Utf8
        } else {
            Encoding::ShiftJis
        }
    }

    /// Whether the command line, the date and the end are written around the output: by default
    /// when the output is not in the window; 0x20 asks for it and 0x40 (after it) refuses
    /// (2010/04/12 Moca).
    pub fn extra_info(self, to_window: bool) -> bool {
        let mut info = !to_window;
        if self.has(Self::EXT_INFO) {
            info = true;
        }
        if self.has(Self::NO_EXT_INFO) {
            info = false;
        }
        info
    }

    /// Whether the folder that comes with the command is used (0x200).
    pub fn uses_cur_dir(self) -> bool {
        self.has(Self::CUR_DIR)
    }

    /// What the dialog shows first (`CDlgExec::SetData`): a document that cannot be edited does
    /// not offer the window (`nExecFlgOpt &= ~0x02`, 2009/02/21 ryoji).
    pub fn for_dialog(self, editable: bool) -> ExecFlags {
        if editable {
            self
        } else {
            ExecFlags(self.0 & !Self::TO_WINDOW)
        }
    }
}

/// The encodings of the two combo boxes of the dialog, in the order of the boxes
/// (`codes[]` of `CDlgExec::OnInitDialog`: Shift_JIS, Unicode, UTF-8).
pub const DIALOG_ENCODINGS: [Encoding; 3] = [Encoding::ShiftJis, Encoding::Utf16Le, Encoding::Utf8];

/// The bits of the output encoding for each item of the first combo box (`codeTable1`).
pub const OUTPUT_BITS: [i32; 3] = [0x00, ExecFlags::OUT_UNICODE, ExecFlags::OUT_UTF8];
/// The bits of the input encoding for each item of the second combo box (`codeTable2`).
pub const INPUT_BITS: [i32; 3] = [0x00, ExecFlags::IN_UNICODE, ExecFlags::IN_UTF8];

/// The item of the first combo box for `flags` (`m_nExecFlgOpt & 0x88`; the first when the bits
/// are none of the three, as the box keeps what it had).
pub fn output_choice(flags: ExecFlags) -> usize {
    OUTPUT_BITS
        .iter()
        .position(|&bits| bits == flags.0 & 0x88)
        .unwrap_or(0)
}

/// The item of the second combo box for `flags` (`m_nExecFlgOpt & 0x110`).
pub fn input_choice(flags: ExecFlags) -> usize {
    INPUT_BITS
        .iter()
        .position(|&bits| bits == flags.0 & 0x110)
        .unwrap_or(0)
}

/// `STR_EDITVIEW_EXECCMD_STOP`, with the line breaks as `eol`.
pub fn stopped_text(eol: &str) -> String {
    format!("{eol}中断しました。{eol}")
}

/// `STR_EDITVIEW_EXECCMD_RET`, with the line breaks as `eol`.
pub fn exit_code_text(code: i32, eol: &str) -> String {
    format!("{eol}終了コード: {code}{eol}")
}

/// The lines that come before the output when the extra information is on: a line of `=`, the
/// date and time, the command line, a line of `=`. `date` and `time` are already formatted.
pub fn header_text(date: &str, time: &str, command: &str, eol: &str) -> String {
    let rule = "#============================================================";
    format!("{eol}{rule}{eol}#DateTime : {date} {time}{eol}#CmdLine  : {command}{eol}{rule}{eol}")
}

/// The most that `ExpandParameter` keeps of the command line (`bufmax = 1024` characters of
/// `Command_EXECCOMMAND`, in UTF-16 units).
pub const MAX_COMMAND_UNITS: usize = 1024;

/// Cuts `text` to at most [`MAX_COMMAND_UNITS`] UTF-16 units, not in the middle of a character.
pub fn limit_command(text: &str) -> &str {
    let mut units = 0;
    for (at, c) in text.char_indices() {
        units += c.len_utf16();
        if units > MAX_COMMAND_UNITS {
            return &text[..at];
        }
    }
    text
}

/// Turns the output of a command into text, a piece at a time.
///
/// A piece of bytes can end in the middle of a character, or between the CR and the LF of a line
/// break. Sakura keeps those last bytes for the next read (`bufidx` in `ExecCmd`); this does the
/// same, so a line break is never split in two and no character becomes two bad ones. Shift_JIS,
/// UTF-8 and UTF-16 (little endian) are the three that the dialog offers; other encodings are
/// decoded as they come, without waiting for the rest of a character.
#[derive(Debug)]
pub struct OutputDecoder {
    encoding: Encoding,
    waiting: Vec<u8>,
    /// Nothing has been given out yet: a BOM at the start is not text.
    at_start: bool,
}

impl OutputDecoder {
    pub fn new(encoding: Encoding) -> Self {
        Self {
            encoding,
            waiting: Vec::new(),
            at_start: true,
        }
    }

    /// The text of the bytes read so far; what may continue in the next piece waits.
    pub fn push(&mut self, bytes: &[u8]) -> String {
        self.waiting.extend_from_slice(bytes);
        let keep = held_len(self.encoding, &self.waiting);
        let ready = self.waiting.len() - keep;
        let text = self.decode(&self.waiting[..ready]);
        self.waiting.drain(..ready);
        self.without_bom(text)
    }

    /// A program that writes the BOM of UTF-8 or UTF-16 (a filter that was fed the BOM of the
    /// document, for one) has it taken off once, so that it does not become a character in the
    /// text.
    fn without_bom(&mut self, text: String) -> String {
        if text.is_empty() || !self.at_start {
            return text;
        }
        self.at_start = false;
        match text.strip_prefix('\u{FEFF}') {
            Some(rest) if self.encoding.can_have_bom() => rest.to_owned(),
            _ => text,
        }
    }

    /// The end of the output: what waited (`最後の文字の出力`). A last odd byte of UTF-16 gets a
    /// zero high byte.
    pub fn finish(&mut self) -> String {
        let mut rest = std::mem::take(&mut self.waiting);
        if rest.len() % 2 == 1 {
            match self.encoding {
                Encoding::Utf16Le => rest.push(0),
                Encoding::Utf16Be => rest.insert(0, 0),
                _ => {}
            }
        }
        let text = self.decode(&rest);
        self.without_bom(text)
    }

    fn decode(&self, bytes: &[u8]) -> String {
        let mut text = String::new();
        if !bytes.is_empty() {
            self.encoding.codec().decode(bytes, &mut text);
        }
        text
    }
}

/// How many bytes at the end of `bytes` wait for the next piece.
fn held_len(encoding: Encoding, bytes: &[u8]) -> usize {
    let n = bytes.len();
    match encoding {
        Encoding::Utf16Le | Encoding::Utf16Be => {
            let big = encoding == Encoding::Utf16Be;
            let mut keep = n % 2;
            let whole = n - keep;
            if whole >= 2 {
                let (a, b) = (bytes[whole - 2], bytes[whole - 1]);
                let unit = if big {
                    u16::from_be_bytes([a, b])
                } else {
                    u16::from_le_bytes([a, b])
                };
                // the first half of a surrogate pair, or a CR that an LF may follow
                if (0xD800..0xDC00).contains(&unit) || unit == 0x000D {
                    keep += 2;
                }
            }
            keep
        }
        Encoding::Utf8 => {
            let cut = utf8_cut(bytes);
            if cut > 0 {
                cut
            } else {
                usize::from(bytes.last() == Some(&b'\r'))
            }
        }
        Encoding::ShiftJis => {
            let mut i = 0;
            while i < n {
                if is_sjis_lead(bytes[i]) {
                    if i + 1 == n {
                        return 1;
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            usize::from(bytes.last() == Some(&b'\r'))
        }
        _ => 0,
    }
}

/// The length of an unfinished UTF-8 character at the end of `bytes` (0 if it ends cleanly or in
/// bytes that can never be finished).
fn utf8_cut(bytes: &[u8]) -> usize {
    let n = bytes.len();
    for back in 1..=3.min(n) {
        let b = bytes[n - back];
        if b & 0xC0 == 0x80 {
            continue;
        }
        let need = match b {
            0xC2..=0xDF => 2,
            0xE0..=0xEF => 3,
            0xF0..=0xF4 => 4,
            _ => return 0,
        };
        return if need > back { back } else { 0 };
    }
    0
}

/// The first byte of a two-byte Shift_JIS character (`_IS_SJIS_1`).
fn is_sjis_lead(b: u8) -> bool {
    matches!(b, 0x81..=0x9F | 0xE0..=0xFC)
}

/// How much text is encoded and handed over at a time.
pub const FEED_CHUNK_BYTES: usize = 64 * 1024;

/// Reads the document by lines and hands it over, encoded, in pieces of about
/// [`FEED_CHUNK_BYTES`]: the standard input of the command. `range` is the part to send (the
/// selection), else the whole document. `bom` writes the BOM of the encoding first. `sink` takes a
/// piece and says whether to go on (`false`: the command no longer reads).
///
/// Characters that the encoding cannot write become what Sakura writes for them (`?`); the
/// document is not made into one string at any time.
pub fn feed_text(
    doc: &Document,
    range: Option<(Pos, Pos)>,
    encoding: Encoding,
    bom: bool,
    mut sink: impl FnMut(Vec<u8>) -> bool,
) -> Result<(), BufferError> {
    if bom && encoding.can_have_bom() && !sink(encoding.bom().to_vec()) {
        return Ok(());
    }
    let codec = encoding.codec();
    let (first, last, from, to) = match range {
        Some((from, to)) => {
            let (from, to) = if (from.line, from.col) <= (to.line, to.col) {
                (from, to)
            } else {
                (to, from)
            };
            (from.line, to.line, Some(from), Some(to))
        }
        None => (0, doc.line_count().saturating_sub(1), None, None),
    };
    let mut text = String::new();
    let mut send = |text: &mut String| -> bool {
        let mut bytes = Vec::new();
        // the report of the characters that cannot be written is not needed here
        let _ = codec.encode(text, &mut bytes);
        text.clear();
        bytes.is_empty() || sink(bytes)
    };
    for index in first..=last {
        let Some(line) = doc.line(index)? else { break };
        let start = from.filter(|f| f.line == index).map_or(0, |f| f.col);
        let end = to
            .filter(|t| t.line == index)
            .map_or(line.text.len(), |t| t.col);
        text.push_str(line.text.get(start..end.max(start)).unwrap_or(""));
        let last_line = to.is_some_and(|t| t.line == index);
        if !last_line && let Some(eol) = line.eol {
            text.push_str(eol.as_str());
        }
        if text.len() >= FEED_CHUNK_BYTES && !send(&mut text) {
            return Ok(());
        }
    }
    if !text.is_empty() {
        send(&mut text);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
