// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/cmd/CViewCommander_Clipboard.cpp (AppendHTMLColor and the loop over the
//   characters of a row in Command_COPY_COLOR_HTML)
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2000-2001, jepro, genta
// Copyright (C) 2001, novice
// Copyright (C) 2002, hor, genta, Azumaiya, すなふき
// Copyright (C) 2004, Moca
// Copyright (C) 2005, genta
// Copyright (C) 2007, ryoji
// Copyright (C) 2010, ryoji
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (UTF-16 buffers, the
// color strategies that tell where a color changes) was rewritten in Rust on characters and on
// the style of each character, which the caller works out from the runs of `ume-coloring`.
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

//! The HTML of the colored copy: `<pre>` with `<span>`, `<u>` and `<b>` around the pieces of
//! text, in the nesting and the order of Sakura's `AppendHTMLColor`.
//!
//! This module only writes the tags. Which text is selected and what style each character has is
//! worked out by [`super::collect`]; no window is needed here.

use crate::settings::view_settings::Rgb;

/// How a character is drawn: the colors of the text and the background (`None` is Sakura's
/// `(COLORREF)-1`: no `<span>` is open), and the font.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Attr {
    pub color: Option<(Rgb, Rgb)>,
    pub bold: bool,
    pub underline: bool,
}

impl Attr {
    /// Nothing is open.
    pub(super) const NONE: Self = Self {
        color: None,
        bold: false,
        underline: false,
    };
}

/// A row to write: its characters (with the line ending that ends it, if it does), the style of
/// each, and the part of it that is selected.
pub(super) struct Row {
    pub chars: Vec<char>,
    /// The style of each character; as many as `chars`.
    pub attrs: Vec<Attr>,
    /// The selected part: from the first character, up to the end. `None`: no part of the row
    /// is selected (the row is above the selection).
    pub from: Option<usize>,
    pub to: usize,
}

/// The line delimiters that end a piece of text (`WCODE::IsLineDelimiter` without the extended
/// line endings, which are off by default).
fn is_delimiter(c: char) -> bool {
    matches!(c, '\r' | '\n')
}

/// The HTML that is being written, and the tags that are open.
pub(super) struct Html {
    out: String,
    /// The style of the text to write next (`sColorAttrLast`, `sFontAttrLast`).
    last: Attr,
    /// The style of the tags that are open (`sColorAttrLast2`, `sFontAttrLast2`).
    open: Attr,
}

impl Html {
    /// Starts the `<pre>` with the background of the text color (`COLORIDX_TEXT`).
    pub(super) fn new(back: Rgb) -> Self {
        Self {
            out: format!("<pre style=\"background-color:#{back:06x}\">"),
            last: Attr::NONE,
            open: Attr::NONE,
        }
    }

    /// Writes `text` that needs no tags and no escaping (a line number).
    pub(super) fn raw(&mut self, text: &str) {
        self.out.push_str(text);
    }

    /// Closes the open tags and forgets the style, so that the next text opens them again
    /// (after a line number, `bAddLineNum`).
    pub(super) fn close_tags(&mut self) {
        self.close_open();
        self.last = Attr::NONE;
    }

    fn close_open(&mut self) {
        if self.open.bold {
            self.out.push_str("</b>");
        }
        if self.open.underline {
            self.out.push_str("</u>");
        }
        if self.open.color.is_some() {
            self.out.push_str("</span>");
        }
        self.open = Attr::NONE;
    }

    /// Closes what is open and the `<pre>`, and gives the HTML.
    pub(super) fn finish(mut self) -> String {
        self.close_open();
        self.out.push_str("</pre>");
        self.out
    }

    /// `AppendHTMLColor`: writes `text` in the style `last`, opening and closing the tags that
    /// the change from the style before it needs. Returns whether `text` ends with a line
    /// delimiter.
    fn append(&mut self, text: &[char]) -> bool {
        let (last, open) = (self.last, self.open);
        let font_changed = last.bold != open.bold || last.underline != open.underline;
        let color_changed = last.color != open.color;
        if font_changed || color_changed {
            if open.bold {
                self.out.push_str("</b>");
            }
            if open.underline && (color_changed || last.underline != open.underline) {
                self.out.push_str("</u>");
            }
            if color_changed {
                if open.color.is_some() {
                    self.out.push_str("</span>");
                }
                if let Some((fore, back)) = last.color {
                    self.out.push_str(&format!(
                        "<span style=\"color:#{fore:06x};background-color:#{back:06x}\">"
                    ));
                }
            }
            if last.underline && (color_changed || last.underline != open.underline) {
                self.out.push_str("<u>");
            }
            if last.bold {
                self.out.push_str("<b>");
            }
            self.open = last;
        }
        for &c in text {
            match c {
                '&' => self.out.push_str("&amp;"),
                '<' => self.out.push_str("&lt;"),
                '>' => self.out.push_str("&gt;"),
                _ => self.out.push(c),
            }
        }
        text.last().copied().is_some_and(is_delimiter)
    }

    /// Writes the selected part of a row: the loop over the characters of
    /// `Command_COPY_COLOR_HTML`. A piece of text ends where the style changes, and at the end
    /// of the selection. Returns whether the last piece written ends with a line delimiter
    /// (`bAddCRLF`).
    pub(super) fn row(&mut self, row: &Row) -> bool {
        let mut ends_with_delimiter = false;
        let Some(from) = row.from else {
            return false;
        };
        let (to, len) = (row.to, row.chars.len());
        // The start of the piece that is not written yet.
        let mut begin = from;
        // The style of the character before, to see where the style changes.
        let mut before = row.attrs.first().copied().unwrap_or(Attr::NONE);
        for (i, &next) in row.attrs.iter().enumerate() {
            if from <= i && i <= to {
                if from == i {
                    self.last = next;
                } else if from < i && before != next {
                    ends_with_delimiter = self.append(&row.chars[begin..i]);
                    self.last = next;
                    begin = i;
                } else if to == i {
                    ends_with_delimiter = self.append(&row.chars[begin..i]);
                    begin = i;
                }
            }
            before = next;
        }
        if to == len {
            ends_with_delimiter = self.append(&row.chars[begin..len]);
        }
        ends_with_delimiter
    }
}
