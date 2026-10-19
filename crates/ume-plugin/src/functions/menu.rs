// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/macro/CMacro.cpp (F_CREATEMENU of CMacro::HandleFunction)
//   sakura_core/util/string_ex.cpp (my_strtok)
//
// Copyright (C) 1998-2001, Norio Nakatani
// Copyright (C) 2000-2001, jepro
// Copyright (C) 2001, hor
// Copyright (C) 2002, YAZAKI, aroka, genta, Moca, hor
// Copyright (C) 2003, 鬼, ryoji, Moca
// Copyright (C) 2004, genta, zenryaku
// Copyright (C) 2005, MIK, genta, maru, zenryaku, FILE
// Copyright (C) 2006, かろと, ryoji
// Copyright (C) 2007, ryoji, maru
// Copyright (C) 2008, nasukoji, ryoji
// Copyright (C) 2009, ryoji, nasukoji
// Copyright (C) 2011, syat
// Copyright (C) 2018-2026, Sakura Editor Organization
//
// This file is derived from the files above and was modified: the C++ code (a `WCHAR` buffer
// that `my_strtok` cuts in place, and Win32 menu handles) was rewritten in Rust on UTF-8
// strings and a tree of plain values, and the tests are new.
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

//! The menu string of `CreateMenu`, as a tree.
//!
//! The string is a list of items separated by `,`. An item can start with flags in square
//! brackets (`help/sakura/res/HLP000268.html`, `CreateMenu`): `C` checked, `D` disabled, `R`
//! radio, `S` opens a submenu, `E` closes one submenu (once for each `E`), `B` starts a new
//! column. `-` alone is a separator. The window shows the tree and answers the number of the
//! item the user chose.

/// One entry of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuNode {
    /// An item the user can choose. `number` counts from 1 over the items only: separators and
    /// submenus do not take a number.
    Item {
        label: String,
        number: i64,
        checked: bool,
        disabled: bool,
        /// `R`: the item is drawn with the radio mark instead of the check mark.
        radio: bool,
    },
    Separator,
    Submenu {
        label: String,
        checked: bool,
        disabled: bool,
        children: Vec<MenuNode>,
    },
}

/// Splits `text` at `,` outside double quotes (the quotes stay in the item) and leaves out the
/// empty items (`my_strtok`).
fn tokens(text: &str) -> Vec<Vec<char>> {
    let mut out = Vec::new();
    let mut current = Vec::new();
    let mut quoted = false;
    for c in text.chars() {
        if c == '"' {
            quoted = !quoted;
        }
        if !quoted && c == ',' {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Closes the innermost submenu: its entries become the children of the submenu item that was
/// inserted last into the menu one level up.
fn close_level(levels: &mut Vec<Vec<MenuNode>>) {
    if levels.len() < 2 {
        return;
    }
    let done = levels.pop().unwrap_or_default();
    if let Some(MenuNode::Submenu { children, .. }) = levels.last_mut().and_then(|l| l.last_mut()) {
        *children = done;
    }
}

/// The tree of a menu string (`F_CREATEMENU`). Spaces are not trimmed: `" [E]x"` is an item
/// whose label starts with a space, as in Sakura.
pub fn parse(text: &str) -> Vec<MenuNode> {
    let mut levels: Vec<Vec<MenuNode>> = vec![Vec::new()];
    let mut number = 1;
    for token in tokens(text) {
        let mut at = 0;
        let (mut checked, mut disabled, mut radio, mut submenu) = (false, false, false, false);
        let mut closes = 0;
        if token[0] == '[' {
            at = 1;
            while at < token.len() && token[at] != ']' {
                match token[at] {
                    'S' => submenu = true,
                    'E' => closes += 1,
                    'C' => checked = true,
                    'D' => disabled = true,
                    'R' => radio = true,
                    // `B` (a new column) has no counterpart in a pop-up of this window, and
                    // any other letter is ignored, as in Sakura.
                    _ => {}
                }
                at += 1;
            }
            if at < token.len() {
                at += 1;
            }
        }
        let label: String = token[at..].iter().collect();
        let node = if submenu {
            MenuNode::Submenu {
                label,
                checked,
                disabled,
                children: Vec::new(),
            }
        } else if token == ['-'] {
            MenuNode::Separator
        } else {
            let item = MenuNode::Item {
                label,
                number,
                checked,
                disabled,
                radio,
            };
            number += 1;
            item
        };
        if let Some(level) = levels.last_mut() {
            level.push(node);
        }
        if submenu {
            levels.push(Vec::new());
        }
        for _ in 0..closes {
            close_level(&mut levels);
        }
    }
    while levels.len() > 1 {
        close_level(&mut levels);
    }
    levels.pop().unwrap_or_default()
}

/// How many items can be chosen (the numbers run from 1 to this).
pub fn item_count(nodes: &[MenuNode]) -> i64 {
    nodes
        .iter()
        .map(|node| match node {
            MenuNode::Item { .. } => 1,
            MenuNode::Separator => 0,
            MenuNode::Submenu { children, .. } => item_count(children),
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str, number: i64) -> MenuNode {
        MenuNode::Item {
            label: label.into(),
            number,
            checked: false,
            disabled: false,
            radio: false,
        }
    }

    #[test]
    fn plain_items_are_numbered_from_one() {
        assert_eq!(
            parse("あ,い,う"),
            [item("あ", 1), item("い", 2), item("う", 3)]
        );
        // empty items are skipped and spaces stay
        assert_eq!(parse(",a,,b ,"), [item("a", 1), item("b ", 2)]);
        assert!(parse("").is_empty());
        assert!(parse(",,").is_empty());
    }

    #[test]
    fn a_separator_and_a_submenu_take_no_number() {
        let nodes = parse("a,-,[S]sub,b,[E]c,d");
        assert_eq!(
            nodes,
            [
                item("a", 1),
                MenuNode::Separator,
                MenuNode::Submenu {
                    label: "sub".into(),
                    checked: false,
                    disabled: false,
                    children: vec![item("b", 2), item("c", 3)],
                },
                item("d", 4),
            ]
        );
        assert_eq!(item_count(&nodes), 4);
    }

    #[test]
    fn the_flags_combine_and_unknown_letters_are_ignored() {
        let nodes = parse("[CDRB?]x,[C]y");
        assert_eq!(
            nodes,
            [
                MenuNode::Item {
                    label: "x".into(),
                    number: 1,
                    checked: true,
                    disabled: true,
                    radio: true
                },
                MenuNode::Item {
                    label: "y".into(),
                    number: 2,
                    checked: true,
                    disabled: false,
                    radio: false
                },
            ]
        );
    }

    #[test]
    fn nested_submenus_close_once_for_each_e() {
        // [EE] closes two levels.
        let nodes = parse("[S]a,[S]b,c,[EE]d,e");
        let MenuNode::Submenu { children, .. } = &nodes[0] else {
            panic!("{nodes:?}");
        };
        let MenuNode::Submenu {
            children: inner, ..
        } = &children[0]
        else {
            panic!("{children:?}");
        };
        assert_eq!(inner, &[item("c", 1), item("d", 2)]);
        assert_eq!(children.len(), 1);
        assert_eq!(nodes[1], item("e", 3));
        // a submenu that is not closed ends with the string; an E with no submenu open does nothing
        let nodes = parse("[E]a,[S]b,c");
        assert_eq!(nodes.len(), 2);
        assert_eq!(item_count(&nodes), 2);
        // [SE] opens and closes at once: an empty submenu
        let nodes = parse("[SE]s,t");
        assert_eq!(
            nodes[0],
            MenuNode::Submenu {
                label: "s".into(),
                checked: false,
                disabled: false,
                children: Vec::new()
            }
        );
        assert_eq!(nodes[1], item("t", 1));
    }

    #[test]
    fn a_bracket_is_written_with_an_empty_flag_list() {
        assert_eq!(parse("[][str]"), [item("[str]", 1)]);
        // no closing bracket: everything is flags, and the label is empty
        assert_eq!(parse("[abc"), [item("", 1)]);
    }

    #[test]
    fn a_comma_inside_double_quotes_does_not_split() {
        assert_eq!(parse("\"a,b\",c"), [item("\"a,b\"", 1), item("c", 2)]);
        // an open quote swallows the rest
        assert_eq!(parse("a,\"b,c"), [item("a", 1), item("\"b,c", 2)]);
    }

    #[test]
    fn only_a_lone_dash_is_a_separator() {
        assert_eq!(
            parse("-,--,[C]-"),
            [
                MenuNode::Separator,
                item("--", 1),
                MenuNode::Item {
                    label: "-".into(),
                    number: 2,
                    checked: true,
                    disabled: false,
                    radio: false
                }
            ]
        );
    }

    #[test]
    fn a_leading_space_stops_the_flags_as_in_sakura() {
        // The help writes " [E]..." with a space, but the code looks at the first character.
        assert_eq!(parse(" [E]x"), [item(" [E]x", 1)]);
    }

    #[test]
    fn japanese_labels_and_access_keys_are_kept() {
        assert_eq!(
            parse("[C]チェック項目(&A),[D]非活性"),
            [
                MenuNode::Item {
                    label: "チェック項目(&A)".into(),
                    number: 1,
                    checked: true,
                    disabled: false,
                    radio: false
                },
                MenuNode::Item {
                    label: "非活性".into(),
                    number: 2,
                    checked: false,
                    disabled: true,
                    radio: false
                }
            ]
        );
    }
}
