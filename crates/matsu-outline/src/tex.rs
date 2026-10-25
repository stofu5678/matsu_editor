// Ported from Sakura Editor (https://github.com/sakura-editor/sakura) and modified for MATSU Editor.
//
// Original files: sakura_core/types/CType_Tex.cpp
//
// Original copyright notices of those files:
//   Copyright (C) 2008, kobake
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

//! The outline of a TeX source: `\chapter`, `\section`, `\subsection` and `\subsubsection`, a tree
//! with numbered topics.
//!
//! Sakura: `CDocOutline::MakeTopicList_tex` and its helpers `TagProcessor` and `TagIterator` in
//! `sakura_core/types/CType_Tex.cpp` (naoh, 2003.07.21; 2005.01.03 the slide of prosper).

use crate::{FuncInfoArr, LineSource};

/// The tags that make topics, the biggest structure first (`TagHierarchy`).
const TAG_HIERARCHY: [&str; 4] = ["chapter", "section", "subsection", "subsubsection"];

/// The size of the text of a topic, with its number (`StaticString<256> szTopic`), in characters,
/// with the room for the terminating null.
const TOPIC_BUFFER: usize = 256;

/// What `TagProcessor` remembers between the tags.
struct TagProcessor {
    /// The depth of the previous tag (`tagDepth`).
    tag_depth: usize,
    /// The depth of the previous tag in the tree (`treeDepth`).
    tree_depth: isize,
    /// The topic number last given at each depth (`serials`).
    serials: [usize; TAG_HIERARCHY.len()],
}

/// Where a tag and its title are in a line (`pTag`, `pTagEnd`, `pTitle`, `pTitleEnd`).
struct TagSpan {
    tag: usize,
    tag_end: usize,
    title: usize,
    title_end: usize,
}

impl TagProcessor {
    fn new() -> Self {
        Self {
            tag_depth: 0,
            tree_depth: -1,
            serials: [0; TAG_HIERARCHY.len()],
        }
    }

    /// `TagProcessor::operator()`: takes `\tag{title}` and adds the topic of the tag. Returns where
    /// the scan of the line goes on (the end of the title).
    fn process(
        &mut self,
        out: &mut FuncInfoArr,
        line_number: usize,
        line: &[char],
        span: TagSpan,
    ) -> usize {
        let TagSpan {
            tag,
            mut tag_end,
            mut title,
            mut title_end,
        } = span;
        let line_end = line.len();
        // a tag that ends with `*` has no topic number
        let add_number = tag < tag_end && line[tag_end - 1] != '*';
        if !add_number {
            tag_end -= 1;
        }

        // the depth of the tag
        let tag_name = &line[tag..tag_end];
        let mut depth = TAG_HIERARCHY
            .iter()
            .rposition(|name| name.chars().eq(tag_name.iter().copied()));
        if depth.is_none() {
            // `wcsncmp(L"begin", pTag, pTagEnd - pTag) == 0`: the tag is `begin` (or the start
            // of it, as the comparison is limited to the length of the tag). It may be the
            // slide of prosper: `\begin{slide}{title}` has the title in the second braces.
            let is_begin = tag_name.len() <= "begin".len()
                && "begin".chars().zip(tag_name).all(|(a, &b)| a == b)
                && !tag_name.is_empty();
            if is_begin && let Some(slide) = find_str(line, tag_end, "{slide}") {
                title = line[slide + 7..]
                    .iter()
                    .position(|&c| c == '{')
                    .map_or(line_end, |i| slide + 7 + i + 1);
                title_end = line[title..]
                    .iter()
                    .position(|&c| c == '}')
                    .map_or(line_end, |i| title + i);
                if title < title_end {
                    depth = Some(0);
                }
            }
        }
        let Some(depth) = depth else {
            // not a tag of a topic
            return title_end;
        };

        // 1. the depth in the tree grows by one at most
        if depth <= self.tag_depth {
            self.tree_depth -= (self.tag_depth - depth) as isize;
        } else {
            self.tree_depth += 1;
        }
        // the topic was put higher than its own depth when the growth was held to one: put it right
        for i in depth..self.tag_depth {
            if self.serials[i] == 0 {
                self.tree_depth += 1;
            }
        }
        if self.tree_depth < 0 {
            // the first topic, or a chapter after the topics of a deeper tag
            self.tree_depth = 0;
        }

        // 2. the topic numbers
        self.serials[depth] += 1;
        for i in depth + 1..=self.tag_depth {
            if i < self.serials.len() {
                self.serials[i] = 0;
            }
        }

        // 3. the depth of the tag
        self.tag_depth = depth;

        // the text of the topic: the number ("1.", "1.2."), then the title
        let mut topic = String::new();
        if add_number {
            let mut i = 0;
            // no "0." prefixes
            while i <= self.tag_depth && self.serials[i] == 0 {
                i += 1;
            }
            while i <= self.tag_depth && i < self.serials.len() {
                // "1.", "2.", ..., "999.", then "000.", "001."
                let n = self.serials[i];
                if n / 1000 != 0 {
                    topic.push_str(&format!("{:03}.", n % 1000));
                } else {
                    topic.push_str(&format!("{}.", n % 1000));
                }
                i += 1;
            }
            topic.push(' ');
        }
        let room = (TOPIC_BUFFER - 1).saturating_sub(topic.chars().count());
        topic.extend(line[title..title_end].iter().take(room));

        out.push(line_number + 1, topic, 0, self.tree_depth as usize);
        title_end
    }
}

/// `wcsstr`: the first place at or after `from` where `needle` is in `line`.
fn find_str(line: &[char], from: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    if from > line.len() || line.len() - from < needle.len() {
        return None;
    }
    (from..=line.len() - needle.len()).find(|&i| line[i..i + needle.len()] == needle[..])
}

/// `MakeTopicList_tex`.
pub fn make_topic_list_tex(src: &dyn LineSource) -> FuncInfoArr {
    let mut out = FuncInfoArr::new();
    let mut processor = TagProcessor::new();
    // `TagIterator::each`
    for n in 0..src.line_count() {
        let Some(line) = src.line(n) else {
            break;
        };
        let line_end = line.len();
        let mut p = 0;
        while let Some(found) = line[p..]
            .iter()
            .position(|&c| c == '\\' || c == '%')
            .map(|i| p + i)
        {
            if line[found] == '%' {
                // a comment: the rest of the line is not read
                break;
            }
            // from the `\`, the tag and the title are found by the braces
            let tag = found + 1;
            let tag_end = line[tag..]
                .iter()
                .position(|&c| c == '{')
                .map_or(line_end, |i| tag + i);
            let title = if tag_end < line_end {
                tag_end + 1
            } else {
                line_end
            };
            let title_end = line[title..]
                .iter()
                .position(|&c| c == '}')
                .map_or(line_end, |i| title + i);
            if tag < tag_end && title < title_end && title_end < line_end {
                let span = TagSpan {
                    tag,
                    tag_end,
                    title,
                    title_end,
                };
                p = processor.process(&mut out, n, &line, span);
                if p < tag || line_end < p {
                    // an invalid value, or a loop that would not end: the analysis stops
                    return out;
                }
            } else {
                p = found + 1;
            }
        }
    }
    out
}
