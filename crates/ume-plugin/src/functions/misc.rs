// Ported from Sakura Editor (https://github.com/sakura-editor/sakura, commit 48db6c7c25e2):
//   sakura_core/util/format.cpp (ParseVersion, CompareVersion)
//
// Copyright (C) 2011, syat
// Copyright (C) 2018-2022, Sakura Editor Organization
//
// This file is derived from the file above and was modified: the C++ code (a `WCHAR` buffer, a
// `UINT32` result) was rewritten in Rust on UTF-8 strings, and the tests are new.
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

//! Version comparison, the draw switch, the status bar and the document type of an extension.

use serde_json::Value;

use super::EditorFunc;
use crate::api::{Args, Ty, req};
use crate::host::Editor;
use crate::protocol::RpcError;

type Result<T = Value> = std::result::Result<T, RpcError>;

/// `ParseVersion` (`sakura_core/util/format.cpp`): a version string as four signed bytes in a
/// `UINT32`, most significant first (major, minor, build, revision). A part is a run of at most
/// two digits; a word before it ("alpha", "beta", "rc", "pl", or a letter that starts one)
/// shifts it below or above the plain release, so that "1.0a" < "1.0" < "1.0pl". Parts that are
/// left out are 0. Separators are `.`, `-`, `_` and `+`. The idea is that of PHP's
/// `version_compare`.
fn parse_version(version: &str) -> u32 {
    let chars: Vec<char> = version.chars().collect();
    let at = |p: usize| chars.get(p).copied();
    let starts = |p: usize, word: &str| word.chars().enumerate().all(|(i, w)| at(p + i) == Some(w));
    let starts_ignoring_case = |p: usize, word: &str| {
        word.chars()
            .enumerate()
            .all(|(i, w)| at(p + i).is_some_and(|c| c.to_lowercase().eq(w.to_lowercase())))
    };
    let is_digit = |c: Option<char>| c.is_some_and(|c| c.is_ascii_digit());

    let mut p = 0;
    let mut result: u32 = 0;
    let mut i = 0;
    while i < 4 && at(p).is_some() {
        // The special words, which shift the number.
        let shift: i32 = match at(p) {
            Some('a') => {
                p += if starts(p, "alpha") { 5 } else { 1 };
                -0x60
            }
            Some('b') => {
                p += if starts(p, "beta") { 4 } else { 1 };
                -0x40
            }
            Some('r' | 'R') => {
                p += if starts_ignoring_case(p, "rc") { 2 } else { 1 };
                -0x20
            }
            Some('p') => {
                p += if starts(p, "pl") { 2 } else { 1 };
                0x20
            }
            c if !is_digit(c) => -0x80,
            _ => 0,
        };
        while at(p).is_some() && !is_digit(at(p)) {
            p += 1;
        }
        // The number: two digits at most (a third digit is left for the next part).
        let mut number: i32 = 0;
        let mut digits = 0;
        while is_digit(at(p)) {
            digits += 1;
            if digits > 2 {
                break;
            }
            number = number * 10 + at(p).map_or(0, |c| c as i32 - '0' as i32);
            p += 1;
        }
        // The separators.
        while matches!(at(p), Some('.' | '-' | '_' | '+')) {
            p += 1;
        }
        let part = shift + number + 128;
        result |= (part << (24 - 8 * i)) as u32;
        i += 1;
    }
    // The parts that are left: signed 0 (0x80).
    while i < 4 {
        result |= 128u32 << (24 - 8 * i);
        i += 1;
    }
    result
}

/// `CompareVersion`: `ParseVersion(a) - ParseVersion(b)` as the `UINT32` it is, then as an `int`.
/// 0 is equal; positive means `a` is newer, negative `b`. (The size of the number depends on the
/// part that differs; only its sign is meant.)
pub fn compare_version(a: &str, b: &str) -> i32 {
    parse_version(a).wrapping_sub(parse_version(b)) as i32
}

/// F_COMPAREVERSION.
fn compare_version_call(_: &mut Editor, args: &Args) -> Result {
    Ok(i64::from(compare_version(args.str(0), args.str(1))).into())
}

/// F_SETDRAWSWITCH: sets the draw switch and returns the one before (1 on, 0 off). The window
/// keeps the flag for `GetDrawSwitch`; the text is still painted when the window decides to
/// (a macro runs on the UI thread, so nothing is painted in the middle of a call anyway).
fn set_draw_switch(ed: &mut Editor, args: &Args) -> Result {
    let old = ed.env.state.draw_switch;
    ed.env.state.draw_switch = args.int(0) != 0;
    Ok(i64::from(old).into())
}

/// F_GETDRAWSWITCH.
fn get_draw_switch(ed: &mut Editor, _: &Args) -> Result {
    Ok(i64::from(ed.env.state.draw_switch).into())
}

/// F_ISSHOWNSTATUS: 1 when the status bar is shown.
fn is_shown_status(ed: &mut Editor, _: &Args) -> Result {
    Ok(i64::from(ed.env.status_bar_shown).into())
}

/// F_ISCURTYPEEXT: 1 when the document type of the file in the window is the type that the
/// extension belongs to. The extension is looked up like a file name in the extension lists of
/// the types (`CDocTypeManager::GetDocumentTypeOfExt`). Without a window, or without type
/// settings, every extension is the basic type.
fn is_cur_type_ext(ed: &mut Editor, args: &Args) -> Result {
    Ok(i64::from(ed.env.type_of(args.str(0)) == ed.env.current_type).into())
}

/// F_ISSAMETYPEEXT: 1 when two extensions belong to the same document type.
fn is_same_type_ext(ed: &mut Editor, args: &Args) -> Result {
    Ok(i64::from(ed.env.type_of(args.str(0)) == ed.env.type_of(args.str(1))).into())
}

pub const FUNCS: &[EditorFunc] = &[
    EditorFunc {
        name: "CompareVersion",
        params: &[req("versionA", Ty::Str), req("versionB", Ty::Str)],
        result: Ty::Int,
        doc: "Compares two version strings: 0 when equal, positive when A is newer, negative \
              when B is. Four parts at most; a part has two digits at most; \"alpha\", \"beta\", \
              \"rc\" and \"pl\" put a part before or after the release (like PHP's \
              version_compare).",
        run: compare_version_call,
    },
    EditorFunc {
        name: "SetDrawSwitch",
        params: &[req("on", Ty::Int)],
        result: Ty::Int,
        doc: "Turns the draw switch on (non-zero) or off (0) and returns the state before (1 or \
              0). The editor keeps the flag for GetDrawSwitch but still paints the text when it \
              likes.",
        run: set_draw_switch,
    },
    EditorFunc {
        name: "GetDrawSwitch",
        params: &[],
        result: Ty::Int,
        doc: "1 when the draw switch is on (the start state), 0 when a macro turned it off.",
        run: get_draw_switch,
    },
    EditorFunc {
        name: "IsShownStatus",
        params: &[],
        result: Ty::Int,
        doc: "1 when the status bar is shown, else 0.",
        run: is_shown_status,
    },
    EditorFunc {
        name: "IsCurTypeExt",
        params: &[req("extension", Ty::Str)],
        result: Ty::Int,
        doc: "1 when the document type of this file is the type of the extension (looked up in \
              the extension lists of the types in the settings file, as a file name). Without \
              type settings every extension is the basic type.",
        run: is_cur_type_ext,
    },
    EditorFunc {
        name: "IsSameTypeExt",
        params: &[req("extension1", Ty::Str), req("extension2", Ty::Str)],
        result: Ty::Int,
        doc: "1 when the two extensions belong to the same document type (see IsCurTypeExt).",
        run: is_same_type_ext,
    },
];

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::testing::Fixture;
    use super::*;

    #[test]
    fn versions_compare_part_by_part() {
        assert_eq!(compare_version("1.2.3", "1.2.3"), 0);
        assert!(compare_version("1.2.4", "1.2.3") > 0);
        assert!(compare_version("1.2.3", "1.2.4") < 0);
        assert!(compare_version("2.0", "1.99") > 0);
        // the missing parts are 0
        assert_eq!(compare_version("1.2", "1.2.0.0"), 0);
        assert_eq!(compare_version("", ""), 0);
        // only two digits per part: "100" is 10 and then 0
        assert_eq!(compare_version("100", "10.0"), 0);
        // the exact differences: a step in the third part is 1 << 8
        assert_eq!(compare_version("1.2.4", "1.2.3"), 256);
        assert_eq!(compare_version("1.2.3", "1.2.4"), -256);
    }

    #[test]
    fn special_words_put_a_part_before_or_after_the_release() {
        // alpha < beta < rc < release < pl
        let order = ["1.0alpha", "1.0beta", "1.0rc", "1.0", "1.0pl"];
        for pair in order.windows(2) {
            assert!(compare_version(pair[0], pair[1]) < 0, "{pair:?}");
            assert!(compare_version(pair[1], pair[0]) > 0, "{pair:?}");
        }
        // one letter is enough, and RC can be in capitals
        assert_eq!(compare_version("1.0a", "1.0alpha"), 0);
        assert_eq!(compare_version("1.0b", "1.0beta"), 0);
        assert_eq!(compare_version("1.0RC1", "1.0rc1"), 0);
        assert!(compare_version("1.0rc1", "1.0rc2") < 0);
        assert_eq!(compare_version("1.0p", "1.0pl"), 0);
    }

    #[test]
    fn separators_and_text_are_skipped() {
        assert_eq!(compare_version("1-2_3+4", "1.2.3.4"), 0);
        // a word that is not special puts the part far below a plain number
        assert!(compare_version("v1.2", "1.2") < 0);
        // a version with only text is below every number
        assert!(compare_version("x", "0") < 0);
    }

    #[test]
    fn the_function_answers_an_int() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("CompareVersion", json!(["1.0", "1.0"])), json!(0));
        assert!(
            f.ok("CompareVersion", json!(["1.1", "1.0"]))
                .as_i64()
                .unwrap()
                > 0
        );
        assert!(f.call("CompareVersion", json!(["1.1"])).is_err());
    }

    #[test]
    fn the_draw_switch_answers_the_old_state() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("GetDrawSwitch", json!([])), json!(1));
        assert_eq!(f.ok("SetDrawSwitch", json!([0])), json!(1));
        assert_eq!(f.ok("GetDrawSwitch", json!([])), json!(0));
        assert_eq!(f.ok("SetDrawSwitch", json!([0])), json!(0));
        assert_eq!(f.ok("SetDrawSwitch", json!([5])), json!(0));
        assert_eq!(f.ok("GetDrawSwitch", json!([])), json!(1));
    }

    #[test]
    fn the_status_bar() {
        let mut f = Fixture::new("");
        assert_eq!(f.ok("IsShownStatus", json!([])), json!(0));
        f.env.status_bar_shown = true;
        assert_eq!(f.ok("IsShownStatus", json!([])), json!(1));
    }

    #[test]
    fn types_of_extensions_come_from_the_window() {
        let mut f = Fixture::new("");
        // without types: all basic, so the current type (0) matches everything
        assert_eq!(f.ok("IsCurTypeExt", json!(["cpp"])), json!(1));
        assert_eq!(f.ok("IsSameTypeExt", json!(["cpp", "txt"])), json!(1));
        f.env.current_type = 2;
        f.env.type_of_ext = Some(Box::new(|ext| match ext {
            "c" | "cpp" | "h" => 2,
            "txt" => 1,
            _ => 0,
        }));
        assert_eq!(f.ok("IsCurTypeExt", json!(["cpp"])), json!(1));
        assert_eq!(f.ok("IsCurTypeExt", json!(["txt"])), json!(0));
        assert_eq!(f.ok("IsCurTypeExt", json!(["zzz"])), json!(0));
        assert_eq!(f.ok("IsSameTypeExt", json!(["c", "h"])), json!(1));
        assert_eq!(f.ok("IsSameTypeExt", json!(["c", "txt"])), json!(0));
    }
}
