// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for UME Editor.
//
// Original files: sakura_core/types/CType_Html.cpp
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

//! The outline of an HTML or XML document: its tags, nested by their start and end tags, with the
//! headings and paragraphs of HTML kept in order.
//!
//! Sakura: `CDocOutline::MakeTopicList_html` in `sakura_core/types/CType_Html.cpp`
//! (2003.05.20 zenryaku; 2004.04.19 empty elements; 2004.04.20 Moca comments and unknown end tags;
//! 2008.08.15 aroka headings and paragraphs; 2014.12.25 Moca XML mode and CDATA).

use crate::chars::{at, is_line_delimiter};
use crate::{FuncInfoArr, LineSource};

/// The deepest nesting (`nMaxStack`; 2014.12.25 32 to 64).
const MAX_STACK: usize = 64;
/// The size of `szTitle`, in characters with the NUL.
const TITLE_SIZE: usize = 32;

/// `ELabelType`: what the tag is for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Label {
    Default,
    /// A physical or inline element: not part of the structure.
    Inline,
    /// An empty element that does not matter for the structure.
    Ignore,
    /// An empty element that does.
    Empty,
    Block,
    Paragraph,
    Heading,
}

fn is_in(tag: &str, names: &[&str]) -> bool {
    names.contains(&tag)
}

/// The kind of a tag (HTML only; XML has no kinds).
fn label_of(tag: &str, title: &[char]) -> Label {
    let mut label = Label::Default;
    // physical elements (they only change the looks)
    if is_in(
        tag,
        &[
            "b", "big", "blink", "font", "i", "marquee", "nobr", "s", "small", "strike", "tt", "u",
            "bdo", "sub", "sup",
        ],
    ) {
        label = Label::Inline;
    }
    // ruby elements (XHTML 1.1)
    if is_in(tag, &["rbc", "rtc", "ruby", "rb", "rt", "rp"]) {
        label = Label::Inline;
    }
    // empty elements that do not matter for the structure
    if is_in(tag, &["br", "base", "basefont", "frame", "wbr"]) {
        label = Label::Ignore;
    }
    // empty elements that do
    if is_in(
        tag,
        &[
            "area", "hr", "img", "input", "link", "meta", "param", "col", "command", "embed",
            "keygen", "source", "track",
        ],
    ) {
        label = Label::Empty;
    }
    if is_in(
        tag,
        &[
            "div",
            "center",
            "address",
            "blockquote",
            "noscript",
            "noframes",
            "ol",
            "ul",
            "dl",
            "dir",
            "menu",
            "pre",
            "table",
            "form",
            "fieldset",
            "isindex",
        ],
    ) {
        label = Label::Block;
    }
    if tag == "p" {
        label = Label::Paragraph;
    }
    if tag.starts_with('h') && ('1'..='6').contains(&at(title, 1)) {
        label = Label::Heading;
    }
    label
}

/// A tag name stops at a blank, at an ASCII character that is not `_:-.` or alphanumeric, and a
/// digit, `-` or `.` cannot start it.
fn ends_tag_name(c: char, j: usize) -> bool {
    c == ' '
        || c == '\t'
        || ((c as u32) < 0x80 && !"_:-.".contains(c) && !c.is_ascii_alphanumeric())
        || (j == 0 && (c.is_ascii_digit() || c == '-' || c == '.'))
}

fn same_name(a: &[char], b: &[char], xml: bool) -> bool {
    if xml {
        a == b
    } else {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.eq_ignore_ascii_case(y))
    }
}

/// Lists the tags of an HTML document, or of an XML document with `xml` (names are case sensitive
/// there, and a CDATA section is skipped). The depth of an item is the number of tags it is
/// inside; an item is the name of the tag and the text up to the next tag.
pub fn make_topic_list_html(src: &dyn LineSource, xml: bool) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let mut in_comment = false;
    let mut in_cdata = false;
    let mut in_paragraph = false;
    let mut depth = 0usize;
    let mut stack: Vec<Vec<char>> = vec![Vec::new(); MAX_STACK];
    // The depth of the last heading of each level, until a heading of a higher level comes
    // (2008.08.15 aroka). Index 0 is not used.
    let mut head_depth = [-1isize; 7];

    let mut n = 0usize;
    while n < src.line_count() {
        let Some(mut line) = src.line(n) else {
            break;
        };
        let mut i = 0usize;
        while (line.len() as isize) - 1 > i as isize {
            let len = line.len() as isize;
            // 2004.04.20 Moca: comments
            if in_comment {
                if (i as isize) < len - 3 && crate::chars::starts_at(&line, i, "-->") {
                    in_comment = false;
                    i += 2;
                }
                i += 1;
                continue;
            }
            // 2014.12.25 Moca: CDATA
            if in_cdata {
                if (i as isize) < len - 3 && crate::chars::starts_at(&line, i, "]]>") {
                    in_cdata = false;
                    i += 2;
                }
                i += 1;
                continue;
            }
            if line[i] != '<' || depth >= MAX_STACK {
                i += 1;
                continue;
            }
            let mut end_tag = false;
            i += 1;
            if at(&line, i) == '/' {
                i += 1;
                end_tag = true;
            }
            // cut out the name of the tag
            let mut title: Vec<char> = Vec::new();
            let mut j = 0usize;
            while i + j < line.len() && j < TITLE_SIZE - 1 {
                let c = line[i + j];
                if ends_tag_name(c, j) {
                    break;
                }
                title.push(c);
                j += 1;
            }
            if j == 0 {
                // 2004.04.20 Moca: the start of a comment
                if (i as isize) < len - 3 && crate::chars::starts_at(&line, i, "!--") {
                    in_comment = true;
                    i += 3;
                }
                // 2014.12.25 Moca: CDATA
                if xml && (i as isize) < len - 8 && crate::chars::starts_at(&line, i, "![CDATA[") {
                    in_cdata = true;
                    i += 8;
                }
                i += 1;
                continue;
            }
            // sorted once, in lower case for HTML
            let tag: String = if xml {
                title.iter().collect()
            } else {
                title.iter().collect::<String>().to_lowercase()
            };
            let label = if xml {
                Label::Default
            } else {
                label_of(&tag, &title)
            };

            // 2009.08.13 syat: the end tag is handled after the start tag, because a start tag
            // that ends in "/>" is an end tag too
            if !end_tag && label != Label::Inline && label != Label::Ignore {
                // a block element in a paragraph closes it (2008.09.07 aroka)
                if in_paragraph && matches!(label, Label::Heading | Label::Paragraph | Label::Block)
                {
                    depth = depth.saturating_sub(1);
                }
                if label == Label::Heading {
                    let level = (at(&title, 1) as u8 - b'0') as usize;
                    if head_depth[level] != -1 {
                        // a lower heading that was seen: the same depth
                        depth = head_depth[level] as usize;
                        for slot in head_depth.iter_mut().skip(level) {
                            *slot = -1;
                        }
                        head_depth[level] = depth as isize;
                        in_paragraph = false;
                    }
                }
                if label == Label::Paragraph {
                    in_paragraph = true;
                }
                if label == Label::Block {
                    in_paragraph = false;
                }
                if label != Label::Empty {
                    // every tag but the ones without an end tag
                    stack[depth] = title.clone();
                    let mut k = j;
                    let x = j;
                    // 2014.12.25: a tag over lines, and "/>"
                    let mut has_line = true;
                    let mut keep_going = true;
                    while keep_going && has_line {
                        let limit = line.len().saturating_sub(i);
                        while j < limit {
                            if at(&line, i + j) == '/' && at(&line, i + j + 1) == '>' {
                                end_tag = true; // <emptytag />
                                keep_going = false;
                                j += 2;
                                break;
                            } else if at(&line, i + j) == '>' {
                                keep_going = false;
                                j += 1;
                                break;
                            }
                            j += 1;
                        }
                        if keep_going {
                            n += 1;
                            match src.line(n) {
                                Some(next) => line = next,
                                None => {
                                    // no more lines: the length is 0 and the line is null
                                    line = Vec::new();
                                    has_line = false;
                                }
                            }
                            j = 0;
                            i = 0;
                        }
                    }
                    if x < TITLE_SIZE - 3 && !end_tag {
                        title.truncate(k.min(title.len()));
                        title.push(' ');
                        k += 1;
                        while i + j < line.len() && k < TITLE_SIZE - 1 {
                            let c = line[i + j];
                            if c == '<' || is_line_delimiter(c) {
                                j = j.wrapping_sub(1);
                                break;
                            }
                            title.push(c);
                            k += 1;
                            j += 1;
                        }
                    }
                    title.truncate(k);
                    out.push(n + 1, title.iter().collect::<String>(), 0, depth);
                    depth += 1;
                } else {
                    while i + j < line.len() && j < TITLE_SIZE - 1 {
                        let c = line[i + j];
                        if c == '>' {
                            break;
                        }
                        if title.len() <= j {
                            title.push(c);
                        } else {
                            title[j] = c;
                        }
                        j += 1;
                    }
                    title.truncate(j);
                    out.push(n + 1, title.iter().collect::<String>(), 0, depth);
                }
            }
            if end_tag {
                let depth_org = depth;
                while depth > 0 {
                    depth -= 1;
                    if same_name(&stack[depth], &title, xml) {
                        break;
                    }
                }
                // 2004.04.20 Moca: an end tag that matches nothing in the tree is ignored
                if depth == 0 {
                    if !same_name(&stack[0], &title, xml) {
                        depth = depth_org;
                    }
                } else {
                    if label == Label::Heading {
                        // the end of a heading
                        let level = (at(&title, 1) as u8 - b'0') as usize;
                        head_depth[level] = depth as isize;
                        depth += 1;
                    }
                    if label == Label::Paragraph {
                        in_paragraph = false;
                    }
                }
            }
            // `j` may have gone back by one to the "<" that ends the text of the item
            i = ((i as isize) + (j as isize) + 1) as usize;
        }
        n += 1;
    }
    out
}
